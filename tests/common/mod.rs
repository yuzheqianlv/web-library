// 集成测试公共模块
//
// 提供测试辅助工具和共享功能

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use monolith::translation::config::{TranslationConfig, constants};
use monolith::translation::storage::cache::{CacheConfig, CacheManager, CacheKey, CacheEntry};
use monolith::translation::pipeline::collector::{TextCollector, CollectorConfig, TextItem, TextPriority, TextType};
use monolith::translation::pipeline::batch::{BatchManager, BatchManagerConfig};
use monolith::translation::pipeline::filters::TextFilter;

use html5ever::parse_document;
use markup5ever_rcdom::{RcDom, Handle};
use html5ever::tendril::TendrilSink;
use std::io::Cursor;

/// 简化的批次配置（用于测试）
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub max_batch_size: usize,
    pub max_effective_size: f32,
    pub min_batch_size: usize,
    pub batch_timeout: Duration,
    pub max_concurrent_batches: usize,
    pub enable_smart_batching: bool,
    pub enable_priority_queue: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            max_effective_size: 200.0,
            min_batch_size: 10,
            batch_timeout: Duration::from_secs(10),
            max_concurrent_batches: 4,
            enable_smart_batching: true,
            enable_priority_queue: true,
        }
    }
}

/// 测试配置构建器
pub struct TestConfigBuilder {
    translation_config: TranslationConfig,
    cache_config: CacheConfig,
    collector_config: CollectorConfig,
    batch_config: BatchManagerConfig,
}

impl TestConfigBuilder {
    pub fn new() -> Self {
        Self {
            translation_config: TranslationConfig::default(),
            cache_config: CacheConfig::default(),
            collector_config: CollectorConfig::default(),
            batch_config: BatchManagerConfig::default(),
        }
    }

    pub fn with_fast_cache(mut self) -> Self {
        self.cache_config = CacheConfig {
            enable_local_cache: true,
            local_cache_size: 100,
            default_ttl: Duration::from_secs(300),
            enable_warmup: false,
            cleanup_interval: Duration::from_secs(60),
        };
        self
    }

    pub fn with_small_batches(mut self) -> Self {
        self.batch_config = BatchManagerConfig {
            max_batch_size: 50,
            min_batch_chars: 5,
            max_effective_size: 100.0,
            min_effective_size: 5.0,
            batch_delay: Duration::from_millis(100),
            small_batch_threshold: 10,
            enable_priority_sorting: true,
            enable_batch_merging: true,
            enable_smart_grouping: true,
            enable_smart_sizing: true,
            max_concurrent_batches: 2,
            batch_timeout: Duration::from_secs(5),
        };
        self
    }

    pub fn with_simple_collection(mut self) -> Self {
        self.collector_config = CollectorConfig {
            enable_parallel: false,
            max_depth: 20,
            skip_elements: vec!["script".to_string(), "style".to_string()],
            collect_attributes: vec!["title".to_string(), "alt".to_string()],
            min_text_length: constants::MIN_TEXT_LENGTH,
            enable_priority_sorting: true,
        };
        self
    }

    pub fn build(self) -> TestConfiguration {
        TestConfiguration {
            translation_config: self.translation_config,
            cache_config: self.cache_config,
            collector_config: self.collector_config,
            batch_config: self.batch_config,
        }
    }
}

/// 测试配置
pub struct TestConfiguration {
    pub translation_config: TranslationConfig,
    pub cache_config: CacheConfig,
    pub collector_config: CollectorConfig,
    pub batch_config: BatchManagerConfig,
}

/// 测试环境
pub struct TestEnvironment {
    pub config: TestConfiguration,
    pub cache_manager: CacheManager,
    pub text_collector: TextCollector,
    pub batch_manager: BatchManager,
    pub text_filter: TextFilter,
}

impl TestEnvironment {
    pub fn new(config: TestConfiguration) -> Self {
        let cache_manager = CacheManager::new(config.cache_config.clone());
        let text_collector = TextCollector::new(config.collector_config.clone());
        let batch_manager = BatchManager::new(config.batch_config.clone());
        let text_filter = TextFilter::new();

        Self {
            config,
            cache_manager,
            text_collector,
            batch_manager,
            text_filter,
        }
    }

    pub fn default() -> Self {
        let config = TestConfigBuilder::new()
            .with_fast_cache()
            .with_small_batches()
            .with_simple_collection()
            .build();
        
        Self::new(config)
    }
}

/// HTML测试工具
pub struct HtmlTestHelper;

impl HtmlTestHelper {
    /// 创建测试用的DOM结构
    pub fn create_test_dom(html: &str) -> RcDom {
        let mut input = Cursor::new(html);
        parse_document(RcDom::default(), Default::default())
            .from_utf8()
            .read_from(&mut input)
            .unwrap()
    }

    /// 创建简单的英文HTML页面
    pub fn create_simple_english_page() -> String {
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <title>Test Page</title>
    <meta charset="UTF-8">
</head>
<body>
    <h1>Welcome to Test</h1>
    <p>This is a test paragraph with some <strong>important</strong> text.</p>
    <div>
        <p>Another paragraph in a div.</p>
        <a href="https://example.com" title="Example Link">Visit Example</a>
    </div>
    <button>Click Me</button>
    <input type="text" placeholder="Enter your name">
    <img src="test.jpg" alt="Test Image">
</body>
</html>"#.to_string()
    }

    /// 创建包含中文的混合HTML页面
    pub fn create_mixed_language_page() -> String {
        r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <title>测试页面 - Test Page</title>
    <meta charset="UTF-8">
</head>
<body>
    <h1>欢迎来到测试页面</h1>
    <p>This is an English paragraph.</p>
    <div>
        <p>这是一个中文段落，包含一些<strong>重要</strong>内容。</p>
        <a href="https://example.com" title="示例链接">访问示例</a>
    </div>
    <button>点击我</button>
    <input type="text" placeholder="请输入您的姓名">
    <img src="test.jpg" alt="测试图片">
    <p>Another English paragraph for testing.</p>
</body>
</html>"#.to_string()
    }

    /// 创建复杂的HTML页面（用于性能测试）
    pub fn create_complex_page() -> String {
        let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <title>Complex Test Page</title>
    <meta charset="UTF-8">
</head>
<body>
    <header>
        <h1>Complex Website</h1>
        <nav>
            <ul>"#);

        // 添加导航链接
        for i in 1..=10 {
            html.push_str(&format!(
                r#"<li><a href="/page{}" title="Page {}">Page {}</a></li>"#,
                i, i, i
            ));
        }

        html.push_str(r#"
            </ul>
        </nav>
    </header>
    <main>
        <article>
            <h2>Main Article</h2>"#);

        // 添加多个段落
        for i in 1..=20 {
            html.push_str(&format!(
                r#"<p>This is paragraph number {}. It contains some meaningful text that should be translated. The content includes various <strong>important</strong> keywords and <em>emphasized</em> phrases.</p>"#,
                i
            ));
        }

        html.push_str(r#"
        </article>
        <aside>
            <h3>Sidebar Content</h3>
            <div>
                <h4>Related Links</h4>
                <ul>"#);

        // 添加侧边栏链接
        for i in 1..=15 {
            html.push_str(&format!(
                r#"<li><a href="/related{}" title="Related Article {}">Related Article {}</a></li>"#,
                i, i, i
            ));
        }

        html.push_str(r#"
                </ul>
            </div>
        </aside>
    </main>
    <footer>
        <p>Copyright 2024. All rights reserved.</p>
        <div>
            <button>Subscribe</button>
            <input type="email" placeholder="Enter your email">
        </div>
    </footer>
</body>
</html>"#);

        html
    }
}

/// 测试数据生成器
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// 生成测试用的TextItem列表
    pub fn create_test_text_items(count: usize) -> Vec<TextItem> {
        let mut items = Vec::new();
        let dom = HtmlTestHelper::create_test_dom("<div>test</div>");
        let handle = dom.document.clone();

        for i in 0..count {
            let text = format!("Test text item number {}", i + 1);
            let priority = match i % 4 {
                0 => TextPriority::Critical,
                1 => TextPriority::High,
                2 => TextPriority::Normal,
                _ => TextPriority::Low,
            };
            let text_type = match i % 3 {
                0 => TextType::Content,
                1 => TextType::Title,
                _ => TextType::Link,
            };

            let item = TextItem {
                text,
                node: handle.clone(),
                attr_name: None,
                priority,
                text_type,
                depth: i % 10,
                parent_tag: Some(format!("div")),
            };
            items.push(item);
        }

        items
    }

    /// 生成测试用的缓存条目
    pub fn create_test_cache_entries(count: usize) -> Vec<(CacheKey, CacheEntry)> {
        let mut entries = Vec::new();

        for i in 0..count {
            let key = CacheKey::new(
                format!("Test text {}", i + 1),
                "en".to_string(),
                "zh".to_string(),
            );

            let entry = CacheEntry::new(
                format!("Test text {}", i + 1),
                format!("测试文本 {}", i + 1),
                Some(Duration::from_secs(3600)),
            );

            entries.push((key, entry));
        }

        entries
    }

    /// 生成不同优先级的文本项
    pub fn create_priority_mixed_items() -> Vec<TextItem> {
        let dom = HtmlTestHelper::create_test_dom("<div>test</div>");
        let handle = dom.document.clone();

        vec![
            TextItem {
                text: "Critical title text".to_string(),
                node: handle.clone(),
                attr_name: None,
                priority: TextPriority::Critical,
                text_type: TextType::Title,
                depth: 1,
                parent_tag: Some("h1".to_string()),
            },
            TextItem {
                text: "High priority button".to_string(),
                node: handle.clone(),
                attr_name: None,
                priority: TextPriority::High,
                text_type: TextType::Button,
                depth: 2,
                parent_tag: Some("button".to_string()),
            },
            TextItem {
                text: "Normal content paragraph with some longer text to test batching".to_string(),
                node: handle.clone(),
                attr_name: None,
                priority: TextPriority::Normal,
                text_type: TextType::Content,
                depth: 3,
                parent_tag: Some("p".to_string()),
            },
            TextItem {
                text: "Low priority attribute".to_string(),
                node: handle.clone(),
                attr_name: Some("title".to_string()),
                priority: TextPriority::Low,
                text_type: TextType::Attribute("title".to_string()),
                depth: 2,
                parent_tag: Some("div".to_string()),
            },
        ]
    }
}

/// 性能测试辅助工具
pub struct PerformanceHelper;

impl PerformanceHelper {
    /// 测量执行时间
    pub fn measure_time<F, R>(f: F) -> (R, Duration)
    where
        F: FnOnce() -> R,
    {
        let start = std::time::Instant::now();
        let result = f();
        let duration = start.elapsed();
        (result, duration)
    }

    /// 测量异步执行时间
    pub async fn measure_async_time<F, Fut, R>(f: F) -> (R, Duration)
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let start = std::time::Instant::now();
        let result = f().await;
        let duration = start.elapsed();
        (result, duration)
    }

    /// 创建性能断言
    pub fn assert_performance<F, R>(
        f: F,
        max_duration: Duration,
        description: &str,
    ) -> R
    where
        F: FnOnce() -> R,
    {
        let (result, duration) = Self::measure_time(f);
        assert!(
            duration <= max_duration,
            "{} took {:?}, expected <= {:?}",
            description, duration, max_duration
        );
        result
    }

    /// 创建异步性能断言
    pub async fn assert_async_performance<F, Fut, R>(
        f: F,
        max_duration: Duration,
        description: &str,
    ) -> R
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let (result, duration) = Self::measure_async_time(f).await;
        assert!(
            duration <= max_duration,
            "{} took {:?}, expected <= {:?}",
            description, duration, max_duration
        );
        result
    }
}

/// 断言辅助工具
pub struct AssertionHelper;

impl AssertionHelper {
    /// 断言集合包含预期的项目
    pub fn assert_contains_items<T>(actual: &[T], expected: &[T], description: &str)
    where
        T: PartialEq + std::fmt::Debug,
    {
        for expected_item in expected {
            assert!(
                actual.contains(expected_item),
                "{}: Expected to find {:?} in {:?}",
                description, expected_item, actual
            );
        }
    }

    /// 断言集合大小在预期范围内
    pub fn assert_size_in_range<T>(
        collection: &[T],
        min_size: usize,
        max_size: usize,
        description: &str,
    ) {
        let size = collection.len();
        assert!(
            size >= min_size && size <= max_size,
            "{}: Size {} is not in range [{}, {}]",
            description, size, min_size, max_size
        );
    }

    /// 断言翻译结果的质量
    pub fn assert_translation_quality(
        original: &str,
        translated: &str,
        min_length_ratio: f32,
        description: &str,
    ) {
        assert!(!translated.is_empty(), "{}: Translation should not be empty", description);
        
        let original_len = original.chars().count() as f32;
        let translated_len = translated.chars().count() as f32;
        let ratio = translated_len / original_len;
        
        assert!(
            ratio >= min_length_ratio,
            "{}: Translation length ratio {} is below minimum {}",
            description, ratio, min_length_ratio
        );
    }
}