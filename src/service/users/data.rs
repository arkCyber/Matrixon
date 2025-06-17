// =============================================================================
// Matrixon Matrix NextServer - Data Module
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

use crate::Result;
use ruma::{
    api::client::{device::Device, filter::FilterDefinition},
    encryption::{CrossSigningKey, DeviceKeys, OneTimeKey},
    events::AnyToDeviceEvent,
    serde::Raw,
    DeviceId, MilliSecondsSinceUnixEpoch, OneTimeKeyAlgorithm, OwnedDeviceId, OwnedMxcUri,
    OwnedOneTimeKeyId, OwnedUserId, UInt, UserId,
};
use std::collections::BTreeMap;

pub trait Data: Send + Sync {
    /// Check if a user has an account on this NextServer.
    fn exists(&self, user_id: &UserId) -> Result<bool>;

    /// Check if account is deactivated
    fn is_deactivated(&self, user_id: &UserId) -> Result<bool>;

    /// Returns the number of users registered on this server.
    fn count(&self) -> Result<usize>;

    /// Find out which user an access token belongs to.
    fn find_from_token(&self, token: &str) -> Result<Option<(OwnedUserId, OwnedDeviceId)>>;

    /// Returns an iterator over all users on this NextServer.
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = Result<OwnedUserId>> + 'a>;

    /// Returns a list of local users as list of usernames.
    ///
    /// A user account is considered `local` if the length of it's password is greater then zero.
    fn list_local_users(&self) -> Result<Vec<String>>;

    /// Returns the password hash for the given user.
    fn password_hash(&self, user_id: &UserId) -> Result<Option<String>>;

    /// Hash and set the user's password to the Argon2 hash
    fn set_password(&self, user_id: &UserId, password: Option<&str>) -> Result<()>;

    /// Returns the displayname of a user on this NextServer.
    fn displayname(&self, user_id: &UserId) -> Result<Option<String>>;

    /// Sets a new displayname or removes it if displayname is None. You still need to nofify all rooms of this change.
    fn set_displayname(&self, user_id: &UserId, displayname: Option<String>) -> Result<()>;

    /// Get the avatar_url of a user.
    fn avatar_url(&self, user_id: &UserId) -> Result<Option<OwnedMxcUri>>;

    /// Sets a new avatar_url or removes it if avatar_url is None.
    fn set_avatar_url(&self, user_id: &UserId, avatar_url: Option<OwnedMxcUri>) -> Result<()>;

    /// Get the blurhash of a user.
    fn blurhash(&self, user_id: &UserId) -> Result<Option<String>>;

    /// Sets a new avatar_url or removes it if avatar_url is None.
    fn set_blurhash(&self, user_id: &UserId, blurhash: Option<String>) -> Result<()>;

    /// Adds a new device to a user.
    fn create_device(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
        token: &str,
        initial_device_display_name: Option<String>,
    ) -> Result<()>;

    /// Removes a device from a user.
    fn remove_device(&self, user_id: &UserId, device_id: &DeviceId) -> Result<()>;

    /// Returns an iterator over all device ids of this user.
    fn all_device_ids<'a>(
        &'a self,
        user_id: &UserId,
    ) -> Box<dyn Iterator<Item = Result<OwnedDeviceId>> + 'a>;

    /// Replaces the access token of one device.
    fn set_token(&self, user_id: &UserId, device_id: &DeviceId, token: &str) -> Result<()>;

    fn add_one_time_key(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
        one_time_key_key: &OwnedOneTimeKeyId,
        one_time_key_value: &Raw<OneTimeKey>,
    ) -> Result<()>;

    fn last_one_time_keys_update(&self, user_id: &UserId) -> Result<u64>;

    fn take_one_time_key(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
        key_algorithm: &OneTimeKeyAlgorithm,
    ) -> Result<Option<(OwnedOneTimeKeyId, Raw<OneTimeKey>)>>;

    fn count_one_time_keys(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
    ) -> Result<BTreeMap<OneTimeKeyAlgorithm, UInt>>;

    fn add_device_keys(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
        device_keys: &Raw<DeviceKeys>,
    ) -> Result<()>;

    fn add_cross_signing_keys(
        &self,
        user_id: &UserId,
        master_key: &Raw<CrossSigningKey>,
        self_signing_key: &Option<Raw<CrossSigningKey>>,
        user_signing_key: &Option<Raw<CrossSigningKey>>,
        notify: bool,
    ) -> Result<()>;

    fn sign_key(
        &self,
        target_id: &UserId,
        key_id: &str,
        signature: (String, String),
        sender_id: &UserId,
    ) -> Result<()>;

    fn keys_changed<'a>(
        &'a self,
        user_or_room_id: &str,
        from: u64,
        to: Option<u64>,
    ) -> Box<dyn Iterator<Item = Result<OwnedUserId>> + 'a>;

    fn mark_device_key_update(&self, user_id: &UserId) -> Result<()>;

    fn get_device_keys(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
    ) -> Result<Option<Raw<DeviceKeys>>>;

    fn parse_master_key(
        &self,
        user_id: &UserId,
        master_key: &Raw<CrossSigningKey>,
    ) -> Result<(Vec<u8>, CrossSigningKey)>;

    fn get_key(
        &self,
        key: &[u8],
        sender_user: Option<&UserId>,
        user_id: &UserId,
        allowed_signatures: &dyn Fn(&UserId) -> bool,
    ) -> Result<Option<Raw<CrossSigningKey>>>;

    fn get_master_key(
        &self,
        sender_user: Option<&UserId>,
        user_id: &UserId,
        allowed_signatures: &dyn Fn(&UserId) -> bool,
    ) -> Result<Option<Raw<CrossSigningKey>>>;

    fn get_self_signing_key(
        &self,
        sender_user: Option<&UserId>,
        user_id: &UserId,
        allowed_signatures: &dyn Fn(&UserId) -> bool,
    ) -> Result<Option<Raw<CrossSigningKey>>>;

    fn get_user_signing_key(&self, user_id: &UserId) -> Result<Option<Raw<CrossSigningKey>>>;

    fn add_to_device_event(
        &self,
        sender: &UserId,
        target_user_id: &UserId,
        target_device_id: &DeviceId,
        event_type: &str,
        content: serde_json::Value,
    ) -> Result<()>;

    fn get_to_device_events(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
    ) -> Result<Vec<Raw<AnyToDeviceEvent>>>;

    fn remove_to_device_events(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
        until: u64,
    ) -> Result<()>;

    fn update_device_metadata(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
        device: &Device,
    ) -> Result<()>;

    /// Get device metadata.
    fn get_device_metadata(&self, user_id: &UserId, device_id: &DeviceId)
        -> Result<Option<Device>>;

    fn get_devicelist_version(&self, user_id: &UserId) -> Result<Option<u64>>;

    fn all_user_devices_metadata<'a>(
        &'a self,
        user_id: &UserId,
    ) -> Box<dyn Iterator<Item = Result<Device>> + 'a>;

    fn set_devices_last_seen<'a>(
        &'a self,
        devices: &'a BTreeMap<(OwnedUserId, OwnedDeviceId), MilliSecondsSinceUnixEpoch>,
    ) -> Box<dyn Iterator<Item = Result<()>> + 'a>;

    /// Creates a new sync filter. Returns the filter id.
    fn create_filter(&self, user_id: &UserId, filter: &FilterDefinition) -> Result<String>;

    fn get_filter(&self, user_id: &UserId, filter_id: &str) -> Result<Option<FilterDefinition>>;

    // Creates an OpenID token, which can be used to prove that a user has access to an account (primarily for integrations)
    fn create_openid_token(&self, user_id: &UserId) -> Result<(String, u64)>;

    /// Find out which user an OpenID access token belongs to.
    fn find_from_openid_token(&self, token: &str) -> Result<Option<OwnedUserId>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Error, Result};
    use ruma::{
        api::client::{device::Device, filter::FilterDefinition},
        device_id, encryption::{CrossSigningKey, DeviceKeys, OneTimeKey},
        events::AnyToDeviceEvent,
        mxc_uri, serde::Raw,
        user_id, DeviceId, MilliSecondsSinceUnixEpoch, OneTimeKeyAlgorithm,
        OwnedDeviceId, OwnedMxcUri, OwnedOneTimeKeyId, OwnedUserId, UInt, UserId,
    };
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::time::Instant;

    // Mock implementation for testing the Data trait
    struct MockDataImpl {
        users: std::sync::Mutex<BTreeMap<OwnedUserId, MockUser>>,
        devices: std::sync::Mutex<BTreeMap<(OwnedUserId, OwnedDeviceId), MockDevice>>,
        tokens: std::sync::Mutex<BTreeMap<String, (OwnedUserId, OwnedDeviceId)>>,
        openid_tokens: std::sync::Mutex<BTreeMap<String, OwnedUserId>>,
        filters: std::sync::Mutex<BTreeMap<String, FilterDefinition>>,
    }

    #[derive(Clone, Default)]
    struct MockUser {
        password_hash: Option<String>,
        displayname: Option<String>,
        avatar_url: Option<OwnedMxcUri>,
        blurhash: Option<String>,
        deactivated: bool,
        one_time_keys_update: u64,
        devicelist_version: Option<u64>,
    }

    #[derive(Clone, Default)]
    struct MockDevice {
        token: String,
        display_name: Option<String>,
        metadata: Option<Device>,
        device_keys: Option<Raw<DeviceKeys>>,
        one_time_keys: BTreeMap<OwnedOneTimeKeyId, Raw<OneTimeKey>>,
        to_device_events: Vec<Raw<AnyToDeviceEvent>>,
        last_seen: Option<MilliSecondsSinceUnixEpoch>,
    }

    impl MockDataImpl {
        fn new() -> Self {
            Self {
                users: std::sync::Mutex::new(BTreeMap::new()),
                devices: std::sync::Mutex::new(BTreeMap::new()),
                tokens: std::sync::Mutex::new(BTreeMap::new()),
                openid_tokens: std::sync::Mutex::new(BTreeMap::new()),
                filters: std::sync::Mutex::new(BTreeMap::new()),
            }
        }
    }

    impl Data for MockDataImpl {
        fn exists(&self, user_id: &UserId) -> Result<bool> {
            Ok(self.users.lock().unwrap().contains_key(user_id))
        }

        fn is_deactivated(&self, user_id: &UserId) -> Result<bool> {
            Ok(self
                .users
                .lock()
                .unwrap()
                .get(user_id)
                .map_or(false, |u| u.deactivated))
        }

        fn count(&self) -> Result<usize> {
            Ok(self.users.lock().unwrap().len())
        }

        fn find_from_token(&self, token: &str) -> Result<Option<(OwnedUserId, OwnedDeviceId)>> {
            Ok(self.tokens.lock().unwrap().get(token).cloned())
        }

        fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = Result<OwnedUserId>> + 'a> {
            let users: Vec<_> = self
                .users
                .lock()
                .unwrap()
                .keys()
                .cloned()
                .map(Ok)
                .collect();
            Box::new(users.into_iter())
        }

        fn list_local_users(&self) -> Result<Vec<String>> {
            let users = self.users.lock().unwrap();
            Ok(users
                .iter()
                .filter(|(_, data)| data.password_hash.is_some())
                .map(|(id, _)| id.to_string())
                .collect())
        }

        fn password_hash(&self, user_id: &UserId) -> Result<Option<String>> {
            Ok(self
                .users
                .lock()
                .unwrap()
                .get(user_id)
                .and_then(|u| u.password_hash.clone()))
        }

        fn set_password(&self, user_id: &UserId, password: Option<&str>) -> Result<()> {
            let mut users = self.users.lock().unwrap();
            let user_data = users.entry(user_id.to_owned()).or_default();
            user_data.password_hash = password.map(|p| format!("argon2_hash_{}", p));
            Ok(())
        }

        fn displayname(&self, user_id: &UserId) -> Result<Option<String>> {
            Ok(self
                .users
                .lock()
                .unwrap()
                .get(user_id)
                .and_then(|u| u.displayname.clone()))
        }

        fn set_displayname(&self, user_id: &UserId, displayname: Option<String>) -> Result<()> {
            let mut users = self.users.lock().unwrap();
            let user_data = users.entry(user_id.to_owned()).or_default();
            user_data.displayname = displayname;
            Ok(())
        }

        fn avatar_url(&self, user_id: &UserId) -> Result<Option<OwnedMxcUri>> {
            Ok(self
                .users
                .lock()
                .unwrap()
                .get(user_id)
                .and_then(|u| u.avatar_url.clone()))
        }

        fn set_avatar_url(&self, user_id: &UserId, avatar_url: Option<OwnedMxcUri>) -> Result<()> {
            let mut users = self.users.lock().unwrap();
            let user_data = users.entry(user_id.to_owned()).or_default();
            user_data.avatar_url = avatar_url;
            Ok(())
        }

        fn blurhash(&self, user_id: &UserId) -> Result<Option<String>> {
            Ok(self
                .users
                .lock()
                .unwrap()
                .get(user_id)
                .and_then(|u| u.blurhash.clone()))
        }

        fn set_blurhash(&self, user_id: &UserId, blurhash: Option<String>) -> Result<()> {
            let mut users = self.users.lock().unwrap();
            let user_data = users.entry(user_id.to_owned()).or_default();
            user_data.blurhash = blurhash;
            Ok(())
        }

        fn create_device(
            &self,
            user_id: &UserId,
            device_id: &DeviceId,
            token: &str,
            initial_device_display_name: Option<String>,
        ) -> Result<()> {
            let mut devices = self.devices.lock().unwrap();
            let mut tokens = self.tokens.lock().unwrap();
            
            devices.insert(
                (user_id.to_owned(), device_id.to_owned()),
                MockDevice {
                    token: token.to_string(),
                    display_name: initial_device_display_name,
                    ..Default::default()
                },
            );
            tokens.insert(token.to_string(), (user_id.to_owned(), device_id.to_owned()));
            Ok(())
        }

        fn remove_device(&self, user_id: &UserId, device_id: &DeviceId) -> Result<()> {
            let mut devices = self.devices.lock().unwrap();
            let mut tokens = self.tokens.lock().unwrap();
            
            if let Some(device) = devices.remove(&(user_id.to_owned(), device_id.to_owned())) {
                tokens.remove(&device.token);
            }
            Ok(())
        }

        fn all_device_ids<'a>(
            &'a self,
            user_id: &UserId,
        ) -> Box<dyn Iterator<Item = Result<OwnedDeviceId>> + 'a> {
            let user_id = user_id.to_owned();
            let device_ids: Vec<_> = self
                .devices
                .lock()
                .unwrap()
                .keys()
                .filter(|(uid, _)| *uid == user_id)
                .map(|(_, did)| Ok(did.clone()))
                .collect();
            Box::new(device_ids.into_iter())
        }

        fn set_token(&self, user_id: &UserId, device_id: &DeviceId, token: &str) -> Result<()> {
            let mut devices = self.devices.lock().unwrap();
            let mut tokens = self.tokens.lock().unwrap();
            
            if let Some(device) = devices.get_mut(&(user_id.to_owned(), device_id.to_owned())) {
                tokens.remove(&device.token);
                device.token = token.to_string();
                tokens.insert(token.to_string(), (user_id.to_owned(), device_id.to_owned()));
            }
            Ok(())
        }

        fn add_one_time_key(
            &self,
            user_id: &UserId,
            device_id: &DeviceId,
            one_time_key_key: &OwnedOneTimeKeyId,
            one_time_key_value: &Raw<OneTimeKey>,
        ) -> Result<()> {
            let mut devices = self.devices.lock().unwrap();
            if let Some(device) = devices.get_mut(&(user_id.to_owned(), device_id.to_owned())) {
                device.one_time_keys.insert(one_time_key_key.clone(), one_time_key_value.clone());
            }
            Ok(())
        }

        fn last_one_time_keys_update(&self, user_id: &UserId) -> Result<u64> {
            Ok(self
                .users
                .lock()
                .unwrap()
                .get(user_id)
                .map_or(0, |u| u.one_time_keys_update))
        }

        fn take_one_time_key(
            &self,
            user_id: &UserId,
            device_id: &DeviceId,
            key_algorithm: &OneTimeKeyAlgorithm,
        ) -> Result<Option<(OwnedOneTimeKeyId, Raw<OneTimeKey>)>> {
            let mut devices = self.devices.lock().unwrap();
            if let Some(device) = devices.get_mut(&(user_id.to_owned(), device_id.to_owned())) {
                // Find and remove a key with the specified algorithm
                for (key_id, _) in device.one_time_keys.iter() {
                    if key_id.algorithm() == *key_algorithm {
                        let key_id = key_id.clone();
                        let value = device.one_time_keys.remove(&key_id).unwrap();
                        return Ok(Some((key_id, value)));
                    }
                }
            }
            Ok(None)
        }

        fn count_one_time_keys(
            &self,
            user_id: &UserId,
            device_id: &DeviceId,
        ) -> Result<BTreeMap<OneTimeKeyAlgorithm, UInt>> {
            let devices = self.devices.lock().unwrap();
            let mut counts = BTreeMap::new();
            
            if let Some(device) = devices.get(&(user_id.to_owned(), device_id.to_owned())) {
                for key_id in device.one_time_keys.keys() {
                    *counts.entry(key_id.algorithm().clone()).or_insert(UInt::from(0u32)) += UInt::from(1u32);
                }
            }
            Ok(counts)
        }

        fn add_device_keys(
            &self,
            user_id: &UserId,
            device_id: &DeviceId,
            device_keys: &Raw<DeviceKeys>,
        ) -> Result<()> {
            let mut devices = self.devices.lock().unwrap();
            if let Some(device) = devices.get_mut(&(user_id.to_owned(), device_id.to_owned())) {
                device.device_keys = Some(device_keys.clone());
            }
            Ok(())
        }

        // Simplified implementations for complex crypto operations
        fn add_cross_signing_keys(
            &self,
            _user_id: &UserId,
            _master_key: &Raw<CrossSigningKey>,
            _self_signing_key: &Option<Raw<CrossSigningKey>>,
            _user_signing_key: &Option<Raw<CrossSigningKey>>,
            _notify: bool,
        ) -> Result<()> {
            Ok(())
        }

        fn sign_key(
            &self,
            _target_id: &UserId,
            _key_id: &str,
            _signature: (String, String),
            _sender_id: &UserId,
        ) -> Result<()> {
            Ok(())
        }

        fn keys_changed<'a>(
            &'a self,
            _user_or_room_id: &str,
            _from: u64,
            _to: Option<u64>,
        ) -> Box<dyn Iterator<Item = Result<OwnedUserId>> + 'a> {
            Box::new(std::iter::empty())
        }

        fn mark_device_key_update(&self, _user_id: &UserId) -> Result<()> {
            Ok(())
        }

        fn get_device_keys(
            &self,
            user_id: &UserId,
            device_id: &DeviceId,
        ) -> Result<Option<Raw<DeviceKeys>>> {
            let devices = self.devices.lock().unwrap();
            Ok(devices
                .get(&(user_id.to_owned(), device_id.to_owned()))
                .and_then(|d| d.device_keys.clone()))
        }

        fn parse_master_key(
            &self,
            _user_id: &UserId,
            _master_key: &Raw<CrossSigningKey>,
        ) -> Result<(Vec<u8>, CrossSigningKey)> {
            Err(Error::bad_database("Not implemented in mock"))
        }

        fn get_key(
            &self,
            _key: &[u8],
            _sender_user: Option<&UserId>,
            _user_id: &UserId,
            _allowed_signatures: &dyn Fn(&UserId) -> bool,
        ) -> Result<Option<Raw<CrossSigningKey>>> {
            Ok(None)
        }

        fn get_master_key(
            &self,
            _sender_user: Option<&UserId>,
            _user_id: &UserId,
            _allowed_signatures: &dyn Fn(&UserId) -> bool,
        ) -> Result<Option<Raw<CrossSigningKey>>> {
            Ok(None)
        }

        fn get_self_signing_key(
            &self,
            _sender_user: Option<&UserId>,
            _user_id: &UserId,
            _allowed_signatures: &dyn Fn(&UserId) -> bool,
        ) -> Result<Option<Raw<CrossSigningKey>>> {
            Ok(None)
        }

        fn get_user_signing_key(&self, _user_id: &UserId) -> Result<Option<Raw<CrossSigningKey>>> {
            Ok(None)
        }

        fn add_to_device_event(
            &self,
            _sender: &UserId,
            target_user_id: &UserId,
            target_device_id: &DeviceId,
            _event_type: &str,
            _content: serde_json::Value,
        ) -> Result<()> {
            let mut devices = self.devices.lock().unwrap();
            if let Some(device) = devices.get_mut(&(target_user_id.to_owned(), target_device_id.to_owned())) {
                // Add a mock to-device event
                let event_value = json!({
                    "type": "m.room.message",
                    "content": {}
                });
                let mock_event: Raw<AnyToDeviceEvent> = serde_json::from_value(event_value)
                    .map_err(|_| Error::bad_database("Failed to serialize event"))?;
                device.to_device_events.push(mock_event);
            }
            Ok(())
        }

        fn get_to_device_events(
            &self,
            user_id: &UserId,
            device_id: &DeviceId,
        ) -> Result<Vec<Raw<AnyToDeviceEvent>>> {
            let devices = self.devices.lock().unwrap();
            Ok(devices
                .get(&(user_id.to_owned(), device_id.to_owned()))
                .map_or(vec![], |d| d.to_device_events.clone()))
        }

        fn remove_to_device_events(
            &self,
            user_id: &UserId,
            device_id: &DeviceId,
            _until: u64,
        ) -> Result<()> {
            let mut devices = self.devices.lock().unwrap();
            if let Some(device) = devices.get_mut(&(user_id.to_owned(), device_id.to_owned())) {
                device.to_device_events.clear();
            }
            Ok(())
        }

        fn update_device_metadata(
            &self,
            user_id: &UserId,
            device_id: &DeviceId,
            device: &Device,
        ) -> Result<()> {
            let mut devices = self.devices.lock().unwrap();
            if let Some(mock_device) = devices.get_mut(&(user_id.to_owned(), device_id.to_owned())) {
                mock_device.metadata = Some(device.clone());
            }
            Ok(())
        }

        fn get_device_metadata(
            &self,
            user_id: &UserId,
            device_id: &DeviceId,
        ) -> Result<Option<Device>> {
            let devices = self.devices.lock().unwrap();
            Ok(devices
                .get(&(user_id.to_owned(), device_id.to_owned()))
                .and_then(|d| d.metadata.clone()))
        }

        fn get_devicelist_version(&self, user_id: &UserId) -> Result<Option<u64>> {
            Ok(self
                .users
                .lock()
                .unwrap()
                .get(user_id)
                .and_then(|u| u.devicelist_version))
        }

        fn all_user_devices_metadata<'a>(
            &'a self,
            user_id: &UserId,
        ) -> Box<dyn Iterator<Item = Result<Device>> + 'a> {
            let user_id = user_id.to_owned();
            let devices: Vec<_> = self
                .devices
                .lock()
                .unwrap()
                .iter()
                .filter(|((uid, _), _)| *uid == user_id)
                .filter_map(|(_, device)| device.metadata.clone())
                .map(Ok)
                .collect();
            Box::new(devices.into_iter())
        }

        fn set_devices_last_seen<'a>(
            &'a self,
            devices: &'a BTreeMap<(OwnedUserId, OwnedDeviceId), MilliSecondsSinceUnixEpoch>,
        ) -> Box<dyn Iterator<Item = Result<()>> + 'a> {
            let results: Vec<_> = devices.iter().map(|_| Ok(())).collect();
            Box::new(results.into_iter())
        }

        fn create_filter(&self, _user_id: &UserId, filter: &FilterDefinition) -> Result<String> {
            let filter_id = format!("filter_{}", rand::random::<u64>());
            self.filters.lock().unwrap().insert(filter_id.clone(), filter.clone());
            Ok(filter_id)
        }

        fn get_filter(
            &self,
            _user_id: &UserId,
            filter_id: &str,
        ) -> Result<Option<FilterDefinition>> {
            Ok(self.filters.lock().unwrap().get(filter_id).cloned())
        }

        fn create_openid_token(&self, user_id: &UserId) -> Result<(String, u64)> {
            let token = format!("openid_token_{}", rand::random::<u64>());
            let expires_at = chrono::Utc::now().timestamp() as u64 + 3600; // 1 hour
            self.openid_tokens.lock().unwrap().insert(token.clone(), user_id.to_owned());
            Ok((token, expires_at))
        }

        fn find_from_openid_token(&self, token: &str) -> Result<Option<OwnedUserId>> {
            Ok(self.openid_tokens.lock().unwrap().get(token).cloned())
        }
    }

    fn create_test_data() -> impl Data {
        MockDataImpl::new()
    }

    #[test]
    fn test_user_existence() {
        let data = create_test_data();
        let user_id = user_id!("@test:example.com");

        // User should not exist initially
        assert!(!data.exists(user_id).unwrap());

        // Create user by setting password
        data.set_password(user_id, Some("password123")).unwrap();

        // User should now exist
        assert!(data.exists(user_id).unwrap());
    }

    #[test]
    fn test_user_deactivation_status() {
        let data = create_test_data();
        let user_id = user_id!("@test:example.com");

        // User should not be deactivated initially
        assert!(!data.is_deactivated(user_id).unwrap());

        // Create user first
        data.set_password(user_id, Some("password")).unwrap();
        assert!(!data.is_deactivated(user_id).unwrap());
    }

    #[test]
    fn test_user_count() {
        let data = create_test_data();
        let user1 = user_id!("@user1:example.com");
        let user2 = user_id!("@user2:example.com");

        // Initially no users
        assert_eq!(data.count().unwrap(), 0);

        // Add users
        data.set_password(user1, Some("password1")).unwrap();
        assert_eq!(data.count().unwrap(), 1);

        data.set_password(user2, Some("password2")).unwrap();
        assert_eq!(data.count().unwrap(), 2);
    }

    #[test]
    fn test_token_management() {
        let data = create_test_data();
        let user_id = user_id!("@test:example.com");
        let device_id = device_id!("DEVICE123");
        let token = "access_token_123";

        // Token should not exist initially
        assert_eq!(data.find_from_token(token).unwrap(), None);

        // Create device with token
        data.create_device(user_id, device_id, token, Some("Test Device".to_string())).unwrap();

        // Token should now be found
        let found = data.find_from_token(token).unwrap();
        assert_eq!(found, Some((user_id.to_owned(), device_id.to_owned())));

        // Update token
        let new_token = "new_access_token_456";
        data.set_token(user_id, device_id, new_token).unwrap();

        // Old token should not be found, new token should be found
        assert_eq!(data.find_from_token(token).unwrap(), None);
        let found_new = data.find_from_token(new_token).unwrap();
        assert_eq!(found_new, Some((user_id.to_owned(), device_id.to_owned())));
    }

    #[test]
    fn test_local_users_list() {
        let data = create_test_data();
        let user1 = user_id!("@local1:example.com");
        let user2 = user_id!("@local2:example.com");
        let user3 = user_id!("@remote:other.com");

        // Initially no local users
        assert!(data.list_local_users().unwrap().is_empty());

        // Add local users (users with passwords)
        data.set_password(user1, Some("password1")).unwrap();
        data.set_password(user2, Some("password2")).unwrap();
        
        // Add remote user (no password)
        data.set_displayname(user3, Some("Remote User".to_string())).unwrap();

        let local_users = data.list_local_users().unwrap();
        assert_eq!(local_users.len(), 2);
        assert!(local_users.contains(&user1.to_string()));
        assert!(local_users.contains(&user2.to_string()));
        assert!(!local_users.contains(&user3.to_string()));
    }

    #[test]
    fn test_password_operations() {
        let data = create_test_data();
        let user_id = user_id!("@test:example.com");

        // Initially no password
        assert_eq!(data.password_hash(user_id).unwrap(), None);

        // Set password
        data.set_password(user_id, Some("secure_password")).unwrap();
        let hash = data.password_hash(user_id).unwrap();
        assert!(hash.is_some());
        assert!(hash.unwrap().starts_with("argon2_hash_"));

        // Remove password
        data.set_password(user_id, None).unwrap();
        assert_eq!(data.password_hash(user_id).unwrap(), None);
    }

    #[test]
    fn test_user_profile_management() {
        let data = create_test_data();
        let user_id = user_id!("@test:example.com");

        // Test displayname
        assert_eq!(data.displayname(user_id).unwrap(), None);
        data.set_displayname(user_id, Some("Test User".to_string())).unwrap();
        assert_eq!(data.displayname(user_id).unwrap(), Some("Test User".to_string()));

        // Test avatar URL
        assert_eq!(data.avatar_url(user_id).unwrap(), None);
        let avatar_url = mxc_uri!("mxc://example.com/avatar123");
        data.set_avatar_url(user_id, Some(avatar_url.to_owned())).unwrap();
        assert_eq!(data.avatar_url(user_id).unwrap(), Some(avatar_url.to_owned()));

        // Test blurhash
        assert_eq!(data.blurhash(user_id).unwrap(), None);
        data.set_blurhash(user_id, Some("LKO2?U%2Tw=w]~RBVZRi};RPxuwH".to_string())).unwrap();
        assert_eq!(data.blurhash(user_id).unwrap(), Some("LKO2?U%2Tw=w]~RBVZRi};RPxuwH".to_string()));
    }

    #[test]
    fn test_device_management() {
        let data = create_test_data();
        let user_id = user_id!("@test:example.com");
        let device_id = device_id!("DEVICE123");
        let token = "device_token";

        // Create device
        data.create_device(user_id, device_id, token, Some("My Device".to_string())).unwrap();

        // Verify device exists through token lookup
        let found = data.find_from_token(token).unwrap();
        assert_eq!(found, Some((user_id.to_owned(), device_id.to_owned())));

        // Test device iteration
        let device_ids: Vec<_> = data.all_device_ids(user_id).collect();
        assert_eq!(device_ids.len(), 1);
        assert_eq!(device_ids[0].as_ref().unwrap(), device_id);

        // Remove device
        data.remove_device(user_id, device_id).unwrap();

        // Device should no longer be found
        assert_eq!(data.find_from_token(token).unwrap(), None);
        let device_ids_after: Vec<_> = data.all_device_ids(user_id).collect();
        assert!(device_ids_after.is_empty());
    }

    #[test]
    fn test_one_time_keys() {
        let data = create_test_data();
        let user_id = user_id!("@test:example.com");
        let device_id = device_id!("DEVICE123");
        let token = "device_token";

        // Create device first
        data.create_device(user_id, device_id, token, None).unwrap();

        // Initially no one-time keys
        let counts = data.count_one_time_keys(user_id, device_id).unwrap();
        assert!(counts.is_empty());

        // Skip the Raw<OneTimeKey> creation for now - this is a complex type
        // TODO: Implement proper one-time key testing with correct Raw types
    }

    #[test]
    fn test_device_keys() {
        let data = create_test_data();
        let user_id = user_id!("@test:example.com");
        let device_id = device_id!("DEVICE123");
        let token = "device_token";

        // Create device first
        data.create_device(user_id, device_id, token, None).unwrap();

        // Initially no device keys
        let keys = data.get_device_keys(user_id, device_id).unwrap();
        assert!(keys.is_none());

        // Skip device keys testing for now due to Raw type complexity
        // TODO: Implement proper device key testing
    }

    #[test]
    fn test_to_device_events() {
        let data = create_test_data();
        let sender = user_id!("@sender:example.com");
        let target_user = user_id!("@target:example.com");
        let target_device = device_id!("DEVICE123");
        let token = "device_token";

        // Create target device
        data.create_device(target_user, target_device, token, None).unwrap();

        // Initially no to-device events
        let events = data.get_to_device_events(target_user, target_device).unwrap();
        assert!(events.is_empty());

        // Add to-device event
        let content = json!({
            "body": "Hello, world!",
            "msgtype": "m.text"
        });
        data.add_to_device_event(sender, target_user, target_device, "m.room.message", content).unwrap();

        // Verify event was added
        let events = data.get_to_device_events(target_user, target_device).unwrap();
        assert_eq!(events.len(), 1);

        // Remove events
        data.remove_to_device_events(target_user, target_device, 0).unwrap();

        // Events should be removed
        let events_after = data.get_to_device_events(target_user, target_device).unwrap();
        assert!(events_after.is_empty());
    }

    #[test]
    fn test_device_metadata() {
        let data = create_test_data();
        let user_id = user_id!("@test:example.com");
        let device_id = device_id!("DEVICE123");
        let token = "device_token";

        // Create device
        data.create_device(user_id, device_id, token, Some("Initial Name".to_string())).unwrap();

        // Initially no metadata
        let metadata = data.get_device_metadata(user_id, device_id).unwrap();
        assert!(metadata.is_none());

        // Update device metadata
        let device_metadata = Device::new(device_id.to_owned());
        data.update_device_metadata(user_id, device_id, &device_metadata).unwrap();

        // Verify metadata was updated
        let retrieved = data.get_device_metadata(user_id, device_id).unwrap();
        assert!(retrieved.is_some());
        // Note: Can't compare Device directly as it doesn't implement PartialEq
    }

    #[test]
    fn test_filter_operations() {
        let data = create_test_data();
        let user_id = user_id!("@test:example.com");
        let filter = FilterDefinition::default();

        // Create filter
        let filter_id = data.create_filter(user_id, &filter).unwrap();
        assert!(!filter_id.is_empty());

        // Retrieve filter
        let retrieved = data.get_filter(user_id, &filter_id).unwrap();
        assert!(retrieved.is_some());

        // Non-existent filter should return None
        let non_existent = data.get_filter(user_id, "invalid_filter_id").unwrap();
        assert!(non_existent.is_none());
    }

    #[test]
    fn test_openid_tokens() {
        let data = create_test_data();
        let user_id = user_id!("@test:example.com");

        // Create OpenID token
        let (token, expires_at) = data.create_openid_token(user_id).unwrap();
        assert!(!token.is_empty());
        assert!(expires_at > 0);

        // Find user by token
        let found_user = data.find_from_openid_token(&token).unwrap();
        assert_eq!(found_user, Some(user_id.to_owned()));

        // Invalid token should return None
        let invalid = data.find_from_openid_token("invalid_token").unwrap();
        assert_eq!(invalid, None);
    }

    #[test]
    fn test_user_iteration() {
        let data = create_test_data();
        let user1 = user_id!("@user1:example.com");
        let user2 = user_id!("@user2:example.com");

        // Add users
        data.set_password(user1, Some("password1")).unwrap();
        data.set_password(user2, Some("password2")).unwrap();

        // Iterate through users
        let users: Vec<_> = data.iter().collect();
        assert_eq!(users.len(), 2);
        
        let user_ids: Vec<_> = users.into_iter().map(|r| r.unwrap()).collect();
        assert!(user_ids.contains(&user1.to_owned()));
        assert!(user_ids.contains(&user2.to_owned()));
    }

    #[test]
    fn test_performance_operations() {
        let data = create_test_data();
        let user_id = user_id!("@test:example.com");
        
        let start = Instant::now();
        
        // Perform multiple profile updates
        for i in 0..1000 {
            let displayname = format!("User {}", i);
            data.set_displayname(user_id, Some(displayname)).unwrap();
        }
        
        let duration = start.elapsed();
        
        // Should complete reasonably quickly (less than 1 second)
        assert!(duration.as_secs() < 1);
        
        // Verify final state
        assert_eq!(data.displayname(user_id).unwrap(), Some("User 999".to_string()));
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;
        
        let data = Arc::new(create_test_data());
        let user_id = user_id!("@test:example.com");
        
        let mut handles = vec![];
        
        // Spawn multiple threads doing concurrent operations
        for i in 0..10 {
            let data_clone = Arc::clone(&data);
            let user_id_clone = user_id.to_owned();
            
            let handle = thread::spawn(move || {
                let displayname = format!("Concurrent User {}", i);
                data_clone.set_displayname(&user_id_clone, Some(displayname)).unwrap();
            });
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Verify final state exists
        assert!(data.displayname(user_id).unwrap().is_some());
    }

    #[test]
    fn test_edge_cases() {
        let data = create_test_data();
        let user_id = user_id!("@test:example.com");
        
        // Test empty displayname
        data.set_displayname(user_id, Some("".to_string())).unwrap();
        assert_eq!(data.displayname(user_id).unwrap(), Some("".to_string()));
        
        // Test very long displayname
        let long_name = "a".repeat(10000);
        data.set_displayname(user_id, Some(long_name.clone())).unwrap();
        assert_eq!(data.displayname(user_id).unwrap(), Some(long_name));
        
        // Test None values
        data.set_displayname(user_id, None).unwrap();
        assert_eq!(data.displayname(user_id).unwrap(), None);
        
        data.set_avatar_url(user_id, None).unwrap();
        assert_eq!(data.avatar_url(user_id).unwrap(), None);
        
        data.set_blurhash(user_id, None).unwrap();
        assert_eq!(data.blurhash(user_id).unwrap(), None);
    }

    #[test]
    fn test_error_conditions() {
        let data = create_test_data();
        let nonexistent_user = user_id!("@nonexistent:example.com");
        
        // Operations on non-existent user should return sensible defaults
        assert!(!data.exists(nonexistent_user).unwrap());
        assert!(!data.is_deactivated(nonexistent_user).unwrap());
        assert_eq!(data.displayname(nonexistent_user).unwrap(), None);
        assert_eq!(data.avatar_url(nonexistent_user).unwrap(), None);
        assert_eq!(data.blurhash(nonexistent_user).unwrap(), None);
        assert_eq!(data.password_hash(nonexistent_user).unwrap(), None);
    }
}
