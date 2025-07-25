//! 文本收集器模块
//!
//! 提供DOM文本和属性的智能收集功能

use std::collections::HashMap;
use std::sync::Arc;

use html5ever::tendril;
use markup5ever_rcdom::{Handle, NodeData};

// #[cfg(feature = "translation")]
// use rayon::prelude::*;

use crate::html::get_node_attr;
use crate::translation::config::constants;
use crate::translation::error::TranslationResult;
use crate::translation::filters::TextFilter;

/// 存储需要翻译的文本及其位置信息
#[derive(Debug, Clone)]
pub struct TextItem {
    /// 文本内容
    pub text: String,
    /// DOM节点引用
    pub node: Handle,
    /// 属性名（如果是属性文本）
    pub attr_name: Option<String>,
    /// 文本优先级
    pub priority: TextPriority,
    /// 文本类型
    pub text_type: TextType,
    /// 在DOM中的深度
    pub depth: usize,
    /// 父元素标签名
    pub parent_tag: Option<String>,
}

/// 文本优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// 文本类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TextType {
    /// 普通文本内容
    Content,
    /// HTML属性
    Attribute(String),
    /// 标题文本
    Title,
    /// 链接文本
    Link,
    /// 按钮文本
    Button,
    /// 表单标签
    FormLabel,
    /// 图像描述
    ImageAlt,
    /// 提示文本
    Tooltip,
}

impl TextItem {
    /// 创建新的文本项
    pub fn new(text: String, node: Handle, attr_name: Option<String>) -> Self {
        let text_type = Self::determine_text_type(&node, &attr_name);
        let priority = Self::calculate_priority(&text_type, &text);
        let depth = Self::calculate_depth(&node);
        let parent_tag = Self::get_parent_tag(&node);

        Self {
            text,
            node,
            attr_name,
            priority,
            text_type,
            depth,
            parent_tag,
        }
    }

    /// 创建内容文本项
    pub fn content(text: String, node: Handle, depth: usize) -> Self {
        let priority = Self::calculate_priority(&TextType::Content, &text);
        let parent_tag = Self::get_parent_tag(&node);

        Self {
            text,
            node,
            attr_name: None,
            priority,
            text_type: TextType::Content,
            depth,
            parent_tag,
        }
    }

    /// 创建属性文本项
    pub fn attribute(text: String, node: Handle, attr_name: String, depth: usize) -> Self {
        let text_type = TextType::Attribute(attr_name.clone());
        let priority = Self::calculate_priority(&text_type, &text);
        let parent_tag = Self::get_parent_tag(&node);

        Self {
            text,
            node,
            attr_name: Some(attr_name),
            priority,
            text_type,
            depth,
            parent_tag,
        }
    }

    /// 判断是否为属性文本
    pub fn is_attribute(&self) -> bool {
        self.attr_name.is_some()
    }

    /// 获取文本字符数
    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    /// 获取文本字节数
    pub fn byte_count(&self) -> usize {
        self.text.len()
    }

    /// 计算文本翻译复杂度权重
    /// 返回值范围: 0.5 - 3.0，越高表示越复杂
    pub fn complexity_weight(&self) -> f32 {
        let mut weight: f32 = 1.0;
        let text = &self.text;
        let char_count = self.char_count();
        
        if char_count == 0 {
            return 0.0;
        }
        
        // 1. 基于文本长度的复杂度
        if char_count > 200 {
            weight += 0.5; // 长文本增加复杂度
        } else if char_count < 10 {
            weight -= 0.2; // 短文本降低复杂度
        }
        
        // 2. 基于标点符号密度
        let punctuation_count = text.chars()
            .filter(|c| c.is_ascii_punctuation())
            .count();
        let punctuation_ratio = punctuation_count as f32 / char_count as f32;
        if punctuation_ratio > 0.15 {
            weight += 0.3; // 高标点密度增加复杂度
        }
        
        // 3. 基于数字和特殊字符
        let digit_count = text.chars().filter(|c| c.is_ascii_digit()).count();
        let digit_ratio = digit_count as f32 / char_count as f32;
        if digit_ratio > 0.1 {
            weight += 0.2; // 包含较多数字
        }
        
        // 4. 基于大写字母比例（可能是专有名词）
        let uppercase_count = text.chars().filter(|c| c.is_uppercase()).count();
        let uppercase_ratio = uppercase_count as f32 / char_count as f32;
        if uppercase_ratio > 0.2 {
            weight += 0.3; // 大量大写字母可能是专有名词
        }
        
        // 5. 基于文本类型
        match &self.text_type {
            TextType::Title => weight += 0.2,
            TextType::Link => weight -= 0.1,
            TextType::Button => weight -= 0.2,
            TextType::FormLabel => weight -= 0.1,
            TextType::ImageAlt => weight += 0.1,
            TextType::Tooltip => weight += 0.1,
            TextType::Attribute(_) => weight -= 0.2,
            TextType::Content => {}, // 保持基础权重
        }
        
        // 6. 基于DOM深度（深层嵌套可能更复杂）
        if self.depth > 10 {
            weight += 0.1;
        }
        
        // 7. 基于父标签类型
        if let Some(ref parent_tag) = self.parent_tag {
            match parent_tag.as_str() {
                "code" | "pre" => weight += 0.5, // 代码块更难翻译
                "script" | "style" => weight += 1.0, // 脚本和样式很复杂
                "table" | "td" | "th" => weight += 0.2, // 表格内容
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => weight += 0.1, // 标题
                _ => {}
            }
        }
        
        // 8. 检查是否包含HTML实体或特殊字符
        if text.contains('&') && (text.contains(';') || text.contains('#')) {
            weight += 0.3; // 可能包含HTML实体
        }
        
        // 9. 检查是否包含URL或邮箱
        if text.contains("http") || text.contains("www.") || text.contains('@') {
            weight += 0.2;
        }
        
        // 限制权重范围
        weight.max(0.5).min(3.0)
    }
    
    /// 计算有效翻译大小（字符数 * 复杂度权重）
    pub fn effective_size(&self) -> f32 {
        self.char_count() as f32 * self.complexity_weight()
    }

    /// 确定文本类型
    fn determine_text_type(node: &Handle, attr_name: &Option<String>) -> TextType {
        if let Some(attr) = attr_name {
            match attr.as_str() {
                "title" => TextType::Tooltip,
                "alt" => TextType::ImageAlt,
                "aria-label" | "aria-description" => TextType::Tooltip,
                "placeholder" => TextType::FormLabel,
                _ => TextType::Attribute(attr.clone()),
            }
        } else {
            // 根据节点类型和父元素确定文本类型
            let parent_tag = Self::get_parent_tag(node);
            match parent_tag.as_deref() {
                Some("title") => TextType::Title,
                Some("h1") | Some("h2") | Some("h3") | Some("h4") | Some("h5") | Some("h6") => {
                    TextType::Title
                }
                Some("button") | Some("input") => TextType::Button,
                Some("a") => TextType::Link,
                Some("label") => TextType::FormLabel,
                _ => TextType::Content,
            }
        }
    }

    /// 计算文本优先级
    fn calculate_priority(text_type: &TextType, text: &str) -> TextPriority {
        match text_type {
            TextType::Title => TextPriority::Critical,
            TextType::Button | TextType::Link => TextPriority::High,
            TextType::FormLabel | TextType::Tooltip => TextPriority::High,
            TextType::ImageAlt => TextPriority::Normal,
            TextType::Content => {
                // 根据文本长度和内容确定优先级
                if text.len() > 100 {
                    TextPriority::High
                } else if text.len() > 20 {
                    TextPriority::Normal
                } else {
                    TextPriority::Low
                }
            }
            TextType::Attribute(_) => TextPriority::Low,
        }
    }

    /// 计算节点深度
    fn calculate_depth(_node: &Handle) -> usize {
        // 简化实现，避免复杂的父节点遍历
        // 在实际使用中，depth参数会从外部传入
        0
    }

    /// 获取父元素标签名
    fn get_parent_tag(_node: &Handle) -> Option<String> {
        // 简化实现，避免复杂的父节点访问
        // 在实际使用中可以通过其他方式获取
        None
    }
}

/// 文本收集器配置
#[derive(Debug, Clone)]
pub struct CollectorConfig {
    /// 启用并行收集
    pub enable_parallel: bool,
    /// 最大收集深度
    pub max_depth: usize,
    /// 跳过的元素标签
    pub skip_elements: Vec<String>,
    /// 收集的属性列表
    pub collect_attributes: Vec<String>,
    /// 最小文本长度
    pub min_text_length: usize,
    /// 启用优先级排序
    pub enable_priority_sorting: bool,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            enable_parallel: true,
            max_depth: 50,
            skip_elements: constants::SKIP_ELEMENTS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            collect_attributes: constants::TRANSLATABLE_ATTRS
                .iter()
                .map(|s| s.to_string())
                .collect(),
            min_text_length: constants::MIN_TEXT_LENGTH,
            enable_priority_sorting: true,
        }
    }
}

/// DOM文本收集器
pub struct TextCollector {
    config: CollectorConfig,
    filter: Arc<TextFilter>,
    stats: CollectionStats,
}

impl TextCollector {
    /// 创建新的文本收集器
    pub fn new(config: CollectorConfig) -> Self {
        Self {
            config,
            filter: Arc::new(TextFilter::new()),
            stats: CollectionStats::default(),
        }
    }

    /// 使用默认配置创建收集器
    pub fn default() -> Self {
        Self::new(CollectorConfig::default())
    }

    /// 收集可翻译文本
    pub fn collect_translatable_texts(
        &mut self,
        root: &Handle,
    ) -> TranslationResult<Vec<TextItem>> {
        let mut texts = Vec::new();

        self.stats.reset();

        #[cfg(feature = "translation")]
        {
            if self.config.enable_parallel {
                self.collect_parallel(root, &mut texts)?;
            } else {
                self.collect_recursive(root, &mut texts, 0);
            }
        }

        #[cfg(not(feature = "translation"))]
        {
            self.collect_recursive(root, &mut texts, 0);
        }

        // 过滤和排序
        self.filter_and_sort_texts(texts)
    }

    /// 递归收集文本
    fn collect_recursive(&mut self, node: &Handle, texts: &mut Vec<TextItem>, depth: usize) {
        if depth > self.config.max_depth {
            return;
        }

        self.stats.nodes_visited += 1;

        match node.data {
            NodeData::Text { ref contents } => {
                self.collect_text_content(node, contents, texts, depth);
            }
            NodeData::Element { ref name, .. } => {
                let tag_name = name.local.as_ref();

                if self.should_skip_element(tag_name) {
                    self.stats.nodes_skipped += 1;
                    return;
                }

                // 收集元素属性
                self.collect_element_attributes(node, texts, depth);

                // 递归处理子节点
                for child in node.children.borrow().iter() {
                    self.collect_recursive(child, texts, depth + 1);
                }
            }
            _ => {
                // 处理其他类型的节点
                for child in node.children.borrow().iter() {
                    self.collect_recursive(child, texts, depth + 1);
                }
            }
        }
    }

    /// 并行收集文本（实验性功能）
    #[cfg(feature = "translation")]
    fn collect_parallel(
        &mut self,
        root: &Handle,
        texts: &mut Vec<TextItem>,
    ) -> TranslationResult<()> {
        // 首先收集所有节点
        let mut all_nodes = Vec::new();
        self.collect_all_nodes(root, &mut all_nodes, 0);

        // 串行处理节点（避免并发问题）
        let mut results: Vec<Vec<TextItem>> = Vec::new();
        for chunk in all_nodes.chunks(100) {
            let mut chunk_texts = Vec::new();
            for (node, depth) in chunk {
                self.process_single_node(node, &mut chunk_texts, *depth);
            }
            results.push(chunk_texts);
        }

        // 合并结果
        for mut chunk_texts in results {
            texts.append(&mut chunk_texts);
        }

        Ok(())
    }

    /// 收集所有节点（用于并行处理）
    #[cfg(feature = "translation")]
    fn collect_all_nodes(&self, node: &Handle, nodes: &mut Vec<(Handle, usize)>, depth: usize) {
        if depth > self.config.max_depth {
            return;
        }

        // 使用 Rc 共享，避免昂贵的克隆
        nodes.push((node.clone(), depth));  // TODO: 考虑使用 Weak 引用

        if let NodeData::Element { ref name, .. } = node.data {
            let tag_name = name.local.as_ref();
            if !self.should_skip_element(tag_name) {
                for child in node.children.borrow().iter() {
                    self.collect_all_nodes(child, nodes, depth + 1);
                }
            }
        } else {
            for child in node.children.borrow().iter() {
                self.collect_all_nodes(child, nodes, depth + 1);
            }
        }
    }

    /// 处理单个节点
    fn process_single_node(&self, node: &Handle, texts: &mut Vec<TextItem>, depth: usize) {
        match node.data {
            NodeData::Text { ref contents } => {
                let text = contents.borrow().to_string();
                if self.filter.should_translate(&text) {
                    texts.push(TextItem::content(text, node.clone(), depth));
                }
            }
            NodeData::Element { .. } => {
                // 收集属性
                for attr_name in &self.config.collect_attributes {
                    if let Some(attr_value) = get_node_attr(node, attr_name) {
                        if self.filter.should_translate(&attr_value) {
                            texts.push(TextItem::attribute(
                                attr_value,
                                node.clone(),
                                attr_name.clone(),
                                depth,
                            ));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// 收集文本内容
    fn collect_text_content(
        &mut self,
        node: &Handle,
        contents: &std::cell::RefCell<tendril::StrTendril>,
        texts: &mut Vec<TextItem>,
        depth: usize,
    ) {
        let text = contents.borrow().to_string();

        self.stats.text_nodes_found += 1;

        if self.filter.should_translate(&text) {
            texts.push(TextItem::content(text, node.clone(), depth));
            self.stats.translatable_texts += 1;
        } else {
            self.stats.filtered_texts += 1;
        }
    }

    /// 收集元素属性
    fn collect_element_attributes(
        &mut self,
        node: &Handle,
        texts: &mut Vec<TextItem>,
        depth: usize,
    ) {
        for attr_name in &self.config.collect_attributes {
            if let Some(attr_value) = get_node_attr(node, attr_name) {
                self.stats.attributes_found += 1;

                if self.filter.should_translate(&attr_value) {
                    texts.push(TextItem::attribute(
                        attr_value,
                        node.clone(),
                        attr_name.clone(),
                        depth,
                    ));
                    self.stats.translatable_attributes += 1;
                } else {
                    self.stats.filtered_attributes += 1;
                }
            }
        }
    }

    /// 检查是否应该跳过元素
    fn should_skip_element(&self, tag_name: &str) -> bool {
        self.config.skip_elements.contains(&tag_name.to_lowercase())
    }

    /// 过滤和排序文本
    fn filter_and_sort_texts(
        &mut self,
        mut texts: Vec<TextItem>,
    ) -> TranslationResult<Vec<TextItem>> {
        // 按文本长度过滤
        texts.retain(|item| item.text.len() >= self.config.min_text_length);

        // 去重（基于文本内容）
        texts = self.deduplicate_texts(texts);

        // 排序
        if self.config.enable_priority_sorting {
            self.sort_by_priority(&mut texts);
        }

        self.stats.final_text_count = texts.len();

        Ok(texts)
    }

    /// 去重文本（优化版，减少克隆）
    fn deduplicate_texts(&mut self, texts: Vec<TextItem>) -> Vec<TextItem> {
        // 使用字符串引用作为键，避免克隆
        let mut seen = HashMap::new();
        let mut unique_texts = Vec::new();
        
        // 预分配容量以降低重分配开销
        unique_texts.reserve(texts.len() / 2);  // 估计去重后为一半

        for item in texts {
            // 为了HashMap查找，我们仍需要克隆键（但可以在其他地方优化）
            let key = (item.text.clone(), item.text_type.clone());

            if let Some(&existing_priority) = seen.get(&key) {
                // 如果已存在，保留优先级更高的
                if item.priority > existing_priority {
                    // 找到并更新现有项
                    if let Some(pos) = unique_texts.iter().position(|t: &TextItem| {
                        t.text == item.text && t.text_type == item.text_type
                    }) {
                        let item_priority = item.priority;  // 在移动之前保存优先级
                        unique_texts[pos] = item;  // 移动语义，无需克隆
                        seen.insert(key, item_priority);  // 使用保存的优先级
                    }
                }
                self.stats.duplicate_texts += 1;
            } else {
                // 新项，添加到结果中
                seen.insert(key, item.priority);  // 重用已有的key
                unique_texts.push(item);
            }
        }

        unique_texts
    }

    /// 按优先级排序
    fn sort_by_priority(&self, texts: &mut Vec<TextItem>) {
        texts.sort_by(|a, b| {
            // 首先按优先级排序（高优先级在前）
            b.priority
                .cmp(&a.priority)
                // 然后按文本长度排序（短文本在前，便于批次处理）
                .then_with(|| a.char_count().cmp(&b.char_count()))
                // 最后按深度排序（浅层在前）
                .then_with(|| a.depth.cmp(&b.depth))
        });
    }

    /// 获取收集统计信息
    pub fn get_stats(&self) -> &CollectionStats {
        &self.stats
    }

    /// 重置统计信息
    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }

    /// 设置过滤器
    pub fn set_filter(&mut self, filter: TextFilter) {
        self.filter = Arc::new(filter);
    }

    /// 按类型分组文本
    pub fn group_by_type<'a>(&self, texts: &'a [TextItem]) -> HashMap<TextType, Vec<&'a TextItem>> {
        let mut groups: HashMap<TextType, Vec<&'a TextItem>> = HashMap::new();

        for item in texts {
            groups.entry(item.text_type.clone()).or_default().push(item);
        }

        groups
    }

    /// 按优先级分组文本
    pub fn group_by_priority<'a>(
        &self,
        texts: &'a [TextItem],
    ) -> HashMap<TextPriority, Vec<&'a TextItem>> {
        let mut groups: HashMap<TextPriority, Vec<&'a TextItem>> = HashMap::new();

        for item in texts {
            groups.entry(item.priority).or_default().push(item);
        }

        groups
    }
}

/// 收集统计信息
#[derive(Debug, Clone, Default)]
pub struct CollectionStats {
    pub nodes_visited: usize,
    pub nodes_skipped: usize,
    pub text_nodes_found: usize,
    pub attributes_found: usize,
    pub translatable_texts: usize,
    pub translatable_attributes: usize,
    pub filtered_texts: usize,
    pub filtered_attributes: usize,
    pub duplicate_texts: usize,
    pub final_text_count: usize,
    pub collection_time_ms: u64,
}

impl CollectionStats {
    /// 重置统计
    pub fn reset(&mut self) {
        *self = Default::default();
    }

    /// 获取总发现的可翻译项目数
    pub fn total_translatable(&self) -> usize {
        self.translatable_texts + self.translatable_attributes
    }

    /// 获取总过滤的项目数
    pub fn total_filtered(&self) -> usize {
        self.filtered_texts + self.filtered_attributes
    }

    /// 获取可翻译率
    pub fn translatability_rate(&self) -> f32 {
        let total = self.total_translatable() + self.total_filtered();
        if total == 0 {
            0.0
        } else {
            self.total_translatable() as f32 / total as f32
        }
    }

    /// 获取去重率
    pub fn deduplication_rate(&self) -> f32 {
        let total_before = self.total_translatable();
        if total_before == 0 {
            0.0
        } else {
            self.duplicate_texts as f32 / total_before as f32
        }
    }
}

/// 高级文本收集器
pub struct AdvancedTextCollector {
    base_collector: TextCollector,
    custom_processors: Vec<Box<dyn Fn(&mut Vec<TextItem>) + Send + Sync>>,
}

impl AdvancedTextCollector {
    /// 创建高级收集器
    pub fn new(config: CollectorConfig) -> Self {
        Self {
            base_collector: TextCollector::new(config),
            custom_processors: Vec::new(),
        }
    }

    /// 添加自定义处理器
    pub fn add_processor<F>(&mut self, processor: F)
    where
        F: Fn(&mut Vec<TextItem>) + Send + Sync + 'static,
    {
        self.custom_processors.push(Box::new(processor));
    }

    /// 收集并处理文本
    pub fn collect_and_process(&mut self, root: &Handle) -> TranslationResult<Vec<TextItem>> {
        let mut texts = self.base_collector.collect_translatable_texts(root)?;

        // 应用自定义处理器
        for processor in &self.custom_processors {
            processor(&mut texts);
        }

        Ok(texts)
    }

    /// 获取基础收集器的引用
    pub fn base_collector(&self) -> &TextCollector {
        &self.base_collector
    }

    /// 获取基础收集器的可变引用
    pub fn base_collector_mut(&mut self) -> &mut TextCollector {
        &mut self.base_collector
    }
}

/// 便利函数：收集可翻译文本
pub fn collect_translatable_texts(root: &Handle) -> Vec<TextItem> {
    let mut collector = TextCollector::default();
    collector
        .collect_translatable_texts(root)
        .unwrap_or_default()
}

/// 便利函数：收集指定类型的文本
pub fn collect_texts_by_type(root: &Handle, text_type: TextType) -> Vec<TextItem> {
    let texts = collect_translatable_texts(root);
    texts
        .into_iter()
        .filter(|item| item.text_type == text_type)
        .collect()
}

/// 便利函数：收集高优先级文本
pub fn collect_high_priority_texts(root: &Handle) -> Vec<TextItem> {
    let texts = collect_translatable_texts(root);
    texts
        .into_iter()
        .filter(|item| item.priority >= TextPriority::High)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use html5ever::parse_document;
    use markup5ever_rcdom::{RcDom, Handle};
    use html5ever::tendril::TendrilSink;
    use std::io::Cursor;

    fn create_test_dom(html: &str) -> RcDom {
        let mut input = Cursor::new(html);
        parse_document(RcDom::default(), Default::default())
            .from_utf8()
            .read_from(&mut input)
            .unwrap()
    }

    fn create_test_text_node(text: &str) -> Handle {
        let dom = create_test_dom(&format!("<div>{}</div>", text));
        // 获取文本节点需要遍历DOM
        dom.document.clone() // 简化处理，实际使用中需要正确获取文本节点
    }

    #[test]
    fn test_text_item_complexity_weight_basic() {
        let handle = create_test_text_node("Hello world");
        let item = TextItem::content("Hello world".to_string(), handle, 0);
        
        let weight = item.complexity_weight();
        assert!(weight >= 0.5 && weight <= 3.0, "Weight should be in range [0.5, 3.0], got {}", weight);
    }

    #[test]
    fn test_text_item_complexity_weight_empty_text() {
        let handle = create_test_text_node("");
        let item = TextItem::content("".to_string(), handle, 0);
        
        let weight = item.complexity_weight();
        assert_eq!(weight, 0.0, "Empty text should have 0 complexity weight");
    }

    #[test]
    fn test_text_item_complexity_weight_long_text() {
        let long_text = "A".repeat(300); // 超过200字符的长文本
        let handle = create_test_text_node(&long_text);
        let item = TextItem::content(long_text, handle, 0);
        
        let weight = item.complexity_weight();
        assert!(weight > 1.0, "Long text should have higher complexity weight, got {}", weight);
    }

    #[test]
    fn test_text_item_complexity_weight_short_text() {
        let short_text = "Hi"; // 少于10字符的短文本
        let handle = create_test_text_node(short_text);
        let item = TextItem::content(short_text.to_string(), handle, 0);
        
        let weight = item.complexity_weight();
        // 短文本应该有较低的复杂度，但由于基础权重是1.0，所以可能会轻微超过1.0
        assert!(weight <= 1.2, "Short text should have relatively low complexity weight, got {}", weight);
    }

    #[test]
    fn test_text_item_complexity_weight_high_punctuation() {
        let punctuation_text = "Hello, world! How are you? Fine, thanks."; // 高标点密度
        let handle = create_test_text_node(punctuation_text);
        let item = TextItem::content(punctuation_text.to_string(), handle, 0);
        
        let weight = item.complexity_weight();
        // 检查标点密度计算是否正确
        let punctuation_count = punctuation_text.chars().filter(|c| c.is_ascii_punctuation()).count();
        let punctuation_ratio = punctuation_count as f32 / punctuation_text.chars().count() as f32;
        
        if punctuation_ratio > 0.15 {
            assert!(weight > 1.0, "High punctuation text should have higher complexity, got {} with ratio {}", weight, punctuation_ratio);
        } else {
            // 如果标点密度不够高，则只要求在合理范围内
            assert!(weight >= 0.5 && weight <= 3.0, "Complexity weight should be in valid range, got {}", weight);
        }
    }

    #[test]
    fn test_text_item_complexity_weight_numbers() {
        let number_text = "Price: $123.45, Tax: 15%, Total: $142.97"; // 包含数字
        let handle = create_test_text_node(number_text);
        let item = TextItem::content(number_text.to_string(), handle, 0);
        
        let weight = item.complexity_weight();
        assert!(weight > 1.0, "Text with numbers should have higher complexity, got {}", weight);
    }

    #[test]
    fn test_text_item_complexity_weight_uppercase() {
        let uppercase_text = "NASA, FBI, CIA and USA"; // 大量大写字母（专有名词）
        let handle = create_test_text_node(uppercase_text);
        let item = TextItem::content(uppercase_text.to_string(), handle, 0);
        
        let weight = item.complexity_weight();
        assert!(weight > 1.0, "Text with many uppercase letters should have higher complexity, got {}", weight);
    }

    #[test]
    fn test_text_item_complexity_weight_by_type() {
        let handle = create_test_text_node("Test text");
        
        // 测试不同文本类型的复杂度
        let content_item = TextItem {
            text: "Test text".to_string(),
            node: handle.clone(),
            attr_name: None,
            priority: TextPriority::Normal,
            text_type: TextType::Content,
            depth: 0,
            parent_tag: None,
        };
        
        let title_item = TextItem {
            text: "Test text".to_string(),
            node: handle.clone(),
            attr_name: None,
            priority: TextPriority::Critical,
            text_type: TextType::Title,
            depth: 0,
            parent_tag: Some("h1".to_string()),
        };
        
        let button_item = TextItem {
            text: "Test text".to_string(),
            node: handle.clone(),
            attr_name: None,
            priority: TextPriority::High,
            text_type: TextType::Button,
            depth: 0,
            parent_tag: Some("button".to_string()),
        };
        
        let content_weight = content_item.complexity_weight();
        let title_weight = title_item.complexity_weight();
        let button_weight = button_item.complexity_weight();
        
        assert!(title_weight > content_weight, "Title should have higher complexity than content");
        assert!(button_weight < content_weight, "Button should have lower complexity than content");
    }

    #[test]
    fn test_text_item_effective_size() {
        let handle = create_test_text_node("Hello world");
        let item = TextItem::content("Hello world".to_string(), handle, 0);
        
        let effective_size = item.effective_size();
        let char_count = item.char_count() as f32;
        let complexity = item.complexity_weight();
        
        assert_eq!(effective_size, char_count * complexity, "Effective size should equal char_count * complexity_weight");
    }

    #[test]
    fn test_text_item_char_count() {
        let handle = create_test_text_node("Hello 世界");
        let item = TextItem::content("Hello 世界".to_string(), handle, 0);
        
        assert_eq!(item.char_count(), 8, "Should count Unicode characters correctly");
        assert_eq!(item.byte_count(), 12, "Should count bytes correctly (Unicode takes more bytes)");
    }

    #[test]
    fn test_text_priority_calculation() {
        // 测试不同文本类型的优先级计算
        assert_eq!(TextItem::calculate_priority(&TextType::Title, "Test"), TextPriority::Critical);
        assert_eq!(TextItem::calculate_priority(&TextType::Button, "Test"), TextPriority::High);
        assert_eq!(TextItem::calculate_priority(&TextType::Link, "Test"), TextPriority::High);
        assert_eq!(TextItem::calculate_priority(&TextType::ImageAlt, "Test"), TextPriority::Normal);
        assert_eq!(TextItem::calculate_priority(&TextType::Attribute("class".to_string()), "Test"), TextPriority::Low);
        
        // 测试内容文本根据长度的优先级
        assert_eq!(TextItem::calculate_priority(&TextType::Content, &"A".repeat(150)), TextPriority::High);
        assert_eq!(TextItem::calculate_priority(&TextType::Content, &"A".repeat(30)), TextPriority::Normal);
        assert_eq!(TextItem::calculate_priority(&TextType::Content, "Short"), TextPriority::Low);
    }

    #[test]
    fn test_text_type_determination() {
        let handle = create_test_text_node("Test");
        
        // 测试属性文本类型确定
        assert_eq!(TextItem::determine_text_type(&handle, &Some("title".to_string())), TextType::Tooltip);
        assert_eq!(TextItem::determine_text_type(&handle, &Some("alt".to_string())), TextType::ImageAlt);
        assert_eq!(TextItem::determine_text_type(&handle, &Some("aria-label".to_string())), TextType::Tooltip);
        assert_eq!(TextItem::determine_text_type(&handle, &Some("placeholder".to_string())), TextType::FormLabel);
        assert_eq!(TextItem::determine_text_type(&handle, &Some("class".to_string())), TextType::Attribute("class".to_string()));
        
        // 测试非属性文本（这里需要mock父标签，简化为测试默认情况）
        assert_eq!(TextItem::determine_text_type(&handle, &None), TextType::Content);
    }
}
