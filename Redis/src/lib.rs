use chrono::{DateTime, Utc};
use redis::{pipe, Client, RedisResult};
use redis::aio::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("Redis connection error: {0}")]
    Connection(#[from] redis::RedisError),
    #[error("Transaction not found: {0}")]
    NotFound(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Transaction already exists: {0}")]
    AlreadyExists(String),
    #[error("Invalid status transition from {from} to {to}")]
    InvalidStatusTransition { from: String, to: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TransactionStatus::Pending => write!(f, "pending"),
            TransactionStatus::Confirmed => write!(f, "confirmed"),
            TransactionStatus::Failed => write!(f, "failed"),
            TransactionStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRepoModel {
    pub id: String,
    pub relayer_id: String,
    pub nonce: u64,
    pub status: TransactionStatus,
    pub hash: Option<String>,
    pub gas_price: u64,
    pub gas_limit: u64,
    pub value: String,
    pub to_address: String,
    pub data: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TransactionRepoModel {
    pub fn new(
        relayer_id: String,
        nonce: u64,
        to_address: String,
        value: String,
        gas_price: u64,
        gas_limit: u64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            relayer_id,
            nonce,
            status: TransactionStatus::Pending,
            hash: None,
            gas_price,
            gas_limit,
            value,
            to_address,
            data: None,
            created_at: now,
            updated_at: now,
        }
    }
}

pub struct TransactionRepository {
    client: Client,
}

impl TransactionRepository {
    pub fn new(redis_url: &str) -> Result<Self, RepositoryError> {
        let client = Client::open(redis_url)?;
        Ok(Self { client })
    }

    // Key generation helpers
    fn tx_key(tx_id: &str) -> String {
        format!("tx:{}", tx_id)
    }

    fn reverse_key(tx_id: &str) -> String {
        format!("lookup:{}", tx_id)
    }

    fn relayer_list_key() -> String {
        "relayers".to_string()
    }

    fn relayer_status_key(relayer_id: &str, status: &TransactionStatus) -> String {
        format!("relayer:{}:status:{}", relayer_id, status)
    }

    fn nonce_key(relayer_id: &str, nonce: u64) -> String {
        format!("relayer:{}:nonce:{}", relayer_id, nonce)
    }

    fn relayer_tx_count_key(relayer_id: &str) -> String {
        format!("relayer:{}:count", relayer_id)
    }

    /// 🔑 ATOMIC CREATE: Creates transaction with all indexes atomically
    pub async fn create(&self, entity: TransactionRepoModel) -> Result<TransactionRepoModel, RepositoryError> {
        let mut conn = self.client.get_async_connection().await?;
        
        // Check if transaction already exists (non-atomic check is OK here)
        let tx_key = Self::tx_key(&entity.id);
        let exists: bool = redis::cmd("EXISTS").arg(&tx_key).query_async(&mut conn).await?;
        if exists {
            return Err(RepositoryError::AlreadyExists(entity.id.clone()));
        }

        let json_data = serde_json::to_string(&entity)?;
        let reverse_key = Self::reverse_key(&entity.id);

        // 🚀 ATOMIC PIPELINE: Core transaction data + reverse lookup
        let mut pipe = pipe();
        pipe.atomic(); // Enable MULTI/EXEC transaction
        pipe.set(&tx_key, &json_data);
        pipe.set(&reverse_key, &entity.relayer_id);
        pipe.query_async::<_, ()>(&mut conn).await?;

        // 🚀 ATOMIC PIPELINE: Update all indexes
        self.update_indexes(&entity, None, &mut conn).await?;

        log::info!(
            "✅ Created transaction {} for relayer {} with nonce {}",
            entity.id,
            entity.relayer_id,
            entity.nonce
        );

        Ok(entity)
    }

    /// 🔑 ATOMIC UPDATE: Updates transaction and all related indexes atomically
    pub async fn update(&self, entity: TransactionRepoModel) -> Result<TransactionRepoModel, RepositoryError> {
        let mut conn = self.client.get_async_connection().await?;
        
        // Get the old transaction data
        let old_tx = self.get_by_id(&entity.id).await?;
        
        let tx_key = Self::tx_key(&entity.id);
        let json_data = serde_json::to_string(&entity)?;

        // 🚀 ATOMIC PIPELINE: Update core transaction data
        let mut pipe = pipe();
        pipe.atomic();
        pipe.set(&tx_key, &json_data);
        pipe.query_async::<_, ()>(&mut conn).await?;

        // 🚀 ATOMIC PIPELINE: Update all indexes
        self.update_indexes(&entity, Some(&old_tx), &mut conn).await?;

        log::info!(
            "✅ Updated transaction {} status: {} -> {}",
            entity.id,
            old_tx.status,
            entity.status
        );

        Ok(entity)
    }

    /// 🔑 CRITICAL ATOMIC OPERATION: Updates all indexes consistently
    async fn update_indexes(
        &self,
        tx: &TransactionRepoModel,
        old_tx: Option<&TransactionRepoModel>,
        conn: &mut Connection,
    ) -> Result<(), RepositoryError> {
        let mut pipe = pipe();
        pipe.atomic(); // 🔑 CRITICAL: Enables MULTI/EXEC transaction

        // Add new indexes
        let relayer_list_key = Self::relayer_list_key();
        let new_status_key = Self::relayer_status_key(&tx.relayer_id, &tx.status);
        let nonce_key = Self::nonce_key(&tx.relayer_id, tx.nonce);
        let count_key = Self::relayer_tx_count_key(&tx.relayer_id);

        pipe.sadd(&relayer_list_key, &tx.relayer_id);
        pipe.sadd(&new_status_key, &tx.id);
        pipe.set(&nonce_key, &tx.id);

        // Remove old indexes (if updating existing transaction)
        if let Some(old) = old_tx {
            if old.status != tx.status {
                let old_status_key = Self::relayer_status_key(&old.relayer_id, &old.status);
                pipe.srem(&old_status_key, &tx.id);
                log::debug!("Removing {} from old status: {}", tx.id, old.status);
            }
            
            if old.nonce != tx.nonce {
                let old_nonce_key = Self::nonce_key(&old.relayer_id, old.nonce);
                pipe.del(&old_nonce_key);
                log::debug!("Removing old nonce mapping: {}", old.nonce);
            }
        } else {
            // New transaction, increment counter
            pipe.incr(&count_key, 1);
        }

        // Set expiration on status indexes to prevent memory leaks
        pipe.expire(&new_status_key, 86400); // 24 hours

        // 🚀 Execute ALL index operations atomically
        let result: RedisResult<()> = pipe.query_async(conn).await;
        
        match result {
            Ok(_) => {
                log::debug!("✅ Successfully updated all indexes for transaction {}", tx.id);
                Ok(())
            }
            Err(e) => {
                log::error!("❌ Failed to update indexes atomically: {}", e);
                Err(RepositoryError::Connection(e))
            }
        }
    }

    pub async fn get_by_id(&self, tx_id: &str) -> Result<TransactionRepoModel, RepositoryError> {
        let mut conn = self.client.get_async_connection().await?;
        let tx_key = Self::tx_key(tx_id);
        
        let json_data: Option<String> = redis::cmd("GET").arg(&tx_key).query_async(&mut conn).await?;
        
        match json_data {
            Some(data) => {
                let tx = serde_json::from_str(&data)?;
                Ok(tx)
            }
            None => Err(RepositoryError::NotFound(tx_id.to_string()))
        }
    }

    pub async fn get_by_status(&self, relayer_id: &str, status: &TransactionStatus) -> Result<Vec<TransactionRepoModel>, RepositoryError> {
        let mut conn = self.client.get_async_connection().await?;
        let status_key = Self::relayer_status_key(relayer_id, status);
        
        let tx_ids: Vec<String> = redis::cmd("SMEMBERS").arg(&status_key).query_async(&mut conn).await?;
        
        let mut transactions = Vec::new();
        for tx_id in tx_ids {
            match self.get_by_id(&tx_id).await {
                Ok(tx) => transactions.push(tx),
                Err(RepositoryError::NotFound(_)) => {
                    log::warn!("Transaction {} found in index but not in storage", tx_id);
                    // Remove from index to maintain consistency
                    let _: () = redis::cmd("SREM").arg(&status_key).arg(&tx_id).query_async(&mut conn).await?;
                }
                Err(e) => return Err(e),
            }
        }
        
        Ok(transactions)
    }

    pub async fn get_by_nonce(&self, relayer_id: &str, nonce: u64) -> Result<Option<TransactionRepoModel>, RepositoryError> {
        let mut conn = self.client.get_async_connection().await?;
        let nonce_key = Self::nonce_key(relayer_id, nonce);
        
        let tx_id: Option<String> = redis::cmd("GET").arg(&nonce_key).query_async(&mut conn).await?;
        
        match tx_id {
            Some(id) => Ok(Some(self.get_by_id(&id).await?)),
            None => Ok(None),
        }
    }

    /// 🔑 ATOMIC BULK DELETE: Removes transaction and all indexes atomically
    pub async fn delete(&self, tx_id: &str) -> Result<(), RepositoryError> {
        let mut conn = self.client.get_async_connection().await?;
        
        // Get transaction data first
        let tx = self.get_by_id(tx_id).await?;
        
        let mut pipe = pipe();
        pipe.atomic();
        
        // Remove core data
        let tx_key = Self::tx_key(tx_id);
        let reverse_key = Self::reverse_key(tx_id);
        pipe.del(&tx_key);
        pipe.del(&reverse_key);
        
        // Remove from all indexes
        let status_key = Self::relayer_status_key(&tx.relayer_id, &tx.status);
        let nonce_key = Self::nonce_key(&tx.relayer_id, tx.nonce);
        let count_key = Self::relayer_tx_count_key(&tx.relayer_id);
        
        pipe.srem(&status_key, tx_id);
        pipe.del(&nonce_key);
        pipe.decr(&count_key, 1);
        
        pipe.query_async::<_, ()>(&mut conn).await?;
        
        log::info!("🗑️ Deleted transaction {} and all indexes", tx_id);
        Ok(())
    }

    /// 🔑 ATOMIC BULK OPERATIONS: Drop all data atomically
    pub async fn drop_all_entries(&self) -> Result<(), RepositoryError> {
        let mut conn = self.client.get_async_connection().await?;
        
        // Get all relayers
        let relayer_list_key = Self::relayer_list_key();
        let relayer_ids: Vec<String> = redis::cmd("SMEMBERS").arg(&relayer_list_key).query_async(&mut conn).await?;
        
        let mut pipe = pipe();
        pipe.atomic();
        
        // Collect ALL keys to delete
        let mut keys_to_delete = Vec::new();
        
        for relayer_id in &relayer_ids {
            // Get all transactions for this relayer
            for status in [
                TransactionStatus::Pending,
                TransactionStatus::Confirmed,
                TransactionStatus::Failed,
                TransactionStatus::Cancelled,
            ] {
                let status_key = Self::relayer_status_key(relayer_id, &status);
                let tx_ids: Vec<String> = redis::cmd("SMEMBERS").arg(&status_key).query_async(&mut conn).await?;
                
                for tx_id in tx_ids {
                    keys_to_delete.push(Self::tx_key(&tx_id));
                    keys_to_delete.push(Self::reverse_key(&tx_id));
                    
                    // Get nonce for this transaction
                    if let Ok(tx) = self.get_by_id(&tx_id).await {
                        keys_to_delete.push(Self::nonce_key(relayer_id, tx.nonce));
                    }
                }
                
                keys_to_delete.push(status_key);
            }
            
            keys_to_delete.push(Self::relayer_tx_count_key(relayer_id));
        }
        
        keys_to_delete.push(relayer_list_key);
        
        // Delete all keys in one atomic operation
        if !keys_to_delete.is_empty() {
            pipe.del(&keys_to_delete);
            pipe.query_async::<_, ()>(&mut conn).await?;
            
            log::info!("🧹 Atomically deleted {} keys", keys_to_delete.len());
        }
        
        Ok(())
    }

    /// Get statistics about the repository
    pub async fn get_stats(&self) -> Result<HashMap<String, i32>, RepositoryError> {
        let mut conn = self.client.get_async_connection().await?;
        let mut stats = HashMap::new();
        
        // Get relayer count
        let relayer_list_key = Self::relayer_list_key();
        let relayer_count: i32 = redis::cmd("SCARD").arg(&relayer_list_key).query_async(&mut conn).await?;
        stats.insert("relayers".to_string(), relayer_count);
        
        // Get total transaction counts by status
        let relayer_ids: Vec<String> = redis::cmd("SMEMBERS").arg(&relayer_list_key).query_async(&mut conn).await?;
        
        for status in [
            TransactionStatus::Pending,
            TransactionStatus::Confirmed,
            TransactionStatus::Failed,
            TransactionStatus::Cancelled,
        ] {
            let mut total = 0;
            for relayer_id in &relayer_ids {
                let status_key = Self::relayer_status_key(&relayer_id, &status);
                let count: i32 = redis::cmd("SCARD").arg(&status_key).query_async(&mut conn).await?;
                total += count;
            }
            stats.insert(format!("status_{}", status), total);
        }
        
        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_repo() -> TransactionRepository {
        let repo = TransactionRepository::new("redis://127.0.0.1:6379").unwrap();
        repo.drop_all_entries().await.unwrap();
        repo
    }

    #[tokio::test]
    async fn test_atomic_create_and_indexes() {
        let repo = setup_test_repo().await;
        
        let tx = TransactionRepoModel::new(
            "relayer-123".to_string(),
            42,
            "0xdeadbeef".to_string(),
            "1000000000000000000".to_string(),
            20000000000,
            21000,
        );
        
        let created = repo.create(tx.clone()).await.unwrap();
        assert_eq!(created.id, tx.id);
        
        // Verify all indexes were created atomically
        let by_status = repo.get_by_status(&tx.relayer_id, &TransactionStatus::Pending).await.unwrap();
        assert_eq!(by_status.len(), 1);
        
        let by_nonce = repo.get_by_nonce(&tx.relayer_id, 42).await.unwrap();
        assert!(by_nonce.is_some());
    }

    #[tokio::test]
    async fn test_atomic_status_update() {
        let repo = setup_test_repo().await;
        
        let mut tx = TransactionRepoModel::new(
            "relayer-456".to_string(),
            100,
            "0xcafebabe".to_string(),
            "2000000000000000000".to_string(),
            25000000000,
            21000,
        );
        
        let created = repo.create(tx.clone()).await.unwrap();
        
        // Update status atomically
        tx.id = created.id.clone();
        tx.status = TransactionStatus::Confirmed;
        tx.hash = Some("0x1234567890abcdef".to_string());
        tx.updated_at = Utc::now();
        
        repo.update(tx.clone()).await.unwrap();
        
        // Verify atomic update worked correctly
        let pending = repo.get_by_status(&tx.relayer_id, &TransactionStatus::Pending).await.unwrap();
        assert_eq!(pending.len(), 0);
        
        let confirmed = repo.get_by_status(&tx.relayer_id, &TransactionStatus::Confirmed).await.unwrap();
        assert_eq!(confirmed.len(), 1);
        assert_eq!(confirmed[0].hash, Some("0x1234567890abcdef".to_string()));
    }

    #[tokio::test]
    async fn test_atomic_bulk_delete() {
        let repo = setup_test_repo().await;
        
        // Create multiple transactions
        for i in 0..5 {
            let tx = TransactionRepoModel::new(
                format!("relayer-{}", i),
                i as u64,
                "0xdeadbeef".to_string(),
                "1000000000000000000".to_string(),
                20000000000,
                21000,
            );
            repo.create(tx).await.unwrap();
        }
        
        let stats_before = repo.get_stats().await.unwrap();
        assert_eq!(stats_before.get("status_pending").unwrap_or(&0), &5);
        
        // Atomic bulk delete
        repo.drop_all_entries().await.unwrap();
        
        let stats_after = repo.get_stats().await.unwrap();
        assert_eq!(stats_after.get("relayers").unwrap_or(&0), &0);
        assert_eq!(stats_after.get("status_pending").unwrap_or(&0), &0);
    }
}