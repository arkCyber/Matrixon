use std::sync::Arc;

use anyhow::Result;
use matrix_sdk::ruma::{
    events::AnySyncStateEvent,
    OwnedRoomId, OwnedUserId, RoomId, UserId,
};
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

use crate::{
    config::Config,
    db::Database,
    error::Error,
    service::{
        admin::{
            model::{AdminUser, AdminUserRole},
            repository::AdminRepository,
        },
        room::RoomService,
        user::UserService,
    },
};

// ... rest of the file remains unchanged ... 
