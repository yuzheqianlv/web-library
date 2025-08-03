# 路由映射分析报告

## 当前路由结构分析

基于 `src/web/routes.rs` 文件的完整路由分析：

### 📄 页面路由 (Page Routes)
| 路由 | 处理器 | 状态 | 说明 |
|------|--------|------|------|
| `/` | `index` | ✅ **保留** | 主页 - 需要简化为书签脚本介绍页 |
| `/library` | `library_page` | ❌ **删除** | 文档库管理页面 |
| `/library/:id` | `view_translated_html` | ❌ **删除** | 查看翻译后的HTML |
| `/library-debug` | `library_debug_page` | ❌ **删除** | 调试页面 |
| `/bookmarklet` | `bookmarklet_page` | ✅ **保留** | 书签脚本页面 |
| `/website/*url` | `website_bookmarklet` | ✅ **保留** | 网页处理接口 |
| `/smart-website/*url` | `smart_website_handler` | ✅ **保留** | 智能网页处理 |

### 🔌 核心API路由 (Core API Routes)
| 路由 | 处理器 | 状态 | 说明 |
|------|--------|------|------|
| `/api/translate` | `translate_url` | 🔄 **修改** | 翻译API - 需要简化，移除批量处理 |
| `/api/content` | `get_content` | ✅ **保留** | 内容获取API |
| `/api/cache/stats` | `get_cache_stats` | 🔄 **简化** | 缓存统计 - 简化为基础缓存管理 |
| `/api/cache/clear` | `clear_cache` | ✅ **保留** | 清理缓存 |
| `/api/cache/cleanup` | `cleanup_cache` | ✅ **保留** | 缓存清理 |
| `/api/bookmarklet` | `generate_bookmarklet` | ✅ **保留** | 书签脚本生成 |

### 🔗 链接状态API (Link Status API) - V2
| 路由 | 处理器 | 状态 | 说明 |
|------|--------|------|------|
| `/api/v2/link-status` | `check_link_status` | ❌ **删除** | 链接状态检查 |
| `/api/v2/link-status/batch` | `check_batch_link_status` | ❌ **删除** | 批量链接状态检查 |
| `/api/v2/domain-links` | `get_domain_links` | ❌ **删除** | 域名链接获取 |

### 🎯 智能拦截器API (Smart Interceptor API)
| 路由 | 处理器 | 状态 | 说明 |
|------|--------|------|------|
| `/api/v2/interceptor/script` | `get_interceptor_script` | ✅ **保留** | 拦截器脚本 |
| `/api/v2/interceptor/injection` | `get_interceptor_injection_script` | ✅ **保留** | 注入脚本 |
| `/api/v2/interceptor/bookmarklet` | `get_enhanced_bookmarklet_script` | ✅ **保留** | 增强书签脚本 |

### 📚 Library API路由 (V1 - Legacy) - **全部删除**
| 路由 | 处理器 | 状态 | 说明 |
|------|--------|------|------|
| `/api/library` | `lib_handlers::get_library_data` | ❌ **删除** | 获取库数据 |
| `/api/library/stats` | `lib_handlers::get_library_stats` | ❌ **删除** | 库统计信息 |
| `/api/library/domain` | `lib_handlers::get_domain_details` | ❌ **删除** | 域名详情 |
| `/api/library/action` | `library_action` | ❌ **删除** | 库操作 |
| `/api/library/batch-delete` | `batch_delete_urls` | ❌ **删除** | 批量删除 |
| `/api/library/size-stats` | `get_size_statistics` | ❌ **删除** | 大小统计 |
| `/api/library/health` | `health_check` | ❌ **删除** | 健康检查 |

### 📚 Library API V2路由 (New) - **全部删除**
| 路由 | 处理器 | 状态 | 说明 |
|------|--------|------|------|
| `/api/v2/library` | `get_library_records_v2` | ❌ **删除** | V2库记录 |
| `/api/v2/library/stats` | `get_library_stats_v2` | ❌ **删除** | V2库统计 |
| `/api/v2/library/:id` | `get_library_record_by_id_v2` | ❌ **删除** | 按ID获取记录 |
| `/api/v2/library/:id` | `delete_library_record_v2` | ❌ **删除** | 按ID删除记录 |
| `/api/v2/library/:id/download` | `download_library_record_v2` | ❌ **删除** | 下载记录 |
| `/api/v2/library/batch-delete` | `batch_delete_library_records_v2` | ❌ **删除** | V2批量删除 |
| `/api/v2/library/export` | `export_library_records_v2` | ❌ **删除** | 导出记录 |
| `/api/v2/library/indexes/init` | `initialize_v2_indexes` | ❌ **删除** | 初始化索引 |

### 📊 增强统计API (Enhanced Stats API) - **全部删除**
| 路由 | 处理器 | 状态 | 说明 |
|------|--------|------|------|
| `/api/v2/library/stats/today` | `get_today_stats` | ❌ **删除** | 今日统计 |
| `/api/v2/library/stats/languages` | `get_language_distribution` | ❌ **删除** | 语言分布 |
| `/api/v2/library/stats/cache` | `get_cache_stats` | ❌ **删除** | 缓存统计 |
| `/api/v2/library/stats/dashboard` | `get_stats_dashboard` | ❌ **删除** | 统计仪表板 |

### 🗄️ 缓存管理API (Cache Management API) - **全部删除**
| 路由 | 处理器 | 状态 | 说明 |
|------|--------|------|------|
| `/api/v2/library/cache/clear` | `clear_stats_cache` | ❌ **删除** | 清理统计缓存 |
| `/api/v2/library/cache/invalidate` | `invalidate_stats_cache` | ❌ **删除** | 失效统计缓存 |

### 🎨 主题API路由 (Theme API Routes)
| 路由 | 处理器 | 状态 | 说明 |
|------|--------|------|------|
| `/api/theme/list` | `get_themes` | 🔄 **简化** | 主题列表 - 保留基础功能 |
| `/api/theme/current` | `get_current_theme_css` | ✅ **保留** | 当前主题CSS |
| `/api/theme/set` | `set_theme` | ❌ **删除** | 设置主题 - 删除复杂主题管理 |
| `/api/theme/css/:theme` | `get_theme_css` | ✅ **保留** | 获取主题CSS |
| `/api/theme/preview/:theme` | `preview_theme` | ❌ **删除** | 预览主题 |
| `/api/theme/selector` | `get_theme_selector` | ❌ **删除** | 主题选择器 |
| `/api/theme/script` | `get_theme_script` | ❌ **删除** | 主题脚本 |
| `/api/theme/register` | `register_theme` | ❌ **删除** | 注册主题 |
| `/api/theme/delete/:theme` | `delete_theme` | ❌ **删除** | 删除主题 |
| `/api/theme/preference` | `get_user_preference` | ❌ **删除** | 用户偏好 |
| `/api/theme/preference` | `set_user_preference` | ❌ **删除** | 设置用户偏好 |
| `/api/theme/auto-switch` | `auto_switch_theme` | ❌ **删除** | 自动切换主题 |

## 📋 轻量化后的路由结构

### 🎯 保留的核心路由
```rust
// 页面路由
.route("/", get(simple_index))                    // 简化主页
.route("/bookmarklet", get(bookmarklet_page))     // 书签脚本页面
.route("/preview/*url", get(preview_handler))     // 新增：预览路由

// 核心处理API
.route("/api/process", post(process_url))         // 新增：统一处理API
.route("/api/content", post(get_content))         // 内容获取
.route("/api/bookmarklet", get(generate_bookmarklet)) // 书签脚本生成

// 智能拦截器 (保留核心功能)
.route("/api/interceptor/script", get(get_interceptor_script))
.route("/api/interceptor/bookmarklet", get(get_enhanced_bookmarklet_script))

// 基础缓存管理
.route("/api/cache/clear", post(clear_cache))
.route("/api/cache/cleanup", post(cleanup_cache))

// 基础主题支持
.route("/api/theme/current", get(get_current_theme_css))
.route("/api/theme/css/:theme", get(get_theme_css))
```

### 🗑️ 删除的路由统计
- **页面路由**: 删除4个，保留3个（1个新增）
- **Library API**: 删除24个路由（V1和V2全部删除）
- **统计API**: 删除8个复杂统计路由
- **主题API**: 删除10个复杂主题管理路由
- **链接状态API**: 删除3个V2路由

**总计**: 从47个路由简化到约12个核心路由，删除率约75%

## 🔄 需要新增的路由

1. **`/preview/*url`** - 直接预览处理后的网页
2. **`/api/process`** - 统一的网页处理API，替代复杂的翻译API

## 📝 实施注意事项

1. **处理器依赖**: 删除的路由对应的处理器也需要删除
2. **导入清理**: 清理`routes.rs`中不再需要的导入
3. **状态管理**: 简化`AppState`，移除MongoDB和复杂状态
4. **错误处理**: 确保删除路由不会影响错误处理逻辑
5. **中间件**: 检查是否有路由特定的中间件需要清理

这个简化将大大减少代码复杂度，专注于核心的"书签脚本+预览"功能。