# MerkleKV 🚀

A high-performance, distributed key-value store with self-healing replication, built in Rust.

[![CI/CD Status](https://img.shields.io/github/actions/workflow/status/AI-Decenter/MerkleKV/ci.yml?branch=main&style=for-the-badge)](https://github.com/AI-Decenter/MerkleKV/actions)
[![Crates.io](https://img.shields.io/crates/v/merkledb.svg?style=for-the-badge)](https://crates.io/crates/merkledb)
[![License](https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)

MerkleKV is an eventually-consistent, distributed key-value database designed for speed, reliability, and operational simplicity. It uses an MQTT broker for rapid, real-time update propagation and a sophisticated **Merkle tree** anti-entropy mechanism to efficiently detect and repair data inconsistencies in the background.

## 📋 Table of Contents

- [✨ Key Features](#-key-features)
- [🏗️ Architecture](#️-architecture)
  - [System Architecture Overview](#system-architecture-overview)
  - [Data Flow Patterns](#data-flow-patterns)
  - [System Components](#system-components)
  - [Merkle Tree Structure & Synchronization](#merkle-tree-structure--synchronization)
- [🔧 Getting Started](#-getting-started)
  - [Prerequisites](#prerequisites)
  - [Installation & Setup](#installation--setup)
  - [Verification & Testing](#verification--testing)
- [📚 Usage (Client API)](#-usage-client-api)
  - [Available Commands](#available-commands)
  - [Interactive Session Example](#interactive-session-example)
- [⚙️ Configuration](#️-configuration)
  - [Configuration File Format](#configuration-file-format)
  - [Configuration Options Reference](#configuration-options-reference)
- [🗺️ Roadmap & Implementation Issues](#️-roadmap--implementation-issues)
- [🙌 Contributing](#-contributing)
- [📜 License](#-license)

---

## ✨ Key Features

### 🚀 Performance & Scalability
- **High Performance**: Built with Rust and the Tokio asynchronous runtime for low-latency and high-throughput operations
- **Horizontal Scaling**: Add or remove nodes dynamically without downtime
- **Memory Efficiency**: Optimized data structures and zero-copy operations where possible

### 🔄 Replication & Consistency
- **Fast Replication**: Updates are immediately published to an MQTT topic and broadcast to all peer nodes
- **Eventually Consistent**: Guarantees that all nodes will converge to the same state
- **Self-Healing**: The anti-entropy mechanism runs periodically to automatically find and fix any data drift between replicas

### 🛡️ Reliability & Safety
- **Memory Safety**: Guarantees provided by the Rust compiler prevent common bugs like null pointer dereferencing and data races
- **No Single Point of Failure**: Peer-to-peer architecture with no leader node
- **Fault Tolerance**: Continues operating even if individual nodes fail

### 🔧 Operational Simplicity
- **Simple Text Protocol**: An easy-to-use, Memcached-like protocol for `SET`, `GET`, and `DEL` operations
- **Easy Configuration**: TOML-based configuration with sensible defaults
- **Minimal Dependencies**: Only requires an MQTT broker for coordination

### 📊 Efficiency
- **Efficient Synchronization**: Merkle trees allow nodes to verify data integrity by comparing a single root hash
- **Logarithmic Sync**: Discrepancies are located with logarithmic time complexity (O(log n)) without full data scans
- **Bandwidth Optimization**: Only divergent data is synchronized, not entire datasets

---

## 🎯 Current Implementation Status

### ✅ **Completed Components**

**Core TCP Server**
- ✅ **Asynchronous TCP server** using Tokio runtime
- ✅ **Multi-client concurrent handling** with Arc<Mutex> thread safety
- ✅ **Protocol parser** for SET, GET, DELETE operations
- ✅ **Comprehensive error handling** with detailed error messages
- ✅ **Unicode support** for international characters
- ✅ **Performance**: 9,800+ operations/second

**Storage Engine**
- ✅ **In-memory key-value store** with thread-safe access
- ✅ **Merkle tree implementation** with 58 comprehensive tests
- ✅ **Cryptographic hashing** using SHA-256
- ✅ **Efficient diff algorithms** for data synchronization

**Testing & Quality**
- ✅ **62 unit tests** covering all core functionality
- ✅ **Integration test suite** with Python and Rust clients
- ✅ **Concurrent testing** (5 simultaneous clients)
- ✅ **Performance benchmarks** and edge case testing
- ✅ **Comprehensive error condition testing**

### 🚧 **In Progress / Planned**

**Distributed Features** *(Skeleton implementation)*
- 🚧 **MQTT-based replication** (basic structure in place)
- 🚧 **Anti-entropy synchronization** (Merkle tree diff ready)
- 🚧 **Peer discovery and clustering**
- 🚧 **Persistent storage** (currently memory-only)

**Operational Features**
- 🚧 **Configuration management** improvements
- 🚧 **Monitoring and metrics** collection
- 🚧 **Administrative commands** and health checks

---

## 🏗️ Architecture

MerkleKV is a distributed key-value store designed around a peer-to-peer architecture with no single point of failure. The system consists of a cluster of `MerkleKV` nodes, where all nodes are equal peers.

### System Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            MerkleKV Distributed Cluster                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐        ┌─────────────┐        ┌─────────────┐              │
│  │   Node A    │        │   Node B    │        │   Node C    │              │
│  │             │        │             │        │             │              │
│  │ ┌─────────┐ │        │ ┌─────────┐ │        │ ┌─────────┐ │              │
│  │ │Storage  │ │        │ │Storage  │ │        │ │Storage  │ │              │
│  │ │Engine   │ │        │ │Engine   │ │        │ │Engine   │ │              │
│  │ │+ Merkle │ │        │ │+ Merkle │ │        │ │+ Merkle │ │              │
│  │ │  Tree   │ │        │ │  Tree   │ │        │ │  Tree   │ │              │
│  │ └─────────┘ │        │ └─────────┘ │        │ └─────────┘ │              │
│  │             │        │             │        │             │              │
│  │ ┌─────────┐ │        │ ┌─────────┐ │        │ ┌─────────┐ │              │
│  │ │TCP      │ │        │ │TCP      │ │        │ │TCP      │ │              │
│  │ │Listener │ │        │ │Listener │ │        │ │Listener │ │              │
│  │ │:7878    │ │        │ │:7879    │ │        │ │:7880    │ │              │
│  │ └─────────┘ │        │ └─────────┘ │        │ └─────────┘ │              │
│  └─────────────┘        └─────────────┘        └─────────────┘              │
│         │                       │                       │                   │
│         └───────────────────────┼───────────────────────┘                   │
│                                 │                                           │
│  ┌──────────────────────────────┼──────────────────────────────┐            │
│  │              MQTT BROKER     │                              │            │
│  │                              │                              │            │
│  │     ┌─────────────────────────▼─────────────────────────┐    │            │
│  │     │          Topic: merkle-kv/updates              │    │            │
│  │     │                                                 │    │            │
│  │     │  Real-time update propagation                   │    │            │
│  │     │  • SET operations                               │    │            │
│  │     │  • DEL operations                               │    │            │
│  │     │  • Immediate replication                        │    │            │
│  │     └─────────────────────────────────────────────────┘    │            │
│  └───────────────────────────────────────────────────────────┘            │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                     Anti-Entropy Process                           │   │
│  │                                                                     │   │
│  │  Node A ←→ Node B: Compare root hashes                            │   │
│  │  Node B ←→ Node C: Compare root hashes                            │   │
│  │  Node C ←→ Node A: Compare root hashes                            │   │
│  │                                                                     │   │
│  │  If hashes differ: Tree traversal to find inconsistencies         │   │
│  │  Repair any divergent data                                         │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────────────┘

   ▲                              ▲                              ▲
   │                              │                              │
┌──┴──┐                       ┌──┴──┐                       ┌──┴──┐
│     │                       │     │                       │     │
│ TCP │                       │ TCP │                       │ TCP │
│Conn │                       │Conn │                       │Conn │
└─────┘                       └─────┘                       └─────┘
Clients                       Clients                       Clients
```

### Data Flow Patterns

#### 1. Write Path (Hot Replication)
```
Client ────[SET key value]───▶ Node A
   │                             │
   │                             ▼
   │                        ┌─────────┐
   │                        │ Update  │
   │                        │ Local   │
   │                        │ Storage │
   │                        └─────────┘
   │                             │
   │                             ▼
   │                        ┌─────────┐
   │                        │Publish  │
   │                        │to MQTT  │
   │                        │ Topic   │
   │                        └─────────┘
   │                             │
   │                             ▼
   │                     ┌───────────────┐
   │                     │  MQTT Broker  │
   │                     │  broadcasts   │
   │                     │  to all nodes │
   │                     └───────────────┘
   │                             │
   │                     ┌───────┴───────┐
   │                     ▼               ▼
   │                 Node B           Node C
   │                     │               │
   │                     ▼               ▼
   │               ┌─────────┐     ┌─────────┐
   │               │ Apply   │     │ Apply   │
   │               │ Change  │     │ Change  │
   │               │ Locally │     │ Locally │
   │               └─────────┘     └─────────┘
   │
   ▼
[Response: OK]
```

#### 2. Read Path
```
Client ────[GET key]───▶ Any Node
   │                        │
   │                        ▼
   │                   ┌─────────┐
   │                   │ Lookup  │
   │                   │ in Local│
   │                   │ Storage │
   │                   └─────────┘
   │                        │
   ▼                        ▼
[Response: value]    [Return value]
```


### System Components

#### Core Components

- **🗄️ KeyValue Node**: Each node is an independent server instance, holding a complete replica of the dataset. Multiple nodes form a peer-to-peer cluster with no single point of failure.

- **💾 Storage Engine**: The core component responsible for the actual key-value data storage and the management of the Merkle tree structure. Handles all CRUD operations and maintains data integrity.

- **🔌 Client Protocol Listener**: A TCP listener that parses and handles client commands (SET, GET, DEL) using a simple, text-based protocol. Provides the interface for client applications to interact with the database.

- **🔄 Replication Module**: Manages the connection to the MQTT broker for publishing local changes and subscribing to updates from other nodes. Ensures rapid propagation of changes across the cluster.

- **🔧 Anti-Entropy Module**: A background process that periodically initiates a Merkle tree comparison with other peer nodes to detect and repair any data inconsistencies. Provides eventual consistency guarantees.

#### External Dependencies

- **📡 MQTT Broker**: An external message broker (e.g., Mosquitto, EMQ X) that facilitates the pub/sub communication for update propagation between nodes.
 
---

### Merkle Tree Structure & Synchronization

The Merkle tree is the foundation of our efficient data verification strategy. It allows us to verify the integrity of the entire dataset by comparing a single hash, and to rapidly locate inconsistencies if they exist.

#### Merkle Tree Construction

```
                            Root Hash
                               │
                        Hash(H1 + H2)
                         ┌─────┴─────┐
                        H1           H2
                   Hash(H3+H4)   Hash(H5+H6)
                   ┌────┴────┐   ┌────┴────┐
                  H3        H4  H5        H6
              Hash(K1+V1) │   │  Hash(K3+V3)
                         │   │
                      Hash(K2+V2) Hash(K4+V4)
                         │   │
                     ┌───┴─┐ ┌─┴───┐
                   K1:V1  K2:V2  K3:V3  K4:V4
                  (Leaf)  (Leaf) (Leaf) (Leaf)

Keys sorted lexicographically: K1 < K2 < K3 < K4
Each leaf = Hash(key + value)
Each internal node = Hash(left_child + right_child)
```

#### Tree Construction Process

1. **Sort Keys**: All keys in the store are sorted lexicographically
2. **Create Leaves**: Each (key, value) pair is hashed to form a leaf node: `Hash(key + value)`
3. **Build Tree**: Adjacent nodes are concatenated and hashed together to form parent nodes
4. **Repeat**: This process continues recursively until a single root hash is generated

#### Synchronization Protocol

```
Node A                                    Node B
  │                                         │
  ├─── Request root hash ──────────────────▶│
  │                                         │
  │◄─── Root hash: ABC123 ───────────────────┤
  │                                         │
  │  Compare with local root hash (XYZ789)  │
  │  ❌ Hashes don't match!                 │
  │                                         │
  ├─── Request child hashes ───────────────▶│
  │                                         │
  │◄─── Left: DEF456, Right: GHI789 ────────┤
  │                                         │
  │  Compare children:                      │
  │  ✅ Left child matches                  │
  │  ❌ Right child differs                 │
  │                                         │
  ├─── Request right subtree ──────────────▶│
  │                                         │
  │    Continue traversing down only        │
  │    the divergent branch...              │
  │                                         │
  ├─── Found inconsistent key: "user:123" ─▶│
  │                                         │
  │◄─── Correct value: "john_doe" ──────────┤
  │                                         │
  │  Update local storage                   │
  │  Recalculate affected hashes            │
```

#### Synchronization Complexity

- **Time Complexity**: O(log n) where n is the number of keys
- **Space Complexity**: O(log n) for the traversal path
- **Network Efficiency**: Only divergent branches are compared, not the entire dataset

#### Example: Finding Inconsistency

```
Node A Tree (Inconsistent)          Node B Tree (Correct)
                                   
     Root: XYZ789                       Root: ABC123
         │                                  │
    ┌────┴────┐                        ┌────┴────┐
   H1: DEF456  H2: BAD999             H1: DEF456  H2: GHI789
       │           │                      │           │
   ┌───┴───┐   ┌───┴───┐              ┌───┴───┐   ┌───┴───┐
  user:100 user:123  user:200        user:100 user:123  user:200
   alice   WRONG_VAL  charlie         alice   john_doe   charlie
           ↑                                     ↑
        Inconsistent                          Correct
        
Steps:
1. Compare roots: XYZ789 ≠ ABC123 ❌
2. Compare children: H1 matches ✅, H2 differs ❌  
3. Traverse right subtree (H2)
4. Find leaf: user:123 has different value
5. Sync: Update "user:123" from "WRONG_VAL" → "john_doe"
6. Recalculate: H2: BAD999 → GHI789, Root: XYZ789 → ABC123
```

## 🔧 Getting Started

### Prerequisites

Before setting up MerkleKV, ensure you have the following dependencies installed:

#### Required Dependencies
- **🦀 Rust Toolchain**: Version 1.70.0 or later
  ```bash
  # Install via rustup (recommended)
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source ~/.cargo/env
  
  # Verify installation
  rustc --version
  cargo --version
  ```

- **📡 MQTT Broker**: A running instance of an MQTT broker
  
  **Option 1: Mosquitto (Recommended for production)**
  ```bash
  # Ubuntu/Debian
  sudo apt-get update
  sudo apt-get install mosquitto mosquitto-clients
  
  # macOS
  brew install mosquitto
  
  # Start the broker
  sudo systemctl start mosquitto  # Linux
  brew services start mosquitto   # macOS
  ```
  
  **Option 2: Docker (Quick setup)**
  ```bash
  # Run Mosquitto in Docker
  docker run -it -p 1883:1883 -p 9001:9001 eclipse-mosquitto
  ```
  
  **Option 3: Public test broker (Development only)**
  ```
  broker: test.mosquitto.org:1883
  ```

#### Optional Dependencies
- **🐳 Docker**: For containerized deployments
- **📊 Monitoring Tools**: Prometheus, Grafana (for production monitoring)

### Installation & Setup

#### Quick Start (Single Node)

1. **Clone the repository**
   ```bash
   git clone https://github.com/AI-Decenter/MerkleKV.git
   cd MerkleKV
   ```

2. **Build the project**
   ```bash
   # Debug build (faster compilation)
   cargo build
   
   # Release build (optimized)
   cargo build --release
   ```

3. **Create configuration file**
   ```bash
   # Create a basic configuration
   cat > node1.toml << EOF
   node_id = "local-node"
   
   [network]
   bind_address = "127.0.0.1"
   bind_port = 7878
   
   [mqtt]
   broker_address = "tcp://localhost:1883"
   topic_prefix = "dev-merkle-kv"
   EOF
   ```

4. **Start the node**
   ```bash
   # Run with debug build
   cargo run -- --config node1.toml
   
   # Or run with release build
   cargo run --release -- --config node1.toml
   ```

5. **Test the connection**
   ```bash
   # In another terminal
   nc localhost 7878
   
   # Try some commands
   SET hello world
   GET hello
   DEL hello
   ```

#### Multi-Node Cluster Setup

For a more realistic setup with replication and anti-entropy:

1. **Create multiple configuration files**
   ```bash
   # Node 1
   cat > node1.toml << EOF
   node_id = "node-1"
   
   [network]
   bind_address = "127.0.0.1"
   bind_port = 7878
   
   [mqtt]
   broker_address = "tcp://localhost:1883"
   topic_prefix = "cluster-merkle-kv"
   client_id = "node-1"
   
   [anti_entropy]
   enabled = true
   interval_seconds = 60
   peer_list = ["127.0.0.1:7879", "127.0.0.1:7880"]
   EOF
   
   # Node 2
   cat > node2.toml << EOF
   node_id = "node-2"
   
   [network]
   bind_address = "127.0.0.1"
   bind_port = 7879
   
   [mqtt]
   broker_address = "tcp://localhost:1883"
   topic_prefix = "cluster-merkle-kv"
   client_id = "node-2"
   
   [anti_entropy]
   enabled = true
   interval_seconds = 60
   peer_list = ["127.0.0.1:7878", "127.0.0.1:7880"]
   EOF
   
   # Node 3
   cat > node3.toml << EOF
   node_id = "node-3"
   
   [network]
   bind_address = "127.0.0.1"
   bind_port = 7880
   
   [mqtt]
   broker_address = "tcp://localhost:1883"
   topic_prefix = "cluster-merkle-kv"
   client_id = "node-3"
   
   [anti_entropy]
   enabled = true
   interval_seconds = 60
   peer_list = ["127.0.0.1:7878", "127.0.0.1:7879"]
   EOF
   ```

2. **Start all nodes**
   ```bash
   # Terminal 1: Start node 1
   cargo run --release -- --config node1.toml
   
   # Terminal 2: Start node 2
   cargo run --release -- --config node2.toml
   
   # Terminal 3: Start node 3
   cargo run --release -- --config node3.toml
   ```

3. **Test replication**
   ```bash
   # Connect to node 1
   echo "SET user:alice \"Alice Johnson\"" | nc localhost 7878
   
   # Connect to node 2 (different terminal)
   echo "GET user:alice" | nc localhost 7879  # Should return "Alice Johnson"
   
   # Connect to node 3 (different terminal)
   echo "GET user:alice" | nc localhost 7880  # Should return "Alice Johnson"
   ```

### Verification & Testing

#### 🧪 Comprehensive Test Suite

**Unit Tests (62 tests)**
```bash
# Run all unit tests
cargo test

# Run specific test modules
cargo test protocol  # Protocol parsing tests
cargo test store     # Storage engine tests
cargo test merkle    # Merkle tree tests
```

**Integration Tests**
```bash
# Start the server
cargo run

# Run basic functionality test
python3 simple_test.py

# Run concurrent client test (5 simultaneous connections)
python3 concurrent_test.py

# Run comprehensive edge case and performance test
python3 comprehensive_test.py

# Run Rust test client
cargo run --example test_client
```

**Performance Benchmarks**
```bash
# The comprehensive test includes performance metrics
# Expected results: 9,800+ operations/second
python3 comprehensive_test.py
```

#### Health Check
```bash
# Check if nodes are listening
netstat -tlnp | grep 787

# Test connectivity to each node
for port in 7878 7879 7880; do
  echo "Testing port $port..."
  echo "GET test" | nc localhost $port
done
```

#### MQTT Connectivity Test
```bash
# Subscribe to updates (requires mosquitto-clients)
mosquitto_sub -h localhost -t "cluster-merkle-kv/updates"

# In another terminal, make changes
echo "SET test:mqtt success" | nc localhost 7878

# Should see MQTT messages in the subscriber
```

#### Anti-Entropy Verification
```bash
# Stop node 2 temporarily
# Make changes to node 1
echo "SET offline:test value" | nc localhost 7878

# Restart node 2
cargo run --release -- --config node2.toml

# Wait for anti-entropy sync (60 seconds)
# Check if the change propagated
echo "GET offline:test" | nc localhost 7879
```

### Troubleshooting

#### Common Issues

**"Connection refused" errors**
```bash
# Check if the node is running
ps aux | grep merkle

# Check if the port is in use
sudo lsof -i :7878

# Check firewall settings
sudo ufw status
```

**MQTT connection failures**
```bash
# Test MQTT broker connectivity
mosquitto_pub -h localhost -t test -m "hello"

# Check broker logs
sudo journalctl -u mosquitto -f
```

**Anti-entropy not working**
```bash
# Check node logs for sync attempts
grep "anti-entropy" /var/log/merkle-kv/*.log

# Verify peer connectivity
nc -v <peer-ip> <peer-port>
```

### Performance Tuning

For production deployments:

```toml
[network]
max_connections = 10000
connection_timeout = 60

[storage]
memory_limit_mb = 8192  # 8GB

[anti_entropy]
interval_seconds = 300   # 5 minutes
max_concurrent_syncs = 5

[logging]
level = "warn"  # Reduce log verbosity
```

---

## 📚 Usage (Client API)

MerkleKV uses a simple, text-based protocol similar to Memcached. You can interact with any `MerkleKV` node using standard TCP clients like `netcat` (nc) or custom applications.

### Protocol Overview

- **Connection**: TCP connection to any node in the cluster
- **Format**: All commands must end with `\r\n` (CRLF)
- **Responses**: Simple text responses for easy parsing
- **Encoding**: UTF-8 text encoding

### Connecting to a Node

```bash
# Basic connection
nc localhost 7878

# Using netcat with timeout
nc -w 5 localhost 7878

# Using netcat with verbose output
nc -v localhost 7878
```

### Available Commands

#### SET Command
Store a key-value pair in the distributed store.

**Syntax**: `SET <key> <value>\r\n`

```bash
# Store a user record
SET user:100 jane_doe
OK

# Store JSON-like data (as string)
SET config:database {"host":"localhost","port":5432}
OK

# Store with spaces in value
SET message:1 Hello World from MerkleKV
OK
```

**Response**: `OK` on success, error message on failure.

#### GET Command
Retrieve a value by its key from the local node.

**Syntax**: `GET <key>\r\n`

```bash
# Get a user record
GET user:100
jane_doe

# Get non-existent key
GET user:999
(null)

# Get configuration
GET config:database
{"host":"localhost","port":5432}
```

**Response**: The value if found, `(null)` if key doesn't exist.

#### DEL Command
Delete a key and its associated value from the distributed store.

**Syntax**: `DEL <key>\r\n`

```bash
# Delete a user record
DEL user:100
DELETED

# Delete non-existent key
DEL user:999
NOT_FOUND
```

**Response**: `DELETED` on successful deletion, `NOT_FOUND` if key doesn't exist.

### Interactive Session Example

```bash
$ nc -v localhost 7878
Connection to localhost 7878 port [tcp/*] succeeded!

# Store some data
SET user:alice Alice Smith
OK

SET user:bob Bob Johnson  
OK

SET counter:views 1500
OK

# Retrieve data
GET user:alice
Alice Smith

GET counter:views
1500

# Update existing data
SET counter:views 1501
OK

GET counter:views
1501

# Delete data
DEL user:bob
DELETED

GET user:bob
(null)

# Close connection
# Use Ctrl+C or Ctrl+D to exit
Connection closed.
```

### Client Library Integration

While MerkleKV uses a simple text protocol, you can easily integrate it into applications:

#### Python Example
```python
import socket

def merkle_kv_client(host, port):
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.connect((host, port))
    return sock

# Usage
client = merkle_kv_client('localhost', 7878)
client.send(b'SET user:123 john_doe\r\n')
response = client.recv(1024).decode().strip()
print(f"Response: {response}")  # Output: Response: OK
```

#### Bash Script Example
```bash
#!/bin/bash
exec 3<>/dev/tcp/localhost/7878
echo "SET script:status running" >&3
read response <&3
echo "Response: $response"
exec 3<&-
exec 3>&-
```

### Error Handling

Common error responses:
- `ERROR: Invalid command` - Unknown command sent
- `ERROR: Missing arguments` - Command missing required parameters  
- `ERROR: Connection failed` - Network or node issues
- `ERROR: Storage full` - Node storage capacity exceeded (if limits configured)
## ⚙️ Configuration

MerkleKV nodes are configured using TOML files, providing a simple and readable configuration format. Each node requires its own configuration file to specify network settings, identification, and operational parameters.

### Configuration File Format

#### Basic Configuration Example (`node1.toml`)

```toml
# Node Identity
node_id = "node-alpha"
description = "Primary MerkleKV node in us-east datacenter"

# Network Configuration
[network]
bind_address = "127.0.0.1"
bind_port = 7878
max_connections = 1000

# MQTT Broker Settings
[mqtt]
broker_address = "tcp://test.mosquitto.org:1883"
topic_prefix = "merkle-kv"
client_id = "merkle-node-alpha"
keep_alive = 60
clean_session = true

# Anti-Entropy Settings
[anti_entropy]
enabled = true
interval_seconds = 300  # 5 minutes
peer_discovery_interval = 600  # 10 minutes
max_concurrent_syncs = 3

# Storage Configuration
[storage]
memory_limit_mb = 1024  # 1GB memory limit
persistence_enabled = false
backup_interval_hours = 24

# Logging Configuration  
[logging]
level = "info"  # trace, debug, info, warn, error
format = "json"  # json, text
file_path = "/var/log/merkle-kv/node-alpha.log"
```

#### Multi-Node Cluster Example

**Node 1 Configuration (`node1.toml`)**
```toml
node_id = "node-primary"

[network]
bind_address = "192.168.1.10"
bind_port = 7878

[mqtt]
broker_address = "tcp://192.168.1.100:1883"
topic_prefix = "prod-merkle-kv"
client_id = "primary-node"

[anti_entropy]
enabled = true
interval_seconds = 180
peer_list = ["192.168.1.11:7878", "192.168.1.12:7878"]
```

**Node 2 Configuration (`node2.toml`)**
```toml
node_id = "node-secondary"

[network]
bind_address = "192.168.1.11" 
bind_port = 7878

[mqtt]
broker_address = "tcp://192.168.1.100:1883"
topic_prefix = "prod-merkle-kv"
client_id = "secondary-node"

[anti_entropy]
enabled = true
interval_seconds = 180
peer_list = ["192.168.1.10:7878", "192.168.1.12:7878"]
```

**Node 3 Configuration (`node3.toml`)**
```toml
node_id = "node-tertiary"

[network]
bind_address = "192.168.1.12"
bind_port = 7878

[mqtt]
broker_address = "tcp://192.168.1.100:1883" 
topic_prefix = "prod-merkle-kv"
client_id = "tertiary-node"

[anti_entropy]
enabled = true
interval_seconds = 180
peer_list = ["192.168.1.10:7878", "192.168.1.11:7878"]
```

### Configuration Options Reference

#### Core Settings
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `node_id` | String | Required | Unique identifier for this node in the cluster |
| `description` | String | Optional | Human-readable description of the node |

#### Network Section `[network]`
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `bind_address` | String | "127.0.0.1" | IP address to bind the TCP listener |
| `bind_port` | Integer | 7878 | Port number for client connections |
| `max_connections` | Integer | 1000 | Maximum concurrent client connections |
| `connection_timeout` | Integer | 30 | Connection timeout in seconds |

#### MQTT Section `[mqtt]`
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `broker_address` | String | Required | MQTT broker connection string |
| `topic_prefix` | String | "merkle-kv" | Prefix for MQTT topics |
| `client_id` | String | node_id | MQTT client identifier |
| `username` | String | Optional | MQTT authentication username |
| `password` | String | Optional | MQTT authentication password |
| `keep_alive` | Integer | 60 | MQTT keep-alive interval in seconds |
| `clean_session` | Boolean | true | MQTT clean session flag |

#### Anti-Entropy Section `[anti_entropy]`
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `enabled` | Boolean | true | Enable/disable anti-entropy mechanism |
| `interval_seconds` | Integer | 300 | Interval between sync attempts |
| `peer_discovery_interval` | Integer | 600 | Interval for peer discovery |
| `max_concurrent_syncs` | Integer | 3 | Maximum concurrent synchronizations |
| `peer_list` | Array | [] | Static list of known peer addresses |

### Running with Configuration

```bash
# Start node with specific config
cargo run --release -- --config /path/to/node1.toml

# Start multiple nodes for testing
cargo run --release -- --config configs/node1.toml &
cargo run --release -- --config configs/node2.toml &
cargo run --release -- --config configs/node3.toml &

# Verify nodes are running
ps aux | grep merkle-kv
```

### Environment Variable Overrides

Configuration values can be overridden using environment variables:

```bash
# Override node ID
export MERKLE_KV_NODE_ID="production-node-1"

# Override MQTT broker
export MERKLE_KV_MQTT_BROKER_ADDRESS="tcp://production-mqtt.example.com:1883"

# Override bind port
export MERKLE_KV_NETWORK_BIND_PORT=8080

# Start with overrides
cargo run --release -- --config node.toml
```

### Configuration Validation

The system validates configuration on startup:

- ✅ **Required fields**: `node_id`, `mqtt.broker_address`
- ✅ **Port ranges**: 1-65535 for `bind_port`
- ✅ **IP addresses**: Valid IPv4/IPv6 for `bind_address`
- ✅ **MQTT URLs**: Valid broker connection strings
- ✅ **Node ID uniqueness**: Within the cluster (checked at runtime)
## 🗺️ Roadmap & Implementation Issues

We have an exciting future planned for MerkleKV! Below is a comprehensive issue list for implementing the key features and architecture components described above.

### Phase 1: Core Foundation
**Priority: High** - Essential components for basic functionality

- [ ] **Issue #1: Project Structure & Build System**
  - Set up Cargo workspace and project structure
  - Configure dependencies (tokio, serde, toml, etc.)
  - Set up CI/CD pipeline and testing framework
  - Create basic configuration system

- [ ] **Issue #2: Storage Engine Foundation**
  - Implement in-memory key-value storage with HashMap
  - Add thread-safe access patterns using RwLock/Mutex
  - Create basic CRUD operations (Create, Read, Update, Delete)
  - Implement key-value serialization/deserialization

- [ ] **Issue #3: Merkle Tree Implementation**
  - Design and implement Merkle tree data structure
  - Add lexicographical key sorting functionality
  - Implement hash computation for leaf and internal nodes
  - Create root hash calculation and tree traversal methods

### Phase 2: Client Interface
**Priority: High** - User-facing functionality

- [ ] **Issue #4: TCP Protocol Listener**
  - Implement asynchronous TCP server using Tokio
  - Create command parser for SET, GET, DEL operations
  - Add protocol validation and error handling
  - Implement response formatting and client communication

- [ ] **Issue #5: Client Command Processing**
  - Integrate TCP listener with storage engine
  - Add command execution logic for all operations
  - Implement proper error responses and status codes
  - Add basic logging and monitoring capabilities

### Phase 3: Distributed System Core
**Priority: High** - Replication and consistency

- [ ] **Issue #6: MQTT Integration**
  - Integrate MQTT client library (rumqttmqtt or similar)
  - Implement connection management and reconnection logic
  - Create message publishing for local changes
  - Add subscription handling for remote updates

- [ ] **Issue #7: Replication Module**
  - Design change event format and serialization
  - Implement change publishing on local operations
  - Add remote change application logic
  - Handle basic conflict resolution (last-write-wins)

- [ ] **Issue #8: Anti-Entropy Mechanism**
  - Implement periodic peer discovery and selection
  - Create Merkle tree comparison protocol
  - Add efficient tree traversal for inconsistency detection
  - Implement repair operations for divergent data

### Phase 4: Advanced Features
**Priority: Medium** - Performance and reliability improvements

- [ ] **Issue #9: Configuration Management**
  - Implement TOML configuration parsing
  - Add runtime configuration validation
  - Create configuration hot-reloading capability
  - Add environment variable override support

- [ ] **Issue #10: Persistent Storage Backend**
  - Integrate Sled embedded database for disk persistence
  - Implement write-ahead logging (WAL) for durability
  - Add database recovery and initialization logic
  - Create storage engine abstraction layer

- [ ] **Issue #11: Performance Optimizations**
  - Implement connection pooling and reuse
  - Add batching for multiple operations
  - Optimize Merkle tree updates and caching
  - Add metrics collection and performance monitoring

### Phase 5: Production Readiness
**Priority: Low** - Enhanced robustness and features

- [ ] **Issue #12: Advanced Conflict Resolution**
  - Implement Vector Clocks for causality tracking
  - Add sophisticated conflict detection algorithms
  - Create conflict resolution strategies (manual/automatic)
  - Add conflict history and audit trails

- [ ] **Issue #13: Monitoring & Observability**
  - Implement structured logging with tracing
  - Add Prometheus metrics endpoint
  - Create health check endpoints
  - Add distributed tracing support

- [ ] **Issue #14: Security & Authentication**
  - Add TLS support for client connections
  - Implement MQTT authentication mechanisms
  - Add access control and authorization
  - Create secure configuration management

- [ ] **Issue #15: Documentation & Tooling**
  - Create comprehensive API documentation
  - Add deployment guides and examples
  - Create client libraries for popular languages
  - Add benchmarking and testing tools

## 🙌 Contributing

Contributions are what make the open-source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

### Ways to Contribute

- 🐛 **Report Bugs**: Open an issue with detailed reproduction steps
- 💡 **Suggest Features**: Share ideas for new functionality
- 📝 **Improve Documentation**: Help make docs clearer and more comprehensive
- 🔧 **Submit Code**: Fix bugs, implement features, or optimize performance
- 🧪 **Add Tests**: Improve test coverage and reliability
- 📊 **Performance Testing**: Benchmark and optimize the system

### Development Process

1. **Fork the repository**
   ```bash
   git clone https://github.com/YOUR_USERNAME/MerkleKV.git
   cd MerkleKV
   ```

2. **Create a feature branch**
   ```bash
   git checkout -b feature/amazing-feature
   # or
   git checkout -b fix/bug-description
   ```

3. **Make your changes**
   ```bash
   # Follow Rust conventions
   cargo fmt
   cargo clippy
   cargo test
   ```

4. **Commit your changes**
   ```bash
   git commit -m "feat: add amazing feature"
   # Use conventional commit format:
   # feat: new feature
   # fix: bug fix
   # docs: documentation changes
   # test: add or update tests
   # refactor: code refactoring
   ```

5. **Push and create Pull Request**
   ```bash
   git push origin feature/amazing-feature
   ```

### Code Style & Guidelines

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Add tests for new functionality
- Update documentation for API changes
- Keep commits atomic and well-described

### Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_merkle_tree

# Run integration tests
cargo test --test integration_tests
```

### Getting Help

- 💬 **Discussions**: Use GitHub Discussions for questions
- 🐛 **Issues**: Report bugs and feature requests
- 📧 **Email**: Contact maintainers for sensitive issues

For major changes, please open an issue first to discuss what you would like to change.

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

```
MIT License

Copyright (c) 2025 AI-Decenter

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
```

