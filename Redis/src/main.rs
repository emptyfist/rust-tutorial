use chrono::Utc;
use clap::{Parser, Subcommand};
use redis_atomic_demo::{TransactionRepository, TransactionRepoModel, TransactionStatus};

#[derive(Parser)]
#[command(name = "redis-atomic-demo")]
#[command(about = "Redis Atomic Operations Demo - Transaction Management System")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(short, long, default_value = "redis://127.0.0.1:6379")]
    redis_url: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new transaction
    Create {
        relayer_id: String,
        nonce: u64,
        to_address: String,
        value: String,
        #[arg(long, default_value = "20000000000")]
        gas_price: u64,
        #[arg(long, default_value = "21000")]
        gas_limit: u64,
    },
    /// Get transaction by ID
    Get {
        tx_id: String,
    },
    /// Update transaction status
    Update {
        tx_id: String,
        status: String,
        #[arg(long)]
        hash: Option<String>,
    },
    /// List transactions by status
    ListByStatus {
        relayer_id: String,
        status: String,
    },
    /// Get transaction by nonce
    GetByNonce {
        relayer_id: String,
        nonce: u64,
    },
    /// Delete transaction
    Delete {
        tx_id: String,
    },
    /// Show repository statistics
    Stats,
    /// Clear all data
    Clear,
    /// Run performance benchmark
    Benchmark {
        #[arg(long, default_value = "100")]
        count: usize,
    },
    /// Demonstrate race condition prevention
    RaceTest {
        #[arg(long, default_value = "10")]
        concurrent_updates: usize,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    let cli = Cli::parse();
    let repo = TransactionRepository::new(&cli.redis_url)?;
    
    match cli.command {
        Commands::Create { relayer_id, nonce, to_address, value, gas_price, gas_limit } => {
            let tx = TransactionRepoModel::new(relayer_id, nonce, to_address, value, gas_price, gas_limit);
            let created = repo.create(tx).await?;
            println!("✅ Created transaction: {}", created.id);
            println!("   Relayer: {}", created.relayer_id);
            println!("   Nonce: {}", created.nonce);
            println!("   Status: {}", created.status);
            println!("   Created at: {}", created.created_at);
        },
        
        Commands::Get { tx_id } => {
            match repo.get_by_id(&tx_id).await {
                Ok(tx) => {
                    println!("📄 Transaction Details:");
                    println!("   ID: {}", tx.id);
                    println!("   Relayer: {}", tx.relayer_id);
                    println!("   Nonce: {}", tx.nonce);
                    println!("   Status: {}", tx.status);
                    println!("   To: {}", tx.to_address);
                    println!("   Value: {}", tx.value);
                    println!("   Gas Price: {}", tx.gas_price);
                    println!("   Gas Limit: {}", tx.gas_limit);
                    if let Some(hash) = &tx.hash {
                        println!("   Hash: {}", hash);
                    }
                    println!("   Created: {}", tx.created_at);
                    println!("   Updated: {}", tx.updated_at);
                },
                Err(e) => {
                    eprintln!("❌ Error: {}", e);
                    std::process::exit(1);
                }
            }
        },
        
        Commands::Update { tx_id, status, hash } => {
            let mut tx = repo.get_by_id(&tx_id).await?;
            
            tx.status = match status.to_lowercase().as_str() {
                "pending" => TransactionStatus::Pending,
                "confirmed" => TransactionStatus::Confirmed,
                "failed" => TransactionStatus::Failed,
                "cancelled" => TransactionStatus::Cancelled,
                _ => {
                    eprintln!("❌ Invalid status. Use: pending, confirmed, failed, cancelled");
                    std::process::exit(1);
                }
            };
            
            if let Some(h) = hash {
                tx.hash = Some(h);
            }
            tx.updated_at = Utc::now();
            
            let updated = repo.update(tx).await?;
            println!("✅ Updated transaction {} to status: {}", updated.id, updated.status);
        },
        
        Commands::ListByStatus { relayer_id, status } => {
            let status_enum = match status.to_lowercase().as_str() {
                "pending" => TransactionStatus::Pending,
                "confirmed" => TransactionStatus::Confirmed,
                "failed" => TransactionStatus::Failed,
                "cancelled" => TransactionStatus::Cancelled,
                _ => {
                    eprintln!("❌ Invalid status. Use: pending, confirmed, failed, cancelled");
                    std::process::exit(1);
                }
            };
            
            let transactions = repo.get_by_status(&relayer_id, &status_enum).await?;
            
            if transactions.is_empty() {
                println!("📭 No {} transactions found for relayer {}", status, relayer_id);
            } else {
                println!("📋 {} transactions with status '{}' for relayer {}:", 
                    transactions.len(), status, relayer_id);
                for tx in transactions {
                    println!("   {} | Nonce: {} | Value: {} | Created: {}", 
                        tx.id, tx.nonce, tx.value, tx.created_at.format("%Y-%m-%d %H:%M:%S"));
                }
            }
        },
        
        Commands::GetByNonce { relayer_id, nonce } => {
            match repo.get_by_nonce(&relayer_id, nonce).await? {
                Some(tx) => {
                    println!("🎯 Found transaction for nonce {}:", nonce);
                    println!("   ID: {}", tx.id);
                    println!("   Status: {}", tx.status);
                    println!("   Value: {}", tx.value);
                    println!("   Created: {}", tx.created_at);
                },
                None => {
                    println!("❌ No transaction found for relayer {} with nonce {}", relayer_id, nonce);
                }
            }
        },
        
        Commands::Delete { tx_id } => {
            repo.delete(&tx_id).await?;
            println!("🗑️ Deleted transaction: {}", tx_id);
        },
        
        Commands::Stats => {
            let stats = repo.get_stats().await?;
            println!("📊 Repository Statistics:");
            println!("   Total Relayers: {}", stats.get("relayers").unwrap_or(&0));
            println!("   Pending Transactions: {}", stats.get("status_pending").unwrap_or(&0));
            println!("   Confirmed Transactions: {}", stats.get("status_confirmed").unwrap_or(&0));
            println!("   Failed Transactions: {}", stats.get("status_failed").unwrap_or(&0));
            println!("   Cancelled Transactions: {}", stats.get("status_cancelled").unwrap_or(&0));
            
            let total_txs = stats.get("status_pending").unwrap_or(&0) +
                          stats.get("status_confirmed").unwrap_or(&0) +
                          stats.get("status_failed").unwrap_or(&0) +
                          stats.get("status_cancelled").unwrap_or(&0);
            println!("   Total Transactions: {}", total_txs);
        },
        
        Commands::Clear => {
            print!("⚠️  Are you sure you want to clear ALL data? (y/N): ");
            use std::io::{self, Write};
            io::stdout().flush().unwrap();
            
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            
            if input.trim().to_lowercase() == "y" {
                repo.drop_all_entries().await?;
                println!("🧹 All data cleared atomically");
            } else {
                println!("❌ Operation cancelled");
            }
        },
        
        Commands::Benchmark { count } => {
            println!("🏃 Running benchmark: Creating {} transactions...", count);
            
            let start = std::time::Instant::now();
            
            // Atomic batch creation
            let mut handles = Vec::new();
            for i in 0..count {
                let repo = TransactionRepository::new(&cli.redis_url)?;
                let handle = tokio::spawn(async move {
                    let tx = TransactionRepoModel::new(
                        format!("bench-relayer-{}", i % 10), // 10 different relayers
                        i as u64,
                        "0xbenchmark".to_string(),
                        "1000000000000000000".to_string(),
                        20000000000,
                        21000,
                    );
                    repo.create(tx).await
                });
                handles.push(handle);
            }
            
            let mut successes = 0;
            let mut errors = 0;
            
            for handle in handles {
                match handle.await.unwrap() {
                    Ok(_) => successes += 1,
                    Err(_) => errors += 1,
                }
            }
            
            let duration = start.elapsed();
            
            println!("📈 Benchmark Results:");
            println!("   Total Operations: {}", count);
            println!("   Successful: {}", successes);
            println!("   Errors: {}", errors);
            println!("   Duration: {:?}", duration);
            println!("   Operations/sec: {:.2}", count as f64 / duration.as_secs_f64());
        },
        
        Commands::RaceTest { concurrent_updates } => {
            println!("🏁 Running race condition test with {} concurrent updates...", concurrent_updates);
            
            // First create a test transaction
            let tx = TransactionRepoModel::new(
                "race-test-relayer".to_string(),
                999,
                "0xracetest".to_string(),
                "1000000000000000000".to_string(),
                20000000000,
                21000,
            );
            
            let created = repo.create(tx).await?;
            println!("✅ Created test transaction: {}", created.id);
            
            let tx_id = created.id.clone();
            let redis_url = cli.redis_url.clone();
            
            // Launch concurrent updates
            let mut handles = Vec::new();
            
            for i in 0..concurrent_updates {
                let tx_id = tx_id.clone();
                let redis_url = redis_url.clone();
                
                let handle = tokio::spawn(async move {
                    let repo = TransactionRepository::new(&redis_url).unwrap();
                    
                    // Simulate race condition: multiple processes trying to update same transaction
                    let mut tx = repo.get_by_id(&tx_id).await.unwrap();
                    
                    // Each update changes status in a cycle
                    tx.status = match i % 4 {
                        0 => TransactionStatus::Pending,
                        1 => TransactionStatus::Confirmed,
                        2 => TransactionStatus::Failed,
                        3 => TransactionStatus::Cancelled,
                        _ => unreachable!(),
                    };
                    
                    tx.hash = Some(format!("0xrace{:04x}", i));
                    tx.updated_at = Utc::now();
                    
                    // Random delay to increase chance of race conditions
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        rand::random::<u64>() % 50
                    )).await;
                    
                    repo.update(tx).await
                });
                
                handles.push(handle);
            }
            
            // Wait for all updates
            let mut successes = 0;
            let mut errors = 0;
            
            for (i, handle) in handles.into_iter().enumerate() {
                match handle.await.unwrap() {
                    Ok(updated_tx) => {
                        println!("   ✅ Update {} succeeded: status = {}", i, updated_tx.status);
                        successes += 1;
                    },
                    Err(e) => {
                        println!("   ❌ Update {} failed: {}", i, e);
                        errors += 1;
                    }
                }
            }
            
            // Check final state
            let final_tx = repo.get_by_id(&tx_id).await?;
            println!("\n🎯 Final Transaction State:");
            println!("   Status: {}", final_tx.status);
            println!("   Hash: {}", final_tx.hash.unwrap_or("None".to_string()));
            println!("   Updated: {}", final_tx.updated_at);
            
            // Verify index consistency
            let pending = repo.get_by_status("race-test-relayer", &TransactionStatus::Pending).await?;
            let confirmed = repo.get_by_status("race-test-relayer", &TransactionStatus::Confirmed).await?;
            let failed = repo.get_by_status("race-test-relayer", &TransactionStatus::Failed).await?;
            let cancelled = repo.get_by_status("race-test-relayer", &TransactionStatus::Cancelled).await?;
            
            println!("\n🔍 Index Consistency Check:");
            println!("   Pending: {} transactions", pending.len());
            println!("   Confirmed: {} transactions", confirmed.len());
            println!("   Failed: {} transactions", failed.len());
            println!("   Cancelled: {} transactions", cancelled.len());
            
            let total_in_indexes = pending.len() + confirmed.len() + failed.len() + cancelled.len();
            
            if total_in_indexes == 1 {
                println!("   ✅ ATOMICITY MAINTAINED: Transaction appears in exactly 1 status index");
            } else {
                println!("   ❌ ATOMICITY VIOLATED: Transaction appears in {} status indexes!", total_in_indexes);
            }
            
            println!("\n📊 Race Test Results:");
            println!("   Concurrent Updates: {}", concurrent_updates);
            println!("   Successful: {}", successes);
            println!("   Errors: {}", errors);
            println!("   Data Consistency: {}", if total_in_indexes == 1 { "✅ PRESERVED" } else { "❌ VIOLATED" });
            
            // Cleanup
            repo.delete(&tx_id).await?;
        },
    }
    
    Ok(())
}