#!/usr/bin/env python3
"""
🧪 Simple TCP client test for MerkleKV server
Tests the basic SET, GET, DELETE operations.
"""
import socket
import time

def test_server():
    print("🚀 Starting MerkleKV TCP Client Test...")
    
    try:
        # Connect to server
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect(('127.0.0.1', 7878))
        print("✅ Connected to server at 127.0.0.1:7878")
        
        def send_command(cmd):
            print(f"📤 Sending: {cmd}")
            sock.send((cmd + '\r\n').encode('utf-8'))
            response = sock.recv(1024).decode('utf-8').strip()
            print(f"📥 Response: {response}")
            return response
        
        # Test SET command
        print("\n🧪 Testing SET command...")
        response = send_command("SET user:123 Alice Johnson")
        assert "OK" in response, f"Expected OK, got: {response}"
        
        # Test GET command
        print("\n🧪 Testing GET command...")
        response = send_command("GET user:123")
        assert "Alice Johnson" in response, f"Expected 'Alice Johnson', got: {response}"
        
        # Test DELETE command
        print("\n🧪 Testing DELETE command...")
        response = send_command("DELETE user:123")
        assert "OK" in response, f"Expected OK, got: {response}"
        
        # Test GET after DELETE
        print("\n🧪 Testing GET after DELETE...")
        response = send_command("GET user:123")
        assert "NOT_FOUND" in response, f"Expected NOT_FOUND, got: {response}"
        
        # Test Unicode support
        print("\n🧪 Testing Unicode support...")
        response = send_command("SET 用户:123 こんにちは世界")
        assert "OK" in response, f"Expected OK, got: {response}"
        
        response = send_command("GET 用户:123")
        assert "こんにちは世界" in response, f"Expected Unicode value, got: {response}"
        
        # Test error conditions
        print("\n🧪 Testing error conditions...")
        response = send_command("INVALID_COMMAND")
        assert "ERROR" in response, f"Expected ERROR, got: {response}"
        
        response = send_command("GET")
        assert "ERROR" in response, f"Expected ERROR, got: {response}"
        
        print("\n🎉 All tests passed successfully!")
        
    except Exception as e:
        print(f"❌ Test failed: {e}")
        return False
    finally:
        sock.close()
        print("👋 Connection closed")
    
    return True

if __name__ == "__main__":
    test_server()
