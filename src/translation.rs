//! 翻译功能模块
//!
//! 集成 markdown-translator 库，为 monolith 提供网页内容翻译功能。

use crate::core::MonolithError;
use crate::html::get_node_attr;
use markup5ever_rcdom::{Handle, NodeData, RcDom};

#[cfg(feature = "translation")]
use markdown_translator::{TranslationConfig, TranslationService};

/// 存储需要翻译的文本及其位置信息
#[cfg(feature = "translation")]
struct TextToTranslate {
    text: String,
    node: Handle,
    attr_name: Option<String>, // 如果是属性，存储属性名
}

/// 翻译DOM内容 - 高性能批量翻译
///
/// # 参数
///
/// * `dom` - 要翻译的DOM树
/// * `target_lang` - 目标语言代码（如 "zh-CN", "en", "ja" 等）
/// * `api_url` - 翻译API地址（可选，默认使用本地DeepLX）
///
/// # 返回
///
/// * `Ok(RcDom)` - 翻译后的DOM树
/// * `Err(MonolithError)` - 翻译过程中的错误
#[cfg(feature = "translation")]
pub async fn translate_dom_content(
    dom: RcDom,
    target_lang: &str,
    api_url: Option<&str>,
) -> Result<RcDom, MonolithError> {
    // 配置翻译服务 - 高性能参数
    let config = TranslationConfig {
        enabled: true,
        source_lang: "auto".to_string(),
        target_lang: target_lang.to_string(),
        deeplx_api_url: api_url.unwrap_or("https://deepl3.fileaiwork.online/dptrans?token=ej0ab47388ed86e843de9f499e52e6e664ae1m491cad7bf1.bIrYaAAAAAA=.b9c326068ac3c37ff36b8fea77867db51ddf235150945d7ad43472d68581e6c4pd14&newllm=1").to_string(),
        max_requests_per_second: 5.0, // 提高请求频率
        max_text_length: 10000, // 增加单次翻译长度
        max_paragraphs_per_request: 100, // 更大批次处理
    };

    // 创建翻译服务
    let translator = TranslationService::new(config);

    // 第一步：收集所有需要翻译的文本
    let mut texts_to_translate = Vec::new();
    collect_translatable_texts(&dom.document, &mut texts_to_translate);

    if texts_to_translate.is_empty() {
        println!("没有找到需要翻译的文本");
        return Ok(dom);
    }

    println!("收集到 {} 个待翻译文本", texts_to_translate.len());

    // 第二步：智能批量翻译文本
    batch_translate_texts_optimized(&translator, &mut texts_to_translate).await?;

    Ok(dom)
}

/// 收集所有需要翻译的文本
#[cfg(feature = "translation")]
fn collect_translatable_texts(node: &Handle, texts: &mut Vec<TextToTranslate>) {
    match node.data {
        NodeData::Text { ref contents } => {
            let text = contents.borrow().to_string();
            if should_translate_for_collection(&text) {
                texts.push(TextToTranslate {
                    text,
                    node: node.clone(),
                    attr_name: None,
                });
            }
        }
        NodeData::Element { ref name, .. } => {
            // 收集元素属性中的文本
            collect_element_attributes(node, texts);

            // 跳过不需要翻译的元素
            let tag_name = name.local.as_ref();
            if should_skip_element(tag_name) {
                return;
            }

            // 递归处理子节点
            for child in node.children.borrow().iter() {
                collect_translatable_texts(child, texts);
            }
        }
        _ => {
            // 递归处理其他类型节点的子节点
            for child in node.children.borrow().iter() {
                collect_translatable_texts(child, texts);
            }
        }
    }
}

/// 收集元素属性中需要翻译的文本
#[cfg(feature = "translation")]
fn collect_element_attributes(node: &Handle, texts: &mut Vec<TextToTranslate>) {
    let translatable_attrs = ["title", "alt", "placeholder", "aria-label", "aria-description"];
    
    for attr_name in &translatable_attrs {
        if let Some(attr_value) = get_node_attr(node, attr_name) {
            if should_translate_for_collection(&attr_value) {
                texts.push(TextToTranslate {
                    text: attr_value,
                    node: node.clone(),
                    attr_name: Some(attr_name.to_string()),
                });
            }
        }
    }
}

/// 智能批量翻译文本 - 优化版本
#[cfg(feature = "translation")]
async fn batch_translate_texts_optimized(
    translator: &TranslationService,
    texts: &mut Vec<TextToTranslate>,
) -> Result<(), MonolithError> {
    let max_batch_size = 9000; // 增加批次大小
    let mut batches = Vec::new();
    let mut current_batch = Vec::new();
    let mut current_size = 0;

    // 按文本长度预排序，短文本优先批处理
    texts.sort_by_key(|item| item.text.len());

    for text_item in texts.iter() {
        let text_len = text_item.text.len();
        
        // 超大文本单独处理
        if text_len > max_batch_size {
            if !current_batch.is_empty() {
                batches.push(std::mem::take(&mut current_batch));
                current_size = 0;
            }
            batches.push(vec![text_item]);
            continue;
        }

        // 检查是否可以加入当前批次
        if current_size + text_len + 1 > max_batch_size && !current_batch.is_empty() {
            batches.push(std::mem::take(&mut current_batch));
            current_size = 0;
        }

        current_batch.push(text_item);
        current_size += text_len + 1; // +1 为换行符
    }

    // 添加最后一个批次
    if !current_batch.is_empty() {
        batches.push(current_batch);
    }

    let batch_count = batches.len();
    println!("创建了 {} 个批次进行翻译", batch_count);

    // 串行处理批次 (避免并发复杂性)
    for (i, batch) in batches.into_iter().enumerate() {
        println!("开始处理批次 {}/{}", i + 1, batch_count);
        if let Err(e) = process_batch_optimized(translator, &batch, i + 1).await {
            eprintln!("批次处理失败: {}", e);
        }
        
        // 批次间稍作休息，避免API限制
        if i < batch_count - 1 {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    Ok(())
}

/// 优化的批次处理函数
#[cfg(feature = "translation")]
async fn process_batch_optimized(
    translator: &TranslationService,
    batch: &[&TextToTranslate],
    batch_id: usize,
) -> Result<(), MonolithError> {
    let start_time = std::time::Instant::now();
    
    // 如果批次很小，直接单独翻译
    if batch.len() <= 2 {
        for (i, text_item) in batch.iter().enumerate() {
            print!("批次{}-{}/{}: ", batch_id, i + 1, batch.len());
            match translator.translate(&text_item.text).await {
                Ok(translated) => {
                    apply_translation(text_item, &translated.trim());
                    println!("✓ {}", text_item.text.chars().take(25).collect::<String>());
                }
                Err(e) => {
                    println!("✗ 失败: {} - {}", e, text_item.text.chars().take(25).collect::<String>());
                }
            }
        }
        return Ok(());
    }

    // 批量处理
    let combined_text: String = batch
        .iter()
        .map(|item| item.text.trim())
        .collect::<Vec<_>>()
        .join("\n");

    print!("批次{}: 翻译{}个文本({} 字符)... ", batch_id, batch.len(), combined_text.len());

    match translator.translate(&combined_text).await {
        Ok(translated) => {
            let translated_lines: Vec<&str> = translated.lines().collect();
            
            if translated_lines.len() == batch.len() {
                // 成功批量应用
                for (i, text_item) in batch.iter().enumerate() {
                    if let Some(translated_line) = translated_lines.get(i) {
                        let cleaned_text = translated_line.trim();
                        if !cleaned_text.is_empty() {
                            apply_translation(text_item, cleaned_text);
                        }
                    }
                }
                println!("✓ 用时 {:?}", start_time.elapsed());
            } else {
                println!("行数不匹配，切换单独翻译");
                // 回退到单独翻译
                for text_item in batch.iter() {
                    if let Ok(individual_translated) = translator.translate(&text_item.text).await {
                        apply_translation(text_item, &individual_translated.trim());
                    }
                }
            }
        }
        Err(e) => {
            println!("✗ 批量失败: {}", e);
            // 回退到单独翻译
            for text_item in batch.iter() {
                if let Ok(individual_translated) = translator.translate(&text_item.text).await {
                    apply_translation(text_item, &individual_translated.trim());
                }
            }
        }
    }

    Ok(())
}

/// 处理一个批次的翻译
#[cfg(feature = "translation")]
async fn process_batch(
    translator: &TranslationService,
    batch: &[&TextToTranslate],
) -> Result<(), MonolithError> {
    // 每段文本单独一行，不使用分隔符
    let combined_text: String = batch
        .iter()
        .map(|item| item.text.trim())
        .collect::<Vec<_>>()
        .join("\n");

    println!("批量翻译 {} 个文本，总长度: {} 字符", batch.len(), combined_text.len());
    println!("合并后的文本前100字符: {}", &combined_text.chars().take(100).collect::<String>());

    // 翻译合并后的文本
    match translator.translate(&combined_text).await {
        Ok(translated) => {
            println!("翻译成功，结果长度: {} 字符", translated.len());
            
            // 按行分割翻译结果
            let translated_lines: Vec<&str> = translated.lines().collect();
            
            println!("分割后得到 {} 行，期望 {} 个", translated_lines.len(), batch.len());
            
            if translated_lines.len() == batch.len() {
                // 应用翻译结果
                for (i, text_item) in batch.iter().enumerate() {
                    if let Some(translated_line) = translated_lines.get(i) {
                        let cleaned_text = translated_line.trim();
                        if !cleaned_text.is_empty() {
                            println!("应用翻译 {}: '{}' -> '{}'", i + 1, text_item.text.trim(), cleaned_text);
                            apply_translation(text_item, cleaned_text);
                        }
                    }
                }
            } else {
                println!("行数不匹配，改为单独翻译每个文本");
                // 行数不匹配时单独翻译
                for (i, text_item) in batch.iter().enumerate() {
                    println!("单独翻译第 {} 个文本: '{}'", i + 1, text_item.text.trim());
                    if let Ok(individual_translated) = translator.translate(&text_item.text).await {
                        println!("单独翻译结果: '{}'", individual_translated.trim());
                        apply_translation(text_item, &individual_translated.trim());
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("批量翻译失败: {}", e);
            // 失败时尝试单独翻译每个文本
            for (i, text_item) in batch.iter().enumerate() {
                println!("fallback单独翻译第 {} 个文本: '{}'", i + 1, text_item.text.trim());
                if let Ok(translated) = translator.translate(&text_item.text).await {
                    println!("fallback翻译结果: '{}'", translated.trim());
                    apply_translation(text_item, &translated.trim());
                }
            }
        }
    }

    Ok(())
}

/// 应用翻译结果到DOM节点 - 增强错误处理
#[cfg(feature = "translation")]
fn apply_translation(text_item: &TextToTranslate, translated: &str) {
    let trimmed_translated = translated.trim();
    
    // 验证翻译结果质量
    if trimmed_translated.is_empty() {
        eprintln!("警告: 翻译结果为空，跳过: '{}'", text_item.text.trim());
        return;
    }
    
    // 检查翻译结果是否合理（不应该与原文完全相同，除非是专有名词）
    if trimmed_translated == text_item.text.trim() && text_item.text.len() > 5 {
        println!("注意: 翻译结果与原文相同，可能是专有名词: '{}'", trimmed_translated);
    }
    
    if let Some(attr_name) = &text_item.attr_name {
        // 属性翻译 - 直接更新属性，移除不必要的 panic 处理
        use crate::html::set_node_attr;
        set_node_attr(&text_item.node, attr_name, Some(trimmed_translated.to_string()));
    } else {
        // 文本节点翻译
        if let NodeData::Text { ref contents } = text_item.node.data {
            let mut content_ref = contents.borrow_mut();
            content_ref.clear();
            content_ref.push_slice(trimmed_translated);
        }
    }
}

/// 智能判断文本是否需要翻译 - 优化过滤逻辑
fn should_translate_for_collection(text: &str) -> bool {
    let trimmed = text.trim();

    // 基本检查
    if trimmed.is_empty() || trimmed.len() < 2 {
        return false;
    }

    // 排除常见的非翻译内容
    if is_non_translatable_content(trimmed) {
        return false;
    }

    // 必须包含字母字符
    if !trimmed.chars().any(|c| c.is_alphabetic()) {
        return false;
    }

    // 检查语言特征（如果已经是中文，跳过）
    if is_already_chinese(trimmed) {
        return false;
    }

    // 排除过短的功能性文本
    if trimmed.len() < 3 && is_functional_text(trimmed) {
        return false;
    }

    true
}

/// 检查是否为不需要翻译的内容
fn is_non_translatable_content(text: &str) -> bool {
    // URL 模式
    if text.starts_with("http://") || text.starts_with("https://") || text.starts_with("ftp://") {
        return true;
    }

    // 邮箱模式
    if text.contains('@') && text.contains('.') && text.len() < 100 {
        return true;
    }

    // 代码模式（包含大量特殊字符）
    let special_chars = text.chars().filter(|&c| {
        matches!(c, '{' | '}' | '[' | ']' | '(' | ')' | ';' | '=' | '<' | '>' | '/' | '\\')
    }).count();
    
    if special_chars > text.len() / 3 {
        return true;
    }

    // CSS选择器或类名
    if text.starts_with('.') || text.starts_with('#') || text.contains("::") {
        return true;
    }

    // 纯数字或纯符号
    if text.chars().all(|c| c.is_numeric() || c.is_ascii_punctuation() || c.is_whitespace()) {
        return true;
    }

    false
}

/// 检查是否已经是中文
fn is_already_chinese(text: &str) -> bool {
    let chinese_chars = text.chars().filter(|c| {
        ('\u{4e00}'..='\u{9fff}').contains(c) // 中文字符范围
    }).count();
    
    // 如果中文字符占比超过50%，认为已经是中文
    chinese_chars as f32 / text.chars().count() as f32 > 0.5
}

/// 检查是否为功能性文本
fn is_functional_text(text: &str) -> bool {
    matches!(text.to_lowercase().as_str(), 
        "ok" | "yes" | "no" | "on" | "off" | "go" | "up" | "x" | ">"
    )
}


/// 判断是否应该跳过翻译的元素 - 扩展列表
fn should_skip_element(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "script" | "style" | "code" | "pre" | "noscript" 
        | "meta" | "link" | "head" | "svg" | "math"
        | "canvas" | "video" | "audio" | "embed" | "object"
        | "iframe" | "map" | "area" | "base" | "br" | "hr"
        | "img" | "input" | "source" | "track" | "wbr"
    )
}


/// 翻译CSS内容中的文本
#[cfg(feature = "translation")]
pub async fn translate_css_content(
    css: &str,
    translator: &TranslationService,
) -> Result<String, MonolithError> {
    use regex::Regex;

    // 匹配CSS content属性中的文本
    let content_re = Regex::new(r#"content\s*:\s*["']([^"']+)["']"#)
        .map_err(|e| MonolithError::new(&format!("正则表达式错误: {}", e)))?;

    let mut result = css.to_string();

    for cap in content_re.captures_iter(css) {
        if let Some(text_match) = cap.get(1) {
            let text = text_match.as_str();
            if should_translate_for_collection(text) {
                match translator.translate(text).await {
                    Ok(translated) => {
                        let full_match = cap.get(0).unwrap().as_str();
                        let translated_rule = full_match.replace(text, &translated);
                        result = result.replace(full_match, &translated_rule);
                    }
                    Err(e) => {
                        eprintln!("CSS翻译警告: {}", e);
                        // 翻译失败时保持原CSS
                    }
                }
            }
        }
    }

    Ok(result)
}

/// 同步翻译接口（用于非async上下文）
#[cfg(feature = "translation")]
pub fn translate_dom_content_sync(
    dom: RcDom,
    target_lang: &str,
    api_url: Option<&str>,
) -> Result<RcDom, MonolithError> {
    // 创建运行时执行异步翻译
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| MonolithError::new(&format!("创建异步运行时失败: {}", e)))?;

    rt.block_on(translate_dom_content(dom, target_lang, api_url))
}

#[cfg(not(feature = "translation"))]
pub fn translate_dom_content_sync(
    dom: RcDom,
    _target_lang: &str,
    _api_url: Option<&str>,
) -> Result<RcDom, MonolithError> {
    // 翻译功能未启用时直接返回原DOM
    Ok(dom)
}
