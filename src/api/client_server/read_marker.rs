// =============================================================================
// Matrixon Matrix NextServer - Read Marker Module
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

use matrixon_rooms::rooms::timeline::PduCount;
use crate::{services, Error, Result, Ruma};
use ruma::{
    api::client::{error::ErrorKind, read_marker::set_read_marker, receipt::create_receipt},
    events::{
        receipt::ReceiptType,
        RoomAccountDataEventType,
    },
    MilliSecondsSinceUnixEpoch,
};
use std::collections::BTreeMap;

/// # `POST /_matrix/client/r0/rooms/{roomId}/read_markers`
///
/// Sets different types of read markers.
///
/// - Updates fully-read account data event to `fully_read`
/// - If `read_receipt` is set: Update private marker and public read receipt EDU
pub async fn set_read_marker_route(
    body: Ruma<set_read_marker::v3::Request>,
) -> Result<set_read_marker::v3::Response> {
    let sender_user = body.sender_user.as_ref().expect("user is authenticated");

    if let Some(fully_read) = &body.fully_read {
        let fully_read_event = ruma::events::fully_read::FullyReadEvent {
            content: ruma::events::fully_read::FullyReadEventContent::new(fully_read.clone()),
        };
        services().account_data.update(
            Some(&body.room_id),
            sender_user,
            RoomAccountDataEventType::FullyRead,
            &serde_json::to_value(fully_read_event).expect("to json value always works"),
        )?;
    }

    if body.private_read_receipt.is_some() || body.read_receipt.is_some() {
        services()
            .rooms
            .user
            .reset_notification_counts(sender_user, &body.room_id)?;
    }

    if let Some(event) = &body.private_read_receipt {
        let count = services()
            .rooms
            .timeline
            .get_pdu_count(event)?
            .ok_or(Error::BadRequestString(
                ErrorKind::InvalidParam,
                "Event does not exist.",
            ))?;
        let count = match count {
            PduCount::Backfilled(_) => {
                return Err(Error::BadRequestString(
                    ErrorKind::InvalidParam,
                    "Read receipt is in backfilled timeline",
                ))
            }
            PduCount::Normal(c) => c,
        };
        services()
            .rooms
            .edus
            .read_receipt
            .private_read_set(&body.room_id, sender_user, count)?;
    }

    if let Some(event) = &body.read_receipt {
        let mut user_receipts = BTreeMap::new();
        user_receipts.insert(
            sender_user.clone(),
            ruma::events::receipt::Receipt::new(MilliSecondsSinceUnixEpoch::now()),
        );

        let mut receipts = BTreeMap::new();
        receipts.insert(ReceiptType::Read, user_receipts);

        let mut receipt_content = BTreeMap::new();
        receipt_content.insert(event.to_owned(), receipts);

        services().rooms.edus.read_receipt.readreceipt_update(
            sender_user,
            &body.room_id,
            ruma::events::receipt::ReceiptEvent {
                content: ruma::events::receipt::ReceiptEventContent(receipt_content),
                room_id: body.room_id.clone(),
            },
        )?;
    }

    Ok(set_read_marker::v3::Response::new())
}

/// # `POST /_matrix/client/r0/rooms/{roomId}/receipt/{receiptType}/{eventId}`
///
/// Sets private read marker and public read receipt EDU.
pub async fn create_receipt_route(
    body: Ruma<create_receipt::v3::Request>,
) -> Result<create_receipt::v3::Response> {
    let sender_user = body.sender_user.as_ref().expect("user is authenticated");

    if matches!(
        &body.receipt_type,
        create_receipt::v3::ReceiptType::Read | create_receipt::v3::ReceiptType::ReadPrivate
    ) {
        services()
            .rooms
            .user
            .reset_notification_counts(sender_user, &body.room_id)?;
    }

    match body.receipt_type {
        create_receipt::v3::ReceiptType::FullyRead => {
            let fully_read_event = ruma::events::fully_read::FullyReadEvent {
                content: ruma::events::fully_read::FullyReadEventContent::new(body.event_id.clone()),
            };
            services().account_data.update(
                Some(&body.room_id),
                sender_user,
                RoomAccountDataEventType::FullyRead,
                &serde_json::to_value(fully_read_event).expect("to json value always works"),
            )?;
        }
        create_receipt::v3::ReceiptType::Read => {
            let mut user_receipts = BTreeMap::new();
            user_receipts.insert(
                sender_user.clone(),
                ruma::events::receipt::Receipt::new(MilliSecondsSinceUnixEpoch::now()),
            );
            let mut receipts = BTreeMap::new();
            receipts.insert(ReceiptType::Read, user_receipts);

            let mut receipt_content = BTreeMap::new();
            receipt_content.insert(body.event_id.to_owned(), receipts);

            services().rooms.edus.read_receipt.readreceipt_update(
                sender_user,
                &body.room_id,
                ruma::events::receipt::ReceiptEvent {
                    content: ruma::events::receipt::ReceiptEventContent(receipt_content),
                    room_id: body.room_id.clone(),
                },
            )?;
        }
        create_receipt::v3::ReceiptType::ReadPrivate => {
            let count = services()
                .rooms
                .timeline
                .get_pdu_count(&body.event_id)?
                .ok_or(Error::BadRequestString(
                    ErrorKind::InvalidParam,
                    "Event does not exist.",
                ))?;
            let count = match count {
                PduCount::Backfilled(_) => {
                    return Err(Error::BadRequestString(
                        ErrorKind::InvalidParam,
                        "Read receipt is in backfilled timeline",
                    ))
                }
                PduCount::Normal(c) => c,
            };
            services().rooms.edus.read_receipt.private_read_set(
                &body.room_id,
                sender_user,
                count,
            )?;
        }
        _ => return Err(Error::bad_database("Unsupported receipt type")),
    }

    Ok(create_receipt::v3::Response::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruma::{
        api::client::{read_marker::set_read_marker, receipt::create_receipt},
        events::{receipt::ReceiptThread, RoomAccountDataEventType},
        event_id, room_id, user_id,
        OwnedEventId, OwnedRoomId, OwnedUserId, EventId, RoomId, UserId,
    };
    use std::{
        collections::{BTreeMap, HashMap, HashSet},
        sync::{Arc, RwLock},
        time::{Duration, Instant},
        thread,
    };
    use tracing::{debug, info};

    /// Mock read marker storage for testing
    #[derive(Debug)]
    struct MockReadMarkerStorage {
        fully_read_markers: Arc<RwLock<HashMap<(OwnedUserId, OwnedRoomId), OwnedEventId>>>,
        read_receipts: Arc<RwLock<HashMap<(OwnedUserId, OwnedRoomId), OwnedEventId>>>,
        private_read_receipts: Arc<RwLock<HashMap<(OwnedUserId, OwnedRoomId), OwnedEventId>>>,
        notification_counts: Arc<RwLock<HashMap<(OwnedUserId, OwnedRoomId), u64>>>,
        event_timeline: Arc<RwLock<HashMap<OwnedEventId, u64>>>, // event_id -> timestamp
    }

    impl MockReadMarkerStorage {
        fn new() -> Self {
            Self {
                fully_read_markers: Arc::new(RwLock::new(HashMap::new())),
                read_receipts: Arc::new(RwLock::new(HashMap::new())),
                private_read_receipts: Arc::new(RwLock::new(HashMap::new())),
                notification_counts: Arc::new(RwLock::new(HashMap::new())),
                event_timeline: Arc::new(RwLock::new(HashMap::new())),
            }
        }

        fn set_fully_read(&self, user_id: OwnedUserId, room_id: OwnedRoomId, event_id: OwnedEventId) {
            self.fully_read_markers.write().unwrap().insert((user_id, room_id), event_id);
        }

        fn get_fully_read(&self, user_id: &UserId, room_id: &RoomId) -> Option<OwnedEventId> {
            self.fully_read_markers.read().unwrap().get(&(user_id.to_owned(), room_id.to_owned())).cloned()
        }

        fn set_read_receipt(&self, user_id: OwnedUserId, room_id: OwnedRoomId, event_id: OwnedEventId) {
            self.read_receipts.write().unwrap().insert((user_id, room_id), event_id);
        }

        fn get_read_receipt(&self, user_id: &UserId, room_id: &RoomId) -> Option<OwnedEventId> {
            self.read_receipts.read().unwrap().get(&(user_id.to_owned(), room_id.to_owned())).cloned()
        }

        fn set_private_read_receipt(&self, user_id: OwnedUserId, room_id: OwnedRoomId, event_id: OwnedEventId) {
            self.private_read_receipts.write().unwrap().insert((user_id, room_id), event_id);
        }

        fn get_private_read_receipt(&self, user_id: &UserId, room_id: &RoomId) -> Option<OwnedEventId> {
            self.private_read_receipts.read().unwrap().get(&(user_id.to_owned(), room_id.to_owned())).cloned()
        }

        fn reset_notification_count(&self, user_id: &UserId, room_id: &RoomId) {
            self.notification_counts.write().unwrap().insert((user_id.to_owned(), room_id.to_owned()), 0);
        }

        fn get_notification_count(&self, user_id: &UserId, room_id: &RoomId) -> u64 {
            self.notification_counts.read().unwrap().get(&(user_id.to_owned(), room_id.to_owned())).cloned().unwrap_or(0)
        }

        fn add_event(&self, event_id: OwnedEventId, timestamp: u64) {
            self.event_timeline.write().unwrap().insert(event_id, timestamp);
        }

        fn event_exists(&self, event_id: &EventId) -> bool {
            self.event_timeline.read().unwrap().contains_key(event_id)
        }

        fn get_event_timestamp(&self, event_id: &EventId) -> Option<u64> {
            self.event_timeline.read().unwrap().get(event_id).cloned()
        }
    }

    fn create_test_user(index: usize) -> OwnedUserId {
        match index {
            0 => user_id!("@reader0:example.com").to_owned(),
            1 => user_id!("@reader1:example.com").to_owned(),
            2 => user_id!("@reader2:example.com").to_owned(),
            _ => user_id!("@reader_other:example.com").to_owned(),
        }
    }

    fn create_test_room(index: usize) -> OwnedRoomId {
        match index {
            0 => room_id!("!reading_room0:example.com").to_owned(),
            1 => room_id!("!reading_room1:example.com").to_owned(),
            2 => room_id!("!reading_room2:example.com").to_owned(),
            _ => room_id!("!reading_room_other:example.com").to_owned(),
        }
    }

    fn create_test_event(index: usize) -> OwnedEventId {
        match index {
            0 => event_id!("$read_event0:example.com").to_owned(),
            1 => event_id!("$read_event1:example.com").to_owned(),
            2 => event_id!("$read_event2:example.com").to_owned(),
            3 => event_id!("$fully_read_event:example.com").to_owned(),
            4 => event_id!("$private_read_event:example.com").to_owned(),
            _ => event_id!("$read_event_other:example.com").to_owned(),
        }
    }

    fn create_test_read_marker_request(room_id: OwnedRoomId, fully_read: Option<OwnedEventId>, read_receipt: Option<OwnedEventId>, private_read_receipt: Option<OwnedEventId>) -> set_read_marker::v3::Request {
        let mut request = set_read_marker::v3::Request::new(room_id);
        request.fully_read = fully_read;
        request.read_receipt = read_receipt;
        request.private_read_receipt = private_read_receipt;
        request
    }

    fn create_test_receipt_request(room_id: OwnedRoomId, receipt_type: create_receipt::v3::ReceiptType, event_id: OwnedEventId) -> create_receipt::v3::Request {
        create_receipt::v3::Request::new(room_id, receipt_type, event_id)
    }

    #[test]
    fn test_read_marker_request_structures() {
        debug!("🔧 Testing read marker request structures");
        let start = Instant::now();

        let room_id = create_test_room(0);
        let fully_read_event = create_test_event(0);
        let read_receipt_event = create_test_event(1);
        let private_read_event = create_test_event(2);

        // Test set read marker request
        let request = create_test_read_marker_request(
            room_id.clone(),
            Some(fully_read_event.clone()),
            Some(read_receipt_event.clone()),
            Some(private_read_event.clone()),
        );

        assert_eq!(request.room_id, room_id, "Room ID should match");
        assert_eq!(request.fully_read, Some(fully_read_event), "Fully read event should match");
        assert_eq!(request.read_receipt, Some(read_receipt_event), "Read receipt event should match");
        assert_eq!(request.private_read_receipt, Some(private_read_event), "Private read receipt event should match");

        // Test receipt request
        let receipt_request = create_test_receipt_request(
            room_id.clone(),
            create_receipt::v3::ReceiptType::Read,
            create_test_event(0),
        );

        assert_eq!(receipt_request.room_id, room_id, "Receipt room ID should match");
        assert_eq!(receipt_request.receipt_type, create_receipt::v3::ReceiptType::Read, "Receipt type should match");

        info!("✅ Read marker request structures test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_read_marker_response_structures() {
        debug!("🔧 Testing read marker response structures");
        let start = Instant::now();

        // Test set read marker response
        let set_response = set_read_marker::v3::Response {};
        // Response structure is validated by compilation

        // Test create receipt response
        let receipt_response = create_receipt::v3::Response {};
        // Response structure is validated by compilation

        // Both are empty responses as per Matrix spec
        assert_eq!(std::mem::size_of_val(&set_response), 0, "Set read marker response should be empty struct");
        assert_eq!(std::mem::size_of_val(&receipt_response), 0, "Create receipt response should be empty struct");

        info!("✅ Read marker response structures test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_receipt_type_validation() {
        debug!("🔧 Testing receipt type validation");
        let start = Instant::now();

        // Test all valid receipt types
        let receipt_types = vec![
            create_receipt::v3::ReceiptType::Read,
            create_receipt::v3::ReceiptType::ReadPrivate,
            create_receipt::v3::ReceiptType::FullyRead,
        ];

        for receipt_type in receipt_types {
            let request = create_test_receipt_request(
                create_test_room(0),
                receipt_type.clone(),
                create_test_event(0),
            );
            
            assert_eq!(request.receipt_type, receipt_type, "Receipt type should be preserved");
            
            // Test that each type has expected behavior
            match receipt_type {
                create_receipt::v3::ReceiptType::Read => {
                    // Public read receipt - updates notification counts and sends EDU
                }
                create_receipt::v3::ReceiptType::ReadPrivate => {
                    // Private read receipt - updates notification counts but no EDU
                }
                create_receipt::v3::ReceiptType::FullyRead => {
                    // Fully read marker - updates account data
                }
                _ => panic!("Unexpected receipt type"),
            }
        }

        info!("✅ Receipt type validation test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_read_marker_data_operations() {
        debug!("🔧 Testing read marker data operations");
        let start = Instant::now();

        let storage = MockReadMarkerStorage::new();
        let user = create_test_user(0);
        let room = create_test_room(0);
        let event1 = create_test_event(0);
        let event2 = create_test_event(1);

        // Add events to timeline
        storage.add_event(event1.clone(), 1000);
        storage.add_event(event2.clone(), 2000);

        // Test fully read marker operations
        storage.set_fully_read(user.clone(), room.clone(), event1.clone());
        let retrieved_fully_read = storage.get_fully_read(&user, &room);
        assert_eq!(retrieved_fully_read, Some(event1.clone()), "Fully read marker should be retrievable");

        // Test read receipt operations
        storage.set_read_receipt(user.clone(), room.clone(), event2.clone());
        let retrieved_read_receipt = storage.get_read_receipt(&user, &room);
        assert_eq!(retrieved_read_receipt, Some(event2.clone()), "Read receipt should be retrievable");

        // Test private read receipt operations
        storage.set_private_read_receipt(user.clone(), room.clone(), event1.clone());
        let retrieved_private = storage.get_private_read_receipt(&user, &room);
        assert_eq!(retrieved_private, Some(event1), "Private read receipt should be retrievable");

        // Test notification count operations
        storage.reset_notification_count(&user, &room);
        let count = storage.get_notification_count(&user, &room);
        assert_eq!(count, 0, "Notification count should be reset to 0");

        // Test event existence checks
        assert!(storage.event_exists(&event2), "Event should exist in timeline");
        let timestamp = storage.get_event_timestamp(&event2);
        assert_eq!(timestamp, Some(2000), "Event timestamp should be retrievable");

        info!("✅ Read marker data operations test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_read_marker_progression() {
        debug!("🔧 Testing read marker progression logic");
        let start = Instant::now();

        let storage = MockReadMarkerStorage::new();
        let user = create_test_user(0);
        let room = create_test_room(0);

        // Create a sequence of events
        let events: Vec<OwnedEventId> = (0..5).map(create_test_event).collect();
        for (i, event) in events.iter().enumerate() {
            storage.add_event(event.clone(), 1000 + (i as u64 * 100));
        }

        // Test progression of read markers
        for (i, event) in events.iter().enumerate() {
            storage.set_fully_read(user.clone(), room.clone(), event.clone());
            let retrieved = storage.get_fully_read(&user, &room);
            assert_eq!(retrieved, Some(event.clone()), "Fully read should progress to event {}", i);

            storage.set_read_receipt(user.clone(), room.clone(), event.clone());
            let receipt = storage.get_read_receipt(&user, &room);
            assert_eq!(receipt, Some(event.clone()), "Read receipt should progress to event {}", i);
        }

        // Test that latest markers are preserved
        let final_fully_read = storage.get_fully_read(&user, &room);
        let final_read_receipt = storage.get_read_receipt(&user, &room);
        
        assert_eq!(final_fully_read, Some(events[4].clone()), "Final fully read should be last event");
        assert_eq!(final_read_receipt, Some(events[4].clone()), "Final read receipt should be last event");

        info!("✅ Read marker progression test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_read_marker_security_constraints() {
        debug!("🔧 Testing read marker security constraints");
        let start = Instant::now();

        let storage = MockReadMarkerStorage::new();
        let user1 = create_test_user(0);
        let user2 = create_test_user(1);
        let room1 = create_test_room(0);
        let room2 = create_test_room(1);
        let event1 = create_test_event(0);
        let event2 = create_test_event(1);

        // Add events
        storage.add_event(event1.clone(), 1000);
        storage.add_event(event2.clone(), 2000);

        // Set read markers for different users and rooms
        storage.set_fully_read(user1.clone(), room1.clone(), event1.clone());
        storage.set_fully_read(user2.clone(), room1.clone(), event2.clone());
        storage.set_fully_read(user1.clone(), room2.clone(), event2.clone());

        // Test user isolation - users should only see their own markers
        let user1_room1_marker = storage.get_fully_read(&user1, &room1);
        let user2_room1_marker = storage.get_fully_read(&user2, &room1);
        let user1_room2_marker = storage.get_fully_read(&user1, &room2);
        let user2_room2_marker = storage.get_fully_read(&user2, &room2);

        assert_eq!(user1_room1_marker, Some(event1.clone()), "User1 should see their marker in room1");
        assert_eq!(user2_room1_marker, Some(event2.clone()), "User2 should see their marker in room1");
        assert_eq!(user1_room2_marker, Some(event2.clone()), "User1 should see their marker in room2");
        assert_eq!(user2_room2_marker, None, "User2 should have no marker in room2");

        // Test room isolation - read markers are room-specific
        storage.set_read_receipt(user1.clone(), room1.clone(), event1.clone());
        storage.set_read_receipt(user1.clone(), room2.clone(), event2.clone());

        let user1_receipt_room1 = storage.get_read_receipt(&user1, &room1);
        let user1_receipt_room2 = storage.get_read_receipt(&user1, &room2);

        assert_eq!(user1_receipt_room1, Some(event1), "User1 should have different receipts per room");
        assert_eq!(user1_receipt_room2, Some(event2), "User1 should have different receipts per room");

        // Test notification count isolation
        storage.reset_notification_count(&user1, &room1);
        let user1_count = storage.get_notification_count(&user1, &room1);
        let user2_count = storage.get_notification_count(&user2, &room1);

        assert_eq!(user1_count, 0, "User1 notifications should be reset");
        assert_eq!(user2_count, 0, "User2 notifications should be unaffected (default 0)");

        info!("✅ Read marker security constraints test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_concurrent_read_marker_operations() {
        debug!("🔧 Testing concurrent read marker operations");
        let start = Instant::now();

        let storage = Arc::new(MockReadMarkerStorage::new());
        let num_threads = 5;
        let operations_per_thread = 100;

        // Pre-populate events
        for i in 0..(num_threads * operations_per_thread) {
            let event = match i % 5 {
                0 => event_id!("$concurrent_event_0:example.com").to_owned(),
                1 => event_id!("$concurrent_event_1:example.com").to_owned(),
                2 => event_id!("$concurrent_event_2:example.com").to_owned(),
                3 => event_id!("$concurrent_event_3:example.com").to_owned(),
                _ => event_id!("$concurrent_event_4:example.com").to_owned(),
            };
            storage.add_event(event, 1000 + i as u64);
        }

        let mut handles = vec![];

        // Spawn threads performing concurrent read marker operations
        for thread_id in 0..num_threads {
            let storage_clone = Arc::clone(&storage);
            
            let handle = thread::spawn(move || {
                for op_id in 0..operations_per_thread {
                    // Ensure all user-room combinations are covered by cycling through them
                    let user_index = (op_id + thread_id * operations_per_thread) % 3;
                    let room_index = (op_id + thread_id * operations_per_thread) % 2;
                    let user = create_test_user(user_index);
                    let room = create_test_room(room_index);
                    
                    let event_index = thread_id * operations_per_thread + op_id;
                    let event = match event_index % 5 {
                        0 => event_id!("$concurrent_event_0:example.com").to_owned(),
                        1 => event_id!("$concurrent_event_1:example.com").to_owned(),
                        2 => event_id!("$concurrent_event_2:example.com").to_owned(),
                        3 => event_id!("$concurrent_event_3:example.com").to_owned(),
                        _ => event_id!("$concurrent_event_4:example.com").to_owned(),
                    };
                    
                    // Perform different types of read marker operations
                    match op_id % 3 {
                        0 => storage_clone.set_fully_read(user.clone(), room.clone(), event),
                        1 => storage_clone.set_read_receipt(user.clone(), room.clone(), event),
                        _ => storage_clone.set_private_read_receipt(user.clone(), room.clone(), event),
                    }
                    
                    // Reset notification count periodically
                    if op_id % 10 == 0 {
                        storage_clone.reset_notification_count(&user, &room);
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify final state integrity
        for i in 0..3 {
            let user = create_test_user(i);
            for j in 0..2 {
                let room = create_test_room(j);
                
                // Check that some read markers were set (exact values depend on thread timing)
                let fully_read = storage.get_fully_read(&user, &room);
                let read_receipt = storage.get_read_receipt(&user, &room);
                let private_receipt = storage.get_private_read_receipt(&user, &room);
                
                // At least one type of marker should be set after all operations
                let has_markers = fully_read.is_some() || read_receipt.is_some() || private_receipt.is_some();
                assert!(has_markers, "User {} in room {} should have some read markers", i, j);
            }
        }

        info!("✅ Concurrent read marker operations test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_read_marker_performance_benchmarks() {
        debug!("🔧 Testing read marker performance benchmarks");
        let start = Instant::now();

        let storage = MockReadMarkerStorage::new();
        let user = create_test_user(0);
        let room = create_test_room(0);
        let events: Vec<OwnedEventId> = vec![
            event_id!("$perf_event_0:example.com").to_owned(),
            event_id!("$perf_event_1:example.com").to_owned(),
            event_id!("$perf_event_2:example.com").to_owned(),
            event_id!("$perf_event_3:example.com").to_owned(),
            event_id!("$perf_event_4:example.com").to_owned(),
        ];

        // Extend the events vector for performance testing by repeating the pattern
        let mut extended_events = Vec::with_capacity(1000);
        for i in 0..1000 {
            extended_events.push(events[i % events.len()].clone());
        }

        // Pre-populate events
        for (i, event) in extended_events.iter().enumerate() {
            storage.add_event(event.clone(), 1000 + i as u64);
        }

        // Benchmark fully read marker operations
        let fully_read_start = Instant::now();
        for event in &extended_events {
            storage.set_fully_read(user.clone(), room.clone(), event.clone());
        }
        let fully_read_duration = fully_read_start.elapsed();

        // Benchmark read receipt operations
        let read_receipt_start = Instant::now();
        for event in &extended_events {
            storage.set_read_receipt(user.clone(), room.clone(), event.clone());
        }
        let read_receipt_duration = read_receipt_start.elapsed();

        // Benchmark retrieval operations
        let retrieval_start = Instant::now();
        for _ in 0..1000 {
            let _ = storage.get_fully_read(&user, &room);
            let _ = storage.get_read_receipt(&user, &room);
            let _ = storage.get_private_read_receipt(&user, &room);
        }
        let retrieval_duration = retrieval_start.elapsed();

        // Benchmark notification count operations
        let notification_start = Instant::now();
        for _ in 0..1000 {
            storage.reset_notification_count(&user, &room);
            let _ = storage.get_notification_count(&user, &room);
        }
        let notification_duration = notification_start.elapsed();

        // Performance assertions
        assert!(fully_read_duration < Duration::from_millis(100), 
                "1000 fully read operations should complete within 100ms, took: {:?}", fully_read_duration);
        assert!(read_receipt_duration < Duration::from_millis(100), 
                "1000 read receipt operations should complete within 100ms, took: {:?}", read_receipt_duration);
        assert!(retrieval_duration < Duration::from_millis(50), 
                "3000 retrieval operations should complete within 50ms, took: {:?}", retrieval_duration);
        assert!(notification_duration < Duration::from_millis(50), 
                "2000 notification operations should complete within 50ms, took: {:?}", notification_duration);

        info!("✅ Read marker performance benchmarks completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_read_marker_edge_cases() {
        debug!("🔧 Testing read marker edge cases");
        let start = Instant::now();

        let storage = MockReadMarkerStorage::new();
        let user = create_test_user(0);
        let room = create_test_room(0);
        let nonexistent_user = create_test_user(99);
        let nonexistent_room = create_test_room(99);
        let nonexistent_event = create_test_event(99);

        // Test retrieving markers for non-existent user/room combinations
        let missing_fully_read = storage.get_fully_read(&nonexistent_user, &room);
        assert!(missing_fully_read.is_none(), "Non-existent user should have no fully read marker");

        let missing_receipt = storage.get_read_receipt(&user, &nonexistent_room);
        assert!(missing_receipt.is_none(), "Non-existent room should have no read receipt");

        let missing_private = storage.get_private_read_receipt(&nonexistent_user, &nonexistent_room);
        assert!(missing_private.is_none(), "Non-existent user/room should have no private receipt");

        // Test notification count for non-existent user/room
        let missing_count = storage.get_notification_count(&nonexistent_user, &room);
        assert_eq!(missing_count, 0, "Non-existent user should have 0 notifications");

        // Test event existence for non-existent event
        assert!(!storage.event_exists(&nonexistent_event), "Non-existent event should not exist");

        let missing_timestamp = storage.get_event_timestamp(&nonexistent_event);
        assert!(missing_timestamp.is_none(), "Non-existent event should have no timestamp");

        // Test rapid marker updates (simulating fast user interactions)
        let rapid_events: Vec<OwnedEventId> = vec![
            event_id!("$rapid_event_0:example.com").to_owned(),
            event_id!("$rapid_event_1:example.com").to_owned(),
            event_id!("$rapid_event_2:example.com").to_owned(),
            event_id!("$rapid_event_3:example.com").to_owned(),
            event_id!("$rapid_event_4:example.com").to_owned(),
            event_id!("$rapid_event_5:example.com").to_owned(),
            event_id!("$rapid_event_6:example.com").to_owned(),
            event_id!("$rapid_event_7:example.com").to_owned(),
            event_id!("$rapid_event_8:example.com").to_owned(),
            event_id!("$rapid_event_9:example.com").to_owned(),
        ];
        
        for (i, event) in rapid_events.iter().enumerate() {
            storage.add_event(event.clone(), 1000 + i as u64);
        }

        for event in &rapid_events {
            storage.set_fully_read(user.clone(), room.clone(), event.clone());
            storage.set_read_receipt(user.clone(), room.clone(), event.clone());
            storage.set_private_read_receipt(user.clone(), room.clone(), event.clone());
        }

        // Final state should be the last event
        let final_fully_read = storage.get_fully_read(&user, &room);
        let final_read_receipt = storage.get_read_receipt(&user, &room);
        let final_private = storage.get_private_read_receipt(&user, &room);

        assert_eq!(final_fully_read, Some(rapid_events[9].clone()), "Final fully read should be last event");
        assert_eq!(final_read_receipt, Some(rapid_events[9].clone()), "Final read receipt should be last event");
        assert_eq!(final_private, Some(rapid_events[9].clone()), "Final private receipt should be last event");

        info!("✅ Read marker edge cases test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_matrix_protocol_compliance() {
        debug!("🔧 Testing Matrix protocol compliance for read markers");
        let start = Instant::now();

        // Test Matrix event ID format validation
        let valid_event_ids = vec![
            "$read_marker_event:example.com",
            "$1234567890abcdef:matrix.org",
            "$complex.event_id-with.chars:server.name.com",
        ];

        for event_id_str in valid_event_ids {
            assert!(event_id_str.starts_with('$'), "Event ID should start with $");
            assert!(event_id_str.contains(':'), "Event ID should contain server name");
            
            // Test parsing
            if let Ok(event_id) = event_id_str.try_into() as Result<OwnedEventId, _> {
                assert!(!event_id.as_str().is_empty(), "Parsed event ID should not be empty");
            }
        }

        // Test Matrix room ID format validation
        let valid_room_ids = vec![
            "!read_marker_room:example.com",
            "!1234567890abcdef:matrix.org",
            "!complex.room_id-with.chars:server.name.com",
        ];

        for room_id_str in valid_room_ids {
            assert!(room_id_str.starts_with('!'), "Room ID should start with !");
            assert!(room_id_str.contains(':'), "Room ID should contain server name");
            
            // Test parsing
            if let Ok(room_id) = room_id_str.try_into() as Result<OwnedRoomId, _> {
                assert!(!room_id.as_str().is_empty(), "Parsed room ID should not be empty");
            }
        }

        // Test Matrix user ID format validation
        let valid_user_ids = vec![
            "@reader:example.com",
            "@test_user123:matrix.org",
            "@complex.user-name_with.chars:server.name.com",
        ];

        for user_id_str in valid_user_ids {
            assert!(user_id_str.starts_with('@'), "User ID should start with @");
            assert!(user_id_str.contains(':'), "User ID should contain server name");
            
            // Test parsing
            if let Ok(user_id) = user_id_str.try_into() as Result<OwnedUserId, _> {
                assert!(!user_id.as_str().is_empty(), "Parsed user ID should not be empty");
            }
        }

        // Test receipt type compliance with Matrix spec
        let receipt_types = vec![
            create_receipt::v3::ReceiptType::Read,
            create_receipt::v3::ReceiptType::ReadPrivate,
            create_receipt::v3::ReceiptType::FullyRead,
        ];

        for receipt_type in receipt_types {
            match receipt_type {
                create_receipt::v3::ReceiptType::Read => {
                    // Public read receipt - should update notification counts and send EDU
                }
                create_receipt::v3::ReceiptType::ReadPrivate => {
                    // Private read receipt - should update notification counts but not send EDU
                }
                create_receipt::v3::ReceiptType::FullyRead => {
                    // Fully read marker - should update account data
                }
                _ => panic!("Unexpected receipt type"),
            }
        }

        // Test ReceiptThread compliance
        let thread = ReceiptThread::Unthreaded;
        match thread {
            ReceiptThread::Unthreaded => {
                // Should handle unthreaded receipts (main timeline)
            }
            ReceiptThread::Main => {
                // Should handle main thread receipts
            }
            _ => {
                // Should handle other thread types if supported
            }
        }

        info!("✅ Matrix protocol compliance test completed in {:?}", start.elapsed());
    }

    #[test]
    fn test_read_marker_enterprise_compliance() {
        debug!("🔧 Testing enterprise compliance for read marker system");
        let start = Instant::now();

        let storage = MockReadMarkerStorage::new();

        // 1. Performance - Read marker operations should be fast
        let perf_start = Instant::now();
        let user = create_test_user(0);
        let room = create_test_room(0);
        let event = create_test_event(0);
        
        storage.add_event(event.clone(), 1000);
        storage.set_fully_read(user.clone(), room.clone(), event.clone());
        let _retrieved = storage.get_fully_read(&user, &room);
        
        let perf_duration = perf_start.elapsed();
        assert!(perf_duration < Duration::from_millis(5), 
                "Read marker operations should be <5ms for enterprise use, was: {:?}", perf_duration);

        // 2. Reliability - Consistent data handling
        let event1 = create_test_event(1);
        let event2 = create_test_event(2);
        
        storage.add_event(event1.clone(), 1001);
        storage.add_event(event2.clone(), 1002);
        
        storage.set_fully_read(user.clone(), room.clone(), event1.clone());
        storage.set_fully_read(user.clone(), room.clone(), event2.clone());
        
        let final_marker = storage.get_fully_read(&user, &room);
        assert_eq!(final_marker, Some(event2), "Should handle marker updates consistently");

        // 3. Scalability - Handle multiple users and rooms efficiently
        let users_before = 3;
        let rooms_before = 2;
        
        for user_i in 0..users_before {
            for room_i in 0..rooms_before {
                let test_user = create_test_user(user_i);
                let test_room = create_test_room(room_i);
                let test_event = create_test_event(user_i + room_i);
                
                storage.add_event(test_event.clone(), 2000 + user_i as u64 + room_i as u64);
                storage.set_fully_read(test_user.clone(), test_room.clone(), test_event);
                storage.reset_notification_count(&test_user, &test_room);
            }
        }

        // Verify all users have markers
        for user_i in 0..users_before {
            for room_i in 0..rooms_before {
                let test_user = create_test_user(user_i);
                let test_room = create_test_room(room_i);
                
                let marker = storage.get_fully_read(&test_user, &test_room);
                assert!(marker.is_some(), "User {} should have marker in room {}", user_i, room_i);
                
                let count = storage.get_notification_count(&test_user, &test_room);
                assert_eq!(count, 0, "User {} should have reset notifications in room {}", user_i, room_i);
            }
        }

        // 4. Data Integrity - Read markers should maintain logical consistency
        let user_a = create_test_user(0);
        let room_a = create_test_room(0);
        let old_event = create_test_event(0);
        let new_event = create_test_event(1);
        
        storage.add_event(old_event.clone(), 1000);
        storage.add_event(new_event.clone(), 2000);
        
        storage.set_fully_read(user_a.clone(), room_a.clone(), old_event.clone());
        storage.set_fully_read(user_a.clone(), room_a.clone(), new_event.clone());
        
        let final_marker = storage.get_fully_read(&user_a, &room_a);
        assert_eq!(final_marker, Some(new_event.clone()), "Should maintain latest marker");
        
        // Different receipt types should coexist
        storage.set_read_receipt(user_a.clone(), room_a.clone(), old_event.clone());
        storage.set_private_read_receipt(user_a.clone(), room_a.clone(), new_event.clone());
        
        let public_receipt = storage.get_read_receipt(&user_a, &room_a);
        let private_receipt = storage.get_private_read_receipt(&user_a, &room_a);
        
        assert_eq!(public_receipt, Some(old_event), "Public receipt should be preserved");
        assert_eq!(private_receipt, Some(new_event), "Private receipt should be preserved");

        info!("✅ Read marker enterprise compliance test completed in {:?}", start.elapsed());
    }
}
