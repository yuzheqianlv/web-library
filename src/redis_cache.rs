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
            default_ttl: 0, // 0表示永久缓存，手动清除
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

        // 检查是否使用永久缓存（TTL为0）或有指定过期时间
        if self.config.default_ttl == 0 || translation.expires_at == 0 {
            // 永久存储，不设置过期时间
            let _: () = redis::cmd("SET")
                .arg(&key)
                .arg(&serialized)
                .query(&mut conn)?;
        } else {
            // 计算剩余TTL时间
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
            } else {
                // TTL已过期，使用永久存储
                let _: () = redis::cmd("SET")
                    .arg(&key)
                    .arg(&serialized)
                    .query(&mut conn)?;
            }
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
                        // 永久缓存模式：不检查过期时间，直接返回
                        if self.config.default_ttl == 0 || translation.expires_at == 0 {
                            Ok(Some(translation))
                        } else {
                            // 仍然支持有TTL的缓存（向后兼容）
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
        let mut expired_keys = 0; // 在永久缓存模式下，这表示有明确过期时间且已过期的旧缓存
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

            // 只计算有明确过期时间且已过期的缓存
            if let Ok(Some(data)) = redis::cmd("GET").arg(key).query::<Option<String>>(&mut conn) {
                if let Ok(translation) = serde_json::from_str::<CachedTranslation>(&data) {
                    // 只有明确设置了过期时间（非0）且已过期的才被计为expired
                    if translation.expires_at > 0 && translation.expires_at <= now {
                        expired_keys += 1;
                    }
                }
            }
        }

        Ok(CacheStats {
            total_keys: keys.len(),
            total_size_bytes,
            expired_keys, // 现在表示需要清理的旧缓存数量
            hit_count: 0, // TODO: 实现计数器
            miss_count: 0, // TODO: 实现计数器
        })
    }

    /// 清理过期的缓存条目（永久缓存模式下此方法主要用于清理损坏的数据）
    pub fn cleanup_expired(&self) -> RedisResult<u32> {
        let mut conn = self.client.get_connection()?;
        let pattern = format!("{}*", self.config.key_prefix);
        
        let keys: Vec<String> = redis::cmd("KEYS").arg(&pattern).query(&mut conn)?;
        let mut deleted_count = 0;
        
        // 在永久缓存模式下，只清理损坏的数据或明确过期的旧缓存
        for key in keys {
            if let Ok(Some(data)) = redis::cmd("GET").arg(&key).query::<Option<String>>(&mut conn) {
                match serde_json::from_str::<CachedTranslation>(&data) {
                    Ok(translation) => {
                        // 只删除明确设置了过期时间且已过期的旧缓存（向后兼容）
                        if translation.expires_at > 0 {
                            let now = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs();
                            
                            if translation.expires_at <= now {
                                let _: u32 = redis::cmd("DEL").arg(&key).query(&mut conn)?;
                                deleted_count += 1;
                            }
                        }
                        // expires_at == 0 的缓存被视为永久缓存，不删除
                    }
                    Err(_) => {
                        // 清理损坏的缓存数据
                        let _: u32 = redis::cmd("DEL").arg(&key).query(&mut conn)?;
                        deleted_count += 1;
                    }
                }
            }
        }

        Ok(deleted_count)
    }

    /// 按域名获取缓存数据
    pub fn get_by_domain(&self, domain: &str) -> RedisResult<Vec<CachedTranslation>> {
        let mut conn = self.client.get_connection()?;
        let pattern = format!("{}*", self.config.key_prefix);
        
        let keys: Vec<String> = redis::cmd("KEYS").arg(&pattern).query(&mut conn)?;
        let mut domain_translations = Vec::new();
        
        for key in keys {
            if let Ok(Some(data)) = redis::cmd("GET").arg(&key).query::<Option<String>>(&mut conn) {
                if let Ok(translation) = serde_json::from_str::<CachedTranslation>(&data) {
                    if let Ok(url) = url::Url::parse(&translation.url) {
                        if let Some(host) = url.host_str() {
                            if host == domain || host.ends_with(&format!(".{}", domain)) {
                                domain_translations.push(translation);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(domain_translations)
    }

    /// 获取所有域名的统计信息
    pub fn get_domains_stats(&self) -> RedisResult<std::collections::HashMap<String, (usize, usize, u64)>> {
        let mut conn = self.client.get_connection()?;
        let pattern = format!("{}*", self.config.key_prefix);
        
        let keys: Vec<String> = redis::cmd("KEYS").arg(&pattern).query(&mut conn)?;
        let mut domain_stats: std::collections::HashMap<String, (usize, usize, u64)> = std::collections::HashMap::new();
        
        for key in keys {
            if let Ok(Some(data)) = redis::cmd("GET").arg(&key).query::<Option<String>>(&mut conn) {
                if let Ok(translation) = serde_json::from_str::<CachedTranslation>(&data) {
                    if let Ok(url) = url::Url::parse(&translation.url) {
                        if let Some(host) = url.host_str() {
                            let domain = host.to_string();
                            
                            // 获取键的大小
                            let size: usize = redis::cmd("MEMORY")
                                .arg("USAGE")
                                .arg(&key)
                                .query(&mut conn)
                                .unwrap_or(0);
                            
                            let entry = domain_stats.entry(domain).or_insert((0, 0, 0));
                            entry.0 += 1; // 计数增加
                            entry.1 += size; // 大小增加
                            entry.2 = entry.2.max(translation.created_at); // 最新更新时间
                        }
                    }
                }
            }
        }
        
        Ok(domain_stats)
    }

    /// 删除特定域名的所有缓存
    pub fn delete_domain(&self, domain: &str) -> RedisResult<u32> {
        let translations = self.get_by_domain(domain)?;
        let mut conn = self.client.get_connection()?;
        let mut deleted_count = 0;
        
        for translation in translations {
            let key = self.generate_cache_key(
                &translation.url,
                &translation.source_lang,
                &translation.target_lang,
            );
            let deleted: u32 = redis::cmd("DEL").arg(&key).query(&mut conn)?;
            deleted_count += deleted;
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
    
    // 默认使用永久缓存（TTL为0表示永不过期）
    let ttl = ttl_seconds.unwrap_or(0);
    let expires_at = if ttl == 0 { 0 } else { now + ttl };
    
    CachedTranslation {
        id: Uuid::new_v4().to_string(),
        url,
        original_html,
        translated_html,
        title,
        source_lang,
        target_lang,
        created_at: now,
        expires_at,
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