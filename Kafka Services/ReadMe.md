# Kafka Rust Services

A high-performance Kafka producer and consumer implementation in Rust, featuring real-time message streaming with configurable intervals and comprehensive monitoring.

## 🚀 Features

- **High-throughput Producer**: Sends messages every 100ms with unique IDs and timestamps
- **Scalable Consumer**: Supports consumer groups for load balancing
- **Real-time Monitoring**: Latency tracking and message processing statistics
- **Robust Error Handling**: Comprehensive error handling with retry logic
- **Structured Logging**: Detailed tracing for debugging and monitoring
- **Docker Integration**: Seamless integration with Dockerized Kafka

## 📋 Prerequisites

- **Rust** (1.70+)
- **Docker & Docker Compose**
- **Git**

## 🏗️ Project Structure

```
kafka-rust-services/
├── docker-compose.yml          # Kafka setup
├── Cargo.toml                  # Workspace configuration
├── run-services.sh             # Automation script
├── sender/
│   ├── Cargo.toml
│   └── src/
│       └── main.rs             # Producer service
└── receiver/
    ├── Cargo.toml
    └── src/
        └── main.rs             # Consumer service
```

## 🔧 Installation & Setup

### 1. Start Kafka Infrastructure

```bash
docker-compose up -d
```

This will start a Kafka broker using KRaft mode (no Zookeeper required) on `localhost:9092`.

### 2. Build Services

```bash
cargo build --release
```

### 3. Create Kafka Topic

```bash
docker exec kafka kafka-topics.sh \
    --create \
    --topic rust-messages \
    --bootstrap-server localhost:9092 \
    --partitions 3 \
    --replication-factor 1 \
    --if-not-exists
```

## 🚀 Quick Start

### Option 1: Automated Script (Recommended)

```bash
chmod +x run-services.sh
./run-services.sh
```

This script will:

- Check and start Kafka if needed
- Create the required topic
- Build the services
- Run receiver and sender together

### Option 2: Manual Execution

**Terminal 1 - Start Receiver:**

```bash
cargo run --bin receiver
```

**Terminal 2 - Start Sender:**

```bash
cargo run --bin sender
```

## 📊 Service Details

### Sender Service (`sender/`)

- **Interval**: Sends messages every 100ms
- **Message Format**: JSON with ID, content, timestamp, and counter
- **Features**:
  - UUID-based message IDs
  - Monotonic counter tracking
  - ISO 8601 timestamps
  - Configurable retry logic
  - Comprehensive error logging

**Sample Message:**

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "content": "Hello from Rust sender! Message #42",
  "timestamp": "2025-07-30T10:30:45.123Z",
  "counter": 42
}
```

### Receiver Service (`receiver/`)

- **Consumer Group**: `rust-consumer-group`
- **Offset Management**: Manual commits for reliability
- **Features**:
  - Latency calculation
  - Message processing counters
  - Graceful error handling
  - Partition-aware processing

**Sample Output:**

```
INFO receiver: Received message #42: id=550e8400-e29b-41d4-a716-446655440000, content='Hello from Rust sender! Message #42', latency=15ms, total_received=42
```

## 🔧 Configuration

### Kafka Configuration

The services are configured with production-ready settings:

**Producer Settings:**

- `acks=all`: Wait for all replicas to acknowledge
- `retries=3`: Automatic retry on failures
- `message.timeout.ms=5000`: 5-second timeout

**Consumer Settings:**

- `auto.offset.reset=earliest`: Start from beginning if no offset
- `enable.auto.commit=false`: Manual offset management
- `session.timeout.ms=6000`: 6-second session timeout

### Environment Variables

You can customize behavior using environment variables:

```bash
# Kafka connection
export KAFKA_BROKERS=localhost:9092
export KAFKA_TOPIC=rust-messages

# Producer settings
export SEND_INTERVAL_MS=100
export MESSAGE_TIMEOUT_MS=5000

# Consumer settings
export CONSUMER_GROUP=rust-consumer-group
```

## 🛠️ Development

### Running Tests

```bash
cargo test
```

### Code Formatting

```bash
cargo fmt
```

### Linting

```bash
cargo clippy
```

### Viewing Logs

Both services use structured logging. To see detailed logs:

```bash
RUST_LOG=debug cargo run --bin sender
RUST_LOG=debug cargo run --bin receiver
```

## 📈 Monitoring & Troubleshooting

### Kafka Topic Information

```bash
# List all topics
docker exec kafka kafka-topics.sh --list --bootstrap-server localhost:9092

# Describe topic details
docker exec kafka kafka-topics.sh --describe --topic rust-messages --bootstrap-server localhost:9092

# View consumer group status
docker exec kafka kafka-consumer-groups.sh --bootstrap-server localhost:9092 --describe --group rust-consumer-group
```

### Performance Monitoring

The services provide built-in metrics:

- **Sender**: Messages sent per second, failure rates
- **Receiver**: Processing latency, throughput, total messages processed

### Common Issues

**Issue**: Connection refused to Kafka

```bash
# Check if Kafka is running
docker ps | grep kafka

# Check Kafka logs
docker logs kafka
```

**Issue**: Topic doesn't exist

```bash
# Recreate topic
docker exec kafka kafka-topics.sh --create --topic rust-messages --bootstrap-server localhost:9092 --partitions 3 --replication-factor 1
```

**Issue**: Consumer lag

```bash
# Check consumer group lag
docker exec kafka kafka-consumer-groups.sh --bootstrap-server localhost:9092 --describe --group rust-consumer-group
```

## 🔄 Scaling

### Running Multiple Consumers

```bash
# Terminal 1
cargo run --bin receiver

# Terminal 2
cargo run --bin receiver

# Terminal 3
cargo run --bin receiver
```

Multiple consumers will automatically load balance within the same consumer group.

### Running Multiple Producers

```bash
# Terminal 1
PRODUCER_ID=producer-1 cargo run --bin sender

# Terminal 2
PRODUCER_ID=producer-2 cargo run --bin sender
```

## 📦 Dependencies

- **rdkafka**: Kafka client library
- **tokio**: Async runtime
- **serde**: Serialization framework
- **chrono**: Date and time handling
- **uuid**: UUID generation
- **tracing**: Structured logging
