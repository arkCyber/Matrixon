// Matrixon Admin Commands Module
// Author: arkSong (arksong2018@gmail.com)
// Date: 2024
// Version: 1.0
// Purpose: Define admin command structures and types

use std::{
    sync::Arc,
    time::Duration,
};

use crate::{
    config::Config,
    utils::error::Error,
    service::{
        admin::{
            // model::{AdminUser, AdminUserRole},
            // repository::AdminRepository,
            service::Service,
        },
        rooms::RoomService,
        users::UserService,
    },
};
use clap::{Args, Parser};
use ruma::{UserId, ServerName};
use ruma_events::room::message::RoomMessageEventContent;

/// Admin commands for managing server operations
pub struct AdminCommandService {
    config: Arc<Config>,
    service: Arc<Service>,
}

impl AdminCommandService {
    /// Create a new admin commands instance
    pub fn new(config: Arc<Config>, service: Arc<Service>) -> Self {
        Self { config, service }
    }

    /// Get the service instance
    pub fn service(&self) -> &Arc<Service> {
        &self.service
    }
}

#[derive(Parser)]
pub struct AdminCommandArgs {
    #[clap(long, value_name = "COMMAND")]
    pub command: String,

    #[clap(short, long, help = "Run in quiet mode")]
    pub quiet: bool,

    #[clap(short, long, value_name = "SECONDS", help = "Timeout in seconds")]
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone)]
pub enum AdminCommand {
    /// Register an appservice using its registration YAML
    RegisterAppservice,

    /// Unregister an appservice using its ID
    UnregisterAppservice {
        /// The appservice to unregister
        appservice_identifier: String,
    },

    /// List all the currently registered appservices
    ListAppservices,

    // ========== User Management API (Enhanced) ==========
    
    /// Get detailed user information including profile and statistics
    GetUser {
        /// User ID to get information for
        user_id: Box<UserId>,
    },

    /// Update user information and settings
    UpdateUser {
        /// User ID to update
        user_id: Box<UserId>,
        /// New display name (optional)
        display_name: Option<String>,
        /// New avatar URL (optional)
        avatar_url: Option<String>,
        /// Set admin status (optional)
        admin: Option<bool>,
        /// Set user as deactivated (optional)
        deactivated: Option<bool>,
    },

    /// List users with advanced filtering and pagination
    ListUsers {
        /// Start index for pagination
        start: u64,
        /// Maximum number of users to return
        limit: u64,
        /// Filter by admin status
        admin: Option<bool>,
        /// Filter by deactivated status
        deactivated: Option<bool>,
        /// Search term for user display name or ID
        query: Option<String>,
        /// Filter by user type (guest, regular, admin, bot)
        user_type: Option<String>,
    },

    // ... Add other commands here
}

#[derive(Debug, Clone, Args)]
pub struct DeactivatePurgeMediaArgs {
    #[clap(long, short = 'm')]
    /// Purges all media uploaded by the user(s) after deactivating their account
    pub purge_media: bool,

    #[clap(
        long, short = 't',
        value_parser = humantime::parse_duration,
        requires = "purge_media"
    )]
    pub media_from_last: Option<Duration>,

    #[clap(long, short = 'f', requires = "purge_media")]
    pub force_filehash: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ListMediaArgs {
    #[clap(short, long)]
    /// The user that uploaded the media
    pub user: Option<Box<UserId>>,

    #[clap(short, long)]
    /// The server from which the media originated
    pub server: Option<Box<ServerName>>,
}

#[derive(Debug, Clone)]
pub enum AdminRoomEvent {
    ProcessMessage(String),
    SendMessage(RoomMessageEventContent),
}
