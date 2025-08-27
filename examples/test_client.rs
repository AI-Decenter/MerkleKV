//! # 🧪 MerkleKV Test Client
//!
//! This is an educational example client that demonstrates how to interact
//! with the MerkleKV server using TCP sockets and the text protocol.
//!
//! ## 🎯 Learning Objectives
//! - 📡 TCP client socket programming with Tokio
//! - 🔄 Async I/O patterns for network communication  
//! - 📝 Protocol implementation and message formatting
//! - 🛡️ Error handling in networked applications
//!
//! ## 🚀 Usage
//! ```bash
//! # Start the server first
//! cargo run
//! 
//! # Then run this test client
//! cargo run --example test_client
//! ```

use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// 🎯 Main client demonstration function
/// 
/// This function showcases:
/// - 🔌 **Connection Management**: Establishing TCP connections
/// - 📤 **Request Sending**: Formatting and sending commands  
/// - 📥 **Response Reading**: Parsing server responses
/// - 🔄 **Interactive Session**: Multiple commands in sequence
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 MerkleKV Test Client Starting...");
    
    // 🔗 Connect to the server
    let server_addr = "127.0.0.1:7878";
    println!("🔌 Connecting to server at {}", server_addr);
    
    let mut socket = TcpStream::connect(server_addr).await?;
    println!("✅ Connected successfully!");
    
    // 🧪 Run a series of test commands
    test_basic_operations(&mut socket).await?;
    test_error_conditions(&mut socket).await?;
    test_unicode_support(&mut socket).await?;
    
    println!("🎉 All tests completed successfully!");
    Ok(())
}

/// 🔧 Test basic CRUD operations
async fn test_basic_operations(socket: &mut TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 === Testing Basic Operations ===");
    
    // 💾 Test SET operation
    println!("🧪 Testing SET command...");
    send_command(socket, "SET user:123 Alice Johnson").await?;
    let response = read_response(socket).await?;
    println!("📤 SET Response: {}", response);
    assert!(response.contains("OK"), "SET should return OK");
    
    // 🔍 Test GET operation
    println!("🧪 Testing GET command...");
    send_command(socket, "GET user:123").await?;
    let response = read_response(socket).await?;
    println!("📤 GET Response: {}", response);
    assert!(response.contains("Alice Johnson"), "GET should return the stored value");
    
    // 🗑️ Test DELETE operation  
    println!("🧪 Testing DELETE command...");
    send_command(socket, "DELETE user:123").await?;
    let response = read_response(socket).await?;
    println!("📤 DELETE Response: {}", response);
    assert!(response.contains("OK"), "DELETE should return OK");
    
    // 🔍 Verify deletion worked
    println!("🧪 Verifying deletion...");
    send_command(socket, "GET user:123").await?;
    let response = read_response(socket).await?;
    println!("📤 GET (after delete) Response: {}", response);
    assert!(response.contains("NOT_FOUND"), "GET should return NOT_FOUND after deletion");
    
    println!("✅ Basic operations test passed!");
    Ok(())
}

/// 🚨 Test error handling and validation
async fn test_error_conditions(socket: &mut TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 === Testing Error Conditions ===");
    
    // ❓ Test unknown command
    println!("🧪 Testing unknown command...");
    send_command(socket, "UNKNOWN_CMD test").await?;
    let response = read_response(socket).await?;
    println!("📤 Unknown Command Response: {}", response);
    assert!(response.contains("ERROR"), "Unknown command should return ERROR");
    
    // 📭 Test missing arguments
    println!("🧪 Testing missing arguments...");
    send_command(socket, "GET").await?;
    let response = read_response(socket).await?;
    println!("📤 Missing Args Response: {}", response);
    assert!(response.contains("ERROR"), "Missing arguments should return ERROR");
    
    // 🔍 Test missing key
    println!("🧪 Testing missing key...");
    send_command(socket, "GET nonexistent_key_12345").await?;
    let response = read_response(socket).await?;
    println!("📤 Missing Key Response: {}", response);
    assert!(response.contains("NOT_FOUND"), "Missing key should return NOT_FOUND");
    
    println!("✅ Error conditions test passed!");
    Ok(())
}

/// 🌐 Test Unicode and special character support
async fn test_unicode_support(socket: &mut TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📋 === Testing Unicode Support ===");
    
    // 🇯🇵 Test Unicode characters
    println!("🧪 Testing Unicode characters...");
    send_command(socket, "SET 用户:123 こんにちは世界").await?;
    let response = read_response(socket).await?;
    println!("📤 Unicode SET Response: {}", response);
    assert!(response.contains("OK"), "Unicode SET should work");
    
    send_command(socket, "GET 用户:123").await?;
    let response = read_response(socket).await?;
    println!("📤 Unicode GET Response: {}", response);
    assert!(response.contains("こんにちは世界"), "Unicode GET should return correct value");
    
    // 🔣 Test special characters
    println!("🧪 Testing special characters...");
    send_command(socket, "SET key:with@special#chars! value_with_spaces_and_123").await?;
    let response = read_response(socket).await?;
    println!("📤 Special Chars SET Response: {}", response);
    assert!(response.contains("OK"), "Special characters should work");
    
    send_command(socket, "GET key:with@special#chars!").await?;
    let response = read_response(socket).await?;
    println!("📤 Special Chars GET Response: {}", response);
    assert!(response.contains("value_with_spaces_and_123"), "Special chars GET should work");
    
    println!("✅ Unicode support test passed!");
    Ok(())
}

/// 📤 Send a command to the server
async fn send_command(socket: &mut TcpStream, command: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Add CRLF termination as expected by the protocol
    let message = format!("{}\r\n", command);
    socket.write_all(message.as_bytes()).await?;
    println!("📡 Sent: {}", command);
    Ok(())
}

/// 📥 Read response from the server
async fn read_response(socket: &mut TcpStream) -> Result<String, Box<dyn std::error::Error>> {
    let mut buffer = [0u8; 1024];
    let bytes_read = socket.read(&mut buffer).await?;
    
    if bytes_read == 0 {
        return Err("Server closed connection".into());
    }
    
    let response = String::from_utf8_lossy(&buffer[..bytes_read]);
    Ok(response.trim().to_string())
}
