#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Setting up Kafka Rust services...${NC}"

# Function to check if Kafka is running
check_kafka() {
    echo -e "${YELLOW}Checking if Kafka is running...${NC}"
    if ! docker ps | grep -q kafka; then
        echo -e "${YELLOW}Kafka container not found. Starting with docker-compose...${NC}"
        docker-compose up -d
        sleep 5
    else
        echo -e "${GREEN}Kafka is already running${NC}"
    fi
}

# Function to create topic
create_topic() {
    echo -e "${YELLOW}Creating Kafka topic 'rust-messages'...${NC}"
    docker exec kafka kafka-topics.sh \
        --create \
        --topic rust-messages \
        --bootstrap-server localhost:9092 \
        --partitions 3 \
        --replication-factor 1 \
        --if-not-exists
}

# Function to build services
build_services() {
    echo -e "${YELLOW}Building Rust services...${NC}"
    cargo build --release
}

# Function to run receiver in background
run_receiver() {
    echo -e "${YELLOW}Starting receiver service...${NC}"
    cargo run --bin receiver &
    RECEIVER_PID=$!
    echo "Receiver PID: $RECEIVER_PID"
}

# Function to run sender
run_sender() {
    echo -e "${YELLOW}Starting sender service...${NC}"
    sleep 2  # Give receiver time to start
    cargo run --bin sender
}

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Shutting down services...${NC}"
    if [ ! -z "$RECEIVER_PID" ]; then
        kill $RECEIVER_PID 2>/dev/null
    fi
    exit 0
}

# Set up signal handling
trap cleanup SIGINT SIGTERM

# Main execution
check_kafka
create_topic
build_services
run_receiver
run_sender