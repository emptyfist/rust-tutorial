use chrono::Utc;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::{Message, TopicPartitionList};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info, warn};

#[derive(Serialize, Deserialize, Debug)]
struct MessagePayload {
    id: String,
    content: String,
    timestamp: chrono::DateTime<Utc>,
    counter: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Kafka receiver service...");

    // Create Kafka consumer
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "rust-consumer-group")
        .set("bootstrap.servers", "localhost:9092")
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("enable.auto.commit", "false")
        .set("auto.offset.reset", "earliest")
        .create()?;

    let topic = "rust-messages";
    
    // Subscribe to the topic
    consumer.subscribe(&[topic])?;
    info!("Consumer subscribed to topic: {}", topic);

    let mut message_count = 0u64;

    loop {
        match consumer.recv().await {
            Err(e) => {
                warn!("Kafka consumer error: {}", e);
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Ok(m) => {
                let payload = match m.payload_view::<str>() {
                    None => {
                        warn!("Received message with empty payload");
                        continue;
                    }
                    Some(Ok(s)) => s,
                    Some(Err(e)) => {
                        warn!("Error while deserializing message payload: {:?}", e);
                        continue;
                    }
                };

                match serde_json::from_str::<MessagePayload>(payload) {
                    Ok(message_data) => {
                        message_count += 1;
                        let processing_time = Utc::now();
                        let latency = processing_time
                            .signed_duration_since(message_data.timestamp)
                            .num_milliseconds();

                        info!(
                            "Received message #{}: id={}, content='{}', latency={}ms, total_received={}",
                            message_data.counter,
                            message_data.id,
                            message_data.content,
                            latency,
                            message_count
                        );

                        // Commit the message
                        if let Err(e) = consumer.commit_message(&m, CommitMode::Async) {
                            warn!("Failed to commit message: {}", e);
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse message JSON: {} - payload: {}", e, payload);
                    }
                }
            }
        };
    }
}