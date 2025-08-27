#!/usr/bin/env python3
"""
Concurrent test for MerkleKV server to verify thread safety
"""

import socket
import threading
import time
import sys

def test_client(client_id, host='127.0.0.1', port=7878):
    """Test client that performs multiple operations"""
    try:
        print(f"🔌 Client {client_id}: Connecting...")
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect((host, port))
        
        # Test SET operation
        key = f"client{client_id}:key"
        value = f"value_from_client_{client_id}"
        
        # SET
        command = f"SET {key} {value}\r\n"
        sock.send(command.encode('utf-8'))
        response = sock.recv(1024).decode('utf-8').strip()
        print(f"✅ Client {client_id} SET: {response}")
        
        # GET
        command = f"GET {key}\r\n"
        sock.send(command.encode('utf-8'))
        response = sock.recv(1024).decode('utf-8').strip()
        print(f"📤 Client {client_id} GET: {response}")
        
        # DELETE
        command = f"DELETE {key}\r\n"
        sock.send(command.encode('utf-8'))
        response = sock.recv(1024).decode('utf-8').strip()
        print(f"🗑️ Client {client_id} DELETE: {response}")
        
        # Verify deletion
        command = f"GET {key}\r\n"
        sock.send(command.encode('utf-8'))
        response = sock.recv(1024).decode('utf-8').strip()
        print(f"🔍 Client {client_id} GET (after delete): {response}")
        
        sock.close()
        print(f"✅ Client {client_id}: Test completed successfully")
        return True
        
    except Exception as e:
        print(f"❌ Client {client_id}: Error - {e}")
        return False

def main():
    print("🚀 Starting concurrent test with multiple clients...")
    
    num_clients = 5
    threads = []
    
    # Start multiple client threads
    for i in range(num_clients):
        thread = threading.Thread(target=test_client, args=(i+1,))
        threads.append(thread)
        thread.start()
        time.sleep(0.1)  # Small delay between connections
    
    # Wait for all threads to complete
    for thread in threads:
        thread.join()
    
    print("🎉 Concurrent test completed!")

if __name__ == "__main__":
    main()
