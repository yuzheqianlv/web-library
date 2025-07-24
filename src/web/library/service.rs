//! # Library服务层
//!
//! 提供库管理的核心业务逻辑，基于MongoDB数据存储

use super::types::*;

#[cfg(feature = "web")]
use bson::{doc, DateTime};
#[cfg(feature = "web")]
use mongodb::{Collection, Database};
#[cfg(feature = "web")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "web")]
use std::collections::HashMap;
#[cfg(feature = "web")]
use url::Url;

/// MongoDB中存储的HTML文件元数据
#[cfg(feature = "web")]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HtmlDocument {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<bson::oid::ObjectId>,
    /// 原始URL
    pub url: String,
    /// 页面标题
    pub title: Option<String>,
    /// 域名
    pub domain: String,
    /// 源语言
    pub source_lang: String,
    /// 目标语言
    pub target_lang: String,
    /// HTML内容（可选，用于小文件）
    pub html_content: Option<String>,
    /// 文件路径（用于大文件）
    pub file_path: Option<String>,
    /// 文件大小（字节）
    pub size_bytes: i64,
    /// 创建时间
    pub created_at: DateTime,
    /// 更新时间
    pub updated_at: DateTime,
    /// 是否已过期
    pub is_expired: bool,
    /// 过期时间
    pub expires_at: Option<DateTime>,
    /// 缓存类型
    pub cache_type: String, // "permanent", "temporary", "manual"
}

/// Library服务 - 基于MongoDB的HTML库管理
#[cfg(feature = "web")]
pub struct LibraryService {
    /// MongoDB数据库连接
    db: Database,
    /// HTML文档集合 (使用CachedHtml作为兼容现有数据)
    collection: Collection<crate::web::types::CachedHtml>,
}

#[cfg(feature = "web")]
impl LibraryService {
    /// 创建新的Library服务实例
    pub fn new(db: Database) -> Self {
        let collection = db.collection::<crate::web::types::CachedHtml>("html_cache");
        Self { db, collection }
    }

    /// 从URL提取域名
    fn extract_domain_from_url(&self, url: &str) -> String {
        if let Ok(parsed_url) = Url::parse(url) {
            parsed_url.host_str().unwrap_or("unknown").to_string()
        } else {
            "unknown".to_string()
        }
    }

    /// 从数据库获取域名统计信息
    pub async fn get_domain_stats(&self) -> Result<HashMap<String, (usize, usize)>, String> {
        use futures::stream::TryStreamExt;

        // 由于CachedHtml没有domain字段，我们需要从所有文档中计算域名统计
        let mut cursor = self
            .collection
            .find(doc! {})
            .await
            .map_err(|e| format!("查询数据库失败: {}", e))?;

        let mut stats: HashMap<String, (usize, usize)> = HashMap::new();

        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| format!("读取结果失败: {}", e))?
        {
            let domain = self.extract_domain_from_url(&doc.url);

            // 估算文档大小（HTML内容长度）
            let size = doc.original_html.len() + doc.translated_html.len();

            let entry = stats.entry(domain).or_insert((0, 0));
            entry.0 += 1; // 增加计数
            entry.1 += size; // 增加大小
        }

        Ok(stats)
    }

    /// 添加HTML文档到数据库
    pub async fn add_html_document(
        &self,
        doc: crate::web::types::CachedHtml,
    ) -> Result<bson::oid::ObjectId, String> {
        let result = self
            .collection
            .insert_one(doc)
            .await
            .map_err(|e| format!("插入文档失败: {}", e))?;

        Ok(result.inserted_id.as_object_id().unwrap())
    }

    /// 更新HTML文档
    pub async fn update_html_document(
        &self,
        id: bson::oid::ObjectId,
        doc: crate::web::types::CachedHtml,
    ) -> Result<(), String> {
        let filter = doc! { "_id": id };
        let update = doc! {
            "$set": bson::to_bson(&doc).map_err(|e| format!("序列化失败: {}", e))?
        };

        self.collection
            .update_one(filter, update)
            .await
            .map_err(|e| format!("更新文档失败: {}", e))?;

        Ok(())
    }

    /// 获取库数据
    pub async fn get_library_data(
        &self,
        request: &LibraryRequest,
    ) -> Result<LibraryResponse, String> {
        use futures::stream::TryStreamExt;

        let page = request.page.unwrap_or(1);
        let limit = request.limit.unwrap_or(20);

        // 构建查询条件
        let mut filter = doc! {};
        if let Some(ref domain_filter) = request.domain {
            filter.insert("domain", doc! { "$regex": domain_filter, "$options": "i" });
        }

        // 获取域名统计信息
        let domain_stats = self.get_domain_stats().await?;

        // 构建域名列表
        let mut domains: Vec<LibraryDomain> = Vec::new();

        for (domain_name, (count, total_size)) in domain_stats {
            // 如果有域名过滤，检查是否匹配
            if let Some(ref domain_filter) = request.domain {
                if !domain_name
                    .to_lowercase()
                    .contains(&domain_filter.to_lowercase())
                {
                    continue;
                }
            }

            // 获取该域名下的文档
            let mut cursor = self
                .collection
                .find(doc! {})
                .await
                .map_err(|e| format!("查询数据库失败: {}", e))?;

            let mut urls = Vec::new();
            let mut last_updated = 0u64;

            while let Some(doc) = cursor
                .try_next()
                .await
                .map_err(|e| format!("读取结果失败: {}", e))?
            {
                // 检查域名是否匹配
                let doc_domain = self.extract_domain_from_url(&doc.url);
                if doc_domain != domain_name {
                    continue;
                }

                let created_timestamp = doc.created_at.timestamp_millis() as u64 / 1000;
                last_updated = last_updated.max(created_timestamp);

                urls.push(LibraryUrl {
                    url: doc.url,
                    title: doc.title,
                    source_lang: doc.source_lang,
                    target_lang: doc.target_lang,
                    created_at: created_timestamp,
                    expires_at: u64::MAX, // CachedHtml没有过期时间，设为永不过期
                    size_bytes: doc.original_html.len() + doc.translated_html.len(),
                    is_expired: false, // 假设都没过期
                });
            }

            domains.push(LibraryDomain {
                domain: domain_name,
                cache_count: count,
                total_size_bytes: total_size,
                last_updated,
                urls,
            });
        }

        // 排序
        match request.sort_by.as_deref() {
            Some("cache_count") => {
                if request.sort_order.as_deref() == Some("desc") {
                    domains.sort_by(|a, b| b.cache_count.cmp(&a.cache_count));
                } else {
                    domains.sort_by(|a, b| a.cache_count.cmp(&b.cache_count));
                }
            }
            Some("last_updated") => {
                if request.sort_order.as_deref() == Some("desc") {
                    domains.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));
                } else {
                    domains.sort_by(|a, b| a.last_updated.cmp(&b.last_updated));
                }
            }
            Some("domain_name") => {
                if request.sort_order.as_deref() == Some("desc") {
                    domains.sort_by(|a, b| b.domain.cmp(&a.domain));
                } else {
                    domains.sort_by(|a, b| a.domain.cmp(&b.domain));
                }
            }
            _ => {
                // 默认按最后更新时间降序排序
                domains.sort_by(|a, b| b.last_updated.cmp(&a.last_updated));
            }
        }

        let total_domains = domains.len();
        let total_pages = (total_domains + limit - 1) / limit;

        // 分页
        let start = (page - 1) * limit;
        let domains = domains.into_iter().skip(start).take(limit).collect();

        Ok(LibraryResponse {
            domains,
            total_domains,
            page,
            limit,
            total_pages,
        })
    }

    /// 获取域名详情
    pub async fn get_domain_details(
        &self,
        request: &DomainDetailsRequest,
    ) -> Result<DomainDetailsResponse, String> {
        use futures::stream::TryStreamExt;

        let page = request.page.unwrap_or(1);
        let limit = request.limit.unwrap_or(20);
        let include_expired = request.include_expired.unwrap_or(true);

        // 构建查询条件
        let mut filter = doc! { "domain": &request.domain };
        if !include_expired {
            filter.insert("is_expired", false);
        }

        // 获取总数
        let total_count = self
            .collection
            .count_documents(filter.clone())
            .await
            .map_err(|e| format!("查询数据库失败: {}", e))? as usize;

        if total_count == 0 {
            return Err(format!("域名 {} 不存在或没有数据", request.domain));
        }

        // 分页查询
        let mut cursor = self
            .collection
            .find(filter)
            .sort(doc! { "created_at": -1 })
            .skip(((page - 1) * limit) as u64)
            .limit(limit as i64)
            .await
            .map_err(|e| format!("查询数据库失败: {}", e))?;

        let mut urls = Vec::new();
        let mut domain_info = LibraryDomain {
            domain: request.domain.clone(),
            cache_count: 0,
            total_size_bytes: 0,
            last_updated: 0,
            urls: Vec::new(),
        };

        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| format!("读取结果失败: {}", e))?
        {
            let created_timestamp = doc.created_at.timestamp_millis() as u64 / 1000;
            domain_info.last_updated = domain_info.last_updated.max(created_timestamp);
            domain_info.total_size_bytes += doc.original_html.len() + doc.translated_html.len();

            urls.push(LibraryUrl {
                url: doc.url,
                title: doc.title,
                source_lang: doc.source_lang,
                target_lang: doc.target_lang,
                created_at: doc.created_at.timestamp_millis() as u64 / 1000,
                expires_at: u64::MAX, // CachedHtml没有过期时间，设为永不过期
                size_bytes: doc.original_html.len() + doc.translated_html.len(),
                is_expired: false, // CachedHtml没有is_expired字段，默认为未过期
            });
        }

        domain_info.cache_count = total_count;
        let total_pages = (total_count + limit - 1) / limit;

        Ok(DomainDetailsResponse {
            domain: domain_info,
            urls,
            page,
            limit,
            total_pages,
        })
    }

    /// 获取库统计信息
    pub async fn get_library_stats(&self) -> Result<LibraryStats, String> {
        use futures::TryStreamExt;

        // 获取域名统计
        let domain_stats = self.get_domain_stats().await?;
        let total_domains = domain_stats.len();
        let mut total_urls = 0;
        let mut total_size_bytes = 0;

        for (_, (count, size)) in &domain_stats {
            total_urls += count;
            total_size_bytes += size;
        }

        // 获取过期数量
        let expired_filter = doc! { "is_expired": true };
        let expired_urls = self
            .collection
            .count_documents(expired_filter)
            .await
            .map_err(|e| format!("查询过期数据失败: {}", e))? as usize;

        // 获取Top 10域名
        let mut top_domains = Vec::new();
        for (domain_name, (count, size)) in domain_stats {
            top_domains.push(LibraryDomain {
                domain: domain_name,
                cache_count: count,
                total_size_bytes: size,
                last_updated: 0, // 详细时间需要额外查询
                urls: Vec::new(),
            });
        }
        top_domains.sort_by(|a, b| b.cache_count.cmp(&a.cache_count));
        top_domains.truncate(10);

        // 获取最大的条目
        let mut cursor = self
            .collection
            .find(doc! {})
            .sort(doc! { "size_bytes": -1 })
            .limit(10)
            .await
            .map_err(|e| format!("查询最大条目失败: {}", e))?;

        let mut largest_entries = Vec::new();
        while let Some(doc) = cursor
            .try_next()
            .await
            .map_err(|e| format!("读取结果失败: {}", e))?
        {
            largest_entries.push(LibraryUrl {
                url: doc.url,
                title: doc.title,
                source_lang: doc.source_lang,
                target_lang: doc.target_lang,
                created_at: doc.created_at.timestamp_millis() as u64 / 1000,
                expires_at: u64::MAX, // CachedHtml没有过期时间，设为永不过期
                size_bytes: doc.original_html.len() + doc.translated_html.len(),
                is_expired: false, // CachedHtml没有is_expired字段，默认为未过期
            });
        }

        Ok(LibraryStats {
            total_domains,
            total_urls,
            total_size_bytes,
            expired_urls,
            top_domains,
            largest_entries,
        })
    }

    /// 删除域名
    pub async fn delete_domain(&self, domain: &str) -> Result<usize, String> {
        let filter = doc! { "domain": domain };

        // 先查询有多少文档
        let count = self
            .collection
            .count_documents(filter.clone())
            .await
            .map_err(|e| format!("查询数据库失败: {}", e))? as usize;

        if count == 0 {
            return Err(format!("域名 {} 不存在", domain));
        }

        // 删除所有该域名下的文档
        let result = self
            .collection
            .delete_many(filter)
            .await
            .map_err(|e| format!("删除数据失败: {}", e))?;

        Ok(result.deleted_count as usize)
    }

    /// 删除特定URL
    pub async fn delete_url(
        &self,
        url: &str,
        source_lang: &str,
        target_lang: &str,
    ) -> Result<bool, String> {
        let filter = doc! {
            "url": url,
            "source_lang": source_lang,
            "target_lang": target_lang
        };

        let result = self
            .collection
            .delete_one(filter)
            .await
            .map_err(|e| format!("删除数据失败: {}", e))?;

        Ok(result.deleted_count > 0)
    }

    /// 刷新统计信息（重新计算索引）
    pub async fn refresh(&self) -> Result<(), String> {
        // 对于 MongoDB，可以重新创建索引或执行数据清理
        // 这里可以添加索引优化逻辑
        Ok(())
    }

    /// 批量删除URL列表
    pub async fn batch_delete_urls(&self, urls: &[String]) -> Result<usize, String> {
        let filter = doc! {
            "url": { "$in": urls }
        };

        let result = self
            .collection
            .delete_many(filter)
            .await
            .map_err(|e| format!("批量删除失败: {}", e))?;

        Ok(result.deleted_count as usize)
    }

    /// 创建数据库索引
    pub async fn create_indexes(&self) -> Result<(), String> {
        use mongodb::{options::IndexOptions, IndexModel};

        let indexes = vec![
            IndexModel::builder()
                .keys(doc! { "domain": 1 })
                .options(IndexOptions::builder().name("domain_1".to_string()).build())
                .build(),
            IndexModel::builder()
                .keys(doc! { "url": 1, "source_lang": 1, "target_lang": 1 })
                .options(
                    IndexOptions::builder()
                        .unique(true)
                        .name("url_lang_unique".to_string())
                        .build(),
                )
                .build(),
            IndexModel::builder()
                .keys(doc! { "created_at": -1 })
                .options(
                    IndexOptions::builder()
                        .name("created_at_desc".to_string())
                        .build(),
                )
                .build(),
            IndexModel::builder()
                .keys(doc! { "updated_at": -1 })
                .options(
                    IndexOptions::builder()
                        .name("updated_at_desc".to_string())
                        .build(),
                )
                .build(),
            IndexModel::builder()
                .keys(doc! { "size_bytes": -1 })
                .options(
                    IndexOptions::builder()
                        .name("size_bytes_desc".to_string())
                        .build(),
                )
                .build(),
        ];

        self.collection
            .create_indexes(indexes)
            .await
            .map_err(|e| format!("创建索引失败: {}", e))?;

        Ok(())
    }
}

/// 非 web feature 的占位结构
#[cfg(not(feature = "web"))]
pub struct LibraryService;

#[cfg(not(feature = "web"))]
impl LibraryService {
    pub fn new() -> Self {
        Self
    }
}
