# JavaScript链接拦截与批量翻译扩展设计方案

## 1. 项目现状分析

### 1.1 当前JavaScript处理机制

#### JavaScript事件处理器识别
- **位置**: `src/parsers/js.rs`
- **功能**: 识别DOM事件处理器属性（onclick、onload等115种事件）
- **特点**: 基于WHATWG HTML规范的完整事件处理器列表
- **用途**: 在保存HTML时正确处理JavaScript事件属性

#### 链接重写器核心功能
- **位置**: `src/parsers/link_rewriter.rs`
- **当前功能**:
  - 重写`<a>`标签的`href`属性
  - 重写`<form>`标签的`action`属性
  - URL编码和翻译服务路径构建
  - 智能跳过特殊链接（锚点、JavaScript、邮件等）

#### 域名过滤机制
- **位置**: `src/utils/url.rs`
- **功能**: `domain_is_within_domain()` 函数提供域名匹配逻辑
- **支持**: 子域名匹配、通配符域名、精确域名匹配

### 1.2 现有翻译系统架构

```
src/translation/
├── core/           # 翻译引擎和服务
├── pipeline/       # 文本处理管道
│   ├── collector.rs    # 文本收集器
│   ├── batch.rs       # 批处理管理
│   └── filters.rs     # 文本过滤器
└── storage/        # 缓存存储系统
```

## 2. 批量翻译拦截链接架构设计

### 2.1 整体架构概览

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   主页面处理    │ -> │  链接发现引擎   │ -> │  批量翻译系统   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         v                       v                       v
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ 链接拦截注入    │    │ 同域链接过滤    │    │ 翻译缓存管理    │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### 2.2 核心组件设计

#### 2.2.1 链接发现引擎 (`src/parsers/link_discoverer.rs`)

```rust
//! 链接发现引擎
//! 
//! 负责从HTML文档中发现、分类和过滤链接，为批量翻译做准备

use url::Url;
use std::collections::HashSet;

/// 链接发现配置
#[derive(Debug, Clone)]
pub struct LinkDiscoveryConfig {
    /// 基础域名（当前页面域名）
    pub base_domain: String,
    /// 是否包含子域名
    pub include_subdomains: bool,
    /// 是否发现跨域链接
    pub include_cross_domain: bool,
    /// 最大发现深度
    pub max_depth: u32,
    /// 排除的路径模式
    pub exclude_patterns: Vec<String>,
    /// 包含的路径模式
    pub include_patterns: Vec<String>,
}

/// 发现的链接信息
#[derive(Debug, Clone)]
pub struct DiscoveredLink {
    /// 链接URL
    pub url: Url,
    /// 链接文本
    pub text: Option<String>,
    /// 链接标题
    pub title: Option<String>,
    /// 是否为同域链接
    pub is_same_domain: bool,
    /// 发现来源（页面URL）
    pub source_url: Url,
    /// 优先级评分
    pub priority_score: u32,
}

/// 链接分类
#[derive(Debug, Clone, PartialEq)]
pub enum LinkCategory {
    /// 导航链接（菜单、导航栏）
    Navigation,
    /// 内容链接（文章内、相关链接）
    Content,
    /// 资源链接（图片、CSS、JS）
    Resource,
    /// 外部链接
    External,
    /// 其他
    Other,
}

/// 链接发现引擎
pub struct LinkDiscoverer {
    config: LinkDiscoveryConfig,
    discovered_links: HashSet<Url>,
}

impl LinkDiscoverer {
    pub fn new(config: LinkDiscoveryConfig) -> Self {
        Self {
            config,
            discovered_links: HashSet::new(),
        }
    }

    /// 从DOM中发现所有相关链接
    pub fn discover_links_from_dom(&mut self, dom: &RcDom, base_url: &Url) -> Vec<DiscoveredLink> {
        let mut links = Vec::new();
        self.walk_and_discover(&dom.document, base_url, &mut links);
        self.categorize_and_prioritize(links)
    }

    /// 递归遍历DOM并发现链接
    fn walk_and_discover(&mut self, node: &Handle, base_url: &Url, links: &mut Vec<DiscoveredLink>) {
        match node.data {
            NodeData::Element { ref name, .. } => {
                match name.local.as_ref() {
                    "a" => self.extract_anchor_link(node, base_url, links),
                    "area" => self.extract_area_link(node, base_url, links),
                    "form" => self.extract_form_action(node, base_url, links),
                    _ => {}
                }

                // 继续遍历子节点
                for child in node.children.borrow().iter() {
                    self.walk_and_discover(child, base_url, links);
                }
            }
            _ => {}
        }
    }

    /// 对链接进行分类和优先级排序
    fn categorize_and_prioritize(&self, links: Vec<DiscoveredLink>) -> Vec<DiscoveredLink> {
        let mut categorized_links = links;
        
        // 根据链接文本、位置等因素计算优先级
        for link in &mut categorized_links {
            link.priority_score = self.calculate_priority(&link);
        }

        // 按优先级排序
        categorized_links.sort_by(|a, b| b.priority_score.cmp(&a.priority_score));
        categorized_links
    }

    /// 计算链接优先级
    fn calculate_priority(&self, link: &DiscoveredLink) -> u32 {
        let mut score = 0;

        // 同域链接优先级高
        if link.is_same_domain {
            score += 100;
        }

        // 有意义的链接文本增加优先级
        if let Some(ref text) = link.text {
            if text.len() > 5 && text.len() < 100 {
                score += 50;
            }
        }

        // 导航相关链接优先级高
        if let Some(ref text) = link.text {
            if text.contains("首页") || text.contains("导航") || text.contains("菜单") {
                score += 30;
            }
        }

        score
    }

    /// 检查链接是否应该被包含
    fn should_include_link(&self, url: &Url) -> bool {
        // 检查域名匹配
        if !self.is_target_domain(url) {
            return false;
        }

        // 检查排除模式
        for pattern in &self.config.exclude_patterns {
            if url.path().contains(pattern) {
                return false;
            }
        }

        // 检查包含模式
        if !self.config.include_patterns.is_empty() {
            return self.config.include_patterns.iter()
                .any(|pattern| url.path().contains(pattern));
        }

        true
    }

    /// 检查是否为目标域名
    fn is_target_domain(&self, url: &Url) -> bool {
        use crate::utils::url::domain_is_within_domain;
        
        if let Some(domain) = url.domain() {
            if self.config.include_subdomains {
                domain_is_within_domain(domain, &format!(".{}", self.config.base_domain))
            } else {
                domain == self.config.base_domain
            }
        } else {
            false
        }
    }
}
```

#### 2.2.2 批量翻译协调器 (`src/translation/batch_coordinator.rs`)

```rust
//! 批量翻译协调器
//! 
//! 协调链接发现、内容获取和翻译的整个流程

use std::collections::HashMap;
use url::Url;
use tokio::sync::mpsc;

/// 批量翻译配置
#[derive(Debug, Clone)]
pub struct BatchTranslationConfig {
    /// 最大并发数
    pub max_concurrent: usize,
    /// 单个页面超时时间(秒)
    pub page_timeout: u64,
    /// 翻译批次大小
    pub translation_batch_size: usize,
    /// 最大处理页面数
    pub max_pages: usize,
    /// 是否启用缓存
    pub enable_cache: bool,
}

/// 页面处理状态
#[derive(Debug, Clone)]
pub enum PageStatus {
    Pending,
    Fetching,
    Translating,
    Completed,
    Failed(String),
}

/// 批量翻译结果
#[derive(Debug)]
pub struct BatchTranslationResult {
    /// 成功处理的页面数
    pub success_count: usize,
    /// 失败的页面数
    pub failure_count: usize,
    /// 翻译的文本总数
    pub translated_text_count: usize,
    /// 处理时间(毫秒)
    pub processing_time_ms: u64,
    /// 详细结果
    pub page_results: HashMap<Url, PageTranslationResult>,
}

/// 单页翻译结果
#[derive(Debug)]
pub struct PageTranslationResult {
    /// 原始URL
    pub original_url: Url,
    /// 翻译后的HTML
    pub translated_html: String,
    /// 翻译的文本数量
    pub translated_count: usize,
    /// 处理状态
    pub status: PageStatus,
    /// 错误信息
    pub error: Option<String>,
}

/// 批量翻译协调器
pub struct BatchTranslationCoordinator {
    config: BatchTranslationConfig,
    translation_engine: Arc<TranslationEngine>,
    session: Arc<Session>,
}

impl BatchTranslationCoordinator {
    pub fn new(
        config: BatchTranslationConfig,
        translation_engine: Arc<TranslationEngine>,
        session: Arc<Session>,
    ) -> Self {
        Self {
            config,
            translation_engine,
            session,
        }
    }

    /// 执行批量翻译
    pub async fn execute_batch_translation(
        &self,
        base_url: &Url,
        discovered_links: Vec<DiscoveredLink>,
    ) -> Result<BatchTranslationResult, Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        let mut page_results = HashMap::new();
        
        // 限制处理页面数
        let links_to_process: Vec<_> = discovered_links
            .into_iter()
            .take(self.config.max_pages)
            .collect();

        // 创建信号量来控制并发
        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent));
        
        // 创建任务
        let mut tasks = Vec::new();
        for link in links_to_process {
            let sem = semaphore.clone();
            let engine = self.translation_engine.clone();
            let session = self.session.clone();
            let config = self.config.clone();
            
            let task = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                Self::process_single_page(link, engine, session, config).await
            });
            
            tasks.push(task);
        }

        // 等待所有任务完成
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut translated_text_count = 0;

        for task in tasks {
            match task.await {
                Ok(result) => {
                    match &result.status {
                        PageStatus::Completed => {
                            success_count += 1;
                            translated_text_count += result.translated_count;
                        }
                        PageStatus::Failed(_) => failure_count += 1,
                        _ => failure_count += 1,
                    }
                    page_results.insert(result.original_url.clone(), result);
                }
                Err(e) => {
                    failure_count += 1;
                    eprintln!("Task failed: {}", e);
                }
            }
        }

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(BatchTranslationResult {
            success_count,
            failure_count,
            translated_text_count,
            processing_time_ms,
            page_results,
        })
    }

    /// 处理单个页面
    async fn process_single_page(
        link: DiscoveredLink,
        translation_engine: Arc<TranslationEngine>,
        session: Arc<Session>,
        config: BatchTranslationConfig,
    ) -> PageTranslationResult {
        // 获取页面内容
        let html_content = match Self::fetch_page_content(&link.url, &session, config.page_timeout).await {
            Ok(content) => content,
            Err(e) => {
                return PageTranslationResult {
                    original_url: link.url,
                    translated_html: String::new(),
                    translated_count: 0,
                    status: PageStatus::Failed(e.to_string()),
                    error: Some(e.to_string()),
                };
            }
        };

        // 翻译页面内容
        match translation_engine.translate_html_content(&html_content, &link.url).await {
            Ok((translated_html, translated_count)) => PageTranslationResult {
                original_url: link.url,
                translated_html,
                translated_count,
                status: PageStatus::Completed,
                error: None,
            },
            Err(e) => PageTranslationResult {
                original_url: link.url,
                translated_html: String::new(),
                translated_count: 0,
                status: PageStatus::Failed(e.to_string()),
                error: Some(e.to_string()),
            },
        }
    }

    /// 获取页面内容
    async fn fetch_page_content(
        url: &Url,
        session: &Session,
        timeout_seconds: u64,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // 实现页面内容获取逻辑
        // 使用现有的Session系统进行HTTP请求
        todo!("Implement page content fetching")
    }
}
```

#### 2.2.3 JavaScript链接拦截注入器 (`src/parsers/link_interceptor.rs`)

```rust
//! JavaScript链接拦截注入器
//! 
//! 在HTML页面中注入JavaScript代码，拦截链接点击并重定向到翻译服务

/// JavaScript拦截器配置
#[derive(Debug, Clone)]
pub struct LinkInterceptorConfig {
    /// 翻译服务基础路径
    pub translation_base_path: String,
    /// 是否拦截外部链接
    pub intercept_external_links: bool,
    /// 是否拦截同域链接
    pub intercept_same_domain_links: bool,
    /// 是否显示加载指示器
    pub show_loading_indicator: bool,
}

/// 链接拦截注入器
pub struct LinkInterceptor {
    config: LinkInterceptorConfig,
}

impl LinkInterceptor {
    pub fn new(config: LinkInterceptorConfig) -> Self {
        Self { config }
    }

    /// 向HTML中注入链接拦截脚本
    pub fn inject_interceptor_script(&self, dom: &mut RcDom, base_url: &Url) {
        let script_content = self.generate_interceptor_script(base_url);
        self.inject_script_to_dom(dom, &script_content);
    }

    /// 生成拦截脚本内容
    fn generate_interceptor_script(&self, base_url: &Url) -> String {
        format!(
            r#"
            (function() {{
                'use strict';
                
                const TRANSLATION_BASE_PATH = '{}';
                const BASE_DOMAIN = '{}';
                const INTERCEPT_EXTERNAL = {};
                const INTERCEPT_SAME_DOMAIN = {};
                const SHOW_LOADING = {};
                
                // URL编码函数
                function encodeURL(url) {{
                    return encodeURIComponent(url);
                }}
                
                // 检查是否为同域链接
                function isSameDomain(url) {{
                    try {{
                        const linkDomain = new URL(url).hostname;
                        return linkDomain === BASE_DOMAIN || linkDomain.endsWith('.' + BASE_DOMAIN);
                    }} catch (e) {{
                        return false;
                    }}
                }}
                
                // 检查是否应该拦截链接
                function shouldInterceptLink(url) {{
                    // 跳过锚点链接
                    if (url.startsWith('#')) return false;
                    
                    // 跳过JavaScript链接
                    if (url.startsWith('javascript:')) return false;
                    
                    // 跳过邮件和电话链接
                    if (url.startsWith('mailto:') || url.startsWith('tel:')) return false;
                    
                    // 跳过数据URL
                    if (url.startsWith('data:') || url.startsWith('blob:')) return false;
                    
                    const sameDomain = isSameDomain(url);
                    
                    if (sameDomain && INTERCEPT_SAME_DOMAIN) return true;
                    if (!sameDomain && INTERCEPT_EXTERNAL) return true;
                    
                    return false;
                }}
                
                // 显示加载指示器
                function showLoadingIndicator() {{
                    if (!SHOW_LOADING) return;
                    
                    const indicator = document.createElement('div');
                    indicator.id = 'translation-loading-indicator';
                    indicator.style.cssText = `
                        position: fixed;
                        top: 50%;
                        left: 50%;
                        transform: translate(-50%, -50%);
                        background: rgba(0, 0, 0, 0.8);
                        color: white;
                        padding: 20px;
                        border-radius: 5px;
                        z-index: 10000;
                        font-family: Arial, sans-serif;
                    `;
                    indicator.textContent = '正在翻译页面...';
                    document.body.appendChild(indicator);
                }}
                
                // 拦截链接点击
                function interceptClick(event) {{
                    const link = event.target.closest('a');
                    if (!link || !link.href) return;
                    
                    const originalUrl = link.href;
                    
                    if (shouldInterceptLink(originalUrl)) {{
                        event.preventDefault();
                        
                        showLoadingIndicator();
                        
                        const translationUrl = TRANSLATION_BASE_PATH + encodeURL(originalUrl);
                        window.location.href = translationUrl;
                    }}
                }}
                
                // 拦截表单提交
                function interceptFormSubmit(event) {{
                    const form = event.target;
                    if (!form.action) return;
                    
                    const originalAction = form.action;
                    
                    if (shouldInterceptLink(originalAction)) {{
                        event.preventDefault();
                        
                        showLoadingIndicator();
                        
                        const translationUrl = TRANSLATION_BASE_PATH + encodeURL(originalAction);
                        form.action = translationUrl;
                        form.submit();
                    }}
                }}
                
                // 初始化事件监听器
                function initializeInterceptors() {{
                    // 使用事件委托拦截所有链接点击
                    document.addEventListener('click', interceptClick, true);
                    
                    // 拦截表单提交
                    document.addEventListener('submit', interceptFormSubmit, true);
                    
                    console.log('Link interceptor initialized for domain:', BASE_DOMAIN);
                }}
                
                // DOM准备就绪时初始化
                if (document.readyState === 'loading') {{
                    document.addEventListener('DOMContentLoaded', initializeInterceptors);
                }} else {{
                    initializeInterceptors();
                }}
            }})();
            "#,
            self.config.translation_base_path,
            base_url.domain().unwrap_or(""),
            self.config.intercept_external_links,
            self.config.intercept_same_domain_links,
            self.config.show_loading_indicator
        )
    }

    /// 向DOM中注入脚本
    fn inject_script_to_dom(&self, dom: &mut RcDom, script_content: &str) {
        use markup5ever::{LocalName, Namespace, QualName};
        use markup5ever_rcdom::{Node, NodeData};
        use std::rc::Rc;

        // 查找head元素，如果没有则创建
        let head_node = self.find_or_create_head(dom);
        
        // 创建script元素
        let script_name = QualName::new(None, Namespace::from("http://www.w3.org/1999/xhtml"), LocalName::from("script"));
        let script_node = Rc::new(Node {
            parent: Some(Rc::downgrade(&head_node)),
            children: RefCell::new(Vec::new()),
            data: NodeData::Element {
                name: script_name,
                attrs: RefCell::new(Vec::new()),
            },
        });
        
        // 创建脚本文本节点
        let text_node = Rc::new(Node {
            parent: Some(Rc::downgrade(&script_node)),
            children: RefCell::new(Vec::new()),
            data: NodeData::Text {
                contents: RefCell::new(script_content.to_string()),
            },
        });
        
        script_node.children.borrow_mut().push(text_node);
        head_node.children.borrow_mut().push(script_node);
    }

    /// 查找或创建head元素
    fn find_or_create_head(&self, dom: &mut RcDom) -> Handle {
        // 实现查找或创建head元素的逻辑
        todo!("Implement find_or_create_head")
    }
}
```

### 2.3 集成到现有系统

#### 2.3.1 扩展MonolithOptions

```rust
// 在src/core.rs中的MonolithOptions添加新字段
impl MonolithOptions {
    // 批量翻译相关选项
    #[cfg(feature = "translation")]
    pub enable_batch_translation: bool,
    #[cfg(feature = "translation")]
    pub batch_translation_config: Option<BatchTranslationConfig>,
    #[cfg(feature = "translation")]
    pub link_discovery_config: Option<LinkDiscoveryConfig>,
    #[cfg(feature = "translation")]
    pub inject_link_interceptor: bool,
}
```

#### 2.3.2 主处理流程集成

```rust
// 在src/core.rs的create_monolithic_document函数中集成
pub async fn create_monolithic_document_with_batch_translation(
    options: &MonolithOptions,
) -> Result<BatchTranslationResult, MonolithError> {
    // 1. 处理主页面
    let main_html = create_monolithic_document(options).await?;
    
    #[cfg(feature = "translation")]
    if options.enable_batch_translation {
        // 2. 发现相关链接
        let base_url = Url::parse(&options.target_url).map_err(|e| MonolithError::new(&e.to_string()))?;
        let discovery_config = options.link_discovery_config.clone().unwrap_or_default();
        
        let mut discoverer = LinkDiscoverer::new(discovery_config);
        let dom = html_to_dom(main_html.as_bytes(), "utf-8".to_string());
        let discovered_links = discoverer.discover_links_from_dom(&dom, &base_url);
        
        // 3. 执行批量翻译
        let batch_config = options.batch_translation_config.clone().unwrap_or_default();
        let coordinator = BatchTranslationCoordinator::new(
            batch_config,
            translation_engine,
            session,
        );
        
        let batch_result = coordinator.execute_batch_translation(&base_url, discovered_links).await?;
        
        return Ok(batch_result);
    }
    
    // 如果没有启用批量翻译，返回单页结果
    Ok(BatchTranslationResult::single_page(main_html))
}
```

## 3. 技术可行性评估

### 3.1 优势

1. **完整的基础设施**: 项目已有完善的翻译系统、DOM处理、网络请求等基础组件
2. **模块化设计**: 新功能可以作为独立模块集成，不影响现有功能
3. **性能优化**: 支持并发处理、缓存系统、智能过滤
4. **用户体验**: JavaScript拦截器提供无缝的浏览体验

### 3.2 技术挑战

1. **性能影响**: 批量翻译可能消耗大量系统资源
2. **网络负载**: 大量并发请求可能对目标网站造成压力
3. **缓存管理**: 需要高效的缓存策略避免重复翻译
4. **错误处理**: 需要优雅处理网络错误、翻译失败等情况

### 3.3 实现复杂度

- **链接发现**: 中等复杂度，需要DOM解析和URL处理
- **批量翻译**: 高复杂度，需要并发控制和资源管理
- **JavaScript注入**: 低复杂度，主要是文本处理和DOM操作

## 4. 实施建议

### 4.1 开发阶段

1. **阶段1**: 实现链接发现引擎
2. **阶段2**: 开发批量翻译协调器
3. **阶段3**: 集成JavaScript拦截器
4. **阶段4**: 性能优化和错误处理

### 4.2 配置示例

```rust
let options = MonolithOptions {
    target_url: "https://example.com".to_string(),
    enable_translation: true,
    enable_batch_translation: true,
    batch_translation_config: Some(BatchTranslationConfig {
        max_concurrent: 5,
        page_timeout: 30,
        translation_batch_size: 50,
        max_pages: 20,
        enable_cache: true,
    }),
    link_discovery_config: Some(LinkDiscoveryConfig {
        base_domain: "example.com".to_string(),
        include_subdomains: true,
        include_cross_domain: false,
        max_depth: 2,
        exclude_patterns: vec!["/admin".to_string(), "/api".to_string()],
        include_patterns: vec![],
    }),
    inject_link_interceptor: true,
    // ... 其他配置
};
```

### 4.3 使用示例

```bash
# CLI模式批量翻译
monolith --translate --batch-translation --max-pages 10 https://example.com

# Web服务器模式批量翻译
curl -X POST http://localhost:3000/api/batch-translate \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://example.com",
    "max_pages": 10,
    "include_subdomains": true
  }'
```

## 5. 总结

该设计方案在现有Monolith架构基础上，添加了批量翻译拦截链接的能力，具备以下特点：

1. **模块化**: 新功能作为可选特性，不影响现有代码
2. **可扩展**: 支持不同的链接发现策略和翻译配置
3. **高性能**: 并发处理、智能缓存、资源控制
4. **用户友好**: JavaScript拦截器提供无缝体验

通过这个设计，用户可以一次性获取并翻译当前域名下的相关页面，大大提升了翻译服务的实用性和用户体验。