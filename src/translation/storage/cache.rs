//! 翻译缓存模块（简化版）
//!
//! 提供简单高效的翻译结果缓存功能。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};


// ============================================================================
// 核心类型
// ============================================================================

/// 缓存条目
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub original_text: String,
    pub translated_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub created_at: Instant,
    pub access_count: u64,
    pub last_accessed: Instant,
}

/// 缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_size: usize,
    pub ttl: Duration,
    pub enable_local_cache: bool,
    pub local_cache_size: usize,
    pub default_ttl: Duration,
    pub enable_warmup: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: 1000,
            ttl: Duration::from_secs(3600),
            enable_local_cache: true,
            local_cache_size: 1000,
            default_ttl: Duration::from_secs(3600),
            enable_warmup: false,
        }
    }
}

/// 缓存统计信息
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub total_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub total_entries: usize,
    pub evictions: u64,
}

/// 缓存管理器
pub struct CacheManager {
    cache: TranslationCache,
}

/// 翻译缓存
pub struct TranslationCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    max_size: usize,
    ttl: Duration,
    stats: Arc<RwLock<CacheStats>>,
}

// ============================================================================
// 实现
// ============================================================================

impl CacheEntry {
    /// 创建新的缓存条目
    pub fn new(
        original_text: String,
        translated_text: String,
        source_lang: String,
        target_lang: String,
    ) -> Self {
        let now = Instant::now();
        Self {
            original_text,
            translated_text,
            source_lang,
            target_lang,
            created_at: now,
            access_count: 0,
            last_accessed: now,
        }
    }

    /// 更新访问信息
    pub fn access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Instant::now();
    }

    /// 检查条目是否过期
    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }

    /// 生成缓存键
    pub fn cache_key(&self) -> String {
        format!("{}:{}:{}", self.source_lang, self.target_lang, self.original_text)
    }
}

impl CacheManager {
    /// 创建新的缓存管理器
    pub fn new(config: CacheConfig) -> Self {
        let max_size = if config.enable_local_cache {
            config.local_cache_size
        } else {
            config.max_size
        };
        let ttl = config.default_ttl;
        
        Self {
            cache: TranslationCache::with_config(max_size, ttl),
        }
    }

    /// 获取缓存条目
    pub fn get(&self, key: &str) -> Option<String> {
        self.cache.get(key)
    }

    /// 插入缓存条目
    pub fn insert(&self, original: String, translated: String) {
        self.cache.insert(original, translated);
    }

    /// 插入带语言信息的缓存条目
    pub fn insert_with_lang(
        &self,
        original: String,
        translated: String,
        source_lang: String,
        target_lang: String,
    ) {
        self.cache.insert_with_lang(original, translated, source_lang, target_lang);
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> CacheStats {
        self.cache.get_stats()
    }

    /// 清空缓存
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// 清理过期条目
    pub fn cleanup_expired(&self) -> usize {
        self.cache.cleanup_expired()
    }
}

impl TranslationCache {
    /// 创建新的翻译缓存
    pub fn new() -> Self {
        Self::with_config(1000, Duration::from_secs(3600))
    }

    /// 使用指定配置创建缓存
    pub fn with_config(max_size: usize, ttl: Duration) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            ttl,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// 获取缓存条目
    pub fn get(&self, text: &str) -> Option<String> {
        let mut stats = self.stats.write().unwrap();
        stats.total_requests += 1;

        let mut entries = self.entries.write().unwrap();
        
        if let Some(entry) = entries.get_mut(text) {
            if !entry.is_expired(self.ttl) {
                entry.access();
                stats.cache_hits += 1;
                return Some(entry.translated_text.clone());
            } else {
                // 删除过期条目
                entries.remove(text);
            }
        }

        stats.cache_misses += 1;
        None
    }

    /// 插入缓存条目
    pub fn insert(&self, original: String, translated: String) {
        self.insert_with_lang(original, translated, "auto".to_string(), "zh".to_string())
    }

    /// 插入带语言信息的缓存条目
    pub fn insert_with_lang(
        &self,
        original: String,
        translated: String,
        source_lang: String,
        target_lang: String,
    ) {
        let mut entries = self.entries.write().unwrap();
        
        // 如果达到最大容量，先清理
        if entries.len() >= self.max_size {
            self.evict_lru(&mut entries);
        }

        let entry = CacheEntry::new(original.clone(), translated, source_lang, target_lang);
        entries.insert(original, entry);

        // 更新统计
        let mut stats = self.stats.write().unwrap();
        stats.total_entries = entries.len();
    }

    /// 清空缓存
    pub fn clear(&self) {
        let mut entries = self.entries.write().unwrap();
        entries.clear();

        let mut stats = self.stats.write().unwrap();
        stats.total_entries = 0;
    }

    /// 清理过期条目
    pub fn cleanup_expired(&self) -> usize {
        let mut entries = self.entries.write().unwrap();
        let initial_size = entries.len();

        entries.retain(|_, entry| !entry.is_expired(self.ttl));

        let removed = initial_size - entries.len();
        
        let mut stats = self.stats.write().unwrap();
        stats.total_entries = entries.len();
        stats.evictions += removed as u64;

        removed
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> CacheStats {
        let stats = self.stats.read().unwrap();
        let entries = self.entries.read().unwrap();
        
        let mut result = stats.clone();
        result.total_entries = entries.len();
        result
    }

    /// 获取缓存命中率
    pub fn hit_rate(&self) -> f64 {
        let stats = self.stats.read().unwrap();
        if stats.total_requests == 0 {
            0.0
        } else {
            stats.cache_hits as f64 / stats.total_requests as f64
        }
    }

    /// 获取缓存大小
    pub fn size(&self) -> usize {
        let entries = self.entries.read().unwrap();
        entries.len()
    }

    /// 检查是否包含指定键
    pub fn contains_key(&self, key: &str) -> bool {
        let entries = self.entries.read().unwrap();
        entries.contains_key(key) && !entries[key].is_expired(self.ttl)
    }

    /// LRU驱逐算法
    fn evict_lru(&self, entries: &mut HashMap<String, CacheEntry>) {
        if entries.is_empty() {
            return;
        }

        // 找到最久未访问的条目
        let oldest_key = entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(key, _)| key.clone());

        if let Some(key) = oldest_key {
            entries.remove(&key);
            let mut stats = self.stats.write().unwrap();
            stats.evictions += 1;
        }
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        let mut stats = self.stats.write().unwrap();
        *stats = CacheStats::default();
    }

    /// 获取所有缓存键
    pub fn keys(&self) -> Vec<String> {
        let entries = self.entries.read().unwrap();
        entries.keys().cloned().collect()
    }

    /// 批量获取
    pub fn get_batch(&self, keys: &[String]) -> HashMap<String, String> {
        let mut results = HashMap::new();
        
        for key in keys {
            if let Some(value) = self.get(key) {
                results.insert(key.clone(), value);
            }
        }
        
        results
    }

    /// 批量插入
    pub fn insert_batch(&self, items: Vec<(String, String)>) {
        for (original, translated) in items {
            self.insert(original, translated);
        }
    }
}

impl Default for TranslationCache {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 实用函数
// ============================================================================

/// 创建默认缓存
pub fn create_default_cache() -> TranslationCache {
    TranslationCache::new()
}

/// 创建指定大小的缓存
pub fn create_cache_with_size(max_size: usize) -> TranslationCache {
    TranslationCache::with_config(max_size, Duration::from_secs(3600))
}

/// 生成缓存键
pub fn generate_cache_key(text: &str, source_lang: &str, target_lang: &str) -> String {
    format!("{}:{}:{}", source_lang, target_lang, text)
}

impl CacheStats {
    /// 计算缓存命中率
    pub fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.cache_hits as f64 / self.total_requests as f64
        }
    }

    /// 计算缓存未命中率
    pub fn miss_rate(&self) -> f64 {
        1.0 - self.hit_rate()
    }

    /// 重置统计信息
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// 合并统计信息
    pub fn merge(&mut self, other: &CacheStats) {
        self.total_requests += other.total_requests;
        self.cache_hits += other.cache_hits;
        self.cache_misses += other.cache_misses;
        self.evictions += other.evictions;
        // total_entries使用当前值，因为它是快照
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic_operations() {
        let cache = TranslationCache::new();
        
        // 测试插入和获取
        cache.insert("hello".to_string(), "你好".to_string());
        assert_eq!(cache.get("hello"), Some("你好".to_string()));
        assert_eq!(cache.get("world"), None);
        
        // 测试大小
        assert_eq!(cache.size(), 1);
        
        // 测试清空
        cache.clear();
        assert_eq!(cache.size(), 0);
        assert_eq!(cache.get("hello"), None);
    }

    #[test]
    fn test_cache_stats() {
        let cache = TranslationCache::new();
        
        cache.insert("hello".to_string(), "你好".to_string());
        
        // 命中
        cache.get("hello");
        // 未命中
        cache.get("world");
        
        let stats = cache.get_stats();
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 1);
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.hit_rate(), 0.5);
    }

    #[test]
    fn test_cache_expiration() {
        let cache = TranslationCache::with_config(100, Duration::from_millis(1));
        
        cache.insert("hello".to_string(), "你好".to_string());
        assert_eq!(cache.get("hello"), Some("你好".to_string()));
        
        // 等待过期
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(cache.get("hello"), None);
    }

    #[test]
    fn test_batch_operations() {
        let cache = TranslationCache::new();
        
        let items = vec![
            ("hello".to_string(), "你好".to_string()),
            ("world".to_string(), "世界".to_string()),
        ];
        
        cache.insert_batch(items);
        
        let keys = vec!["hello".to_string(), "world".to_string(), "missing".to_string()];
        let results = cache.get_batch(&keys);
        
        assert_eq!(results.len(), 2);
        assert_eq!(results.get("hello"), Some(&"你好".to_string()));
        assert_eq!(results.get("world"), Some(&"世界".to_string()));
        assert!(!results.contains_key("missing"));
    }

    #[test]
    fn test_lru_eviction() {
        let cache = TranslationCache::with_config(2, Duration::from_secs(3600));
        
        cache.insert("1".to_string(), "一".to_string());
        cache.insert("2".to_string(), "二".to_string());
        assert_eq!(cache.size(), 2);
        
        // 访问第一个，使其成为最近使用的
        cache.get("1");
        
        // 插入第三个，应该驱逐第二个
        cache.insert("3".to_string(), "三".to_string());
        assert_eq!(cache.size(), 2);
        
        assert_eq!(cache.get("1"), Some("一".to_string()));
        assert_eq!(cache.get("2"), None); // 应该被驱逐
        assert_eq!(cache.get("3"), Some("三".to_string()));
    }
}