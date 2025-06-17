// Matrixon Admin Module
// Author: arkSong (arksong2018@gmail.com)
// Date: 2024
// Version: 1.0
// Purpose: Main admin module that exports all admin functionality

use std::sync::Arc;

use crate::config::Config;
use crate::service::admin::service::Service;

mod commands;
mod handlers;
mod utils;
pub mod service;

pub use commands::*;
pub use handlers::*;
pub use utils::*;
pub use service::Service as AdminServiceImpl;

/// Admin service for managing server operations
pub struct AdminService {
    config: Arc<Config>,
    service: Arc<Service>,
}

impl AdminService {
    /// Create a new admin service instance
    pub fn new(config: Arc<Config>, service: Arc<Service>) -> Self {
        Self { config, service }
    }

    /// Get the service instance
    pub fn service(&self) -> &Arc<Service> {
        &self.service
    }
}

