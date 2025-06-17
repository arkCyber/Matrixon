#!/bin/bash

# Quick test script for Matrixon room communication features
echo "🚀 Quick Matrixon Room Communication Test"
echo "==========================================="

# Check server health
echo "📡 Checking server health..."
curl -s http://localhost:6167/health | jq .

# Register Alice
echo "👤 Registering Alice..."
ALICE_RESPONSE=$(curl -s -X POST http://localhost:6167/_matrix/client/r0/register \
  -H "Content-Type: application/json" \
  -d '{"username": "alice_quick", "password": "password123"}')

echo "Alice registered: $ALICE_RESPONSE"
ALICE_TOKEN=$(echo "$ALICE_RESPONSE" | jq -r '.access_token')
echo "Alice token: $ALICE_TOKEN"

# Create room
echo "🏠 Creating room..."
ROOM_RESPONSE=$(curl -s -X POST http://localhost:6167/_matrix/client/r0/createRoom \
  -H "Authorization: Bearer $ALICE_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Quick Test Room", "topic": "Testing Matrixon"}')

echo "Room created: $ROOM_RESPONSE"
ROOM_ID=$(echo "$ROOM_RESPONSE" | jq -r '.room_id')
echo "Room ID: $ROOM_ID"

# Send message
echo "💬 Sending message..."
TXN_ID="quick_test_$(date +%s)"
MSG_RESPONSE=$(curl -s -X PUT "http://localhost:6167/_matrix/client/r0/rooms/$ROOM_ID/send/m.room.message/$TXN_ID" \
  -H "Authorization: Bearer $ALICE_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"msgtype": "m.text", "body": "Hello from Matrixon! 🎉"}')

echo "Message sent: $MSG_RESPONSE"

# Get messages
echo "📖 Getting messages..."
MESSAGES_RESPONSE=$(curl -s "http://localhost:6167/_matrix/client/r0/rooms/$ROOM_ID/messages?limit=5" \
  -H "Authorization: Bearer $ALICE_TOKEN")

echo "Messages: $MESSAGES_RESPONSE"

echo "✅ Quick test completed!" 
