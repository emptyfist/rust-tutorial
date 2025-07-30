use chrono::Utc;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    id: String,
    content: String,
    timestamp: chrono::DateTime<Utc>,
    counter: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Kafka sender service...");

    // Create Kafka producer
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092")
        .set("message.timeout.ms", "5000")
        .set("acks", "all")
        .set("retries", "3")
        .create()?;

    let topic = "rust-messages";
    let mut counter = 0u64;

    info!("Producer created successfully. Starting to send messages...");

    loop {
        counter += 1;
        
        let message = Message {
            id: Uuid::new_v4().to_string(),
            content: format!("Hello from Rust sender! Message #{}", counter),
            timestamp: Utc::now(),
            counter,
        };

        let payload = match serde_json::to_string(&message) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize message: {}", e);
                continue;
            }
        };

        let record = FutureRecord::to(topic)
            .key(&message.id)
            .payload(&payload);

        match producer.send(record, Duration::from_secs(5)).await {
            Ok(delivery) => {
                info!(
                    "Message sent successfully: partition={}, offset={}, counter={}",
                    delivery.0, delivery.1, counter
                );
            }
            Err((kafka_error, _)) => {
                warn!("Failed to send message {}: {}", counter, kafka_error);
            }
        }

        // Wait 100ms before sending next message
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}