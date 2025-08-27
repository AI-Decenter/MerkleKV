#!/usr/bin/env python3
"""
Edge case and performance test for MerkleKV
"""

import socket
import time

def test_edge_cases():
    print("🧪 Testing edge cases...")
    
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(('127.0.0.1', 7878))
    
    # Test empty values
    print("📝 Testing empty values...")
    sock.send(b"SET empty_key \r\n")
    response = sock.recv(1024).decode('utf-8').strip()
    print(f"  Empty value SET: {response}")
    
    sock.send(b"GET empty_key\r\n")
    response = sock.recv(1024).decode('utf-8').strip()
    print(f"  Empty value GET: {response}")
    
    # Test very long keys/values
    print("📏 Testing long keys and values...")
    long_key = "k" * 100
    long_value = "v" * 500
    
    command = f"SET {long_key} {long_value}\r\n"
    sock.send(command.encode('utf-8'))
    response = sock.recv(1024).decode('utf-8').strip()
    print(f"  Long key/value SET: {response}")
    
    command = f"GET {long_key}\r\n"
    sock.send(command.encode('utf-8'))
    response = sock.recv(2048).decode('utf-8').strip()
    print(f"  Long key/value GET: {response[:50]}...")
    
    # Test special characters in keys
    print("🔤 Testing special characters...")
    special_key = "key:with@special#chars!&*"
    special_value = "value with spaces, symbols: @#$%^&*()"
    
    command = f'SET {special_key} {special_value}\r\n'
    sock.send(command.encode('utf-8'))
    response = sock.recv(1024).decode('utf-8').strip()
    print(f"  Special chars SET: {response}")
    
    command = f'GET {special_key}\r\n'
    sock.send(command.encode('utf-8'))
    response = sock.recv(1024).decode('utf-8').strip()
    print(f"  Special chars GET: {response}")
    
    sock.close()
    print("✅ Edge cases test completed!")

def performance_test():
    print("\n⚡ Running performance test...")
    
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect(('127.0.0.1', 7878))
    
    num_operations = 100
    start_time = time.time()
    
    # Bulk SET operations
    for i in range(num_operations):
        command = f"SET perf_key_{i} performance_value_{i}\r\n"
        sock.send(command.encode('utf-8'))
        response = sock.recv(1024)
    
    set_time = time.time()
    print(f"📊 {num_operations} SET operations: {set_time - start_time:.3f}s")
    
    # Bulk GET operations
    for i in range(num_operations):
        command = f"GET perf_key_{i}\r\n"
        sock.send(command.encode('utf-8'))
        response = sock.recv(1024)
    
    get_time = time.time()
    print(f"📊 {num_operations} GET operations: {get_time - set_time:.3f}s")
    
    # Bulk DELETE operations
    for i in range(num_operations):
        command = f"DELETE perf_key_{i}\r\n"
        sock.send(command.encode('utf-8'))
        response = sock.recv(1024)
    
    delete_time = time.time()
    print(f"📊 {num_operations} DELETE operations: {delete_time - get_time:.3f}s")
    
    total_time = delete_time - start_time
    total_ops = num_operations * 3
    ops_per_sec = total_ops / total_time
    
    print(f"⚡ Total: {total_ops} operations in {total_time:.3f}s")
    print(f"🚀 Performance: {ops_per_sec:.0f} operations/second")
    
    sock.close()

def main():
    print("🔬 Starting comprehensive test suite...")
    test_edge_cases()
    performance_test()
    print("\n🎉 All comprehensive tests completed!")

if __name__ == "__main__":
    main()
