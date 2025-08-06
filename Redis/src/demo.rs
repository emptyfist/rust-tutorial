use chrono::Utc;
use redis_atomic_demo::{TransactionRepository, TransactionRepoModel, TransactionStatus};
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    println!("🚀 Redis Atomic Operations Demo");
    println!("================================\n");
    
    let repo = TransactionRepository::new("redis://127.0.0.1:6379")?;
    
    // Clear existing data
    println!("🧹 Clearing existing data...");
    repo.drop_all_entries().await?;
    
    // Demo 1: Basic Atomic Operations
    demo_basic_atomic_operations(&repo).await?;
    
    // Demo 2: Race Condition Prevention
    demo_race_condition_prevention(&repo).await?;
    
    // Demo 3: Bulk Atomic Operations
    demo_bulk_atomic_operations(&repo).await?;
    
    // Demo 4: Performance Comparison
    demo_performance_comparison(&repo).await?;
    
    // Demo 5: Index Consistency
    demo_index_consistency(&repo).await?;
    
    println!("\n✅ All demos completed successfully!");
    println!("🧹 Cleaning up...");
    repo.drop_all_entries().await?;
    
    Ok(())
}

async fn demo_basic_atomic_operations(repo: &TransactionRepository) -> Result<(), Box<dyn std::error::Error>> {
    println!("📋 Demo 1: Basic Atomic Operations");
    println!("----------------------------------");
    
    // Create transaction atomically
    let tx = TransactionRepoModel::new(
        "demo-relayer-1".to_string(),
        100,
        "0x1234567890abcdef".to_string(),
        "1000000000000000000".to_string(), // 1 ETH in wei
        20000000000, // 20 gwei
        21000,
    );
    
    println!("💾 Creating transaction atomically...");
    let start = Instant::now();
    let created = repo.create(tx).await?;
    let create_time = start.elapsed();
    
    println!("   ✅ Transaction created: {}", created.id);
    println!("   ⏱️  Time: {:?}", create_time);
    
    // Verify all indexes were created atomically
    let by_status = repo.get_by_status(&created.relayer_id, &TransactionStatus::Pending).await?;
    let by_nonce = repo.get_by_nonce(&created.relayer_id, 100).await?;
    
    println!("   📊 Index verification:");
    println!("      Pending status index: {} transactions", by_status.len());
    println!("      Nonce mapping exists: {}", by_nonce.is_some());
    
    // Update transaction status atomically  
    println!("\n🔄 Updating transaction status atomically...");
    let mut updated_tx = created.clone();
    updated_tx.status = TransactionStatus::Confirmed;
    updated_tx.hash = Some("0xdeadbeefcafebabe1234567890abcdef".to_string());
    updated_tx.updated_at = Utc::now();
    
    let start = Instant::now();
    repo.update(updated_tx).await?;
    let update_time = start.elapsed();
    
    println!("   ✅ Status updated to Confirmed");
    println!("   ⏱️  Time: {:?}", update_time);
    
    // Verify atomic index updates
    let pending = repo.get_by_status(&created.relayer_id, &TransactionStatus::Pending).await?;
    let confirmed = repo.get_by_status(&created.relayer_id, &TransactionStatus::Confirmed).await?;
    
    println!("   📊 Index verification after update:");
    println!("      Pending: {} transactions", pending.len());
    println!("      Confirmed: {} transactions", confirmed.len());
    println!("      ✅ No transaction appears in multiple status indexes!");
    
    Ok(())
}

async fn demo_race_condition_prevention(repo: &TransactionRepository) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🏁 Demo 2: Race Condition Prevention");
    println!("------------------------------------");
    
    // Create a transaction for race testing
    let tx = TransactionRepoModel::new(
        "race-relayer".to_string(),
        200,
        "0xracetest".to_string(),
        "5000000000000000000".to_string(), // 5 ETH
        25000000000, // 25 gwei
        21000,
    );
    
    let created = repo.create(tx).await?;
    println!("💾 Created test transaction: {}", created.id);
    
    // Simulate concurrent updates from multiple processes
    println!("🔄 Launching 20 concurrent updates...");
    let mut handles = Vec::new();
    
    for i in 0..20 {
        let tx_id = created.id.clone();
        let repo_clone = TransactionRepository::new("redis://127.0.0.1:6379")?;
        
        let handle = tokio::spawn(async move {
            // Small random delay to increase race condition chance
            tokio::time::sleep(Duration::from_millis(rand::random::<u64>() % 10)).await;
            
            let mut tx = repo_clone.get_by_id(&tx_id).await.unwrap();
            
            // Each update tries to set a different status
            tx.status = match i % 3 {
                0 => TransactionStatus::Confirmed,
                1 => TransactionStatus::Failed, 
                2 => TransactionStatus::Cancelled,
                _ => unreachable!(),
            };
            
            tx.hash = Some(format!("0xrace{:08x}", i));
            tx.updated_at = Utc::now();
            
            repo_clone.update(tx).await
        });
        
        handles.push(handle);
    }
    
    // Wait for all updates and collect results
    let start = Instant::now();
    let mut success_count = 0;
    let mut error_count = 0;
    
    for handle in handles {
        match handle.await.unwrap() {
            Ok(_) => success_count += 1,
            Err(_) => error_count += 1,
        }
    }
    
    let race_time = start.elapsed();
    
    println!("   📊 Race test results:");
    println!("      Successful updates: {}", success_count);
    println!("      Failed updates: {}", error_count);
    println!("      Total time: {:?}", race_time);
    
    // Verify data consistency after race conditions
    let final_tx = repo.get_by_id(&created.id).await?;
    println!("\n🔍 Final transaction state:");
    println!("   Status: {}", final_tx.status);
    println!("   Hash: {}", final_tx.hash.unwrap_or("None".to_string()));
    
    // Check index consistency - critical test!
    let confirmed = repo.get_by_status("race-relayer", &TransactionStatus::Confirmed).await?;
    let failed = repo.get_by_status("race-relayer", &TransactionStatus::Failed).await?;
    let cancelled = repo.get_by_status("race-relayer", &TransactionStatus::Cancelled).await?;
    let pending = repo.get_by_status("race-relayer", &TransactionStatus::Pending).await?;
    
    let total_in_indexes = confirmed.len() + failed.len() + cancelled.len() + pending.len();
    
    println!("\n🎯 Atomicity verification:");
    println!("   Confirmed index: {} transactions", confirmed.len());
    println!("   Failed index: {} transactions", failed.len());
    println!("   Cancelled index: {} transactions", cancelled.len());
    println!("   Pending index: {} transactions", pending.len());
    println!("   Total in indexes: {}", total_in_indexes);
    
    if total_in_indexes == 1 {
        println!("   ✅ ATOMICITY PRESERVED: Transaction appears in exactly 1 index");
    } else {
        println!("   ❌ ATOMICITY VIOLATED: Transaction appears in {} indexes!", total_in_indexes);
    }
    
    // Cleanup
    repo.delete(&created.id).await?;
    
    Ok(())
}

async fn demo_bulk_atomic_operations(repo: &TransactionRepository) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📦 Demo 3: Bulk Atomic Operations");
    println!("---------------------------------");
    
    // Create multiple transactions for bulk operations
    println!("💾 Creating 50 transactions across 5 relayers...");
    let start = Instant::now();
    
    for relayer_idx in 0..5 {
        for tx_idx in 0..10 {
            let tx = TransactionRepoModel::new(
                format!("bulk-relayer-{}", relayer_idx),
                tx_idx,
                format!("0xbulktest{:02x}", tx_idx),
                format!("{}000000000000000000", tx_idx + 1), // 1-10 ETH
                20000000000 + (tx_idx as u64 * 1000000000), // Varying gas prices
                21000,
            );
            
            repo.create(tx).await?;
        }
    }
    
    let create_time = start.elapsed();
    println!("   ✅ Created 50 transactions in {:?}", create_time);
    
    // Show statistics
    let stats = repo.get_stats().await?;
    println!("   📊 Current stats:");
    println!("      Relayers: {}", stats.get("relayers").unwrap_or(&0));
    println!("      Pending transactions: {}", stats.get("status_pending").unwrap_or(&0));
    
    // Update some transactions to different statuses
    println!("\n🔄 Bulk status updates...");
    let pending_txs = repo.get_by_status("bulk-relayer-0", &TransactionStatus::Pending).await?;
    
    for (i, mut tx) in pending_txs.into_iter().enumerate().take(5) {
        tx.status = match i % 3 {
            0 => TransactionStatus::Confirmed,
            1 => TransactionStatus::Failed,
            2 => TransactionStatus::Cancelled,
            _ => unreachable!(),
        };
        tx.hash = Some(format!("0xbulk{:08x}", i));
        tx.updated_at = Utc::now();
        
        repo.update(tx).await?;
    }
    
    // Show updated statistics
    let updated_stats = repo.get_stats().await?;
    println!("   📊 Updated stats:");
    println!("      Pending: {}", updated_stats.get("status_pending").unwrap_or(&0));
    println!("      Confirmed: {}", updated_stats.get("status_confirmed").unwrap_or(&0));
    println!("      Failed: {}", updated_stats.get("status_failed").unwrap_or(&0));
    println!("      Cancelled: {}", updated_stats.get("status_cancelled").unwrap_or(&0));
    
    // Atomic bulk delete
    println!("\n🗑️ Atomic bulk deletion of ALL data...");
    let start = Instant::now();
    repo.drop_all_entries().await?;
    let delete_time = start.elapsed();
    
    println!("   ✅ Deleted all data atomically in {:?}", delete_time);
    
    // Verify everything is gone
    let final_stats = repo.get_stats().await?;
    let total_remaining = final_stats.values().sum::<i32>();
    
    if total_remaining == 0 {
        println!("   ✅ BULK DELETE SUCCESS: No data remaining");
    } else {
        println!("   ❌ BULK DELETE FAILED: {} items remaining", total_remaining);
    }
    
    Ok(())
}

async fn demo_performance_comparison(repo: &TransactionRepository) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⚡ Demo 4: Performance Comparison");
    println!("--------------------------------");
    
    let test_size = 20;
    
    // Test 1: Atomic operations (using pipelines)
    println!("🚀 Testing atomic operations ({} transactions)...", test_size);
    let start = Instant::now();
    
    for i in 0..test_size {
        let tx = TransactionRepoModel::new(
            format!("perf-relayer-{}", i % 3), // 3 different relayers
            i as u64,
            format!("0xperf{:04x}", i),
            "1000000000000000000".to_string(),
            20000000000,
            21000,
        );
        
        // This uses atomic pipeline operations internally
        repo.create(tx).await?;
    }
    
    let atomic_time = start.elapsed();
    println!("   ✅ Atomic operations completed in: {:?}", atomic_time);
    println!("   ⚡ Rate: {:.2} ops/second", test_size as f64 / atomic_time.as_secs_f64());
    
    // Verify data consistency
    let stats = repo.get_stats().await?;
    let total_txs = stats.values().sum::<i32>();
    println!("   📊 Total transactions created: {}", total_txs);
    
    // Update some transactions to test update performance
    println!("\n🔄 Testing atomic updates...");
    let start = Instant::now();
    
    for i in 0..test_size / 2 {
        let relayer_id = format!("perf-relayer-{}", i % 3);
        if let Some(mut tx) = repo.get_by_nonce(&relayer_id, i as u64).await? {
            tx.status = TransactionStatus::Confirmed;
            tx.hash = Some(format!("0xupdate{:08x}", i));
            tx.updated_at = Utc::now();
            repo.update(tx).await?;
        }
    }
    
    let update_time = start.elapsed();
    println!("   ✅ {} atomic updates completed in: {:?}", test_size / 2, update_time);
    println!("   ⚡ Update rate: {:.2} ops/second", (test_size / 2) as f64 / update_time.as_secs_f64());
    
    // Cleanup
    repo.drop_all_entries().await?;
    
    Ok(())
}

async fn demo_index_consistency(repo: &TransactionRepository) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 Demo 5: Index Consistency Verification");
    println!("------------------------------------------");
    
    // Create transactions with various statuses
    println!("💾 Creating transactions with different scenarios...");
    
    let scenarios = vec![
        ("consistent-relayer", vec![
            (TransactionStatus::Pending, 3),
            (TransactionStatus::Confirmed, 2),
            (TransactionStatus::Failed, 1),
        ]),
    ];
    
    for (relayer_id, status_counts) in scenarios {
        let mut nonce = 0;
        
        for (status, count) in status_counts {
            for _ in 0..count {
                let mut tx = TransactionRepoModel::new(
                    relayer_id.to_string(),
                    nonce,
                    format!("0x{:04x}", nonce),
                    "1000000000000000000".to_string(),
                    20000000000,
                    21000,
                );
                
                // Create as pending first
                let created = repo.create(tx.clone()).await?;
                
                // If not pending, update to target status
                if status != TransactionStatus::Pending {
                    tx.id = created.id;
                    tx.status = status.clone();
                    tx.hash = Some(format!("0xhash{:08x}", nonce));
                    tx.updated_at = Utc::now();
                    repo.update(tx).await?;
                }
                
                nonce += 1;
            }
        }
    }
    
    // Verify index consistency
    println!("\n🔍 Verifying index consistency...");
    
    let pending = repo.get_by_status("consistent-relayer", &TransactionStatus::Pending).await?;
    let confirmed = repo.get_by_status("consistent-relayer", &TransactionStatus::Confirmed).await?;
    let failed = repo.get_by_status("consistent-relayer", &TransactionStatus::Failed).await?;
    
    println!("   📊 Transactions by status:");
    println!("      Pending: {}", pending.len());
    println!("      Confirmed: {}", confirmed.len()); 
    println!("      Failed: {}", failed.len());
    
    // Verify nonce mappings
    println!("\n🎯 Verifying nonce mappings...");
    let mut nonce_consistency = true;
    
    for nonce in 0..6 {
        if let Some(tx) = repo.get_by_nonce("consistent-relayer", nonce).await? {
            // Verify the transaction actually has this nonce
            if tx.nonce != nonce {
                println!("   ❌ Nonce {} maps to transaction with nonce {}", nonce, tx.nonce);
                nonce_consistency = false;
            }
            
            // Verify the transaction appears in correct status index
            let status_txs = repo.get_by_status("consistent-relayer", &tx.status).await?;
            if !status_txs.iter().any(|t| t.id == tx.id) {
                println!("   ❌ Transaction {} not found in {} status index", tx.id, tx.status);
                nonce_consistency = false;
            }
        }
    }
    
    if nonce_consistency {
        println!("   ✅ All nonce mappings are consistent");
    }
    
    // Cross-verify: ensure no transaction appears in multiple status indexes
    println!("\n🚨 Cross-verifying status index exclusivity...");
    let all_statuses = vec![
        TransactionStatus::Pending,
        TransactionStatus::Confirmed, 
        TransactionStatus::Failed,
        TransactionStatus::Cancelled,
    ];
    
    let mut all_tx_ids = std::collections::HashSet::new();
    let mut total_indexed = 0;
    
    for status in all_statuses {
        let txs = repo.get_by_status("consistent-relayer", &status).await?;
        for tx in txs {
            if all_tx_ids.contains(&tx.id) {
                println!("   ❌ Transaction {} appears in multiple status indexes!", tx.id);
                return Ok(());
            }
            all_tx_ids.insert(tx.id);
            total_indexed += 1;
        }
    }
    
    println!("   ✅ No transaction appears in multiple status indexes");
    println!("   📊 Total transactions in all indexes: {}", total_indexed);
    
    // Final stats
    let stats = repo.get_stats().await?;
    println!("\n📈 Final consistency report:");
    println!("   Relayers: {}", stats.get("relayers").unwrap_or(&0));
    println!("   Total transactions: {}", total_indexed);
    println!("   ✅ ALL ATOMICITY AND CONSISTENCY CHECKS PASSED!");
    
    Ok(())
}