//! 多级缓存系统
//!
//! 提供本地LRU缓存和Redis分布式缓存

// use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

#[cfg(feature = "translation")]
use blake3::Hasher;
#[cfg(feature = "translation")]
use lru::LruCache;
#[cfg(feature = "translation")]
use std::num::NonZeroUsize;
#[cfg(feature = "translation")]
use tokio::sync::RwLock;

use crate::translation::error::TranslationResult;

/// 缓存条目
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub key: String,
    pub original_text: String,
    pub translated_text: String,
    pub source_lang: String,
    pub target_lang: String,
    pub created_at: SystemTime,
    pub ttl: Duration,
}

impl CacheEntry {
    /// 检查是否过期
    pub fn is_expired(&self) -> bool {
        if let Ok(elapsed) = self.created_at.elapsed() {
            elapsed > self.ttl
        } else {
            true
        }
    }

    /// 生成缓存键
    #[cfg(feature = "translation")]
    pub fn generate_key(text: &str, source_lang: &str, target_lang: &str) -> String {
        let mut hasher = Hasher::new();
        hasher.update(text.as_bytes());
        hasher.update(source_lang.as_bytes());
        hasher.update(target_lang.as_bytes());
        format!("trans:{}", hasher.finalize().to_hex())
    }

    #[cfg(not(feature = "translation"))]
    pub fn generate_key(text: &str, source_lang: &str, target_lang: &str) -> String {
        format!("trans:{}:{}:{}", source_lang, target_lang, text.len())
    }
}

/// 本地LRU缓存
#[cfg(feature = "translation")]
pub struct LocalCache {
    cache: Arc<RwLock<LruCache<String, CacheEntry>>>,
    stats: CacheStats,
}

#[cfg(feature = "translation")]
impl LocalCache {
    /// 创建新的本地缓存
    pub fn new(capacity: usize) -> Self {
        let cache = Arc::new(RwLock::new(
            LruCache::new(NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(1000).unwrap()))
        ));

        Self {
            cache,
            stats: CacheStats::default(),
        }
    }

    /// 获取缓存项
    pub async fn get(&mut self, key: &str) -> Option<CacheEntry> {
        let mut cache = self.cache.write().await;
        
        if let Some(entry) = cache.get(key) {
            if entry.is_expired() {
                cache.pop(key);
                self.stats.expired_items += 1;
                None
            } else {
                self.stats.hits += 1;
                Some(entry.clone())
            }
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// 设置缓存项
    pub async fn set(&mut self, entry: CacheEntry) {
        let mut cache = self.cache.write().await;
        cache.put(entry.key.clone(), entry);
        self.stats.sets += 1;
    }

    /// 删除缓存项
    pub async fn remove(&mut self, key: &str) -> bool {
        let mut cache = self.cache.write().await;
        cache.pop(key).is_some()
    }

    /// 清理过期项
    pub async fn cleanup_expired(&mut self) -> usize {
        let mut cache = self.cache.write().await;
        let mut expired_keys = Vec::new();

        // 收集过期的键
        for (key, entry) in cache.iter() {
            if entry.is_expired() {
                expired_keys.push(key.clone());
            }
        }

        // 删除过期项
        for key in &expired_keys {
            cache.pop(key);
        }

        self.stats.expired_items += expired_keys.len();
        expired_keys.len()
    }

    /// 获取缓存大小
    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// 清空缓存
    pub async fn clear(&mut self) {
        let mut cache = self.cache.write().await;
        cache.clear();
        self.stats.clears += 1;
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> &CacheStats {
        &self.stats
    }
}

/// 多级缓存管理器
pub struct CacheManager {
    #[cfg(feature = "translation")]
    local_cache: Option<LocalCache>,
    #[cfg(feature = "web")]
    redis_cache: Option<crate::redis_cache::RedisCache>,
    #[cfg(not(feature = "web"))]
    redis_cache: Option<()>,
    config: CacheConfig,
    stats: CacheStats,
}

impl CacheManager {
    /// 创建新的缓存管理器
    pub fn new(config: CacheConfig) -> Self {
        #[cfg(feature = "translation")]
        let local_cache = if config.enable_local_cache {
            Some(LocalCache::new(config.local_cache_size))
        } else {
            None
        };

        Self {
            #[cfg(feature = "translation")]
            local_cache,
            redis_cache: None,
            config,
            stats: CacheStats::default(),
        }
    }

    /// 设置Redis缓存
    #[cfg(feature = "web")]
    pub fn set_redis_cache(&mut self, redis_cache: crate::redis_cache::RedisCache) {
        self.redis_cache = Some(redis_cache);
    }
    
    #[cfg(not(feature = "web"))]
    pub fn set_redis_cache(&mut self, _redis_cache: ()) {
        // No-op when web feature is not enabled
    }

    /// 获取翻译缓存
    pub async fn get_translation(
        &mut self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> TranslationResult<Option<String>> {
        let key = CacheEntry::generate_key(text, source_lang, target_lang);

        // 先查本地缓存
        #[cfg(feature = "translation")]
        if let Some(ref mut local_cache) = self.local_cache {
            if let Some(entry) = local_cache.get(&key).await {
                self.stats.local_hits += 1;
                return Ok(Some(entry.translated_text));
            }
        }

        // 再查Redis缓存
        #[cfg(feature = "web")]
        if let Some(ref redis_cache) = self.redis_cache {
            match redis_cache.get(text, source_lang, target_lang) {
                Ok(Some(cached)) => {
                    self.stats.redis_hits += 1;
                    
                    // 回填本地缓存
                    #[cfg(feature = "translation")]
                    if let Some(ref mut local_cache) = self.local_cache {
                        let entry = CacheEntry {
                            key: key.clone(),
                            original_text: text.to_string(),
                            translated_text: cached.translated_html.clone(),
                            source_lang: source_lang.to_string(),
                            target_lang: target_lang.to_string(),
                            created_at: SystemTime::now(),
                            ttl: self.config.default_ttl,
                        };
                        local_cache.set(entry).await;
                    }
                    
                    return Ok(Some(cached.translated_html));
                }
                Ok(None) => {
                    self.stats.redis_misses += 1;
                }
                Err(e) => {
                    tracing::warn!("Redis缓存查询失败: {}", e);
                    self.stats.redis_errors += 1;
                }
            }
        }

        self.stats.total_misses += 1;
        Ok(None)
    }

    /// 设置翻译缓存
    pub async fn set_translation(
        &mut self,
        text: &str,
        source_lang: &str,
        target_lang: &str,
        translated: &str,
    ) -> TranslationResult<()> {
        let key = CacheEntry::generate_key(text, source_lang, target_lang);
        
        let entry = CacheEntry {
            key: key.clone(),
            original_text: text.to_string(),
            translated_text: translated.to_string(),
            source_lang: source_lang.to_string(),
            target_lang: target_lang.to_string(),
            created_at: SystemTime::now(),
            ttl: self.config.default_ttl,
        };

        // 设置本地缓存
        #[cfg(feature = "translation")]
        if let Some(ref mut local_cache) = self.local_cache {
            local_cache.set(entry.clone()).await;
            self.stats.local_sets += 1;
        }

        // 设置Redis缓存
        #[cfg(feature = "web")]
        if let Some(ref redis_cache) = self.redis_cache {
            let cached_translation = crate::redis_cache::create_cached_translation(
                format!("cache://{}", key),
                text.to_string(),
                translated.to_string(),
                None,
                source_lang.to_string(),
                target_lang.to_string(),
                Some(self.config.default_ttl.as_secs()),
            );

            if let Err(e) = redis_cache.set(&cached_translation) {
                tracing::warn!("Redis缓存设置失败: {}", e);
                self.stats.redis_errors += 1;
            } else {
                self.stats.redis_sets += 1;
            }
        }

        Ok(())
    }

    /// 清理过期缓存
    pub async fn cleanup_expired(&mut self) -> TranslationResult<usize> {
        let mut total_cleaned = 0;

        // 清理本地缓存
        #[cfg(feature = "translation")]
        if let Some(ref mut local_cache) = self.local_cache {
            total_cleaned += local_cache.cleanup_expired().await;
        }

        // Redis缓存自动过期，无需手动清理

        Ok(total_cleaned)
    }

    /// 预热缓存
    pub async fn warmup(&mut self, common_texts: Vec<(String, String, String)>) -> TranslationResult<()> {
        if !self.config.enable_warmup {
            return Ok(());
        }

        tracing::info!("开始缓存预热，预热 {} 个常用文本", common_texts.len());

        for (text, source_lang, target_lang) in common_texts {
            // 这里可以预先计算和缓存常用翻译
            // 实际实现中需要调用翻译服务
            let _ = self.get_translation(&text, &source_lang, &target_lang).await;
        }

        Ok(())
    }

    /// 获取缓存统计
    pub fn get_stats(&self) -> &CacheStats {
        &self.stats
    }

    /// 获取缓存大小信息
    pub async fn get_cache_info(&self) -> CacheInfo {
        #[cfg(feature = "translation")]
        let local_size = if let Some(ref local_cache) = self.local_cache {
            local_cache.len().await
        } else {
            0
        };

        #[cfg(not(feature = "translation"))]
        let local_size = 0;

        let redis_size = if let Some(ref _redis_cache) = self.redis_cache {
            // Redis缓存大小查询实现
            0 // 占位符
        } else {
            0
        };

        CacheInfo {
            local_cache_size: local_size,
            redis_cache_size: redis_size,
            total_entries: local_size + redis_size,
            hit_rate: self.stats.hit_rate(),
        }
    }
}

/// 缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// 启用本地缓存
    pub enable_local_cache: bool,
    /// 本地缓存大小
    pub local_cache_size: usize,
    /// 启用Redis缓存
    pub enable_redis_cache: bool,
    /// 默认TTL
    pub default_ttl: Duration,
    /// 启用缓存预热
    pub enable_warmup: bool,
    /// 自动清理间隔
    pub cleanup_interval: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enable_local_cache: true,
            local_cache_size: 1000,
            enable_redis_cache: false,
            default_ttl: Duration::from_secs(3600),
            enable_warmup: false,
            cleanup_interval: Duration::from_secs(300),
        }
    }
}

/// 缓存统计信息
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: usize,
    pub misses: usize,
    pub sets: usize,
    pub local_hits: usize,
    pub local_misses: usize,
    pub local_sets: usize,
    pub redis_hits: usize,
    pub redis_misses: usize,
    pub redis_sets: usize,
    pub redis_errors: usize,
    pub total_misses: usize,
    pub expired_items: usize,
    pub clears: usize,
}

impl CacheStats {
    /// 计算命中率
    pub fn hit_rate(&self) -> f32 {
        let total = self.hits + self.misses;
        if total > 0 {
            self.hits as f32 / total as f32
        } else {
            0.0
        }
    }

    /// 计算本地缓存命中率
    pub fn local_hit_rate(&self) -> f32 {
        let total = self.local_hits + self.local_misses;
        if total > 0 {
            self.local_hits as f32 / total as f32
        } else {
            0.0
        }
    }

    /// 计算Redis缓存命中率
    pub fn redis_hit_rate(&self) -> f32 {
        let total = self.redis_hits + self.redis_misses;
        if total > 0 {
            self.redis_hits as f32 / total as f32
        } else {
            0.0
        }
    }

    /// 重置统计
    pub fn reset(&mut self) {
        *self = Default::default();
    }
}

/// 缓存信息
#[derive(Debug, Clone)]
pub struct CacheInfo {
    pub local_cache_size: usize,
    pub redis_cache_size: usize,
    pub total_entries: usize,
    pub hit_rate: f32,
}

/// 非translation feature的简化实现
#[cfg(not(feature = "translation"))]
pub struct LocalCache;

#[cfg(not(feature = "translation"))]
impl LocalCache {
    pub fn new(_capacity: usize) -> Self {
        Self
    }
}