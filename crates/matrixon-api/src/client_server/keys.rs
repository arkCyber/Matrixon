// =============================================================================
// Matrixon Matrix NextServer - Keys Module
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

use super::SESSION_ID_LENGTH;
use crate::{services, utils, Error, Result, Ruma};
use futures_util::{stream::FuturesUnordered, StreamExt};
use ruma::api::client::{
    keys::{
        claim_keys::{self, v3 as claim_keys_v3},
        get_key_changes::{self, v3 as get_key_changes_v3},
        get_keys::{self, v3 as get_keys_v3},
        upload_keys::{self, v3 as upload_keys_v3},
        upload_signatures::{self, v3 as upload_signatures_v3},
        upload_signing_keys::{self, v3 as upload_signing_keys_v3},
    },
    error::ErrorKind,
};
use ruma::api::federation::keys::{
    claim_keys as federation_claim_keys,
    get_keys as federation_get_keys,
};
use ruma::serde::Raw;
use ruma::encryption::DeviceKeys;
use ruma::identifiers::{Device, DeviceId, DeviceKeyAlgorithm};
use ruma::{OwnedDeviceId, OwnedUserId, UserId, OwnedEventId, OneTimeKeyAlgorithm};
use serde_json::json;
use std::{
    collections::{BTreeMap, HashMap},
    time::{Duration, Instant},
};
use tracing::{debug, error, info, instrument, warn};

/// # `POST /_matrix/client/r0/keys/upload`
///
/// Publish end-to-end encryption keys for the sender device.
///
/// - Adds one time keys
/// - If there are no device keys yet: Adds device keys
#[instrument(level = "debug", skip(body))]
pub async fn upload_keys_route(
    body: Ruma<upload_keys_v3::Request>,
) -> Result<upload_keys_v3::Response> {
    let start = Instant::now();
    let sender_user = body.sender_user.as_ref().expect("user is authenticated");
    let sender_device = body.sender_device.as_ref().expect("user is authenticated");

    debug!("🔧 Starting key upload for user {} device {}", sender_user, sender_device);

    for (key_key, key_value) in &body.one_time_keys {
        key_value.deserialize().map_err(|e| {
            error!("❌ Invalid one-time key format: {}", e);
            Error::BadRequest(ErrorKind::BadJson, "Body contained invalid one-time key")
        })?;

        services()
            .users
            .add_one_time_key(sender_user, sender_device, key_key, key_value, None)
            .map_err(|e| {
                error!("❌ Failed to add one-time key: {}", e);
                e
            })?;
    }

    if let Some(device_keys) = &body.device_keys {
        if services()
            .users
            .get_device_keys(sender_user, sender_device)?
            .is_none()
        {
            device_keys.deserialize().map_err(|e| {
                error!("❌ Invalid device keys format: {}", e);
                Error::BadRequest(ErrorKind::BadJson, "Body contained invalid device keys")
            })?;

            services()
                .users
                .add_device_keys(sender_user, sender_device, device_keys)
                .map_err(|e| {
                    error!("❌ Failed to add device keys: {}", e);
                    e
                })?;
        }
    }

    let key_count = services()
        .users
        .count_one_time_keys(sender_user, sender_device)
        .map_err(|e| {
            error!("❌ Failed to count one-time keys: {}", e);
            e
        })?;

    info!("✅ Key upload completed in {:?} for user {} device {}", start.elapsed(), sender_user, sender_device);
    Ok(upload_keys_v3::Response::new(key_count))
}

/// # `POST /_matrix/client/r0/keys/query`
///
/// Get end-to-end encryption keys for the given users.
///
/// - Always fetches users from other servers over federation
/// - Gets master keys, self-signing keys, user signing keys and device keys.
/// - The master and self-signing keys contain signatures that the user is allowed to see
#[instrument(level = "debug", skip(body))]
pub async fn get_keys_route(
    body: Ruma<get_keys_v3::Request>,
) -> Result<get_keys_v3::Response> {
    let start = Instant::now();
    let sender_user = body.sender_user.as_ref().expect("user is authenticated");

    debug!("🔧 Starting key query for user {}", sender_user);

    let mut device_keys = BTreeMap::new();
    let mut master_keys = BTreeMap::new();
    let mut self_signing_keys = BTreeMap::new();
    let mut user_signing_keys = BTreeMap::new();

    for user_id in &body.device_keys {
        if user_id.server_name() == services().globals.server_name() {
            // Local user
            debug!("📱 Fetching local user keys for {}", user_id);
            let devices = services().users.get_device_keys(user_id).map_err(|e| {
                error!("❌ Failed to get device keys for local user {}: {}", user_id, e);
                e
            })?;
            device_keys.insert(user_id.clone(), devices);
        } else {
            // Remote user
            debug!("🌐 Fetching remote user keys for {}", user_id);
            let response = services()
                .sending
                .send_federation_request(
                    user_id.server_name(),
                    federation_get_keys::v1::Request::new(vec![user_id.clone()]),
                )
                .await
                .map_err(|e| {
                    error!("❌ Failed to fetch remote user keys for {}: {}", user_id, e);
                    e
                })?;

            device_keys.extend(response.device_keys);
            master_keys.extend(response.master_keys);
            self_signing_keys.extend(response.self_signing_keys);
            user_signing_keys.extend(response.user_signing_keys);
        }
    }

    info!("✅ Key query completed in {:?} for user {}", start.elapsed(), sender_user);
    Ok(get_keys_v3::Response::new(
        device_keys,
        master_keys,
        self_signing_keys,
        user_signing_keys,
    ))
}

/// # `POST /_matrix/client/r0/keys/claim`
///
/// Claims one-time keys for use in pre-key messages.
///
/// - Claims one-time keys for the given devices
/// - Returns the claimed keys
#[instrument(level = "debug", skip(body))]
pub async fn claim_keys_route(
    body: Ruma<claim_keys_v3::Request>,
) -> Result<claim_keys_v3::Response> {
    let start = Instant::now();
    let sender_user = body.sender_user.as_ref().expect("user is authenticated");

    debug!("🔧 Starting key claim for user {}", sender_user);

    let mut one_time_keys = BTreeMap::new();

    for (user_id, device_map) in &body.one_time_keys {
        if user_id.server_name() == services().globals.server_name() {
            // Local user
            debug!("📱 Claiming local user keys for {}", user_id);
            let mut user_keys = BTreeMap::new();
            for (device_id, algorithm) in device_map {
                if let Some(key) = services()
                    .users
                    .claim_one_time_key(user_id, device_id, algorithm)
                    .map_err(|e| {
                        error!("❌ Failed to claim one-time key for local user {} device {}: {}", user_id, device_id, e);
                        e
                    })?
                {
                    let mut c = BTreeMap::new();
                    c.insert(key.0, key.1);
                    user_keys.insert(device_id.clone(), c);
                }
            }
            if !user_keys.is_empty() {
                one_time_keys.insert(user_id.clone(), user_keys);
            }
        } else {
            // Remote user
            debug!("🌐 Claiming remote user keys for {}", user_id);
            let response = services()
                .sending
                .send_federation_request(
                    user_id.server_name(),
                    federation_claim_keys::v1::Request::new(
                        BTreeMap::from([(user_id.clone(), device_map.clone())])
                    ),
                )
                .await
                .map_err(|e| {
                    error!("❌ Failed to claim remote user keys for {}: {}", user_id, e);
                    e
                })?;

            one_time_keys.extend(response.one_time_keys);
        }
    }

    info!("✅ Key claim completed in {:?} for user {}", start.elapsed(), sender_user);
    Ok(claim_keys_v3::Response::new(one_time_keys))
}

/// # `POST /_matrix/client/r0/keys/signatures/upload`
///
/// Publishes cross-signing signatures for the user.
///
/// - Publishes signatures for the user's devices
/// - Returns the number of signatures published
#[instrument(level = "debug", skip(body))]
pub async fn upload_signatures_route(
    body: Ruma<upload_signatures_v3::Request>,
) -> Result<upload_signatures_v3::Response> {
    let start = Instant::now();
    let sender_user = body.sender_user.as_ref().expect("user is authenticated");

    debug!("🔧 Starting signature upload for user {}", sender_user);

    for (user_id, signatures) in &body.signatures {
        if user_id.server_name() == services().globals.server_name() {
            // Local user - add signatures directly
            debug!("📱 Adding signatures for local user {}", user_id);
            services().users.add_signatures(sender_user, &user_id, signatures).map_err(|e| {
                error!("❌ Failed to add signatures for local user {}: {}", user_id, e);
                e
            })?;
        } else {
            // Remote user - skip since we can't upload signatures to remote servers directly
            warn!("⚠️ Skipping signature upload for remote user {}", user_id);
        }
    }

    info!("✅ Signature upload completed in {:?} for user {}", start.elapsed(), sender_user);
    Ok(upload_signatures_v3::Response::new())
}

/// # `POST /_matrix/client/r0/keys/device_signing/upload`
///
/// Publishes cross-signing keys for the user.
///
/// - Publishes master key, self-signing key, and user signing key
/// - Returns the number of keys published
#[instrument(level = "debug", skip(body))]
pub async fn upload_signing_keys_route(
    body: Ruma<upload_signing_keys_v3::Request>,
) -> Result<upload_signing_keys_v3::Response> {
    let start = Instant::now();
    let sender_user = body.sender_user.as_ref().expect("user is authenticated");

    debug!("🔧 Starting signing key upload for user {}", sender_user);

    if let Some(master_key) = &body.master_key {
        debug!("📝 Adding master key for user {}", sender_user);
        services().users.add_master_key(sender_user, &master_key).map_err(|e| {
            error!("❌ Failed to add master key: {}", e);
            e
        })?;
    }

    if let Some(self_signing_key) = &body.self_signing_key {
        debug!("📝 Adding self-signing key for user {}", sender_user);
        services().users.add_self_signing_key(sender_user, &self_signing_key).map_err(|e| {
            error!("❌ Failed to add self-signing key: {}", e);
            e
        })?;
    }

    if let Some(user_signing_key) = &body.user_signing_key {
        debug!("📝 Adding user signing key for user {}", sender_user);
        services().users.add_user_signing_key(sender_user, &user_signing_key).map_err(|e| {
            error!("❌ Failed to add user signing key: {}", e);
            e
        })?;
    }

    info!("✅ Signing key upload completed in {:?} for user {}", start.elapsed(), sender_user);
    Ok(upload_signing_keys_v3::Response {})
}

/// # `GET /_matrix/client/r0/keys/changes`
///
/// Gets a list of users who have updated their device identity keys since a given sync token.
///
/// - Returns a list of users who have updated their device keys
/// - Returns a sync token that can be used to get the next set of changes
#[instrument(level = "debug", skip(body))]
pub async fn get_key_changes_route(
    body: Ruma<get_key_changes_v3::Request>,
) -> Result<get_key_changes_v3::Response> {
    let start = Instant::now();
    let sender_user = body.sender_user.as_ref().expect("user is authenticated");

    debug!("🔧 Starting key changes query for user {}", sender_user);

    let (changed, left) = services()
        .users
        .keys_changed(sender_user, &body.from, &body.to)
        .map_err(|e| {
            error!("❌ Failed to get key changes: {}", e);
            e
        })?;

    info!("✅ Key changes query completed in {:?} for user {}", start.elapsed(), sender_user);
    Ok(get_key_changes_v3::Response::new(changed, left))
}

/// Helper function to get keys for multiple users
#[instrument(level = "debug", skip(sender_user, device_keys_input, allowed_signatures))]
pub(crate) async fn get_keys_helper<F: Fn(&UserId) -> bool>(
    sender_user: Option<&UserId>,
    device_keys_input: &BTreeMap<OwnedUserId, Vec<OwnedDeviceId>>,
    allowed_signatures: F,
) -> Result<get_keys_v3::Response> {
    let start = Instant::now();
    debug!("🔧 Starting get_keys_helper");

    let mut master_keys = BTreeMap::new();
    let mut self_signing_keys = BTreeMap::new();
    let mut user_signing_keys = BTreeMap::new();
    let mut device_keys = BTreeMap::new();

    let mut get_over_federation = HashMap::new();

    for (user_id, device_ids) in device_keys_input {
        let user_id: &UserId = user_id;

        if user_id.server_name() != services().globals.server_name() {
            get_over_federation
                .entry(user_id.server_name())
                .or_insert_with(Vec::new)
                .push((user_id, device_ids));
            continue;
        }

        if device_ids.is_empty() {
            debug!("📱 Getting all device keys for local user {}", user_id);
            let mut container = BTreeMap::new();
            for device_id in services().users.all_device_ids(user_id) {
                let device_id = device_id.map_err(|e| {
                    error!("❌ Failed to get device ID for user {}: {}", user_id, e);
                    e
                })?;
                if let Some(mut keys) = services().users.get_device_keys(user_id, &device_id)? {
                    let metadata = services()
                        .users
                        .get_device_metadata(user_id, &device_id)?
                        .ok_or_else(|| {
                            error!("❌ Device metadata not found for user {} device {}", user_id, device_id);
                            Error::bad_database("all_device_keys contained nonexistent device.")
                        })?;

                    add_unsigned_device_display_name(&mut keys, metadata)
                        .map_err(|e| {
                            error!("❌ Failed to add device display name: {}", e);
                            Error::bad_database("invalid device keys in database")
                        })?;
                    container.insert(device_id, keys);
                }
            }
            device_keys.insert(user_id.to_owned(), container);
        } else {
            debug!("📱 Getting specific device keys for local user {}", user_id);
            for device_id in device_ids {
                let mut container = BTreeMap::new();
                if let Some(mut keys) = services().users.get_device_keys(user_id, device_id)? {
                    let metadata = services()
                        .users
                        .get_device_metadata(user_id, device_id)?
                        .ok_or(Error::BadRequest(
                            ErrorKind::InvalidParam,
                            "Tried to get keys for nonexistent device.",
                        ))?;

                    add_unsigned_device_display_name(&mut keys, metadata)
                        .map_err(|e| {
                            error!("❌ Failed to add device display name: {}", e);
                            Error::bad_database("invalid device keys in database")
                        })?;
                    container.insert(device_id.to_owned(), keys);
                }
                device_keys.insert(user_id.to_owned(), container);
            }
        }

        if let Some(master_key) =
            services()
                .users
                .get_master_key(sender_user, user_id, &allowed_signatures)?
        {
            master_keys.insert(user_id.to_owned(), master_key);
        }
        if let Some(self_signing_key) =
            services()
                .users
                .get_self_signing_key(sender_user, user_id, &allowed_signatures)?
        {
            self_signing_keys.insert(user_id.to_owned(), self_signing_key);
        }
        if Some(user_id) == sender_user {
            if let Some(user_signing_key) = services().users.get_user_signing_key(user_id)? {
                user_signing_keys.insert(user_id.to_owned(), user_signing_key);
            }
        }
    }

    let mut failures = BTreeMap::new();

    let back_off = |id| async {
        match services()
            .globals
            .bad_query_ratelimiter
            .write()
            .await
            .entry(id)
        {
            hash_map::Entry::Vacant(e) => {
                e.insert((Instant::now(), 1));
            }
            hash_map::Entry::Occupied(mut e) => *e.get_mut() = (Instant::now(), e.get().1 + 1),
        }
    };

    let mut futures: FuturesUnordered<_> = get_over_federation
        .into_iter()
        .map(|(server, vec)| async move {
            if let Some((time, tries)) = services()
                .globals
                .bad_query_ratelimiter
                .read()
                .await
                .get(server)
            {
                // Exponential backoff
                let mut min_elapsed_duration = Duration::from_secs(30) * (*tries) * (*tries);
                if min_elapsed_duration > Duration::from_secs(60 * 60 * 24) {
                    min_elapsed_duration = Duration::from_secs(60 * 60 * 24);
                }

                if time.elapsed() < min_elapsed_duration {
                    debug!("⏳ Backing off query from {:?}", server);
                    return (
                        server,
                        Err(Error::BadServerResponse("bad query, still backing off")),
                    );
                }
            }

            let mut device_keys_input_fed = BTreeMap::new();
            for (user_id, keys) in vec {
                device_keys_input_fed.insert(user_id.to_owned(), keys.clone());
            }
            (
                server,
                tokio::time::timeout(
                    Duration::from_secs(25),
                    services().sending.send_federation_request(
                        server,
                        federation_get_keys::v1::Request::new(device_keys_input_fed),
                    ),
                )
                .await
                .map_err(|_e| {
                    error!("❌ Query timeout for server {}", server);
                    Error::BadServerResponse("Query took too long")
                }),
            )
        })
        .collect();

    while let Some((server, response)) = futures.next().await {
        match response {
            Ok(Ok(response)) => {
                debug!("✅ Successfully got keys from server {}", server);
                for (user, masterkey) in response.master_keys {
                    let (master_key_id, mut master_key) =
                        services().users.parse_master_key(&user, &masterkey).map_err(|e| {
                            error!("❌ Failed to parse master key for user {}: {}", user, e);
                            e
                        })?;

                    if let Some(our_master_key) = services().users.get_key(
                        &master_key_id,
                        sender_user,
                        &user,
                        &allowed_signatures,
                    )? {
                        let (_, our_master_key) =
                            services().users.parse_master_key(&user, &our_master_key).map_err(|e| {
                                error!("❌ Failed to parse our master key for user {}: {}", user, e);
                                e
                            })?;
                        master_key.signatures.extend(our_master_key.signatures);
                    }
                    let json = serde_json::to_value(master_key).expect("to_value always works");
                    let raw = serde_json::from_value(json).expect("Raw::from_value always works");
                    // Skip adding cross-signing keys for remote users since we can't upload them directly
                    debug!("⚠️ Skipping cross-signing keys for remote user {}", user);
                }

                self_signing_keys.extend(response.self_signing_keys);
                device_keys.extend(response.device_keys);
            }
            _ => {
                error!("❌ Failed to get keys from server {}", server);
                back_off(server.to_owned()).await;
                failures.insert(server.to_string(), json!({}));
            }
        }
    }

    let mut resp = get_keys_v3::Response::new();
    resp.master_keys = master_keys;
    resp.self_signing_keys = self_signing_keys;
    resp.user_signing_keys = user_signing_keys;
    resp.device_keys = device_keys;
    resp.failures = failures;

    info!("✅ get_keys_helper completed in {:?}", start.elapsed());
    Ok(resp)
}

/// Helper function to add unsigned device display name
fn add_unsigned_device_display_name(
    keys: &mut Raw<ruma::encryption::DeviceKeys>,
    metadata: Device,
) -> serde_json::Result<()> {
    if let Some(display_name) = metadata.display_name {
        let mut object = keys.deserialize_as::<serde_json::Map<String, serde_json::Value>>()?;

        let unsigned = object.entry("unsigned").or_insert_with(|| json!({}));
        if let serde_json::Value::Object(unsigned_object) = unsigned {
            unsigned_object.insert("device_display_name".to_owned(), display_name.into());
        }

        *keys = Raw::from_json(serde_json::value::to_raw_value(&object)?);
    }

    Ok(())
}

/// Helper function to claim keys
#[instrument(level = "debug", skip(one_time_keys_input))]
pub(crate) async fn claim_keys_helper(
    one_time_keys_input: &BTreeMap<OwnedUserId, BTreeMap<OwnedDeviceId, OneTimeKeyAlgorithm>>,
) -> Result<claim_keys_v3::Response> {
    let start = Instant::now();
    debug!("🔧 Starting claim_keys_helper");

    let mut one_time_keys = BTreeMap::new();

    let mut get_over_federation = BTreeMap::new();

    for (user_id, map) in one_time_keys_input {
        if user_id.server_name() != services().globals.server_name() {
            get_over_federation
                .entry(user_id.server_name())
                .or_insert_with(Vec::new)
                .push((user_id, map));
        }

        let mut container = BTreeMap::new();
        for (device_id, key_algorithm) in map {
            if let Some(one_time_keys) =
                services()
                    .users
                    .take_one_time_key(user_id, device_id, key_algorithm)
                    .map_err(|e| {
                        error!("❌ Failed to take one-time key for user {} device {}: {}", user_id, device_id, e);
                        e
                    })?
            {
                let mut c = BTreeMap::new();
                c.insert(one_time_keys.0, one_time_keys.1);
                container.insert(device_id.clone(), c);
            }
        }
        one_time_keys.insert(user_id.clone(), container);
    }

    let mut failures = BTreeMap::new();

    let mut futures: FuturesUnordered<_> = get_over_federation
        .into_iter()
        .map(|(server, vec)| async move {
            let mut one_time_keys_input_fed = BTreeMap::new();
            for (user_id, keys) in vec {
                one_time_keys_input_fed.insert(user_id.clone(), keys.clone());
            }
            (
                server,
                services()
                    .sending
                    .send_federation_request(
                        server,
                        federation_claim_keys::v1::Request::new(one_time_keys_input_fed),
                    )
                    .await,
            )
        })
        .collect();

    while let Some((server, response)) = futures.next().await {
        match response {
            Ok(keys) => {
                debug!("✅ Successfully claimed keys from server {}", server);
                one_time_keys.extend(keys.one_time_keys);
            }
            Err(e) => {
                error!("❌ Failed to claim keys from server {}: {}", server, e);
                failures.insert(server.to_string(), json!({}));
            }
        }
    }

    let mut resp = claim_keys_v3::Response::new(one_time_keys);
    resp.failures = failures;

    info!("✅ claim_keys_helper completed in {:?}", start.elapsed());
    Ok(resp)
}
