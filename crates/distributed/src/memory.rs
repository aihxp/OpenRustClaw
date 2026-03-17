//! Distributed memory and state management.

use crate::config::{MemoryBackend, MemoryConfig};
use crate::error::{DistributedError, Result};
use async_trait::async_trait;
use dashmap::DashMap;
#[cfg(feature = "redis")]
use redis::aio::ConnectionManager;
#[cfg(feature = "redis")]
use redis::{AsyncCommands, Client as RedisClient};
use std::sync::Arc;
use std::time::Duration;
#[cfg(feature = "redis")]
use tokio_stream::StreamExt;
#[cfg(any(feature = "redis", feature = "etcd"))]
use tracing::info;

/// Distributed memory backend trait.
#[async_trait]
pub trait DistributedMemory: Send + Sync {
    /// Get a value by key.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Set a value.
    async fn set(&self, key: &str, value: Vec<u8>, ttl_secs: Option<u64>) -> Result<()>;

    /// Delete a key.
    async fn delete(&self, key: &str) -> Result<bool>;

    /// Check if a key exists.
    async fn exists(&self, key: &str) -> Result<bool>;

    /// Get multiple keys.
    async fn mget(&self, keys: &[&str]) -> Result<Vec<Option<Vec<u8>>>>;

    /// Set multiple keys.
    async fn mset(&self, items: &[(String, Vec<u8>)], ttl_secs: Option<u64>) -> Result<()>;

    /// List keys with prefix.
    async fn keys(&self, prefix: &str) -> Result<Vec<String>>;

    /// Acquire a distributed lock.
    async fn acquire_lock(&self, lock_name: &str, holder: &str, ttl_secs: u64) -> Result<bool>;

    /// Release a distributed lock.
    async fn release_lock(&self, lock_name: &str, holder: &str) -> Result<bool>;

    /// Renew a lock.
    async fn renew_lock(&self, lock_name: &str, holder: &str, ttl_secs: u64) -> Result<bool>;

    /// Publish a message to a channel.
    async fn publish(&self, channel: &str, message: Vec<u8>) -> Result<u64>;

    /// Subscribe to a channel.
    async fn subscribe(&self, channel: &str) -> Result<Box<dyn MemorySubscription>>;

    /// Close the connection.
    async fn close(&self) -> Result<()>;
}

/// Subscription to memory events.
#[async_trait]
pub trait MemorySubscription: Send + Sync {
    /// Receive the next message.
    async fn recv(&mut self) -> Result<Option<Vec<u8>>>;

    /// Unsubscribe.
    async fn unsubscribe(self: Box<Self>) -> Result<()>;
}

/// Create a distributed memory backend.
pub async fn create_memory(config: &MemoryConfig) -> Result<Arc<dyn DistributedMemory>> {
    match config.backend {
        #[cfg(feature = "redis")]
        MemoryBackend::Redis => {
            let redis = RedisMemory::new(config).await?;
            Ok(Arc::new(redis))
        }
        #[cfg(not(feature = "redis"))]
        MemoryBackend::Redis => Err(DistributedError::Config(
            "Redis memory backend requires the 'redis' feature to be enabled".to_string(),
        )),
        MemoryBackend::Gossip => {
            let gossip = GossipMemory::new(config)?;
            Ok(Arc::new(gossip))
        }
        #[cfg(feature = "etcd")]
        MemoryBackend::Etcd => {
            let etcd = EtcdMemory::new(config).await?;
            Ok(Arc::new(etcd))
        }
        #[cfg(not(feature = "etcd"))]
        MemoryBackend::Etcd => Err(DistributedError::Config(
            "etcd memory backend requires the 'etcd' feature to be enabled".to_string(),
        )),
    }
}

/// Redis-backed distributed memory.
///
/// Uses `ConnectionManager` which provides interior mutability and is
/// designed to be cheaply cloneable for concurrent operations.
#[cfg(feature = "redis")]
pub struct RedisMemory {
    client: RedisClient,
    connection: ConnectionManager,
}

#[cfg(feature = "redis")]
impl RedisMemory {
    /// Create a new Redis memory backend.
    pub async fn new(config: &MemoryConfig) -> Result<Self> {
        let client = if config.redis_cluster.is_empty() {
            RedisClient::open(config.redis_url.clone()).map_err(|e| {
                DistributedError::Memory(format!("Failed to connect to Redis: {}", e))
            })?
        } else {
            // Cluster mode - use the first node as the connection point
            // In production, you'd use redis::cluster::ClusterClient here
            RedisClient::open(config.redis_cluster[0].clone()).map_err(|e| {
                DistributedError::Memory(format!("Failed to connect to Redis cluster: {}", e))
            })?
        };

        let connection = client
            .get_connection_manager()
            .await
            .map_err(|e| DistributedError::Memory(format!("Failed to get connection: {}", e)))?;

        info!("Connected to Redis at {}", config.redis_url);

        Ok(Self { client, connection })
    }

    /// Get a clone of the connection.
    ///
    /// `ConnectionManager` uses interior mutability and is cheap to clone.
    fn conn(&self) -> ConnectionManager {
        self.connection.clone()
    }
}

#[cfg(feature = "redis")]
#[async_trait]
impl DistributedMemory for RedisMemory {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let mut conn = self.conn();
        let value: Option<Vec<u8>> = conn
            .get(key)
            .await
            .map_err(|e| DistributedError::Memory(format!("Redis get error: {}", e)))?;
        Ok(value)
    }

    async fn set(&self, key: &str, value: Vec<u8>, ttl_secs: Option<u64>) -> Result<()> {
        let mut conn = self.conn();

        if let Some(ttl) = ttl_secs {
            let _: () = conn
                .set_ex(key, value, ttl)
                .await
                .map_err(|e| DistributedError::Memory(format!("Redis set error: {}", e)))?;
        } else {
            let _: () = conn
                .set(key, value)
                .await
                .map_err(|e| DistributedError::Memory(format!("Redis set error: {}", e)))?;
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let mut conn = self.conn();
        let deleted: i32 = conn
            .del(key)
            .await
            .map_err(|e| DistributedError::Memory(format!("Redis delete error: {}", e)))?;
        Ok(deleted > 0)
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.conn();
        let exists: bool = conn
            .exists(key)
            .await
            .map_err(|e| DistributedError::Memory(format!("Redis exists error: {}", e)))?;
        Ok(exists)
    }

    async fn mget(&self, keys: &[&str]) -> Result<Vec<Option<Vec<u8>>>> {
        let mut conn = self.conn();
        let values: Vec<Option<Vec<u8>>> = conn
            .get(keys)
            .await
            .map_err(|e| DistributedError::Memory(format!("Redis mget error: {}", e)))?;
        Ok(values)
    }

    async fn mset(&self, items: &[(String, Vec<u8>)], ttl_secs: Option<u64>) -> Result<()> {
        let mut conn = self.conn();

        // Use pipeline for efficiency
        let _pipeline = redis::pipe();

        if let Some(ttl) = ttl_secs {
            // Use atomic MSET with TTL
            // Use MSET with pipelined EXPIRE
            let _: () = redis::cmd("MSET")
                .arg(
                    items
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<Vec<_>>(),
                )
                .query_async(&mut conn)
                .await
                .map_err(|e| DistributedError::Memory(format!("Redis mset error: {}", e)))?;

            // Set TTLs individually
            for (key, _) in items {
                let _: () = conn
                    .expire(key, ttl as i64)
                    .await
                    .map_err(|e| DistributedError::Memory(format!("Redis expire error: {}", e)))?;
            }
        } else {
            let _: () = redis::cmd("MSET")
                .arg(
                    items
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<Vec<_>>(),
                )
                .query_async(&mut conn)
                .await
                .map_err(|e| DistributedError::Memory(format!("Redis mset error: {}", e)))?;
        }

        Ok(())
    }

    async fn keys(&self, prefix: &str) -> Result<Vec<String>> {
        let mut conn = self.conn();
        let pattern = format!("{}*", prefix);
        let keys: Vec<String> = conn
            .keys(&pattern)
            .await
            .map_err(|e| DistributedError::Memory(format!("Redis keys error: {}", e)))?;
        Ok(keys)
    }

    async fn acquire_lock(&self, lock_name: &str, holder: &str, ttl_secs: u64) -> Result<bool> {
        let key = format!("lock:{}", lock_name);
        let mut conn = self.conn();

        // Use SET with NX (only if not exists) and EX (expire)
        let result: Option<String> = redis::cmd("SET")
            .arg(&key)
            .arg(holder)
            .arg("NX")
            .arg("EX")
            .arg(ttl_secs)
            .query_async(&mut conn)
            .await
            .map_err(|e| DistributedError::Memory(format!("Redis lock error: {}", e)))?;

        Ok(result.is_some())
    }

    async fn release_lock(&self, lock_name: &str, holder: &str) -> Result<bool> {
        let key = format!("lock:{}", lock_name);
        let mut conn = self.conn();

        // Use Lua script for atomic check-and-delete
        let script = r#"
            if redis.call("get", KEYS[1]) == ARGV[1] then
                return redis.call("del", KEYS[1])
            else
                return 0
            end
        "#;

        let result: i32 = redis::Script::new(script)
            .key(&key)
            .arg(holder)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| DistributedError::Memory(format!("Redis unlock error: {}", e)))?;

        Ok(result > 0)
    }

    async fn renew_lock(&self, lock_name: &str, holder: &str, ttl_secs: u64) -> Result<bool> {
        let key = format!("lock:{}", lock_name);
        let mut conn = self.conn();

        // Check if we still hold the lock and renew it
        let script = r#"
            if redis.call("get", KEYS[1]) == ARGV[1] then
                return redis.call("expire", KEYS[1], ARGV[2])
            else
                return 0
            end
        "#;

        let result: i32 = redis::Script::new(script)
            .key(&key)
            .arg(holder)
            .arg(ttl_secs)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| DistributedError::Memory(format!("Redis renew error: {}", e)))?;

        Ok(result > 0)
    }

    async fn publish(&self, channel: &str, message: Vec<u8>) -> Result<u64> {
        let mut conn = self.conn();
        let receivers: i64 = conn
            .publish(channel, message)
            .await
            .map_err(|e| DistributedError::Memory(format!("Redis publish error: {}", e)))?;
        Ok(receivers as u64)
    }

    async fn subscribe(&self, channel: &str) -> Result<Box<dyn MemorySubscription>> {
        let mut pubsub = self
            .client
            .get_async_pubsub()
            .await
            .map_err(|e| DistributedError::Memory(format!("Redis subscribe error: {}", e)))?;

        pubsub
            .subscribe(channel)
            .await
            .map_err(|e| DistributedError::Memory(format!("Redis subscribe error: {}", e)))?;

        let stream = pubsub.into_on_message();

        Ok(Box::new(RedisSubscription { stream }))
    }

    async fn close(&self) -> Result<()> {
        // Connection manager handles reconnection automatically
        Ok(())
    }
}

/// Redis subscription wrapper.
#[cfg(feature = "redis")]
struct RedisSubscription {
    stream: redis::aio::PubSubStream,
}

#[cfg(feature = "redis")]
#[async_trait]
impl MemorySubscription for RedisSubscription {
    async fn recv(&mut self) -> Result<Option<Vec<u8>>> {
        match self.stream.next().await {
            Some(msg) => Ok(msg.get_payload().ok()),
            None => Ok(None),
        }
    }

    async fn unsubscribe(self: Box<Self>) -> Result<()> {
        // The stream is dropped, which unsubscribes
        Ok(())
    }
}

/// Gossip-based distributed memory (eventual consistency).
pub struct GossipMemory {
    data: DashMap<String, (Vec<u8>, u64)>, // key -> (value, version)
    locks: DashMap<String, (String, std::time::Instant)>, // lock_name -> (holder, expiry)
    #[allow(dead_code)]
    config: MemoryConfig,
}

impl GossipMemory {
    /// Create a new gossip memory backend.
    pub fn new(config: &MemoryConfig) -> Result<Self> {
        Ok(Self {
            data: DashMap::new(),
            locks: DashMap::new(),
            config: config.clone(),
        })
    }

    /// Clean up expired locks.
    fn cleanup_locks(&self) {
        let now = std::time::Instant::now();
        let expired: Vec<String> = self
            .locks
            .iter()
            .filter(|entry| entry.value().1 < now)
            .map(|entry| entry.key().clone())
            .collect();

        for lock in expired {
            self.locks.remove(&lock);
        }
    }
}

#[async_trait]
impl DistributedMemory for GossipMemory {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.data.get(key).map(|entry| entry.value().0.clone()))
    }

    async fn set(&self, key: &str, value: Vec<u8>, _ttl_secs: Option<u64>) -> Result<()> {
        let version = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("System time should be after UNIX epoch")
            .as_secs();

        // Simple conflict resolution: higher version wins
        let mut entry = self
            .data
            .entry(key.to_string())
            .or_insert((value.clone(), version));
        if version > entry.value().1 {
            *entry.value_mut() = (value, version);
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        Ok(self.data.remove(key).is_some())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.data.contains_key(key))
    }

    async fn mget(&self, keys: &[&str]) -> Result<Vec<Option<Vec<u8>>>> {
        let mut results = Vec::new();
        for key in keys {
            results.push(self.get(key).await?);
        }
        Ok(results)
    }

    async fn mset(&self, items: &[(String, Vec<u8>)], _ttl_secs: Option<u64>) -> Result<()> {
        for (key, value) in items {
            self.set(key, value.clone(), None).await?;
        }
        Ok(())
    }

    async fn keys(&self, prefix: &str) -> Result<Vec<String>> {
        Ok(self
            .data
            .iter()
            .filter(|entry| entry.key().starts_with(prefix))
            .map(|entry| entry.key().clone())
            .collect())
    }

    async fn acquire_lock(&self, lock_name: &str, holder: &str, ttl_secs: u64) -> Result<bool> {
        self.cleanup_locks();

        let expiry = std::time::Instant::now() + Duration::from_secs(ttl_secs);

        match self.locks.entry(lock_name.to_string()) {
            dashmap::mapref::entry::Entry::Occupied(_) => Ok(false),
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                entry.insert((holder.to_string(), expiry));
                Ok(true)
            }
        }
    }

    async fn release_lock(&self, lock_name: &str, holder: &str) -> Result<bool> {
        self.cleanup_locks();

        let should_remove = self
            .locks
            .get(lock_name)
            .map(|entry| entry.value().0 == holder)
            .unwrap_or(false);

        if should_remove {
            self.locks.remove(lock_name);
            return Ok(true);
        }

        Ok(false)
    }

    async fn renew_lock(&self, lock_name: &str, holder: &str, ttl_secs: u64) -> Result<bool> {
        self.cleanup_locks();

        if let Some(mut entry) = self.locks.get_mut(lock_name)
            && entry.value().0 == holder
        {
            entry.value_mut().1 = std::time::Instant::now() + Duration::from_secs(ttl_secs);
            return Ok(true);
        }

        Ok(false)
    }

    async fn publish(&self, _channel: &str, _message: Vec<u8>) -> Result<u64> {
        // Gossip memory doesn't support pub/sub directly
        Ok(0)
    }

    async fn subscribe(&self, _channel: &str) -> Result<Box<dyn MemorySubscription>> {
        Err(DistributedError::Internal(
            "Gossip memory doesn't support subscriptions".to_string(),
        ))
    }

    async fn close(&self) -> Result<()> {
        self.data.clear();
        self.locks.clear();
        Ok(())
    }
}

/// etcd-based distributed memory.
#[cfg(feature = "etcd")]
pub struct EtcdMemory {
    client: etcd_client::Client,
    #[allow(dead_code)]
    config: MemoryConfig,
}

#[cfg(feature = "etcd")]
impl EtcdMemory {
    /// Create a new etcd memory backend.
    pub async fn new(config: &MemoryConfig) -> Result<Self> {
        // etcd endpoints would be from config
        let endpoints: Vec<String> = vec!["http://127.0.0.1:2379".to_string()];

        let client = etcd_client::Client::connect(endpoints, None)
            .await
            .map_err(|e| {
                DistributedError::Discovery(format!("Failed to connect to etcd: {}", e))
            })?;

        info!("Connected to etcd");

        Ok(Self {
            client,
            config: config.clone(),
        })
    }
}

#[cfg(feature = "etcd")]
#[async_trait]
impl DistributedMemory for EtcdMemory {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let mut client = self.client.clone();
        let response = client
            .get(key, None)
            .await
            .map_err(|e| DistributedError::Memory(format!("etcd get error: {}", e)))?;

        if let Some(kv) = response.kvs().first() {
            Ok(Some(kv.value().to_vec()))
        } else {
            Ok(None)
        }
    }

    async fn set(&self, key: &str, value: Vec<u8>, ttl_secs: Option<u64>) -> Result<()> {
        let mut client = self.client.clone();

        if let Some(ttl) = ttl_secs {
            let lease = client
                .lease_grant(ttl as i64, None)
                .await
                .map_err(|e| DistributedError::Memory(format!("etcd lease error: {}", e)))?;

            client
                .put(
                    key,
                    value,
                    Some(etcd_client::PutOptions::new().with_lease(lease.id())),
                )
                .await
                .map_err(|e| DistributedError::Memory(format!("etcd put error: {}", e)))?;
        } else {
            client
                .put(key, value, None)
                .await
                .map_err(|e| DistributedError::Memory(format!("etcd put error: {}", e)))?;
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool> {
        let mut client = self.client.clone();
        let response = client
            .delete(key, None)
            .await
            .map_err(|e| DistributedError::Memory(format!("etcd delete error: {}", e)))?;

        Ok(response.deleted() > 0)
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        self.get(key).await.map(|v| v.is_some())
    }

    async fn mget(&self, keys: &[&str]) -> Result<Vec<Option<Vec<u8>>>> {
        let mut results = Vec::new();
        for key in keys {
            results.push(self.get(key).await?);
        }
        Ok(results)
    }

    async fn mset(&self, items: &[(String, Vec<u8>)], ttl_secs: Option<u64>) -> Result<()> {
        for (key, value) in items {
            self.set(key, value.clone(), ttl_secs).await?;
        }
        Ok(())
    }

    async fn keys(&self, prefix: &str) -> Result<Vec<String>> {
        let mut client = self.client.clone();
        let response = client
            .get(prefix, Some(etcd_client::GetOptions::new().with_prefix()))
            .await
            .map_err(|e| DistributedError::Memory(format!("etcd keys error: {}", e)))?;

        Ok(response
            .kvs()
            .iter()
            .map(|kv| String::from_utf8_lossy(kv.key()).to_string())
            .collect())
    }

    async fn acquire_lock(&self, lock_name: &str, holder: &str, ttl_secs: u64) -> Result<bool> {
        // Use etcd's distributed locking
        let key = format!("/locks/{}", lock_name);
        let value = holder.as_bytes().to_vec();

        let mut client = self.client.clone();
        let lease = client
            .lease_grant(ttl_secs as i64, None)
            .await
            .map_err(|e| DistributedError::Memory(format!("etcd lease error: {}", e)))?;

        let txn = etcd_client::Txn::new()
            .when([etcd_client::Compare::version(
                key.clone(),
                etcd_client::CompareOp::Equal,
                0,
            )])
            .and_then([etcd_client::TxnOp::put(
                key.clone(),
                value,
                Some(etcd_client::PutOptions::new().with_lease(lease.id())),
            )]);

        let response = client
            .txn(txn)
            .await
            .map_err(|e| DistributedError::Memory(format!("etcd lock error: {}", e)))?;

        Ok(response.succeeded())
    }

    async fn release_lock(&self, lock_name: &str, holder: &str) -> Result<bool> {
        let key = format!("/locks/{}", lock_name);

        // Check if we hold the lock
        if let Some(value) = self.get(&key).await? {
            if value == holder.as_bytes() {
                return self.delete(&key).await;
            }
        }

        Ok(false)
    }

    async fn renew_lock(&self, lock_name: &str, holder: &str, _ttl_secs: u64) -> Result<bool> {
        let key = format!("/locks/{}", lock_name);

        // In etcd, we need to keep the lease alive
        // For simplicity, this is a no-op - in production, use lease keepalive
        if let Some(value) = self.get(&key).await? {
            if value == holder.as_bytes() {
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn publish(&self, _channel: &str, _message: Vec<u8>) -> Result<u64> {
        // etcd doesn't have pub/sub, would need to be built on watches
        Ok(0)
    }

    async fn subscribe(&self, _channel: &str) -> Result<Box<dyn MemorySubscription>> {
        Err(DistributedError::Internal(
            "etcd subscriptions not yet implemented".to_string(),
        ))
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gossip_memory_basic() {
        let config = MemoryConfig::default();
        let memory = GossipMemory::new(&config).unwrap();

        // Set and get
        memory.set("key1", b"value1".to_vec(), None).await.unwrap();
        let value = memory.get("key1").await.unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));

        // Delete
        assert!(memory.delete("key1").await.unwrap());
        assert!(memory.get("key1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_gossip_memory_locks() {
        let config = MemoryConfig::default();
        let memory = GossipMemory::new(&config).unwrap();

        // Acquire lock
        assert!(memory.acquire_lock("lock1", "holder1", 60).await.unwrap());

        // Can't acquire same lock
        assert!(!memory.acquire_lock("lock1", "holder2", 60).await.unwrap());

        // Release with wrong holder fails
        assert!(!memory.release_lock("lock1", "holder2").await.unwrap());

        // Release with correct holder succeeds
        assert!(memory.release_lock("lock1", "holder1").await.unwrap());

        // Can acquire again
        assert!(memory.acquire_lock("lock1", "holder2", 60).await.unwrap());
    }
}
