// Matrixon Admin Utils Module
// Author: arkSong (arksong2018@gmail.com)
// Date: 2024
// Version: 1.0
// Purpose: Implement admin utility functions

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use ruma::{
    events::room::message::RoomMessageEventContent,
    identifiers::{MxcUri, ServerName, UserId},
    OwnedServerName,
};
use crate::Result;
use crate::Error;
use std::sync::Arc;
use crate::config::Config;
use crate::service::admin::service::Service;

/// Admin utilities for managing server operations
pub struct AdminUtils {
    config: Arc<Config>,
    service: Arc<Service>,
}

impl AdminUtils {
    /// Create a new admin utilities instance
    pub fn new(config: Arc<Config>, service: Arc<Service>) -> Self {
        Self { config, service }
    }

    /// Get the service instance
    pub fn service(&self) -> &Arc<Service> {
        &self.service
    }
}

pub fn userids_from_body<'a>(
    body: &'a [&'a str],
) -> Result<Result<Vec<&'a UserId>, RoomMessageEventContent>, Error> {
    let users = body.to_owned().drain(1..body.len() - 1).collect::<Vec<_>>();

    let mut user_ids = Vec::new();
    let mut remote_ids = Vec::new();
    let mut non_existent_ids = Vec::new();
    let mut invalid_users = Vec::new();

    for &user in &users {
        match <&UserId>::try_from(user) {
            Ok(user_id) => {
                if user_id.server_name() != crate::services().globals.server_name() {
                    remote_ids.push(user_id)
                } else if !crate::services().users.exists(user_id)? {
                    non_existent_ids.push(user_id)
                } else {
                    user_ids.push(user_id)
                }
            }
            Err(_) => {
                invalid_users.push(user);
            }
        }
    }

    let mut markdown_message = String::new();
    let mut html_message = String::new();
    if !invalid_users.is_empty() {
        markdown_message.push_str("The following user ids are not valid:\n```\n");
        html_message.push_str("The following user ids are not valid:\n<pre>\n");
        for invalid_user in invalid_users {
            markdown_message.push_str(&format!("{invalid_user}\n"));
            html_message.push_str(&format!("{invalid_user}\n"));
        }
        markdown_message.push_str("```\n\n");
        html_message.push_str("</pre>\n\n");
    }
    if !remote_ids.is_empty() {
        markdown_message.push_str("The following users are not from this server:\n```\n");
        html_message.push_str("The following users are not from this server:\n<pre>\n");
        for remote_id in remote_ids {
            markdown_message.push_str(&format!("{remote_id}\n"));
            html_message.push_str(&format!("{remote_id}\n"));
        }
        markdown_message.push_str("```\n\n");
        html_message.push_str("</pre>\n\n");
    }
    if !non_existent_ids.is_empty() {
        markdown_message.push_str("The following users do not exist:\n```\n");
        html_message.push_str("The following users do not exist:\n<pre>\n");
        for non_existent_id in non_existent_ids {
            markdown_message.push_str(&format!("{non_existent_id}\n"));
            html_message.push_str(&format!("{non_existent_id}\n"));
        }
        markdown_message.push_str("```\n\n");
        html_message.push_str("</pre>\n\n");
    }
    if !markdown_message.is_empty() {
        return Ok(Err(RoomMessageEventContent::text_html(
            markdown_message,
            html_message,
        )
        .into()));
    }

    Ok(Ok(user_ids))
}

pub fn media_from_body(body: Vec<&str>) -> Result<Vec<(OwnedServerName, String)>, RoomMessageEventContent> {
    if body.len() > 2 && body[0].trim() == "```" && body.last().unwrap().trim() == "```" {
        Ok(body
            .clone()
            .drain(1..body.len() - 1)
            .map(<Box<MxcUri>>::from)
            .filter_map(|mxc| {
                mxc.parts()
                    .map(|(server_name, media_id)| (server_name.to_owned(), media_id.to_owned()))
                    .ok()
            })
            .collect::<Vec<_>>())
    } else {
        Err(RoomMessageEventContent::text_plain(
            "Expected code block in command body. Add --help for details.",
        )
        .into())
    }
}

pub fn unix_secs_from_duration(duration: Duration) -> Result<u64, Error> {
    SystemTime::now()
        .checked_add(duration)
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .ok_or_else(|| Error::AdminCommand("Given timeframe cannot be represented as system time, please use a shorter timeframe"))
} 
