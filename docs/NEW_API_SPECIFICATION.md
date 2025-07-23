# 新API端点规范设计

## 📋 概述

本文档定义了Library模块化迁移所需的新API端点规范，采用RESTful设计原则，提供完整的库管理功能。

## 🎯 设计原则

### 1. RESTful架构
- 使用HTTP动词表示操作类型
- URL路径表示资源层级关系
- 统一的响应格式

### 2. 一致性原则
- 统一的错误处理格式
- 一致的分页参数
- 标准化的时间格式

### 3. 性能考虑
- 支持高效分页查询
- 缓存友好的设计
- 批量操作支持

### 4. 扩展性
- 版本控制支持
- 向后兼容保证
- 预留扩展字段

## 🛠 API版本控制策略

### URL版本控制
```
/api/v1/library/*  - 当前版本（兼容现有）
/api/v2/library/*  - 新版本（本次设计）
```

### 响应头版本信息
```http
X-API-Version: v2
X-API-Deprecated: false
```

## 📊 数据模型定义

### LibraryRecord - 库记录模型
```rust
#[derive(Debug, Serialize, Deserialize)]
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
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 更新时间
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// 过期时间
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 是否已过期
    pub is_expired: bool,
    /// 缓存类型
    pub cache_type: CacheType,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordStatus {
    Success,   // 翻译成功
    Pending,   // 处理中
    Error,     // 处理失败
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheType {
    Permanent,  // 永久缓存
    Temporary,  // 临时缓存
    Manual,     // 手动缓存
}
```

### Pagination - 分页信息
```rust
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
```

## 🔗 API端点详细规范

### 1. 库记录列表查询

#### `GET /api/v2/library`

**功能**: 获取库记录列表，支持分页、排序和筛选

**查询参数**:
```rust
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    Asc,
    Desc,
}
```

**响应格式**:
```rust
#[derive(Debug, Serialize)]
pub struct LibraryListResponse {
    /// 记录列表
    pub records: Vec<LibraryRecord>,
    /// 分页信息
    pub pagination: Pagination,
    /// 查询统计
    pub meta: QueryMeta,
}

#[derive(Debug, Serialize)]
pub struct QueryMeta {
    /// 查询耗时（毫秒）
    pub query_time_ms: i32,
    /// 总筛选条件数
    pub filters_applied: i32,
    /// 是否使用了搜索
    pub search_applied: bool,
}
```

**示例请求**:
```http
GET /api/v2/library?page=1&page_size=20&sort=created_at&order=desc&search=github&status=success&source_lang=en
```

**示例响应**:
```json
{
  "records": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "url": "https://github.com/example/repo",
      "title": "Example Repository - GitHub",
      "domain": "github.com",
      "source_lang": "en",
      "target_lang": "zh",
      "status": "success",
      "file_size": 125648,
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-15T10:30:00Z",
      "expires_at": "2024-02-15T10:30:00Z",
      "is_expired": false,
      "cache_type": "permanent"
    }
  ],
  "pagination": {
    "page": 1,
    "page_size": 20,
    "total": 1500,
    "total_pages": 75,
    "has_next": true,
    "has_prev": false
  },
  "meta": {
    "query_time_ms": 45,
    "filters_applied": 3,
    "search_applied": true
  }
}
```

### 2. 库统计信息

#### `GET /api/v2/library/stats`

**功能**: 获取库的统计信息

**查询参数**:
```rust
#[derive(Debug, Deserialize)]
pub struct StatsQuery {
    /// 是否包含详细统计
    #[serde(default)]
    pub detailed: bool,
    /// 时间范围（天）
    pub days: Option<i32>,
}
```

**响应格式**:
```rust
#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct LanguageStats {
    pub source_languages: HashMap<String, i64>,
    pub target_languages: HashMap<String, i64>,
}

#[derive(Debug, Serialize)]
pub struct StatusDistribution {
    pub success_rate: f64,
    pub error_rate: f64,
    pub pending_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct DomainStat {
    pub domain: String,
    pub count: i64,
    pub total_size: i64,
    pub avg_size: i64,
    pub success_rate: f64,
}

#[derive(Debug, Serialize)]
pub struct DailyStat {
    pub date: chrono::NaiveDate,
    pub count: i64,
    pub size: i64,
}
```

**示例响应**:
```json
{
  "total": 1500,
  "success": 1450,
  "pending": 30,
  "error": 20,
  "today": 45,
  "storage_size": 1048576000,
  "languages": {
    "source_languages": {
      "en": 800,
      "ja": 400,
      "ko": 300
    },
    "target_languages": {
      "zh": 1200,
      "en": 300
    }
  },
  "status_distribution": {
    "success_rate": 0.967,
    "error_rate": 0.013,
    "pending_rate": 0.020
  }
}
```

### 3. 获取单个记录

#### `GET /api/v2/library/{id}`

**功能**: 获取指定ID的库记录详情

**路径参数**:
- `id` - 记录唯一标识符

**查询参数**:
```rust
#[derive(Debug, Deserialize)]
pub struct RecordDetailQuery {
    /// 是否包含HTML内容预览
    #[serde(default)]
    pub include_preview: bool,
    /// 预览长度（字符数）
    pub preview_length: Option<i32>,
}
```

**响应格式**:
```rust
#[derive(Debug, Serialize)]
pub struct LibraryRecordDetailResponse {
    /// 记录信息
    pub record: LibraryRecord,
    /// HTML内容预览（可选）
    pub preview: Option<ContentPreview>,
    /// 元数据
    pub metadata: RecordMetadata,
}

#[derive(Debug, Serialize)]
pub struct ContentPreview {
    /// 原始HTML预览
    pub original_preview: String,
    /// 翻译HTML预览
    pub translated_preview: String,
    /// 是否截断
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
pub struct RecordMetadata {
    /// 文件存储路径
    pub file_path: Option<String>,
    /// 缓存命中次数
    pub hit_count: i64,
    /// 最后访问时间
    pub last_accessed: Option<chrono::DateTime<chrono::Utc>>,
    /// 翻译耗时（毫秒）
    pub translation_time_ms: Option<i32>,
}
```

### 4. 删除单个记录

#### `DELETE /api/v2/library/{id}`

**功能**: 删除指定ID的库记录

**路径参数**:
- `id` - 记录唯一标识符

**查询参数**:
```rust
#[derive(Debug, Deserialize)]
pub struct DeleteQuery {
    /// 是否同时删除文件
    #[serde(default = "default_true")]
    pub delete_files: bool,
    /// 是否强制删除
    #[serde(default)]
    pub force: bool,
}
```

**响应格式**:
```rust
#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    /// 是否成功
    pub success: bool,
    /// 删除的记录ID
    pub deleted_id: String,
    /// 释放的存储空间（字节）
    pub freed_bytes: i64,
    /// 删除时间
    pub deleted_at: chrono::DateTime<chrono::Utc>,
}
```

### 5. 下载翻译文件

#### `GET /api/v2/library/{id}/download`

**功能**: 下载指定记录的翻译HTML文件

**路径参数**:
- `id` - 记录唯一标识符

**查询参数**:
```rust
#[derive(Debug, Deserialize)]
pub struct DownloadQuery {
    /// 下载类型
    #[serde(default)]
    pub content_type: DownloadType,
    /// 是否内联显示
    #[serde(default)]
    pub inline: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadType {
    Original,    // 原始HTML
    Translated,  // 翻译后HTML
    Both,        // 压缩包含两个文件
}
```

**响应**: 直接返回文件流，设置适当的Content-Type和Content-Disposition头

### 6. 批量删除

#### `DELETE /api/v2/library/batch`

**功能**: 批量删除多个记录

**请求体**:
```rust
#[derive(Debug, Deserialize)]
pub struct BatchDeleteRequest {
    /// 要删除的记录ID列表
    pub ids: Vec<String>,
    /// 是否同时删除文件
    #[serde(default = "default_true")]
    pub delete_files: bool,
    /// 是否强制删除
    #[serde(default)]
    pub force: bool,
    /// 批量大小限制检查
    pub confirm_large_batch: Option<bool>,
}
```

**响应格式**:
```rust
#[derive(Debug, Serialize)]
pub struct BatchDeleteResponse {
    /// 请求删除的总数
    pub requested: i32,
    /// 成功删除的数量
    pub deleted: i32,
    /// 失败的数量
    pub failed: i32,
    /// 释放的存储空间总量
    pub total_freed_bytes: i64,
    /// 删除详情
    pub details: Vec<DeleteDetail>,
    /// 处理时间
    pub processing_time_ms: i32,
}

#[derive(Debug, Serialize)]
pub struct DeleteDetail {
    pub id: String,
    pub success: bool,
    pub error: Option<String>,
    pub freed_bytes: i64,
}
```

### 7. 批量导出

#### `POST /api/v2/library/export`

**功能**: 导出记录为不同格式

**请求体**:
```rust
#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    /// 要导出的记录ID列表（空表示全部）
    pub ids: Option<Vec<String>>,
    /// 导出格式
    pub format: ExportFormat,
    /// 筛选条件（替代IDs）
    pub filters: Option<LibraryListQuery>,
    /// 是否包含HTML内容
    #[serde(default)]
    pub include_content: bool,
    /// 压缩选项
    pub compression: Option<CompressionType>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Json,    // JSON格式
    Csv,     // CSV格式  
    Zip,     // ZIP压缩包
    Excel,   // Excel文件
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionType {
    None,
    Gzip,
    Zip,
}
```

**响应格式**:
```rust
#[derive(Debug, Serialize)]
pub struct ExportResponse {
    /// 导出任务ID
    pub task_id: String,
    /// 导出状态
    pub status: ExportStatus,
    /// 预估完成时间
    pub estimated_completion: chrono::DateTime<chrono::Utc>,
    /// 下载链接（完成后可用）
    pub download_url: Option<String>,
    /// 文件大小预估
    pub estimated_size_bytes: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportStatus {
    Queued,     // 排队中
    Processing, // 处理中
    Completed,  // 完成
    Failed,     // 失败
}
```

### 8. 搜索建议

#### `GET /api/v2/library/search/suggestions`

**功能**: 获取搜索建议和自动完成

**查询参数**:
```rust
#[derive(Debug, Deserialize)]
pub struct SearchSuggestionsQuery {
    /// 搜索关键词
    pub q: String,
    /// 建议类型
    pub suggestion_type: Option<SuggestionType>,
    /// 最大建议数量
    #[serde(default = "default_suggestion_limit")]
    pub limit: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionType {
    All,      // 所有类型
    Domains,  // 域名建议
    Titles,   // 标题建议
    URLs,     // URL建议
}
```

**响应格式**:
```rust
#[derive(Debug, Serialize)]
pub struct SearchSuggestionsResponse {
    /// 建议列表
    pub suggestions: Vec<SearchSuggestion>,
    /// 查询关键词
    pub query: String,
    /// 响应时间
    pub response_time_ms: i32,
}

#[derive(Debug, Serialize)]
pub struct SearchSuggestion {
    /// 建议文本
    pub text: String,
    /// 建议类型
    pub suggestion_type: SuggestionType,
    /// 匹配记录数
    pub match_count: i64,
    /// 高亮信息
    pub highlight: Option<HighlightInfo>,
}
```

## 🛡 错误处理规范

### 统一错误响应格式
```rust
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
```

### 错误代码定义
| HTTP状态码 | 错误代码 | 描述 |
|-----------|---------|------|
| 400 | INVALID_REQUEST | 请求参数无效 |
| 401 | UNAUTHORIZED | 未授权访问 |
| 403 | FORBIDDEN | 禁止访问 |
| 404 | RECORD_NOT_FOUND | 记录不存在 |
| 409 | RESOURCE_CONFLICT | 资源冲突 |
| 422 | VALIDATION_ERROR | 数据验证失败 |
| 429 | RATE_LIMIT_EXCEEDED | 请求频率超限 |
| 500 | INTERNAL_ERROR | 服务器内部错误 |
| 503 | SERVICE_UNAVAILABLE | 服务不可用 |

### 参数验证规则
```rust
pub struct ValidationRules {
    // 分页参数
    pub max_page_size: i32,      // 最大100
    pub min_page_size: i32,      // 最小1
    pub max_page: i32,           // 最大10000
    
    // 批量操作
    pub max_batch_size: i32,     // 批量操作最大1000个
    pub max_export_size: i32,    // 导出最大10000个
    
    // 搜索参数
    pub min_search_length: i32,  // 搜索关键词最短2字符
    pub max_search_length: i32,  // 搜索关键词最长100字符
    
    // 文件大小
    pub max_file_size: i64,      // 单文件最大100MB
}
```

## 🔐 认证和授权

### API密钥认证
```http
Authorization: Bearer <api_key>
X-API-Key: <api_key>
```

### 权限级别
```rust
pub enum Permission {
    ReadOnly,    // 只读权限
    ReadWrite,   // 读写权限
    Admin,       // 管理员权限
}
```

## 📝 请求和响应示例

### 完整的搜索请求示例
```http
POST /api/v2/library?page=1&page_size=20&sort=created_at&order=desc
Content-Type: application/json
Authorization: Bearer your-api-key

{
  "search": "github react",
  "status": "success",
  "source_lang": "en",
  "date_from": "2024-01-01T00:00:00Z",
  "date_to": "2024-01-31T23:59:59Z"
}
```

### 错误响应示例
```json
{
  "error_code": "VALIDATION_ERROR",
  "message": "Invalid page_size parameter",
  "details": {
    "field": "page_size",
    "value": 500,
    "max_allowed": 100
  },
  "request_id": "req_123456789",
  "timestamp": "2024-01-15T10:30:00Z"
}
```

## 🚀 性能指标和SLA

### 响应时间目标
- 列表查询: < 500ms (1000条记录)
- 统计查询: < 200ms  
- 单记录操作: < 100ms
- 文件下载: 并发支持100+

### 并发限制
- 单用户QPS限制: 100/秒
- 批量操作并发: 10个/用户
- 导出任务: 3个/用户

---

**设计完成时间**: 2025-01-22  
**下一步**: 实现GET /api/library分页查询接口 (Task 3)