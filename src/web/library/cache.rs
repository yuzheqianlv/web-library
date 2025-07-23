//! # Library缓存模块
//!
//! 提供库统计信息的缓存机制，提升查询性能

#[cfg(feature = "web")]
use std::sync::Arc;
#[cfg(feature = "web")]
use std::time::{Duration, Instant};
#[cfg(feature = "web")]
use tokio::sync::RwLock;
#[cfg(feature = "web")]
use std::collections::HashMap;

use super::v2_types::LibraryStatsResponse;

/// 缓存条目
#[cfg(feature = "web")]
#[derive(Debug, Clone)]
struct CacheEntry {
    /// 缓存的数据
    data: LibraryStatsResponse,
    /// 创建时间
    created_at: Instant,
    /// 过期时间
    expires_at: Instant,
}

/// 统计信息缓存
#[cfg(feature = "web")]
pub struct StatsCache {
    /// 缓存存储
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// 默认TTL（存活时间）
    default_ttl: Duration,
}

#[cfg(feature = "web")]
impl StatsCache {
    /// 创建新的缓存实例
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_ttl,
        }
    }
    
    /// 生成缓存键
    fn generate_key(&self, detailed: bool, days: Option<i32>) -> String {
        match (detailed, days) {
            (true, Some(d)) => format!("stats:detailed:{}d", d),
            (true, None) => "stats:detailed:all".to_string(),
            (false, Some(d)) => format!("stats:basic:{}d", d),
            (false, None) => "stats:basic:all".to_string(),
        }
    }
    
    /// 获取缓存数据
    pub async fn get(&self, detailed: bool, days: Option<i32>) -> Option<LibraryStatsResponse> {
        let key = self.generate_key(detailed, days);
        let cache = self.cache.read().await;
        
        if let Some(entry) = cache.get(&key) {
            if Instant::now() < entry.expires_at {
                return Some(entry.data.clone());
            }
        }
        
        None
    }
    
    /// 设置缓存数据
    pub async fn set(&self, detailed: bool, days: Option<i32>, data: LibraryStatsResponse) {
        self.set_with_ttl(detailed, days, data, self.default_ttl).await;
    }
    
    /// 设置缓存数据（自定义TTL）
    pub async fn set_with_ttl(&self, detailed: bool, days: Option<i32>, data: LibraryStatsResponse, ttl: Duration) {
        let key = self.generate_key(detailed, days);
        let now = Instant::now();
        
        let entry = CacheEntry {
            data,
            created_at: now,
            expires_at: now + ttl,
        };
        
        let mut cache = self.cache.write().await;
        cache.insert(key, entry);
    }
    
    /// 清理过期缓存
    pub async fn cleanup_expired(&self) {
        let mut cache = self.cache.write().await;
        let now = Instant::now();
        
        cache.retain(|_, entry| now < entry.expires_at);
    }
    
    /// 清空所有缓存
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
    
    /// 获取缓存统计信息
    pub async fn get_cache_info(&self) -> CacheInfo {
        let cache = self.cache.read().await;
        let now = Instant::now();
        
        let total_entries = cache.len();
        let expired_entries = cache.values().filter(|entry| now >= entry.expires_at).count();
        let active_entries = total_entries - expired_entries;
        
        let total_memory_bytes = cache.len() * std::mem::size_of::<CacheEntry>();
        
        CacheInfo {
            total_entries,
            active_entries,
            expired_entries,
            total_memory_bytes,
            hit_count: 0, // 需要额外的计数器来跟踪
            miss_count: 0, // 需要额外的计数器来跟踪
        }
    }
    
    /// 使不同类型的统计缓存失效
    pub async fn invalidate_stats(&self) {
        let mut cache = self.cache.write().await;
        cache.retain(|key, _| !key.starts_with("stats:"));
    }
    
    /// 获取所有缓存键
    pub async fn get_cache_keys(&self) -> Vec<String> {
        let cache = self.cache.read().await;
        cache.keys().cloned().collect()
    }
}

/// 缓存信息
#[cfg(feature = "web")]
#[derive(Debug, serde::Serialize)]
pub struct CacheInfo {
    /// 总缓存条目数
    pub total_entries: usize,
    /// 活跃条目数
    pub active_entries: usize,
    /// 过期条目数
    pub expired_entries: usize,
    /// 总内存使用（字节）
    pub total_memory_bytes: usize,
    /// 命中次数
    pub hit_count: u64,
    /// 未命中次数
    pub miss_count: u64,
}

/// 带统计的缓存（包含命中率统计）
#[cfg(feature = "web")]
pub struct StatsCacheWithMetrics {
    /// 基础缓存
    cache: StatsCache,
    /// 命中次数
    hit_count: Arc<tokio::sync::Mutex<u64>>,
    /// 未命中次数
    miss_count: Arc<tokio::sync::Mutex<u64>>,
}

#[cfg(feature = "web")]
impl StatsCacheWithMetrics {
    /// 创建带统计的缓存
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            cache: StatsCache::new(default_ttl),
            hit_count: Arc::new(tokio::sync::Mutex::new(0)),
            miss_count: Arc::new(tokio::sync::Mutex::new(0)),
        }
    }
    
    /// 获取缓存数据（带统计）
    pub async fn get(&self, detailed: bool, days: Option<i32>) -> Option<LibraryStatsResponse> {
        if let Some(data) = self.cache.get(detailed, days).await {
            *self.hit_count.lock().await += 1;
            Some(data)
        } else {
            *self.miss_count.lock().await += 1;
            None
        }
    }
    
    /// 设置缓存数据
    pub async fn set(&self, detailed: bool, days: Option<i32>, data: LibraryStatsResponse) {
        self.cache.set(detailed, days, data).await;
    }
    
    /// 设置缓存数据（自定义TTL）
    pub async fn set_with_ttl(&self, detailed: bool, days: Option<i32>, data: LibraryStatsResponse, ttl: Duration) {
        self.cache.set_with_ttl(detailed, days, data, ttl).await;
    }
    
    /// 获取缓存统计信息
    pub async fn get_cache_info(&self) -> CacheInfo {
        let mut info = self.cache.get_cache_info().await;
        info.hit_count = *self.hit_count.lock().await;
        info.miss_count = *self.miss_count.lock().await;
        info
    }
    
    /// 清理过期缓存
    pub async fn cleanup_expired(&self) {
        self.cache.cleanup_expired().await;
    }
    
    /// 清空所有缓存和统计
    pub async fn clear(&self) {
        self.cache.clear().await;
        *self.hit_count.lock().await = 0;
        *self.miss_count.lock().await = 0;
    }
    
    /// 使统计缓存失效
    pub async fn invalidate_stats(&self) {
        self.cache.invalidate_stats().await;
    }
}

/// 默认缓存TTL配置
#[cfg(feature = "web")]
pub struct CacheConfig {
    /// 基础统计TTL（5分钟）
    pub basic_stats_ttl: Duration,
    /// 详细统计TTL（15分钟）
    pub detailed_stats_ttl: Duration,
    /// 今日统计TTL（1分钟）
    pub today_stats_ttl: Duration,
    /// 历史统计TTL（1小时）
    pub historical_stats_ttl: Duration,
}

#[cfg(feature = "web")]
impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            basic_stats_ttl: Duration::from_secs(300),      // 5分钟
            detailed_stats_ttl: Duration::from_secs(900),   // 15分钟
            today_stats_ttl: Duration::from_secs(60),       // 1分钟
            historical_stats_ttl: Duration::from_secs(3600), // 1小时
        }
    }
}

// 非 web feature 的占位类型
#[cfg(not(feature = "web"))]
pub struct StatsCache;

#[cfg(not(feature = "web"))]
pub struct StatsCacheWithMetrics;

#[cfg(not(feature = "web"))]
pub struct CacheConfig;