//! # Library V2 HTTP处理器
//!
//! 提供新版本库管理功能的HTTP路由处理器，支持记录级分页和高级查询

#[cfg(feature = "web")]
use std::sync::Arc;

#[cfg(feature = "web")]
use axum::{
    extract::{Query, State, Path},
    http::StatusCode,
    response::Json,
};

#[cfg(feature = "web")]
use futures::TryStreamExt;

#[cfg(feature = "web")]
use crate::web::types::AppState;

use super::v2_types::*;
use super::v2_service::LibraryServiceV2;

/// 获取库记录列表处理器 (V2版本)
///
/// 支持记录级分页、搜索、筛选和排序
#[cfg(feature = "web")]
pub async fn get_library_records_v2(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LibraryListQuery>,
) -> Result<Json<LibraryListResponse>, (StatusCode, Json<ApiError>)> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    if let Some(ref database) = state.mongo_database {
        let service = LibraryServiceV2::new(database.clone());
        
        match service.get_library_records(&query).await {
            Ok(response) => Ok(Json(response)),
            Err(e) => Err(create_api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "QUERY_ERROR",
                &format!("查询库记录失败: {}", e),
                request_id,
                None,
            ))
        }
    } else {
        // 如果没有MongoDB连接，返回空数据
        Ok(Json(LibraryListResponse {
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
                query_time_ms: 0,
                filters_applied: 0,
                search_applied: false,
            },
        }))
    }
}

/// 获取库统计信息处理器 (V2版本)
#[cfg(feature = "web")]
pub async fn get_library_stats_v2(
    State(state): State<Arc<AppState>>,
    Query(query): Query<StatsQuery>,
) -> Result<Json<LibraryStatsResponse>, (StatusCode, Json<ApiError>)> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    if let Some(ref database) = state.mongo_database {
        let service = LibraryServiceV2::new(database.clone());
        
        match service.get_library_stats(&query).await {
            Ok(stats) => Ok(Json(stats)),
            Err(e) => Err(create_api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "STATS_ERROR",
                &format!("获取统计信息失败: {}", e),
                request_id,
                None,
            ))
        }
    } else {
        Err(create_api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "DATABASE_UNAVAILABLE",
            "数据库服务不可用",
            request_id,
            None,
        ))
    }
}

/// 获取单个记录详情处理器
#[cfg(feature = "web")]
pub async fn get_library_record_by_id_v2(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<LibraryRecord>, (StatusCode, Json<ApiError>)> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    if let Some(ref database) = state.mongo_database {
        use mongodb::bson::{doc, oid::ObjectId};
        
        let collection = database.collection::<crate::web::types::CachedHtml>("html_cache");
        
        // 尝试通过ObjectId查找
        let filter = if let Ok(object_id) = ObjectId::parse_str(&id) {
            doc! { "_id": object_id }
        } else {
            // 如果不是ObjectId，尝试通过URL+语言组合查找
            return Err(create_api_error(
                StatusCode::BAD_REQUEST,
                "INVALID_ID",
                "无效的记录ID格式",
                request_id,
                None,
            ));
        };
        
        match collection.find_one(filter).await {
            Ok(Some(cached)) => {
                let service = LibraryServiceV2::new(database.clone());
                let record = service.cached_html_to_record(&cached, Some(ObjectId::parse_str(&id).unwrap()));
                Ok(Json(record))
            }
            Ok(None) => Err(create_api_error(
                StatusCode::NOT_FOUND,
                "RECORD_NOT_FOUND",
                &format!("记录不存在: {}", id),
                request_id,
                None,
            )),
            Err(e) => Err(create_api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "QUERY_ERROR",
                &format!("查询记录失败: {}", e),
                request_id,
                None,
            ))
        }
    } else {
        Err(create_api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "DATABASE_UNAVAILABLE",
            "数据库服务不可用",
            request_id,
            None,
        ))
    }
}

/// 删除单个记录处理器
#[cfg(feature = "web")]
pub async fn delete_library_record_v2(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<DeleteResponse>, (StatusCode, Json<ApiError>)> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    if let Some(ref database) = state.mongo_database {
        use mongodb::bson::{doc, oid::ObjectId};
        
        let collection = database.collection::<crate::web::types::CachedHtml>("html_cache");
        
        // 尝试通过ObjectId删除
        let filter = if let Ok(object_id) = ObjectId::parse_str(&id) {
            doc! { "_id": object_id }
        } else {
            return Err(create_api_error(
                StatusCode::BAD_REQUEST,
                "INVALID_ID",
                "无效的记录ID格式",
                request_id,
                None,
            ));
        };
        
        // 先查询记录以计算释放的空间
        match collection.find_one(filter.clone()).await {
            Ok(Some(cached)) => {
                let freed_bytes = (cached.original_html.len() + cached.translated_html.len()) as i64;
                
                match collection.delete_one(filter).await {
                    Ok(result) if result.deleted_count > 0 => {
                        Ok(Json(DeleteResponse {
                            success: true,
                            deleted_id: id,
                            freed_bytes,
                            deleted_at: chrono::Utc::now(),
                        }))
                    }
                    Ok(_) => Err(create_api_error(
                        StatusCode::NOT_FOUND,
                        "RECORD_NOT_FOUND",
                        "记录不存在或已被删除",
                        request_id,
                        None,
                    )),
                    Err(e) => Err(create_api_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "DELETE_ERROR",
                        &format!("删除记录失败: {}", e),
                        request_id,
                        None,
                    ))
                }
            }
            Ok(None) => Err(create_api_error(
                StatusCode::NOT_FOUND,
                "RECORD_NOT_FOUND",
                &format!("记录不存在: {}", id),
                request_id,
                None,
            )),
            Err(e) => Err(create_api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "QUERY_ERROR",
                &format!("查询记录失败: {}", e),
                request_id,
                None,
            ))
        }
    } else {
        Err(create_api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "DATABASE_UNAVAILABLE",
            "数据库服务不可用",
            request_id,
            None,
        ))
    }
}

/// 下载翻译文件处理器
#[cfg(feature = "web")]
pub async fn download_library_record_v2(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<DownloadQuery>,
) -> Result<axum::response::Response, (StatusCode, Json<ApiError>)> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    if let Some(ref database) = state.mongo_database {
        use mongodb::bson::{doc, oid::ObjectId};
        use axum::response::IntoResponse;
        use axum::http::{header, HeaderMap, HeaderValue};
        
        let collection = database.collection::<crate::web::types::CachedHtml>("html_cache");
        
        // 尝试通过ObjectId查找
        let filter = if let Ok(object_id) = ObjectId::parse_str(&id) {
            doc! { "_id": object_id }
        } else {
            return Err(create_api_error(
                StatusCode::BAD_REQUEST,
                "INVALID_ID",
                "无效的记录ID格式",
                request_id,
                None,
            ));
        };
        
        match collection.find_one(filter).await {
            Ok(Some(cached)) => {
                let (content, filename, content_type) = match query.content_type {
                    DownloadType::Original => {
                        (cached.original_html, format!("{}_original.html", id), "text/html")
                    }
                    DownloadType::Translated => {
                        (cached.translated_html, format!("{}_translated.html", id), "text/html")
                    }
                    DownloadType::Both => {
                        // 创建ZIP文件包含两个HTML文件
                        use std::io::{Cursor, Write};
                        use zip::{ZipWriter, write::FileOptions, CompressionMethod};
                        
                        let mut zip_buffer = Cursor::new(Vec::new());
                        {
                            let mut zip_writer = ZipWriter::new(&mut zip_buffer);
                            
                            // 添加原始HTML
                            zip_writer.start_file(
                                format!("{}_original.html", id),
                                FileOptions::default().compression_method(CompressionMethod::Deflated)
                            ).map_err(|e| create_api_error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "ZIP_ERROR",
                                &format!("创建ZIP文件失败: {}", e),
                                request_id.clone(),
                                None,
                            ))?;
                            zip_writer.write_all(cached.original_html.as_bytes()).map_err(|e| create_api_error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "ZIP_ERROR",
                                &format!("写入ZIP文件失败: {}", e),
                                request_id.clone(),
                                None,
                            ))?;
                            
                            // 添加翻译HTML
                            zip_writer.start_file(
                                format!("{}_translated.html", id),
                                FileOptions::default().compression_method(CompressionMethod::Deflated)
                            ).map_err(|e| create_api_error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "ZIP_ERROR",
                                &format!("创建ZIP文件失败: {}", e),
                                request_id.clone(),
                                None,
                            ))?;
                            zip_writer.write_all(cached.translated_html.as_bytes()).map_err(|e| create_api_error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "ZIP_ERROR",
                                &format!("写入ZIP文件失败: {}", e),
                                request_id.clone(),
                                None,
                            ))?;
                            
                            zip_writer.finish().map_err(|e| create_api_error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "ZIP_ERROR",
                                &format!("完成ZIP文件失败: {}", e),
                                request_id.clone(),
                                None,
                            ))?;
                        }
                        
                        let zip_data = zip_buffer.into_inner();
                        // ZIP文件是二进制数据，不能作为字符串返回
                        // 这里我们需要返回二进制响应
                        let headers = {
                            let mut h = HeaderMap::new();
                            h.insert(header::CONTENT_TYPE, HeaderValue::from_str("application/zip").unwrap());
                            let disposition = if query.inline {
                                format!("inline; filename=\"{}_both.zip\"", id)
                            } else {
                                format!("attachment; filename=\"{}_both.zip\"", id)
                            };
                            h.insert(header::CONTENT_DISPOSITION, HeaderValue::from_str(&disposition).unwrap());
                            h
                        };
                        return Ok((headers, zip_data).into_response());
                    }
                };
                
                let mut headers = HeaderMap::new();
                headers.insert(header::CONTENT_TYPE, HeaderValue::from_str(content_type).unwrap());
                
                let disposition = if query.inline {
                    format!("inline; filename=\"{}\"", filename)
                } else {
                    format!("attachment; filename=\"{}\"", filename)
                };
                headers.insert(header::CONTENT_DISPOSITION, HeaderValue::from_str(&disposition).unwrap());
                
                Ok((headers, content).into_response())
            }
            Ok(None) => Err(create_api_error(
                StatusCode::NOT_FOUND,
                "RECORD_NOT_FOUND",
                &format!("记录不存在: {}", id),
                request_id,
                None,
            )),
            Err(e) => Err(create_api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "QUERY_ERROR",
                &format!("查询记录失败: {}", e),
                request_id,
                None,
            ))
        }
    } else {
        Err(create_api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "DATABASE_UNAVAILABLE",
            "数据库服务不可用",
            request_id,
            None,
        ))
    }
}

/// 初始化V2索引处理器
#[cfg(feature = "web")]
pub async fn initialize_v2_indexes(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    if let Some(ref database) = state.mongo_database {
        let service = LibraryServiceV2::new(database.clone());
        
        match service.create_v2_indexes().await {
            Ok(()) => Ok(Json(serde_json::json!({
                "success": true,
                "message": "V2数据库索引创建成功"
            }))),
            Err(e) => Err(create_api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INDEX_ERROR",
                &format!("创建V2索引失败: {}", e),
                request_id,
                None,
            ))
        }
    } else {
        Err(create_api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "DATABASE_UNAVAILABLE",
            "数据库服务不可用",
            request_id,
            None,
        ))
    }
}

/// 下载查询参数
#[cfg(feature = "web")]
#[derive(Debug, serde::Deserialize)]
pub struct DownloadQuery {
    /// 下载类型
    #[serde(default)]
    pub content_type: DownloadType,
    /// 是否内联显示
    #[serde(default)]
    pub inline: bool,
}

/// 下载类型枚举
#[cfg(feature = "web")]
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadType {
    Original,    // 原始HTML
    Translated,  // 翻译后HTML
    Both,        // 压缩包含两个文件
}

#[cfg(feature = "web")]
impl Default for DownloadType {
    fn default() -> Self {
        DownloadType::Translated
    }
}

/// 删除响应
#[cfg(feature = "web")]
#[derive(Debug, serde::Serialize)]
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

/// 创建API错误响应的辅助函数
#[cfg(feature = "web")]
fn create_api_error(
    status: StatusCode,
    error_code: &str,
    message: &str,
    request_id: String,
    details: Option<serde_json::Value>,
) -> (StatusCode, Json<ApiError>) {
    (
        status,
        Json(ApiError {
            error_code: error_code.to_string(),
            message: message.to_string(),
            details,
            request_id,
            timestamp: chrono::Utc::now(),
        })
    )
}


/// 非 web feature 的占位函数
#[cfg(not(feature = "web"))]
pub async fn get_library_records_v2() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn get_library_stats_v2() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn get_library_record_by_id_v2() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn delete_library_record_v2() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn download_library_record_v2() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

/// 批量删除请求
#[cfg(feature = "web")]
#[derive(Debug, serde::Deserialize)]
pub struct BatchDeleteRequest {
    pub ids: Vec<String>,
}

/// 批量删除响应
#[cfg(feature = "web")]
#[derive(Debug, serde::Serialize)]
pub struct BatchDeleteResponse {
    pub deleted_count: usize,
    pub freed_bytes: u64,
    pub success: bool,
}

/// 导出请求
#[cfg(feature = "web")]
#[derive(Debug, serde::Deserialize)]
pub struct ExportRequest {
    pub ids: Vec<String>,
    pub format: Option<String>, // zip, json, csv
    pub include_originals: Option<bool>,
}

/// 批量删除记录处理器
#[cfg(feature = "web")]
pub async fn batch_delete_library_records_v2(
    State(state): State<Arc<AppState>>,
    Json(request): Json<BatchDeleteRequest>,
) -> Result<Json<BatchDeleteResponse>, (StatusCode, Json<ApiError>)> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    if let Some(ref database) = state.mongo_database {
        use mongodb::bson::{doc, oid::ObjectId};
        
        let collection = database.collection::<crate::web::types::CachedHtml>("html_cache");
        
        let mut object_ids = Vec::new();
        for id_str in &request.ids {
            if let Ok(object_id) = ObjectId::parse_str(id_str) {
                object_ids.push(object_id);
            }
        }
        
        if object_ids.is_empty() {
            return Err(create_api_error(
                StatusCode::BAD_REQUEST,
                "INVALID_IDS",
                "没有有效的记录ID",
                request_id,
                None,
            ));
        }
        
        // 先查询记录以计算释放的空间
        let filter = doc! { "_id": { "$in": &object_ids } };
        let mut total_freed_bytes = 0u64;
        
        match collection.find(filter.clone()).await {
            Ok(mut cursor) => {
                while let Some(cached) = cursor.try_next().await.unwrap_or(None) {
                    total_freed_bytes += (cached.original_html.len() + cached.translated_html.len()) as u64;
                }
            }
            Err(e) => {
                return Err(create_api_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "QUERY_ERROR",
                    &format!("查询记录失败: {}", e),
                    request_id,
                    None,
                ));
            }
        }
        
        // 执行批量删除
        match collection.delete_many(filter).await {
            Ok(result) => {
                Ok(Json(BatchDeleteResponse {
                    deleted_count: result.deleted_count as usize,
                    freed_bytes: total_freed_bytes,
                    success: true,
                }))
            }
            Err(e) => Err(create_api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DELETE_ERROR",
                &format!("批量删除失败: {}", e),
                request_id,
                None,
            ))
        }
    } else {
        Err(create_api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "DATABASE_UNAVAILABLE",
            "数据库服务不可用",
            request_id,
            None,
        ))
    }
}

/// 批量导出记录处理器
#[cfg(feature = "web")]
pub async fn export_library_records_v2(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ExportRequest>,
) -> Result<axum::response::Response, (StatusCode, Json<ApiError>)> {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    if let Some(ref database) = state.mongo_database {
        use mongodb::bson::{doc, oid::ObjectId};
        use axum::response::IntoResponse;
        use axum::http::{header, HeaderMap, HeaderValue};
        use std::io::{Cursor, Write};
        use zip::{ZipWriter, write::FileOptions, CompressionMethod};
        
        let collection = database.collection::<crate::web::types::CachedHtml>("html_cache");
        
        let mut object_ids = Vec::new();
        for id_str in &request.ids {
            if let Ok(object_id) = ObjectId::parse_str(id_str) {
                object_ids.push(object_id);
            }
        }
        
        if object_ids.is_empty() {
            return Err(create_api_error(
                StatusCode::BAD_REQUEST,
                "INVALID_IDS",
                "没有有效的记录ID",
                request_id,
                None,
            ));
        }
        
        let filter = doc! { "_id": { "$in": &object_ids } };
        
        match collection.find(filter).await {
            Ok(mut cursor) => {
                let mut zip_buffer = Cursor::new(Vec::new());
                let include_originals = request.include_originals.unwrap_or(true);
                
                {
                    let mut zip_writer = ZipWriter::new(&mut zip_buffer);
                    let mut file_index = 1;
                    
                    while let Some(cached) = cursor.try_next().await.unwrap_or(None) {
                        // 添加翻译后的HTML
                        let translated_filename = format!("{:03}_translated.html", file_index);
                        zip_writer.start_file(
                            translated_filename,
                            FileOptions::default().compression_method(CompressionMethod::Deflated)
                        ).map_err(|e| create_api_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "ZIP_ERROR",
                            &format!("创建ZIP文件失败: {}", e),
                            request_id.clone(),
                            None,
                        ))?;
                        zip_writer.write_all(cached.translated_html.as_bytes()).map_err(|e| create_api_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "ZIP_ERROR",
                            &format!("写入ZIP文件失败: {}", e),
                            request_id.clone(),
                            None,
                        ))?;
                        
                        // 如果需要，添加原始HTML
                        if include_originals {
                            let original_filename = format!("{:03}_original.html", file_index);
                            zip_writer.start_file(
                                original_filename,
                                FileOptions::default().compression_method(CompressionMethod::Deflated)
                            ).map_err(|e| create_api_error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "ZIP_ERROR",
                                &format!("创建ZIP文件失败: {}", e),
                                request_id.clone(),
                                None,
                            ))?;
                            zip_writer.write_all(cached.original_html.as_bytes()).map_err(|e| create_api_error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "ZIP_ERROR",
                                &format!("写入ZIP文件失败: {}", e),
                                request_id.clone(),
                                None,
                            ))?;
                        }
                        
                        file_index += 1;
                    }
                    
                    zip_writer.finish().map_err(|e| create_api_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "ZIP_ERROR",
                        &format!("完成ZIP文件失败: {}", e),
                        request_id.clone(),
                        None,
                    ))?;
                }
                
                let zip_data = zip_buffer.into_inner();
                let filename = format!("library_export_{}.zip", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
                
                let mut headers = HeaderMap::new();
                headers.insert(header::CONTENT_TYPE, HeaderValue::from_str("application/zip").unwrap());
                headers.insert(
                    header::CONTENT_DISPOSITION,
                    HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename)).unwrap()
                );
                
                Ok((headers, zip_data).into_response())
            }
            Err(e) => Err(create_api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "QUERY_ERROR",
                &format!("查询记录失败: {}", e),
                request_id,
                None,
            ))
        }
    } else {
        Err(create_api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "DATABASE_UNAVAILABLE",
            "数据库服务不可用",
            request_id,
            None,
        ))
    }
}

#[cfg(not(feature = "web"))]
pub async fn batch_delete_library_records_v2() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}

#[cfg(not(feature = "web"))]
pub async fn export_library_records_v2() -> Result<String, String> {
    Err("Web feature not enabled".to_string())
}