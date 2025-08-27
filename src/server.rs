//! # 🚀 Asynchronous TCP Server Implementation
//!
//! This module provides a high-performance TCP server that handles client connections 
//! and processes commands using Tokio's async runtime. It demonstrates modern Rust
//! async patterns and concurrent programming techniques.
//!
//! ## 🏗️ Architecture Overview
//! 
//! The server uses an asynchronous, multi-connection design pattern:
//! - 🎯 **Main Event Loop**: Accepts incoming connections continuously
//! - ⚡ **Task Spawning**: Each connection spawns a separate async task
//! - 🔧 **Command Processing**: Commands are parsed and executed against shared storage
//! - 📡 **Response Protocol**: Structured responses sent back to clients
//!
//! ## 📋 Protocol Specification
//! 
//! The server implements a Redis-inspired text protocol with clear delimiters:
//! - 📝 **Commands**: `GET key`, `SET key value`, `DELETE key`
//! - ✅ **Success**: `VALUE data`, `OK`
//! - ❌ **Errors**: `NOT_FOUND`, `ERROR message`
//! - 🔚 **Termination**: All messages end with `\r\n` (CRLF)
//!
//! ## 🔒 Concurrency & Thread Safety
//! 
//! The storage engine uses `Arc<Mutex<T>>` for safe concurrent access:
//! - 🧵 **Arc**: Atomic Reference Counting for shared ownership
//! - 🔐 **Mutex**: Mutual exclusion for synchronized access
//! - 🎭 **Async Tasks**: Each connection runs independently

use crate::store::kv_engine::KvEngine;
use anyhow::Result;
use log::{error, info};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::config::Config;
use crate::protocol::{Command, Protocol};

/// 🏗️ TCP Server for handling client connections with async I/O.
/// 
/// This server demonstrates several key Rust async patterns:
/// - 📡 **TcpListener**: Binds to an address and accepts connections
/// - 🎯 **Task Spawning**: Uses `tokio::spawn` for concurrent handling
/// - 🔄 **Event Loop**: Continuously processes incoming connections
/// - 🛡️ **Error Recovery**: Continues serving despite individual connection failures
pub struct Server {
    /// 📋 Server configuration including bind address and port
    config: Config,
    
    /// 🗄️ The storage engine shared across all client connections
    /// Uses Arc<Mutex<T>> pattern for thread-safe sharing
    store: KvEngine,
}

impl Server {
    /// Create a new server instance.
    /// 
    /// # Arguments
    /// * `config` - Server configuration (address, port, etc.)
    /// * `store` - Storage engine instance to use for all operations
    /// 
    /// # Returns
    /// * `Server` - New server instance ready to run
    pub fn new(config: Config, store: KvEngine) -> Self {
        Self { config, store }
    }

    /// 🚀 Start the server and begin accepting connections.
    /// 
    /// This method implements the main server event loop using Tokio's async runtime:
    /// 1. 🔌 **Bind**: Creates a TcpListener on the configured address
    /// 2. 🔄 **Loop**: Continuously accepts new connections
    /// 3. 🧵 **Spawn**: Creates independent tasks for each client
    /// 4. 🛡️ **Recovery**: Handles individual connection failures gracefully
    /// 
    /// # 🎯 Async Patterns Demonstrated
    /// - **Non-blocking I/O**: Uses `.await` for async operations
    /// - **Task Concurrency**: Multiple connections handled simultaneously
    /// - **Resource Sharing**: Storage engine shared via Arc<Mutex<T>>
    /// 
    /// # 📊 Returns
    /// * `Result<()>` - Never returns normally, only on fatal bind errors
    /// 
    /// # ⚠️ Error Conditions
    /// Returns an error if:
    /// - 🚫 Unable to bind to the specified address/port (port in use, permissions)
    /// - 🌐 Network-level configuration errors occur
    /// - 🔒 System resource limitations prevent binding
    /// 
    /// # 💡 Usage Example
    /// ```rust
    /// let config = Config::default();
    /// let store = KvEngine::new("./data")?;
    /// let server = Server::new(config, store);
    /// 
    /// // 🚀 This runs forever, handling multiple concurrent connections
    /// server.run().await?;
    /// ```
    pub async fn run(&self) -> Result<()> {
        // 🔗 Construct the bind address from config
        let addr = format!("{}:{}", self.config.host, self.config.port);
        
        // 🎯 Bind to the address - this can fail if port is in use
        let listener = TcpListener::bind(&addr).await?;
        info!("🚀 Server listening on {} (ready for connections)", addr);

        // 🔄 Wrap storage in Arc<Mutex<T>> for thread-safe concurrent access
        // Arc = Atomic Reference Counting (shared ownership)
        // Mutex = Mutual exclusion lock (synchronized access)
        let store = Arc::new(Mutex::new(self.store.clone()));

        // 🔮 Future enhancements to consider:
        // TODO: 🛑 Add graceful shutdown handling (SIGINT/SIGTERM)
        // TODO: 🚦 Add connection limits and rate limiting per client
        // TODO: 📊 Add metrics collection (connections/sec, commands/sec, errors)
        // TODO: 🔍 Add request/response logging for debugging
        // TODO: 🕐 Add connection timeouts to prevent resource leaks

        // 🔄 Main event loop - runs indefinitely
        loop {
            // 🤝 Accept incoming connection (blocking until client connects)
            match listener.accept().await {
                Ok((socket, client_addr)) => {
                    info!("✅ New client connected from: {}", client_addr);
                    
                    // 📋 Clone the store reference for this connection
                    let store_clone = Arc::clone(&store);
                    
                    // 🧵 Spawn independent async task for this client
                    // Each task runs concurrently without blocking others
                    tokio::spawn(async move {
                        // 🎭 Handle this client's session
                        match handle_connection(socket, client_addr, store_clone).await {
                            Ok(_) => info!("👋 Client {} disconnected cleanly", client_addr),
                            Err(e) => error!("💥 Error handling client {}: {}", client_addr, e),
                        }
                    });
                }
                Err(e) => {
                    // 🚨 Log accept errors but continue serving other clients
                    error!("⚠️  Failed to accept connection: {}", e);
                    // 🔄 Continue the loop - one bad connection shouldn't kill server
                }
            }
        }
    }
}

/// 🎭 Handle a single client connection with full async I/O.
/// 
/// This function demonstrates several advanced async patterns:
/// - 📖 **Buffered Reading**: Reads data in chunks for efficiency
/// - 🔄 **Event Loop**: Processes multiple commands per connection
/// - 🔐 **Mutex Management**: Acquires locks only when needed
/// - 🛡️ **Error Handling**: Graceful degradation and client feedback
/// 
/// # 🎯 Parameters
/// * `socket` - 📡 The TCP stream for bidirectional communication
/// * `client_addr` - 🏠 Client's network address (for logging & debugging)
/// * `store` - 🗄️ Thread-safe reference to the shared storage engine
/// 
/// # 📊 Returns
/// * `Result<()>` - Success when client disconnects gracefully
/// 
/// # 🔄 Protocol Flow
/// 1. 📥 **Read**: Get raw bytes from socket buffer
/// 2. 🔤 **Parse**: Convert bytes to UTF-8 string
/// 3. 🧠 **Process**: Parse command and execute against storage
/// 4. 📤 **Respond**: Send formatted response back to client
/// 5. 🔄 **Repeat**: Continue until client disconnects
/// 
/// # ⚠️ Error Scenarios
/// - 🔌 **Network errors**: Client disconnection, timeouts
/// - 📝 **Protocol errors**: Invalid UTF-8, malformed commands  
/// - 🗄️ **Storage errors**: Lock contention, internal failures
async fn handle_connection(
    mut socket: TcpStream,
    client_addr: SocketAddr,
    store: Arc<Mutex<KvEngine>>,
) -> Result<()> {
    // 📦 Create buffer for reading client data (1KB chunks)
    let mut buffer = [0u8; 1024];
    
    info!("🔗 Starting session for client: {}", client_addr);

    // 🔄 Main client communication loop
    loop {
        // 📖 Read data from client socket (async, non-blocking)
        match socket.read(&mut buffer).await {
            Ok(0) => {
                // 👋 Client closed connection gracefully (EOF)
                info!("🔚 Client {} closed connection", client_addr);
                return Ok(());
            }
            Ok(bytes_read) => {
                // 📊 Log received data size for debugging
                log::debug!("📥 Received {} bytes from {}", bytes_read, client_addr);
                
                // 🔤 Convert raw bytes to UTF-8 string
                let request_str = match std::str::from_utf8(&buffer[..bytes_read]) {
                    Ok(s) => s.trim(), // Remove whitespace/newlines
                    Err(e) => {
                        // 💥 Invalid UTF-8 - send error response
                        let error_msg = format!("❌ ERROR Invalid UTF-8: {}\r\n", e);
                        socket.write_all(error_msg.as_bytes()).await?;
                        continue; // Try next command
                    }
                };

                log::debug!("🔍 Processing command: '{}'", request_str);
                
                // 🧠 Parse and execute the command
                let response = process_command(request_str, &store, client_addr).await;
                
                // 📤 Send response back to client
                socket.write_all(response.as_bytes()).await?;
                log::debug!("📤 Sent response to {}: {}", client_addr, response.trim());
            }
            Err(e) => {
                // 🚨 Network error occurred
                error!("💥 Network error with client {}: {}", client_addr, e);
            }
        }
    }
}

/// 🧠 Process a single command and return a formatted response.
/// 
/// This function encapsulates the command processing logic, demonstrating:
/// - 🔍 **Command Parsing**: Using the Protocol parser for validation
/// - 🔐 **Lock Management**: Acquiring mutex locks efficiently  
/// - 🎯 **Pattern Matching**: Rust's powerful match expressions
/// - 📊 **Response Formatting**: Consistent protocol responses
/// 
/// # 🎯 Parameters
/// * `request` - 📝 Raw command string from client
/// * `store` - 🗄️ Thread-safe storage reference
/// * `client_addr` - 🏠 Client address for enhanced logging
/// 
/// # 📊 Returns
/// * `String` - Formatted response ready to send to client
/// 
/// # 🔄 Command Processing Flow
/// 1. 🔍 **Parse**: Validate command syntax using Protocol parser
/// 2. 🔐 **Lock**: Acquire exclusive access to storage (brief duration)
/// 3. 🎯 **Execute**: Perform the requested operation
/// 4. 📤 **Format**: Create standardized response string
async fn process_command(
    request: &str, 
    store: &Arc<Mutex<KvEngine>>, 
    client_addr: SocketAddr
) -> String {
    // 🔧 Create protocol parser instance
    let protocol = Protocol::new();
    
    // 🔍 Parse the incoming command
    match protocol.parse(request) {
        Ok(command) => {
            // 🔐 Acquire exclusive lock on storage (async-aware mutex)
            let mut store_guard = store.lock().await;
            
            // 🎯 Execute command using Rust's pattern matching
            match command {
                Command::Get { key } => {
                    log::debug!("🔍 GET operation for key: '{}' from {}", key, client_addr);
                    match store_guard.get(&key) {
                        Some(value) => {
                            log::debug!("✅ Found value for '{}': '{}'", key, value);
                            format!("✨ VALUE {}\r\n", value)
                        },
                        None => {
                            log::debug!("❌ Key '{}' not found", key);
                            "🔍 NOT_FOUND\r\n".to_string()
                        }
                    }
                }
                Command::Set { key, value } => {
                    log::debug!("💾 SET operation: '{}' = '{}' from {}", key, value, client_addr);
                    store_guard.set(key.clone(), value.clone());
                    log::debug!("✅ Successfully stored '{}' = '{}'", key, value);
                    "✅ OK\r\n".to_string()
                }
                Command::Delete { key } => {
                    log::debug!("🗑️ DELETE operation for key: '{}' from {}", key, client_addr);
                    store_guard.delete(&key);
                    log::debug!("✅ Deleted key: '{}'", key);
                    "🗑️ OK\r\n".to_string()
                }
            }
            // 🔓 Lock automatically released when guard goes out of scope
        }
        Err(parse_error) => {
            // 💥 Command parsing failed - return detailed error
            log::warn!("⚠️ Invalid command from {}: '{}' -> {}", client_addr, request, parse_error);
            format!("❌ ERROR Invalid command: {}\r\n", parse_error)
        }
    }
}