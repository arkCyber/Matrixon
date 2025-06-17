//! Federation Ephemeral Data Unit (EDU) types
//!
//! Matrixon - Matrix NextServer (Synapse alternative)
//! Author: arkSong (arksong2018@gmail.com)
//! Date: 2025-06-14
//! Version: 0.1.0
//! 
//! Reference: 
//! - Matrix Spec: https://spec.matrix.org/v1.8/server-server-api/#ephemeral-data-units-edus
//! - Synapse: https://github.com/element-hq/synapse/blob/develop/synapse/federation/units.py

use ruma_identifiers::{RoomId, UserId, ServerName, EventId};
use serde::{Serialize, Deserialize};
use tracing::instrument;

/// Federation Ephemeral Data Unit types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum FederationEdu {
    /// Read receipt EDU
    ReadReceipt {
        room_id: RoomId,
        user_id: UserId,
        event_id: EventId,
        timestamp: u64,
    },
    /// Typing notification EDU
    Typing {
        room_id: RoomId,
        user_id: UserId,
        typing: bool,
        timeout: u64,
    },
    /// Presence update EDU
    Presence {
        user_id: UserId,
        presence: String,
        status_msg: Option<String>,
        last_active_ago: Option<u64>,
        currently_active: Option<bool>,
    },
}

impl FederationEdu {
    #[instrument(level = "debug")]
    pub fn destination_server(&self) -> &ServerName {
        match self {
            FederationEdu::ReadReceipt { room_id, .. } => room_id.server_name(),
            FederationEdu::Typing { room_id, .. } => room_id.server_name(),
            FederationEdu::Presence { user_id, .. } => user_id.server_name(),
        }
    }
}
