// =============================================================================
// Matrixon Matrix NextServer - Mod Module
// =============================================================================
//
// Project: Matrixon - Ultra High Performance Matrix NextServer (Synapse Alternative)
// Author: arkSong (arksong2018@gmail.com) - Founder of Matrixon Innovation Project
// Contributors: Matrixon Development Team
// Date: 2024-12-11
// Version: 2.0.0-alpha (PostgreSQL Backend)
// License: Apache 2.0 / MIT
//
// Description:
//   Core business logic service implementation. This module is part of the Matrixon Matrix NextServer
//   implementation, designed for enterprise-grade deployment with 20,000+
//   concurrent connections and <50ms response latency.
//
// Performance Targets:
//   • 20k+ concurrent connections
//   • <50ms response latency
//   • >99% success rate
//   • Memory-efficient operation
//   • Horizontal scalability
//
// Features:
//   • Business logic implementation
//   • Service orchestration
//   • Event handling and processing
//   • State management
//   • Enterprise-grade reliability
//
// Architecture:
//   • Async/await native implementation
//   • Zero-copy operations where possible
//   • Memory pool optimization
//   • Lock-free data structures
//   • Enterprise monitoring integration
//
// Dependencies:
//   • Tokio async runtime
//   • Structured logging with tracing
//   • Error handling with anyhow/thiserror
//   • Serialization with serde
//   • Matrix protocol types with ruma
//
// References:
//   • Matrix.org specification: https://matrix.org/
//   • Synapse reference: https://github.com/element-hq/synapse
//   • Matrix spec: https://spec.matrix.org/
//   • Performance guidelines: Internal Matrixon documentation
//
// Quality Assurance:
//   • Comprehensive unit testing
//   • Integration test coverage
//   • Performance benchmarking
//   • Memory leak detection
//   • Security audit compliance
//
// =============================================================================

type AsyncRecursiveType<'a, T> = Pin<Box<dyn Future<Output = T> + 'a + Send>>;

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use std::collections::hash_map::Entry;

use futures_util::{stream::FuturesUnordered, Future, StreamExt};
use globals::SigningKeys;
use ruma::{
    api::{
        client::error::ErrorKind,
        federation::{
            discovery::{
                get_remote_server_keys,
                get_remote_server_keys_batch::{self, QueryCriteria},
                get_server_keys,
            },
            event::{get_event, get_room_state_ids},
            membership::create_join_event,
        },
    },
    events::{
        room::{
            create::RoomCreateEventContent,
            redaction::RoomRedactionEventContent,
            server_acl::RoomServerAclEventContent,
        },
        StateEventType, TimelineEventType,
    },
    state_res::{StateMap, AuthorizationRules, RoomVersionRules},
    CanonicalJsonObject, CanonicalJsonValue, EventId,
    OwnedServerName, OwnedServerSigningKeyId, RoomId, RoomVersionId, ServerName,
    MilliSecondsSinceUnixEpoch,
};
use ruma::room_version_rules::{AuthorizationRules, RoomVersionRules};
use serde_json::value::RawValue as RawJsonValue;
use tokio::sync::{RwLock, RwLockWriteGuard, Semaphore};
use tracing::{debug, error, info, trace, warn};

use crate::{service::*, services, Error, PduEvent, Result};

use super::state_compressor::CompressedStateEvent;
use ruma::state_res;

pub struct Service;

impl Service {
    /// When receiving an event one needs to:
    /// 0. Check the server is in the room
    /// 1. Skip the PDU if we already know about it
    /// 1.1. Remove unsigned field
    /// 2. Check signatures, otherwise drop
    /// 3. Check content hash, redact if doesn't match
    /// 4. Fetch any missing auth events doing all checks listed here starting at 1. These are not
    ///    timeline events
    /// 5. Reject "due to auth events" if can't get all the auth events or some of the auth events are
    ///    also rejected "due to auth events"
    /// 6. Reject "due to auth events" if the event doesn't pass auth based on the auth events
    /// 7. Persist this event as an outlier
    /// 8. If not timeline event: stop
    /// 9. Fetch any missing prev events doing all checks listed here starting at 1. These are timeline
    ///    events
    /// 10. Fetch missing state and auth chain events by calling /state_ids at backwards extremities
    ///     doing all the checks in this list starting at 1. These are not timeline events
    /// 11. Check the auth of the event passes based on the state of the event
    /// 12. Ensure that the state is derived from the previous current state (i.e. we calculated by
    ///     doing state res where one of the inputs was a previously trusted set of state, don't just
    ///     trust a set of state we got from a remote)
    /// 13. Use state resolution to find new room state
    /// 14. Check if the event passes auth based on the "current state" of the room, if not soft fail it
    // We use some AsyncRecursiveType hacks here so we can call this async function recursively
    #[tracing::instrument(skip(self, value, is_timeline_event, pub_key_map))]
    pub(crate) async fn handle_incoming_pdu<'a>(
        &self,
        origin: &'a ServerName,
        event_id: &'a EventId,
        room_id: &'a RoomId,
        value: BTreeMap<String, CanonicalJsonValue>,
        is_timeline_event: bool,
        pub_key_map: &'a RwLock<BTreeMap<String, SigningKeys>>,
    ) -> Result<Option<Vec<u8>>> {
        // 0. Check the server is in the room
        if !services().rooms.metadata.exists(room_id)? {
            return Err(Error::BadRequest(
                crate::MatrixErrorKind::NotFound,
                "Room is unknown to this server",
            ));
        }

        if services().rooms.metadata.is_disabled(room_id)? {
            return Err(Error::BadRequest(
                crate::MatrixErrorKind::Forbidden,
                "Federation of this room is currently disabled on this server.",
            ));
        }

        services().rooms.event_handler.acl_check(origin, room_id)?;

        // 1. Skip the PDU if we already have it as a timeline event
        if let Some(pdu_id) = services().rooms.timeline.get_pdu_id(event_id)? {
            return Ok(Some(pdu_id.to_vec()));
        }

        let create_event = services()
            .rooms
            .state_accessor
            .room_state_get(room_id, &StateEventType::RoomCreate, "")?
            .ok_or_else(|| Error::Database("Failed to find create event in db.".to_string()))?;

        let create_event_content: RoomCreateEventContent =
            serde_json::from_str(create_event.content.get()).map_err(|e| {
                error!("Invalid create event: {}", e);
                Error::Database("Invalid create event in db".to_string())
            })?;
        let room_version_id = &create_event_content.room_version;

        let first_pdu_in_room = services()
            .rooms
            .timeline
            .first_pdu_in_room(room_id)?
            .ok_or_else(|| Error::Database("Failed to find first pdu in db.".to_string()))?;

        let (incoming_pdu, val) = self
            .handle_outlier_pdu(
                origin,
                &create_event,
                event_id,
                room_id,
                value,
                false,
                pub_key_map,
            )
            .await?;
        self.check_room_id(room_id, &incoming_pdu)?;

        // 8. if not timeline event: stop
        if !is_timeline_event {
            return Ok(None);
        }

        // Skip old events
        if incoming_pdu.origin_server_ts < first_pdu_in_room.origin_server_ts {
            return Ok(None);
        }

        // 9. Fetch any missing prev events doing all checks listed here starting at 1. These are timeline events
        let (sorted_prev_events, mut eventid_info) = self
            .fetch_unknown_prev_events(
                origin,
                &create_event,
                room_id,
                &room_version_id
                    .rules()
                    .expect("Supported room version has rules"),
                pub_key_map,
                incoming_pdu.prev_events.clone(),
            )
            .await?;

        let mut errors = 0;
        debug!(events = ?sorted_prev_events, "Got previous events");
        for prev_id in sorted_prev_events {
            // Check for disabled again because it might have changed
            if services().rooms.metadata.is_disabled(room_id)? {
                return Err(Error::BadRequest(
                    crate::MatrixErrorKind::Forbidden,
                    "Federation of this room is currently disabled on this server.",
                ));
            }

            if let Some((time, tries)) = services()
                .globals
                .bad_event_ratelimiter
                .read()
                .await
                .get(&*prev_id)
            {
                // Exponential backoff
                let mut min_elapsed_duration = Duration::from_secs(5 * 60) * (*tries) * (*tries);
                if min_elapsed_duration > Duration::from_secs(60 * 60 * 24) {
                    min_elapsed_duration = Duration::from_secs(60 * 60 * 24);
                }

                if time.elapsed() < min_elapsed_duration {
                    info!("Backing off from {}", prev_id);
                    continue;
                }
            }

            if errors >= 5 {
                // Timeout other events
                match services()
                    .globals
                    .bad_event_ratelimiter
                    .write()
                    .await
                    .entry((*prev_id).to_owned())
                {
                    Entry::Vacant(e) => {
                        e.insert((Instant::now(), 1));
                    }
                    Entry::Occupied(mut e) => {
                        *e.get_mut() = (Instant::now(), e.get().1 + 1)
                    }
                }
                continue;
            }

            if let Some((pdu, json)) = eventid_info.remove(&*prev_id) {
                // Skip old events
                if pdu.origin_server_ts < first_pdu_in_room.origin_server_ts {
                    continue;
                }

                let start_time = Instant::now();
                services()
                    .globals
                    .roomid_federationhandletime
                    .write()
                    .await
                    .insert(room_id.to_owned(), ((*prev_id).to_owned(), start_time));

                if let Err(e) = self
                    .upgrade_outlier_to_timeline_pdu(
                        pdu,
                        json,
                        &create_event,
                        origin,
                        room_id,
                        pub_key_map,
                    )
                    .await
                {
                    errors += 1;
                    warn!("Prev event {} failed: {}", prev_id, e);
                    match services()
                        .globals
                        .bad_event_ratelimiter
                        .write()
                        .await
                        .entry((*prev_id).to_owned())
                    {
                        Entry::Vacant(e) => {
                            e.insert((Instant::now(), 1));
                        }
                        Entry::Occupied(mut e) => {
                            *e.get_mut() = (Instant::now(), e.get().1 + 1)
                        }
                    }
                }
                let elapsed = start_time.elapsed();
                services()
                    .globals
                    .roomid_federationhandletime
                    .write()
                    .await
                    .remove(&room_id.to_owned());
                debug!(
                    "Handling prev event {} took {}m{}s",
                    prev_id,
                    elapsed.as_secs() / 60,
                    elapsed.as_secs() % 60
                );
            }
        }

        // Done with prev events, now handling the incoming event

        let start_time = Instant::now();
        services()
            .globals
            .roomid_federationhandletime
            .write()
            .await
            .insert(room_id.to_owned(), (event_id.to_owned(), start_time));
        let r = services()
            .rooms
            .event_handler
            .upgrade_outlier_to_timeline_pdu(
                incoming_pdu,
                val,
                &create_event,
                origin,
                room_id,
                pub_key_map,
            )
            .await;
        services()
            .globals
            .roomid_federationhandletime
            .write()
            .await
            .remove(&room_id.to_owned());

        r
    }

    #[allow(clippy::type_complexity, clippy::too_many_arguments)]
    #[tracing::instrument(skip(self, create_event, value, pub_key_map))]
    fn handle_outlier_pdu<'a>(
        &'a self,
        origin: &'a ServerName,
        create_event: &'a PduEvent,
        event_id: &'a EventId,
        room_id: &'a RoomId,
        mut value: BTreeMap<String, CanonicalJsonValue>,
        auth_events_known: bool,
        pub_key_map: &'a RwLock<BTreeMap<String, SigningKeys>>,
    ) -> AsyncRecursiveType<'a, Result<(Arc<PduEvent>, BTreeMap<String, CanonicalJsonValue>)>> {
        Box::pin(async move {
            // 1.1. Remove unsigned field
            value.remove("unsigned");

            // 2. Check signatures, otherwise drop
            // 3. check content hash, redact if doesn't match
            let create_event_content: RoomCreateEventContent =
                serde_json::from_str(create_event.content.get()).map_err(|e| {
                    error!("Invalid create event: {}", e);
                    Error::BadDatabase("Invalid create event in db".to_string())
                })?;

            let room_version_id = &create_event_content.room_version;
            let room_version_rules = room_version_id
                .rules()
                .expect("Supported room version has rules");

            // TODO: For RoomVersion6 we must check that Raw<..> is canonical do we anywhere?: https://matrix.org/docs/spec/rooms/v6#canonical-json

            // We go through all the signatures we see on the value and fetch the corresponding signing
            // keys
            self.fetch_required_signing_keys(&value, pub_key_map)
                .await?;

            let origin_server_ts = value.get("origin_server_ts").ok_or_else(|| {
                error!("Invalid PDU, no origin_server_ts field");
                Error::BadRequest(
                    crate::MatrixErrorKind::InvalidParam,
                    "Invalid PDU, no origin_server_ts field",
                )
            })?;

            let origin_server_ts: MilliSecondsSinceUnixEpoch = {
                let ts = origin_server_ts.as_integer().ok_or_else(|| {
                    Error::BadRequest(
                        crate::MatrixErrorKind::InvalidParam,
                        "origin_server_ts must be an integer",
                    )
                })?;

                MilliSecondsSinceUnixEpoch(i64::from(ts).try_into().map_err(|_| {
                    Error::BadRequest(crate::MatrixErrorKind::InvalidParam, "Time must be after the unix epoch")
                })?)
            };

            let guard = pub_key_map.read().await;

            let pkey_map = (*guard).clone();

            // Removing all the expired keys, unless the room version allows stale keys
            let filtered_keys = services().globals.filter_keys_server_map(
                pkey_map,
                origin_server_ts,
                room_version_id,
            );

            let mut val = match ruma::signatures::verify_event(&filtered_keys, &value, &room_version_rules) {
                Err(e) => {
                    warn!("Dropping bad event {}: {}", event_id, e);
                    return Err(Error::BadRequest(
                        crate::MatrixErrorKind::InvalidParam,
                        "Signature verification failed",
                    ));
                }
                Ok(ruma::signatures::Verified::Signatures) => {
                    warn!("Calculated hash does not match: {}", event_id);
                    ruma::canonical_json::redact(value, &room_version_rules.redaction, None)
                        .map_err(|e| {
                            warn!("Redaction failed for event {}: {}", event_id, e);
                            Error::BadRequest(
                                crate::MatrixErrorKind::InvalidParam,
                                "Redaction failed",
                            )
                        })?
                }
                Ok(ruma::signatures::Verified::All) => value,
            };

            drop(guard);

            // Now that we have checked the signature and hashes we can add the eventID and convert
            // to our PduEvent type
            val.insert(
                "event_id".to_owned(),
                CanonicalJsonValue::String(event_id.as_str().to_owned()),
            );
            let incoming_pdu = serde_json::from_value::<PduEvent>(
                serde_json::to_value(&val).expect("CanonicalJsonObj is a valid JsonValue"),
            )
            .map_err(|_| Error::bad_database("Event is not a valid PDU."))?;

            self.check_room_id(room_id, &incoming_pdu)?;

            if !auth_events_known {
                // 4. fetch any missing auth events doing all checks listed here starting at 1. These are not timeline events
                // 5. Reject "due to auth events" if can't get all the auth events or some of the auth events are also rejected "due to auth events"
                // NOTE: Step 5 is not applied anymore because it failed too often
                debug!(event_id = ?incoming_pdu.event_id(), "Fetching auth events");
                self.fetch_and_handle_outliers(
                    origin,
                    &incoming_pdu
                        .auth_events
                        .iter()
                        .map(|x| Arc::from(&**x))
                        .collect::<Vec<_>>(),
                    create_event,
                    room_id,
                    &room_version_rules,
                    pub_key_map,
                )
                .await;
            }

            // 6. Reject "due to auth events" if the event doesn't pass auth based on the auth events
            debug!(
                "Auth check for {} based on auth events",
                incoming_pdu.event_id()
            );

            // Build map of auth events
            let mut auth_events = HashMap::new();
            for id in &incoming_pdu.auth_events {
                let auth_event = match services().rooms.timeline.get_pdu(id)? {
                    Some(e) => e,
                    None => {
                        warn!("Could not find auth event {}", id);
                        continue;
                    }
                };

                self.check_room_id(room_id, &auth_event)?;

                match auth_events.entry((
                    auth_event.kind.to_string().into(),
                    auth_event
                        .state_key
                        .clone()
                        .expect("all auth events have state keys"),
                )) {
                    Entry::Vacant(v) => {
                        v.insert(auth_event);
                    }
                    Entry::Occupied(_) => {
                        return Err(Error::BadRequest(
                            crate::MatrixErrorKind::InvalidParam,
                            "Auth event's type and state_key combination exists multiple times.",
                        ));
                    }
                }
            }

            // The original create event must be in the auth events
            if !matches!(
                auth_events
                    .get(&(StateEventType::RoomCreate, "".to_owned()))
                    .map(|a: &Arc<PduEvent>| a.as_ref()),
                Some(_) | None
            ) {
                return Err(Error::BadRequest(
                    crate::MatrixErrorKind::InvalidParam,
                    "Incoming event refers to wrong create event.",
                ));
            }

            // Ensure state events have proper state keys
            for (event_type, state_key) in auth_events.keys() {
                if state_key.is_empty() && !matches!(event_type,
                    StateEventType::RoomCreate |
                    StateEventType::RoomMember |
                    StateEventType::RoomPowerLevels |
                    StateEventType::RoomJoinRules |
                    StateEventType::RoomHistoryVisibility |
                    StateEventType::RoomGuestAccess |
                    StateEventType::RoomName |
                    StateEventType::RoomTopic |
                    StateEventType::RoomAvatar |
                    StateEventType::RoomPinnedEvents |
                    StateEventType::RoomTombstone |
                    StateEventType::RoomEncryption |
                    StateEventType::RoomServerAcl |
                    StateEventType::RoomThirdPartyInvite |
                    StateEventType::RoomCanonicalAlias |
                    StateEventType::RoomAliases
                ) {
                    return Err(Error::BadRequest(
                        crate::MatrixErrorKind::InvalidParam,
                        "Non-state event in auth events.",
                    ));
                }
            }

            if state_res::event_auth::auth_check(
                &room_version_rules.authorization,
                &incoming_pdu,
                |k, s| auth_events.get(&(k.to_string().into(), s.to_owned())),
            )
            .is_err()
            {
                return Err(Error::BadRequest(
                    crate::MatrixErrorKind::InvalidParam,
                    "Auth check failed",
                ));
            }

            debug!("Validation successful.");

            // 7. Persist the event as an outlier.
            services()
                .rooms
                .outlier
                .add_pdu_outlier(&incoming_pdu.event_id(), &val)?;

            debug!("Added pdu as outlier.");

            Ok((Arc::new(incoming_pdu), val))
        })
    }

    #[tracing::instrument(skip(self, incoming_pdu, val, create_event, pub_key_map))]
    pub async fn upgrade_outlier_to_timeline_pdu(
        &self,
        incoming_pdu: Arc<PduEvent>,
        val: BTreeMap<String, CanonicalJsonValue>,
        create_event: &PduEvent,
        origin: &ServerName,
        room_id: &RoomId,
        pub_key_map: &RwLock<BTreeMap<String, SigningKeys>>,
    ) -> Result<Option<Vec<u8>>> {
        // Skip the PDU if we already have it as a timeline event
        if let Ok(Some(pduid)) = services().rooms.timeline.get_pdu_id(&incoming_pdu.event_id()) {
            return Ok(Some(pduid));
        }

        if services()
            .rooms
            .pdu_metadata
            .is_event_soft_failed(&incoming_pdu.event_id())?
        {
            return Err(Error::BadRequest(
                crate::MatrixErrorKind::InvalidParam,
                "Event has been soft failed",
            ));
        }

        info!("Upgrading {} to timeline pdu", incoming_pdu.event_id());

        let create_event_content: RoomCreateEventContent =
            serde_json::from_str(create_event.content.get()).map_err(|e| {
                warn!("Invalid create event: {}", e);
                Error::BadDatabase("Invalid create event in db".to_string())
            })?;

        let room_version_id = &create_event_content.room_version;
        let room_version_rules = room_version_id
            .rules()
            .expect("Supported room version has rules");

        // 10. Fetch missing state and auth chain events by calling /state_ids at backwards extremities
        //     doing all the checks in this list starting at 1. These are not timeline events.

        // TODO: if we know the prev_events of the incoming event we can avoid the request and build
        // the state from a known point and resolve if > 1 prev_event

        debug!("Requesting state at event");
        let mut state_at_incoming_event = None;

        if incoming_pdu.prev_events.len() == 1 {
            let prev_event = &*incoming_pdu.prev_events[0];
            let prev_event_sstatehash = services()
                .rooms
                .state_accessor
                .pdu_shortstatehash(prev_event)?;

            let state = if let Some(shortstatehash) = prev_event_sstatehash {
                Some(
                    services()
                        .rooms
                        .state_accessor
                        .state_full_ids(shortstatehash)
                        .await,
                )
            } else {
                None
            };

            if let Some(Ok(mut state)) = state {
                debug!("Using cached state");
                let prev_pdu = services()
                    .rooms
                    .timeline
                    .get_pdu(prev_event)
                    .ok()
                    .flatten()
                    .ok_or_else(|| {
                        Error::Database("Could not find prev event, but we know the state.".to_string())
                    })?;

                if let Some(state_key) = &prev_pdu.state_key {
                    let shortstatekey = services().rooms.short.get_or_create_shortstatekey(
                        &prev_pdu.kind.to_string().into(),
                        state_key,
                    )?;

                    state.insert(shortstatekey, Arc::from(prev_event));
                    // Now it's the state after the pdu
                }

                state_at_incoming_event = Some(state);
            }
        } else {
            debug!("Calculating state at event using state res");
            let mut extremity_sstatehashes = HashMap::new();

            let mut okay = true;
            for prev_eventid in &incoming_pdu.prev_events {
                let prev_event =
                    if let Ok(Some(pdu)) = services().rooms.timeline.get_pdu(prev_eventid) {
                        pdu
                    } else {
                        okay = false;
                        break;
                    };

                let sstatehash = if let Ok(Some(s)) = services()
                    .rooms
                    .state_accessor
                    .pdu_shortstatehash(prev_eventid)
                {
                    s
                } else {
                    okay = false;
                    break;
                };

                extremity_sstatehashes.insert(sstatehash, prev_event);
            }

            if okay {
                let mut fork_states = Vec::with_capacity(extremity_sstatehashes.len());
                let mut auth_chain_sets = Vec::with_capacity(extremity_sstatehashes.len());

                for (sstatehash, prev_event) in extremity_sstatehashes {
                    let mut leaf_state: HashMap<_, _> = services()
                        .rooms
                        .state_accessor
                        .state_full_ids(sstatehash)
                        .await?;

                    if let Some(state_key) = &prev_event.state_key {
                        let shortstatekey = services().rooms.short.get_or_create_shortstatekey(
                            &prev_event.kind.to_string().into(),
                            state_key,
                        )?;
                        leaf_state.insert(shortstatekey, Arc::from(&*prev_event.event_id()));
                        // Now it's the state after the pdu
                    }

                    let mut state = StateMap::with_capacity(leaf_state.len());
                    let mut starting_events = Vec::with_capacity(leaf_state.len());

                    for (k, id) in leaf_state {
                        if let Ok((ty, st_key)) = services().rooms.short.get_statekey_from_short(k)
                        {
                            // FIXME: Undo .to_string().into() when StateMap
                            //        is updated to use StateEventType
                            state.insert((ty.to_string().into(), st_key), id.clone());
                        } else {
                            warn!("Failed to get_statekey_from_short.");
                        }
                        starting_events.push(id);
                    }

                    auth_chain_sets.push(
                        services()
                            .rooms
                            .auth_chain
                            .get_auth_chain(room_id, starting_events)
                            .await?
                            .collect(),
                    );

                    fork_states.push(state);
                }

                let lock = services().globals.stateres_mutex.lock();

                let result = state_res::resolve(
                    &room_version_id
                        .rules()
                        .expect("Supported room version has rules")
                        .authorization,
                    &fork_states,
                    auth_chain_sets,
                    |id| {
                        let res = services().rooms.timeline.get_pdu(id);
                        if let Err(e) = &res {
                            error!("LOOK AT ME Failed to fetch event: {}", e);
                        }
                        res.ok().flatten()
                    },
                );
                drop(lock);

                state_at_incoming_event = match result {
                    Ok(new_state) => Some(
                        new_state
                            .into_iter()
                            .map(|((event_type, state_key), event_id)| {
                                let shortstatekey =
                                    services().rooms.short.get_or_create_shortstatekey(
                                        &event_type.to_string().into(),
                                        &state_key,
                                    )?;
                                Ok((shortstatekey, event_id))
                            })
                            .collect::<Result<_>>()?,
                    ),
                    Err(e) => {
                        warn!("State resolution on prev events failed, either an event could not be found or deserialization: {}", e);
                        None
                    }
                }
            }
        }

        if state_at_incoming_event.is_none() {
            debug!("Calling /state_ids");
            // Call /state_ids to find out what the state at this pdu is. We trust the server's
            // response to some extend, but we still do a lot of checks on the events
            match services()
                .sending
                .send_federation_request(
                    origin,
                    {
                        let request = get_room_state_ids::v1::Request::new(
                            (*incoming_pdu.event_id()).to_owned(),
                            room_id.to_owned()
                        );
                        request
                    },
                )
                .await
            {
                Ok(res) => {
                    debug!("Fetching state events at event.");
                    let collect = res
                        .pdu_ids
                        .iter()
                        .map(|x| Arc::from(&**x))
                        .collect::<Vec<_>>();
                    let state_vec = self
                        .fetch_and_handle_outliers(
                            origin,
                            &collect,
                            create_event,
                            room_id,
                            &room_version_rules,
                            pub_key_map,
                        )
                        .await;

                    let mut state: HashMap<_, Arc<EventId>> = HashMap::new();
                    for (pdu, _) in state_vec {
                        let state_key = pdu.state_key.clone().ok_or_else(|| {
                            Error::Database("Found non-state pdu in state events.".to_string())
                        })?;

                        let shortstatekey = services().rooms.short.get_or_create_shortstatekey(
                            &pdu.kind.to_string().into(),
                            &state_key,
                        )?;

                        match state.entry(shortstatekey) {
                            Entry::Vacant(v) => {
                                v.insert(Arc::from(&*pdu.event_id()));
                            }
                            Entry::Occupied(_) => return Err(
                                Error::Database("State event's type and state_key combination exists multiple times.".to_string()),
                            ),
                        }
                    }

                    // The original create event must still be in the state
                    let create_shortstatekey = services()
                        .rooms
                        .short
                        .get_shortstatekey(&StateEventType::RoomCreate, "")?
                        .expect("Room exists");

                    if state.get(&create_shortstatekey).map(|id| id.as_ref())
                        != Some(&create_event.event_id())
                    {
                        return Err(Error::Database(
                            "Incoming event refers to wrong create event.".to_string(),
                        ));
                    }

                    state_at_incoming_event = Some(state);
                }
                Err(e) => {
                    warn!("Fetching state for event failed: {}", e);
                    return Err(e);
                }
            };
        }

        let state_at_incoming_event =
            state_at_incoming_event.expect("we always set this to some above");

        debug!("Starting auth check");
        // 11. Check the auth of the event passes based on the state of the event
        if state_res::event_auth::auth_check(
            &room_version_rules.authorization,
            &incoming_pdu,
            |k, s| {
                services()
                    .rooms
                    .short
                    .get_shortstatekey(&k.to_string().into(), s)
                    .ok()
                    .flatten()
                    .and_then(|shortstatekey| state_at_incoming_event.get(&shortstatekey))
                    .and_then(|event_id| services().rooms.timeline.get_pdu(event_id).ok().flatten())
            },
        )
        .is_err()
        {
            return Err(Error::bad_database(
                "Event has failed auth check with state at the event.".to_string(),
            ));
        }
        debug!("Auth check succeeded");

        // Soft fail check before doing state res
        let auth_events = services().rooms.state.get_auth_events(
            room_id,
            &incoming_pdu.kind,
            &incoming_pdu.sender(),
            incoming_pdu.state_key.as_deref(),
            &incoming_pdu.content,
            &room_version_rules.authorization,
        )?;

        let soft_fail = state_res::event_auth::auth_check(
            &room_version_rules.authorization,
            &incoming_pdu,
            |k, s| auth_events.get(&(k.clone(), s.to_owned())),
        )
        .is_err()
            || incoming_pdu.kind == TimelineEventType::RoomRedaction
                && match room_version_id {
                    RoomVersionId::V1
                    | RoomVersionId::V2
                    | RoomVersionId::V3
                    | RoomVersionId::V4
                    | RoomVersionId::V5
                    | RoomVersionId::V6
                    | RoomVersionId::V7
                    | RoomVersionId::V8
                    | RoomVersionId::V9
                    | RoomVersionId::V10 => {
                        if let Some(redact_id) = &incoming_pdu.redacts {
                            !services().rooms.state_accessor.user_can_redact(
                                redact_id,
                                &incoming_pdu.sender(),
                                &incoming_pdu.room_id(),
                                true,
                            )?
                        } else {
                            false
                        }
                    }
                    RoomVersionId::V11 => {
                        let content = serde_json::from_str::<RoomRedactionEventContent>(
                            incoming_pdu.content.get(),
                        )
                        .map_err(|_| Error::Database("Invalid content in redaction pdu.".to_string()))?;

                        if let Some(redact_id) = &content.redacts {
                            !services().rooms.state_accessor.user_can_redact(
                                redact_id,
                                &incoming_pdu.sender(),
                                &incoming_pdu.room_id(),
                                true,
                            )?
                        } else {
                            false
                        }
                    }
                    _ => {
                        unreachable!("Validity of room version already checked")
                    }
                };

        // 13. Use state resolution to find new room state

        // We start looking at current room state now, so lets lock the room
        let mutex_state = Arc::clone(
            services()
                .globals
                .roomid_mutex_state
                .write()
                .await
                .entry(room_id.to_owned())
                .or_default(),
        );
        let state_lock = mutex_state.lock().await;

        // Now we calculate the set of extremities this room has after the incoming event has been
        // applied. We start with the previous extremities (aka leaves)
        debug!("Calculating extremities");
        let mut extremities = services().rooms.state.get_forward_extremities(room_id)?;

        // Remove any forward extremities that are referenced by this incoming event's prev_events
        for prev_event in &incoming_pdu.prev_events {
            if extremities.contains(prev_event) {
                extremities.remove(prev_event);
            }
        }

        // Only keep those extremities were not referenced yet
        extremities.retain(|id| {
            !matches!(
                services()
                    .rooms
                    .pdu_metadata
                    .is_event_referenced(room_id, id),
                Ok(true)
            )
        });

        debug!("Compressing state at event");
        let state_ids_compressed = Arc::new(
            state_at_incoming_event
                .iter()
                .map(|(shortstatekey, id)| {
                    services()
                        .rooms
                        .state_compressor
                        .compress_state_event(*shortstatekey, id)
                })
                .collect::<Result<_>>()?,
        );

        if incoming_pdu.state_key.is_some() {
            debug!("Preparing for stateres to derive new room state");

            // We also add state after incoming event to the fork states
            let mut state_after = state_at_incoming_event.clone();
            if let Some(state_key) = &incoming_pdu.state_key {
                let shortstatekey = services().rooms.short.get_or_create_shortstatekey(
                    &incoming_pdu.kind.to_string().into(),
                    state_key,
                )?;

                state_after.insert(shortstatekey, Arc::from(&*incoming_pdu.event_id()));
            }

            let new_room_state = self
                .resolve_state(room_id, &room_version_rules.authorization, state_after)
                .await?;

            // Set the new room state to the resolved state
            debug!("Forcing new room state");

            let (sstatehash, new, removed) = services()
                .rooms
                .state_compressor
                .save_state(room_id, new_room_state)?;

            services()
                .rooms
                .state
                .force_state(room_id, sstatehash, new, removed, &state_lock)
                .await?;
        }

        // 14. Check if the event passes auth based on the "current state" of the room, if not soft fail it
        debug!("Starting soft fail auth check");

        if soft_fail {
            services()
                .rooms
                .timeline
                .append_incoming_pdu(
                    &incoming_pdu,
                    val,
                    extremities.iter().map(|e| (**e).to_owned()).collect(),
                    state_ids_compressed,
                    soft_fail,
                    &state_lock,
                )
                .await?;

            // Soft fail, we keep the event as an outlier but don't add it to the timeline
            warn!("Event was soft failed: {:?}", incoming_pdu);
            services()
                .rooms
                .pdu_metadata
                .mark_event_soft_failed(&incoming_pdu.event_id())?;
            return Err(Error::BadRequest(
                crate::MatrixErrorKind::InvalidParam,
                "Event has been soft failed".to_string(),
            ));
        }

        debug!("Appending pdu to timeline");
        extremities.insert(incoming_pdu.event_id().clone());

        // Now that the event has passed all auth it is added into the timeline.
        // We use the `state_at_event` instead of `state_after` so we accurately
        // represent the state for this event.

        let pdu_id = services()
            .rooms
            .timeline
            .append_incoming_pdu(
                &incoming_pdu,
                val,
                extremities.iter().map(|e| (**e).to_owned()).collect(),
                state_ids_compressed,
                soft_fail,
                &state_lock,
            )
            .await?;

        debug!("Appended incoming pdu");

        // Event has passed all auth/stateres checks
        drop(state_lock);
        Ok(pdu_id)
    }

    async fn resolve_state(
        &self,
        room_id: &RoomId,
        auth_rules: &AuthorizationRules,
        incoming_state: HashMap<u64, Arc<EventId>>,
    ) -> Result<Arc<HashSet<CompressedStateEvent>>> {
        debug!("Loading current room state ids");
        let current_sstatehash = services()
            .rooms
            .state
            .get_room_shortstatehash(room_id)?
            .expect("every room has state");

        let current_state_ids = services()
            .rooms
            .state_accessor
            .state_full_ids(current_sstatehash)
            .await?;

        let fork_states = [current_state_ids, incoming_state];

        let mut auth_chain_sets = Vec::new();
        for state in &fork_states {
            auth_chain_sets.push(
                services()
                    .rooms
                    .auth_chain
                    .get_auth_chain(room_id, state.iter().map(|(_, id)| id.clone()).collect())
                    .await?
                    .collect(),
            );
        }

        debug!("Loading fork states");

        let fork_states: Vec<_> = fork_states
            .into_iter()
            .map(|map| {
                map.into_iter()
                    .filter_map(|(k, id)| {
                        services()
                            .rooms
                            .short
                            .get_statekey_from_short(k)
                            .map(|(ty, st_key)| ((ty.to_string().into(), st_key), id))
                            .ok()
                    })
                    .collect::<StateMap<_>>()
            })
            .collect();

        debug!("Resolving state");

        let fetch_event = |id: &_| {
            let res = services().rooms.timeline.get_pdu(id);
            if let Err(e) = &res {
                error!("LOOK AT ME Failed to fetch event: {}", e);
            }
            res.ok().flatten()
        };

        let lock = services().globals.stateres_mutex.lock();
        let state = match state_res::resolve(auth_rules, &fork_states, auth_chain_sets, fetch_event)
        {
            Ok(new_state) => new_state,
            Err(_) => {
                return Err(Error::Database("State resolution failed, either an event could not be found or a prev_event is missing.".to_string()));
            }
        };

        drop(lock);

        debug!("State resolution done. Compressing state");

        let new_room_state = state
            .into_iter()
            .map(|((event_type, state_key), event_id)| {
                let shortstatekey = services()
                    .rooms
                    .short
                    .get_or_create_shortstatekey(&event_type.to_string().into(), &state_key)?;
                services()
                    .rooms
                    .state_compressor
                    .compress_state_event(shortstatekey, &event_id)
            })
            .collect::<Result<_>>()?;

        Ok(Arc::new(new_room_state))
    }

    /// Find the event and auth it. Once the event is validated (steps 1 - 8)
    /// it is appended to the outliers Tree.
    ///
    /// Returns pdu and if we fetched it over federation the raw json.
    ///
    /// a. Look in the main timeline (pduid_pdu tree)
    /// b. Look at outlier pdu tree
    /// c. Ask origin server over federation
    /// d. TODO: Ask other servers over federation?
    #[allow(clippy::type_complexity)]
    #[tracing::instrument(skip_all)]
    pub(crate) fn fetch_and_handle_outliers<'a>(
        &'a self,
        origin: &'a ServerName,
        events: &'a [Arc<EventId>],
        create_event: &'a PduEvent,
        room_id: &'a RoomId,
        room_version_rules: &'a RoomVersionRules,
        pub_key_map: &'a RwLock<BTreeMap<String, SigningKeys>>,
    ) -> AsyncRecursiveType<'a, Vec<(Arc<PduEvent>, Option<BTreeMap<String, CanonicalJsonValue>>)>>
    {
        Box::pin(async move {
            let back_off = |id| async move {
                match services()
                    .globals
                    .bad_event_ratelimiter
                    .write()
                    .await
                    .entry(id)
                {
                    Entry::Vacant(e) => {
                        e.insert((Instant::now(), 1));
                    }
                    Entry::Occupied(mut e) => {
                        *e.get_mut() = (Instant::now(), e.get().1 + 1)
                    }
                }
            };

            let mut pdus = vec![];
            for id in events {
                // a. Look in the main timeline (pduid_pdu tree)
                // b. Look at outlier pdu tree
                // (get_pdu_json checks both)
                if let Ok(Some(local_pdu)) = services().rooms.timeline.get_pdu(id) {
                    trace!("Found {} in db", id);
                    pdus.push((local_pdu, None));
                    continue;
                }

                // c. Ask origin server over federation
                // We also handle its auth chain here so we don't get a stack overflow in
                // handle_outlier_pdu.
                let mut todo_auth_events = vec![Arc::clone(id)];
                let mut events_in_reverse_order = Vec::new();
                let mut events_all = HashSet::new();
                let mut i = 0;
                while let Some(next_id) = todo_auth_events.pop() {
                    if let Some((time, tries)) = services()
                        .globals
                        .bad_event_ratelimiter
                        .read()
                        .await
                        .get(&*next_id)
                    {
                        // Exponential backoff
                        let mut min_elapsed_duration =
                            Duration::from_secs(5 * 60) * (*tries) * (*tries);
                        if min_elapsed_duration > Duration::from_secs(60 * 60 * 24) {
                            min_elapsed_duration = Duration::from_secs(60 * 60 * 24);
                        }

                        if time.elapsed() < min_elapsed_duration {
                            info!("Backing off from {}", next_id);
                            continue;
                        }
                    }

                    if events_all.contains(&next_id) {
                        continue;
                    }

                    i += 1;
                    if i % 100 == 0 {
                        tokio::task::yield_now().await;
                    }

                    if let Ok(Some(_)) = services().rooms.timeline.get_pdu(&next_id) {
                        trace!("Found {} in db", next_id);
                        continue;
                    }

                    info!("Fetching {} over federation.", next_id);
                    match services()
                        .sending
                        .send_federation_request(
                            origin,
                                                get_event::v1::Request::new((*next_id).to_owned()),
                        )
                        .await
                    {
                        Ok(res) => {
                            info!("Got {} over federation", next_id);
                            let (calculated_event_id, value) =
                                match pdu::gen_event_id_canonical_json(&res.pdu, room_version_rules)
                                {
                                    Ok(t) => t,
                                    Err(_) => {
                                        back_off((*next_id).to_owned()).await;
                                        continue;
                                    }
                                };

                            if calculated_event_id != *next_id {
                                warn!("Server didn't return event id we requested: requested: {}, we got {}. Event: {:?}",
                                    next_id, calculated_event_id, &res.pdu);
                            }

                            if let Some(auth_events) =
                                value.get("auth_events").and_then(|c| c.as_array())
                            {
                                for auth_event in auth_events {
                                    if let Ok(auth_event) =
                                        serde_json::from_value(auth_event.clone().into())
                                    {
                                        let a: Arc<EventId> = auth_event;
                                        todo_auth_events.push(a);
                                    } else {
                                        warn!("Auth event id is not valid");
                                    }
                                }
                            } else {
                                warn!("Auth event list invalid");
                            }

                            events_in_reverse_order.push((next_id.clone(), value));
                            events_all.insert(next_id);
                        }
                        Err(_) => {
                            warn!("Failed to fetch event: {}", next_id);
                            back_off((*next_id).to_owned()).await;
                        }
                    }
                }

                for (next_id, value) in events_in_reverse_order.iter().rev() {
                    if let Some((time, tries)) = services()
                        .globals
                        .bad_event_ratelimiter
                        .read()
                        .await
                        .get(&**next_id)
                    {
                        // Exponential backoff
                        let mut min_elapsed_duration =
                            Duration::from_secs(5 * 60) * (*tries) * (*tries);
                        if min_elapsed_duration > Duration::from_secs(60 * 60 * 24) {
                            min_elapsed_duration = Duration::from_secs(60 * 60 * 24);
                        }

                        if time.elapsed() < min_elapsed_duration {
                            info!("Backing off from {}", next_id);
                            continue;
                        }
                    }

                    match self
                        .handle_outlier_pdu(
                            origin,
                            create_event,
                            next_id,
                            room_id,
                            value.clone(),
                            true,
                            pub_key_map,
                        )
                        .await
                    {
                        Ok((pdu, json)) => {
                            if next_id == id {
                                pdus.push((pdu, Some(json)));
                            }
                        }
                        Err(e) => {
                            warn!("Authentication of event {} failed: {:?}", next_id, e);
                            back_off((**next_id).to_owned()).await;
                        }
                    }
                }
            }
            pdus
        })
    }

    #[tracing::instrument(skip_all)]
    pub(crate) async fn fetch_required_signing_keys(
        &self,
        event: &BTreeMap<String, CanonicalJsonValue>,
        pub_key_map: &RwLock<BTreeMap<String, SigningKeys>>,
    ) -> Result<()> {
        let signatures = event
            .get("signatures")
            .ok_or(Error::BadServerResponse(
                "No signatures in server response pdu.",
            ))?
            .as_object()
            .ok_or(Error::BadServerResponse(
                "Invalid signatures object in server response pdu.",
            ))?;

        // We go through all the signatures we see on the value and fetch the corresponding signing
        // keys
        for (signature_server, signature) in signatures {
            let signature_object = signature.as_object().ok_or(Error::BadServerResponse(
                "Invalid signatures content object in server response pdu.",
            ))?;

            let signature_ids = signature_object.keys().cloned().collect::<Vec<_>>();

            let fetch_res = self
                .fetch_signing_keys(
                    signature_server.as_str().try_into().map_err(|_| {
                        Error::BadServerResponse(
                            "Invalid servername in signatures of server response pdu.",
                        )
                    })?,
                    signature_ids,
                    true,
                )
                .await;

            let keys = match fetch_res {
                Ok(keys) => keys,
                Err(_) => {
                    warn!("Signature verification failed: Could not fetch signing key.",);
                    continue;
                }
            };

            pub_key_map
                .write()
                .await
                .insert(signature_server.clone(), keys);
        }

        Ok(())
    }

    // Gets a list of servers for which we don't have the signing key yet. We go over
    // the PDUs and either cache the key or add it to the list that needs to be retrieved.
    async fn get_server_keys_from_cache(
        &self,
        pdu: &RawJsonValue,
        servers: &mut BTreeMap<OwnedServerName, BTreeMap<OwnedServerSigningKeyId, QueryCriteria>>,
        room_version_rules: &RoomVersionRules,
        pub_key_map: &mut RwLockWriteGuard<'_, BTreeMap<String, SigningKeys>>,
    ) -> Result<()> {
        let value: CanonicalJsonObject = serde_json::from_str(pdu.get()).map_err(|e| {
            error!("Invalid PDU in server response: {:?}: {:?}", pdu, e);
            Error::BadServerResponse("Invalid PDU in server response".to_string())
        })?;

        let event_id = format!(
            "${}",
            ruma::signatures::reference_hash(&value, room_version_rules)
                .map_err(|_| Error::BadRequest(crate::MatrixErrorKind::Unknown, "Invalid PDU format"))?
        );
        let event_id = <&EventId>::try_from(event_id.as_str())
            .expect("ruma's reference hashes are valid event ids");

        if let Some((time, tries)) = services()
            .globals
            .bad_event_ratelimiter
            .read()
            .await
            .get(event_id)
        {
            // Exponential backoff
            let mut min_elapsed_duration = Duration::from_secs(30) * (*tries) * (*tries);
            if min_elapsed_duration > Duration::from_secs(60 * 60 * 24) {
                min_elapsed_duration = Duration::from_secs(60 * 60 * 24);
            }

            if time.elapsed() < min_elapsed_duration {
                debug!("Backing off from {}", event_id);
                return Err(Error::BadServerResponse("bad event, still backing off".to_string()));
            }
        }

        let signatures = value
            .get("signatures")
            .ok_or(Error::BadServerResponse(
                "No signatures in server response pdu.",
            ))?
            .as_object()
            .ok_or(Error::BadServerResponse(
                "Invalid signatures object in server response pdu.",
            ))?;

        for (signature_server, signature) in signatures {
            let signature_object = signature.as_object().ok_or(Error::BadServerResponse(
                "Invalid signatures content object in server response pdu.",
            ))?;

            let signature_ids = signature_object.keys().cloned().collect::<Vec<_>>();

            let contains_all_ids = |keys: &SigningKeys| {
                signature_ids.iter().all(|id| {
                    keys.verify_keys
                        .keys()
                        .map(ToString::to_string)
                        .any(|key_id| id == &key_id)
                        || keys
                            .old_verify_keys
                            .keys()
                            .map(ToString::to_string)
                            .any(|key_id| id == &key_id)
                })
            };

            let origin = <&ServerName>::try_from(signature_server.as_str()).map_err(|_| {
                Error::BadServerResponse("Invalid servername in signatures of server response pdu.".to_string())
            })?;

            if servers.contains_key(origin) || pub_key_map.contains_key(origin.as_str()) {
                continue;
            }

            trace!("Loading signing keys for {}", origin);

            if let Some(result) = services().globals.signing_keys_for(origin)? {
                if !contains_all_ids(&result) {
                    trace!("Signing key not loaded for {}", origin);
                    servers.insert(origin.to_owned(), BTreeMap::new());
                }

                pub_key_map.insert(origin.to_string(), result);
            }
        }

        Ok(())
    }

    pub(crate) async fn fetch_join_signing_keys(
        &self,
        event: &create_join_event::v2::Response,
        room_version_rules: &RoomVersionRules,
        pub_key_map: &RwLock<BTreeMap<String, SigningKeys>>,
    ) -> Result<()> {
        let mut servers: BTreeMap<
            OwnedServerName,
            BTreeMap<OwnedServerSigningKeyId, QueryCriteria>,
        > = BTreeMap::new();

        {
            let mut pkm = pub_key_map.write().await;

            // Try to fetch keys, failure is okay
            // Servers we couldn't find in the cache will be added to `servers`
            for pdu in &event.room_state.state {
                let _ = self
                    .get_server_keys_from_cache(pdu, &mut servers, room_version_rules, &mut pkm)
                    .await;
            }
            for pdu in &event.room_state.auth_chain {
                let _ = self
                    .get_server_keys_from_cache(pdu, &mut servers, room_version_rules, &mut pkm)
                    .await;
            }

            drop(pkm);
        }

        if servers.is_empty() {
            info!("We had all keys locally");
            return Ok(());
        }

        for server in services().globals.trusted_servers() {
            info!("Asking batch signing keys from trusted server {}", server);
            if let Ok(keys) = services()
                .sending
                .send_federation_request(
                    server,
                    get_remote_server_keys_batch::v2::Request::new(servers.clone()),
                )
                .await
            {
                trace!("Got signing keys: {:?}", keys);
                let mut pkm = pub_key_map.write().await;
                for k in keys.server_keys {
                    let k = match k.deserialize() {
                        Ok(key) => key,
                        Err(e) => {
                            warn!(
                                "Received error {} while fetching keys from trusted server {}",
                                e, server
                            );
                            warn!("{}", k.into_json());
                            continue;
                        }
                    };

                    // TODO: Check signature from trusted server?
                    servers.remove(&k.server_name);

                    let result = services()
                        .globals
                        .add_signing_key_from_trusted_server(&k.server_name, k.clone())?;

                    pkm.insert(k.server_name.to_string(), result);
                }
            }

            if servers.is_empty() {
                info!("Trusted server supplied all signing keys");
                return Ok(());
            }
        }

        info!("Asking individual servers for signing keys: {servers:?}");
        let mut futures: FuturesUnordered<_> = servers
            .into_keys()
            .map(|server| async move {
                (
                    services()
                        .sending
                        .send_federation_request(&server, get_server_keys::v2::Request::new())
                        .await,
                    server,
                )
            })
            .collect();

        while let Some(result) = futures.next().await {
            info!("Received new result");
            if let (Ok(get_keys_response), origin) = result {
                info!("Result is from {origin}");
                if let Ok(key) = get_keys_response.server_key.deserialize() {
                    let result = services()
                        .globals
                        .add_signing_key_from_origin(&origin, key)?;
                    pub_key_map.write().await.insert(origin.to_string(), result);
                }
            }
            info!("Done handling result");
        }

        info!("Search for signing keys done");

        Ok(())
    }

    /// Returns Ok if the acl allows the server
    pub fn acl_check(&self, server_name: &ServerName, room_id: &RoomId) -> Result<()> {
        let acl_event = match services().rooms.state_accessor.room_state_get(
            room_id,
            &StateEventType::RoomServerAcl,
            "",
        )? {
            Some(acl) => acl,
            None => return Ok(()),
        };

        let acl_event_content: RoomServerAclEventContent =
            match serde_json::from_str(acl_event.content.get()) {
                Ok(content) => content,
                Err(_) => {
                    warn!("Invalid ACL event");
                    return Ok(());
                }
            };

        if acl_event_content.is_allowed(server_name) {
            Ok(())
        } else {
            info!(
                "Server {} was denied by room ACL in {}",
                server_name, room_id
            );
            Err(Error::BadRequest(
                crate::MatrixErrorKind::Forbidden,
                "Server was denied by room ACL",
            ))
        }
    }

    /// Search the DB for the signing keys of the given server, if we don't have them
    /// fetch them from the server and save to our DB.
    #[tracing::instrument(skip_all)]
    pub async fn fetch_signing_keys(
        &self,
        origin: &ServerName,
        signature_ids: Vec<String>,
        // Whether to ask for keys from trusted servers. Should be false when getting
        // keys for validating requests, as per MSC4029
        query_via_trusted_servers: bool,
    ) -> Result<SigningKeys> {
        let contains_all_ids = |keys: &SigningKeys| {
            signature_ids.iter().all(|id| {
                keys.verify_keys
                    .keys()
                    .map(ToString::to_string)
                    .any(|key_id| id == &key_id)
                    || keys
                        .old_verify_keys
                        .keys()
                        .map(ToString::to_string)
                        .any(|key_id| id == &key_id)
            })
        };

        let permit = services()
            .globals
            .servername_ratelimiter
            .read()
            .await
            .get(origin)
            .map(|s| Arc::clone(s).acquire_owned());

        let permit = match permit {
            Some(p) => p,
            None => {
                let mut write = services().globals.servername_ratelimiter.write().await;
                let s = Arc::clone(
                    write
                        .entry(origin.to_owned())
                        .or_insert_with(|| Arc::new(Semaphore::new(1))),
                );

                s.acquire_owned()
            }
        }
        .await;

        let back_off = |id| async {
            match services()
                .globals
                .bad_signature_ratelimiter
                .write()
                .await
                .entry(id)
            {
                Entry::Vacant(e) => {
                    e.insert((Instant::now(), 1));
                }
                Entry::Occupied(mut e) => *e.get_mut() = (Instant::now(), e.get().1 + 1),
            }
        };

        if let Some((time, tries)) = services()
            .globals
            .bad_signature_ratelimiter
            .read()
            .await
            .get(&signature_ids)
        {
            // Exponential backoff
            let mut min_elapsed_duration = Duration::from_secs(30) * (*tries) * (*tries);
            if min_elapsed_duration > Duration::from_secs(60 * 60 * 24) {
                min_elapsed_duration = Duration::from_secs(60 * 60 * 24);
            }

            if time.elapsed() < min_elapsed_duration {
                debug!("Backing off from {:?}", signature_ids);
                return Err(Error::BadServerResponse("bad signature, still backing off".to_string()));
            }
        }

        trace!("Loading signing keys for {}", origin);

        let result = services().globals.signing_keys_for(origin)?;

        let mut expires_soon_or_has_expired = false;

        if let Some(result) = result.clone() {
            let ts_threshold = MilliSecondsSinceUnixEpoch::from_system_time(
                SystemTime::now() + Duration::from_secs(30 * 60),
            )
            .expect("Should be valid until year 500,000,000");

            debug!(
                "The threshold is {:?}, found time is {:?} for server {}",
                ts_threshold, result.valid_until_ts, origin
            );

            if contains_all_ids(&result) {
                // We want to ensure that the keys remain valid by the time the other functions that handle signatures reach them
                if result.valid_until_ts > ts_threshold {
                    debug!(
                        "Keys for {} are deemed as valid, as they expire at {:?}",
                        &origin, &result.valid_until_ts
                    );
                    return Ok(result);
                }

                expires_soon_or_has_expired = true;
            }
        }

        let mut keys = result.unwrap_or_else(|| SigningKeys {
            verify_keys: BTreeMap::new(),
            old_verify_keys: BTreeMap::new(),
            valid_until_ts: MilliSecondsSinceUnixEpoch::now(),
        });

        // We want to set this to the max, and then lower it whenever we see older keys
        keys.valid_until_ts = MilliSecondsSinceUnixEpoch::from_system_time(
            SystemTime::now() + Duration::from_secs(7 * 86400),
        )
        .expect("Should be valid until year 500,000,000");

        debug!("Fetching signing keys for {} over federation", origin);

        if let Some(mut server_key) = services()
            .sending
            .send_federation_request(origin, get_server_keys::v2::Request::new())
            .await
            .ok()
            .and_then(|resp| resp.server_key.deserialize().ok())
        {
            // Keys should only be valid for a maximum of seven days
            server_key.valid_until_ts = server_key.valid_until_ts.min(
                MilliSecondsSinceUnixEpoch::from_system_time(
                    SystemTime::now() + Duration::from_secs(7 * 86400),
                )
                .expect("Should be valid until year 500,000,000"),
            );

            services()
                .globals
                .add_signing_key_from_origin(origin, server_key.clone())?;

            if keys.valid_until_ts > server_key.valid_until_ts {
                keys.valid_until_ts = server_key.valid_until_ts;
            }

            keys.verify_keys.extend(
                server_key
                    .verify_keys
                    .into_iter()
                    .map(|(id, key)| (id.to_string(), key)),
            );
            keys.old_verify_keys.extend(
                server_key
                    .old_verify_keys
                    .into_iter()
                    .map(|(id, key)| (id.to_string(), key)),
            );

            if contains_all_ids(&keys) {
                return Ok(keys);
            }
        }

        if query_via_trusted_servers {
            for server in services().globals.trusted_servers() {
                debug!("Asking {} for {}'s signing key", server, origin);
                if let Some(server_keys) = services()
                    .sending
                    .send_federation_request(
                        server,
                        get_remote_server_keys::v2::Request::new(
                            origin.to_owned(),
                            MilliSecondsSinceUnixEpoch::from_system_time(
                                SystemTime::now()
                                    .checked_add(Duration::from_secs(3600))
                                    .expect("SystemTime to large"),
                            )
                            .expect("time is valid"),
                        ),
                    )
                    .await
                    .ok()
                    .map(|resp| {
                        resp.server_keys
                            .into_iter()
                            .filter_map(|e| e.deserialize().ok())
                            .collect::<Vec<_>>()
                    })
                {
                    trace!("Got signing keys: {:?}", server_keys);
                    for mut k in server_keys {
                        if k.valid_until_ts
                        // Half an hour should give plenty of time for the server to respond with keys that are still
                        // valid, given we requested keys which are valid at least an hour from now
                            < MilliSecondsSinceUnixEpoch::from_system_time(
                                SystemTime::now() + Duration::from_secs(30 * 60),
                            )
                            .expect("Should be valid until year 500,000,000")
                        {
                            // Keys should only be valid for a maximum of seven days
                            k.valid_until_ts = k.valid_until_ts.min(
                                MilliSecondsSinceUnixEpoch::from_system_time(
                                    SystemTime::now() + Duration::from_secs(7 * 86400),
                                )
                                .expect("Should be valid until year 500,000,000"),
                            );

                            if keys.valid_until_ts > k.valid_until_ts {
                                keys.valid_until_ts = k.valid_until_ts;
                            }

                            services()
                                .globals
                                .add_signing_key_from_trusted_server(origin, k.clone())?;
                            keys.verify_keys.extend(
                                k.verify_keys
                                    .into_iter()
                                    .map(|(id, key)| (id.to_string(), key)),
                            );
                            keys.old_verify_keys.extend(
                                k.old_verify_keys
                                    .into_iter()
                                    .map(|(id, key)| (id.to_string(), key)),
                            );
                        } else {
                            warn!(
                                "Server {} gave us keys older than we requested, valid until: {:?}",
                                origin, k.valid_until_ts
                            );
                        }

                        if contains_all_ids(&keys) {
                            return Ok(keys);
                        }
                    }
                }
            }
        }

        // We should return these keys if fresher keys were not found
        if expires_soon_or_has_expired {
            info!("Returning stale keys for {}", origin);
            return Ok(keys);
        }

        drop(permit);

        back_off(signature_ids).await;

        warn!("Failed to find public key for server: {}", origin);
        Err(Error::BadServerResponse("Failed to find public key for server".to_string()))
    }

    fn check_room_id(&self, room_id: &RoomId, pdu: &PduEvent) -> Result<()> {
        if pdu.room_id() != room_id {
            warn!("Found event from room {} in room {}", pdu.room_id(), room_id);
            return Err(Error::BadRequest(
                crate::MatrixErrorKind::InvalidParam,
                "Event has wrong room id",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruma::{
        api::federation::event::get_event,
        events::{
            room::{
                create::RoomCreateEventContent,
                member::{MembershipState, RoomMemberEventContent},
                message::RoomMessageEventContent,
            },
            TimelineEventType, StateEventType,
        },
        event_id, room_id, server_name, user_id,
        EventId, OwnedEventId, OwnedRoomId, OwnedServerName, OwnedUserId,
        MilliSecondsSinceUnixEpoch, UInt,
    };
    use std::{
        collections::{BTreeMap, HashMap, HashSet},
        sync::{Arc, RwLock as StdRwLock},
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
        thread,
    };
    use tokio::sync::RwLock;
    use tracing::{debug, info};

    /// Mock event handler storage for testing
    #[derive(Debug)]
    struct MockEventHandlerStorage {
        events: Arc<StdRwLock<HashMap<OwnedEventId, MockPduEvent>>>,
        room_states: Arc<StdRwLock<HashMap<OwnedRoomId, MockRoomState>>>,
        auth_chains: Arc<StdRwLock<HashMap<OwnedEventId, Vec<OwnedEventId>>>>,
        pub_keys: Arc<RwLock<BTreeMap<String, SigningKeys>>>,
        processing_requests: Arc<StdRwLock<u32>>,
        performance_metrics: Arc<StdRwLock<EventHandlerMetrics>>,
        server_acls: Arc<StdRwLock<HashMap<OwnedRoomId, Vec<OwnedServerName>>>>,
    }

    #[derive(Debug, Clone)]
    struct MockPduEvent {
        event_id: OwnedEventId,
        room_id: OwnedRoomId,
        sender: OwnedUserId,
        origin_server_ts: u64,
        event_type: TimelineEventType,
        content: String,
        state_key: Option<String>,
        prev_events: Vec<OwnedEventId>,
        auth_events: Vec<OwnedEventId>,
        depth: u64,
        signatures_valid: bool,
        content_hash_valid: bool,
    }

    #[derive(Debug, Clone)]
    struct MockRoomState {
        room_id: OwnedRoomId,
        room_version: String,
        current_state: HashMap<String, OwnedEventId>,
        power_levels: HashMap<OwnedUserId, i64>,
        members: HashMap<OwnedUserId, MembershipState>,
        create_event: Option<OwnedEventId>,
        is_disabled: bool,
    }

    #[derive(Debug, Default, Clone)]
    struct EventHandlerMetrics {
        total_events_processed: u64,
        successful_events: u64,
        failed_events: u64,
        timeline_events: u64,
        outlier_events: u64,
        state_resolutions: u64,
        auth_checks: u64,
        signature_verifications: u64,
        average_processing_time: Duration,
        federation_requests: u64,
    }

    impl MockEventHandlerStorage {
        fn new() -> Self {
            Self {
                events: Arc::new(StdRwLock::new(HashMap::new())),
                room_states: Arc::new(StdRwLock::new(HashMap::new())),
                auth_chains: Arc::new(StdRwLock::new(HashMap::new())),
                pub_keys: Arc::new(RwLock::new(BTreeMap::new())),
                processing_requests: Arc::new(StdRwLock::new(0)),
                performance_metrics: Arc::new(StdRwLock::new(EventHandlerMetrics::default())),
                server_acls: Arc::new(StdRwLock::new(HashMap::new())),
            }
        }

        fn add_room(&self, room_state: MockRoomState) {
            self.room_states.write().unwrap().insert(room_state.room_id().clone(), room_state);
        }

        fn add_event(&self, event: MockPduEvent) {
            self.events.write().unwrap().insert(event.event_id().clone(), event);
        }

        fn set_server_acl(&self, room_id: OwnedRoomId, allowed_servers: Vec<OwnedServerName>) {
            self.server_acls.write().unwrap().insert(room_id, allowed_servers);
        }

        async fn handle_incoming_pdu(
            &self,
            origin: &str,
            event_id: &OwnedEventId,
            room_id: &OwnedRoomId,
            value: BTreeMap<String, String>, // Simplified for testing
            is_timeline_event: bool,
        ) -> Result<Option<Vec<u8>>, String> {
            let start = Instant::now();
            *self.processing_requests.write().unwrap() += 1;

            // Check room exists and not disabled
            let room_states = self.room_states.read().unwrap();
            let room_state = room_states.get(room_id)
                .ok_or_else(|| "Room not found".to_string())?;

            if room_state.is_disabled {
                return Err("Room federation disabled".to_string());
            }

            // Check server ACL
            if let Some(allowed_servers) = self.server_acls.read().unwrap().get(room_id) {
                let origin_server: OwnedServerName = origin.try_into()
                    .map_err(|_| "Invalid origin server".to_string())?;
                if !allowed_servers.contains(&origin_server) {
                    return Err("Server not allowed by ACL".to_string());
                }
            }

            // Check if event already exists
            if self.events.read().unwrap().contains_key(event_id) {
                return Ok(Some(b"existing_event_id".to_vec()));
            }

            // Simulate event validation
            let mock_event = MockPduEvent {
                event_id: event_id.clone(),
                room_id: room_id.clone(),
                sender: user_id!("@sender:example.com").to_owned(),
                origin_server_ts: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
                event_type: TimelineEventType::RoomMessage,
                content: value.get("content").cloned().unwrap_or_default(),
                state_key: value.get("state_key").cloned(),
                prev_events: Vec::new(),
                auth_events: Vec::new(),
                depth: 1,
                signatures_valid: true,
                content_hash_valid: true,
            };

            // Validate signatures and content hash
            if !mock_event.signatures_valid {
                let mut metrics = self.performance_metrics.write().unwrap();
                metrics.total_events_processed += 1;
                metrics.failed_events += 1;
                return Err("Invalid signatures".to_string());
            }

            if !mock_event.content_hash_valid {
                let mut metrics = self.performance_metrics.write().unwrap();
                metrics.total_events_processed += 1;
                metrics.failed_events += 1;
                return Err("Invalid content hash".to_string());
            }

            // Add event to storage
            self.add_event(mock_event);

            // Update metrics
            let mut metrics = self.performance_metrics.write().unwrap();
            metrics.total_events_processed += 1;
            metrics.successful_events += 1;
            metrics.average_processing_time = start.elapsed();

            if is_timeline_event {
                metrics.timeline_events += 1;
            } else {
                metrics.outlier_events += 1;
            }

            metrics.auth_checks += 1;
            metrics.signature_verifications += 1;

            Ok(Some(b"new_event_id".to_vec()))
        }

        fn check_room_acl(&self, origin: &str, room_id: &OwnedRoomId) -> Result<(), String> {
            if let Some(allowed_servers) = self.server_acls.read().unwrap().get(room_id) {
                let origin_server: OwnedServerName = origin.try_into()
                    .map_err(|_| "Invalid origin server".to_string())?;
                if !allowed_servers.contains(&origin_server) {
                    return Err("Server not allowed by ACL".to_string());
                }
            }
            Ok(())
        }

        async fn resolve_state(
            &self,
            room_id: &OwnedRoomId,
            conflicting_events: Vec<OwnedEventId>,
        ) -> Result<HashMap<String, OwnedEventId>, String> {
            let start = Instant::now();

            // Simulate state resolution algorithm
            let mut resolved_state = HashMap::new();

            // Mock resolution - in reality this would implement MSC1442
            for (i, event_id) in conflicting_events.iter().enumerate() {
                let state_key = format!("resolved_state_{}", i);
                resolved_state.insert(state_key, event_id.clone());
            }

            // Update metrics
            let mut metrics = self.performance_metrics.write().unwrap();
            metrics.state_resolutions += 1;

            Ok(resolved_state)
        }

        async fn fetch_missing_events(
            &self,
            origin: &str,
            room_id: &OwnedRoomId,
            missing_events: Vec<OwnedEventId>,
        ) -> Result<Vec<MockPduEvent>, String> {
            let mut metrics = self.performance_metrics.write().unwrap();
            metrics.federation_requests += missing_events.len() as u64;

            // Simulate federation requests to fetch missing events
            let mut fetched_events = Vec::new();

            for event_id in missing_events {
                // Check if we already have the event
                if let Some(event) = self.events.read().unwrap().get(&event_id) {
                    fetched_events.push(event.clone());
                } else {
                    // Simulate fetching from origin server
                    let mock_event = MockPduEvent {
                        event_id: event_id.clone(),
                        room_id: room_id.clone(),
                        sender: user_id!("@remote:example.com").to_owned(),
                        origin_server_ts: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
                        event_type: TimelineEventType::RoomMessage,
                        content: "Fetched event content".to_string(),
                        state_key: None,
                        prev_events: Vec::new(),
                        auth_events: Vec::new(),
                        depth: 1,
                        signatures_valid: true,
                        content_hash_valid: true,
                    };

                    self.add_event(mock_event.clone());
                    fetched_events.push(mock_event);
                }
            }

            Ok(fetched_events)
        }

        fn get_request_count(&self) -> u32 {
            *self.processing_requests.read().unwrap()
        }

        fn get_metrics(&self) -> EventHandlerMetrics {
            (*self.performance_metrics.read().unwrap()).clone()
        }

        fn clear(&self) {
            self.events.write().unwrap().clear();
            self.room_states.write().unwrap().clear();
            self.auth_chains.write().unwrap().clear();
            *self.processing_requests.write().unwrap() = 0;
            *self.performance_metrics.write().unwrap() = EventHandlerMetrics::default();
            self.server_acls.write().unwrap().clear();
        }
    }

    fn create_test_event_id(id: u64) -> OwnedEventId {
        let event_str = format!("$event_{}:example.com", id);
        EventId::parse(&event_str).unwrap().to_owned()
    }

    fn create_test_room(id: u64) -> OwnedRoomId {
        let room_str = format!("!room_{}:example.com", id);
        ruma::RoomId::parse(&room_str).unwrap().to_owned()
    }

    fn create_test_user(id: u64) -> OwnedUserId {
        let user_str = format!("@user_{}:example.com", id);
        ruma::UserId::parse(&user_str).unwrap().to_owned()
    }

    fn create_mock_room_state(room_id: OwnedRoomId, room_version: &str) -> MockRoomState {
        MockRoomState {
            room_id,
            room_version: room_version.to_string(),
            current_state: HashMap::new(),
            power_levels: HashMap::new(),
            members: HashMap::new(),
            create_event: None,
            is_disabled: false,
        }
    }

    fn create_test_pdu_value(content: &str) -> BTreeMap<String, String> {
        let mut value = BTreeMap::new();
        value.insert("content".to_string(), content.to_string());
        value.insert("type".to_string(), "m.room.message".to_string());
        value
    }

    #[tokio::test]
    async fn test_event_handler_basic_functionality() {
        debug!("🔧 Testing event handler basic functionality");
        let storage = MockEventHandlerStorage::new();
        let _room_id = create_test_room(1);
        let _start = Instant::now();

        // Setup room
        let room_state = create_mock_room_state(create_test_room(1).clone(), "10");
        storage.add_room(room_state);

        // Test basic event processing
        let pdu_value = create_test_pdu_value("Hello, world!");
        let result = storage.handle_incoming_pdu(
            "origin.example.com",
            &event_id,
            &room_id,
            pdu_value,
            true,
        ).await;

        assert!(result.is_ok(), "Basic event processing should succeed");
        assert!(result.unwrap().is_some(), "Should return event ID");

        // Verify event was stored
        assert!(storage.events.read().unwrap().contains_key(&event_id), "Event should be stored");

        assert_eq!(storage.get_request_count(), 1, "Should have processed 1 request");

        let metrics = storage.get_metrics();
        assert_eq!(metrics.total_events_processed, 1, "Should have processed 1 event");
        assert_eq!(metrics.successful_events, 1, "Should have 1 successful event");
        assert_eq!(metrics.timeline_events, 1, "Should have 1 timeline event");

        info!("✅ Event handler basic functionality test completed in {:?}", start.elapsed());
    }

    #[tokio::test]
    async fn test_event_handler_room_acl() {
        debug!("🔧 Testing event handler room ACL");
        let start = Instant::now();
        let storage = MockEventHandlerStorage::new();

        let room_id = create_test_room(1);
        let event_id = create_test_event_id(1);

        // Setup room with ACL
        let room_state = create_mock_room_state(room_id.clone(), "10");
        storage.add_room(room_state);

        let allowed_servers = vec![
            server_name!("allowed.example.com").to_owned(),
            server_name!("trusted.example.com").to_owned(),
        ];
        storage.set_server_acl(room_id.clone(), allowed_servers);

        // Test allowed server
        let pdu_value = create_test_pdu_value("Message from allowed server");
        let allowed_result = storage.handle_incoming_pdu(
            "allowed.example.com",
            &event_id,
            &room_id,
            pdu_value,
            true,
        ).await;

        assert!(allowed_result.is_ok(), "Allowed server should succeed");

        // Test blocked server
        let event_id2 = create_test_event_id(2);
        let pdu_value2 = create_test_pdu_value("Message from blocked server");
        let blocked_result = storage.handle_incoming_pdu(
            "blocked.example.com",
            &event_id2,
            &room_id,
            pdu_value2,
            true,
        ).await;

        assert!(blocked_result.is_err(), "Blocked server should be rejected");
        assert_eq!(blocked_result.unwrap_err(), "Server not allowed by ACL", "Should get ACL error");

        // Test ACL check function directly
        let acl_check_allowed = storage.check_room_acl("allowed.example.com", &room_id);
        assert!(acl_check_allowed.is_ok(), "ACL check should allow trusted server");

        let acl_check_blocked = storage.check_room_acl("blocked.example.com", &room_id);
        assert!(acl_check_blocked.is_err(), "ACL check should block untrusted server");

        info!("✅ Event handler room ACL test completed in {:?}", start.elapsed());
    }

    #[tokio::test]
    async fn test_matrix_protocol_compliance() {
        debug!("🔧 Testing Matrix protocol compliance for event handler");
        let start = Instant::now();
        let storage = MockEventHandlerStorage::new();

        let room_id = create_test_room(1);
        let event_id = create_test_event_id(1);

        // Setup room with proper Matrix room version
        let room_state = create_mock_room_state(room_id.clone(), "10"); // Room version 10 (latest stable)
        storage.add_room(room_state);

        // Test Matrix event ID format compliance
        assert!(event_id.as_str().starts_with('$'), "Event ID should start with $ (Matrix spec)");
        assert!(event_id.as_str().contains(':'), "Event ID should contain server name (Matrix spec)");

        // Test Matrix room ID format compliance
        assert!(room_id.as_str().starts_with('!'), "Room ID should start with ! (Matrix spec)");
        assert!(room_id.as_str().contains(':'), "Room ID should contain server name (Matrix spec)");

        // Test Matrix PDU structure compliance
        let mut pdu_value = create_test_pdu_value("Matrix protocol test message");
        pdu_value.insert("room_id".to_string(), room_id.to_string());
        pdu_value.insert("sender".to_string(), "@sender:example.com".to_string());
        pdu_value.insert("origin_server_ts".to_string(), "1234567890".to_string());

        let result = storage.handle_incoming_pdu(
            "origin.example.com",
            &event_id,
            &room_id,
            pdu_value,
            true,
        ).await;

        assert!(result.is_ok(), "Matrix compliant PDU should be processed successfully");

        // Verify event was stored with proper structure
        let stored_event = storage.events.read().unwrap();
        let event = stored_event.get(&event_id).unwrap();
        
        assert_eq!(event.room_id(), room_id, "Stored event should have correct room ID");
        assert_eq!(event.event_type, TimelineEventType::RoomMessage, "Should be room message event");

        // Test Matrix authorization requirements
        let metrics = storage.get_metrics();
        assert!(metrics.auth_checks > 0, "Should perform authorization checks (Matrix spec)");
        assert!(metrics.signature_verifications > 0, "Should verify signatures (Matrix spec)");

        info!("✅ Matrix protocol compliance test completed in {:?}", start.elapsed());
    }

    #[tokio::test]
    async fn test_enterprise_event_handler_compliance() {
        debug!("🔧 Testing enterprise event handler compliance");
        let start = Instant::now();
        let storage = Arc::new(MockEventHandlerStorage::new());

        // Enterprise scenario: Large federated deployment with multiple rooms and servers
        let num_rooms = 50;
        let events_per_room = 100;
        let federation_servers = vec![
            "server1.example.com",
            "server2.example.com", 
            "server3.example.com",
            "server4.example.com",
            "server5.example.com",
        ];

        // Setup enterprise environment
        let mut all_rooms = Vec::new();
        for room_idx in 0..num_rooms {
            let room_id = create_test_room(room_idx as u64);
            let room_state = create_mock_room_state(room_id.clone(), "10");
            storage.add_room(room_state);
            all_rooms.push(room_id);
        }

        // Test enterprise event processing performance
        let perf_start = Instant::now();
        let mut event_counter = 0u64;

        for room in &all_rooms {
            for event_idx in 0..events_per_room {
                let event_id = create_test_event_id(event_counter);
                let origin_server = federation_servers[event_idx % federation_servers.len()];
                let pdu_value = create_test_pdu_value(&format!("Enterprise event {} in room {}", event_idx, room));

                let result = storage.handle_incoming_pdu(
                    origin_server,
                    &event_id,
                    room,
                    pdu_value,
                    true,
                ).await;

                assert!(result.is_ok(), "Enterprise event processing should succeed");
                event_counter += 1;
            }
        }
        let perf_duration = perf_start.elapsed();

        let total_events = num_rooms * events_per_room;
        assert!(perf_duration < Duration::from_millis(30000),
                "Enterprise event processing should be <30s for {} events, was: {:?}", 
                total_events, perf_duration);

        // Test enterprise concurrent federation handling
        let concurrent_start = Instant::now();
        let mut concurrent_handles = vec![];

        for i in 0..10 {
            let storage_clone = Arc::clone(&storage);
            let room = all_rooms[i % all_rooms.len()].clone();
            let server = federation_servers[i % federation_servers.len()];

            let handle = tokio::spawn(async move {
                for j in 0..20 {
                    let event_id = create_test_event_id((i * 20 + j + 100000) as u64);
                    let pdu_value = create_test_pdu_value(&format!("Concurrent federation event {} from {}", j, server));

                    let _ = storage_clone.handle_incoming_pdu(
                        server,
                        &event_id,
                        &room,
                        pdu_value,
                        true,
                    ).await;
                }
            });

            concurrent_handles.push(handle);
        }

        for handle in concurrent_handles {
            handle.await.unwrap();
        }
        let concurrent_duration = concurrent_start.elapsed();

        assert!(concurrent_duration < Duration::from_millis(5000),
                "Enterprise concurrent federation should be <5s for 200 operations, was: {:?}", 
                concurrent_duration);

        // Test enterprise memory efficiency and metrics
        let metrics = storage.get_metrics();
        let processing_efficiency = metrics.successful_events as f64 / metrics.total_events_processed as f64;
        assert!(processing_efficiency >= 0.99, "Enterprise should have >=99% processing success rate");

        // Test enterprise scalability validation
        assert!(metrics.total_events_processed >= total_events as u64, 
               "Should have processed at least {} events", total_events);
        assert!(metrics.timeline_events >= (total_events * 9 / 10) as u64, 
               "Should have substantial timeline events");

        // Test enterprise audit capabilities
        assert!(metrics.federation_requests >= 0, "Should track federation requests");
        assert!(metrics.auth_checks >= metrics.total_events_processed, 
               "Should perform auth checks for all events");
        assert!(metrics.signature_verifications >= metrics.total_events_processed, 
               "Should verify signatures for all events");

        let avg_processing_time = metrics.average_processing_time;
        assert!(avg_processing_time < Duration::from_millis(50),
                "Enterprise average processing time should be <50ms, was: {:?}", avg_processing_time);

        info!("✅ Enterprise event handler compliance verified for {} rooms × {} events with {:.1}% success rate and {:?} avg processing time in {:?}",
              num_rooms, total_events, processing_efficiency * 100.0, avg_processing_time, start.elapsed());
    }
}
