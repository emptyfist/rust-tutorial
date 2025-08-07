# 🦀 Redis + Rust Configuration Guide

> The most valuable Redis configuration knowledge for Rust developers

## 📚 Table of Contents

1. [Connection Types](#1-connection-types-choose-your-fighter-)
2. [Connection URLs](#2-connection-urls-the-magic-strings-)
3. [Connection Pooling](#3-connection-pooling-dont-be-greedy-)
4. [Dependencies](#4-cargotoml-dependencies-)
5. [Error Handling](#5-error-handling-expect-the-unexpected-)
6. [Configuration Options](#6-configuration-options-the-power-settings-)
7. [Performance Tips](#7-performance-tips-go-fast-)
8. [Real-World App Structure](#8-real-world-app-structure-)
9. [Environment Configuration](#9-environment-configuration-)
10. [Docker Setup](#10-docker-compose-setup-)
11. [Key Takeaways](#-most-valuable-takeaways)
12. [Common Mistakes](#-common-mistakes-to-avoid)

---

## 1. **Connection Types: Choose Your Fighter** 🥊

### Synchronous (Blocking)

```rust
// ❌ SYNC (blocking) - simpler but slower
use redis::{Client, Connection};

let client = Client::open("redis://127.0.0.1:6379")?;
let mut conn = client.get_connection()?;
```

### Asynchronous (Non-blocking) ✅

```rust
// ✅ ASYNC (non-blocking) - faster for web apps
use redis::aio::Connection;

let client = Client::open("redis://127.0.0.1:6379")?;
let mut conn = client.get_async_connection().await?;
```

**Rule of thumb**: Use async for web servers, sync for simple scripts.

---

## 2. **Connection URLs: The Magic Strings** 🪄

```rust
// Basic connection
"redis://127.0.0.1:6379"

// With password
"redis://:password123@127.0.0.1:6379"

// With username + password (Redis 6+)
"redis://username:password@127.0.0.1:6379"

// SSL/TLS connection
"rediss://127.0.0.1:6380"

// Unix socket (super fast for local connections)
"redis+unix:///tmp/redis.sock"

// Select database (0-15 by default)
"redis://127.0.0.1:6379/2"
```

---

## 3. **Connection Pooling: Don't Be Greedy** 🏊‍♂️

### ❌ Bad: Creating connections repeatedly

```rust
async fn bad_approach() {
    let client = Client::open("redis://127.0.0.1:6379")?;
    let mut conn = client.get_async_connection().await?; // EXPENSIVE!
    // use connection once and throw away
}
```

### ✅ Good: Reuse connections

```rust
use redis::aio::ConnectionManager;

async fn good_approach() {
    let client = Client::open("redis://127.0.0.1:6379")?;
    let conn_manager = ConnectionManager::new(client).await?;
    // Reuse this manager across your app
}
```

---

## 4. **Cargo.toml Dependencies** 📦

```toml
[dependencies]
redis = { version = "0.24", features = ["aio", "tokio-comp"] }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### Key Redis Features:

- `aio` = async support
- `tokio-comp` = Tokio compatibility
- `tls` = SSL support
- `cluster` = Redis cluster support
- `connection-manager` = Connection pooling

---

## 5. **Error Handling: Expect the Unexpected** 🚨

```rust
use redis::{RedisError, RedisResult};

// ✅ GOOD: Handle specific errors
match redis_operation().await {
    Ok(result) => println!("Success: {}", result),
    Err(RedisError { kind, .. }) => match kind {
        redis::ErrorKind::IoError => println!("Network problem!"),
        redis::ErrorKind::AuthenticationFailed => println!("Wrong password!"),
        redis::ErrorKind::TypeError => println!("Wrong data type!"),
        redis::ErrorKind::ResponseError => println!("Redis command error!"),
        _ => println!("Other Redis error: {}", kind),
    }
}
```

### Error Types to Handle:

- **IoError**: Network connectivity issues
- **AuthenticationFailed**: Wrong credentials
- **TypeError**: Data type mismatch
- **ResponseError**: Invalid Redis command
- **ParseError**: Data parsing issues

---

## 6. **Configuration Options: The Power Settings** ⚙️

```rust
let client = redis::Client::open(redis::ConnectionInfo {
    addr: redis::ConnectionAddr::Tcp("127.0.0.1".to_string(), 6379),
    redis: redis::RedisConnectionInfo {
        db: 0,                    // Database number (0-15)
        username: None,           // Redis 6+ username
        password: None,           // Password
    },
})?;
```

### Advanced Configuration:

```rust
use redis::ConnectionInfo;
use std::time::Duration;

let client = redis::Client::open(ConnectionInfo {
    addr: redis::ConnectionAddr::Tcp("127.0.0.1".to_string(), 6379),
    redis: redis::RedisConnectionInfo {
        db: 0,
        username: Some("myuser".to_string()),
        password: Some("mypass".to_string()),
    },
})?;
```

---

## 7. **Performance Tips: Go Fast** 🏎️

### Pipeline Operations

```rust
// ✅ PIPELINE: Batch operations for speed
let mut pipe = redis::pipe();
pipe.set("key1", "value1")
    .set("key2", "value2")
    .set("key3", "value3");
pipe.query_async(&mut conn).await?; // Send all at once!
```

### Atomic Transactions

```rust
// ✅ ATOMIC: Use transactions for consistency
let mut pipe = redis::pipe();
pipe.atomic()  // MULTI/EXEC
    .set("inventory", 100)
    .decr("inventory", 1)
    .query_async(&mut conn).await?;
```

### Bulk Operations

```rust
// ✅ BULK: Process multiple keys efficiently
let keys: Vec<String> = (0..1000).map(|i| format!("key:{}", i)).collect();
let values: Vec<i32> = conn.mget(&keys).await?;
```

---

## 8. **Real-World App Structure** 🏗️

### Connection Manager Pattern

```rust
use redis::aio::ConnectionManager;
use redis::{Client, RedisError};

#[derive(Clone)]
pub struct RedisPool {
    manager: ConnectionManager,
}

impl RedisPool {
    pub async fn new(redis_url: &str) -> Result<Self, RedisError> {
        let client = Client::open(redis_url)?;
        let manager = ConnectionManager::new(client).await?;
        Ok(Self { manager })
    }

    pub async fn get_connection(&self) -> ConnectionManager {
        self.manager.clone() // Cheap clone!
    }
}

// Application State
pub struct AppState {
    pub redis: RedisPool,
    // other fields...
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let redis_pool = RedisPool::new("redis://127.0.0.1:6379").await?;

    let app_state = AppState {
        redis: redis_pool,
    };

    // Use throughout your application
    let mut conn = app_state.redis.get_connection().await;
    conn.set("hello", "world").await?;

    Ok(())
}
```

### Repository Pattern

```rust
pub struct UserRepository {
    redis: RedisPool,
}

impl UserRepository {
    pub fn new(redis: RedisPool) -> Self {
        Self { redis }
    }

    pub async fn get_user(&self, id: u64) -> Result<Option<User>, RedisError> {
        let mut conn = self.redis.get_connection().await;
        let user_data: Option<String> = conn.get(format!("user:{}", id)).await?;

        match user_data {
            Some(data) => Ok(Some(serde_json::from_str(&data)?)),
            None => Ok(None),
        }
    }

    pub async fn save_user(&self, user: &User) -> Result<(), RedisError> {
        let mut conn = self.redis.get_connection().await;
        let data = serde_json::to_string(user)?;
        conn.set(format!("user:{}", user.id), data).await?;
        Ok(())
    }
}
```

---

## 9. **Environment Configuration** 🌍

### Using Environment Variables

```rust
use std::env;

pub struct RedisConfig {
    pub url: String,
    pub password: Option<String>,
    pub db: u8,
    pub max_connections: u32,
}

impl RedisConfig {
    pub fn from_env() -> Self {
        Self {
            url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            password: env::var("REDIS_PASSWORD").ok(),
            db: env::var("REDIS_DB")
                .unwrap_or_else(|_| "0".to_string())
                .parse()
                .unwrap_or(0),
            max_connections: env::var("REDIS_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
        }
    }

    pub fn connection_url(&self) -> String {
        match &self.password {
            Some(pwd) => format!("redis://:{}@{}/{}", pwd,
                self.url.trim_start_matches("redis://"), self.db),
            None => format!("{}/{}", self.url, self.db),
        }
    }
}
```

### .env File Support

```bash
# .env
REDIS_URL=redis://127.0.0.1:6379
REDIS_PASSWORD=mypassword
REDIS_DB=0
REDIS_MAX_CONNECTIONS=20
```

```rust
// Add to Cargo.toml
// dotenv = "0.15"

use dotenv::dotenv;

#[tokio::main]
async fn main() {
    dotenv().ok(); // Load .env file
    let config = RedisConfig::from_env();
    // ... rest of app
}
```

---

## 10. **Docker Compose Setup** 🐳

### Basic Redis Container

```yaml
# docker-compose.yml
version: "3.8"
services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    command: redis-server --requirepass mypassword
    volumes:
      - redis_data:/data
    networks:
      - app_network

volumes:
  redis_data:

networks:
  app_network:
    driver: bridge
```

### Production Configuration

```yaml
# docker-compose.prod.yml
version: "3.8"
services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    command: >
      redis-server 
      --requirepass ${REDIS_PASSWORD}
      --maxmemory 256mb
      --maxmemory-policy allkeys-lru
      --save 900 1
      --save 300 10
      --save 60 10000
    volumes:
      - redis_data:/data
      - ./redis.conf:/usr/local/etc/redis/redis.conf
    restart: unless-stopped
    networks:
      - app_network
```

### Redis Configuration File

```bash
# redis.conf
bind 127.0.0.1
port 6379
requirepass mypassword

# Memory management
maxmemory 256mb
maxmemory-policy allkeys-lru

# Persistence
save 900 1    # Save if at least 1 key changed in 900 seconds
save 300 10   # Save if at least 10 keys changed in 300 seconds
save 60 10000 # Save if at least 10000 keys changed in 60 seconds

# Logging
loglevel notice
logfile /var/log/redis/redis-server.log

# Security
protected-mode yes
```

---

## 🎯 **Most Valuable Takeaways**

1. **Always use ConnectionManager** for production applications
2. **Use async connections** unless you have a specific reason not to
3. **Pipeline operations** for better performance (batching)
4. **Handle Redis errors** properly - network issues happen!
5. **Use environment variables** for configuration flexibility
6. **Choose the right features** in Cargo.toml to avoid bloat
7. **Implement connection pooling** to avoid connection overhead
8. **Use atomic transactions** (MULTI/EXEC) for data consistency
9. **Monitor memory usage** and set appropriate policies
10. **Always handle reconnection** scenarios gracefully

---

## 🚨 **Common Mistakes to Avoid**

### ❌ Creating Connections in Loops

```rust
// DON'T DO THIS - Very slow!
for i in 0..1000 {
    let conn = client.get_async_connection().await?; // EXPENSIVE!
    conn.set(format!("key{}", i), i).await?;
}
```

### ✅ Reuse Connections

```rust
// DO THIS - Much faster!
let mut conn = client.get_async_connection().await?;
for i in 0..1000 {
    conn.set(format!("key{}", i), i).await?;
}
```

### ❌ Ignoring Errors

```rust
// DON'T DO THIS
let _: () = conn.set("key", "value").await.unwrap(); // Will panic!
```

### ✅ Proper Error Handling

```rust
// DO THIS
match conn.set("key", "value").await {
    Ok(_) => println!("Success"),
    Err(e) => eprintln!("Redis error: {}", e),
}
```

### ❌ Blocking the Async Runtime

```rust
// DON'T DO THIS in async context
let conn = client.get_connection()?; // Synchronous in async function
```

### ✅ Use Async Connections

```rust
// DO THIS
let conn = client.get_async_connection().await?; // Proper async
```

---

## 📊 **Performance Benchmarks**

Based on the project's demo results:

- **Single operations**: ~600-800 ops/second
- **Bulk operations**: ~3,700+ ops/second
- **Atomic updates**: ~250 ops/second (due to consistency overhead)

### Optimization Tips:

1. **Use pipelining** for bulk operations
2. **Batch related operations** together
3. **Choose appropriate data structures** (Sets, Hashes, Lists)
4. **Set TTL** on temporary data to prevent memory leaks
5. **Monitor Redis memory usage** and set eviction policies

---

## 🔗 **Useful Resources**

- [Redis Rust Crate Documentation](https://docs.rs/redis/)
- [Redis Official Documentation](https://redis.io/documentation)
- [Redis Best Practices](https://redis.io/docs/manual/clients-guide/)
- [Tokio Async Runtime](https://tokio.rs/)

---

**Happy coding with Redis and Rust! 🦀🔴**
