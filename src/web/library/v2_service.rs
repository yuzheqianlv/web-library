//! # Library V2 服务层
//!
//! 提供新版本API的核心业务逻辑，支持记录级分页和高级查询功能

use super::v2_types::*;
use super::cache::{StatsCacheWithMetrics, CacheConfig};
use crate::web::types::CachedHtml;

#[cfg(feature = "web")]
use mongodb::{Collection, Database};
#[cfg(feature = "web")]
use bson::{doc, DateTime as BsonDateTime, oid::ObjectId};
#[cfg(feature = "web")]
use futures::stream::TryStreamExt;
#[cfg(feature = "web")]
use std::collections::HashMap;
#[cfg(feature = "web")]
use url::Url;
#[cfg(feature = "web")]
use uuid::Uuid;

/// Library V2 服务 - 基于记录级分页的MongoDB库管理
#[cfg(feature = "web")]
pub struct LibraryServiceV2 {
    /// MongoDB数据库连接
    db: Database,
    /// HTML文档集合
    collection: Collection<CachedHtml>,
    /// 验证规则
    validation_rules: ValidationRules,
    /// 统计缓存
    stats_cache: StatsCacheWithMetrics,
    /// 缓存配置
    cache_config: CacheConfig,
}

#[cfg(feature = "web")]
impl LibraryServiceV2 {
    /// 创建新的Library V2服务实例
    pub fn new(db: Database) -> Self {
        let collection = db.collection::<CachedHtml>("html_cache");
        let cache_config = CacheConfig::default();
        let stats_cache = StatsCacheWithMetrics::new(cache_config.basic_stats_ttl);
        
        Self { 
            db, 
            collection,
            validation_rules: ValidationRules::default(),
            stats_cache,
            cache_config,
        }
    }
    
    /// 从URL提取域名
    fn extract_domain_from_url(&self, url: &str) -> String {
        if let Ok(parsed_url) = Url::parse(url) {
            parsed_url.host_str().unwrap_or("unknown").to_string()
        } else {
            "unknown".to_string()
        }
    }
    
    /// 生成记录ID（基于URL和语言对的UUID）
    fn generate_record_id(&self, url: &str, source_lang: &str, target_lang: &str) -> String {
        let input = format!("{}:{}:{}", url, source_lang, target_lang);
        let uuid = Uuid::new_v5(&Uuid::NAMESPACE_URL, input.as_bytes());
        uuid.to_string()
    }
    
    /// 将CachedHtml转换为LibraryRecord
    pub fn cached_html_to_record(&self, cached: &CachedHtml, object_id: Option<ObjectId>) -> LibraryRecord {
        let id = if let Some(oid) = object_id {
            oid.to_hex()
        } else {
            self.generate_record_id(&cached.url, &cached.source_lang, &cached.target_lang)
        };
        
        let domain = self.extract_domain_from_url(&cached.url);
        let file_size = (cached.original_html.len() + cached.translated_html.len()) as i64;
        
        // 简单的状态推断逻辑
        let status = if cached.translated_html.is_empty() {
            if cached.original_html.is_empty() {
                RecordStatus::Error
            } else {
                RecordStatus::Pending
            }
        } else {
            RecordStatus::Success
        };
        
        LibraryRecord {
            id,
            url: cached.url.clone(),
            title: cached.title.clone(),
            domain,
            source_lang: cached.source_lang.clone(),
            target_lang: cached.target_lang.clone(),
            status,
            file_size,
            created_at: cached.created_at.to_chrono(),
            updated_at: cached.created_at.to_chrono(), // CachedHtml没有updated_at字段
            expires_at: None, // CachedHtml没有过期时间
            is_expired: false, // 默认不过期
            cache_type: CacheType::Permanent, // 默认永久缓存
        }
    }
    
    /// 构建MongoDB查询条件
    fn build_query_filter(&self, query: &LibraryListQuery) -> bson::Document {
        let mut filter = doc! {};
        
        // 域名过滤
        if let Some(ref domain) = query.domain {
            filter.insert("url", doc! { 
                "$regex": format!("https?://{}/", regex::escape(domain)), 
                "$options": "i" 
            });
        }
        
        // 源语言过滤
        if let Some(ref source_lang) = query.source_lang {
            filter.insert("source_lang", source_lang);
        }
        
        // 目标语言过滤
        if let Some(ref target_lang) = query.target_lang {
            filter.insert("target_lang", target_lang);
        }
        
        // 日期范围过滤
        if query.date_from.is_some() || query.date_to.is_some() {
            let mut date_filter = doc! {};
            if let Some(from) = query.date_from {
                date_filter.insert("$gte", BsonDateTime::from_chrono(from));
            }
            if let Some(to) = query.date_to {
                date_filter.insert("$lte", BsonDateTime::from_chrono(to));
            }
            filter.insert("created_at", date_filter);
        }
        
        // 搜索关键词（在URL或标题中）
        if let Some(ref search) = query.search {
            filter.insert("$or", vec![
                doc! { "url": { "$regex": regex::escape(search), "$options": "i" } },
                doc! { "title": { "$regex": regex::escape(search), "$options": "i" } }
            ]);
        }
        
        // 状态过滤（通过HTML内容长度推断）
        if let Some(ref status) = query.status {
            match status {
                RecordStatus::Success => {
                    filter.insert("translated_html", doc! { "$ne": "" });
                }
                RecordStatus::Pending => {
                    filter.insert("$and", vec![
                        doc! { "original_html": { "$ne": "" } },
                        doc! { "translated_html": "" }
                    ]);
                }
                RecordStatus::Error => {
                    filter.insert("original_html", "");
                }
            }
        }
        
        filter
    }
    
    /// 构建排序条件
    fn build_sort_condition(&self, query: &LibraryListQuery) -> bson::Document {
        let sort_field = match query.sort.as_ref().unwrap_or(&SortField::CreatedAt) {
            SortField::CreatedAt => "created_at",
            SortField::UpdatedAt => "created_at", // CachedHtml没有updated_at，使用created_at
            SortField::Title => "title",
            SortField::Domain => "url", // 通过URL排序近似域名排序
            SortField::FileSize => "original_html", // 通过HTML长度近似文件大小
            SortField::Status => "translated_html", // 通过翻译内容状态排序
        };
        
        let sort_order = match query.order.as_ref().unwrap_or(&SortOrder::Desc) {
            SortOrder::Asc => 1,
            SortOrder::Desc => -1,
        };
        
        doc! { sort_field: sort_order }
    }
    
    /// 获取库记录列表（V2版本）
    pub async fn get_library_records(&self, query: &LibraryListQuery) -> Result<LibraryListResponse, String> {
        let start_time = std::time::Instant::now();
        
        // 验证查询参数
        query.validate(&self.validation_rules)?;
        
        // 构建查询条件
        let filter = self.build_query_filter(query);
        let sort = self.build_sort_condition(query);
        
        // 统计总数
        let total = self.collection.count_documents(filter.clone()).await
            .map_err(|e| format!("查询总数失败: {}", e))? as i64;
        
        if total == 0 {
            return Ok(LibraryListResponse {
                records: Vec::new(),
                pagination: Pagination {
                    page: query.page,
                    page_size: query.page_size,
                    total: 0,
                    total_pages: 0,
                    has_next: false,
                    has_prev: false,
                },
                meta: QueryMeta {
                    query_time_ms: start_time.elapsed().as_millis() as i32,
                    filters_applied: self.count_applied_filters(query),
                    search_applied: query.search.is_some(),
                },
            });
        }
        
        // 计算分页参数
        let total_pages = ((total as f64) / (query.page_size as f64)).ceil() as i32;
        let skip = ((query.page - 1) * query.page_size) as u64;
        let has_prev = query.page > 1;
        let has_next = query.page < total_pages;
        
        // 查询记录
        let mut cursor = self.collection.find(filter)
            .sort(sort)
            .skip(skip)
            .limit(query.page_size as i64)
            .await
            .map_err(|e| format!("查询记录失败: {}", e))?;
        
        let mut records = Vec::new();
        while let Some(cached) = cursor.try_next().await
            .map_err(|e| format!("读取记录失败: {}", e))? {
            records.push(self.cached_html_to_record(&cached, None));
        }
        
        Ok(LibraryListResponse {
            records,
            pagination: Pagination {
                page: query.page,
                page_size: query.page_size,
                total,
                total_pages,
                has_next,
                has_prev,
            },
            meta: QueryMeta {
                query_time_ms: start_time.elapsed().as_millis() as i32,
                filters_applied: self.count_applied_filters(query),
                search_applied: query.search.is_some(),
            },
        })
    }
    
    /// 统计已应用的筛选条件数量
    fn count_applied_filters(&self, query: &LibraryListQuery) -> i32 {
        let mut count = 0;
        if query.domain.is_some() { count += 1; }
        if query.source_lang.is_some() { count += 1; }
        if query.target_lang.is_some() { count += 1; }
        if query.status.is_some() { count += 1; }
        if query.date_from.is_some() { count += 1; }
        if query.date_to.is_some() { count += 1; }
        if query.search.is_some() { count += 1; }
        if !query.include_expired { count += 1; }
        count
    }
    
    /// 获取库统计信息（V2版本）
    pub async fn get_library_stats(&self, query: &StatsQuery) -> Result<LibraryStatsResponse, String> {
        let _start_time = std::time::Instant::now();
        
        // 检查缓存
        if let Some(cached_stats) = self.stats_cache.get(query.detailed, query.days).await {
            return Ok(cached_stats);
        }
        
        // 如果缓存未命中，计算统计信息
        let stats = self.compute_library_stats(query).await?;
        
        // 根据查询类型选择合适的TTL
        let ttl = self.get_stats_ttl(query);
        self.stats_cache.set_with_ttl(query.detailed, query.days, stats.clone(), ttl).await;
        
        Ok(stats)
    }
    
    /// 计算库统计信息（内部方法）
    async fn compute_library_stats(&self, query: &StatsQuery) -> Result<LibraryStatsResponse, String> {
        // 构建时间过滤条件
        let mut base_filter = doc! {};
        if let Some(days) = query.days {
            let since = chrono::Utc::now() - chrono::Duration::days(days as i64);
            base_filter.insert("created_at", doc! { "$gte": BsonDateTime::from_chrono(since) });
        }
        
        // 获取总数
        let total = self.collection.count_documents(base_filter.clone()).await
            .map_err(|e| format!("查询总数失败: {}", e))? as i64;
        
        // 统计各状态数量
        let mut success_filter = base_filter.clone();
        success_filter.insert("translated_html", doc! { "$ne": "" });
        let success = self.collection.count_documents(success_filter).await
            .map_err(|e| format!("查询成功数失败: {}", e))? as i64;
        
        let mut pending_filter = base_filter.clone();
        pending_filter.insert("$and", vec![
            doc! { "original_html": { "$ne": "" } },
            doc! { "translated_html": "" }
        ]);
        let pending = self.collection.count_documents(pending_filter).await
            .map_err(|e| format!("查询处理中数失败: {}", e))? as i64;
        
        let mut error_filter = base_filter.clone();
        error_filter.insert("original_html", "");
        let error = self.collection.count_documents(error_filter).await
            .map_err(|e| format!("查询错误数失败: {}", e))? as i64;
        
        // 今日统计
        let today_start = chrono::Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        let today_filter = doc! { "created_at": { "$gte": BsonDateTime::from_chrono(today_start) } };
        let today = self.collection.count_documents(today_filter).await
            .map_err(|e| format!("查询今日数失败: {}", e))? as i64;
        
        // 计算存储大小和语言分布
        let mut cursor = self.collection.find(base_filter.clone()).await
            .map_err(|e| format!("查询记录失败: {}", e))?;
        
        let mut storage_size = 0i64;
        let mut source_languages: HashMap<String, i64> = HashMap::new();
        let mut target_languages: HashMap<String, i64> = HashMap::new();
        
        while let Some(cached) = cursor.try_next().await
            .map_err(|e| format!("读取记录失败: {}", e))? {
            storage_size += (cached.original_html.len() + cached.translated_html.len()) as i64;
            *source_languages.entry(cached.source_lang).or_insert(0) += 1;
            *target_languages.entry(cached.target_lang).or_insert(0) += 1;
        }
        
        // 计算状态分布比率
        let status_distribution = StatusDistribution {
            success_rate: if total > 0 { success as f64 / total as f64 } else { 0.0 },
            error_rate: if total > 0 { error as f64 / total as f64 } else { 0.0 },
            pending_rate: if total > 0 { pending as f64 / total as f64 } else { 0.0 },
        };
        
        let mut response = LibraryStatsResponse {
            total,
            success,
            pending,
            error,
            today,
            storage_size,
            languages: LanguageStats {
                source_languages,
                target_languages,
            },
            status_distribution,
            domains: None,
            trend: None,
        };
        
        // 如果请求详细统计，添加域名和趋势信息
        if query.detailed {
            response.domains = Some(self.get_domain_stats(base_filter.clone()).await?);
            if query.days.is_some() {
                response.trend = Some(self.get_daily_trend(query.days.unwrap()).await?);
            }
        }
        
        Ok(response)
    }
    
    /// 获取域名统计信息
    async fn get_domain_stats(&self, base_filter: bson::Document) -> Result<Vec<DomainStat>, String> {
        let mut cursor = self.collection.find(base_filter).await
            .map_err(|e| format!("查询域名统计失败: {}", e))?;
        
        let mut domain_stats: HashMap<String, (i64, i64, i64)> = HashMap::new(); // (count, total_size, success_count)
        
        while let Some(cached) = cursor.try_next().await
            .map_err(|e| format!("读取域名统计失败: {}", e))? {
            let domain = self.extract_domain_from_url(&cached.url);
            let size = (cached.original_html.len() + cached.translated_html.len()) as i64;
            let is_success = !cached.translated_html.is_empty();
            
            let entry = domain_stats.entry(domain).or_insert((0, 0, 0));
            entry.0 += 1; // count
            entry.1 += size; // total_size
            if is_success { entry.2 += 1; } // success_count
        }
        
        let mut domains: Vec<DomainStat> = domain_stats.into_iter().map(|(domain, (count, total_size, success_count))| {
            DomainStat {
                domain,
                count,
                total_size,
                avg_size: if count > 0 { total_size / count } else { 0 },
                success_rate: if count > 0 { success_count as f64 / count as f64 } else { 0.0 },
            }
        }).collect();
        
        // 按记录数量降序排序，取前10
        domains.sort_by(|a, b| b.count.cmp(&a.count));
        domains.truncate(10);
        
        Ok(domains)
    }
    
    /// 获取日统计趋势
    async fn get_daily_trend(&self, days: i32) -> Result<Vec<DailyStat>, String> {
        let mut trend = Vec::new();
        let today = chrono::Utc::now().date_naive();
        
        for i in (0..days).rev() {
            let date = today - chrono::Duration::days(i as i64);
            let start = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
            let end = date.and_hms_opt(23, 59, 59).unwrap().and_utc();
            
            let filter = doc! {
                "created_at": {
                    "$gte": BsonDateTime::from_chrono(start),
                    "$lte": BsonDateTime::from_chrono(end)
                }
            };
            
            let count = self.collection.count_documents(filter.clone()).await
                .map_err(|e| format!("查询日统计失败: {}", e))? as i64;
            
            // 计算当天的总大小
            let mut cursor = self.collection.find(filter).await
                .map_err(|e| format!("查询日统计大小失败: {}", e))?;
            
            let mut size = 0i64;
            while let Some(cached) = cursor.try_next().await
                .map_err(|e| format!("读取日统计失败: {}", e))? {
                size += (cached.original_html.len() + cached.translated_html.len()) as i64;
            }
            
            trend.push(DailyStat { date, count, size });
        }
        
        Ok(trend)
    }
    
    /// 根据查询类型获取合适的TTL
    fn get_stats_ttl(&self, query: &StatsQuery) -> std::time::Duration {
        match (query.detailed, query.days) {
            // 今日统计：短TTL，因为数据变化频繁
            (_, Some(1)) => self.cache_config.today_stats_ttl,
            // 详细统计：中等TTL
            (true, _) => self.cache_config.detailed_stats_ttl,
            // 历史统计：长TTL
            (false, Some(_)) => self.cache_config.historical_stats_ttl,
            // 基础统计：默认TTL
            (false, None) => self.cache_config.basic_stats_ttl,
        }
    }
    
    /// 获取缓存统计信息
    pub async fn get_cache_stats(&self) -> Result<super::cache::CacheInfo, String> {
        Ok(self.stats_cache.get_cache_info().await)
    }
    
    /// 清理统计缓存
    pub async fn clear_stats_cache(&self) -> Result<(), String> {
        self.stats_cache.clear().await;
        Ok(())
    }
    
    /// 使统计缓存失效（当数据更新时调用）
    pub async fn invalidate_stats_cache(&self) -> Result<(), String> {
        self.stats_cache.invalidate_stats().await;
        Ok(())
    }
    
    /// 优化的今日统计查询
    pub async fn get_today_stats(&self) -> Result<TodayStats, String> {
        let today_start = chrono::Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc();
        
        // 今日新增记录
        let today_filter = doc! { "created_at": { "$gte": BsonDateTime::from_chrono(today_start) } };
        let today_total = self.collection.count_documents(today_filter.clone()).await
            .map_err(|e| format!("查询今日总数失败: {}", e))? as i64;
        
        // 今日成功翻译
        let mut today_success_filter = today_filter.clone();
        today_success_filter.insert("translated_html", doc! { "$ne": "" });
        let today_success = self.collection.count_documents(today_success_filter).await
            .map_err(|e| format!("查询今日成功数失败: {}", e))? as i64;
        
        // 今日处理中
        let mut today_pending_filter = today_filter.clone();
        today_pending_filter.insert("$and", vec![
            doc! { "original_html": { "$ne": "" } },
            doc! { "translated_html": "" }
        ]);
        let today_pending = self.collection.count_documents(today_pending_filter).await
            .map_err(|e| format!("查询今日处理中数失败: {}", e))? as i64;
        
        // 今日错误
        let mut today_error_filter = today_filter.clone();
        today_error_filter.insert("original_html", "");
        let today_error = self.collection.count_documents(today_error_filter).await
            .map_err(|e| format!("查询今日错误数失败: {}", e))? as i64;
        
        // 计算今日存储大小
        let mut cursor = self.collection.find(today_filter).await
            .map_err(|e| format!("查询今日记录失败: {}", e))?;
        
        let mut today_storage_size = 0i64;
        let mut today_languages: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        
        while let Some(cached) = cursor.try_next().await
            .map_err(|e| format!("读取今日记录失败: {}", e))? {
            today_storage_size += (cached.original_html.len() + cached.translated_html.len()) as i64;
            *today_languages.entry(cached.target_lang).or_insert(0) += 1;
        }
        
        Ok(TodayStats {
            total: today_total,
            success: today_success,
            pending: today_pending,
            error: today_error,
            storage_size: today_storage_size,
            languages: today_languages,
            success_rate: if today_total > 0 { today_success as f64 / today_total as f64 } else { 0.0 },
        })
    }
    
    /// 获取语言分布统计（优化版本）
    pub async fn get_language_distribution(&self) -> Result<LanguageStats, String> {
        // 使用MongoDB聚合管道进行高效统计
        let pipeline = vec![
            doc! {
                "$group": {
                    "_id": null,
                    "source_languages": {
                        "$push": "$source_lang"
                    },
                    "target_languages": {
                        "$push": "$target_lang"
                    }
                }
            }
        ];
        
        let mut cursor = self.collection.aggregate(pipeline).await
            .map_err(|e| format!("聚合查询失败: {}", e))?;
        
        let mut source_languages = std::collections::HashMap::new();
        let mut target_languages = std::collections::HashMap::new();
        
        if let Some(result) = cursor.try_next().await
            .map_err(|e| format!("读取聚合结果失败: {}", e))? {
            
            if let Ok(source_langs) = result.get_array("source_languages") {
                for lang in source_langs {
                    if let Some(lang_str) = lang.as_str() {
                        *source_languages.entry(lang_str.to_string()).or_insert(0) += 1;
                    }
                }
            }
            
            if let Ok(target_langs) = result.get_array("target_languages") {
                for lang in target_langs {
                    if let Some(lang_str) = lang.as_str() {
                        *target_languages.entry(lang_str.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }
        
        Ok(LanguageStats {
            source_languages,
            target_languages,
        })
    }
    
    /// 创建数据库索引以支持V2查询
    pub async fn create_v2_indexes(&self) -> Result<(), String> {
        use mongodb::{IndexModel, options::IndexOptions};
        
        let indexes = vec![
            // URL + 语言组合的唯一索引
            IndexModel::builder()
                .keys(doc! { "url": 1, "source_lang": 1, "target_lang": 1 })
                .options(IndexOptions::builder().unique(true).name("url_lang_unique_v2".to_string()).build())
                .build(),
            // 创建时间索引（用于排序和日期筛选）
            IndexModel::builder()
                .keys(doc! { "created_at": -1 })
                .options(IndexOptions::builder().name("created_at_desc_v2".to_string()).build())
                .build(),
            // 标题索引（用于搜索）
            IndexModel::builder()
                .keys(doc! { "title": "text", "url": "text" })
                .options(IndexOptions::builder().name("title_url_text_v2".to_string()).build())
                .build(),
            // 语言索引
            IndexModel::builder()
                .keys(doc! { "source_lang": 1, "target_lang": 1 })
                .options(IndexOptions::builder().name("languages_v2".to_string()).build())
                .build(),
            // HTML长度索引（用于状态和大小筛选）
            IndexModel::builder()
                .keys(doc! { "original_html": 1, "translated_html": 1 })
                .options(IndexOptions::builder().name("html_length_v2".to_string()).build())
                .build(),
        ];
        
        self.collection.create_indexes(indexes).await
            .map_err(|e| format!("创建V2索引失败: {}", e))?;
        
        Ok(())
    }
}

/// 非 web feature 的占位结构
#[cfg(not(feature = "web"))]
pub struct LibraryServiceV2;

#[cfg(not(feature = "web"))]
impl LibraryServiceV2 {
    pub fn new() -> Self {
        Self
    }
}