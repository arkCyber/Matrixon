#!/usr/bin/env python3
"""
Matrixon Matrix Server Test Suite
Enhanced test script to validate core Matrix functionality
"""

import requests
import json
import time
import sys
from urllib.parse import urljoin

class MatrixTester:
    def __init__(self, base_url="http://localhost:8008"):
        self.base_url = base_url
        self.access_token = None
        self.user_id = None
        self.device_id = None
        
    def test_server_health(self):
        """Test if the Matrix server is responding"""
        print("🔍 Testing server health...")
        try:
            response = requests.get(f"{self.base_url}/_matrix/client/versions", timeout=30)
            if response.status_code == 200:
                versions = response.json()
                print(f"✅ Server is running! Supported versions: {versions.get('versions', [])}")
                return True
            else:
                print(f"❌ Server returned status {response.status_code}")
                return False
        except requests.exceptions.RequestException as e:
            print(f"❌ Server health check failed: {e}")
            return False
    
    def test_federation_status(self):
        """Test federation capabilities"""
        print("🔍 Testing federation status...")
        try:
            response = requests.get(f"{self.base_url}/_matrix/federation/v1/version", timeout=30)
            if response.status_code == 200:
                data = response.json()
                print(f"✅ Federation is working! Server: {data.get('server', {})}")
                return True
            else:
                print(f"⚠️ Federation endpoint returned status {response.status_code}")
                return False
        except requests.exceptions.RequestException as e:
            print(f"⚠️ Federation test failed: {e}")
            return False
    
    def test_user_registration(self):
        """Test user registration"""
        print("🔍 Testing user registration...")
        try:
            username = f"testuser_{int(time.time())}"
            password = "testpassword123"
            
            register_data = {
                "username": username,
                "password": password,
                "device_id": "test_device",
                "initial_device_display_name": "Test Device"
            }
            
            response = requests.post(
                f"{self.base_url}/_matrix/client/r0/register",
                json=register_data,
                timeout=10
            )
            
            if response.status_code == 200:
                data = response.json()
                self.access_token = data.get("access_token")
                self.user_id = data.get("user_id")
                self.device_id = data.get("device_id")
                print(f"✅ User registration successful! User ID: {self.user_id}")
                return True
            elif response.status_code == 401:
                print("⚠️ Registration requires authentication (normal for some servers)")
                return False
            else:
                error_data = response.json() if response.headers.get('content-type', '').startswith('application/json') else {}
                print(f"❌ Registration failed with status {response.status_code}: {error_data}")
                return False
                
        except requests.exceptions.RequestException as e:
            print(f"❌ Registration test failed: {e}")
            return False
    
    def test_login(self):
        """Test user login"""
        print("🔍 Testing user login...")
        if not self.user_id:
            print("⚠️ Skipping login test (no registered user)")
            return False
            
        try:
            login_data = {
                "type": "m.login.password",
                "user": self.user_id,
                "password": "testpassword123",
                "device_id": "test_device"
            }
            
            response = requests.post(
                f"{self.base_url}/_matrix/client/r0/login",
                json=login_data,
                timeout=10
            )
            
            if response.status_code == 200:
                data = response.json()
                self.access_token = data.get("access_token")
                print(f"✅ Login successful! Access token obtained.")
                return True
            else:
                error_data = response.json() if response.headers.get('content-type', '').startswith('application/json') else {}
                print(f"❌ Login failed with status {response.status_code}: {error_data}")
                return False
                
        except requests.exceptions.RequestException as e:
            print(f"❌ Login test failed: {e}")
            return False
    
    def test_room_creation(self):
        """Test room creation"""
        print("🔍 Testing room creation...")
        if not self.access_token:
            print("⚠️ Skipping room creation test (no access token)")
            return False
            
        try:
            room_data = {
                "name": "Test Room",
                "topic": "A room for testing Matrixon functionality",
                "preset": "private_chat"
            }
            
            headers = {"Authorization": f"Bearer {self.access_token}"}
            response = requests.post(
                f"{self.base_url}/_matrix/client/r0/createRoom",
                json=room_data,
                headers=headers,
                timeout=10
            )
            
            if response.status_code == 200:
                data = response.json()
                room_id = data.get("room_id")
                print(f"✅ Room creation successful! Room ID: {room_id}")
                return room_id
            else:
                error_data = response.json() if response.headers.get('content-type', '').startswith('application/json') else {}
                print(f"❌ Room creation failed with status {response.status_code}: {error_data}")
                return False
                
        except requests.exceptions.RequestException as e:
            print(f"❌ Room creation test failed: {e}")
            return False
    
    def test_send_message(self, room_id):
        """Test sending a message to a room"""
        print("🔍 Testing message sending...")
        if not self.access_token or not room_id:
            print("⚠️ Skipping message test (no access token or room)")
            return False
            
        try:
            message_data = {
                "msgtype": "m.text",
                "body": "Hello from Matrixon test suite! 🚀"
            }
            
            headers = {"Authorization": f"Bearer {self.access_token}"}
            response = requests.put(
                f"{self.base_url}/_matrix/client/r0/rooms/{room_id}/send/m.room.message/{int(time.time())}",
                json=message_data,
                headers=headers,
                timeout=10
            )
            
            if response.status_code == 200:
                data = response.json()
                event_id = data.get("event_id")
                print(f"✅ Message sent successfully! Event ID: {event_id}")
                return True
            else:
                error_data = response.json() if response.headers.get('content-type', '').startswith('application/json') else {}
                print(f"❌ Message sending failed with status {response.status_code}: {error_data}")
                return False
                
        except requests.exceptions.RequestException as e:
            print(f"❌ Message sending test failed: {e}")
            return False
    
    def test_sync(self):
        """Test sync functionality"""
        print("🔍 Testing sync functionality...")
        if not self.access_token:
            print("⚠️ Skipping sync test (no access token)")
            return False
            
        try:
            headers = {"Authorization": f"Bearer {self.access_token}"}
            response = requests.get(
                f"{self.base_url}/_matrix/client/r0/sync?timeout=1000",
                headers=headers,
                timeout=15
            )
            
            if response.status_code == 200:
                data = response.json()
                next_batch = data.get("next_batch")
                rooms = data.get("rooms", {})
                print(f"✅ Sync successful! Next batch: {next_batch[:20]}...")
                print(f"   Rooms in sync: {len(rooms.get('join', {}))}")
                return True
            else:
                error_data = response.json() if response.headers.get('content-type', '').startswith('application/json') else {}
                print(f"❌ Sync failed with status {response.status_code}: {error_data}")
                return False
                
        except requests.exceptions.RequestException as e:
            print(f"❌ Sync test failed: {e}")
            return False
    
    def run_all_tests(self):
        """Run all Matrix functionality tests"""
        print("🎯 Starting Matrixon Matrix Server Test Suite")
        print("=" * 50)
        
        results = {}
        
        # Core functionality tests
        results['server_health'] = self.test_server_health()
        results['federation'] = self.test_federation_status()
        results['registration'] = self.test_user_registration()
        results['login'] = self.test_login()
        
        # Advanced functionality tests
        room_id = self.test_room_creation()
        results['room_creation'] = bool(room_id)
        results['message_sending'] = self.test_send_message(room_id) if room_id else False
        results['sync'] = self.test_sync()
        
        # Results summary
        print("\n" + "=" * 50)
        print("📊 Test Results Summary:")
        print("=" * 50)
        
        passed = sum(1 for result in results.values() if result)
        total = len(results)
        
        for test_name, result in results.items():
            status = "✅ PASS" if result else "❌ FAIL"
            print(f"{test_name.replace('_', ' ').title():<20} {status}")
        
        print(f"\nOverall Result: {passed}/{total} tests passed")
        
        if passed >= 4:  # At least server health + 3 other tests
            print("🎉 Matrixon Matrix Server is functioning well!")
            return True
        else:
            print("⚠️ Some Matrix functionality issues detected")
            return False

def main():
    """Main function to run Matrix tests"""
    import argparse
    
    parser = argparse.ArgumentParser(description="Test Matrixon Matrix Server functionality")
    parser.add_argument("--url", default="http://localhost:8008", 
                       help="Matrix server URL (default: http://localhost:8008)")
    parser.add_argument("--wait", type=int, default=5,
                       help="Seconds to wait before starting tests (default: 5)")
    
    args = parser.parse_args()
    
    print(f"⏰ Waiting {args.wait} seconds for server to start...")
    time.sleep(args.wait)
    
    tester = MatrixTester(args.url)
    success = tester.run_all_tests()
    
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main() 
