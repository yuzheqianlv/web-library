# Web端智能链接路由与预览优化设计方案

## 1. 当前系统分析

### 1.1 Web端预览机制现状

#### 路由结构
```rust
// 主要路由
.route("/library/:id", get(view_translated_html))        // 通过ID预览翻译文档
.route("/website/*url", get(website_bookmarklet))        // 通过URL访问翻译服务
.route("/api/translate", post(translate_url))             // 翻译API
```

#### 数据存储结构
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedHtml {
    pub url: String,                    // 原始URL
    pub original_html: String,          // 原始HTML内容
    pub translated_html: String,        // 翻译后HTML内容
    pub title: Option<String>,          // 页面标题
    pub source_lang: String,            // 源语言
    pub target_lang: String,            // 目标语言
    pub status: String,                 // 处理状态: pending, success, error
    pub created_at: bson::DateTime,     // 创建时间
    pub updated_at: bson::DateTime,     // 更新时间
    pub expires_at: Option<bson::DateTime>, // 过期时间
    pub file_size: i64,                 // 文件大小
    pub domain: Option<String>,         // 域名
}
```

### 1.2 当前链接跳转机制

#### 问题分析
1. **重复处理问题**: 
   - 当用户在预览页面中点击链接时，`/website/*url` 路由会触发新的翻译请求
   - 即使数据库中已有该URL的翻译缓存，系统仍会重新处理
   - 缺乏智能的缓存优先查询机制

2. **用户体验问题**:
   - 页面跳转时没有加载状态指示
   - 重复处理导致不必要的等待时间
   - 缺乏链接状态的可视化反馈

3. **性能影响**:
   - 重复的网络请求和翻译处理
   - 数据库写入压力
   - 服务器资源浪费

### 1.3 根本原因分析

#### 当前流程
```
用户点击链接 → /website/{url} 路由 → website_bookmarklet() → 检查缓存 → 
如果有缓存: 直接返回HTML
如果无缓存: 重定向到主页 → 用户手动触发翻译 → /api/translate → 重新处理
```

#### 问题根源
1. **缓存检查不全面**: `website_bookmarklet` 只检查完全匹配的URL，不考虑语言参数
2. **路由设计不合理**: 没有优先使用已有的翻译记录ID进行跳转
3. **缺乏智能决策**: 没有根据缓存状态选择最优的处理策略

## 2. 智能链接路由系统设计

### 2.1 整体架构

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   链接拦截器    │ -> │  智能路由决策   │ -> │   缓存查询引擎  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         v                       v                       v
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ 链接状态检测    │    │ 路由策略选择    │    │  快速预览加载   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### 2.2 核心组件设计

#### 2.2.1 智能缓存查询引擎 (`src/web/services/cache_query_engine.rs`)

```rust
//! 智能缓存查询引擎
//! 
//! 提供高效的缓存查询和路由决策功能

use crate::web::types::CachedHtml;
use mongodb::{bson::doc, Collection, Database};
use std::collections::HashMap;
use url::Url;

/// 缓存查询结果
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

/// 智能缓存查询引擎
pub struct CacheQueryEngine {
    db: Database,
    collection: Collection<CachedHtml>,
    /// 内存缓存 (URL -> CacheQueryResult)
    memory_cache: std::sync::Arc<std::sync::Mutex<HashMap<String, (CacheQueryResult, std::time::Instant)>>>,
}

impl CacheQueryEngine {
    pub fn new(db: Database) -> Self {
        let collection = db.collection::<CachedHtml>("html_cache");
        Self {
            db,
            collection,
            memory_cache: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    /// 智能查询缓存状态
    pub async fn query_cache_status(
        &self,
        url: &str,
        source_lang: Option<&str>,
        target_lang: Option<&str>,
    ) -> Result<CacheQueryResult, mongodb::error::Error> {
        // 构建缓存键
        let cache_key = self.build_cache_key(url, source_lang, target_lang);
        
        // 首先检查内存缓存 (5分钟有效期)
        if let Some((cached_result, timestamp)) = self.get_from_memory_cache(&cache_key) {
            if timestamp.elapsed().as_secs() < 300 { // 5分钟
                return Ok(cached_result);
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

    /// 从内存缓存获取结果
    fn get_from_memory_cache(&self, key: &str) -> Option<(CacheQueryResult, std::time::Instant)> {
        if let Ok(cache) = self.memory_cache.lock() {
            cache.get(key).cloned()
        } else {
            None
        }
    }

    /// 更新内存缓存
    fn update_memory_cache(&self, key: String, result: CacheQueryResult) {
        if let Ok(mut cache) = self.memory_cache.lock() {
            cache.insert(key, (result, std::time::Instant::now()));
            
            // 清理过期条目 (超过1000个条目时)
            if cache.len() > 1000 {
                let cutoff_time = std::time::Instant::now() - std::time::Duration::from_secs(300);
                cache.retain(|_, (_, timestamp)| *timestamp > cutoff_time);
            }
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
}
```

#### 2.2.2 智能路由处理器 (`src/web/handlers/smart_routing.rs`)

```rust
//! 智能路由处理器
//! 
//! 提供智能的链接路由和预览功能

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
};
use serde::Deserialize;
use std::sync::Arc;

use crate::web::{
    services::cache_query_engine::{CacheQueryEngine, RoutingStrategy},
    types::AppState,
};

/// 智能路由查询参数
#[derive(Debug, Deserialize)]
pub struct SmartRoutingQuery {
    /// 源语言
    pub source_lang: Option<String>,
    /// 目标语言
    pub target_lang: Option<String>,
    /// 强制重新处理
    pub force_reprocess: Option<bool>,
    /// 来源页面 (用于链接关联)
    pub ref_url: Option<String>,
}

/// 增强的网站书签处理器 - 智能路由版本
pub async fn smart_website_handler(
    Path(url_path): Path<String>,
    Query(params): Query<SmartRoutingQuery>,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    // URL 处理
    let clean_url = url_path.trim_start_matches('/');
    let final_url = if clean_url.starts_with("http://") || clean_url.starts_with("https://") {
        clean_url.to_string()
    } else {
        format!("https://{}", clean_url)
    };

    // 如果强制重新处理，跳过缓存查询
    if params.force_reprocess.unwrap_or(false) {
        return handle_force_reprocess(&state, &final_url, &params).await;
    }

    // 智能缓存查询
    let cache_engine = CacheQueryEngine::new(
        state.mongo_database.as_ref().unwrap().clone()
    );

    let query_result = match cache_engine.query_cache_status(
        &final_url,
        params.source_lang.as_deref(),
        params.target_lang.as_deref(),
    ).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("缓存查询失败: {}", e);
            // 查询失败时回退到传统处理
            return handle_fallback_processing(&state, &final_url, &params).await;
        }
    };

    // 根据路由策略处理请求
    match query_result.routing_strategy {
        RoutingStrategy::UseCache(record_id) => {
            // 直接重定向到缓存预览页面
            tracing::info!("使用缓存记录: {} for URL: {}", record_id, final_url);
            let redirect_url = format!("/library/{}", record_id);
            Err((StatusCode::SEE_OTHER, Html(generate_redirect_page(&redirect_url, &final_url))))
        }
        
        RoutingStrategy::WaitForProcessing => {
            // 生成等待页面，带有自动刷新
            tracing::info!("URL正在处理中，生成等待页面: {}", final_url);
            Ok(Html(generate_waiting_page(&final_url, &params)))
        }
        
        RoutingStrategy::ReprocessWithCheck => {
            // 重新处理但检查状态
            tracing::info!("缓存已过期，重新处理: {}", final_url);
            handle_reprocess_with_check(&state, &final_url, &params, query_result.record_id).await
        }
        
        RoutingStrategy::FullReprocess => {
            // 完全重新处理
            tracing::info!("执行完全重新处理: {}", final_url);
            handle_full_reprocess(&state, &final_url, &params).await
        }
    }
}

/// 处理强制重新处理
async fn handle_force_reprocess(
    state: &AppState,
    url: &str,
    params: &SmartRoutingQuery,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    // 生成带预加载URL的处理页面
    Ok(Html(generate_processing_page(url, params, true)))
}

/// 处理回退处理
async fn handle_fallback_processing(
    state: &AppState,
    url: &str,
    params: &SmartRoutingQuery,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    // 使用原始的website_bookmarklet逻辑
    Ok(Html(generate_processing_page(url, params, false)))
}

/// 处理重新处理检查
async fn handle_reprocess_with_check(
    state: &AppState,
    url: &str,
    params: &SmartRoutingQuery,
    existing_record_id: Option<String>,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    // 生成智能处理页面，提供选择：使用过期缓存或重新处理
    Ok(Html(generate_choice_page(url, params, existing_record_id)))
}

/// 处理完全重新处理
async fn handle_full_reprocess(
    state: &AppState,
    url: &str,
    params: &SmartRoutingQuery,
) -> Result<Html<String>, (StatusCode, Html<String>)> {
    // 生成标准处理页面
    Ok(Html(generate_processing_page(url, params, false)))
}

/// 生成重定向页面
fn generate_redirect_page(redirect_url: &str, original_url: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>智能跳转 - Monolith</title>
    <style>
        body {{ 
            font-family: Arial, sans-serif; 
            text-align: center; 
            padding: 50px; 
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            margin: 0;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }}
        .container {{
            background: rgba(255, 255, 255, 0.1);
            padding: 2rem;
            border-radius: 15px;
            backdrop-filter: blur(10px);
            border: 1px solid rgba(255, 255, 255, 0.2);
            max-width: 500px;
        }}
        .spinner {{
            border: 3px solid rgba(255, 255, 255, 0.3);
            border-radius: 50%;
            border-top: 3px solid white;
            width: 40px;
            height: 40px;
            animation: spin 1s linear infinite;
            margin: 20px auto;
        }}
        @keyframes spin {{
            0% {{ transform: rotate(0deg); }}
            100% {{ transform: rotate(360deg); }}
        }}
        .url-info {{
            background: rgba(255, 255, 255, 0.1);
            padding: 1rem;
            border-radius: 8px;
            margin: 1rem 0;
            word-break: break-all;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h2>🚀 智能跳转中...</h2>
        <div class="spinner"></div>
        <p>找到已翻译的页面，正在跳转...</p>
        <div class="url-info">
            <strong>目标页面:</strong><br>
            <small>{}</small>
        </div>
        <p><small>如果页面未自动跳转，请<a href="{}" style="color: #4facfe;">点击这里</a></small></p>
    </div>
    <script>
        // 立即跳转到缓存页面
        window.location.href = "{}";
        
        // 备用跳转 (3秒后)
        setTimeout(() => {{
            if (window.location.pathname !== "{}") {{
                window.location.href = "{}";
            }}
        }}, 3000);
    </script>
</body>
</html>"#,
        original_url, redirect_url, redirect_url, redirect_url, redirect_url
    )
}

/// 生成等待页面
fn generate_waiting_page(url: &str, params: &SmartRoutingQuery) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>翻译处理中 - Monolith</title>
    <style>
        body {{ 
            font-family: Arial, sans-serif; 
            text-align: center; 
            padding: 50px; 
            background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
            color: white;
            margin: 0;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }}
        .container {{
            background: rgba(255, 255, 255, 0.1);
            padding: 2rem;
            border-radius: 15px;
            backdrop-filter: blur(10px);
            border: 1px solid rgba(255, 255, 255, 0.2);
            max-width: 600px;
        }}
        .progress-bar {{
            width: 100%;
            height: 6px;
            background: rgba(255, 255, 255, 0.3);
            border-radius: 3px;
            overflow: hidden;
            margin: 20px 0;
        }}
        .progress-fill {{
            height: 100%;
            background: white;
            width: 0%;
            animation: progress 10s ease-in-out infinite;
        }}
        @keyframes progress {{
            0%, 100% {{ width: 0%; }}
            50% {{ width: 80%; }}
        }}
        .refresh-info {{
            background: rgba(255, 255, 255, 0.1);
            padding: 1rem;
            border-radius: 8px;
            margin: 1rem 0;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h2>⏳ 页面翻译处理中...</h2>
        <div class="progress-bar">
            <div class="progress-fill"></div>
        </div>
        <p>检测到该页面正在翻译处理中，请稍候...</p>
        <div class="refresh-info">
            <strong>目标页面:</strong><br>
            <small>{}</small>
        </div>
        <p><small>页面将在处理完成后自动刷新</small></p>
        <p><small>预计等待时间: 30-60秒</small></p>
    </div>
    <script>
        let refreshCount = 0;
        const maxRefresh = 12; // 最多刷新12次 (2分钟)
        
        function checkStatus() {{
            refreshCount++;
            if (refreshCount >= maxRefresh) {{
                // 超时后重定向到强制重新处理
                const url = new URL(window.location);
                url.searchParams.set('force_reprocess', 'true');
                window.location.href = url.toString();
                return;
            }}
            
            // 每10秒刷新一次页面检查状态
            setTimeout(() => {{
                window.location.reload();
            }}, 10000);
        }}
        
        checkStatus();
    </script>
</body>
</html>"#,
        url
    )
}

/// 生成选择页面
fn generate_choice_page(url: &str, params: &SmartRoutingQuery, existing_record_id: Option<String>) -> String {
    let record_link = existing_record_id
        .map(|id| format!("/library/{}", id))
        .unwrap_or_default();

    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>选择处理方式 - Monolith</title>
    <style>
        body {{ 
            font-family: Arial, sans-serif; 
            text-align: center; 
            padding: 50px; 
            background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
            color: white;
            margin: 0;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }}
        .container {{
            background: rgba(255, 255, 255, 0.1);
            padding: 2rem;
            border-radius: 15px;
            backdrop-filter: blur(10px);
            border: 1px solid rgba(255, 255, 255, 0.2);
            max-width: 600px;
        }}
        .choices {{
            display: flex;
            gap: 1rem;
            margin: 2rem 0;
            flex-wrap: wrap;
        }}
        .choice-btn {{
            flex: 1;
            padding: 1rem;
            background: rgba(255, 255, 255, 0.2);
            border: none;
            border-radius: 8px;
            color: white;
            cursor: pointer;
            transition: all 0.3s ease;
            text-decoration: none;
            display: block;
            min-width: 200px;
        }}
        .choice-btn:hover {{
            background: rgba(255, 255, 255, 0.3);
            transform: translateY(-2px);
        }}
        .choice-primary {{
            background: rgba(76, 175, 80, 0.8);
        }}
        .choice-secondary {{
            background: rgba(255, 193, 7, 0.8);
        }}
        .url-info {{
            background: rgba(255, 255, 255, 0.1);
            padding: 1rem;
            border-radius: 8px;
            margin: 1rem 0;
            word-break: break-all;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h2>📋 选择处理方式</h2>
        <p>检测到该页面存在旧的翻译版本，请选择处理方式：</p>
        <div class="url-info">
            <strong>目标页面:</strong><br>
            <small>{}</small>
        </div>
        <div class="choices">
            <a href="{}" class="choice-btn choice-primary">
                <strong>🔄 重新翻译</strong><br>
                <small>获取最新的翻译结果</small>
            </a>
            <a href="{}" class="choice-btn choice-secondary">
                <strong>📖 查看旧版本</strong><br>
                <small>使用已有的翻译缓存</small>
            </a>
        </div>
        <p><small>建议选择重新翻译以获得最新内容</small></p>
    </div>
</body>
</html>"#,
        url,
        format!("{}?force_reprocess=true", url),
        record_link
    )
}

/// 生成处理页面
fn generate_processing_page(url: &str, params: &SmartRoutingQuery, force: bool) -> String {
    let action_text = if force { "强制重新翻译" } else { "智能处理" };
    
    format!(
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - Monolith</title>
    <style>
        body {{ 
            font-family: Arial, sans-serif; 
            text-align: center; 
            padding: 50px; 
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            margin: 0;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
        }}
        .container {{
            background: rgba(255, 255, 255, 0.1);
            padding: 2rem;
            border-radius: 15px;
            backdrop-filter: blur(10px);
            border: 1px solid rgba(255, 255, 255, 0.2);
            max-width: 500px;
        }}
        .spinner {{
            border: 3px solid rgba(255, 255, 255, 0.3);
            border-radius: 50%;
            border-top: 3px solid white;
            width: 40px;
            height: 40px;
            animation: spin 1s linear infinite;
            margin: 20px auto;
        }}
        @keyframes spin {{
            0% {{ transform: rotate(0deg); }}
            100% {{ transform: rotate(360deg); }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <h2>🔄 {}</h2>
        <div class="spinner"></div>
        <p>正在重定向到处理页面...</p>
        <p><small>目标: {}</small></p>
    </div>
    <script>
        // 重定向到主页并预加载 URL
        const url = "{}";
        const targetUrl = "/?preload_url=" + encodeURIComponent(url);
        window.location.href = targetUrl;
    </script>
</body>
</html>"#,
        action_text, action_text, url, url
    )
}
```

### 2.3 增强的链接拦截器

#### 2.3.1 智能链接拦截脚本

```javascript
// 注入到翻译页面中的智能链接拦截脚本
(function() {
    'use strict';
    
    const SMART_ROUTING_BASE = '/smart-website/';
    const CURRENT_DOMAIN = window.location.hostname;
    
    // 链接状态缓存
    const linkStatusCache = new Map();
    
    // 检查链接状态
    async function checkLinkStatus(url) {
        // 先检查缓存
        if (linkStatusCache.has(url)) {
            const cached = linkStatusCache.get(url);
            // 缓存5分钟有效
            if (Date.now() - cached.timestamp < 300000) {
                return cached.status;
            }
        }
        
        try {
            const response = await fetch(`/api/v2/link-status?url=${encodeURIComponent(url)}`);
            const status = await response.json();
            
            // 更新缓存
            linkStatusCache.set(url, {
                status: status,
                timestamp: Date.now()
            });
            
            return status;
        } catch (e) {
            console.warn('检查链接状态失败:', e);
            return { cached: false, processing: false };
        }
    }
    
    // 为链接添加状态指示器
    function addLinkStatusIndicator(link, status) {
        // 移除现有指示器
        const existingIndicator = link.querySelector('.link-status-indicator');
        if (existingIndicator) {
            existingIndicator.remove();
        }
        
        const indicator = document.createElement('span');
        indicator.className = 'link-status-indicator';
        indicator.style.cssText = `
            display: inline-block;
            width: 8px;
            height: 8px;
            border-radius: 50%;
            margin-left: 4px;
            vertical-align: middle;
        `;
        
        if (status.cached) {
            // 绿色 - 已缓存
            indicator.style.backgroundColor = '#4CAF50';
            indicator.title = '已翻译 - 快速加载';
        } else if (status.processing) {
            // 黄色 - 处理中
            indicator.style.backgroundColor = '#FFC107';
            indicator.title = '翻译中 - 请稍候';
            // 添加动画
            indicator.style.animation = 'pulse 1.5s infinite';
        } else {
            // 灰色 - 未处理
            indicator.style.backgroundColor = '#9E9E9E';
            indicator.title = '需要翻译';
        }
        
        link.appendChild(indicator);
    }
    
    // 批量检查页面中的链接状态
    async function batchCheckLinkStatus() {
        const links = document.querySelectorAll('a[href]');
        const urlsToCheck = [];
        
        links.forEach(link => {
            const href = link.getAttribute('href');
            if (shouldInterceptLink(href)) {
                const absoluteUrl = new URL(href, window.location.href).href;
                urlsToCheck.push({ link, url: absoluteUrl });
            }
        });
        
        // 分批检查 (每批20个)
        const batchSize = 20;
        for (let i = 0; i < urlsToCheck.length; i += batchSize) {
            const batch = urlsToCheck.slice(i, i + batchSize);
            
            // 并行检查当前批次
            const promises = batch.map(async ({ link, url }) => {
                try {
                    const status = await checkLinkStatus(url);
                    addLinkStatusIndicator(link, status);
                } catch (e) {
                    console.warn('检查链接状态失败:', url, e);
                }
            });
            
            await Promise.all(promises);
            
            // 避免过于频繁的请求
            if (i + batchSize < urlsToCheck.length) {
                await new Promise(resolve => setTimeout(resolve, 100));
            }
        }
    }
    
    // 智能链接拦截
    function interceptClick(event) {
        const link = event.target.closest('a');
        if (!link || !link.href) return;
        
        const originalUrl = link.href;
        
        if (shouldInterceptLink(originalUrl)) {
            event.preventDefault();
            
            // 显示加载状态
            showLinkLoadingState(link);
            
            // 构建智能路由URL
            const smartUrl = buildSmartRoutingUrl(originalUrl);
            
            // 跳转到智能路由
            window.location.href = smartUrl;
        }
    }
    
    // 构建智能路由URL
    function buildSmartRoutingUrl(originalUrl) {
        const url = new URL(SMART_ROUTING_BASE + encodeURIComponent(originalUrl), window.location.origin);
        
        // 添加当前页面作为引用
        url.searchParams.set('ref_url', window.location.href);
        
        // 添加语言参数 (如果能检测到)
        const currentLang = detectCurrentLanguage();
        if (currentLang.source !== 'auto') {
            url.searchParams.set('source_lang', currentLang.source);
        }
        if (currentLang.target !== 'zh') {
            url.searchParams.set('target_lang', currentLang.target);
        }
        
        return url.toString();
    }
    
    // 检测当前页面语言
    function detectCurrentLanguage() {
        // 从页面元数据检测
        const langMeta = document.querySelector('meta[name="translation-lang"]');
        if (langMeta) {
            const [source, target] = langMeta.content.split('->');
            return { source: source || 'auto', target: target || 'zh' };
        }
        
        // 默认值
        return { source: 'auto', target: 'zh' };
    }
    
    // 显示链接加载状态
    function showLinkLoadingState(link) {
        link.style.opacity = '0.6';
        link.style.pointerEvents = 'none';
        
        const loadingIndicator = document.createElement('span');
        loadingIndicator.textContent = ' ⏳';
        loadingIndicator.className = 'link-loading-indicator';
        link.appendChild(loadingIndicator);
    }
    
    // 应该拦截的链接判断
    function shouldInterceptLink(href) {
        if (!href || href.startsWith('#') || href.startsWith('javascript:') || 
            href.startsWith('mailto:') || href.startsWith('tel:') || 
            href.startsWith('data:') || href.startsWith('blob:')) {
            return false;
        }
        
        try {
            const url = new URL(href, window.location.href);
            // 只拦截同域或相关域名的链接
            return url.hostname === CURRENT_DOMAIN || url.hostname.endsWith('.' + CURRENT_DOMAIN);
        } catch (e) {
            return false;
        }
    }
    
    // 添加样式
    const style = document.createElement('style');
    style.textContent = `
        @keyframes pulse {
            0%, 100% { opacity: 1; }
            50% { opacity: 0.5; }
        }
        
        .link-status-indicator {
            transition: all 0.3s ease;
        }
        
        a:hover .link-status-indicator {
            transform: scale(1.2);
        }
    `;
    document.head.appendChild(style);
    
    // 初始化
    function initialize() {
        // 事件委托 - 拦截链接点击
        document.addEventListener('click', interceptClick, true);
        
        // 页面加载完成后批量检查链接状态
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', () => {
                setTimeout(batchCheckLinkStatus, 1000);
            });
        } else {
            setTimeout(batchCheckLinkStatus, 1000);
        }
        
        console.log('智能链接拦截器已初始化');
    }
    
    initialize();
})();
```

### 2.4 API增强

#### 2.4.1 链接状态检查API

```rust
// src/web/handlers/api/link_status.rs

use axum::{extract::Query, extract::State, response::Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::web::{
    services::cache_query_engine::CacheQueryEngine,
    types::AppState,
};

/// 链接状态查询参数
#[derive(Debug, Deserialize)]
pub struct LinkStatusQuery {
    pub url: String,
    pub source_lang: Option<String>,
    pub target_lang: Option<String>,
}

/// 链接状态响应
#[derive(Debug, Serialize)]
pub struct LinkStatusResponse {
    pub cached: bool,
    pub processing: bool,
    pub record_id: Option<String>,
    pub cache_age_minutes: Option<i64>,
}

/// 链接状态检查API
pub async fn check_link_status(
    Query(params): Query<LinkStatusQuery>,
    State(state): State<Arc<AppState>>,
) -> Json<LinkStatusResponse> {
    let cache_engine = CacheQueryEngine::new(
        state.mongo_database.as_ref().unwrap().clone()
    );

    match cache_engine.query_cache_status(
        &params.url,
        params.source_lang.as_deref(),
        params.target_lang.as_deref(),
    ).await {
        Ok(result) => {
            let cache_age_minutes = if let Some(record) = &result.cached_record {
                let created_at = record.created_at.timestamp_millis();
                let now = chrono::Utc::now().timestamp_millis();
                Some((now - created_at) / (1000 * 60)) // 转换为分钟
            } else {
                None
            };

            Json(LinkStatusResponse {
                cached: matches!(result.cache_status, crate::web::services::cache_query_engine::CacheStatus::Complete),
                processing: matches!(result.cache_status, crate::web::services::cache_query_engine::CacheStatus::Processing),
                record_id: result.record_id,
                cache_age_minutes,
            })
        }
        Err(_) => {
            Json(LinkStatusResponse {
                cached: false,
                processing: false,
                record_id: None,
                cache_age_minutes: None,
            })
        }
    }
}
```

## 3. 集成到现有系统

### 3.1 路由更新

```rust
// 在 src/web/routes.rs 中添加新路由
pub fn create_routes() -> Router<Arc<AppState>> {
    Router::new()
        // 现有路由...
        
        // 智能路由
        .route("/smart-website/*url", get(smart_website_handler))
        .route("/api/v2/link-status", get(check_link_status))
        
        // 现有路由...
}
```

### 3.2 翻译页面链接拦截注入

在翻译处理完成后，向HTML中注入智能链接拦截脚本：

```rust
// 在 translate_url 函数的最后，注入链接拦截脚本
if state.monolith_options.inject_link_interceptor {
    translated_html = inject_smart_link_interceptor(&translated_html, &url);
}
```

## 4. 优势和效果

### 4.1 性能优化
- **减少重复处理**: 智能缓存查询避免不必要的重新翻译
- **快速预览**: 直接使用缓存记录ID进行跳转
- **批量状态检查**: 预加载链接状态，提升用户体验

### 4.2 用户体验增强
- **无缝跳转**: 智能路由选择最优处理策略
- **状态可视化**: 链接状态指示器提供即时反馈
- **智能等待**: 处理中的页面提供等待界面和自动刷新

### 4.3 系统稳定性
- **错误恢复**: 多层回退机制确保系统可用性
- **内存优化**: LRU缓存和定期清理防止内存泄漏
- **并发控制**: 智能处理状态管理避免资源冲突

## 5. 实施计划

### Phase 1: 基础架构 (3-4天)
- 实现 `CacheQueryEngine`
- 创建智能路由处理器
- 添加基础API

### Phase 2: 前端增强 (2-3天)
- 实现智能链接拦截脚本
- 创建状态指示器
- 添加用户界面优化

### Phase 3: 集成测试 (2天)
- 集成到现有系统
- 性能测试和优化
- 用户体验测试

### Phase 4: 部署和监控 (1天)
- 部署配置
- 监控和日志
- 文档更新

这个设计方案彻底解决了web端预览时重复处理链接的问题，通过智能缓存查询和路由决策，大大提升了用户体验和系统性能。