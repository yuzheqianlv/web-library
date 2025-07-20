//! 库管理相关API处理器

use std::sync::Arc;

#[cfg(feature = "web")]
use axum::{
    extract::{Json as ExtractJson, State},
    http::StatusCode,
    response::Json,
};

use crate::web::types::{
    AppState, LibraryResponse, LibraryDomain, LibraryUrl, LibraryActionRequest
};

/// 获取库数据处理器
#[cfg(feature = "web")]
pub async fn get_library_data(
    State(state): State<Arc<AppState>>,
) -> Result<Json<LibraryResponse>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref cache) = state.redis_cache {
        match cache.get_domains_stats() {
            Ok(domain_stats) => {
                let mut domains = Vec::new();
                
                for (domain, (count, size, last_updated)) in domain_stats {
                    // 获取该域名下的所有URL
                    let urls = match cache.get_by_domain(&domain) {
                        Ok(translations) => {
                            translations.into_iter().map(|t| LibraryUrl {
                                url: t.url,
                                title: t.title,
                                source_lang: t.source_lang,
                                target_lang: t.target_lang,
                                created_at: t.created_at,
                                expires_at: t.expires_at,
                                size_bytes: 0, // TODO: 计算实际大小
                            }).collect()
                        }
                        Err(_) => Vec::new(),
                    };
                    
                    domains.push(LibraryDomain {
                        domain,
                        cache_count: count,
                        total_size_bytes: size,
                        last_updated,
                        urls,
                    });
                }
                
                // 按缓存数量排序
                domains.sort_by(|a, b| b.cache_count.cmp(&a.cache_count));
                
                let response = LibraryResponse {
                    total_domains: domains.len(),
                    page: 1,
                    limit: 100,
                    domains,
                };
                
                Ok(Json(response))
            }
            Err(e) => {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": true,
                        "message": format!("获取库数据失败: {}", e)
                    }))
                ))
            }
        }
    } else {
        Ok(Json(LibraryResponse {
            domains: Vec::new(),
            total_domains: 0,
            page: 1,
            limit: 100,
        }))
    }
}

/// 库操作处理器
#[cfg(feature = "web")]
pub async fn library_action(
    State(state): State<Arc<AppState>>,
    ExtractJson(request): ExtractJson<LibraryActionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Some(ref cache) = state.redis_cache {
        let result = match request.action.as_str() {
            "delete_domain" => {
                if let Some(domain) = request.domain {
                    cache.delete_domain(&domain)
                        .map(|count| format!("成功删除域名 {} 的 {} 个缓存条目", domain, count))
                } else {
                    Err(redis::RedisError::from((redis::ErrorKind::TypeError, "Missing domain parameter")))
                }
            }
            "delete_url" => {
                if let (Some(url), Some(source_lang), Some(target_lang)) = 
                    (request.url, request.source_lang, request.target_lang) {
                    cache.delete(&url, &source_lang, &target_lang)
                        .map(|deleted| if deleted { 
                            "成功删除URL缓存".to_string() 
                        } else { 
                            "URL缓存不存在".to_string() 
                        })
                } else {
                    Err(redis::RedisError::from((redis::ErrorKind::TypeError, "Missing URL parameters")))
                }
            }
            "refresh_domain" => {
                // TODO: 实现域名刷新功能
                Ok("刷新功能待实现".to_string())
            }
            _ => {
                Err(redis::RedisError::from((redis::ErrorKind::TypeError, "Unknown action")))
            }
        };
        
        match result {
            Ok(message) => {
                Ok(Json(serde_json::json!({
                    "success": true,
                    "message": message
                })))
            }
            Err(e) => {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({
                        "error": true,
                        "message": format!("操作失败: {}", e)
                    }))
                ))
            }
        }
    } else {
        Ok(Json(serde_json::json!({
            "success": false,
            "message": "缓存未启用"
        })))
    }
}

/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub async fn get_library_data() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn library_action() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}