# API端点和数据结构分析报告

## 📋 概述

基于对 `src/web/` 目录的详细分析，本报告总结了现有API端点、数据结构和架构设计，为新模板的API接口迁移提供基础。

## 🔍 现有API端点分析

### 核心路由结构 (src/web/routes.rs)

#### 页面路由
```rust
/ GET           - 首页
/library GET    - 库页面
/bookmarklet GET - 书签工具页面
/website/* GET  - 网站书签工具
```

#### API路由
```rust
// 翻译API
/api/translate POST        - 翻译URL
/api/content POST          - 获取内容

// 缓存API  
/api/cache/stats GET       - 缓存统计
/api/cache/clear POST      - 清理缓存
/api/cache/cleanup POST    - 清理缓存

// Library API (现有实现)
/api/library GET                  - 获取库数据
/api/library/stats GET           - 获取库统计
/api/library/domain GET          - 获取域名详情
/api/library/action POST         - 库操作
/api/library/batch-delete POST   - 批量删除
/api/library/size-stats GET      - 大小统计
/api/library/health GET          - 健康检查

// Theme API
/api/theme/* (多个端点)     - 主题管理
```

### 现有Library API详细分析

#### 1. GET /api/library
**当前实现**: 域名分组模式
- **输入**: `LibraryRequest`
  ```rust
  {
    domain: Option<String>,         // 域名过滤
    page: Option<usize>,           // 页码
    limit: Option<usize>,          // 每页数量
    sort_by: Option<String>,       // 排序字段
    sort_order: Option<String>     // 排序顺序
  }
  ```

- **输出**: `LibraryResponse`
  ```rust
  {
    domains: Vec<LibraryDomain>,   // 域名列表
    total_domains: usize,
    page: usize,
    limit: usize,
    total_pages: usize
  }
  ```

- **特点**: 
  - 按域名分组显示
  - 支持分页和排序
  - 域名级别的统计信息

#### 2. GET /api/library/stats
**当前实现**: 完整统计信息
- **输入**: 无参数
- **输出**: `LibraryStats`
  ```rust
  {
    total_domains: usize,
    total_urls: usize,
    total_size_bytes: usize,
    expired_urls: usize,
    top_domains: Vec<LibraryDomain>,
    largest_entries: Vec<LibraryUrl>
  }
  ```

#### 3. POST /api/library/action
**当前实现**: 库操作接口
- **输入**: `LibraryActionRequest`
- **支持操作**:
  - DeleteDomain - 删除域名
  - DeleteUrl - 删除URL
  - RefreshDomain - 刷新域名
  - RefreshUrl - 刷新URL
  - CleanupExpired - 清理过期项

## 🏗 数据结构分析

### 数据库模型

#### CachedHtml (src/web/types.rs)
```rust
pub struct CachedHtml {
    pub url: String,
    pub original_html: String,
    pub translated_html: String,
    pub title: Option<String>,
    pub source_lang: String,
    pub target_lang: String,
    pub created_at: bson::DateTime,
}
```

**特点**:
- 基于MongoDB存储
- 没有域名字段（从URL提取）
- 没有过期时间和状态字段
- 没有唯一ID暴露给前端

#### LibraryService业务逻辑
- 从URL动态提取域名
- 通过聚合查询计算统计数据
- 支持分页、排序、筛选

## 🎯 新模板需求对比分析

### 新模板预期API需求（基于TODO文档）

#### 1. GET /api/library (增强版)
**新需求**:
```rust
// 查询参数
{
  page: number,
  page_size: number,
  sort: string,
  order: "asc"|"desc", 
  search: string,        // 🆕 搜索关键词
  status: string,        // 🆕 状态过滤
  source_lang: string,   // 🆕 源语言过滤
  target_lang: string,   // 🆕 目标语言过滤
  date_from: string,     // 🆕 日期范围
  date_to: string        // 🆕 日期范围
}

// 响应格式
{
  records: [{
    id: "uuid",           // 🆕 唯一标识符
    url: string,
    title: string,
    source_lang: string,
    target_lang: string,
    status: "success|pending|error", // 🆕 状态字段
    created_at: string,   // 🆕 ISO格式时间
    file_size: number     // 🆕 文件大小
  }],
  pagination: {
    page: number,
    page_size: number,
    total: number,
    total_pages: number
  }
}
```

#### 2. GET /api/library/stats (增强版)
**新需求**:
```rust
{
  total: number,
  success: number,       // 🆕 成功数量
  pending: number,       // 🆕 处理中数量  
  error: number,         // 🆕 错误数量
  today: number,         // 🆕 今日翻译数量
  storage_size: number,
  languages: {           // 🆕 语言分布
    "en": number,
    "ja": number,
    "ko": number
  }
}
```

#### 3. 新增CRUD接口
```rust
GET    /api/library/{id}          // 🆕 获取单个记录
DELETE /api/library/{id}          // 🆕 删除单个记录
GET    /api/library/{id}/download // 🆕 下载文件
```

## ❌ 存在的差异和问题

### 1. 数据结构差异

| 字段 | 现有 | 新需求 | 影响 |
|------|------|--------|------|
| ID | 无前端暴露 | 需要UUID | 需要添加ID字段 |
| Status | 无 | success/pending/error | 需要添加状态字段 |
| File Size | 动态计算 | 存储字段 | 需要预计算并存储 |
| Domain | 动态提取 | 可能需要存储 | 性能优化需要 |
| Expires At | 无 | 需要 | 缓存管理需要 |
| Is Expired | 无 | 需要 | 过期管理需要 |

### 2. API设计差异

| 方面 | 现有 | 新需求 | 差异 |
|------|------|--------|------|
| 分页模式 | 域名级分页 | 记录级分页 | 完全不同的数据组织方式 |
| 排序字段 | domain_name, cache_count, last_updated | created_at, title, status, size | 需要新字段支持 |
| 搜索功能 | 仅域名筛选 | 全文搜索 + 多字段筛选 | 需要全新实现 |
| 响应格式 | 域名嵌套URL列表 | 扁平化记录列表 | 数据结构重构 |

### 3. 功能缺失

#### 当前没有的功能:
1. ❌ 单个记录CRUD操作
2. ❌ 文件下载接口  
3. ❌ 搜索和高级筛选
4. ❌ 批量导出功能
5. ❌ 状态管理（pending/success/error）
6. ❌ 今日统计和时间范围查询
7. ❌ 语言分布统计

## 💡 迁移建议

### 阶段1: 数据结构增强
1. **扩展CachedHtml结构**:
   ```rust
   pub struct CachedHtml {
       pub id: String,                    // 🆕 UUID
       pub url: String,
       pub title: Option<String>,
       pub domain: String,                // 🆕 预存储域名
       pub original_html: String,
       pub translated_html: String,
       pub source_lang: String,
       pub target_lang: String,
       pub status: String,                // 🆕 处理状态
       pub file_size: i64,                // 🆕 预计算大小
       pub created_at: bson::DateTime,
       pub updated_at: bson::DateTime,    // 🆕 更新时间
       pub expires_at: Option<bson::DateTime>, // 🆕 过期时间
       pub is_expired: bool,              // 🆕 过期标志
   }
   ```

2. **数据迁移脚本**: 为现有数据补充缺失字段

### 阶段2: API接口改造
1. **保持向后兼容**: 现有API继续工作
2. **新增增强接口**: 实现新的API规范
3. **渐进式替换**: 前端逐步切换到新接口

### 阶段3: 性能优化
1. **添加数据库索引**: 支持新的查询模式
2. **缓存优化**: 统计数据缓存
3. **分页优化**: 高效的记录级分页

## 🔄 建议的API演进路径

### 向后兼容策略
```
现有API -> 保持不变，标记为deprecated
新增API -> /api/v2/library/* 新版本接口
迁移期 -> 两套API并行运行
完成后 -> 移除旧API
```

### 新API设计原则
1. **RESTful**: 遵循REST最佳实践
2. **一致性**: 统一的请求/响应格式
3. **性能**: 考虑大数据量场景
4. **扩展性**: 预留未来功能扩展空间

---

**分析完成时间**: 2025-01-22  
**下一步**: 开始设计新API端点规范 (Task 2)