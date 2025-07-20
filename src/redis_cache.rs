//! Redis 缓存模块
//!
//! 为 web 端提供翻译结果的 Redis 缓存支持

use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "web")]
use redis::{Client, RedisError, RedisResult};

#[cfg(feature = "web")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "web")]
use uuid::Uuid;

/// 缓存的翻译结果
#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTranslation {
    pub id: String,
    pub url: String,
    pub original_html: String,
    pub translated_html: String,
    pub title: Option<String>,
    pub source_lang: String,
    pub target_lang: String,
    pub created_at: u64,
    pub expires_at: u64,
}

/// 缓存统计信息
#[cfg(feature = "web")]
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_keys: usize,
    pub total_size_bytes: usize,
    pub expired_keys: usize,
    pub hit_count: u64,
    pub miss_count: u64,
}

/// Redis 缓存配置
#[cfg(feature = "web")]
#[derive(Debug, Clone)]
pub struct RedisCacheConfig {
    pub url: String,
    pub default_ttl: u64, // 默认过期时间（秒）
    pub key_prefix: String,
}

impl Default for RedisCacheConfig {
    fn default() -> Self {
        Self {
            url: "redis://redis.markdown-library.orb.local:6379".to_string(),
            default_ttl: 3600 * 24, // 24小时
            key_prefix: "monolith:translation:".to_string(),
        }
    }
}

/// Redis 缓存客户端
#[cfg(feature = "web")]
#[derive(Clone)]
pub struct RedisCache {
    client: Client,
    config: RedisCacheConfig,
}

#[cfg(feature = "web")]
impl RedisCache {
    /// 创建新的 Redis 缓存实例
    pub fn new(config: RedisCacheConfig) -> RedisResult<Self> {
        let client = Client::open(config.url.as_str())?;
        Ok(Self { client, config })
    }

    /// 使用默认配置创建 Redis 缓存实例
    pub fn with_default_config() -> RedisResult<Self> {
        Self::new(RedisCacheConfig::default())
    }

    /// 测试 Redis 连接
    pub fn test_connection(&self) -> RedisResult<()> {
        let mut conn = self.client.get_connection()?;
        redis::cmd("PING").execute(&mut conn);
        Ok(())
    }

    /// 生成缓存键
    fn generate_cache_key(&self, url: &str, source_lang: &str, target_lang: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        format!("{}:{}:{}", url, source_lang, target_lang).hash(&mut hasher);
        let hash = hasher.finish();
        
        format!("{}{:x}", self.config.key_prefix, hash)
    }

    /// 存储翻译结果到缓存
    pub fn set(&self, translation: &CachedTranslation) -> RedisResult<()> {
        let mut conn = self.client.get_connection()?;
        let key = self.generate_cache_key(
            &translation.url,
            &translation.source_lang,
            &translation.target_lang,
        );

        let serialized = serde_json::to_string(translation)
            .map_err(|e| RedisError::from((redis::ErrorKind::TypeError, "Serialization failed", e.to_string())))?;

        let ttl = translation.expires_at.saturating_sub(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );

        if ttl > 0 {
            let _: () = redis::cmd("SETEX")
                .arg(&key)
                .arg(ttl)
                .arg(&serialized)
                .query(&mut conn)?;
        }

        Ok(())
    }

    /// 从缓存获取翻译结果
    pub fn get(&self, url: &str, source_lang: &str, target_lang: &str) -> RedisResult<Option<CachedTranslation>> {
        let mut conn = self.client.get_connection()?;
        let key = self.generate_cache_key(url, source_lang, target_lang);

        let cached: Option<String> = redis::cmd("GET").arg(&key).query(&mut conn)?;

        match cached {
            Some(data) => {
                match serde_json::from_str::<CachedTranslation>(&data) {
                    Ok(translation) => {
                        let now = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();

                        if translation.expires_at > now {
                            Ok(Some(translation))
                        } else {
                            // 过期，删除缓存
                            let _: () = redis::cmd("DEL").arg(&key).query(&mut conn)?;
                            Ok(None)
                        }
                    }
                    Err(_) => {
                        // 反序列化失败，删除损坏的缓存
                        let _: () = redis::cmd("DEL").arg(&key).query(&mut conn)?;
                        Ok(None)
                    }
                }
            }
            None => Ok(None),
        }
    }

    /// 删除特定的缓存条目
    pub fn delete(&self, url: &str, source_lang: &str, target_lang: &str) -> RedisResult<bool> {
        let mut conn = self.client.get_connection()?;
        let key = self.generate_cache_key(url, source_lang, target_lang);
        let deleted: u32 = redis::cmd("DEL").arg(&key).query(&mut conn)?;
        Ok(deleted > 0)
    }

    /// 清空所有缓存
    pub fn clear_all(&self) -> RedisResult<u32> {
        let mut conn = self.client.get_connection()?;
        let pattern = format!("{}*", self.config.key_prefix);
        
        // 获取所有匹配的键
        let keys: Vec<String> = redis::cmd("KEYS").arg(&pattern).query(&mut conn)?;
        
        if keys.is_empty() {
            return Ok(0);
        }

        // 删除所有键
        let deleted: u32 = redis::cmd("DEL").arg(&keys).query(&mut conn)?;
        Ok(deleted)
    }

    /// 获取缓存统计信息
    pub fn get_stats(&self) -> RedisResult<CacheStats> {
        let mut conn = self.client.get_connection()?;
        let pattern = format!("{}*", self.config.key_prefix);
        
        // 获取所有匹配的键
        let keys: Vec<String> = redis::cmd("KEYS").arg(&pattern).query(&mut conn)?;
        
        let mut total_size_bytes = 0;
        let mut expired_keys = 0;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for key in &keys {
            // 获取键的大小
            let size: usize = redis::cmd("MEMORY")
                .arg("USAGE")
                .arg(key)
                .query(&mut conn)
                .unwrap_or(0);
            total_size_bytes += size;

            // 检查是否过期
            if let Ok(Some(data)) = redis::cmd("GET").arg(key).query::<Option<String>>(&mut conn) {
                if let Ok(translation) = serde_json::from_str::<CachedTranslation>(&data) {
                    if translation.expires_at <= now {
                        expired_keys += 1;
                    }
                }
            }
        }

        Ok(CacheStats {
            total_keys: keys.len(),
            total_size_bytes,
            expired_keys,
            hit_count: 0, // TODO: 实现计数器
            miss_count: 0, // TODO: 实现计数器
        })
    }

    /// 清理过期的缓存条目
    pub fn cleanup_expired(&self) -> RedisResult<u32> {
        let mut conn = self.client.get_connection()?;
        let pattern = format!("{}*", self.config.key_prefix);
        
        let keys: Vec<String> = redis::cmd("KEYS").arg(&pattern).query(&mut conn)?;
        let mut deleted_count = 0;
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for key in keys {
            if let Ok(Some(data)) = redis::cmd("GET").arg(&key).query::<Option<String>>(&mut conn) {
                if let Ok(translation) = serde_json::from_str::<CachedTranslation>(&data) {
                    if translation.expires_at <= now {
                        let _: u32 = redis::cmd("DEL").arg(&key).query(&mut conn)?;
                        deleted_count += 1;
                    }
                }
            }
        }

        Ok(deleted_count)
    }
}

/// 创建缓存的翻译结果
#[cfg(feature = "web")]
pub fn create_cached_translation(
    url: String,
    original_html: String,
    translated_html: String,
    title: Option<String>,
    source_lang: String,
    target_lang: String,
    ttl_seconds: Option<u64>,
) -> CachedTranslation {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let ttl = ttl_seconds.unwrap_or(3600 * 24); // 默认24小时
    
    CachedTranslation {
        id: Uuid::new_v4().to_string(),
        url,
        original_html,
        translated_html,
        title,
        source_lang,
        target_lang,
        created_at: now,
        expires_at: now + ttl,
    }
}

/// 非 web feature 的占位类型和函数
#[cfg(not(feature = "web"))]
pub struct RedisCache;

#[cfg(not(feature = "web"))]
impl RedisCache {
    pub fn with_default_config() -> Result<Self, String> {
        Err("Web feature not enabled".to_string())
    }
}