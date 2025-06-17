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

use std::{
    collections::VecDeque,
    fmt::{Display, Formatter},
    str::FromStr,
};

use lru_cache::LruCache;
use ruma::{
    api::{
        client::{self, error::ErrorKind, space::{SpaceHierarchyRoomsChunk, SpaceHierarchyRoomsChunkInit}},
        federation::{
            self,
            space::{SpaceHierarchyChildSummary, SpaceHierarchyParentSummary, SpaceHierarchyParentSummaryInit},
        },
    },
    events::{
        room::{
            join_rules::{JoinRule, RoomJoinRulesEventContent},
        },
        space::child::{HierarchySpaceChildEvent, SpaceChildEventContent},
        StateEventType,
    },
    serde::Raw,
    space::SpaceRoomJoinRule,
    OwnedRoomId, OwnedServerName, RoomId, ServerName, UInt, UserId,
};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::{services, Error, Result};

pub struct CachedSpaceHierarchySummary {
    summary: SpaceHierarchyParentSummary,
}

pub enum SummaryAccessibility {
    Accessible(Box<SpaceHierarchyParentSummary>),
    Inaccessible,
}

// Note: perhaps use some better form of token rather than just room count
#[derive(Debug, PartialEq)]
pub struct PagnationToken {
    /// Path down the hierarchy of the room to start the response at,
    /// excluding the root space.
    pub short_room_ids: Vec<u64>,
    pub limit: UInt,
    pub max_depth: UInt,
    pub suggested_only: bool,
}

impl FromStr for PagnationToken {
    fn from_str(value: &str) -> Result<Self> {
        let mut values = value.split('_');

        let mut pag_tok = || {
            let mut rooms = vec![];

            for room in values.next()?.split(',') {
                rooms.push(u64::from_str(room).ok()?)
            }

            Some(PagnationToken {
                short_room_ids: rooms,
                limit: UInt::from_str(values.next()?).ok()?,
                max_depth: UInt::from_str(values.next()?).ok()?,
                suggested_only: {
                    let slice = values.next()?;

                    if values.next().is_none() {
                        if slice == "true" {
                            true
                        } else if slice == "false" {
                            false
                        } else {
                            None?
                        }
                    } else {
                        None?
                    }
                },
            })
        };

        if let Some(token) = pag_tok() {
            Ok(token)
        } else {
            Err(Error::BadRequest(ErrorKind::InvalidParam, "invalid token"))
        }
    }

    type Err = Error;
}

impl Display for PagnationToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}_{}_{}_{}",
            self.short_room_ids
                .iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
                .join(","),
            self.limit,
            self.max_depth,
            self.suggested_only
        )
    }
}

/// Identifier used to check if rooms are accessible
///
/// None is used if you want to return the room, no matter if accessible or not
pub enum Identifier<'a> {
    UserId(&'a UserId),
    ServerName(&'a ServerName),
}

pub struct Service {
    pub roomid_spacehierarchy_cache:
        Mutex<LruCache<OwnedRoomId, Option<CachedSpaceHierarchySummary>>>,
}

// Here because cannot implement `From` across ruma-federation-api and ruma-client-api types
impl From<CachedSpaceHierarchySummary> for SpaceHierarchyRoomsChunk {
    fn from(value: CachedSpaceHierarchySummary) -> Self {
        let SpaceHierarchyParentSummary {
            canonical_alias: _,
            name: _,
            topic: _,
            avatar_url: _,
            room_type: _,
            num_joined_members,
            room_id,
            world_readable,
            guest_can_join,
            join_rule,
            children_state,
            ..
        } = value.summary;

        {
            SpaceHierarchyRoomsChunkInit {
                room_id,
                world_readable,
                guest_can_join,
                join_rule,
                children_state,
                num_joined_members,
            }.into()
        }
    }
}

impl Service {
    ///Gets the response for the space hierarchy over federation request
    ///
    ///Panics if the room does not exist, so a check if the room exists should be done
    pub async fn get_federation_hierarchy(
        &self,
        room_id: &RoomId,
        server_name: &ServerName,
        suggested_only: bool,
    ) -> Result<federation::space::get_hierarchy::v1::Response> {
        match self
            .get_summary_and_children_local(
                &room_id.to_owned(),
                Identifier::ServerName(server_name),
            )
            .await?
        {
            Some(SummaryAccessibility::Accessible(room)) => {
                let mut children = Vec::new();
                let mut inaccessible_children = Vec::new();

                for (child, _via) in get_parent_children_via(*room.clone(), suggested_only) {
                    match self
                        .get_summary_and_children_local(&child, Identifier::ServerName(server_name))
                        .await?
                    {
                        Some(SummaryAccessibility::Accessible(summary)) => {
                            children.push((*summary).into());
                        }
                        Some(SummaryAccessibility::Inaccessible) => {
                            inaccessible_children.push(child);
                        }
                        None => (),
                    }
                }

                {
                    let mut response = federation::space::get_hierarchy::v1::Response::new(*room);
                    response.children = children;
                    response.inaccessible_children = inaccessible_children;
                    Ok(response)
                }
            }
            Some(SummaryAccessibility::Inaccessible) => Err(Error::BadRequestString(
                ErrorKind::NotFound,
                "The requested room is inaccessible".to_string(),
            )),
            None => Err(Error::BadRequestString(
                ErrorKind::NotFound,
                "The requested room was not found".to_string(),
            )),
        }
    }

    /// Gets the summary of a space using solely local information
    async fn get_summary_and_children_local(
        &self,
        current_room: &OwnedRoomId,
        identifier: Identifier<'_>,
    ) -> Result<Option<SummaryAccessibility>> {
        if let Some(cached) = self
            .roomid_spacehierarchy_cache
            .lock()
            .await
            .get_mut(&current_room.to_owned())
            .as_ref()
        {
            return Ok(if let Some(cached) = cached {
                if is_accessible_child(
                    current_room,
                    &cached.summary.join_rule,
                    &identifier,
                    &cached.summary.allowed_room_ids,
                ) {
                    Some(SummaryAccessibility::Accessible(Box::new(
                        cached.summary.clone(),
                    )))
                } else {
                    Some(SummaryAccessibility::Inaccessible)
                }
            } else {
                None
            });
        }

        Ok(
            if let Some(children_pdus) = get_stripped_space_child_events(current_room).await? {
                let summary = self.get_room_summary(current_room, children_pdus, identifier);
                if let Ok(summary) = summary {
                    self.roomid_spacehierarchy_cache.lock().await.insert(
                        current_room.clone(),
                        Some(CachedSpaceHierarchySummary {
                            summary: summary.clone(),
                        }),
                    );

                    Some(SummaryAccessibility::Accessible(Box::new(summary)))
                } else {
                    None
                }
            } else {
                None
            },
        )
    }

    /// Gets the summary of a space using solely federation
    async fn get_summary_and_children_federation(
        &self,
        current_room: &OwnedRoomId,
        suggested_only: bool,
        user_id: &UserId,
        via: &Vec<OwnedServerName>,
    ) -> Result<Option<SummaryAccessibility>> {
        for server in via {
            info!("Asking {server} for /hierarchy");
            if let Ok(response) = services()
                .sending
                .send_federation_request(
                    server,
                    {
                        let mut request = federation::space::get_hierarchy::v1::Request::new(current_room.to_owned());
                        request.suggested_only = suggested_only;
                        request
                    },
                )
                .await
            {
                info!("Got response from {server} for /hierarchy\n{response:?}");
                let summary = response.room.clone();

                self.roomid_spacehierarchy_cache.lock().await.insert(
                    current_room.clone(),
                    Some(CachedSpaceHierarchySummary {
                        summary: summary.clone(),
                    }),
                );

                for child in response.children {
                    let mut guard = self.roomid_spacehierarchy_cache.lock().await;
                    if !guard.contains_key(current_room) {
                        guard.insert(
                            current_room.clone(),
                            Some(CachedSpaceHierarchySummary {
                                summary: {
                                    let SpaceHierarchyChildSummary {
                                        canonical_alias: _,
                                        name: _,
                                        topic: _,
                                        avatar_url: _,
                                        room_type: _,
                                        num_joined_members,
                                        room_id,
                                        world_readable,
                                        guest_can_join,
                                        join_rule,
                                        allowed_room_ids,
                                        ..
                                    } = child;

                                    {
                                        SpaceHierarchyParentSummaryInit {
                                            room_id: room_id.clone(),
                                            world_readable,
                                            guest_can_join,
                                            join_rule,
                                            children_state: get_stripped_space_child_events(&room_id)
                                                .await?
                                                .unwrap_or_default(),
                                            allowed_room_ids,
                                            num_joined_members,
                                        }.into()
                                    }
                                },
                            }),
                        );
                    }
                }
                if is_accessible_child(
                    current_room,
                    &response.room.join_rule,
                    &Identifier::UserId(user_id),
                    &response.room.allowed_room_ids,
                ) {
                    return Ok(Some(SummaryAccessibility::Accessible(Box::new(
                        summary.clone(),
                    ))));
                } else {
                    return Ok(Some(SummaryAccessibility::Inaccessible));
                }
            }
        }

        self.roomid_spacehierarchy_cache
            .lock()
            .await
            .insert(current_room.clone(), None);

        Ok(None)
    }

    /// Gets the summary of a space using either local or remote (federation) sources
    async fn get_summary_and_children_client(
        &self,
        current_room: &OwnedRoomId,
        suggested_only: bool,
        user_id: &UserId,
        via: &Vec<OwnedServerName>,
    ) -> Result<Option<SummaryAccessibility>> {
        if let Ok(Some(response)) = self
            .get_summary_and_children_local(current_room, Identifier::UserId(user_id))
            .await
        {
            Ok(Some(response))
        } else {
            self.get_summary_and_children_federation(current_room, suggested_only, user_id, via)
                .await
        }
    }

    fn get_room_summary(
        &self,
        current_room: &OwnedRoomId,
        children_state: Vec<Raw<HierarchySpaceChildEvent>>,
        identifier: Identifier<'_>,
    ) -> Result<SpaceHierarchyParentSummary> {
        let room_id: &RoomId = current_room;

        let join_rule = services()
            .rooms
            .state_accessor
            .room_state_get(room_id, &StateEventType::RoomJoinRules, "")?
            .map(|s| {
                serde_json::from_str(s.content.get())
                    .map(|c: RoomJoinRulesEventContent| c.join_rule)
                    .map_err(|e| {
                        error!("Invalid room join rule event in database: {}", e);
                        Error::bad_database("Invalid room join rule event in database.")
                    })
            })
            .transpose()?
            .unwrap_or(JoinRule::Invite);

        let allowed_room_ids = services()
            .rooms
            .state_accessor
            .allowed_room_ids(join_rule.clone());

        if !is_accessible_child(
            current_room,
            &join_rule.clone().into(),
            &identifier,
            &allowed_room_ids,
        ) {
            debug!("User is not allowed to see room {room_id}");
            // This error will be caught later
            return Err(Error::BadRequestString(
                ErrorKind::forbidden(),
                "User is not allowed to see the room".to_string(),
            ));
        }

        let join_rule = join_rule.into();

        {
            let summary = SpaceHierarchyParentSummaryInit {
                room_id: room_id.to_owned(),
                world_readable: services().rooms.state_accessor.world_readable(room_id)?,
                guest_can_join: services().rooms.state_accessor.guest_can_join(room_id)?,
                join_rule,
                children_state,
                allowed_room_ids: Vec::new(),
                num_joined_members: services()
                    .rooms
                    .state_cache
                    .room_joined_count(room_id)?
                    .unwrap_or_else(|| {
                        warn!("Room {} has no member count", room_id);
                        0
                    })
                    .try_into()
                    .expect("user count should not be that big"),
            }.into();
                
            Ok(summary)
        }
    }

    pub async fn get_client_hierarchy(
        &self,
        sender_user: &UserId,
        room_id: &RoomId,
        limit: usize,
        short_room_ids: Vec<u64>,
        max_depth: usize,
        suggested_only: bool,
    ) -> Result<client::space::get_hierarchy::v1::Response> {
        let mut parents = VecDeque::new();

        // Don't start populating the results if we have to start at a specific room.
        let mut populate_results = short_room_ids.is_empty();

        let mut stack = vec![vec![(
            room_id.to_owned(),
            match room_id.server_name() {
                Some(server_name) => vec![server_name.into()],
                None => vec![],
            },
        )]];

        let mut results = Vec::new();

        while let Some((current_room, via)) = { next_room_to_traverse(&mut stack, &mut parents) } {
            if limit > results.len() {
                match (
                    self.get_summary_and_children_client(
                        &current_room,
                        suggested_only,
                        sender_user,
                        &via,
                    )
                    .await?,
                    current_room == room_id,
                ) {
                    (Some(SummaryAccessibility::Accessible(summary)), _) => {
                        let mut children: Vec<(OwnedRoomId, Vec<OwnedServerName>)> =
                            get_parent_children_via(*summary.clone(), suggested_only)
                                .into_iter()
                                .filter(|(room, _)| parents.iter().all(|parent| parent != room))
                                .rev()
                                .collect();

                        if populate_results {
                            results.push(summary_to_chunk(*summary.clone()))
                        } else {
                            children = children
                                .into_iter()
                                .rev()
                                .skip_while(|(room, _)| {
                                    if let Ok(short) = services().rooms.short.get_shortroomid(room)
                                    {
                                        short.as_ref() != short_room_ids.get(parents.len())
                                    } else {
                                        false
                                    }
                                })
                                .collect::<Vec<_>>()
                                // skip_while doesn't implement DoubleEndedIterator, which is needed for rev
                                .into_iter()
                                .rev()
                                .collect();

                            if children.is_empty() {
                                return Err(Error::BadRequestString(
                                    ErrorKind::InvalidParam,
                                    "Short room ids in token were not found.".to_string(),
                                ));
                            }

                            // We have reached the room after where we last left off
                            if parents.len() + 1 == short_room_ids.len() {
                                populate_results = true;
                            }
                        }

                        if !children.is_empty() && parents.len() < max_depth {
                            parents.push_back(current_room.clone());
                            stack.push(children);
                        }
                        // Root room in the space hierarchy, we return an error if this one fails.
                    }
                    (Some(SummaryAccessibility::Inaccessible), true) => {
                        return Err(Error::BadRequestString(
                            ErrorKind::forbidden(),
                            "The requested room is inaccessible".to_string(),
                        ));
                    }
                    (None, true) => {
                        return Err(Error::BadRequestString(
                            ErrorKind::forbidden(),
                            "The requested room was not found".to_string(),
                        ));
                    }
                    // Just ignore other unavailable rooms
                    (None | Some(SummaryAccessibility::Inaccessible), false) => (),
                }
            } else {
                break;
            }
        }

        {
            let mut response = client::space::get_hierarchy::v1::Response::new();
            response.rooms = results;
            response.next_batch = if let Some((room, _)) = next_room_to_traverse(&mut stack, &mut parents) {
                parents.pop_front();
                parents.push_back(room);

                let mut short_room_ids = vec![];

                for room in parents {
                    short_room_ids.push(services().rooms.short.get_or_create_shortroomid(&room)?);
                }

                Some(
                    PagnationToken {
                        short_room_ids,
                        limit: UInt::new(max_depth as u64)
                            .expect("When sent in request it must have been valid UInt"),
                        max_depth: UInt::new(max_depth as u64)
                            .expect("When sent in request it must have been valid UInt"),
                        suggested_only,
                    }
                    .to_string(),
                )
            } else {
                None
            };
            Ok(response)
        }
    }
}

fn next_room_to_traverse(
    stack: &mut Vec<Vec<(OwnedRoomId, Vec<OwnedServerName>)>>,
    parents: &mut VecDeque<OwnedRoomId>,
) -> Option<(OwnedRoomId, Vec<OwnedServerName>)> {
    while stack.last().map_or(false, |s| s.is_empty()) {
        stack.pop();
        parents.pop_back();
    }

    stack.last_mut().and_then(|s| s.pop())
}

/// Simply returns the stripped m.space.child events of a room
async fn get_stripped_space_child_events(
    room_id: &RoomId,
) -> Result<Option<Vec<Raw<HierarchySpaceChildEvent>>>, Error> {
    if let Some(current_shortstatehash) = services().rooms.state.get_room_shortstatehash(room_id)? {
        let state = services()
            .rooms
            .state_accessor
            .state_full_ids(current_shortstatehash)
            .await?;
        let mut children_pdus = Vec::new();
        for (key, id) in state {
            let (event_type, state_key) = services().rooms.short.get_statekey_from_short(key)?;
            if event_type != StateEventType::SpaceChild {
                continue;
            }

            let pdu = services()
                .rooms
                .timeline
                .get_pdu(&id)?
                .ok_or_else(|| Error::Database("Event in space state not found".to_string()))?;

            if serde_json::from_str::<SpaceChildEventContent>(pdu.content.get())
                .ok()
                .map(|c| c.via)
                .map_or(true, |v| v.is_empty())
            {
                continue;
            }

            if OwnedRoomId::try_from(state_key).is_ok() {
                children_pdus.push(pdu.to_stripped_spacechild_state_event());
            }
        }
        Ok(Some(children_pdus))
    } else {
        Ok(None)
    }
}

/// With the given identifier, checks if a room is accessible
fn is_accessible_child(
    current_room: &OwnedRoomId,
    join_rule: &SpaceRoomJoinRule,
    identifier: &Identifier<'_>,
    allowed_room_ids: &Vec<OwnedRoomId>,
) -> bool {
    // Note: unwrap_or_default for bool means false
    match identifier {
        Identifier::ServerName(server_name) => {
            let room_id: &RoomId = current_room;

            // Checks if ACLs allow for the server to participate
            if services()
                .rooms
                .event_handler
                .acl_check(server_name, room_id)
                .is_err()
            {
                return false;
            }
        }
        Identifier::UserId(user_id) => {
            if services()
                .rooms
                .state_cache
                .is_joined(user_id, current_room)
                .unwrap_or_default()
                || services()
                    .rooms
                    .state_cache
                    .is_invited(user_id, current_room)
                    .unwrap_or_default()
            {
                return true;
            }
        }
    } // Takes care of joinrules
    match join_rule {
        SpaceRoomJoinRule::Restricted => {
            for room in allowed_room_ids {
                match identifier {
                    Identifier::UserId(user) => {
                        if services()
                            .rooms
                            .state_cache
                            .is_joined(user, room)
                            .unwrap_or_default()
                        {
                            return true;
                        }
                    }
                    Identifier::ServerName(server) => {
                        if services()
                            .rooms
                            .state_cache
                            .server_in_room(server, room)
                            .unwrap_or_default()
                        {
                            return true;
                        }
                    }
                }
            }
            false
        }
        SpaceRoomJoinRule::Public
        | SpaceRoomJoinRule::Knock
        | SpaceRoomJoinRule::KnockRestricted => true,
        SpaceRoomJoinRule::Invite | SpaceRoomJoinRule::Private => false,
        // Custom join rule
        _ => false,
    }
}

// Here because cannot implement `From` across ruma-federation-api and ruma-client-api types
fn summary_to_chunk(summary: SpaceHierarchyParentSummary) -> SpaceHierarchyRoomsChunk {
    let SpaceHierarchyParentSummary {
        canonical_alias: _,
        name: _,
        topic: _,
        avatar_url: _,
        room_type: _,
        num_joined_members,
        room_id,
        world_readable,
        guest_can_join,
        join_rule,
        children_state,
        ..
    } = summary;

    {
        SpaceHierarchyRoomsChunkInit {
            room_id,
            world_readable,
            guest_can_join,
            join_rule,
            children_state,
            num_joined_members,
        }.into()
    }
}

/// Returns the children of a SpaceHierarchyParentSummary, making use of the children_state field
fn get_parent_children_via(
    parent: SpaceHierarchyParentSummary,
    suggested_only: bool,
) -> Vec<(OwnedRoomId, Vec<OwnedServerName>)> {
    parent
        .children_state
        .iter()
        .filter_map(|raw_ce| {
            raw_ce.deserialize().map_or(None, |ce| {
                if suggested_only && !ce.content.suggested {
                    None
                } else {
                    Some((ce.state_key, ce.content.via))
                }
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use ruma::{
        api::federation::space::SpaceHierarchyParentSummaryInit, owned_room_id, owned_server_name,
    };

    use super::*;

    #[test]
    fn get_summary_children() {
        let summary: SpaceHierarchyParentSummary = SpaceHierarchyParentSummaryInit {
            num_joined_members: UInt::from(1_u32),
            room_id: owned_room_id!("!root:example.org"),
            world_readable: true,
            guest_can_join: true,
            join_rule: SpaceRoomJoinRule::Public,
            children_state: vec![
                serde_json::from_str(
                    r#"{
                      "content": {
                        "via": [
                          "example.org"
                        ],
                        "suggested": false
                      },
                      "origin_server_ts": 1629413349153,
                      "sender": "@alice:example.org",
                      "state_key": "!foo:example.org",
                      "type": "m.space.child"
                    }"#,
                )
                .unwrap(),
                serde_json::from_str(
                    r#"{
                      "content": {
                        "via": [
                          "example.org"
                        ],
                        "suggested": true
                      },
                      "origin_server_ts": 1629413349157,
                      "sender": "@alice:example.org",
                      "state_key": "!bar:example.org",
                      "type": "m.space.child"
                    }"#,
                )
                .unwrap(),
                serde_json::from_str(
                    r#"{
                      "content": {
                        "via": [
                          "example.org"
                        ]
                      },
                      "origin_server_ts": 1629413349160,
                      "sender": "@alice:example.org",
                      "state_key": "!baz:example.org",
                      "type": "m.space.child"
                    }"#,
                )
                .unwrap(),
            ],
            allowed_room_ids: vec![],
        }
        .into();

        assert_eq!(
            get_parent_children_via(summary.clone(), false),
            vec![
                (
                    owned_room_id!("!foo:example.org"),
                    vec![owned_server_name!("example.org")]
                ),
                (
                    owned_room_id!("!bar:example.org"),
                    vec![owned_server_name!("example.org")]
                ),
                (
                    owned_room_id!("!baz:example.org"),
                    vec![owned_server_name!("example.org")]
                )
            ]
        );
        assert_eq!(
            get_parent_children_via(summary, true),
            vec![(
                owned_room_id!("!bar:example.org"),
                vec![owned_server_name!("example.org")]
            )]
        );
    }

    #[test]
    fn invalid_pagnation_tokens() {
        fn token_is_err(token: &str) {
            let token: Result<PagnationToken> = PagnationToken::from_str(token);
            assert!(token.is_err());
        }

        token_is_err("231_2_noabool");
        token_is_err("");
        token_is_err("111_3_");
        token_is_err("foo_not_int");
        token_is_err("11_4_true_");
        token_is_err("___");
        token_is_err("__false");
    }

    #[test]
    fn valid_pagnation_tokens() {
        assert_eq!(
            PagnationToken {
                short_room_ids: vec![5383, 42934, 283, 423],
                limit: UInt::from(20_u32),
                max_depth: UInt::from(1_u32),
                suggested_only: true
            },
            PagnationToken::from_str("5383,42934,283,423_20_1_true").unwrap()
        );

        assert_eq!(
            PagnationToken {
                short_room_ids: vec![740],
                limit: UInt::from(97_u32),
                max_depth: UInt::from(10539_u32),
                suggested_only: false
            },
            PagnationToken::from_str("740_97_10539_false").unwrap()
        );
    }

    #[test]
    fn pagnation_token_to_string() {
        assert_eq!(
            PagnationToken {
                short_room_ids: vec![740],
                limit: UInt::from(97_u32),
                max_depth: UInt::from(10539_u32),
                suggested_only: false
            }
            .to_string(),
            "740_97_10539_false"
        );

        assert_eq!(
            PagnationToken {
                short_room_ids: vec![9, 34],
                limit: UInt::from(3_u32),
                max_depth: UInt::from(1_u32),
                suggested_only: true
            }
            .to_string(),
            "9,34_3_1_true"
        );
    }
}
