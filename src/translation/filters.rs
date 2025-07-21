//! 文本过滤器模块
//!
//! 提供智能文本过滤功能，判断文本是否需要翻译

use std::collections::HashSet;
use std::sync::OnceLock;

#[cfg(feature = "translation")]
use regex::Regex;

use crate::translation::config::constants;
// use crate::translation::error::{TranslationError, TranslationResult};

/// 文本过滤器
pub struct TextFilter {
    /// 功能性词汇集合
    functional_words: HashSet<String>,
    /// 缓存的正则表达式
    regex_cache: RegexCache,
}

/// 正则表达式缓存
struct RegexCache {
    #[cfg(feature = "translation")]
    url_regex: OnceLock<Regex>,
    #[cfg(feature = "translation")]
    email_regex: OnceLock<Regex>,
    #[cfg(feature = "translation")]
    css_selector_regex: OnceLock<Regex>,
    #[cfg(feature = "translation")]
    code_pattern_regex: OnceLock<Regex>,
}

impl Default for RegexCache {
    fn default() -> Self {
        Self {
            #[cfg(feature = "translation")]
            url_regex: OnceLock::new(),
            #[cfg(feature = "translation")]
            email_regex: OnceLock::new(),
            #[cfg(feature = "translation")]
            css_selector_regex: OnceLock::new(),
            #[cfg(feature = "translation")]
            code_pattern_regex: OnceLock::new(),
        }
    }
}

impl TextFilter {
    /// 创建新的文本过滤器
    pub fn new() -> Self {
        let functional_words = constants::FUNCTIONAL_WORDS
            .iter()
            .map(|&s| s.to_lowercase())
            .collect();

        Self {
            functional_words,
            regex_cache: RegexCache::default(),
        }
    }

    /// 判断文本是否需要翻译
    pub fn should_translate(&self, text: &str) -> bool {
        let trimmed = text.trim();

        // 基础长度检查
        if trimmed.len() < constants::MIN_TEXT_LENGTH {
            return false;
        }

        // 检查是否为非可翻译内容
        if self.is_non_translatable_content(trimmed) {
            return false;
        }

        // 检查是否包含字母字符
        if !trimmed.chars().any(|c| c.is_alphabetic()) {
            return false;
        }

        // 检查是否已经是中文
        if self.is_already_chinese(trimmed) {
            return false;
        }

        // 检查是否为功能性文本
        if trimmed.len() < constants::MIN_TRANSLATION_LENGTH && self.is_functional_text(trimmed) {
            return false;
        }

        true
    }

    /// 判断文本是否为非可翻译内容
    fn is_non_translatable_content(&self, text: &str) -> bool {
        // URL 检查
        if self.is_url(text) {
            return true;
        }

        // 邮箱检查
        if self.is_email(text) {
            return true;
        }

        // 代码模式检查
        if self.is_code_like(text) {
            return true;
        }

        // CSS选择器或类名检查
        if self.is_css_selector(text) {
            return true;
        }

        // 纯数字或纯符号检查
        if self.is_pure_symbols_or_numbers(text) {
            return true;
        }

        false
    }

    /// 检查是否为URL
    fn is_url(&self, text: &str) -> bool {
        // 简单的URL检查
        if text.starts_with("http://") || text.starts_with("https://") || text.starts_with("ftp://") {
            return true;
        }

        #[cfg(feature = "translation")]
        {
            let url_regex = self.regex_cache.url_regex.get_or_init(|| {
                Regex::new(r"^(https?|ftp)://[^\s]+$").unwrap_or_else(|_| Regex::new(r"").unwrap())
            });
            url_regex.is_match(text)
        }

        #[cfg(not(feature = "translation"))]
        false
    }

    /// 检查是否为邮箱
    fn is_email(&self, text: &str) -> bool {
        // 简单的邮箱检查
        if text.len() > 100 {
            return false;
        }

        if !text.contains('@') || !text.contains('.') {
            return false;
        }

        #[cfg(feature = "translation")]
        {
            let email_regex = self.regex_cache.email_regex.get_or_init(|| {
                Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
                    .unwrap_or_else(|_| Regex::new(r"").unwrap())
            });
            email_regex.is_match(text)
        }

        #[cfg(not(feature = "translation"))]
        true
    }

    /// 检查是否为代码模式
    fn is_code_like(&self, text: &str) -> bool {
        let special_chars = text
            .chars()
            .filter(|&c| {
                matches!(
                    c,
                    '{' | '}' | '[' | ']' | '(' | ')' | ';' | '=' | '<' | '>' | '/' | '\\'
                )
            })
            .count();

        special_chars as f32 > text.len() as f32 * constants::SPECIAL_CHAR_THRESHOLD
    }

    /// 检查是否为CSS选择器
    fn is_css_selector(&self, text: &str) -> bool {
        // 简单的CSS选择器检查
        if text.starts_with('.') || text.starts_with('#') || text.contains("::") {
            return true;
        }

        #[cfg(feature = "translation")]
        {
            let css_regex = self.regex_cache.css_selector_regex.get_or_init(|| {
                Regex::new(r"^[.#]?[a-zA-Z][a-zA-Z0-9_-]*(::[a-zA-Z][a-zA-Z0-9_-]*)?$")
                    .unwrap_or_else(|_| Regex::new(r"").unwrap())
            });
            css_regex.is_match(text)
        }

        #[cfg(not(feature = "translation"))]
        false
    }

    /// 检查是否为纯符号或数字
    fn is_pure_symbols_or_numbers(&self, text: &str) -> bool {
        text.chars()
            .all(|c| c.is_numeric() || c.is_ascii_punctuation() || c.is_whitespace())
    }

    /// 检查是否已经是中文
    fn is_already_chinese(&self, text: &str) -> bool {
        let chinese_chars = text
            .chars()
            .filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c))
            .count();

        if text.chars().count() == 0 {
            return false;
        }

        chinese_chars as f32 / text.chars().count() as f32 > constants::CHINESE_CHAR_THRESHOLD
    }

    /// 检查是否为功能性文本
    fn is_functional_text(&self, text: &str) -> bool {
        self.functional_words.contains(&text.to_lowercase())
    }

    /// 批量过滤文本
    pub fn filter_texts(&self, texts: Vec<String>) -> Vec<String> {
        texts
            .into_iter()
            .filter(|text| self.should_translate(text))
            .collect()
    }

    /// 计算文本的可翻译性评分 (0.0-1.0)
    pub fn translatability_score(&self, text: &str) -> f32 {
        let trimmed = text.trim();

        if trimmed.is_empty() {
            return 0.0;
        }

        let mut score = 1.0;

        // 长度因子
        if trimmed.len() < constants::MIN_TEXT_LENGTH {
            score *= 0.1;
        } else if trimmed.len() < constants::MIN_TRANSLATION_LENGTH {
            score *= 0.5;
        }

        // 字母字符比例
        let alpha_ratio = trimmed.chars().filter(|c| c.is_alphabetic()).count() as f32
            / trimmed.chars().count() as f32;
        score *= alpha_ratio;

        // 中文字符比例（降低评分）
        let chinese_ratio = trimmed
            .chars()
            .filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c))
            .count() as f32
            / trimmed.chars().count() as f32;
        score *= (1.0 - chinese_ratio).max(0.1);

        // 特殊字符比例（降低评分）
        let special_ratio = trimmed
            .chars()
            .filter(|&c| {
                matches!(
                    c,
                    '{' | '}' | '[' | ']' | '(' | ')' | ';' | '=' | '<' | '>' | '/' | '\\'
                )
            })
            .count() as f32
            / trimmed.chars().count() as f32;
        if special_ratio > constants::SPECIAL_CHAR_THRESHOLD {
            score *= 0.2;
        }

        // 功能性词汇检查
        if self.is_functional_text(trimmed) {
            score *= 0.1;
        }

        // URL和邮箱检查
        if self.is_url(trimmed) || self.is_email(trimmed) {
            score = 0.0;
        }

        score.clamp(0.0, 1.0)
    }

    /// 高级文本分析
    pub fn analyze_text(&self, text: &str) -> TextAnalysis {
        let trimmed = text.trim();
        
        TextAnalysis {
            original_text: text.to_string(),
            trimmed_text: trimmed.to_string(),
            should_translate: self.should_translate(text),
            translatability_score: self.translatability_score(text),
            char_count: trimmed.chars().count(),
            byte_count: trimmed.len(),
            is_chinese: self.is_already_chinese(trimmed),
            is_url: self.is_url(trimmed),
            is_email: self.is_email(trimmed),
            is_code_like: self.is_code_like(trimmed),
            is_functional: self.is_functional_text(trimmed),
            language_hint: self.detect_language_hint(trimmed),
        }
    }

    /// 检测语言提示
    fn detect_language_hint(&self, text: &str) -> LanguageHint {
        if text.is_empty() {
            return LanguageHint::Unknown;
        }

        let chinese_chars = text
            .chars()
            .filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c))
            .count();

        let japanese_chars = text
            .chars()
            .filter(|c| {
                ('\u{3040}'..='\u{309f}').contains(c) || ('\u{30a0}'..='\u{30ff}').contains(c)
            })
            .count();

        let korean_chars = text
            .chars()
            .filter(|c| ('\u{ac00}'..='\u{d7af}').contains(c))
            .count();

        let latin_chars = text
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .count();

        let total_chars = text.chars().count();

        if chinese_chars as f32 / total_chars as f32 > 0.3 {
            LanguageHint::Chinese
        } else if japanese_chars as f32 / total_chars as f32 > 0.1 {
            LanguageHint::Japanese
        } else if korean_chars as f32 / total_chars as f32 > 0.3 {
            LanguageHint::Korean
        } else if latin_chars as f32 / total_chars as f32 > 0.5 {
            LanguageHint::Latin
        } else {
            LanguageHint::Mixed
        }
    }
}

impl Default for TextFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// 文本分析结果
#[derive(Debug, Clone)]
pub struct TextAnalysis {
    pub original_text: String,
    pub trimmed_text: String,
    pub should_translate: bool,
    pub translatability_score: f32,
    pub char_count: usize,
    pub byte_count: usize,
    pub is_chinese: bool,
    pub is_url: bool,
    pub is_email: bool,
    pub is_code_like: bool,
    pub is_functional: bool,
    pub language_hint: LanguageHint,
}

/// 语言提示
#[derive(Debug, Clone, PartialEq)]
pub enum LanguageHint {
    Chinese,
    Japanese,
    Korean,
    Latin,
    Mixed,
    Unknown,
}

/// 文本过滤统计
#[derive(Debug, Clone, Default)]
pub struct FilterStats {
    pub total_texts: usize,
    pub translatable_texts: usize,
    pub filtered_out: usize,
    pub by_reason: std::collections::HashMap<FilterReason, usize>,
}

/// 过滤原因
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum FilterReason {
    TooShort,
    AlreadyChinese,
    NoAlphabetic,
    IsUrl,
    IsEmail,
    IsCodeLike,
    IsFunctional,
    IsCssSelector,
    PureSymbols,
}

impl FilterStats {
    /// 记录过滤结果
    pub fn record_filter(&mut self, should_translate: bool, reason: Option<FilterReason>) {
        self.total_texts += 1;
        
        if should_translate {
            self.translatable_texts += 1;
        } else {
            self.filtered_out += 1;
            if let Some(reason) = reason {
                *self.by_reason.entry(reason).or_insert(0) += 1;
            }
        }
    }

    /// 获取可翻译率
    pub fn translatability_rate(&self) -> f32 {
        if self.total_texts == 0 {
            0.0
        } else {
            self.translatable_texts as f32 / self.total_texts as f32
        }
    }

    /// 重置统计
    pub fn reset(&mut self) {
        *self = Default::default();
    }
}

/// 高级文本过滤器
pub struct AdvancedTextFilter {
    base_filter: TextFilter,
    stats: FilterStats,
    custom_rules: Vec<Box<dyn Fn(&str) -> bool + Send + Sync>>,
}

impl AdvancedTextFilter {
    /// 创建高级过滤器
    pub fn new() -> Self {
        Self {
            base_filter: TextFilter::new(),
            stats: FilterStats::default(),
            custom_rules: Vec::new(),
        }
    }

    /// 添加自定义过滤规则
    pub fn add_custom_rule<F>(&mut self, rule: F)
    where
        F: Fn(&str) -> bool + Send + Sync + 'static,
    {
        self.custom_rules.push(Box::new(rule));
    }

    /// 使用所有规则过滤文本
    pub fn should_translate(&mut self, text: &str) -> bool {
        let base_result = self.base_filter.should_translate(text);
        
        // 应用自定义规则
        let custom_result = if base_result {
            self.custom_rules.iter().all(|rule| rule(text))
        } else {
            false
        };

        // 记录统计
        let reason = if !base_result {
            self.determine_filter_reason(text)
        } else if !custom_result {
            Some(FilterReason::PureSymbols) // 用作自定义规则拒绝的占位符
        } else {
            None
        };

        self.stats.record_filter(custom_result, reason);

        custom_result
    }

    /// 确定过滤原因
    fn determine_filter_reason(&self, text: &str) -> Option<FilterReason> {
        let trimmed = text.trim();

        if trimmed.len() < constants::MIN_TEXT_LENGTH {
            Some(FilterReason::TooShort)
        } else if self.base_filter.is_already_chinese(trimmed) {
            Some(FilterReason::AlreadyChinese)
        } else if !trimmed.chars().any(|c| c.is_alphabetic()) {
            Some(FilterReason::NoAlphabetic)
        } else if self.base_filter.is_url(trimmed) {
            Some(FilterReason::IsUrl)
        } else if self.base_filter.is_email(trimmed) {
            Some(FilterReason::IsEmail)
        } else if self.base_filter.is_code_like(trimmed) {
            Some(FilterReason::IsCodeLike)
        } else if self.base_filter.is_functional_text(trimmed) {
            Some(FilterReason::IsFunctional)
        } else if self.base_filter.is_css_selector(trimmed) {
            Some(FilterReason::IsCssSelector)
        } else if self.base_filter.is_pure_symbols_or_numbers(trimmed) {
            Some(FilterReason::PureSymbols)
        } else {
            None
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> &FilterStats {
        &self.stats
    }

    /// 重置统计
    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }
}

impl Default for AdvancedTextFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// 便利函数：检查文本是否应该翻译
pub fn should_translate(text: &str) -> bool {
    let filter = TextFilter::new();
    filter.should_translate(text)
}

/// 便利函数：批量过滤文本
pub fn filter_translatable_texts(texts: Vec<String>) -> Vec<String> {
    let filter = TextFilter::new();
    filter.filter_texts(texts)
}

/// 便利函数：获取文本分析
pub fn analyze_text(text: &str) -> TextAnalysis {
    let filter = TextFilter::new();
    filter.analyze_text(text)
}