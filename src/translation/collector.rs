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

        nodes.push((node.clone(), depth));

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

    /// 去重文本
    fn deduplicate_texts(&mut self, texts: Vec<TextItem>) -> Vec<TextItem> {
        let mut seen = HashMap::new();
        let mut unique_texts = Vec::new();

        for item in texts {
            let key = (item.text.clone(), item.text_type.clone());

            if let Some(existing) = seen.get(&key) {
                // 如果已存在，保留优先级更高的
                if item.priority > *existing {
                    // 替换现有项
                    if let Some(pos) = unique_texts.iter().position(|t: &TextItem| {
                        t.text == item.text && t.text_type == item.text_type
                    }) {
                        unique_texts[pos] = item.clone();
                        seen.insert(key, item.priority);
                    }
                }
                self.stats.duplicate_texts += 1;
            } else {
                seen.insert(key, item.priority);
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
