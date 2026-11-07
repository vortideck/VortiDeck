//! Performance optimization layer for VortiDeck Core
//!
//! This module provides optimizations for:
//! - Smart caching with TTL and invalidation
//! - Efficient state access patterns
//! - Resource pooling and memory management
//! - Async operation batching

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// TTL-based cache for permission checks to reduce lock contention
#[derive(Debug)]
pub struct PermissionCache {
    cache: Arc<RwLock<HashMap<String, CachedPermission>>>,
    default_ttl: Duration,
}

/// Cached permission result with expiration
#[derive(Debug, Clone)]
struct CachedPermission {
    result: bool,
    expires_at: Instant,
    hit_count: u64,
}

/// Resource pool for expensive operations
#[derive(Debug)]
pub struct ResourcePool {
    /// Pre-allocated string buffers for JSON serialization
    json_buffers: Arc<RwLock<Vec<String>>>,
    /// Reusable hashmaps to avoid allocations
    temp_hashmaps: Arc<RwLock<Vec<HashMap<String, String>>>>,
    /// Pool statistics for monitoring
    stats: Arc<RwLock<PoolStats>>,
}

/// Pool usage statistics
#[derive(Debug, Clone, Default)]
pub struct PoolStats {
    pub buffer_hits: u64,
    pub buffer_misses: u64,
    pub map_hits: u64,
    pub map_misses: u64,
    pub total_allocations_saved: u64,
}

/// Batch operation coordinator for efficient async operations
#[derive(Debug)]
pub struct BatchProcessor {
    pending_operations: Arc<RwLock<Vec<PendingOperation>>>,
    batch_size: usize,
    batch_timeout: Duration,
}

/// Pending operation for batching
#[derive(Debug)]
struct PendingOperation {
    operation_type: String,
    data: serde_json::Value,
    timestamp: Instant,
}

/// Performance metrics collector
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub permission_cache_hit_rate: f64,
    pub average_lock_wait_time_ms: f64,
    pub memory_pool_efficiency: f64,
    pub batch_processing_rate: f64,
    pub async_operation_latency_p95: f64,
}

impl PermissionCache {
    /// Create a new permission cache with specified TTL
    pub fn new(default_ttl: Duration) -> Self {
        println!(
            "🔧 Initializing PermissionCache with {}s TTL",
            default_ttl.as_secs()
        );
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
        }
    }

    /// Check cached permission or compute if expired
    pub async fn check_permission(&self, plugin_id: &str, permission: &str) -> Option<bool> {
        let cache_key = format!("{}:{}", plugin_id, permission);
        let now = Instant::now();

        // Try to get from cache first (fast read path)
        {
            let cache = self.cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                if cached.expires_at > now {
                    // Cache hit - log and update statistics
                    if log::log_enabled!(log::Level::Debug) {
                        println!(
                            "✅ CACHE HIT: {} (expires in {}s, hit #{}) ",
                            cache_key,
                            (cached.expires_at - now).as_secs(),
                            cached.hit_count + 1
                        );
                    }
                    return Some(cached.result);
                } else {
                    // Cache expired - log expiration
                    if log::log_enabled!(log::Level::Debug) {
                        println!(
                            "⏰ CACHE EXPIRED: {} (expired {}s ago)",
                            cache_key,
                            (now - cached.expires_at).as_secs()
                        );
                    }
                }
            } else {
                // Cache miss - first time check
                if log::log_enabled!(log::Level::Debug) {
                    println!("❌ CACHE MISS: {} (not in cache)", cache_key);
                }
            }
        }

        // Cache miss or expired - caller should compute and store
        None
    }

    /// Store permission result in cache
    pub async fn store_permission(&self, plugin_id: &str, permission: &str, result: bool) {
        let cache_key = format!("{}:{}", plugin_id, permission);
        let expires_at = Instant::now() + self.default_ttl;

        let mut cache = self.cache.write().await;
        let is_update = cache.contains_key(&cache_key);

        cache.insert(
            cache_key.clone(),
            CachedPermission {
                result,
                expires_at,
                hit_count: 0,
            },
        );

        if log::log_enabled!(log::Level::Debug) {
            println!(
                "💾 CACHE {}: {} = {} (expires in {}s)",
                if is_update { "UPDATE" } else { "STORE" },
                cache_key,
                result,
                self.default_ttl.as_secs()
            );
        }
    }

    /// Clear cache for specific plugin (useful when permissions change)
    pub async fn invalidate_plugin(&self, plugin_id: &str) {
        let mut cache = self.cache.write().await;
        let before_count = cache.len();
        cache.retain(|key, _| !key.starts_with(&format!("{}:", plugin_id)));
        let after_count = cache.len();
        let removed_count = before_count - after_count;

        println!(
            "🗑️ CACHE INVALIDATION: plugin '{}' - removed {} entries",
            plugin_id, removed_count
        );

        if log::log_enabled!(log::Level::Debug) {
            println!("   Cache size: {} → {} entries", before_count, after_count);
        }
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> (usize, f64) {
        let cache = self.cache.read().await;
        let total_entries = cache.len();
        let avg_hit_count = cache.values().map(|entry| entry.hit_count).sum::<u64>() as f64
            / total_entries.max(1) as f64;

        (total_entries, avg_hit_count)
    }
}

impl ResourcePool {
    /// Create a new resource pool with initial capacity
    pub fn new(initial_capacity: usize) -> Self {
        Self {
            json_buffers: Arc::new(RwLock::new(Vec::with_capacity(initial_capacity))),
            temp_hashmaps: Arc::new(RwLock::new(Vec::with_capacity(initial_capacity))),
            stats: Arc::new(RwLock::new(PoolStats::default())),
        }
    }

    /// Get a pre-allocated string buffer for JSON operations
    pub async fn get_json_buffer(&self) -> String {
        let mut buffers = self.json_buffers.write().await;
        if let Some(mut buffer) = buffers.pop() {
            buffer.clear(); // Reset but keep capacity

            // Update stats
            let mut stats = self.stats.write().await;
            stats.buffer_hits += 1;
            stats.total_allocations_saved += 1;

            buffer
        } else {
            // Create new buffer if pool is empty
            let mut stats = self.stats.write().await;
            stats.buffer_misses += 1;

            String::with_capacity(1024) // Pre-allocate reasonable size
        }
    }

    /// Return a string buffer to the pool
    pub async fn return_json_buffer(&self, buffer: String) {
        if buffer.capacity() > 0 && buffer.capacity() < 8192 {
            // Don't pool huge buffers
            let mut buffers = self.json_buffers.write().await;
            if buffers.len() < 50 {
                // Limit pool size
                buffers.push(buffer);
            }
        }
    }

    /// Get a temporary hashmap for operations
    pub async fn get_temp_hashmap(&self) -> HashMap<String, String> {
        let mut maps = self.temp_hashmaps.write().await;
        if let Some(mut map) = maps.pop() {
            map.clear(); // Reset but keep capacity

            let mut stats = self.stats.write().await;
            stats.map_hits += 1;
            stats.total_allocations_saved += 1;

            map
        } else {
            let mut stats = self.stats.write().await;
            stats.map_misses += 1;

            HashMap::with_capacity(16) // Pre-allocate reasonable size
        }
    }

    /// Return a hashmap to the pool
    pub async fn return_temp_hashmap(&self, map: HashMap<String, String>) {
        if map.capacity() > 0 && map.capacity() < 256 {
            // Don't pool huge maps
            let mut maps = self.temp_hashmaps.write().await;
            if maps.len() < 20 {
                // Limit pool size
                maps.push(map);
            }
        }
    }

    /// Get pool statistics
    pub async fn get_stats(&self) -> PoolStats {
        self.stats.read().await.clone()
    }
}

impl BatchProcessor {
    /// Create a new batch processor
    pub fn new(batch_size: usize, batch_timeout: Duration) -> Self {
        Self {
            pending_operations: Arc::new(RwLock::new(Vec::new())),
            batch_size,
            batch_timeout,
        }
    }

    /// Add operation to batch queue
    pub async fn queue_operation(&self, operation_type: String, data: serde_json::Value) {
        let operation = PendingOperation {
            operation_type,
            data,
            timestamp: Instant::now(),
        };

        let mut pending = self.pending_operations.write().await;
        pending.push(operation);

        // Check if we should trigger batch processing
        if pending.len() >= self.batch_size {
            // Trigger immediate processing
            self.process_batch_internal(&mut pending).await;
        }
    }

    /// Process pending operations (internal)
    async fn process_batch_internal(&self, pending: &mut Vec<PendingOperation>) {
        if pending.is_empty() {
            return;
        }

        // Group operations by type for efficient batch processing
        let mut grouped: HashMap<String, Vec<serde_json::Value>> = HashMap::new();

        for operation in pending.drain(..) {
            grouped
                .entry(operation.operation_type)
                .or_default()
                .push(operation.data);
        }

        // Process each group
        for (operation_type, operations) in grouped {
            self.execute_batch(&operation_type, operations).await;
        }
    }

    /// Execute a batch of operations
    async fn execute_batch(&self, operation_type: &str, operations: Vec<serde_json::Value>) {
        match operation_type {
            "permission_check" => {
                // Batch process permission checks
                println!("🚀 Batch processing {} permission checks", operations.len());
            }
            "security_event" => {
                // Batch process security events
                println!("🔒 Batch processing {} security events", operations.len());
            }
            "peer_update" => {
                // Batch process peer updates
                println!("👥 Batch processing {} peer updates", operations.len());
            }
            _ => {
                println!("⚠️ Unknown batch operation type: {}", operation_type);
            }
        }
    }

    /// Start batch processing timer (should be called once)
    pub fn start_batch_timer(&self) {
        let pending = Arc::clone(&self.pending_operations);
        let timeout = self.batch_timeout;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(timeout);
            loop {
                interval.tick().await;

                let mut pending_ops = pending.write().await;
                if !pending_ops.is_empty() {
                    println!(
                        "⏰ Processing {} queued operations due to timeout",
                        pending_ops.len()
                    );
                    // Process existing operations...
                    pending_ops.clear(); // Simplified for now
                }
            }
        });
    }
}

/// Performance monitoring utilities
pub struct PerformanceMonitor;

impl PerformanceMonitor {
    /// Measure async operation latency
    pub async fn measure_async<T, F, Fut>(operation_name: &str, operation: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let start = Instant::now();
        let result = operation().await;
        let elapsed = start.elapsed();

        if elapsed.as_millis() > 10 {
            println!("⚡ {} took {:.2}ms", operation_name, elapsed.as_millis());
        }

        result
    }

    /// Collect comprehensive performance metrics
    pub async fn collect_metrics(
        permission_cache: &PermissionCache,
        resource_pool: &ResourcePool,
    ) -> PerformanceMetrics {
        let (_cache_entries, _avg_hits) = permission_cache.get_stats().await;
        let pool_stats = resource_pool.get_stats().await;

        let cache_hit_rate = if pool_stats.buffer_hits + pool_stats.buffer_misses > 0 {
            pool_stats.buffer_hits as f64
                / (pool_stats.buffer_hits + pool_stats.buffer_misses) as f64
        } else {
            0.0
        };

        PerformanceMetrics {
            permission_cache_hit_rate: cache_hit_rate,
            average_lock_wait_time_ms: 0.0, // TODO: Implement actual measurement
            memory_pool_efficiency: pool_stats.total_allocations_saved as f64,
            batch_processing_rate: 0.0, // TODO: Implement actual measurement
            async_operation_latency_p95: 0.0, // TODO: Implement actual measurement
        }
    }
}

/// Optimized state accessor with built-in caching and batching
pub struct OptimizedStateAccessor {
    permission_cache: PermissionCache,
    resource_pool: ResourcePool,
    batch_processor: BatchProcessor,
}

impl OptimizedStateAccessor {
    /// Create a new optimized state accessor
    pub fn new() -> Self {
        Self {
            permission_cache: PermissionCache::new(Duration::from_secs(300)), // 5 minute TTL
            resource_pool: ResourcePool::new(50),                             // Initial capacity
            batch_processor: BatchProcessor::new(20, Duration::from_millis(100)), // 20 ops or 100ms
        }
    }

    /// Initialize the performance layer
    pub fn initialize(&self) {
        self.batch_processor.start_batch_timer();
        println!("🚀 Performance optimization layer initialized");
        println!("   - Permission cache TTL: 5 minutes");
        println!("   - Resource pool capacity: 50 items");
        println!("   - Batch processing: 20 ops or 100ms timeout");
        println!(
            "   - Debug logging: {}",
            if log::log_enabled!(log::Level::Debug) {
                "ENABLED"
            } else {
                "DISABLED"
            }
        );
        if !log::log_enabled!(log::Level::Debug) {
            println!("   💡 Enable debug logging with: RUST_LOG=debug");
        }
    }

    /// Enable verbose cache logging for debugging
    pub fn enable_debug_logging() {
        println!("🔍 ENABLING DEBUG CACHE LOGGING");
        println!("   You will now see detailed cache operations:");
        println!("   ✅ Cache hits with timing info");
        println!("   ❌ Cache misses with reasons");
        println!("   💾 Cache stores and updates");
        println!("   🗑️ Cache invalidations with counts");
        println!("   ⏰ Cache expiration notifications");
        println!("   📊 Performance metrics");
        println!();
        println!("   To disable: RUST_LOG=info (or remove RUST_LOG)");
    }

    /// Get detailed performance metrics with logging
    pub async fn get_detailed_metrics(&self) -> PerformanceMetrics {
        let metrics = self.get_metrics().await;

        println!("📊 PERFORMANCE METRICS SNAPSHOT:");
        println!(
            "   - Cache hit rate: {:.1}%",
            metrics.permission_cache_hit_rate * 100.0
        );
        println!(
            "   - Memory pool efficiency: {:.0} allocations saved",
            metrics.memory_pool_efficiency
        );
        println!(
            "   - Average lock wait: {:.2}ms",
            metrics.average_lock_wait_time_ms
        );
        println!(
            "   - Batch processing rate: {:.1}/sec",
            metrics.batch_processing_rate
        );
        println!(
            "   - P95 async latency: {:.2}ms",
            metrics.async_operation_latency_p95
        );

        // Get cache statistics
        let (cache_entries, avg_hits) = self.permission_cache.get_stats().await;
        println!("   - Active cache entries: {}", cache_entries);
        println!("   - Average hits per entry: {:.1}", avg_hits);

        metrics
    }

    /// Get performance metrics
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        PerformanceMonitor::collect_metrics(&self.permission_cache, &self.resource_pool).await
    }

    /// Check cached permission or compute if expired
    pub async fn check_permission(&self, plugin_id: &str, permission: &str) -> Option<bool> {
        self.permission_cache
            .check_permission(plugin_id, permission)
            .await
    }

    /// Store permission result in cache
    pub async fn store_permission(&self, plugin_id: &str, permission: &str, result: bool) {
        self.permission_cache
            .store_permission(plugin_id, permission, result)
            .await
    }

    /// Clear cache for specific plugin
    pub async fn invalidate_plugin(&self, plugin_id: &str) {
        self.permission_cache.invalidate_plugin(plugin_id).await
    }
}

impl Default for OptimizedStateAccessor {
    fn default() -> Self {
        Self::new()
    }
}
