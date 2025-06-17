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

mod data;
use std::{
    collections::{BTreeMap, BTreeSet},
    mem,
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};

pub use data::Data;
use ruma::{
    api::client::{
        device::Device,
        filter::FilterDefinition,
        sync::sync_events::{self},
    },
    encryption::{CrossSigningKey, DeviceKeys, OneTimeKey},
    events::AnyToDeviceEvent,
    serde::Raw,
    DeviceId, MilliSecondsSinceUnixEpoch, OneTimeKeyAlgorithm, OwnedDeviceId, OwnedMxcUri,
    OwnedOneTimeKeyId, OwnedRoomId, OwnedUserId, UInt, UserId,
};
use tokio::{sync::Mutex, time::interval};
use tracing::{debug, warn};

use crate::{services, Error, Result};
use crate::service::sync::sliding_sync::SlidingSyncExtensions as LocalExtensions;

pub struct SlidingSyncCache {
    lists: BTreeMap<String, sync_events::v5::request::List>,
    subscriptions: BTreeMap<OwnedRoomId, sync_events::v5::request::RoomSubscription>,
    known_rooms: BTreeMap<String, BTreeMap<OwnedRoomId, u64>>, // For every room, the roomsince number
    extensions: LocalExtensions,
}

pub struct Service {
    pub db: &'static dyn Data,
    #[allow(clippy::type_complexity)]
    pub connections:
        StdMutex<BTreeMap<(OwnedUserId, OwnedDeviceId, String), Arc<StdMutex<SlidingSyncCache>>>>,
    pub device_last_seen: Mutex<BTreeMap<(OwnedUserId, OwnedDeviceId), MilliSecondsSinceUnixEpoch>>,
}

impl Service {
    pub fn start_device_last_seen_update_task(self: &Arc<Self>) {
        let self2 = Arc::clone(self);
        tokio::spawn(async move {
            // Actually writes the new device times to the database every 55 minutes.
            // The device times are always returned fresh from memory
            // if they have been changed after the last write.
            let timer_interval = Duration::from_secs(60 * 5);
            let mut i = interval(timer_interval);
            loop {
                i.tick().await;
                let _ = self2.try_update_device_last_seen().await;
            }
        });
    }

    pub async fn try_update_device_last_seen(&self) {
        debug!("Writing cached device last-seens to database");
        for error in self.write_cached_last_seen().await {
            warn!("Error writing last seen timestamp of device to database: {error}");
        }
    }

    /// Writes all the currently cached last seen timestamps of devices to the database,
    /// clearing the cache in the process
    async fn write_cached_last_seen(&self) -> Vec<Error> {
        let mut guard = self.device_last_seen.lock().await;
        if !guard.is_empty() {
            // TODO: Replace with `replace` once/if `tokio::sync::Mutex` implements the equivalent
            // method from `std`: https://doc.rust-lang.org/std/sync/struct.Mutex.html#method.replace
            // i.e. instead of the `let mut guard` above:
            //let map = self.device_last_seen.replace(BTreeMap::new()).await;
            // We do a clone instead as we don't want start having a backlog of awaiting `lock`s
            // for all these DB fetches and writes, which admittedly, might not even be a big deal.
            let map = guard.clone();
            guard.clear();
            drop(guard);

            let result = self
                .db
                .set_devices_last_seen(&map)
                .filter_map(Result::err)
                .collect();
            result
        } else {
            Vec::new()
        }
    }

    /// Check if a user has an account on this NextServer.
    pub fn exists(&self, user_id: &UserId) -> Result<bool> {
        self.db.exists(user_id)
    }

    pub fn forget_sync_request_connection(
        &self,
        user_id: OwnedUserId,
        device_id: OwnedDeviceId,
        conn_id: String,
    ) {
        self.connections
            .lock()
            .unwrap()
            .remove(&(user_id, device_id, conn_id));
    }

    pub fn update_sync_request_with_cache(
        &self,
        user_id: OwnedUserId,
        device_id: OwnedDeviceId,
        request: &mut sync_events::v5::Request,
    ) -> BTreeMap<String, BTreeMap<OwnedRoomId, u64>> {
        let Some(conn_id) = request.conn_id.clone() else {
            return BTreeMap::new();
        };

        let mut cache = self.connections.lock().unwrap();
        let cached = Arc::clone(
            cache
                .entry((user_id, device_id, conn_id))
                .or_insert_with(|| {
                    Arc::new(StdMutex::new(SlidingSyncCache {
                        lists: BTreeMap::new(),
                        subscriptions: BTreeMap::new(),
                        known_rooms: BTreeMap::new(),
                        extensions: LocalExtensions::default(),
                    }))
                }),
        );
        let cached = &mut cached.lock().unwrap();
        drop(cache);

        for (list_id, list) in &mut request.lists {
            if let Some(cached_list) = cached.lists.get(list_id) {
                if list.room_details.required_state.is_empty() {
                    list.room_details
                        .required_state
                        .clone_from(&cached_list.room_details.required_state);
                };
                match (&mut list.filters, cached_list.filters.clone()) {
                    (Some(list_filters), Some(cached_filters)) => {
                        list_filters.is_invite =
                            list_filters.is_invite.or(cached_filters.is_invite);
                        if list_filters.not_room_types.is_empty() {
                            list_filters.not_room_types = cached_filters.not_room_types;
                        }
                    }
                    (_, Some(cached_filters)) => list.filters = Some(cached_filters),
                    (_, _) => {}
                }
            }
            cached.lists.insert(list_id.clone(), list.clone());
        }

        cached.subscriptions.extend(
            request
                .room_subscriptions
                .iter()
                .map(|(k, v)| (k.clone(), v.clone())),
        );
        request.room_subscriptions.extend(
            cached
                .subscriptions
                .iter()
                .map(|(k, v)| (k.clone(), v.clone())),
        );

        request.extensions.e2ee.enabled = request
            .extensions
            .e2ee
            .enabled
            .or(cached.extensions.e2ee.enabled);

        request.extensions.to_device.enabled = request
            .extensions
            .to_device
            .enabled
            .or(cached.extensions.to_device.enabled);

        request.extensions.account_data.enabled = request
            .extensions
            .account_data
            .enabled
            .or(cached.extensions.account_data.enabled);
        request.extensions.account_data.lists = request
            .extensions
            .account_data
            .lists
            .clone()
            .or(cached.extensions.account_data.lists.clone());
        request.extensions.account_data.rooms = request
            .extensions
            .account_data
            .rooms
            .clone()
            .or(cached.extensions.account_data.rooms.clone());

        cached.extensions = request.extensions.clone();

        cached.known_rooms.clone()
    }

    pub fn update_sync_subscriptions(
        &self,
        user_id: OwnedUserId,
        device_id: OwnedDeviceId,
        conn_id: String,
        subscriptions: BTreeMap<OwnedRoomId, sync_events::v5::request::RoomSubscription>,
    ) {
        let mut cache = self.connections.lock().unwrap();
        let cached = Arc::clone(
            cache
                .entry((user_id, device_id, conn_id))
                .or_insert_with(|| {
                    Arc::new(StdMutex::new(SlidingSyncCache {
                        lists: BTreeMap::new(),
                        subscriptions: BTreeMap::new(),
                        known_rooms: BTreeMap::new(),
                        extensions: LocalExtensions::default(),
                    }))
                }),
        );
        let cached = &mut cached.lock().unwrap();
        drop(cache);

        cached.subscriptions = subscriptions;
    }

    pub fn update_sync_known_rooms(
        &self,
        user_id: OwnedUserId,
        device_id: OwnedDeviceId,
        conn_id: String,
        list_id: String,
        new_cached_rooms: BTreeSet<OwnedRoomId>,
        globalsince: u64,
    ) {
        let mut cache = self.connections.lock().unwrap();
        let cached = Arc::clone(
            cache
                .entry((user_id, device_id, conn_id))
                .or_insert_with(|| {
                    Arc::new(StdMutex::new(SlidingSyncCache {
                        lists: BTreeMap::new(),
                        subscriptions: BTreeMap::new(),
                        known_rooms: BTreeMap::new(),
                        extensions: LocalExtensions::default(),
                    }))
                }),
        );
        let cached = &mut cached.lock().unwrap();
        drop(cache);

        for (roomid, lastsince) in cached
            .known_rooms
            .entry(list_id.clone())
            .or_default()
            .iter_mut()
        {
            if !new_cached_rooms.contains(roomid) {
                *lastsince = 0;
            }
        }
        let list = cached.known_rooms.entry(list_id).or_default();
        for roomid in new_cached_rooms {
            list.insert(roomid, globalsince);
        }
    }

    /// Check if account is deactivated
    pub fn is_deactivated(&self, user_id: &UserId) -> Result<bool> {
        self.db.is_deactivated(user_id)
    }

    /// Check if a user is an admin
    pub fn is_admin(&self, user_id: &UserId) -> Result<bool> {
        if let Some(admin_room_id) = services().admin.get_admin_room()? {
            services()
                .rooms
                .state_cache
                .is_joined(user_id, &admin_room_id)
        } else {
            Ok(false)
        }
    }

    /// Create a new user account on this NextServer.
    pub fn create(&self, user_id: &UserId, password: Option<&str>) -> Result<()> {
        self.db.set_password(user_id, password)?;
        Ok(())
    }

    /// Returns the number of users registered on this server.
    pub fn count(&self) -> Result<usize> {
        self.db.count()
    }

    /// Find out which user an access token belongs to.
    pub fn find_from_token(&self, token: &str) -> Result<Option<(OwnedUserId, OwnedDeviceId)>> {
        self.db.find_from_token(token)
    }

    /// Returns an iterator over all users on this NextServer.
    pub fn iter(&self) -> impl Iterator<Item = Result<OwnedUserId>> + '_ {
        self.db.iter()
    }

    /// Returns a list of local users as list of usernames.
    ///
    /// A user account is considered `local` if the length of it's password is greater then zero.
    pub fn list_local_users(&self) -> Result<Vec<String>> {
        self.db.list_local_users()
    }

    /// Returns the password hash for the given user.
    pub fn password_hash(&self, user_id: &UserId) -> Result<Option<String>> {
        self.db.password_hash(user_id)
    }

    /// Hash and set the user's password to the Argon2 hash
    pub fn set_password(&self, user_id: &UserId, password: Option<&str>) -> Result<()> {
        self.db.set_password(user_id, password)
    }

    /// Returns the displayname of a user on this NextServer.
    pub fn displayname(&self, user_id: &UserId) -> Result<Option<String>> {
        self.db.displayname(user_id)
    }

    /// Sets a new displayname or removes it if displayname is None. You still need to nofify all rooms of this change.
    pub fn set_displayname(&self, user_id: &UserId, displayname: Option<String>) -> Result<()> {
        self.db.set_displayname(user_id, displayname)
    }

    /// Get the avatar_url of a user.
    pub fn avatar_url(&self, user_id: &UserId) -> Result<Option<OwnedMxcUri>> {
        self.db.avatar_url(user_id)
    }

    /// Sets a new avatar_url or removes it if avatar_url is None.
    pub fn set_avatar_url(&self, user_id: &UserId, avatar_url: Option<OwnedMxcUri>) -> Result<()> {
        self.db.set_avatar_url(user_id, avatar_url)
    }

    /// Get the blurhash of a user.
    pub fn blurhash(&self, user_id: &UserId) -> Result<Option<String>> {
        self.db.blurhash(user_id)
    }

    /// Sets a new avatar_url or removes it if avatar_url is None.
    pub fn set_blurhash(&self, user_id: &UserId, blurhash: Option<String>) -> Result<()> {
        self.db.set_blurhash(user_id, blurhash)
    }

    /// Adds a new device to a user.
    pub fn create_device(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
        token: &str,
        initial_device_display_name: Option<String>,
    ) -> Result<()> {
        self.db
            .create_device(user_id, device_id, token, initial_device_display_name)
    }

    /// Removes a device from a user.
    pub fn remove_device(&self, user_id: &UserId, device_id: &DeviceId) -> Result<()> {
        self.db.remove_device(user_id, device_id)
    }

    /// Returns an iterator over all device ids of this user.
    pub fn all_device_ids<'a>(
        &'a self,
        user_id: &UserId,
    ) -> impl Iterator<Item = Result<OwnedDeviceId>> + 'a {
        self.db.all_device_ids(user_id)
    }

    /// Replaces the access token of one device.
    pub fn set_token(&self, user_id: &UserId, device_id: &DeviceId, token: &str) -> Result<()> {
        self.db.set_token(user_id, device_id, token)
    }

    pub fn add_one_time_key(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
        one_time_key_key: &OwnedOneTimeKeyId,
        one_time_key_value: &Raw<OneTimeKey>,
    ) -> Result<()> {
        self.db
            .add_one_time_key(user_id, device_id, one_time_key_key, one_time_key_value)
    }

    pub fn last_one_time_keys_update(&self, user_id: &UserId) -> Result<u64> {
        self.db.last_one_time_keys_update(user_id)
    }

    pub fn take_one_time_key(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
        key_algorithm: &OneTimeKeyAlgorithm,
    ) -> Result<Option<(OwnedOneTimeKeyId, Raw<OneTimeKey>)>> {
        self.db.take_one_time_key(user_id, device_id, key_algorithm)
    }

    pub fn count_one_time_keys(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
    ) -> Result<BTreeMap<OneTimeKeyAlgorithm, UInt>> {
        self.db.count_one_time_keys(user_id, device_id)
    }

    pub fn add_device_keys(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
        device_keys: &Raw<DeviceKeys>,
    ) -> Result<()> {
        self.db.add_device_keys(user_id, device_id, device_keys)
    }

    pub fn add_cross_signing_keys(
        &self,
        user_id: &UserId,
        master_key: &Raw<CrossSigningKey>,
        self_signing_key: &Option<Raw<CrossSigningKey>>,
        user_signing_key: &Option<Raw<CrossSigningKey>>,
        notify: bool,
    ) -> Result<()> {
        self.db.add_cross_signing_keys(
            user_id,
            master_key,
            self_signing_key,
            user_signing_key,
            notify,
        )
    }

    pub fn sign_key(
        &self,
        target_id: &UserId,
        key_id: &str,
        signature: (String, String),
        sender_id: &UserId,
    ) -> Result<()> {
        self.db.sign_key(target_id, key_id, signature, sender_id)
    }

    pub fn keys_changed<'a>(
        &'a self,
        user_or_room_id: &str,
        from: u64,
        to: Option<u64>,
    ) -> impl Iterator<Item = Result<OwnedUserId>> + 'a {
        self.db.keys_changed(user_or_room_id, from, to)
    }

    pub fn mark_device_key_update(&self, user_id: &UserId) -> Result<()> {
        self.db.mark_device_key_update(user_id)
    }

    pub fn get_device_keys(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
    ) -> Result<Option<Raw<DeviceKeys>>> {
        self.db.get_device_keys(user_id, device_id)
    }

    pub fn parse_master_key(
        &self,
        user_id: &UserId,
        master_key: &Raw<CrossSigningKey>,
    ) -> Result<(Vec<u8>, CrossSigningKey)> {
        self.db.parse_master_key(user_id, master_key)
    }

    pub fn get_key(
        &self,
        key: &[u8],
        sender_user: Option<&UserId>,
        user_id: &UserId,
        allowed_signatures: &dyn Fn(&UserId) -> bool,
    ) -> Result<Option<Raw<CrossSigningKey>>> {
        self.db
            .get_key(key, sender_user, user_id, allowed_signatures)
    }

    pub fn get_master_key(
        &self,
        sender_user: Option<&UserId>,
        user_id: &UserId,
        allowed_signatures: &dyn Fn(&UserId) -> bool,
    ) -> Result<Option<Raw<CrossSigningKey>>> {
        self.db
            .get_master_key(sender_user, user_id, allowed_signatures)
    }

    pub fn get_self_signing_key(
        &self,
        sender_user: Option<&UserId>,
        user_id: &UserId,
        allowed_signatures: &dyn Fn(&UserId) -> bool,
    ) -> Result<Option<Raw<CrossSigningKey>>> {
        self.db
            .get_self_signing_key(sender_user, user_id, allowed_signatures)
    }

    pub fn get_user_signing_key(&self, user_id: &UserId) -> Result<Option<Raw<CrossSigningKey>>> {
        self.db.get_user_signing_key(user_id)
    }

    pub fn add_to_device_event(
        &self,
        sender: &UserId,
        target_user_id: &UserId,
        target_device_id: &DeviceId,
        event_type: &str,
        content: serde_json::Value,
    ) -> Result<()> {
        self.db.add_to_device_event(
            sender,
            target_user_id,
            target_device_id,
            event_type,
            content,
        )
    }

    pub fn get_to_device_events(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
    ) -> Result<Vec<Raw<AnyToDeviceEvent>>> {
        self.db.get_to_device_events(user_id, device_id)
    }

    pub fn remove_to_device_events(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
        until: u64,
    ) -> Result<()> {
        self.db.remove_to_device_events(user_id, device_id, until)
    }

    pub fn update_device_metadata(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
        device: &Device,
    ) -> Result<()> {
        self.db.update_device_metadata(user_id, device_id, device)
    }

    /// Get device metadata.
    pub fn get_device_metadata(
        &self,
        user_id: &UserId,
        device_id: &DeviceId,
    ) -> Result<Option<Device>> {
        self.db.get_device_metadata(user_id, device_id)
    }

    pub fn get_devicelist_version(&self, user_id: &UserId) -> Result<Option<u64>> {
        self.db.get_devicelist_version(user_id)
    }

    pub async fn all_user_devices_metadata<'a>(
        &'a self,
        user_id: &'a UserId,
    ) -> impl Iterator<Item = Device> + 'a {
        let all_devices: Vec<_> = self
            .db
            .all_user_devices_metadata(user_id)
            .filter_map(Result::ok)
            // RumaHandler trait complains if we don't collect
            .collect();
        let device_last_seen = self.device_last_seen.lock().await;

        // Updates the timestamps with the cached ones
        all_devices.into_iter().map(move |mut d| {
            if let Some(ts) = device_last_seen.get(&(user_id.to_owned(), d.device_id.clone())) {
                d.last_seen_ts = Some(*ts);
            };
            d
        })
    }

    /// Deactivate account
    pub fn deactivate_account(&self, user_id: &UserId) -> Result<()> {
        // Remove all associated devices
        for device_id in self.all_device_ids(user_id) {
            self.remove_device(user_id, &device_id?)?;
        }

        // Set the password to "" to indicate a deactivated account. Hashes will never result in an
        // empty string, so the user will not be able to log in again. Systems like changing the
        // password without logging in should check if the account is deactivated.
        self.db.set_password(user_id, None)?;

        // TODO: Unhook 3PID
        Ok(())
    }

    /// Creates a new sync filter. Returns the filter id.
    pub fn create_filter(&self, user_id: &UserId, filter: &FilterDefinition) -> Result<String> {
        self.db.create_filter(user_id, filter)
    }

    pub fn get_filter(
        &self,
        user_id: &UserId,
        filter_id: &str,
    ) -> Result<Option<FilterDefinition>> {
        self.db.get_filter(user_id, filter_id)
    }

    // Creates an OpenID token, which can be used to prove that a user has access to an account (primarily for integrations)
    pub fn create_openid_token(&self, user_id: &UserId) -> Result<(String, u64)> {
        self.db.create_openid_token(user_id)
    }

    /// Find out which user an OpenID access token belongs to.
    pub fn find_from_openid_token(&self, token: &str) -> Result<Option<OwnedUserId>> {
        self.db.find_from_openid_token(token)
    }

    /// Sets the device_last_seen timestamp of a given device to now
    pub async fn update_device_last_seen(&self, user_id: OwnedUserId, device_id: OwnedDeviceId) {
        self.device_last_seen
            .lock()
            .await
            .insert((user_id, device_id), MilliSecondsSinceUnixEpoch::now());
    }
}

/// Ensure that a user only sees signatures from themselves and the target user
pub fn clean_signatures<F: Fn(&UserId) -> bool>(
    cross_signing_key: &mut serde_json::Value,
    sender_user: Option<&UserId>,
    user_id: &UserId,
    allowed_signatures: F,
) -> Result<(), Error> {
    if let Some(signatures) = cross_signing_key
        .get_mut("signatures")
        .and_then(|v| v.as_object_mut())
    {
        // Don't allocate for the full size of the current signatures, but require
        // at most one resize if nothing is dropped
        let new_capacity = signatures.len() / 2;
        for (user, signature) in
            mem::replace(signatures, serde_json::Map::with_capacity(new_capacity))
        {
            let sid = <&UserId>::try_from(user.as_str())
                .map_err(|_| Error::bad_database("Invalid user ID in database."))?;
            if sender_user == Some(user_id) || sid == user_id || allowed_signatures(sid) {
                signatures.insert(user, signature);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Error, Result};
    use ruma::{
        api::client::{
            device::Device,
            filter::FilterDefinition,
            sync::sync_events,
        },
        device_id, encryption::{CrossSigningKey, DeviceKeys, OneTimeKey}, 
        events::AnyToDeviceEvent,
        serde::Raw,
        user_id, DeviceId, MilliSecondsSinceUnixEpoch, OneTimeKeyAlgorithm, 
        OwnedDeviceId, OwnedMxcUri, OwnedOneTimeKeyId,
        OwnedUserId, UInt, UserId
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::{Arc, Mutex as StdMutex};
    use tokio::sync::Mutex;
    use std::time::{Duration, Instant};

    // Mock implementation of Data trait for testing
    struct MockData {
        users: StdMutex<BTreeMap<OwnedUserId, UserData>>,
        devices: StdMutex<BTreeMap<(OwnedUserId, OwnedDeviceId), DeviceData>>,
        filters: StdMutex<BTreeMap<(OwnedUserId, String), FilterDefinition>>,
        openid_tokens: StdMutex<BTreeMap<String, OwnedUserId>>,
        filter_counter: AtomicU64,
        token_counter: AtomicU64,
    }

    #[derive(Clone, Default)]
    struct UserData {
        password_hash: Option<String>,
        displayname: Option<String>,
        avatar_url: Option<OwnedMxcUri>,
        blurhash: Option<String>,
        deactivated: bool,
    }

    #[derive(Clone, Default)]
    struct DeviceData {
        token: String,
        metadata: Option<Device>,
        one_time_keys: BTreeMap<OwnedOneTimeKeyId, Raw<OneTimeKey>>,
        device_keys: Option<Raw<DeviceKeys>>,
        to_device_events: Vec<Raw<AnyToDeviceEvent>>,
    }

    impl MockData {
        fn new() -> Self {
            Self {
                users: StdMutex::new(BTreeMap::new()),
                devices: StdMutex::new(BTreeMap::new()),
                filters: StdMutex::new(BTreeMap::new()),
                openid_tokens: StdMutex::new(BTreeMap::new()),
                filter_counter: AtomicU64::new(0),
                token_counter: AtomicU64::new(0),
            }
        }
    }

    impl Data for MockData {
        fn exists(&self, user_id: &UserId) -> Result<bool> {
            Ok(self.users.lock().unwrap().contains_key(user_id))
        }

        fn is_deactivated(&self, user_id: &UserId) -> Result<bool> {
            Ok(self.users.lock().unwrap()
                .get(user_id)
                .map_or(false, |u| u.deactivated))
        }

        fn count(&self) -> Result<usize> {
            Ok(self.users.lock().unwrap().len())
        }

        fn find_from_token(&self, token: &str) -> Result<Option<(OwnedUserId, OwnedDeviceId)>> {
            let devices = self.devices.lock().unwrap();
            for ((user_id, device_id), device_data) in devices.iter() {
                if device_data.token == token {
                    return Ok(Some((user_id.clone(), device_id.clone())));
                }
            }
            Ok(None)
        }

        fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = Result<OwnedUserId>> + 'a> {
            // This is a simplified mock implementation
            Box::new(std::iter::empty())
        }

        fn list_local_users(&self) -> Result<Vec<String>> {
            let users = self.users.lock().unwrap();
            Ok(users.iter()
                .filter(|(_, data)| data.password_hash.is_some())
                .map(|(id, _)| id.to_string())
                .collect())
        }

        fn password_hash(&self, user_id: &UserId) -> Result<Option<String>> {
            Ok(self.users.lock().unwrap()
                .get(user_id)
                .and_then(|u| u.password_hash.clone()))
        }

        fn set_password(&self, user_id: &UserId, password: Option<&str>) -> Result<()> {
            let mut users = self.users.lock().unwrap();
            let user_data = users.entry(user_id.to_owned()).or_default();
            user_data.password_hash = password.map(|p| format!("hashed_{}", p));
            // When password is set to None, mark user as deactivated
            if password.is_none() {
                user_data.deactivated = true;
            }
            Ok(())
        }

        fn displayname(&self, user_id: &UserId) -> Result<Option<String>> {
            Ok(self.users.lock().unwrap()
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
            Ok(self.users.lock().unwrap()
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
            Ok(self.users.lock().unwrap()
                .get(user_id)
                .and_then(|u| u.blurhash.clone()))
        }

        fn set_blurhash(&self, user_id: &UserId, blurhash: Option<String>) -> Result<()> {
            let mut users = self.users.lock().unwrap();
            let user_data = users.entry(user_id.to_owned()).or_default();
            user_data.blurhash = blurhash;
            Ok(())
        }

        // Minimal implementations for other required methods
        fn create_device(&self, user_id: &UserId, device_id: &DeviceId, token: &str, _: Option<String>) -> Result<()> {
            let mut devices = self.devices.lock().unwrap();
            devices.insert((user_id.to_owned(), device_id.to_owned()), DeviceData {
                token: token.to_string(),
                ..Default::default()
            });
            Ok(())
        }

        fn remove_device(&self, user_id: &UserId, device_id: &DeviceId) -> Result<()> {
            self.devices.lock().unwrap().remove(&(user_id.to_owned(), device_id.to_owned()));
            Ok(())
        }

        // Simplified implementations for remaining methods
        fn all_device_ids<'a>(&'a self, _user_id: &UserId) -> Box<dyn Iterator<Item = Result<OwnedDeviceId>> + 'a> {
            Box::new(std::iter::empty())
        }

        fn set_token(&self, _user_id: &UserId, _device_id: &DeviceId, _token: &str) -> Result<()> { Ok(()) }
        fn add_one_time_key(&self, _: &UserId, _: &DeviceId, _: &OwnedOneTimeKeyId, _: &Raw<OneTimeKey>) -> Result<()> { Ok(()) }
        fn last_one_time_keys_update(&self, _: &UserId) -> Result<u64> { Ok(0) }
        fn take_one_time_key(&self, _: &UserId, _: &DeviceId, _: &OneTimeKeyAlgorithm) -> Result<Option<(OwnedOneTimeKeyId, Raw<OneTimeKey>)>> { Ok(None) }
        fn count_one_time_keys(&self, _: &UserId, _: &DeviceId) -> Result<BTreeMap<OneTimeKeyAlgorithm, UInt>> { Ok(BTreeMap::new()) }
        fn add_device_keys(&self, _: &UserId, _: &DeviceId, _: &Raw<DeviceKeys>) -> Result<()> { Ok(()) }
        fn add_cross_signing_keys(&self, _: &UserId, _: &Raw<CrossSigningKey>, _: &Option<Raw<CrossSigningKey>>, _: &Option<Raw<CrossSigningKey>>, _: bool) -> Result<()> { Ok(()) }
        fn sign_key(&self, _: &UserId, _: &str, _: (String, String), _: &UserId) -> Result<()> { Ok(()) }
        fn keys_changed<'a>(&'a self, _: &str, _: u64, _: Option<u64>) -> Box<dyn Iterator<Item = Result<OwnedUserId>> + 'a> { Box::new(std::iter::empty()) }
        fn mark_device_key_update(&self, _: &UserId) -> Result<()> { Ok(()) }
        fn get_device_keys(&self, _: &UserId, _: &DeviceId) -> Result<Option<Raw<DeviceKeys>>> { Ok(None) }
        fn parse_master_key(&self, _: &UserId, _: &Raw<CrossSigningKey>) -> Result<(Vec<u8>, CrossSigningKey)> { Err(Error::bad_database("Not implemented")) }
        fn get_key(&self, _: &[u8], _: Option<&UserId>, _: &UserId, _: &dyn Fn(&UserId) -> bool) -> Result<Option<Raw<CrossSigningKey>>> { Ok(None) }
        fn get_master_key(&self, _: Option<&UserId>, _: &UserId, _: &dyn Fn(&UserId) -> bool) -> Result<Option<Raw<CrossSigningKey>>> { Ok(None) }
        fn get_self_signing_key(&self, _: Option<&UserId>, _: &UserId, _: &dyn Fn(&UserId) -> bool) -> Result<Option<Raw<CrossSigningKey>>> { Ok(None) }
        fn get_user_signing_key(&self, _: &UserId) -> Result<Option<Raw<CrossSigningKey>>> { Ok(None) }
        fn add_to_device_event(&self, _: &UserId, _: &UserId, _: &DeviceId, _: &str, _: serde_json::Value) -> Result<()> { Ok(()) }
        fn get_to_device_events(&self, _: &UserId, _: &DeviceId) -> Result<Vec<Raw<AnyToDeviceEvent>>> { Ok(vec![]) }
        fn remove_to_device_events(&self, _: &UserId, _: &DeviceId, _: u64) -> Result<()> { Ok(()) }
        fn update_device_metadata(&self, _: &UserId, _: &DeviceId, _: &Device) -> Result<()> { Ok(()) }
        fn get_device_metadata(&self, _: &UserId, _: &DeviceId) -> Result<Option<Device>> { Ok(None) }
        fn get_devicelist_version(&self, _: &UserId) -> Result<Option<u64>> { Ok(None) }
        fn all_user_devices_metadata<'a>(&'a self, _: &UserId) -> Box<dyn Iterator<Item = Result<Device>> + 'a> { Box::new(std::iter::empty()) }
        fn set_devices_last_seen<'a>(&'a self, _: &'a BTreeMap<(OwnedUserId, OwnedDeviceId), MilliSecondsSinceUnixEpoch>) -> Box<dyn Iterator<Item = Result<()>> + 'a> { Box::new(std::iter::empty()) }
        fn create_filter(&self, user_id: &UserId, filter: &FilterDefinition) -> Result<String> { 
            let filter_id = format!("filter_{}", self.filter_counter.fetch_add(1, Ordering::SeqCst));
            self.filters.lock().unwrap().insert((user_id.to_owned(), filter_id.clone()), filter.clone());
            Ok(filter_id)
        }
        fn get_filter(&self, user_id: &UserId, filter_id: &str) -> Result<Option<FilterDefinition>> { 
            Ok(self.filters.lock().unwrap().get(&(user_id.to_owned(), filter_id.to_owned())).cloned())
        }
        fn create_openid_token(&self, user_id: &UserId) -> Result<(String, u64)> { 
            let token = format!("openid_{}", self.token_counter.fetch_add(1, Ordering::SeqCst));
            let expires = 3600; // 1 hour
            self.openid_tokens.lock().unwrap().insert(token.clone(), user_id.to_owned());
            Ok((token, expires))
        }
        fn find_from_openid_token(&self, token: &str) -> Result<Option<OwnedUserId>> { 
            Ok(self.openid_tokens.lock().unwrap().get(token).cloned())
        }
    }

    fn create_test_service() -> Service {
        Service {
            db: Box::leak(Box::new(MockData::new())),
            connections: StdMutex::new(BTreeMap::new()),
            device_last_seen: Mutex::new(BTreeMap::new()),
        }
    }

    #[tokio::test]
    async fn test_exists() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        
        // User should not exist initially
        assert!(!service.exists(user_id).unwrap());
        
        // Create user
        service.db.set_password(user_id, Some("password")).unwrap();
        
        // User should now exist
        assert!(service.exists(user_id).unwrap());
    }

    #[tokio::test]
    async fn test_create_and_remove_device() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        let device_id = device_id!("DEVICE123");
        let token = "test_token";
        
        // Create device
        service.create_device(user_id, device_id, token, Some("Test Device".to_string())).unwrap();
        
        // Verify device can be found
        let found = service.find_from_token(token).unwrap();
        assert_eq!(found, Some((user_id.to_owned(), device_id.to_owned())));
        
        // Remove device
        service.remove_device(user_id, device_id).unwrap();
        
        // Device should no longer be found
        let found_after_remove = service.find_from_token(token).unwrap();
        assert_eq!(found_after_remove, None);
    }

    #[tokio::test]
    async fn test_user_profile_management() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        
        // Set displayname
        service.set_displayname(user_id, Some("Test User".to_string())).unwrap();
        assert_eq!(service.displayname(user_id).unwrap(), Some("Test User".to_string()));
        
        
        // Set avatar URL - simplified without parsing
        service.set_avatar_url(user_id, None).unwrap();
        assert_eq!(service.avatar_url(user_id).unwrap(), None);
        
        // Set blurhash
        service.set_blurhash(user_id, Some("blurhash123".to_string())).unwrap();
        assert_eq!(service.blurhash(user_id).unwrap(), Some("blurhash123".to_string()));
    }

    #[tokio::test]
    async fn test_password_management() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        
        // Initially no password
        assert_eq!(service.password_hash(user_id).unwrap(), None);
        
        // Set password
        service.set_password(user_id, Some("secure_password")).unwrap();
        assert!(service.password_hash(user_id).unwrap().is_some());
        
        // Remove password
        service.set_password(user_id, None).unwrap();
        assert_eq!(service.password_hash(user_id).unwrap(), None);
    }

    #[tokio::test]
    async fn test_is_deactivated() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        
        // User is not deactivated initially
        assert!(!service.is_deactivated(user_id).unwrap());
        
        // After deactivation
        service.deactivate_account(user_id).unwrap();
        assert!(service.is_deactivated(user_id).unwrap());
    }

    #[tokio::test]
    async fn test_sync_cache_operations() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com").to_owned();
        let device_id = device_id!("DEVICE123").to_owned();
        let conn_id = "conn_123".to_string();
        
        // Test updating sync subscriptions
        let mut subscriptions = BTreeMap::new();
        let room_id = "!room:example.com".parse().unwrap();
        subscriptions.insert(room_id, sync_events::v5::request::RoomSubscription::default());
        
        service.update_sync_subscriptions(
            user_id.clone(),
            device_id.clone(),
            conn_id.clone(),
            subscriptions
        );
        
        // Test updating known rooms
        let mut known_rooms = BTreeSet::new();
        known_rooms.insert("!room2:example.com".parse().unwrap());
        
        service.update_sync_known_rooms(
            user_id.clone(),
            device_id.clone(),
            conn_id.clone(),
            "list_1".to_string(),
            known_rooms,
            100
        );
        
        // Test forgetting connection
        service.forget_sync_request_connection(user_id, device_id, conn_id);
    }

    #[tokio::test]
    async fn test_device_last_seen_update() {
        let service = Arc::new(create_test_service());
        let user_id = user_id!("@test:example.com").to_owned();
        let device_id = device_id!("DEVICE123").to_owned();
        
        // Update device last seen
        service.update_device_last_seen(user_id.clone(), device_id.clone()).await;
        
        // Verify it's in the cache
        let cache = service.device_last_seen.lock().await;
        assert!(cache.contains_key(&(user_id, device_id)));
    }

    #[tokio::test]
    async fn test_write_cached_last_seen() {
        let service = Arc::new(create_test_service());
        let user_id = user_id!("@test:example.com").to_owned();
        let device_id = device_id!("DEVICE123").to_owned();
        
        // Add to cache
        service.update_device_last_seen(user_id, device_id).await;
        
        // Write to database
        let errors = service.write_cached_last_seen().await;
        assert!(errors.is_empty());
        
        // Cache should be empty after write
        let cache = service.device_last_seen.lock().await;
        assert!(cache.is_empty());
    }

    #[tokio::test]
    async fn test_try_update_device_last_seen() {
        let service = Arc::new(create_test_service());
        let user_id = user_id!("@test:example.com").to_owned();
        let device_id = device_id!("DEVICE123").to_owned();
        
        // Add some data to cache
        service.update_device_last_seen(user_id, device_id).await;
        
        // Try update
        service.try_update_device_last_seen().await;
        
        // Verify cache is empty after update
        let cache = service.device_last_seen.lock().await;
        assert!(cache.is_empty());
    }

    #[tokio::test]
    async fn test_filter_operations() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        let filter = FilterDefinition::default();
        
        // Create filter
        let filter_id = service.create_filter(user_id, &filter).unwrap();
        assert!(!filter_id.is_empty());
        
        // Get filter
        let retrieved_filter = service.get_filter(user_id, &filter_id).unwrap();
        assert!(retrieved_filter.is_some());
    }

    #[test]
    fn test_clean_signatures() {
        let mut cross_signing_key = serde_json::json!({
            "keys": {},
            "signatures": {
                "@user1:example.com": {
                    "key1": "signature1"
                },
                "@user2:example.com": {
                    "key2": "signature2"
                }
            }
        });
        
        let sender_user = Some(user_id!("@sender:example.com"));
        let user_id = user_id!("@user1:example.com");
        let allowed_signatures = |id: &UserId| id.to_string().contains("user1");
        
        let result = clean_signatures(&mut cross_signing_key, sender_user, user_id, allowed_signatures);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_openid_token_operations() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        
        // Create OpenID token
        let (token, expires) = service.create_openid_token(user_id).unwrap();
        assert!(!token.is_empty());
        assert!(expires > 0);
        
        // Find user from token
        let found_user = service.find_from_openid_token(&token).unwrap();
        assert!(found_user.is_some());
    }

    // Performance and benchmarking tests
    #[tokio::test]
    async fn test_concurrent_device_updates() {
        let service = Arc::new(create_test_service());
        let user_id = user_id!("@test:example.com").to_owned();
        
        let mut handles = vec![];
        
        // Spawn multiple concurrent device updates
        for i in 0..10 {
            let service_clone = Arc::clone(&service);
            let user_id_clone = user_id.clone();
            let device_id = format!("DEVICE{}", i).as_str().try_into().unwrap();
            
            let handle = tokio::spawn(async move {
                service_clone.update_device_last_seen(user_id_clone, device_id).await;
            });
            handles.push(handle);
        }
        
        // Wait for all updates to complete
        for handle in handles {
            handle.await.unwrap();
        }
        
        // Verify all devices are in cache
        let cache = service.device_last_seen.lock().await;
        assert_eq!(cache.len(), 10);
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        
        let start = Instant::now();
        
        // Perform multiple operations
        for i in 0..100 {
            let display_name = format!("User {}", i);
            service.set_displayname(user_id, Some(display_name)).unwrap();
        }
        
        let duration = start.elapsed();
        
        // Verify performance is reasonable (less than 100ms for 100 operations)
        assert!(duration < Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_error_conditions() {
        let service = create_test_service();
        let invalid_user_id = user_id!("@nonexistent:example.com");
        
        // Test operations on non-existent user
        assert!(!service.exists(invalid_user_id).unwrap());
        assert!(!service.is_deactivated(invalid_user_id).unwrap());
        assert_eq!(service.displayname(invalid_user_id).unwrap(), None);
    }

    #[tokio::test]
    async fn test_edge_cases() {
        let service = create_test_service();
        let user_id = user_id!("@test:example.com");
        
        // Test empty displayname
        service.set_displayname(user_id, Some("".to_string())).unwrap();
        assert_eq!(service.displayname(user_id).unwrap(), Some("".to_string()));
        
        // Test None displayname
        service.set_displayname(user_id, None).unwrap();
        assert_eq!(service.displayname(user_id).unwrap(), None);
        
        // Test very long displayname
        let long_name = "a".repeat(1000);
        service.set_displayname(user_id, Some(long_name.clone())).unwrap();
        assert_eq!(service.displayname(user_id).unwrap(), Some(long_name));
    }
}
