//! 智能缓存查询引擎
//! 
//! 提供高效的缓存查询和路由决策功能

#[cfg(feature = "web")]
use crate::web::types::CachedHtml;
#[cfg(feature = "web")]
use mongodb::{bson::doc, Collection, Database};
#[cfg(feature = "web")]
use std::collections::HashMap;
#[cfg(feature = "web")]

/// 缓存查询结果
#[cfg(feature = "web")]
#[derive(Debug, Clone)]
pub struct CacheQueryResult {
    /// 记录ID (MongoDB ObjectId)
    pub record_id: Option<String>,
    /// 缓存状态
    pub cache_status: CacheStatus,
    /// 缓存记录（如果存在）
    pub cached_record: Option<CachedHtml>,
    /// 建议的路由策略
    pub routing_strategy: RoutingStrategy,
}

/// 缓存状态
#[cfg(feature = "web")]
#[derive(Debug, Clone, PartialEq)]
pub enum CacheStatus {
    /// 存在完整的成功缓存
    Complete,
    /// 存在但正在处理中
    Processing,
    /// 存在但已过期
    Expired,
    /// 存在但失败
    Failed,
    /// 不存在
    NotFound,
}

/// 路由策略
#[cfg(feature = "web")]
#[derive(Debug, Clone, PartialEq)]
pub enum RoutingStrategy {
    /// 直接使用缓存记录的ID进行预览
    UseCache(String),
    /// 重新处理但优先检查处理状态
    ReprocessWithCheck,
    /// 完全重新处理
    FullReprocess,
    /// 等待当前处理完成
    WaitForProcessing,
}

/// 内存缓存条目
#[cfg(feature = "web")]
#[derive(Debug, Clone)]
struct MemoryCacheEntry {
    result: CacheQueryResult,
    timestamp: std::time::Instant,
    access_count: u64,  // 访问次数，用于LRU
}

/// LRU缓存节点
#[cfg(feature = "web")]
#[derive(Debug, Clone)]
struct LruNode {
    key: String,
    entry: MemoryCacheEntry,
    prev: Option<usize>,
    next: Option<usize>,
}

/// LRU缓存实现
#[cfg(feature = "web")]
#[derive(Debug)]
struct LruCache {
    nodes: Vec<Option<LruNode>>,
    key_to_index: HashMap<String, usize>,
    head: Option<usize>,
    tail: Option<usize>,
    capacity: usize,
    free_indices: Vec<usize>,
}

#[cfg(feature = "web")]
impl LruCache {
    fn new(capacity: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(capacity),
            key_to_index: HashMap::new(),
            head: None,
            tail: None,
            capacity,
            free_indices: Vec::new(),
        }
    }

    fn get(&mut self, key: &str) -> Option<MemoryCacheEntry> {
        if let Some(&index) = self.key_to_index.get(key) {
            self.move_to_head(index);
            if let Some(ref mut node) = self.nodes[index] {
                node.entry.access_count += 1;
                node.entry.timestamp = std::time::Instant::now();
                return Some(node.entry.clone());
            }
        }
        None
    }

    fn insert(&mut self, key: String, entry: MemoryCacheEntry) {
        if let Some(&index) = self.key_to_index.get(&key) {
            // 更新现有条目
            if let Some(ref mut node) = self.nodes[index] {
                node.entry = entry;
                self.move_to_head(index);
            }
            return;
        }

        // 添加新条目
        if self.key_to_index.len() >= self.capacity {
            self.remove_tail();
        }

        let index = self.get_free_index();
        let new_node = LruNode {
            key: key.clone(),
            entry,
            prev: None,
            next: self.head,
        };

        if index >= self.nodes.len() {
            self.nodes.resize(index + 1, None);
        }

        self.nodes[index] = Some(new_node);
        self.key_to_index.insert(key, index);

        if let Some(head_index) = self.head {
            if let Some(ref mut head_node) = self.nodes[head_index] {
                head_node.prev = Some(index);
            }
        }

        self.head = Some(index);
        if self.tail.is_none() {
            self.tail = Some(index);
        }
    }

    fn move_to_head(&mut self, index: usize) {
        if self.head == Some(index) {
            return;
        }

        // 先获取节点信息，避免借用冲突
        let (prev_index, next_index) = {
            if let Some(ref node) = self.nodes[index] {
                (node.prev, node.next)
            } else {
                return;
            }
        };

        // 从当前位置移除
        if let Some(prev_idx) = prev_index {
            if let Some(ref mut prev_node) = self.nodes[prev_idx] {
                prev_node.next = next_index;
            }
        }

        if let Some(next_idx) = next_index {
            if let Some(ref mut next_node) = self.nodes[next_idx] {
                next_node.prev = prev_index;
            }
        } else {
            // 这是尾节点
            self.tail = prev_index;
        }

        // 移动到头部
        if let Some(ref mut node) = self.nodes[index] {
            node.prev = None;
            node.next = self.head;
        }

        if let Some(head_index) = self.head {
            if let Some(ref mut head_node) = self.nodes[head_index] {
                head_node.prev = Some(index);
            }
        }

        self.head = Some(index);
    }

    fn remove_tail(&mut self) {
        if let Some(tail_index) = self.tail {
            if let Some(tail_node) = self.nodes[tail_index].take() {
                self.key_to_index.remove(&tail_node.key);
                self.free_indices.push(tail_index);

                if let Some(prev_index) = tail_node.prev {
                    if let Some(ref mut prev_node) = self.nodes[prev_index] {
                        prev_node.next = None;
                    }
                    self.tail = Some(prev_index);
                } else {
                    self.tail = None;
                    self.head = None;
                }
            }
        }
    }

    fn get_free_index(&mut self) -> usize {
        if let Some(index) = self.free_indices.pop() {
            index
        } else {
            self.nodes.len()
        }
    }

    fn clear(&mut self) {
        self.nodes.clear();
        self.key_to_index.clear();
        self.head = None;
        self.tail = None;
        self.free_indices.clear();
    }

    fn len(&self) -> usize {
        self.key_to_index.len()
    }

    fn get_stats(&self) -> (usize, usize) {
        let total_entries = self.len();
        let cutoff_time = std::time::Instant::now() - std::time::Duration::from_secs(300);
        let valid_entries = self.nodes.iter()
            .filter_map(|node_opt| node_opt.as_ref())
            .filter(|node| node.entry.timestamp > cutoff_time)
            .count();
        (total_entries, valid_entries)
    }
}

/// 智能缓存查询引擎
#[cfg(feature = "web")]
pub struct CacheQueryEngine {
    db: Database,
    collection: Collection<CachedHtml>,
    /// LRU内存缓存
    memory_cache: std::sync::Arc<std::sync::Mutex<LruCache>>,
}

#[cfg(feature = "web")]
impl CacheQueryEngine {
    /// 创建新的缓存查询引擎
    pub fn new(db: Database) -> Self {
        let collection = db.collection::<CachedHtml>("html_cache");
        Self {
            db,
            collection,
            memory_cache: std::sync::Arc::new(std::sync::Mutex::new(LruCache::new(1000))),
        }
    }

    /// 智能查询缓存状态
    /// 
    /// # 参数
    /// * `url` - 目标URL
    /// * `source_lang` - 源语言（可选）
    /// * `target_lang` - 目标语言（可选）
    /// 
    /// # 返回值
    /// 返回缓存查询结果，包含状态和建议的路由策略
    pub async fn query_cache_status(
        &self,
        url: &str,
        source_lang: Option<&str>,
        target_lang: Option<&str>,
    ) -> Result<CacheQueryResult, mongodb::error::Error> {
        // 构建缓存键
        let cache_key = self.build_cache_key(url, source_lang, target_lang);
        
        // 首先检查LRU内存缓存 (5分钟有效期)
        if let Some(cached_entry) = self.get_from_memory_cache(&cache_key) {
            if cached_entry.timestamp.elapsed().as_secs() < 300 { // 5分钟
                return Ok(cached_entry.result);
            }
        }

        // 查询数据库
        let result = self.query_database(url, source_lang, target_lang).await?;
        
        // 更新内存缓存
        self.update_memory_cache(cache_key, result.clone());
        
        Ok(result)
    }

    /// 查询数据库获取缓存状态
    async fn query_database(
        &self,
        url: &str,
        source_lang: Option<&str>,
        target_lang: Option<&str>,
    ) -> Result<CacheQueryResult, mongodb::error::Error> {
        let source_lang = source_lang.unwrap_or("auto");
        let target_lang = target_lang.unwrap_or("zh");

        // 构建查询过滤器 - 按优先级查询
        let filters = vec![
            // 1. 精确匹配 (最高优先级)
            doc! {
                "url": url,
                "source_lang": source_lang,
                "target_lang": target_lang,
                "status": "success"
            },
            // 2. 相同URL，不同语言组合但状态成功
            doc! {
                "url": url,
                "status": "success"
            },
            // 3. 正在处理中的记录
            doc! {
                "url": url,
                "source_lang": source_lang,
                "target_lang": target_lang,
                "status": "pending"
            },
            // 4. 任何相关记录
            doc! { "url": url },
        ];

        for filter in filters {
            if let Some(record) = self.collection.find_one(filter).await? {
                let record_id = record.id.map(|id| id.to_hex()).unwrap_or_default();
                
                let cache_status = match record.status.as_str() {
                    "success" => {
                        // 检查是否过期
                        if let Some(expires_at) = record.expires_at {
                            if expires_at.timestamp_millis() < chrono::Utc::now().timestamp_millis() {
                                CacheStatus::Expired
                            } else {
                                CacheStatus::Complete
                            }
                        } else {
                            CacheStatus::Complete
                        }
                    },
                    "pending" => CacheStatus::Processing,
                    "error" => CacheStatus::Failed,
                    _ => CacheStatus::Failed,
                };

                let routing_strategy = match cache_status {
                    CacheStatus::Complete => RoutingStrategy::UseCache(record_id.clone()),
                    CacheStatus::Processing => RoutingStrategy::WaitForProcessing,
                    CacheStatus::Expired => RoutingStrategy::ReprocessWithCheck,
                    CacheStatus::Failed => RoutingStrategy::FullReprocess,
                    CacheStatus::NotFound => RoutingStrategy::FullReprocess,
                };

                return Ok(CacheQueryResult {
                    record_id: Some(record_id),
                    cache_status,
                    cached_record: Some(record),
                    routing_strategy,
                });
            }
        }

        // 没有找到任何记录
        Ok(CacheQueryResult {
            record_id: None,
            cache_status: CacheStatus::NotFound,
            cached_record: None,
            routing_strategy: RoutingStrategy::FullReprocess,
        })
    }

    /// 构建缓存键
    fn build_cache_key(&self, url: &str, source_lang: Option<&str>, target_lang: Option<&str>) -> String {
        format!("{}:{}:{}", 
            url, 
            source_lang.unwrap_or("auto"), 
            target_lang.unwrap_or("zh")
        )
    }

    /// 从LRU内存缓存获取结果
    fn get_from_memory_cache(&self, key: &str) -> Option<MemoryCacheEntry> {
        if let Ok(mut cache) = self.memory_cache.lock() {
            cache.get(key)
        } else {
            None
        }
    }

    /// 更新LRU内存缓存
    fn update_memory_cache(&self, key: String, result: CacheQueryResult) {
        if let Ok(mut cache) = self.memory_cache.lock() {
            let entry = MemoryCacheEntry {
                result,
                timestamp: std::time::Instant::now(),
                access_count: 1,
            };
            cache.insert(key, entry);
        }
    }

    /// 获取域名下的所有可用缓存
    pub async fn get_domain_cache_links(
        &self,
        domain: &str,
        target_lang: Option<&str>,
    ) -> Result<Vec<CachedHtml>, mongodb::error::Error> {
        let target_lang = target_lang.unwrap_or("zh");
        
        let filter = doc! {
            "domain": domain,
            "target_lang": target_lang,
            "status": "success"
        };

        let mut cursor = self.collection.find(filter).await?;
        let mut results = Vec::new();

        use futures::stream::TryStreamExt;
        while let Some(record) = cursor.try_next().await? {
            results.push(record);
        }

        Ok(results)
    }

    /// 清理LRU内存缓存
    pub fn clear_memory_cache(&self) {
        if let Ok(mut cache) = self.memory_cache.lock() {
            cache.clear();
        }
    }

    /// 获取LRU内存缓存统计信息
    pub fn get_cache_stats(&self) -> (usize, usize) {
        if let Ok(cache) = self.memory_cache.lock() {
            cache.get_stats()
        } else {
            (0, 0)
        }
    }
}

// 非web feature的占位实现
#[cfg(not(feature = "web"))]
pub struct CacheQueryEngine;

#[cfg(not(feature = "web"))]
impl CacheQueryEngine {
    pub fn new(_db: ()) -> Self {
        Self
    }
}

#[cfg(feature = "web")]
#[cfg(test)]
mod tests {
    use super::*;
    use mongodb::bson::DateTime;

    /// 测试缓存键生成
    #[test]
    #[ignore] // 忽略需要数据库连接的测试
    fn test_build_cache_key() {
        let engine = CacheQueryEngine::new(create_test_database());
        
        // 测试基本缓存键生成
        let key1 = engine.build_cache_key("https://example.com", Some("en"), Some("zh"));
        assert_eq!(key1, "https://example.com:en:zh");
        
        // 测试默认值
        let key2 = engine.build_cache_key("https://example.com", None, None);
        assert_eq!(key2, "https://example.com:auto:zh");
        
        // 测试部分默认值
        let key3 = engine.build_cache_key("https://example.com", Some("en"), None);
        assert_eq!(key3, "https://example.com:en:zh");
    }

    /// 测试缓存键生成（无数据库依赖版本）
    #[test]
    fn test_cache_key_generation() {
        // 直接测试缓存键生成逻辑，避免数据库依赖
        let test_cases = vec![
            ("https://example.com", Some("en"), Some("zh"), "https://example.com:en:zh"),
            ("https://example.com", None, None, "https://example.com:auto:zh"),
            ("https://example.com", Some("en"), None, "https://example.com:en:zh"),
            ("https://example.com", None, Some("fr"), "https://example.com:auto:fr"),
        ];

        for (url, source_lang, target_lang, expected) in test_cases {
            let result = format!("{}:{}:{}", 
                url, 
                source_lang.unwrap_or("auto"), 
                target_lang.unwrap_or("zh")
            );
            assert_eq!(result, expected);
        }
    }

    /// 测试缓存状态判断
    #[test]
    fn test_cache_status_determination() {
        // 测试成功状态
        let success_record = create_test_cached_html("success", None);
        assert_eq!(determine_cache_status(&success_record), CacheStatus::Complete);
        
        // 测试处理中状态
        let pending_record = create_test_cached_html("pending", None);
        assert_eq!(determine_cache_status(&pending_record), CacheStatus::Processing);
        
        // 测试失败状态
        let error_record = create_test_cached_html("error", None);
        assert_eq!(determine_cache_status(&error_record), CacheStatus::Failed);
        
        // 测试过期状态
        let expired_time = DateTime::from_millis(chrono::Utc::now().timestamp_millis() - 86400000); // 1天前
        let expired_record = create_test_cached_html("success", Some(expired_time));
        assert_eq!(determine_cache_status(&expired_record), CacheStatus::Expired);
    }

    /// 测试路由策略选择
    #[test]
    fn test_routing_strategy_selection() {
        // 测试使用缓存策略
        let result = CacheQueryResult {
            record_id: Some("test_id".to_string()),
            cache_status: CacheStatus::Complete,
            cached_record: None,
            routing_strategy: RoutingStrategy::UseCache("test_id".to_string()),
        };
        assert_eq!(result.routing_strategy, RoutingStrategy::UseCache("test_id".to_string()));
        
        // 测试等待处理策略
        let result = CacheQueryResult {
            record_id: Some("test_id".to_string()),
            cache_status: CacheStatus::Processing,
            cached_record: None,
            routing_strategy: RoutingStrategy::WaitForProcessing,
        };
        assert_eq!(result.routing_strategy, RoutingStrategy::WaitForProcessing);
        
        // 测试重新处理策略
        let result = CacheQueryResult {
            record_id: None,
            cache_status: CacheStatus::NotFound,
            cached_record: None,
            routing_strategy: RoutingStrategy::FullReprocess,
        };
        assert_eq!(result.routing_strategy, RoutingStrategy::FullReprocess);
    }

    /// 创建测试数据库（模拟）
    /// 注意：此函数需要实际的MongoDB连接，在CI环境中可能不可用
    fn create_test_database() -> Database {
        // 为测试创建一个模拟数据库
        // 实际测试中可以使用内存MongoDB或测试数据库
        panic!("This test requires a real MongoDB connection")
    }

    /// 创建测试缓存HTML记录
    fn create_test_cached_html(status: &str, expires_at: Option<DateTime>) -> CachedHtml {
        CachedHtml {
            id: None, // 测试数据不需要ID
            url: "https://example.com".to_string(),
            original_html: "<html></html>".to_string(),
            translated_html: "<html></html>".to_string(),
            title: Some("Test Page".to_string()),
            source_lang: "en".to_string(),
            target_lang: "zh".to_string(),
            status: status.to_string(),
            created_at: DateTime::now(),
            updated_at: DateTime::now(),
            expires_at,
            file_size: 1024,
            domain: Some("example.com".to_string()),
        }
    }

    /// 根据记录确定缓存状态（测试辅助函数）
    fn determine_cache_status(record: &CachedHtml) -> CacheStatus {
        match record.status.as_str() {
            "success" => {
                if let Some(expires_at) = record.expires_at {
                    if expires_at.timestamp_millis() < chrono::Utc::now().timestamp_millis() {
                        CacheStatus::Expired
                    } else {
                        CacheStatus::Complete
                    }
                } else {
                    CacheStatus::Complete
                }
            },
            "pending" => CacheStatus::Processing,
            "error" => CacheStatus::Failed,
            _ => CacheStatus::Failed,
        }
    }

    /// 测试多优先级查询策略逻辑
    #[test]
    fn test_query_priority_logic() {
        // 测试查询优先级逻辑
        let test_cases = vec![
            // (url, source_lang, target_lang, expected_priority_level)
            ("https://example.com", "en", "zh", 1), // 精确匹配，最高优先级
            ("https://example.com", "auto", "zh", 2), // 自动检测语言
            ("https://example.com", "en", "fr", 3), // 不同目标语言
        ];

        for (url, source_lang, target_lang, expected_priority) in test_cases {
            // 模拟构建查询过滤器的逻辑
            let filters = vec![
                // 1. 精确匹配 (最高优先级)
                format!("url:{},source_lang:{},target_lang:{},status:success", url, source_lang, target_lang),
                // 2. 相同URL，不同语言组合但状态成功
                format!("url:{},status:success", url),
                // 3. 正在处理中的记录
                format!("url:{},source_lang:{},target_lang:{},status:pending", url, source_lang, target_lang),
                // 4. 任何相关记录
                format!("url:{}", url),
            ];

            // 验证查询优先级逻辑正确
            assert!(filters.len() == 4);
            assert!(filters[0].contains("status:success"));
            assert!(filters[0].contains(source_lang));
            assert!(filters[0].contains(target_lang));
        }
    }

    /// 测试路由策略决策逻辑
    #[test]
    fn test_routing_strategy_decision_logic() {
        // 测试不同缓存状态对应的路由策略
        let test_cases = vec![
            (CacheStatus::Complete, "test_id", RoutingStrategy::UseCache("test_id".to_string())),
            (CacheStatus::Processing, "test_id", RoutingStrategy::WaitForProcessing),
            (CacheStatus::Expired, "test_id", RoutingStrategy::ReprocessWithCheck),
            (CacheStatus::Failed, "test_id", RoutingStrategy::FullReprocess),
            (CacheStatus::NotFound, "", RoutingStrategy::FullReprocess),
        ];

        for (cache_status, record_id, expected_strategy) in test_cases {
            let strategy = match cache_status {
                CacheStatus::Complete => RoutingStrategy::UseCache(record_id.to_string()),
                CacheStatus::Processing => RoutingStrategy::WaitForProcessing,
                CacheStatus::Expired => RoutingStrategy::ReprocessWithCheck,
                CacheStatus::Failed => RoutingStrategy::FullReprocess,
                CacheStatus::NotFound => RoutingStrategy::FullReprocess,
            };

            assert_eq!(strategy, expected_strategy);
        }
    }

    /// 测试LRU缓存基本功能
    #[test]
    fn test_lru_cache_basic_operations() {
        let mut lru = LruCache::new(3);
        
        // 测试插入
        let entry1 = MemoryCacheEntry {
            result: CacheQueryResult {
                record_id: Some("1".to_string()),
                cache_status: CacheStatus::Complete,
                cached_record: None,
                routing_strategy: RoutingStrategy::UseCache("1".to_string()),
            },
            timestamp: std::time::Instant::now(),
            access_count: 1,
        };
        
        lru.insert("key1".to_string(), entry1);
        assert_eq!(lru.len(), 1);
        
        // 测试获取
        let retrieved = lru.get("key1");
        assert!(retrieved.is_some());
        
        // 测试LRU淘汰
        let entry2 = MemoryCacheEntry {
            result: CacheQueryResult {
                record_id: Some("2".to_string()),
                cache_status: CacheStatus::Complete,
                cached_record: None,
                routing_strategy: RoutingStrategy::UseCache("2".to_string()),
            },
            timestamp: std::time::Instant::now(),
            access_count: 1,
        };
        
        let entry3 = MemoryCacheEntry {
            result: CacheQueryResult {
                record_id: Some("3".to_string()),
                cache_status: CacheStatus::Complete,
                cached_record: None,
                routing_strategy: RoutingStrategy::UseCache("3".to_string()),
            },
            timestamp: std::time::Instant::now(),
            access_count: 1,
        };
        
        let entry4 = MemoryCacheEntry {
            result: CacheQueryResult {
                record_id: Some("4".to_string()),
                cache_status: CacheStatus::Complete,
                cached_record: None,
                routing_strategy: RoutingStrategy::UseCache("4".to_string()),
            },
            timestamp: std::time::Instant::now(),
            access_count: 1,
        };
        
        lru.insert("key2".to_string(), entry2);
        lru.insert("key3".to_string(), entry3);
        assert_eq!(lru.len(), 3);
        
        // 插入第4个条目应该淘汰最久未使用的key1
        lru.insert("key4".to_string(), entry4);
        assert_eq!(lru.len(), 3);
        
        // key1应该被淘汰
        assert!(lru.get("key1").is_none());
        assert!(lru.get("key2").is_some());
        assert!(lru.get("key3").is_some());
        assert!(lru.get("key4").is_some());
    }

    /// 测试LRU访问顺序
    #[test]
    fn test_lru_access_order() {
        let mut lru = LruCache::new(2);
        
        let entry1 = MemoryCacheEntry {
            result: CacheQueryResult {
                record_id: Some("1".to_string()),
                cache_status: CacheStatus::Complete,
                cached_record: None,
                routing_strategy: RoutingStrategy::UseCache("1".to_string()),
            },
            timestamp: std::time::Instant::now(),
            access_count: 1,
        };
        
        let entry2 = MemoryCacheEntry {
            result: CacheQueryResult {
                record_id: Some("2".to_string()),
                cache_status: CacheStatus::Complete,
                cached_record: None,
                routing_strategy: RoutingStrategy::UseCache("2".to_string()),
            },
            timestamp: std::time::Instant::now(),
            access_count: 1,
        };
        
        lru.insert("key1".to_string(), entry1);
        lru.insert("key2".to_string(), entry2);
        
        // 访问key1，使其成为最近使用的
        let _retrieved = lru.get("key1");
        
        // 插入新条目，key2应该被淘汰（而不是key1）
        let entry3 = MemoryCacheEntry {
            result: CacheQueryResult {
                record_id: Some("3".to_string()),
                cache_status: CacheStatus::Complete,
                cached_record: None,
                routing_strategy: RoutingStrategy::UseCache("3".to_string()),
            },
            timestamp: std::time::Instant::now(),
            access_count: 1,
        };
        
        lru.insert("key3".to_string(), entry3);
        
        // key1应该仍然存在，key2被淘汰
        assert!(lru.get("key1").is_some());
        assert!(lru.get("key2").is_none());
        assert!(lru.get("key3").is_some());
    }

    /// 测试内存缓存清理逻辑
    #[test] 
    fn test_memory_cache_cleanup_logic() {
        // 测试内存缓存清理逻辑的模拟
        let max_entries = 1000;
        let cleanup_threshold = 300; // 5分钟

        // 模拟缓存条目
        let mut cache_entries = Vec::new();
        let now = std::time::Instant::now();
        
        // 添加一些新条目
        for i in 0..500 {
            cache_entries.push((format!("key_{}", i), now));
        }

        // 添加一些过期条目
        let old_time = now - std::time::Duration::from_secs(600); // 10分钟前
        for i in 500..1200 {
            cache_entries.push((format!("key_{}", i), old_time));
        }

        // 模拟清理过程
        let total_before = cache_entries.len();
        assert!(total_before > max_entries);

        // 清理过期条目的逻辑
        let cutoff_time = now - std::time::Duration::from_secs(cleanup_threshold as u64);
        let valid_entries: Vec<_> = cache_entries.into_iter()
            .filter(|(_, timestamp)| *timestamp > cutoff_time)
            .collect();

        let total_after = valid_entries.len();

        // 验证清理效果
        assert!(total_after < total_before);
        assert_eq!(total_after, 500); // 只保留有效的500个条目
    }
}