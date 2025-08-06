.PHONY: help setup start stop redis-cli demo benchmark test clean logs

# Default target
help:
	@echo "Redis Atomic Operations Demo - Available Commands:"
	@echo "=================================================="
	@echo "  setup     - Start Redis container"
	@echo "  start     - Start Redis and run demo"
	@echo "  stop      - Stop Redis container"
	@echo "  redis-cli - Connect to Redis CLI"
	@echo "  demo      - Run interactive demo"
	@echo "  benchmark - Run performance benchmark"
	@echo "  test      - Run Rust tests"
	@echo "  build     - Build the project"
	@echo "  clean     - Clean up containers and data"
	@echo "  logs      - Show Redis logs"
	@echo ""
	@echo "Quick Start:"
	@echo "  make setup && make demo"

# Setup Redis container
setup:
	@echo "🐳 Starting Redis container..."
	docker-compose up -d redis
	@echo "⏳ Waiting for Redis to be ready..."
	@sleep 3
	@docker-compose exec redis redis-cli ping > /dev/null 2>&1 && echo "✅ Redis is ready!" || echo "❌ Redis failed to start"

# Start everything and run demo
start: setup demo

# Stop containers
stop:
	@echo "🛑 Stopping containers..."
	docker-compose down

# Connect to Redis CLI
redis-cli:
	@echo "🔗 Connecting to Redis CLI (type 'exit' to quit)..."
	docker-compose exec redis redis-cli

# Run the comprehensive demo
demo: build
	@echo "🚀 Running comprehensive atomic operations demo..."
	cargo run --bin demo

# Run CLI tool examples
cli-examples: build
	@echo "📋 Running CLI examples..."
	@echo ""
	@echo "1. Creating sample transactions..."
	cargo run --bin main -- create "demo-relayer-1" 1 "0x1234567890abcdef" "1000000000000000000"
	@sleep 1
	cargo run --bin main -- create "demo-relayer-1" 2 "0xabcdef1234567890" "2000000000000000000"
	@sleep 1
	cargo run --bin main -- create "demo-relayer-2" 1 "0xfedcba0987654321" "500000000000000000"
	@echo ""
	@echo "2. Showing statistics..."
	cargo run --bin main -- stats
	@echo ""
	@echo "3. Listing pending transactions..."
	cargo run --bin main -- list-by-status "demo-relayer-1" "pending"
	@echo ""
	@echo "4. Updating transaction status..."
	@echo "   (Getting transaction by nonce first...)"
	cargo run --bin main -- get-by-nonce "demo-relayer-1" 1
	@echo "   (Note: Copy the transaction ID from above to update it)"

# Run performance benchmark
benchmark: build
	@echo "⚡ Running performance benchmark..."
	cargo run --bin main -- benchmark --count 200

# Run race condition test
race-test: build
	@echo "🏁 Running race condition prevention test..."
	cargo run --bin main -- race-test --concurrent-updates 15

# Run Rust tests
test: build
	@echo "🧪 Running Rust tests..."
	RUST_LOG=debug cargo test -- --test-threads=1

# Build the project
build:
	@echo "🔨 Building project..."
	cargo build --release

# Run with different Redis configurations
redis-cluster:
	@echo "🔗 Setting up Redis cluster (advanced)..."
	@echo "Note: This would require additional docker-compose configuration"
	@echo "Current setup uses single Redis instance for simplicity"

# Monitor Redis operations
monitor:
	@echo "👀 Monitoring Redis operations (Ctrl+C to stop)..."
	docker-compose exec redis redis-cli monitor

# Clean up everything
clean:
	@echo "🧹 Cleaning up..."
	docker-compose down -v
	docker system prune -f --volumes
	cargo clean
	@echo "✅ Cleanup complete!"

# Show Redis logs
logs:
	@echo "📜 Redis logs:"
	docker-compose logs -f redis

# Advanced: Show Redis memory usage
redis-info:
	@echo "📊 Redis instance information:"
	docker-compose exec redis redis-cli info memory
	@echo ""
	@echo "🔑 Key statistics:"
	docker-compose exec redis redis-cli info keyspace

# Debug: Show all Redis keys
redis-keys:
	@echo "🔑 All Redis keys:"
	docker-compose exec redis redis-cli --scan

# Interactive Redis exploration
explore:
	@echo "🔍 Redis Data Explorer"
	@echo "====================="
	@echo "Available commands in Redis CLI:"
	@echo "  KEYS *                     - List all keys"
	@echo "  GET tx:<id>               - Get transaction data"  
	@echo "  SMEMBERS relayers         - List all relayers"
	@echo "  SMEMBERS relayer:<id>:status:<status> - List transactions by status"
	@echo "  GET relayer:<id>:nonce:<n> - Get transaction by nonce"
	@echo "  TYPE <key>                - Show key type"
	@echo "  TTL <key>                 - Show key expiration"
	@echo ""
	docker-compose exec redis redis-cli

# Development helpers
dev-setup: setup
	@echo "🛠️  Development environment ready!"
	@echo ""
	@echo "Useful development commands:"
	@echo "  make demo          - Run full demo"
	@echo "  make cli-examples  - Try CLI commands"  
	@echo "  make test          - Run tests"
	@echo "  make benchmark     - Performance test"
	@echo "  make race-test     - Race condition test"
	@echo "  make explore       - Explore Redis data"
	@echo "  make logs          - View Redis logs"

# Quick demo with cleanup
quick-demo: setup demo
	@echo ""
	@echo "Demo complete! Redis container is still running."
	@echo "Run 'make stop' to stop it, or 'make explore' to examine the data."

# Comprehensive test suite
test-all: setup build test benchmark race-test
	@echo ""
	@echo "🎉 All tests completed successfully!"
	@echo "✅ Unit tests passed"
	@echo "✅ Performance benchmark completed"  
	@echo "✅ Race condition prevention verified"

# Show project structure
structure:
	@echo "📁 Project Structure:"
	@echo "===================="
	@tree -I 'target|.git' -a || find . -type f -name "*.rs" -o -name "*.toml" -o -name "*.yml" -o -name "Makefile" | grep -v target | sort