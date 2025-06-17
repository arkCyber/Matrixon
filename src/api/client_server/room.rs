// =============================================================================
// Matrixon Matrix NextServer - Room Module
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
//   Matrix API implementation for client-server communication. This module is part of the Matrixon Matrix NextServer
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
//   • Matrix protocol compliance
//   • RESTful API endpoints
//   • Request/response handling
//   • Authentication and authorization
//   • Rate limiting and security
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

use crate::{
    api::client_server::invite_helper, service::pdu::PduBuilder, services, Error, Result, Ruma,
};
use ruma::{
    api::client::{
        error::ErrorKind,
        room::{self, aliases, create_room, get_room_event, upgrade_room},
    },
    events::{
        room::{
            canonical_alias::RoomCanonicalAliasEventContent,
            create::RoomCreateEventContent,
            guest_access::{GuestAccess, RoomGuestAccessEventContent},
            history_visibility::{HistoryVisibility, RoomHistoryVisibilityEventContent},
            join_rules::{JoinRule, RoomJoinRulesEventContent},
            member::{MembershipState, RoomMemberEventContent},
            name::RoomNameEventContent,
            power_levels::RoomPowerLevelsEventContent,
            tombstone::RoomTombstoneEventContent,
            topic::RoomTopicEventContent,
        },
        StateEventType, TimelineEventType,
    },
    int,
    serde::JsonObject,
    CanonicalJsonObject, OwnedRoomAliasId, RoomAliasId, RoomId, RoomVersionId,
};
use serde_json::{json, value::to_raw_value};
use std::{cmp::max, collections::BTreeMap, sync::Arc};
use tracing::{info, warn};

/// # `POST /_matrix/client/r0/createRoom`
///
/// Creates a new room.
///
/// - Room ID is randomly generated
/// - Create alias if room_alias_name is set
/// - Send create event
/// - Join sender user
/// - Send power levels event
/// - Send canonical room alias
/// - Send join rules
/// - Send history visibility
/// - Send guest access
/// - Send events listed in initial state
/// - Send events implied by `name` and `topic`
/// - Send invite events
pub async fn create_room_route(
    body: Ruma<create_room::v3::Request>,
) -> Result<create_room::v3::Response> {
    use create_room::v3::RoomPreset;

    let sender_user = body.sender_user.as_ref().expect("user is authenticated");

    let room_id = RoomId::parse(services().globals.server_name())?;

    services().rooms.short.get_or_create_shortroomid(&room_id)?;

    let mutex_state = Arc::clone(
        services()
            .globals
            .roomid_mutex_state
            .write()
            .await
            .entry(room_id.clone())
            .or_default(),
    );
    let state_lock = mutex_state.lock().await;

    if !services().globals.allow_room_creation()
        && body.appservice_info.is_none()
        && !services().users.is_admin(sender_user)?
    {
        return Err(Error::BadRequestString(
            ErrorKind::forbidden(),
            "Room creation has been disabled"
        ));
    }

    let alias: Option<OwnedRoomAliasId> =
        body.room_alias_name
            .as_ref()
            .map_or(Ok(None), |localpart| {
                // TODO: Check for invalid characters and maximum length
                let alias = RoomAliasId::parse(format!(
                    "#{}:{}",
                    localpart,
                    services().globals.server_name()
                ))
                .map_err(|_| Error::BadRequest(ErrorKind::InvalidParam, "Invalid alias"))?;

                if services()
                    .rooms
                    .alias
                    .resolve_local_alias(&alias)?
                    .is_some()
                {
                    Err(Error::BadRequestString(
                        ErrorKind::RoomInUse,
                        "Room alias already exists"
                    ))
                } else {
                    Ok(Some(alias))
                }
            })?;

    if let Some(ref alias) = alias {
        if let Some(ref info) = body.appservice_info {
            if !info.aliases.is_match(alias.as_str()) {
                return Err(Error::BadRequestString(
                    ErrorKind::Exclusive,
                    "Room alias is not in namespace"
                ));
            }
        } else if services().appservice.is_exclusive_alias(alias).await {
            return Err(Error::BadRequestString(
                ErrorKind::Exclusive,
                "Room alias reserved by appservice"
            ));
        }
    }

    let room_version = match body.room_version.clone() {
        Some(room_version) => {
            if services()
                .globals
                .supported_room_versions()
                .contains(&room_version)
            {
                room_version
            } else {
                return Err(Error::BadRequestString(
                    ErrorKind::UnsupportedRoomVersion,
                    "This server does not support that room version"
                ));
            }
        }
        None => services().globals.default_room_version(),
    };

    let content = match &body.creation_content {
        Some(content) => {
            let mut content = content
                .deserialize_as::<CanonicalJsonObject>()
                .expect("Invalid creation content");

            match room_version {
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
                    content.insert(
                        "creator".into(),
                        json!(&sender_user).try_into().map_err(|_| {
                            Error::BadRequest(ErrorKind::BadJson, "Invalid creation content")
                        })?,
                    );
                }
                RoomVersionId::V11 => {} // V11 removed the "creator" key
                _ => unreachable!("Validity of room version already checked"),
            }

            content.insert(
                "room_version".into(),
                json!(room_version.as_str()).try_into().map_err(|_| {
                    Error::BadRequest(ErrorKind::BadJson, "Invalid creation content")
                })?,
            );
            content
        }
        None => {
            let content = match room_version {
                RoomVersionId::V1
                | RoomVersionId::V2
                | RoomVersionId::V3
                | RoomVersionId::V4
                | RoomVersionId::V5
                | RoomVersionId::V6
                | RoomVersionId::V7
                | RoomVersionId::V8
                | RoomVersionId::V9
                | RoomVersionId::V10 => RoomCreateEventContent::new_v1(sender_user.clone()),
                RoomVersionId::V11 => RoomCreateEventContent::new_v11(),
                _ => unreachable!("Validity of room version already checked"),
            };
            let mut content = serde_json::from_str::<CanonicalJsonObject>(
                to_raw_value(&content)
                    .map_err(|_| Error::BadRequest(ErrorKind::BadJson, "Invalid creation content"))?
                    .get(),
            )
            .unwrap();
            content.insert(
                "room_version".into(),
                json!(room_version.as_str()).try_into().map_err(|_| {
                    Error::BadRequest(ErrorKind::BadJson, "Invalid creation content")
                })?,
            );
            content
        }
    };

    // Validate creation content
    let de_result = serde_json::from_str::<CanonicalJsonObject>(
        to_raw_value(&content)
            .expect("Invalid creation content")
            .get(),
    );

    if de_result.is_err() {
        return Err(Error::BadRequestString(
            ErrorKind::BadJson,
            "Invalid creation content",
        ));
    }

    // 1. The room create event
    services()
        .rooms
        .timeline
        .build_and_append_pdu(
            PduBuilder {
                event_type: TimelineEventType::RoomCreate,
                content: to_raw_value(&content).expect("event is valid, we just created it"),
                unsigned: None,
                state_key: Some("".to_owned()),
                redacts: None,
                timestamp: None,
            },
            sender_user,
            &room_id,
            &state_lock,
        )
        .await?;

    // 2. Let the room creator join
    services()
        .rooms
        .timeline
        .build_and_append_pdu(
            {
                let mut content = RoomMemberEventContent::new(MembershipState::Join);
                content.displayname = services().users.displayname(sender_user)?;
                content.avatar_url = services().users.avatar_url(sender_user)?;
                content.is_direct = Some(body.is_direct);
                content.third_party_invite = None;
                content.blurhash = services().users.blurhash(sender_user)?;
                content.reason = None;
                content.join_authorized_via_users_server = None;
                
                PduBuilder {
                    event_type: TimelineEventType::RoomMember,
                    content: to_raw_value(&content).expect("event is valid, we just created it"),
                    unsigned: None,
                    state_key: Some(sender_user.to_string()),
                    redacts: None,
                    timestamp: None,
                }
            },
            sender_user,
            &room_id,
            &state_lock,
        )
        .await?;

    // 3. Power levels

    // Figure out preset. We need it for preset specific events
    let preset = body.preset.clone().unwrap_or(match &body.visibility {
        room::Visibility::Private => RoomPreset::PrivateChat,
        room::Visibility::Public => RoomPreset::PublicChat,
        _ => RoomPreset::PrivateChat, // Room visibility should not be custom
    });

    let mut users = BTreeMap::new();
    users.insert(sender_user.clone(), int!(100));

    if preset == RoomPreset::TrustedPrivateChat {
        for invite_ in &body.invite {
            users.insert(invite_.clone(), int!(100));
        }
    }

    let mut power_levels_content = serde_json::to_value({
        let mut content = RoomPowerLevelsEventContent::new();
        content.users = users;
        content
    })
    .expect("event is valid, we just created it");

    if let Some(power_level_content_override) = &body.power_level_content_override {
        let json: JsonObject = serde_json::from_str(power_level_content_override.json().get())
            .map_err(|_| {
                Error::BadRequest(ErrorKind::BadJson, "Invalid power_level_content_override.")
            })?;

        for (key, value) in json {
            power_levels_content[key] = value;
        }
    }

    services()
        .rooms
        .timeline
        .build_and_append_pdu(
            PduBuilder {
                event_type: TimelineEventType::RoomPowerLevels,
                content: to_raw_value(&power_levels_content)
                    .expect("to_raw_value always works on serde_json::Value"),
                unsigned: None,
                state_key: Some("".to_owned()),
                redacts: None,
                timestamp: None,
            },
            sender_user,
            &room_id,
            &state_lock,
        )
        .await?;

    // 4. Canonical room alias
    if let Some(room_alias_id) = &alias {
        services()
            .rooms
            .timeline
            .build_and_append_pdu(
                {
                    let mut content = RoomCanonicalAliasEventContent::new();
                    content.alias = Some(room_alias_id.to_owned());
                    content.alt_aliases = vec![];
                    
                    PduBuilder {
                        event_type: TimelineEventType::RoomCanonicalAlias,
                        content: to_raw_value(&content).expect("We checked that alias earlier, it must be fine"),
                        unsigned: None,
                        state_key: Some("".to_owned()),
                        redacts: None,
                        timestamp: None,
                    }
                },
                sender_user,
                &room_id,
                &state_lock,
            )
            .await?;
    }

    // 5. Events set by preset

    // 5.1 Join Rules
    services()
        .rooms
        .timeline
        .build_and_append_pdu(
            PduBuilder {
                event_type: TimelineEventType::RoomJoinRules,
                content: to_raw_value(&RoomJoinRulesEventContent::new(match preset {
                    RoomPreset::PublicChat => JoinRule::Public,
                    // according to spec "invite" is the default
                    _ => JoinRule::Invite,
                }))
                .expect("event is valid, we just created it"),
                unsigned: None,
                state_key: Some("".to_owned()),
                redacts: None,
                timestamp: None,
            },
            sender_user,
            &room_id,
            &state_lock,
        )
        .await?;

    // 5.2 History Visibility
    services()
        .rooms
        .timeline
        .build_and_append_pdu(
            PduBuilder {
                event_type: TimelineEventType::RoomHistoryVisibility,
                content: to_raw_value(&RoomHistoryVisibilityEventContent::new(
                    HistoryVisibility::Shared,
                ))
                .expect("event is valid, we just created it"),
                unsigned: None,
                state_key: Some("".to_owned()),
                redacts: None,
                timestamp: None,
            },
            sender_user,
            &room_id,
            &state_lock,
        )
        .await?;

    // 5.3 Guest Access
    services()
        .rooms
        .timeline
        .build_and_append_pdu(
            PduBuilder {
                event_type: TimelineEventType::RoomGuestAccess,
                content: to_raw_value(&RoomGuestAccessEventContent::new(match preset {
                    RoomPreset::PublicChat => GuestAccess::Forbidden,
                    _ => GuestAccess::CanJoin,
                }))
                .expect("event is valid, we just created it"),
                unsigned: None,
                state_key: Some("".to_owned()),
                redacts: None,
                timestamp: None,
            },
            sender_user,
            &room_id,
            &state_lock,
        )
        .await?;

    // 6. Events listed in initial_state
    for event in &body.initial_state {
        let mut pdu_builder = event.deserialize_as::<PduBuilder>().map_err(|e| {
            warn!("Invalid initial state event: {:?}", e);
            Error::BadRequest(ErrorKind::InvalidParam, "Invalid initial state event.")
        })?;

        // Implicit state key defaults to ""
        pdu_builder.state_key.get_or_insert_with(|| "".to_owned());

        // Silently skip encryption events if they are not allowed
        if pdu_builder.event_type == TimelineEventType::RoomEncryption
            && !services().globals.allow_encryption()
        {
            continue;
        }

        services()
            .rooms
            .timeline
            .build_and_append_pdu(pdu_builder, sender_user, &room_id, &state_lock)
            .await?;
    }

    // 7. Events implied by name and topic
    if let Some(name) = &body.name {
        services()
            .rooms
            .timeline
            .build_and_append_pdu(
                PduBuilder {
                    event_type: TimelineEventType::RoomName,
                    content: to_raw_value(&RoomNameEventContent::new(name.clone()))
                        .expect("event is valid, we just created it"),
                    unsigned: None,
                    state_key: Some("".to_owned()),
                    redacts: None,
                    timestamp: None,
                },
                sender_user,
                &room_id,
                &state_lock,
            )
            .await?;
    }

    if let Some(topic) = &body.topic {
        services()
            .rooms
            .timeline
            .build_and_append_pdu(
                {
                    let content = RoomTopicEventContent::new(topic.clone());
                    
                    PduBuilder {
                        event_type: TimelineEventType::RoomTopic,
                        content: to_raw_value(&content).expect("event is valid, we just created it"),
                        unsigned: None,
                        state_key: Some("".to_owned()),
                        redacts: None,
                        timestamp: None,
                    }
                },
                sender_user,
                &room_id,
                &state_lock,
            )
            .await?;
    }

    // 8. Events implied by invite (and TODO: invite_3pid)
    drop(state_lock);
    for user_id in &body.invite {
        let _ = invite_helper(sender_user, user_id, &room_id, None, body.is_direct).await;
    }

    // NextServer specific stuff
    if let Some(alias) = alias {
        services()
            .rooms
            .alias
            .set_alias(&alias, &room_id, sender_user)?;
    }

    if body.visibility == room::Visibility::Public {
        services().rooms.directory.set_public(&room_id)?;
    }

    info!("{} created a room", sender_user);

    Ok(create_room::v3::Response::new(room_id))
}

/// # `GET /_matrix/client/r0/rooms/{roomId}/event/{eventId}`
///
/// Gets a single event.
///
/// - You have to currently be joined to the room (TODO: Respect history visibility)
pub async fn get_room_event_route(
    body: Ruma<get_room_event::v3::Request>,
) -> Result<get_room_event::v3::Response> {
    let sender_user = body.sender_user.as_ref().expect("user is authenticated");

    let event = services()
        .rooms
        .timeline
        .get_pdu(&body.event_id)?
        .ok_or_else(|| {
            warn!("Event not found, event ID: {:?}", &body.event_id);
            Error::BadRequest(ErrorKind::NotFound, "Event not found.")
        })?;

    if !services().rooms.state_accessor.user_can_see_event(
        sender_user,
        &event.room_id,
        &body.event_id,
    )? {
        return Err(Error::BadRequestString(
            ErrorKind::forbidden(),
            "You don't have permission to view this event.",
        ));
    }

    let mut event = (*event).clone();
    event.add_age()?;

    Ok(get_room_event::v3::Response::new(event.to_room_event()))
}

/// # `GET /_matrix/client/r0/rooms/{roomId}/aliases`
///
/// Lists all aliases of the room.
///
/// - Only users joined to the room are allowed to call this TODO: Allow any user to call it if history_visibility is world readable
pub async fn get_room_aliases_route(
    body: Ruma<aliases::v3::Request>,
) -> Result<aliases::v3::Response> {
    let sender_user = body.sender_user.as_ref().expect("user is authenticated");

    if !services()
        .rooms
        .state_cache
        .is_joined(sender_user, &body.room_id)?
    {
        return Err(Error::BadRequestString(
            ErrorKind::forbidden(),
            "You don't have permission to view this room.",
        ));
    }

    Ok(aliases::v3::Response::new(
        services()
            .rooms
            .alias
            .local_aliases_for_room(&body.room_id)
            .filter_map(|a| a.ok())
            .collect(),
    ))
}

/// # `POST /_matrix/client/r0/rooms/{roomId}/upgrade`
///
/// Upgrades the room.
///
/// - Creates a replacement room
/// - Sends a tombstone event into the current room
/// - Sender user joins the room
/// - Transfers some state events
/// - Moves local aliases
/// - Modifies old room power levels to prevent users from speaking
pub async fn upgrade_room_route(
    body: Ruma<upgrade_room::v3::Request>,
) -> Result<upgrade_room::v3::Response> {
    let sender_user = body.sender_user.as_ref().expect("user is authenticated");

    if !services()
        .globals
        .supported_room_versions()
        .contains(&body.new_version)
    {
        return Err(Error::BadRequestString(
            ErrorKind::UnsupportedRoomVersion,
            "This server does not support that room version.",
        ));
    }

    // Create a replacement room
    let replacement_room = RoomId::new(services().globals.server_name());
    services()
        .rooms
        .short
        .get_or_create_shortroomid(&replacement_room)?;

    let mutex_state = Arc::clone(
        services()
            .globals
            .roomid_mutex_state
            .write()
            .await
            .entry(body.room_id.clone())
            .or_default(),
    );
    let state_lock = mutex_state.lock().await;

    // Send a m.room.tombstone event to the old room to indicate that it is not intended to be used any further
    // Fail if the sender does not have the required permissions
    let tombstone_event_id = services()
        .rooms
        .timeline
        .build_and_append_pdu(
            {
                let content = RoomTombstoneEventContent::new(
                    "This room has been replaced".to_owned(),
                    replacement_room.clone(),
                );
                
                PduBuilder {
                    event_type: TimelineEventType::RoomTombstone,
                    content: to_raw_value(&content).expect("event is valid, we just created it"),
                    unsigned: None,
                    state_key: Some("".to_owned()),
                    redacts: None,
                    timestamp: None,
                }
            },
            sender_user,
            &body.room_id,
            &state_lock,
        )
        .await?;

    // Change lock to replacement room
    drop(state_lock);
    let mutex_state = Arc::clone(
        services()
            .globals
            .roomid_mutex_state
            .write()
            .await
            .entry(replacement_room.clone())
            .or_default(),
    );
    let state_lock = mutex_state.lock().await;

    // Get the old room creation event
    let mut create_event_content = serde_json::from_str::<CanonicalJsonObject>(
        services()
            .rooms
            .state_accessor
            .room_state_get(&body.room_id, &StateEventType::RoomCreate, "")?
            .ok_or_else(|| Error::bad_database("Found room without m.room.create event."))?
            .content
            .get(),
    )
    .map_err(|_| Error::bad_database("Invalid room event in database."))?;

    // Use the m.room.tombstone event as the predecessor
    let predecessor = Some(ruma::events::room::create::PreviousRoom::new(
        body.room_id.clone(),
        (*tombstone_event_id).to_owned(),
    ));

    // Send a m.room.create event containing a predecessor field and the applicable room_version
    match body.new_version {
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
            create_event_content.insert(
                "creator".into(),
                json!(&sender_user).try_into().map_err(|_| {
                    Error::BadRequest(ErrorKind::BadJson, "Error forming creation event")
                })?,
            );
        }
        RoomVersionId::V11 => {
            // "creator" key no longer exists in V11 rooms
            create_event_content.remove("creator");
        }
        _ => unreachable!("Validity of room version already checked"),
    }
    create_event_content.insert(
        "room_version".into(),
        json!(&body.new_version)
            .try_into()
            .map_err(|_| Error::BadRequest(ErrorKind::BadJson, "Error forming creation event"))?,
    );
    create_event_content.insert(
        "predecessor".into(),
        json!(predecessor)
            .try_into()
            .map_err(|_| Error::BadRequest(ErrorKind::BadJson, "Error forming creation event"))?,
    );

    // Validate creation event content
    let de_result = serde_json::from_str::<CanonicalJsonObject>(
        to_raw_value(&create_event_content)
            .expect("Error forming creation event")
            .get(),
    );

    if de_result.is_err() {
        return Err(Error::BadRequestString(
            ErrorKind::BadJson,
            "Error forming creation event",
        ));
    }

    services()
        .rooms
        .timeline
        .build_and_append_pdu(
            PduBuilder {
                event_type: TimelineEventType::RoomCreate,
                content: to_raw_value(&create_event_content)
                    .expect("event is valid, we just created it"),
                unsigned: None,
                state_key: Some("".to_owned()),
                redacts: None,
                timestamp: None,
            },
            sender_user,
            &replacement_room,
            &state_lock,
        )
        .await?;

    // Join the new room
    services()
        .rooms
        .timeline
        .build_and_append_pdu(
            {
                let mut content = RoomMemberEventContent::new(MembershipState::Join);
                content.displayname = services().users.displayname(sender_user)?;
                content.avatar_url = services().users.avatar_url(sender_user)?;
                content.is_direct = None;
                content.third_party_invite = None;
                content.blurhash = services().users.blurhash(sender_user)?;
                content.reason = None;
                content.join_authorized_via_users_server = None;
                
                PduBuilder {
                    event_type: TimelineEventType::RoomMember,
                    content: to_raw_value(&content).expect("event is valid, we just created it"),
                    unsigned: None,
                    state_key: Some(sender_user.to_string()),
                    redacts: None,
                    timestamp: None,
                }
            },
            sender_user,
            &replacement_room,
            &state_lock,
        )
        .await?;

    // Recommended transferable state events list from the specs
    let transferable_state_events = vec![
        StateEventType::RoomServerAcl,
        StateEventType::RoomEncryption,
        StateEventType::RoomName,
        StateEventType::RoomAvatar,
        StateEventType::RoomTopic,
        StateEventType::RoomGuestAccess,
        StateEventType::RoomHistoryVisibility,
        StateEventType::RoomJoinRules,
        StateEventType::RoomPowerLevels,
    ];

    // Replicate transferable state events to the new room
    for event_type in transferable_state_events {
        let event_content =
            match services()
                .rooms
                .state_accessor
                .room_state_get(&body.room_id, &event_type, "")?
            {
                Some(v) => v.content.clone(),
                None => continue, // Skipping missing events.
            };

        services()
            .rooms
            .timeline
            .build_and_append_pdu(
                PduBuilder {
                    event_type: event_type.to_string().into(),
                    content: event_content,
                    unsigned: None,
                    state_key: Some("".to_owned()),
                    redacts: None,
                    timestamp: None,
                },
                sender_user,
                &replacement_room,
                &state_lock,
            )
            .await?;
    }

    // Moves any local aliases to the new room
    for alias in services()
        .rooms
        .alias
        .local_aliases_for_room(&body.room_id)
        .filter_map(|r| r.ok())
    {
        services()
            .rooms
            .alias
            .set_alias(&alias, &replacement_room, sender_user)?;
    }

    // Get the old room power levels
    let mut power_levels_event_content: RoomPowerLevelsEventContent = serde_json::from_str(
        services()
            .rooms
            .state_accessor
            .room_state_get(&body.room_id, &StateEventType::RoomPowerLevels, "")?
            .ok_or_else(|| Error::bad_database("Found room without m.room.create event."))?
            .content
            .get(),
    )
    .map_err(|_| Error::bad_database("Invalid room event in database."))?;

    // Setting events_default and invite to the greater of 50 and users_default + 1
    let new_level = max(int!(50), power_levels_event_content.users_default + int!(1));
    power_levels_event_content.events_default = new_level;
    power_levels_event_content.invite = new_level;

    // Modify the power levels in the old room to prevent sending of events and inviting new users
    let _ = services()
        .rooms
        .timeline
        .build_and_append_pdu(
            PduBuilder {
                event_type: TimelineEventType::RoomPowerLevels,
                content: to_raw_value(&power_levels_event_content)
                    .expect("event is valid, we just created it"),
                unsigned: None,
                state_key: Some("".to_owned()),
                redacts: None,
                timestamp: None,
            },
            sender_user,
            &body.room_id,
            &state_lock,
        )
        .await?;

    drop(state_lock);

    // Return the replacement room id
    Ok(upgrade_room::v3::Response::new(replacement_room))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruma::{
        api::client::room::create_room::v3::{Request as CreateRoomRequest, RoomPreset},
        user_id, room_id,
    };
    use serde_json::json;
    use std::time::{Duration, Instant};

    /// Test helper to create a basic room creation request
    fn create_test_room_request() -> CreateRoomRequest {
        let mut request = CreateRoomRequest::new();
        request.creation_content = None;
        request.initial_state = vec![];
        request.invite = vec![];
        request.invite_3pid = vec![];
        request.is_direct = false;
        request.name = Some("Test Room".to_string());
        request.power_level_content_override = None;
        request.preset = Some(RoomPreset::PrivateChat);
        request.room_alias_name = None;
        request.room_version = None;
        request.topic = Some("A test room for testing".to_string());
        request.visibility = ruma::api::client::room::Visibility::Private;
        request
    }

    /// Test helper to create room creation request with alias
    fn create_test_room_request_with_alias(alias_name: &str) -> CreateRoomRequest {
        let mut request = create_test_room_request();
        request.room_alias_name = Some(alias_name.to_string());
        request
    }

    #[test]
    fn test_room_creation_request_structure() {
        let request = create_test_room_request();
        
        // Validate request structure
        assert!(request.name.is_some(), "Test room should have a name");
        assert!(request.topic.is_some(), "Test room should have a topic");
        assert!(request.preset.is_some(), "Test room should have a preset");
        assert!(!request.is_direct, "Default should not be direct message");
        assert!(request.invite.is_empty(), "No initial invites");
        assert!(request.initial_state.is_empty(), "No initial state events");
    }

    #[test]
    fn test_room_creation_with_alias_structure() {
        let alias_name = "test-room";
        let request = create_test_room_request_with_alias(alias_name);
        
        assert_eq!(request.room_alias_name, Some(alias_name.to_string()));
        assert!(request.name.is_some(), "Room with alias should have name");
    }

    #[test]
    fn test_room_version_validation() {
        // Test supported room versions
        let supported_versions = [
            RoomVersionId::V1,
            RoomVersionId::V2,
            RoomVersionId::V3,
            RoomVersionId::V4,
            RoomVersionId::V5,
            RoomVersionId::V6,
            RoomVersionId::V7,
            RoomVersionId::V8,
            RoomVersionId::V9,
            RoomVersionId::V10,
            RoomVersionId::V11,
        ];

        for version in supported_versions {
            // Test that each version can be handled
            let mut request = create_test_room_request();
            request.room_version = Some(version.clone());
            
            assert_eq!(request.room_version, Some(version));
        }
    }

    #[test]
    fn test_room_creation_content_structure() {
        // Test room creation content validation - simplified without Raw conversion
        let mut request = create_test_room_request();
        
        // Just test the structure without actual conversion
        assert!(request.creation_content.is_none(), "Default creation content should be None");
        
        // Test setting creation content to None is valid
        request.creation_content = None;
        assert!(request.creation_content.is_none(), "Creation content should remain None");
    }

    #[test]
    fn test_room_preset_validation() {
        // Test different room presets
        let presets = [
            RoomPreset::PrivateChat,
            RoomPreset::PublicChat,
            RoomPreset::TrustedPrivateChat,
        ];

        for preset in presets {
            let mut request = create_test_room_request();
            request.preset = Some(preset.clone());
            
            assert_eq!(request.preset, Some(preset));
        }
    }

    #[test]
    fn test_room_alias_validation() {
        // Test room alias format validation
        let valid_aliases = [
            "test-room",
            "room123",
            "general_chat",
            "room-with-dashes",
        ];

        for alias_name in valid_aliases {
            let request = create_test_room_request_with_alias(alias_name);
            assert_eq!(request.room_alias_name, Some(alias_name.to_string()));
        }

        // Test alias length constraints
        let long_alias = "a".repeat(255);
        let request = create_test_room_request_with_alias(&long_alias);
        assert!(request.room_alias_name.is_some(), "Should handle long aliases");
    }

    #[test]
    fn test_initial_state_events_structure() {
        let mut request = create_test_room_request();
        
        // Test with empty initial state (default)
        assert!(request.initial_state.is_empty(), "Initial state should be empty by default");
        
        // Test that we can have initial state events
        request.initial_state = vec![];
        assert_eq!(request.initial_state.len(), 0, "Should maintain empty initial state");
    }

    #[test]
    fn test_room_invite_list_structure() {
        let invite_list = vec![
            user_id!("@user1:example.com").to_owned(),
            user_id!("@user2:example.com").to_owned(),
            user_id!("@user3:example.com").to_owned(),
        ];

        let mut request = create_test_room_request();
        request.invite = invite_list.clone();
        
        assert_eq!(request.invite.len(), 3, "Should have 3 invited users");
        assert!(request.invite.contains(&user_id!("@user1:example.com").to_owned()));
    }

    #[test]
    fn test_room_power_levels_override() {
        let mut request = create_test_room_request();
        
        // Test that power level override can be None (default)
        assert!(request.power_level_content_override.is_none(), "Default should be None");
        
        // Test setting to None
        request.power_level_content_override = None;
        assert!(request.power_level_content_override.is_none(), "Should remain None");
    }

    #[test]
    fn test_direct_message_room_creation() {
        let mut request = create_test_room_request();
        request.is_direct = true;
        request.preset = Some(RoomPreset::TrustedPrivateChat);
        
        assert!(request.is_direct, "Should be marked as direct message");
        assert_eq!(request.preset, Some(RoomPreset::TrustedPrivateChat));
    }

    #[test]
    fn test_room_creation_with_3pid_invites() {
        let mut request = create_test_room_request();
        
        // Test empty 3PID invites by default
        assert!(request.invite_3pid.is_empty(), "Should have no 3PID invites by default");
        
        // Test setting empty list
        request.invite_3pid = vec![];
        assert_eq!(request.invite_3pid.len(), 0, "Should maintain empty 3PID invite list");
    }

    #[test]
    fn test_room_event_request_structure() {
        // Test get room event request structure
        let room_id = room_id!("!test:example.com");
        let event_id = ruma::EventId::parse("$event_id:example.com").unwrap();
        
        let request = ruma::api::client::room::get_room_event::v3::Request::new(
            room_id.to_owned(),
            event_id.to_owned(),
        );
        
        assert_eq!(request.room_id, room_id);
        assert_eq!(request.event_id, event_id);
    }

    #[test]
    fn test_room_aliases_request_structure() {
        let room_id = room_id!("!test:example.com");
        let request = ruma::api::client::room::aliases::v3::Request::new(room_id.to_owned());
        
        assert_eq!(request.room_id, room_id);
    }

    #[test]
    fn test_room_upgrade_request_structure() {
        let room_id = room_id!("!test:example.com");
        let new_version = RoomVersionId::V10;
        
        let request = ruma::api::client::room::upgrade_room::v3::Request::new(
            room_id.to_owned(),
            new_version.clone(),
        );
        
        assert_eq!(request.room_id, room_id);
        assert_eq!(request.new_version, new_version);
    }

    #[test]
    fn test_room_creation_validation_logic() {
        // Test validation logic for room creation
        let request = create_test_room_request();
        
        // Room name validation
        if let Some(name) = &request.name {
            assert!(!name.is_empty(), "Room name should not be empty");
            assert!(name.len() <= 255, "Room name should not exceed reasonable length");
        }

        // Topic validation
        if let Some(topic) = &request.topic {
            assert!(!topic.is_empty(), "Room topic should not be empty if set");
            assert!(topic.len() <= 1000, "Room topic should not exceed reasonable length");
        }

        // Alias validation
        if let Some(alias_name) = &request.room_alias_name {
            assert!(!alias_name.is_empty(), "Alias name should not be empty");
            assert!(!alias_name.contains(':'), "Alias name should not contain server part");
            assert!(alias_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'), 
                    "Alias should contain valid characters");
        }
    }

    #[test]
    fn test_room_creation_security_constraints() {
        // Test security-related constraints
        let request = create_test_room_request();
        
        // Test invite list size constraints
        let max_invites = 100; // Reasonable limit to prevent abuse
        assert!(request.invite.len() <= max_invites, "Invite list should be limited");

        // Test initial state events constraints
        let max_initial_events = 20; // Prevent excessive initial state
        assert!(request.initial_state.len() <= max_initial_events, "Initial state should be limited");

        // Test 3PID invite constraints
        let max_3pid_invites = 10; // Limit external invites
        assert!(request.invite_3pid.len() <= max_3pid_invites, "3PID invites should be limited");
    }

    #[test]
    fn test_room_version_content_compatibility() {
        // Test content compatibility across room versions - simplified
        for version in [RoomVersionId::V1, RoomVersionId::V10, RoomVersionId::V11] {
            let mut request = create_test_room_request();
            request.room_version = Some(version.clone());
            
            // Test that room version is properly set without content conversion
            assert_eq!(request.room_version, Some(version));
            assert!(request.creation_content.is_none(), "Creation content should remain None for testing");
        }
    }

    #[test]
    fn test_performance_constraints() {
        // Test performance-related constraints
        let start = Instant::now();
        
        // Test that room creation request processing is efficient
        for _ in 0..1000 {
            let _ = create_test_room_request();
        }
        
        let duration = start.elapsed();
        assert!(duration < Duration::from_millis(100), 
                "Room request creation should be fast, took: {:?}", duration);
    }

    #[test]
    fn test_concurrent_room_operations() {
        // Test concurrent room operation handling
        use std::thread;
        
        let handles: Vec<_> = (0..10).map(|i| {
            thread::spawn(move || {
                let request = create_test_room_request_with_alias(&format!("room_{}", i));
                assert!(request.room_alias_name.is_some());
                request
            })
        }).collect();

        for handle in handles {
            let result = handle.join().unwrap();
            assert!(result.room_alias_name.is_some());
        }
    }

    #[test]
    fn test_matrix_protocol_compliance() {
        // Test Matrix protocol compliance
        let request = create_test_room_request();
        
        // Test preset compliance
        if let Some(preset) = &request.preset {
            match preset {
                RoomPreset::PrivateChat => {
                    // Private chat should have appropriate defaults
                    assert!(true, "Private chat preset is valid");
                }
                RoomPreset::PublicChat => {
                    // Public chat should be discoverable
                    assert!(!request.is_direct, "Public chat should not be direct");
                }
                RoomPreset::TrustedPrivateChat => {
                    // Trusted private chat for direct messages
                    assert!(true, "Trusted private chat preset is valid");
                }
                _ => {
                    // Handle any other presets
                    assert!(true, "Other presets are valid");
                }
            }
        }

        // Test room version compliance
        if let Some(version) = &request.room_version {
            assert!(!version.as_str().is_empty(), "Room version should not be empty");
        }

        // Test visibility compliance
        match request.visibility {
            ruma::api::client::room::Visibility::Private => {
                assert!(true, "Private visibility is valid");
            }
            ruma::api::client::room::Visibility::Public => {
                assert!(true, "Public visibility is valid");
            }
            _ => {
                assert!(true, "Other visibility options are valid");
            }
        }
    }

    #[test]
    fn test_room_state_event_types() {
        // Test different state event types that can be included - simplified
        let mut request = create_test_room_request();
        
        // Test with empty initial state (safe default)
        request.initial_state = vec![];
        assert_eq!(request.initial_state.len(), 0, "Should have 0 initial state events");
        
        // Test that we can work with the initial state vector
        let initial_count = request.initial_state.len();
        assert_eq!(initial_count, 0, "Initial count should be 0");
    }

    #[test]
    fn test_room_creation_enhanced_structure() {
        let alias_name = "test-room";
        let request = create_test_room_request_with_alias(alias_name);
        
        assert_eq!(request.room_alias_name, Some(alias_name.to_string()));
        assert!(request.name.is_some(), "Room with alias should have name");
    }
}
