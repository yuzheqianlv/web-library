//! # Library V2 API 类型定义
//!
//! 定义了新版本API的数据结构和类型，支持记录级分页和高级查询功能

#[cfg(feature = "web")]
use serde::{Deserialize, Serialize};

/// 库记录 - V2版本
#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryRecord {
    /// 唯一标识符
    pub id: String,
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
    /// 处理状态
    pub status: RecordStatus,
    /// 文件大小（字节）
    pub file_size: i64,
    /// 创建时间
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 更新时间
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// 过期时间
    #[serde(with = "chrono::serde::ts_seconds_option")]
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 是否已过期
    pub is_expired: bool,
    /// 缓存类型
    pub cache_type: CacheType,
}

/// 记录状态枚举
#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RecordStatus {
    Success, // 翻译成功
    Pending, // 处理中
    Error,   // 处理失败
}

/// 缓存类型枚举
#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CacheType {
    Permanent, // 永久缓存
    Temporary, // 临时缓存
    Manual,    // 手动缓存
}

/// 分页信息
#[cfg(feature = "web")]
#[derive(Debug, Serialize)]
pub struct Pagination {
    /// 当前页码
    pub page: i32,
    /// 每页大小
    pub page_size: i32,
    /// 总记录数
    pub total: i64,
    /// 总页数
    pub total_pages: i32,
    /// 是否有下一页
    pub has_next: bool,
    /// 是否有上一页
    pub has_prev: bool,
}

/// 查询元数据
#[cfg(feature = "web")]
#[derive(Debug, Serialize)]
pub struct QueryMeta {
    /// 查询耗时（毫秒）
    pub query_time_ms: i32,
    /// 总筛选条件数
    pub filters_applied: i32,
    /// 是否使用了搜索
    pub search_applied: bool,
}

/// 列表查询请求参数
#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct LibraryListQuery {
    /// 页码（从1开始）
    #[serde(default = "default_page")]
    pub page: i32,
    /// 每页大小（1-100）
    #[serde(default = "default_page_size")]
    pub page_size: i32,
    /// 排序字段
    #[serde(default)]
    pub sort: Option<SortField>,
    /// 排序顺序
    #[serde(default)]
    pub order: Option<SortOrder>,
    /// 搜索关键词
    pub search: Option<String>,
    /// 状态过滤
    pub status: Option<RecordStatus>,
    /// 源语言过滤
    pub source_lang: Option<String>,
    /// 目标语言过滤
    pub target_lang: Option<String>,
    /// 域名过滤
    pub domain: Option<String>,
    /// 开始日期 (ISO 8601)
    pub date_from: Option<chrono::DateTime<chrono::Utc>>,
    /// 结束日期 (ISO 8601)
    pub date_to: Option<chrono::DateTime<chrono::Utc>>,
    /// 是否包含过期记录
    #[serde(default)]
    pub include_expired: bool,
}

/// 排序字段枚举
#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortField {
    CreatedAt,
    UpdatedAt,
    Title,
    Domain,
    FileSize,
    Status,
}

/// 排序顺序枚举
#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// 列表查询响应
#[cfg(feature = "web")]
#[derive(Debug, Serialize)]
pub struct LibraryListResponse {
    /// 记录列表
    pub records: Vec<LibraryRecord>,
    /// 分页信息
    pub pagination: Pagination,
    /// 查询统计
    pub meta: QueryMeta,
}

/// 统计查询请求参数
#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    /// 是否包含详细统计
    #[serde(default)]
    pub detailed: bool,
    /// 时间范围（天）
    pub days: Option<i32>,
}

/// 统计响应
#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize)]
pub struct LibraryStatsResponse {
    /// 总记录数
    pub total: i64,
    /// 成功记录数
    pub success: i64,
    /// 处理中记录数
    pub pending: i64,
    /// 错误记录数
    pub error: i64,
    /// 今日新增数
    pub today: i64,
    /// 总存储大小（字节）
    pub storage_size: i64,
    /// 语言分布
    pub languages: LanguageStats,
    /// 状态分布
    pub status_distribution: StatusDistribution,
    /// 域名统计（可选）
    pub domains: Option<Vec<DomainStat>>,
    /// 时间趋势（可选）
    pub trend: Option<Vec<DailyStat>>,
}

/// 语言统计
#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize)]
pub struct LanguageStats {
    pub source_languages: std::collections::HashMap<String, i64>,
    pub target_languages: std::collections::HashMap<String, i64>,
}

/// 状态分布
#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize)]
pub struct StatusDistribution {
    pub success_rate: f64,
    pub error_rate: f64,
    pub pending_rate: f64,
}

/// 域名统计
#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize)]
pub struct DomainStat {
    pub domain: String,
    pub count: i64,
    pub total_size: i64,
    pub avg_size: i64,
    pub success_rate: f64,
}

/// 日统计
#[cfg(feature = "web")]
#[derive(Debug, Clone, Serialize)]
pub struct DailyStat {
    pub date: chrono::NaiveDate,
    pub count: i64,
    pub size: i64,
}

/// 今日统计
#[cfg(feature = "web")]
#[derive(Debug, Serialize)]
pub struct TodayStats {
    /// 今日总数
    pub total: i64,
    /// 今日成功数
    pub success: i64,
    /// 今日处理中数
    pub pending: i64,
    /// 今日错误数
    pub error: i64,
    /// 今日存储大小
    pub storage_size: i64,
    /// 今日语言分布
    pub languages: std::collections::HashMap<String, i64>,
    /// 今日成功率
    pub success_rate: f64,
}

/// API错误响应
#[cfg(feature = "web")]
#[derive(Debug, Serialize)]
pub struct ApiError {
    /// 错误代码
    pub error_code: String,
    /// 错误消息
    pub message: String,
    /// 详细信息
    pub details: Option<serde_json::Value>,
    /// 请求ID（用于追踪）
    pub request_id: String,
    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// 默认值函数
#[cfg(feature = "web")]
fn default_page() -> i32 {
    1
}

#[cfg(feature = "web")]
fn default_page_size() -> i32 {
    20
}

// 参数验证规则
#[cfg(feature = "web")]
pub struct ValidationRules {
    pub max_page_size: i32,     // 最大100
    pub min_page_size: i32,     // 最小1
    pub max_page: i32,          // 最大10000
    pub max_batch_size: i32,    // 批量操作最大1000个
    pub min_search_length: i32, // 搜索关键词最短2字符
    pub max_search_length: i32, // 搜索关键词最长100字符
}

#[cfg(feature = "web")]
impl Default for ValidationRules {
    fn default() -> Self {
        Self {
            max_page_size: 100,
            min_page_size: 1,
            max_page: 10000,
            max_batch_size: 1000,
            min_search_length: 2,
            max_search_length: 100,
        }
    }
}

#[cfg(feature = "web")]
impl LibraryListQuery {
    /// 验证查询参数
    pub fn validate(&self, rules: &ValidationRules) -> Result<(), String> {
        // 验证页码
        if self.page < 1 {
            return Err("页码必须大于0".to_string());
        }
        if self.page > rules.max_page {
            return Err(format!("页码不能超过{}", rules.max_page));
        }

        // 验证每页大小
        if self.page_size < rules.min_page_size {
            return Err(format!("每页大小不能小于{}", rules.min_page_size));
        }
        if self.page_size > rules.max_page_size {
            return Err(format!("每页大小不能超过{}", rules.max_page_size));
        }

        // 验证搜索关键词
        if let Some(ref search) = self.search {
            if search.len() < rules.min_search_length as usize {
                return Err(format!("搜索关键词长度不能小于{}", rules.min_search_length));
            }
            if search.len() > rules.max_search_length as usize {
                return Err(format!("搜索关键词长度不能超过{}", rules.max_search_length));
            }
        }

        // 验证日期范围
        if let (Some(from), Some(to)) = (&self.date_from, &self.date_to) {
            if from > to {
                return Err("开始日期不能晚于结束日期".to_string());
            }
        }

        Ok(())
    }
}

// 非 web feature 的占位类型
#[cfg(not(feature = "web"))]
pub struct LibraryRecord;

#[cfg(not(feature = "web"))]
pub struct LibraryListQuery;

#[cfg(not(feature = "web"))]
pub struct LibraryListResponse;

#[cfg(not(feature = "web"))]
pub struct LibraryStatsResponse;
