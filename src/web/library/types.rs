//! # Library模块类型定义
//!
//! 定义了库管理功能相关的所有数据结构和类型

#[cfg(feature = "web")]
use serde::{Deserialize, Serialize};

/// 库域名分类请求
#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct LibraryRequest {
    /// 指定域名过滤
    pub domain: Option<String>,
    /// 页码（从1开始）
    pub page: Option<usize>,
    /// 每页限制数量
    pub limit: Option<usize>,
    /// 排序方式 ("cache_count", "last_updated", "domain_name")
    pub sort_by: Option<String>,
    /// 排序顺序 ("asc", "desc")
    pub sort_order: Option<String>,
}

/// 库域名分类响应
#[cfg(feature = "web")]
#[derive(Debug, Serialize)]
pub struct LibraryResponse {
    /// 域名列表
    pub domains: Vec<LibraryDomain>,
    /// 总域名数量
    pub total_domains: usize,
    /// 当前页码
    pub page: usize,
    /// 每页限制数量
    pub limit: usize,
    /// 总页数
    pub total_pages: usize,
}

/// 域名库条目
#[cfg(feature = "web")]
#[derive(Debug, Serialize, Clone)]
pub struct LibraryDomain {
    /// 域名
    pub domain: String,
    /// 缓存数量
    pub cache_count: usize,
    /// 总大小（字节）
    pub total_size_bytes: usize,
    /// 最后更新时间（Unix时间戳）
    pub last_updated: u64,
    /// 该域名下的URL列表
    pub urls: Vec<LibraryUrl>,
}

/// 域名下的URL条目
#[cfg(feature = "web")]
#[derive(Debug, Serialize, Clone)]
pub struct LibraryUrl {
    /// URL地址
    pub url: String,
    /// 页面标题
    pub title: Option<String>,
    /// 源语言
    pub source_lang: String,
    /// 目标语言
    pub target_lang: String,
    /// 创建时间（Unix时间戳）
    pub created_at: u64,
    /// 过期时间（Unix时间戳）
    pub expires_at: u64,
    /// 大小（字节）
    pub size_bytes: usize,
    /// 是否已过期
    pub is_expired: bool,
}

/// 库操作请求
#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct LibraryActionRequest {
    /// 操作类型
    pub action: LibraryAction,
    /// 域名（用于域名级操作）
    pub domain: Option<String>,
    /// URL（用于URL级操作）
    pub url: Option<String>,
    /// 源语言
    pub source_lang: Option<String>,
    /// 目标语言
    pub target_lang: Option<String>,
}

/// 库操作类型
#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryAction {
    /// 删除整个域名的所有缓存
    DeleteDomain,
    /// 删除特定URL的缓存
    DeleteUrl,
    /// 刷新域名（重新抓取最新内容）
    RefreshDomain,
    /// 刷新特定URL
    RefreshUrl,
    /// 清理过期的缓存条目
    CleanupExpired,
    /// 获取域名详细信息
    GetDomainDetails,
}

/// 库操作响应
#[cfg(feature = "web")]
#[derive(Debug, Serialize)]
pub struct LibraryActionResponse {
    /// 操作是否成功
    pub success: bool,
    /// 响应消息
    pub message: String,
    /// 影响的条目数量
    pub affected_count: Option<usize>,
    /// 操作的具体数据（可选）
    pub data: Option<serde_json::Value>,
}

/// 库统计信息
#[cfg(feature = "web")]
#[derive(Debug, Serialize)]
pub struct LibraryStats {
    /// 总域名数量
    pub total_domains: usize,
    /// 总URL数量
    pub total_urls: usize,
    /// 总缓存大小（字节）
    pub total_size_bytes: usize,
    /// 已过期的URL数量
    pub expired_urls: usize,
    /// 最近活跃的域名（Top 10）
    pub top_domains: Vec<LibraryDomain>,
    /// 最大的缓存条目
    pub largest_entries: Vec<LibraryUrl>,
}

/// 域名详情请求
#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct DomainDetailsRequest {
    /// 域名
    pub domain: String,
    /// 页码
    pub page: Option<usize>,
    /// 每页限制数量
    pub limit: Option<usize>,
    /// 是否包含过期条目
    pub include_expired: Option<bool>,
}

/// 域名详情响应
#[cfg(feature = "web")]
#[derive(Debug, Serialize)]
pub struct DomainDetailsResponse {
    /// 域名信息
    pub domain: LibraryDomain,
    /// 分页的URL列表
    pub urls: Vec<LibraryUrl>,
    /// 当前页码
    pub page: usize,
    /// 每页限制数量
    pub limit: usize,
    /// 总页数
    pub total_pages: usize,
}

// 非 web feature 的占位类型
#[cfg(not(feature = "web"))]
pub struct LibraryRequest;

#[cfg(not(feature = "web"))]
pub struct LibraryResponse;

#[cfg(not(feature = "web"))]
pub struct LibraryActionRequest;

#[cfg(not(feature = "web"))]
pub struct LibraryActionResponse;

/// 删除域名请求
#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct DeleteDomainRequest {
    /// 域名
    pub domain: String,
}

/// 刷新域名请求
#[cfg(feature = "web")]
#[derive(Debug, Deserialize)]
pub struct RefreshDomainRequest {
    /// 域名
    pub domain: String,
    /// 是否强制刷新（忽略缓存）
    pub force: Option<bool>,
}

#[cfg(not(feature = "web"))]
pub struct DeleteDomainRequest;

#[cfg(not(feature = "web"))]
pub struct RefreshDomainRequest;