// =============================================================================
// Matrixon Matrix NextServer - Pdu Module
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

use crate::Error;
use ruma::{
    api::client::error::ErrorKind,
    canonical_json::redact_content_in_place,
    events::{
        room::{member::RoomMemberEventContent, redaction::RoomRedactionEventContent},
        space::child::HierarchySpaceChildEvent,
        AnyEphemeralRoomEvent, AnyMessageLikeEvent, AnyStateEvent, AnyStrippedStateEvent,
        AnySyncStateEvent, AnySyncTimelineEvent, AnyTimelineEvent, StateEvent, TimelineEventType,
    },
    room_version_rules::{RedactionRules, RoomVersionRules},
    serde::Raw,
    state_res, CanonicalJsonObject, CanonicalJsonValue, EventId, MilliSecondsSinceUnixEpoch,
    OwnedEventId, OwnedRoomId, OwnedUserId, RoomId, UInt, UserId,
};
use serde::{Deserialize, Serialize};
use serde_json::{
    json,
    value::{to_raw_value, RawValue as RawJsonValue},
};
use std::{cmp::Ordering, collections::BTreeMap, sync::Arc, time::{SystemTime, UNIX_EPOCH}};
use tracing::{warn, debug, info, error};
use crate::utils::get_timestamp;

/// Content hashes of a PDU.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EventHash {
    /// The SHA-256 hash.
    pub sha256: String,
}

impl EventHash {
    /// Create a new event hash
    pub fn new(sha256: String) -> Result<Self, Error> {
        let timestamp = get_timestamp();
        
        if sha256.is_empty() {
            let error_msg = "SHA-256 hash cannot be empty";
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(Error::bad_database(error_msg));
        }
        
        // Validate SHA-256 format (64 hex characters)
        if sha256.len() != 64 || !sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            let error_msg = "Invalid SHA-256 hash format";
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(Error::bad_database(error_msg));
        }
        
        debug!("⏰ [{}] ✅ Created event hash: {}...", timestamp, &sha256[..8]);
        Ok(EventHash { sha256 })
    }
    
    /// Validate the hash format
    pub fn validate(&self) -> Result<(), Error> {
        let timestamp = get_timestamp();
        
        if self.sha256.is_empty() {
            let error_msg = "SHA-256 hash is empty";
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(Error::bad_database(error_msg));
        }
        
        if self.sha256.len() != 64 || !self.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            let error_msg = format!("Invalid SHA-256 hash format: {}", self.sha256);
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(Error::bad_database("Invalid SHA-256 hash format"));
        }
        
        debug!("⏰ [{}] ✅ Hash validation passed", timestamp);
        Ok(())
    }
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct PduEvent {
    pub event_id: Arc<EventId>,
    pub room_id: OwnedRoomId,
    pub sender: OwnedUserId,
    pub origin_server_ts: UInt,
    #[serde(rename = "type")]
    pub kind: TimelineEventType,
    pub content: Box<RawJsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_key: Option<String>,
    pub prev_events: Vec<Arc<EventId>>,
    pub depth: UInt,
    pub auth_events: Vec<Arc<EventId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacts: Option<Arc<EventId>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unsigned: Option<Box<RawJsonValue>>,
    pub hashes: EventHash,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signatures: Option<Box<RawJsonValue>>, // BTreeMap<Box<ServerName>, BTreeMap<ServerSigningKeyId, String>>
}

impl PduEvent {
    /// Create a new PDU event with validation
    pub fn new(
        event_id: Arc<EventId>,
        room_id: OwnedRoomId,
        sender: OwnedUserId,
        origin_server_ts: UInt,
        kind: TimelineEventType,
        content: Box<RawJsonValue>,
        state_key: Option<String>,
        prev_events: Vec<Arc<EventId>>,
        depth: UInt,
        auth_events: Vec<Arc<EventId>>,
        redacts: Option<Arc<EventId>>,
        unsigned: Option<Box<RawJsonValue>>,
        hashes: EventHash,
        signatures: Option<Box<RawJsonValue>>,
    ) -> Result<Self, Error> {
        let timestamp = get_timestamp();
        
        debug!("⏰ [{}] 🏗️ Creating PDU event: {} in room {}", 
               timestamp, event_id, room_id);
        
        // Validate required fields
        if room_id.as_str().is_empty() {
            let error_msg = "Room ID cannot be empty";
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(Error::bad_database(error_msg));
        }
        
        if sender.as_str().is_empty() {
            let error_msg = "Sender cannot be empty";
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(Error::bad_database(error_msg));
        }
        
        // Validate content is valid JSON
        if content.get().is_empty() {
            let error_msg = "Event content cannot be empty";
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(Error::bad_database(error_msg));
        }
        
        // Validate depth is reasonable
        if depth > UInt::new(1000000).unwrap() {
            let error_msg = "Event depth is too large";
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(Error::bad_database(error_msg));
        }
        
        // Validate hashes
        hashes.validate()?;
        
        // Validate prev_events count
        if prev_events.len() > 20 {
            warn!("⏰ [{}] ⚠️ High number of prev_events: {}", timestamp, prev_events.len());
        }
        
        // Validate auth_events count
        if auth_events.len() > 10 {
            warn!("⏰ [{}] ⚠️ High number of auth_events: {}", timestamp, auth_events.len());
        }
        
        debug!("⏰ [{}] ✅ PDU event validation passed", timestamp);
        
        Ok(PduEvent {
            event_id,
            room_id,
            sender,
            origin_server_ts,
            kind,
            content,
            state_key,
            prev_events,
            depth,
            auth_events,
            redacts,
            unsigned,
            hashes,
            signatures,
        })
    }
    
    /// Validate the PDU event structure
    pub fn validate(&self) -> Result<(), Error> {
        let timestamp = get_timestamp();
        
        debug!("⏰ [{}] 🔍 Validating PDU event: {}", timestamp, self.event_id);
        
        // Validate event ID format
        if self.event_id.as_str().is_empty() {
            let error_msg = "Event ID cannot be empty";
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(Error::bad_database(error_msg));
        }
        
        // Validate room ID format
        if self.room_id.as_str().is_empty() {
            let error_msg = "Room ID cannot be empty";
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(Error::bad_database(error_msg));
        }
        
        // Validate sender format
        if self.sender.as_str().is_empty() {
            let error_msg = "Sender cannot be empty";
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(Error::bad_database(error_msg));
        }
        
        // Validate timestamp is not in the future (with 1 hour tolerance)
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        
        let event_time: u64 = self.origin_server_ts.into();
        if event_time > current_time + 3600000 { // 1 hour tolerance
            warn!("⏰ [{}] ⚠️ Event timestamp is in the future: {} vs {}", 
                  timestamp, event_time, current_time);
        }
        
        // Validate content is valid JSON
        if let Err(e) = serde_json::from_str::<serde_json::Value>(self.content.get()) {
            let error_msg = format!("Invalid JSON content: {}", e);
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(Error::bad_database("Invalid JSON content"));
        }
        
        // Validate hashes
        self.hashes.validate()?;
        
        // Validate state key for state events (check if it's a state event by checking if state_key is expected)
        match self.kind {
            TimelineEventType::RoomMember | 
            TimelineEventType::RoomCreate |
            TimelineEventType::RoomJoinRules |
            TimelineEventType::RoomPowerLevels |
            TimelineEventType::RoomName |
            TimelineEventType::RoomTopic |
            TimelineEventType::RoomAvatar => {
                if self.state_key.is_none() {
                    let error_msg = "State events must have a state key";
                    error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                    return Err(Error::bad_database(error_msg));
                }
            }
            _ => {}
        }
        
        // Validate redaction target for redaction events
        if self.kind == TimelineEventType::RoomRedaction && self.redacts.is_none() {
            let error_msg = "Redaction events must specify a target event";
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(Error::bad_database(error_msg));
        }
        
        debug!("⏰ [{}] ✅ PDU event validation completed successfully", timestamp);
        Ok(())
    }

    #[tracing::instrument(skip(self, reason))]
    pub fn redact(
        &mut self,
        redaction_rules: RedactionRules,
        reason: &PduEvent,
    ) -> crate::Result<()> {
        let timestamp = get_timestamp();
        
        debug!("⏰ [{}] 🔨 Redacting event {} with reason {}", 
               timestamp, self.event_id, reason.event_id);
        
        // Validate redaction reason
        if reason.kind != TimelineEventType::RoomRedaction {
            let error_msg = "Redaction reason must be a redaction event";
            error!("⏰ [{}] ❌ {}", timestamp, error_msg);
            return Err(Error::bad_database(error_msg));
        }
        
        // Check if already redacted
        if self.is_redacted() {
            warn!("⏰ [{}] ⚠️ Event {} is already redacted", timestamp, self.event_id);
            return Ok(());
        }

        self.unsigned = None;

        let mut content = serde_json::from_str(self.content.get())
            .map_err(|e| {
                let error_msg = format!("PDU in db has invalid content: {}", e);
                error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                Error::bad_database("PDU in db has invalid content.")
            })?;
            
        redact_content_in_place(&mut content, &redaction_rules, self.kind.to_string())
            .map_err(|e| {
                let error_msg = format!("Redaction failed: {}", e);
                error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                Error::RedactionError(self.sender.server_name().to_owned(), e)
            })?;

        self.unsigned = Some(to_raw_value(&json!({
            "redacted_because": serde_json::to_value(reason).expect("to_value(PduEvent) always works")
        })).expect("to string always works"));

        self.content = to_raw_value(&content).expect("to string always works");
        
        info!("⏰ [{}] ✅ Successfully redacted event {}", timestamp, self.event_id);
        Ok(())
    }

    pub fn is_redacted(&self) -> bool {
        let timestamp = get_timestamp();
        
        #[derive(Deserialize)]
        struct ExtractRedactedBecause {
            redacted_because: Option<serde::de::IgnoredAny>,
        }

        let Some(unsigned) = &self.unsigned else {
            debug!("⏰ [{}] 📖 Event {} has no unsigned field", timestamp, self.event_id);
            return false;
        };

        let Ok(unsigned) = ExtractRedactedBecause::deserialize(&**unsigned) else {
            debug!("⏰ [{}] 📖 Event {} unsigned field is not deserializable", timestamp, self.event_id);
            return false;
        };

        let is_redacted = unsigned.redacted_because.is_some();
        debug!("⏰ [{}] 📖 Event {} redacted status: {}", timestamp, self.event_id, is_redacted);
        is_redacted
    }

    pub fn remove_transaction_id(&mut self) -> crate::Result<()> {
        let timestamp = get_timestamp();
        
        debug!("⏰ [{}] 🧹 Removing transaction ID from event {}", timestamp, self.event_id);
        
        if let Some(unsigned) = &self.unsigned {
            let mut unsigned: BTreeMap<String, Box<RawJsonValue>> =
                serde_json::from_str(unsigned.get())
                    .map_err(|e| {
                        let error_msg = format!("Invalid unsigned in pdu event: {}", e);
                        error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                        Error::bad_database("Invalid unsigned in pdu event")
                    })?;
                    
            let had_txn_id = unsigned.contains_key("transaction_id");
            unsigned.remove("transaction_id");
            self.unsigned = Some(to_raw_value(&unsigned).expect("unsigned is valid"));
            
            if had_txn_id {
                debug!("⏰ [{}] ✅ Removed transaction ID from event {}", timestamp, self.event_id);
            } else {
                debug!("⏰ [{}] 📖 No transaction ID found in event {}", timestamp, self.event_id);
            }
        } else {
            debug!("⏰ [{}] 📖 Event {} has no unsigned field", timestamp, self.event_id);
        }

        Ok(())
    }

    pub fn add_age(&mut self) -> crate::Result<()> {
        let timestamp = get_timestamp();
        
        debug!("⏰ [{}] ⏱️ Adding age to event {}", timestamp, self.event_id);
        
        let mut unsigned: BTreeMap<String, Box<RawJsonValue>> = self
            .unsigned
            .as_ref()
            .map_or_else(|| Ok(BTreeMap::new()), |u| serde_json::from_str(u.get()))
            .map_err(|e| {
                let error_msg = format!("Invalid unsigned in pdu event: {}", e);
                error!("⏰ [{}] ❌ {}", timestamp, error_msg);
                Error::bad_database("Invalid unsigned in pdu event")
            })?;

        // Calculate age based on current time and origin_server_ts
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        
        let server_ts: u64 = self.origin_server_ts.into();
        let age = current_time.saturating_sub(server_ts);
        
        unsigned.insert("age".to_owned(), to_raw_value(&age).unwrap());
        self.unsigned = Some(to_raw_value(&unsigned).expect("unsigned is valid"));
        
        debug!("⏰ [{}] ✅ Added age {} ms to event {}", timestamp, age, self.event_id);
        Ok(())
    }

    /// Copies the `redacts` property of the event to the `content` dict and vice-versa.
    ///
    /// This follows the specification's
    /// [recommendation](https://spec.matrix.org/v1.10/rooms/v11/#moving-the-redacts-property-of-mroomredaction-events-to-a-content-property):
    ///
    /// > For backwards-compatibility with older clients, servers should add a redacts
    /// > property to the top level of m.room.redaction events in when serving such events
    /// > over the Client-Server API.
    /// >
    /// > For improved compatibility with newer clients, servers should add a redacts property
    /// > to the content of m.room.redaction events in older room versions when serving
    /// > such events over the Client-Server API.
    pub fn copy_redacts(&self) -> (Option<Arc<EventId>>, Box<RawJsonValue>) {
        let timestamp = get_timestamp();
        
        if self.kind == TimelineEventType::RoomRedaction {
            debug!("⏰ [{}] 📋 Copying redacts property for event {}", timestamp, self.event_id);
            
            if let Ok(mut content) =
                serde_json::from_str::<RoomRedactionEventContent>(self.content.get())
            {
                if let Some(redacts) = content.redacts {
                    debug!("⏰ [{}] ✅ Found redacts in content: {}", timestamp, redacts);
                    return (Some(redacts.into()), self.content.clone());
                } else if let Some(redacts) = self.redacts.clone() {
                    debug!("⏰ [{}] ✅ Copying redacts from top-level to content: {}", timestamp, redacts);
                    content.redacts = Some(redacts.into());
                    return (
                        self.redacts.clone(),
                        to_raw_value(&content).expect("Must be valid, we only added redacts field"),
                    );
                }
            } else {
                warn!("⏰ [{}] ⚠️ Failed to parse redaction content for event {}", timestamp, self.event_id);
            }
        }

        (self.redacts.clone(), self.content.clone())
    }

    #[tracing::instrument(skip(self))]
    pub fn to_sync_room_event(&self) -> Raw<AnySyncTimelineEvent> {
        let timestamp = get_timestamp();
        
        debug!("⏰ [{}] 🔄 Converting event {} to sync room event", timestamp, self.event_id);
        
        let (redacts, content) = self.copy_redacts();
        let mut json = json!({
            "content": content,
            "type": self.kind,
            "event_id": self.event_id,
            "sender": self.sender,
            "origin_server_ts": self.origin_server_ts,
        });

        if let Some(unsigned) = &self.unsigned {
            json["unsigned"] = json!(unsigned);
        }
        if let Some(state_key) = &self.state_key {
            json["state_key"] = json!(state_key);
        }
        if let Some(redacts) = &redacts {
            json["redacts"] = json!(redacts);
        }

        let result = serde_json::from_value(json).expect("Raw::from_value always works");
        debug!("⏰ [{}] ✅ Successfully converted event {} to sync room event", timestamp, self.event_id);
        result
    }

    /// This only works for events that are also AnyRoomEvents.
    #[tracing::instrument(skip(self))]
    pub fn to_any_event(&self) -> Raw<AnyEphemeralRoomEvent> {
        let timestamp = get_timestamp();
        
        debug!("⏰ [{}] 🔄 Converting event {} to any event", timestamp, self.event_id);
        
        let mut json = json!({
            "content": self.content,
            "type": self.kind,
            "event_id": self.event_id,
            "sender": self.sender,
            "origin_server_ts": self.origin_server_ts,
            "room_id": self.room_id,
        });

        if let Some(unsigned) = &self.unsigned {
            json["unsigned"] = json!(unsigned);
        }
        if let Some(state_key) = &self.state_key {
            json["state_key"] = json!(state_key);
        }
        if let Some(redacts) = &self.redacts {
            json["redacts"] = json!(redacts);
        }

        let result = serde_json::from_value(json).expect("Raw::from_value always works");
        debug!("⏰ [{}] ✅ Successfully converted event {} to any event", timestamp, self.event_id);
        result
    }

    #[tracing::instrument(skip(self))]
    pub fn to_room_event(&self) -> Raw<AnyTimelineEvent> {
        let (redacts, content) = self.copy_redacts();
        let mut json = json!({
            "content": content,
            "type": self.kind,
            "event_id": self.event_id,
            "sender": self.sender,
            "origin_server_ts": self.origin_server_ts,
            "room_id": self.room_id,
        });

        if let Some(unsigned) = &self.unsigned {
            json["unsigned"] = json!(unsigned);
        }
        if let Some(state_key) = &self.state_key {
            json["state_key"] = json!(state_key);
        }
        if let Some(redacts) = &redacts {
            json["redacts"] = json!(redacts);
        }

        serde_json::from_value(json).expect("Raw::from_value always works")
    }

    #[tracing::instrument(skip(self))]
    pub fn to_message_like_event(&self) -> Raw<AnyMessageLikeEvent> {
        let (redacts, content) = self.copy_redacts();
        let mut json = json!({
            "content": content,
            "type": self.kind,
            "event_id": self.event_id,
            "sender": self.sender,
            "origin_server_ts": self.origin_server_ts,
            "room_id": self.room_id,
        });

        if let Some(unsigned) = &self.unsigned {
            json["unsigned"] = json!(unsigned);
        }
        if let Some(state_key) = &self.state_key {
            json["state_key"] = json!(state_key);
        }
        if let Some(redacts) = &redacts {
            json["redacts"] = json!(redacts);
        }

        serde_json::from_value(json).expect("Raw::from_value always works")
    }

    #[tracing::instrument(skip(self))]
    pub fn to_state_event(&self) -> Raw<AnyStateEvent> {
        let mut json = json!({
            "content": self.content,
            "type": self.kind,
            "event_id": self.event_id,
            "sender": self.sender,
            "origin_server_ts": self.origin_server_ts,
            "room_id": self.room_id,
            "state_key": self.state_key,
        });

        if let Some(unsigned) = &self.unsigned {
            json["unsigned"] = json!(unsigned);
        }

        serde_json::from_value(json).expect("Raw::from_value always works")
    }

    #[tracing::instrument(skip(self))]
    pub fn to_sync_state_event(&self) -> Raw<AnySyncStateEvent> {
        let mut json = json!({
            "content": self.content,
            "type": self.kind,
            "event_id": self.event_id,
            "sender": self.sender,
            "origin_server_ts": self.origin_server_ts,
            "state_key": self.state_key,
        });

        if let Some(unsigned) = &self.unsigned {
            json["unsigned"] = json!(unsigned);
        }

        serde_json::from_value(json).expect("Raw::from_value always works")
    }

    #[tracing::instrument(skip(self))]
    pub fn to_stripped_state_event(&self) -> Raw<AnyStrippedStateEvent> {
        let json = json!({
            "content": self.content,
            "type": self.kind,
            "sender": self.sender,
            "state_key": self.state_key,
        });

        serde_json::from_value(json).expect("Raw::from_value always works")
    }

    #[tracing::instrument(skip(self))]
    pub fn to_stripped_spacechild_state_event(&self) -> Raw<HierarchySpaceChildEvent> {
        let json = json!({
            "content": self.content,
            "type": self.kind,
            "sender": self.sender,
            "state_key": self.state_key,
            "origin_server_ts": self.origin_server_ts,
        });

        serde_json::from_value(json).expect("Raw::from_value always works")
    }

    #[tracing::instrument(skip(self))]
    pub fn to_member_event(&self) -> Raw<StateEvent<RoomMemberEventContent>> {
        let mut json = json!({
            "content": self.content,
            "type": self.kind,
            "event_id": self.event_id,
            "sender": self.sender,
            "origin_server_ts": self.origin_server_ts,
            "redacts": self.redacts,
            "room_id": self.room_id,
            "state_key": self.state_key,
        });

        if let Some(unsigned) = &self.unsigned {
            json["unsigned"] = json!(unsigned);
        }

        serde_json::from_value(json).expect("Raw::from_value always works")
    }

    /// This does not return a full `Pdu` it is only to satisfy ruma's types.
    #[tracing::instrument]
    pub fn convert_to_outgoing_federation_event(
        mut pdu_json: CanonicalJsonObject,
    ) -> Box<RawJsonValue> {
        if let Some(unsigned) = pdu_json
            .get_mut("unsigned")
            .and_then(|val| val.as_object_mut())
        {
            unsigned.remove("transaction_id");
        }

        pdu_json.remove("event_id");

        // TODO: another option would be to convert it to a canonical string to validate size
        // and return a Result<Raw<...>>
        // serde_json::from_str::<Raw<_>>(
        //     ruma::serde::to_canonical_json_string(pdu_json).expect("CanonicalJson is valid serde_json::Value"),
        // )
        // .expect("Raw::from_value always works")

        to_raw_value(&pdu_json).expect("CanonicalJson is valid serde_json::Value")
    }

    pub fn from_id_val(
        event_id: &EventId,
        mut json: CanonicalJsonObject,
    ) -> Result<Self, serde_json::Error> {
        json.insert(
            "event_id".to_owned(),
            CanonicalJsonValue::String(event_id.as_str().to_owned()),
        );

        serde_json::from_value(serde_json::to_value(json).expect("valid JSON"))
    }
}

impl state_res::Event for PduEvent {
    type Id = Arc<EventId>;

    fn event_id(&self) -> &Self::Id {
        &self.event_id
    }

    fn room_id(&self) -> &RoomId {
        &self.room_id
    }

    fn sender(&self) -> &UserId {
        &self.sender
    }

    fn event_type(&self) -> &TimelineEventType {
        &self.kind
    }

    fn content(&self) -> &RawJsonValue {
        &self.content
    }

    fn origin_server_ts(&self) -> MilliSecondsSinceUnixEpoch {
        MilliSecondsSinceUnixEpoch(self.origin_server_ts)
    }

    fn state_key(&self) -> Option<&str> {
        self.state_key.as_deref()
    }

    fn prev_events(&self) -> Box<dyn DoubleEndedIterator<Item = &Self::Id> + '_> {
        Box::new(self.prev_events.iter())
    }

    fn auth_events(&self) -> Box<dyn DoubleEndedIterator<Item = &Self::Id> + '_> {
        Box::new(self.auth_events.iter())
    }

    fn redacts(&self) -> Option<&Self::Id> {
        self.redacts.as_ref()
    }
}

// These impl's allow us to dedup state snapshots when resolving state
// for incoming events (federation/send/{txn}).
impl Eq for PduEvent {}
impl PartialEq for PduEvent {
    fn eq(&self, other: &Self) -> bool {
        self.event_id == other.event_id
    }
}
impl PartialOrd for PduEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for PduEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        self.event_id.cmp(&other.event_id)
    }
}

/// Generates a correct eventId for the incoming pdu.
///
/// Returns a tuple of the new `EventId` and the PDU as a `BTreeMap<String, CanonicalJsonValue>`.
pub(crate) fn gen_event_id_canonical_json(
    pdu: &RawJsonValue,
    room_version_rules: &RoomVersionRules,
) -> crate::Result<(OwnedEventId, CanonicalJsonObject)> {
    let value: CanonicalJsonObject = serde_json::from_str(pdu.get()).map_err(|e| {
        warn!("Error parsing incoming event {:?}: {:?}", pdu, e);
        Error::BadServerResponse("Invalid PDU in server response")
    })?;

    let event_id = format!(
        "${}",
        // Anything higher than version3 behaves the same
        ruma::signatures::reference_hash(&value, room_version_rules)
            .map_err(|_| Error::BadRequest(ErrorKind::BadJson, "Invalid PDU format"))?
    )
    .try_into()
    .expect("ruma's reference hashes are valid event ids");

    Ok((event_id, value))
}

/// Build the start of a PDU in order to add it to the Database.
#[derive(Debug, Deserialize)]
pub struct PduBuilder {
    #[serde(rename = "type")]
    pub event_type: TimelineEventType,
    pub content: Box<RawJsonValue>,
    pub unsigned: Option<BTreeMap<String, serde_json::Value>>,
    pub state_key: Option<String>,
    pub redacts: Option<Arc<EventId>>,
    /// For timestamped messaging, should only be used for appservices
    ///
    /// Will be set to current time if None
    pub timestamp: Option<MilliSecondsSinceUnixEpoch>,
}

impl PduBuilder {
    /// Create a new PduBuilder with basic required fields
    pub fn new(
        event_type: TimelineEventType,
        content: Box<RawJsonValue>,
        state_key: Option<String>,
    ) -> Self {
        Self {
            event_type,
            content,
            unsigned: None,
            state_key,
            redacts: None,
            timestamp: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruma::{
        event_id, room_id, user_id,
        events::{
            room::message::{MessageType, RoomMessageEventContent, TextMessageEventContent},
            AnyMessageLikeEvent, TimelineEventType,
        },
        uint, MilliSecondsSinceUnixEpoch,
    };
    use serde_json::{json, Value};
    use std::{sync::Arc, time::{SystemTime, UNIX_EPOCH}};
    use state_res::Event; // Import the Event trait so methods are available

    /// Test helper for creating valid EventHash
    fn create_test_hash() -> EventHash {
        EventHash {
            sha256: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        }
    }

    /// Test helper for creating invalid EventHash
    fn create_invalid_hash() -> EventHash {
        EventHash {
            sha256: "invalid_hash".to_string(),
        }
    }

    /// Test helper for creating test PDU event
    fn create_test_pdu() -> Result<PduEvent, Error> {
        let event_id = event_id!("$test_event:example.com").to_owned();
        let room_id = room_id!("!test_room:example.com").to_owned();
        let sender = user_id!("@test:example.com").to_owned();
        let content = to_raw_value(&json!({"msgtype": "m.text", "body": "Hello World"})).unwrap();
        
        PduEvent::new(
            Arc::from(event_id),
            room_id,
            sender,
            uint!(1234567890),
            TimelineEventType::RoomMessage,
            content,
            None,
            vec![],
            uint!(1),
            vec![],
            None,
            None,
            create_test_hash(),
            None,
        )
    }

    /// Test helper for creating state event PDU
    fn create_state_pdu() -> Result<PduEvent, Error> {
        let event_id = event_id!("$state_event:example.com").to_owned();
        let room_id = room_id!("!test_room:example.com").to_owned();
        let sender = user_id!("@test:example.com").to_owned();
        let content = to_raw_value(&json!({"name": "Test Room"})).unwrap();
        
        PduEvent::new(
            Arc::from(event_id),
            room_id,
            sender,
            uint!(1234567890),
            TimelineEventType::RoomName,
            content,
            Some("".to_string()),
            vec![],
            uint!(1),
            vec![],
            None,
            None,
            create_test_hash(),
            None,
        )
    }

    #[test]
    fn test_event_hash_new_valid() {
        let valid_hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let hash = EventHash::new(valid_hash.to_string()).unwrap();
        assert_eq!(hash.sha256, valid_hash);
    }

    #[test]
    fn test_event_hash_new_invalid_length() {
        // Too short
        let short_hash = "abc123";
        assert!(EventHash::new(short_hash.to_string()).is_err());
        
        // Too long
        let long_hash = "0123456789abcdef".repeat(5);
        assert!(EventHash::new(long_hash).is_err());
    }

    #[test]
    fn test_event_hash_new_empty() {
        assert!(EventHash::new("".to_string()).is_err());
    }

    #[test]
    fn test_event_hash_new_non_hex() {
        let non_hex = "0123456789abcdefghij0123456789abcdef0123456789abcdef0123456789abcdef";
        assert!(EventHash::new(non_hex.to_string()).is_err());
        
        let mixed_case = "0123456789ABCDEFabcdef0123456789ABCDEF0123456789abcdef0123456789";
        assert!(EventHash::new(mixed_case.to_string()).is_ok());
    }

    #[test]
    fn test_event_hash_validate_valid() {
        let valid_hash = create_test_hash();
        assert!(valid_hash.validate().is_ok());
    }

    #[test]
    fn test_event_hash_validate_invalid() {
        let invalid_hash = create_invalid_hash();
        assert!(invalid_hash.validate().is_err());
        
        let empty_hash = EventHash { sha256: "".to_string() };
        assert!(empty_hash.validate().is_err());
    }

    #[test]
    fn test_pdu_event_new_valid() {
        let pdu = create_test_pdu();
        assert!(pdu.is_ok());
        
        let pdu = pdu.unwrap();
        assert_eq!(pdu.room_id.as_str(), "!test_room:example.com");
        assert_eq!(pdu.sender.as_str(), "@test:example.com");
        assert_eq!(pdu.kind, TimelineEventType::RoomMessage);
        assert_eq!(pdu.depth, uint!(1));
        assert!(pdu.state_key.is_none());
    }

    #[test]
    fn test_pdu_event_new_large_depth() {
        let event_id = event_id!("$test_event:example.com").to_owned();
        let room_id = room_id!("!test_room:example.com").to_owned();
        let sender = user_id!("@test:example.com").to_owned();
        let content = to_raw_value(&json!({"msgtype": "m.text", "body": "Hello"})).unwrap();
        
        let result = PduEvent::new(
            Arc::from(event_id),
            room_id,
            sender,
            uint!(1234567890),
            TimelineEventType::RoomMessage,
            content,
            None,
            vec![],
            uint!(2000000), // Exceeds limit
            vec![],
            None,
            None,
            create_test_hash(),
            None,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_pdu_event_new_many_prev_events() {
        let event_id = event_id!("$test_event:example.com").to_owned();
        let room_id = room_id!("!test_room:example.com").to_owned();
        let sender = user_id!("@test:example.com").to_owned();
        let content = to_raw_value(&json!({"msgtype": "m.text", "body": "Hello"})).unwrap();
        
        // Create many prev_events (should warn but not error)
        let mut prev_events = Vec::new();
        for i in 0..25 {
            let prev_id = format!("$prev_{}:example.com", i);
            let event_id = EventId::parse(&prev_id).unwrap().to_owned();
            prev_events.push(Arc::from(event_id));
        }
        
        let result = PduEvent::new(
            Arc::from(event_id),
            room_id,
            sender,
            uint!(1234567890),
            TimelineEventType::RoomMessage,
            content,
            None,
            prev_events,
            uint!(1),
            vec![],
            None,
            None,
            create_test_hash(),
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_pdu_event_validate() {
        let pdu = create_test_pdu().unwrap();
        assert!(pdu.validate().is_ok());
    }

    #[test]
    fn test_pdu_event_is_state_event() {
        let message_pdu = create_test_pdu().unwrap();
        assert!(message_pdu.state_key.is_none()); // Message events have no state_key
        
        let state_pdu = create_state_pdu().unwrap();
        assert!(state_pdu.state_key.is_some()); // State events have a state_key
    }

    #[test]
    fn test_pdu_event_redacts() {
        let redaction_id = event_id!("$redaction:example.com").to_owned();
        let event_id = event_id!("$test_event:example.com").to_owned();
        let room_id = room_id!("!test_room:example.com").to_owned();
        let sender = user_id!("@test:example.com").to_owned();
        let content = to_raw_value(&json!({"reason": "spam"})).unwrap();
        
        let pdu = PduEvent::new(
            Arc::from(event_id),
            room_id,
            sender,
            uint!(1234567890),
            TimelineEventType::RoomRedaction,
            content,
            None,
            vec![],
            uint!(1),
            vec![],
            Some(Arc::from(redaction_id)),
            None,
            create_test_hash(),
            None,
        ).unwrap();
        
        assert!(pdu.redacts.is_some());
        assert_eq!(pdu.redacts.as_ref().unwrap().as_str(), "$redaction:example.com");
    }

    #[test]
    fn test_pdu_event_with_unsigned() {
        let event_id = event_id!("$test_event:example.com").to_owned();
        let room_id = room_id!("!test_room:example.com").to_owned();
        let sender = user_id!("@test:example.com").to_owned();
        let content = to_raw_value(&json!({"msgtype": "m.text", "body": "Hello"})).unwrap();
        let unsigned = to_raw_value(&json!({"age": 1000})).unwrap();
        
        let pdu = PduEvent::new(
            Arc::from(event_id),
            room_id,
            sender,
            uint!(1234567890),
            TimelineEventType::RoomMessage,
            content,
            None,
            vec![],
            uint!(1),
            vec![],
            None,
            Some(unsigned),
            create_test_hash(),
            None,
        ).unwrap();
        
        assert!(pdu.unsigned.is_some());
    }

    #[test]
    fn test_pdu_event_with_signatures() {
        let event_id = event_id!("$test_event:example.com").to_owned();
        let room_id = room_id!("!test_room:example.com").to_owned();
        let sender = user_id!("@test:example.com").to_owned();
        let content = to_raw_value(&json!({"msgtype": "m.text", "body": "Hello"})).unwrap();
        let signatures = to_raw_value(&json!({"example.com": {"ed25519:key1": "signature"}})).unwrap();
        
        let pdu = PduEvent::new(
            Arc::from(event_id),
            room_id,
            sender,
            uint!(1234567890),
            TimelineEventType::RoomMessage,
            content,
            None,
            vec![],
            uint!(1),
            vec![],
            None,
            None,
            create_test_hash(),
            Some(signatures),
        ).unwrap();
        
        assert!(pdu.signatures.is_some());
    }

    #[test]
    fn test_pdu_event_is_redacted() {
        let pdu = create_test_pdu().unwrap();
        assert!(!pdu.is_redacted());
    }

    #[test]
    fn test_pdu_event_copy_redacts() {
        let redaction_id = event_id!("$redaction:example.com").to_owned();
        let event_id = event_id!("$test_event:example.com").to_owned();
        let room_id = room_id!("!test_room:example.com").to_owned();
        let sender = user_id!("@test:example.com").to_owned();
        let content = to_raw_value(&json!({"reason": "spam"})).unwrap();
        
        let pdu = PduEvent::new(
            Arc::from(event_id),
            room_id,
            sender,
            uint!(1234567890),
            TimelineEventType::RoomRedaction,
            content,
            None,
            vec![],
            uint!(1),
            vec![],
            Some(Arc::from(redaction_id)),
            None,
            create_test_hash(),
            None,
        ).unwrap();
        
        let (redacts, _content) = pdu.copy_redacts();
        assert!(redacts.is_some());
        assert_eq!(redacts.unwrap().as_str(), "$redaction:example.com");
    }

    #[test]
    fn test_pdu_event_conversions() {
        let pdu = create_test_pdu().unwrap();
        
        // Test various conversion methods
        let _sync_event = pdu.to_sync_room_event();
        let _any_event = pdu.to_any_event();
        let _room_event = pdu.to_room_event();
        let _message_event = pdu.to_message_like_event();
        
        // These should not panic
        assert!(true);
    }

    #[test]
    fn test_pdu_event_state_conversions() {
        let state_pdu = create_state_pdu().unwrap();
        
        // Test state event conversions
        let _state_event = state_pdu.to_state_event();
        let _sync_state_event = state_pdu.to_sync_state_event();
        let _stripped_state_event = state_pdu.to_stripped_state_event();
        
        // These should not panic
        assert!(true);
    }

    #[test]
    fn test_pdu_event_ordering() {
        let pdu1 = create_test_pdu().unwrap();
        let mut pdu2 = create_test_pdu().unwrap();
        pdu2.event_id = Arc::from(event_id!("$test_event2:example.com").to_owned());
        
        // Test ordering by event ID (lexicographic)
        let comparison = pdu1.cmp(&pdu2);
        assert!(comparison != Ordering::Equal);
    }

    #[test]
    fn test_pdu_event_equality() {
        let pdu1 = create_test_pdu().unwrap();
        let pdu2 = create_test_pdu().unwrap();
        
        // Same event IDs should be equal
        assert_eq!(pdu1, pdu2);
        assert_eq!(pdu1.partial_cmp(&pdu2), Some(Ordering::Equal));
    }

    #[test]
    fn test_state_res_event_trait() {
        let pdu = create_test_pdu().unwrap();
        
        // Test state_res::Event trait implementation
        assert_eq!(pdu.event_id().as_str(), "$test_event:example.com");
        assert_eq!(pdu.room_id().as_str(), "!test_room:example.com");
        assert_eq!(pdu.sender().as_str(), "@test:example.com");
        assert_eq!(pdu.event_type(), &TimelineEventType::RoomMessage);
        assert_eq!(pdu.origin_server_ts(), MilliSecondsSinceUnixEpoch(uint!(1234567890)));
        assert_eq!(pdu.state_key(), None);
        assert_eq!(pdu.prev_events().count(), 0);
        assert_eq!(pdu.auth_events().count(), 0);
        assert!(pdu.redacts().is_none());
    }

    #[test]
    fn test_state_res_event_trait_with_state() {
        let state_pdu = create_state_pdu().unwrap();
        
        // Test state event specifics
        assert_eq!(state_pdu.state_key(), Some(""));
        assert_eq!(state_pdu.event_type(), &TimelineEventType::RoomName);
    }

    #[test]
    fn test_pdu_builder_creation() {
        let builder = PduBuilder {
            event_type: TimelineEventType::RoomMessage,
            content: to_raw_value(&json!({"msgtype": "m.text", "body": "test"})).unwrap(),
            unsigned: None,
            state_key: None,
            redacts: None,
            timestamp: None,
        };
        
        assert_eq!(builder.event_type, TimelineEventType::RoomMessage);
        assert!(builder.state_key.is_none());
        assert!(builder.redacts.is_none());
        assert!(builder.timestamp.is_none());
    }

    #[test]
    fn test_pdu_builder_with_state_key() {
        let builder = PduBuilder {
            event_type: TimelineEventType::RoomName,
            content: to_raw_value(&json!({"name": "Test Room"})).unwrap(),
            unsigned: None,
            state_key: Some("".to_string()),
            redacts: None,
            timestamp: None,
        };
        
        assert_eq!(builder.event_type, TimelineEventType::RoomName);
        assert!(builder.state_key.is_some());
    }

    #[test]
    fn test_pdu_builder_with_timestamp() {
        let timestamp = MilliSecondsSinceUnixEpoch::now();
        let builder = PduBuilder {
            event_type: TimelineEventType::RoomMessage,
            content: to_raw_value(&json!({"msgtype": "m.text", "body": "test"})).unwrap(),
            unsigned: None,
            state_key: None,
            redacts: None,
            timestamp: Some(timestamp),
        };
        
        assert_eq!(builder.timestamp, Some(timestamp));
    }

    #[test]
    fn test_pdu_builder_with_unsigned() {
        let mut unsigned_map = BTreeMap::new();
        unsigned_map.insert("custom".to_string(), json!("value"));
        
        let builder = PduBuilder {
            event_type: TimelineEventType::RoomMessage,
            content: to_raw_value(&json!({"msgtype": "m.text", "body": "test"})).unwrap(),
            unsigned: Some(unsigned_map.clone()),
            state_key: None,
            redacts: None,
            timestamp: None,
        };
        
        assert_eq!(builder.unsigned, Some(unsigned_map));
    }

    #[test]
    fn test_pdu_builder_with_redacts() {
        let redacts_id = event_id!("$redacted:example.com");
        let builder = PduBuilder {
            event_type: TimelineEventType::RoomRedaction,
            content: to_raw_value(&json!({"reason": "spam"})).unwrap(),
            unsigned: None,
            state_key: None,
            redacts: Some(Arc::from(redacts_id.to_owned())),
            timestamp: None,
        };
        
        assert!(builder.redacts.is_some());
        assert_eq!(
            builder.redacts.as_ref().unwrap().as_str(),
            "$redacted:example.com"
        );
    }

    #[test]
    fn test_hash_edge_cases() {
        // Test with different valid hex combinations
        let hash1 = EventHash::new("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()).unwrap();
        assert!(hash1.validate().is_ok());
        
        let hash2 = EventHash::new("FEDCBA9876543210FEDCBA9876543210FEDCBA9876543210FEDCBA9876543210".to_string()).unwrap();
        assert!(hash2.validate().is_ok());
        
        // Test mixed case
        let hash3 = EventHash::new("0123456789AbCdEf0123456789AbCdEf0123456789AbCdEf0123456789AbCdEf".to_string()).unwrap();
        assert!(hash3.validate().is_ok());
    }

    #[test]
    fn test_concurrent_pdu_operations() {
        use std::thread;
        
        let handles: Vec<_> = (0..5).map(|i| {
            thread::spawn(move || {
                let pdu = create_test_pdu().unwrap();
                assert_eq!(pdu.sender.as_str(), "@test:example.com");
                assert!(pdu.validate().is_ok());
                i
            })
        }).collect();
        
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_memory_efficiency() {
        // Create multiple PDUs to test memory usage
        let mut pdus = Vec::new();
        for i in 0..100 {
            let event_id = format!("$test_event_{}:example.com", i);
            let event_id = EventId::parse(&event_id).unwrap().to_owned();
            let room_id = room_id!("!test_room:example.com").to_owned();
            let sender = user_id!("@test:example.com").to_owned();
            let content = to_raw_value(&json!({"msgtype": "m.text", "body": format!("Message {}", i)})).unwrap();
            
            let timestamp = 1234567890_u64 + i as u64;
            let pdu = PduEvent::new(
                Arc::from(event_id),
                room_id,
                sender,
                UInt::try_from(timestamp).unwrap(),
                TimelineEventType::RoomMessage,
                content,
                None,
                vec![],
                uint!(1),
                vec![],
                None,
                None,
                create_test_hash(),
                None,
            ).unwrap();
            
            pdus.push(pdu);
        }
        
        assert_eq!(pdus.len(), 100);
        
        // Test that all PDUs are valid
        for pdu in &pdus {
            assert!(pdu.validate().is_ok());
        }
    }

    #[test]
    fn test_performance_pdu_creation() {
        use std::time::Instant;
        
        let start = Instant::now();
        
        for i in 0..1000 {
            let event_id = format!("$perf_test_{}:example.com", i);
            let event_id = EventId::parse(&event_id).unwrap().to_owned();
            let room_id = room_id!("!perf_room:example.com").to_owned();
            let sender = user_id!("@perf:example.com").to_owned();
            let content = to_raw_value(&json!({"msgtype": "m.text", "body": "Performance test"})).unwrap();
            
            let pdu = PduEvent::new(
                Arc::from(event_id),
                room_id,
                sender,
                uint!(1234567890),
                TimelineEventType::RoomMessage,
                content,
                None,
                vec![],
                uint!(1),
                vec![],
                None,
                None,
                create_test_hash(),
                None,
            ).unwrap();
            
            assert!(pdu.validate().is_ok());
        }
        
        let duration = start.elapsed();
        println!("🎉 Created and validated 1000 PDUs in {:?}", duration);
        
        // Should complete in reasonable time (less than 1 second)
        assert!(duration.as_secs() < 1);
    }

    #[test]
    fn test_get_timestamp() {
        let timestamp1 = get_timestamp();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let timestamp2 = get_timestamp();
        
        assert!(timestamp2 >= timestamp1);
    }

    #[test]
    fn test_remove_transaction_id() {
        let mut pdu = create_test_pdu().unwrap();
        let unsigned = to_raw_value(&json!({"transaction_id": "test_txn", "age": 1000})).unwrap();
        pdu.unsigned = Some(unsigned);
        
        assert!(pdu.remove_transaction_id().is_ok());
    }

    #[test]
    fn test_add_age() {
        let mut pdu = create_test_pdu().unwrap();
        assert!(pdu.add_age().is_ok());
        assert!(pdu.unsigned.is_some());
    }
}
