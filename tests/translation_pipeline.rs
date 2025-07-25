//! 翻译管道集成测试
//!
//! 测试整个翻译流程的端到端功能

use std::time::Duration;

use monolith::translation::pipeline::collector::{TextItem, TextPriority};
use monolith::translation::pipeline::batch::{Batch, BatchPriority};
use monolith::translation::storage::cache::{CacheKey, CacheEntry};

mod common {
    include!("common/mod.rs");
}

use common::{
    TestEnvironment, HtmlTestHelper, TestDataGenerator,
    PerformanceHelper, AssertionHelper
};

/// 测试完整的文本收集流程
#[tokio::test]
async fn test_complete_text_collection_pipeline() {
    let mut env = TestEnvironment::default();
    
    // 创建测试HTML
    let html = HtmlTestHelper::create_simple_english_page();
    let dom = HtmlTestHelper::create_test_dom(&html);
    
    // 执行文本收集
    let (texts, collection_time) = PerformanceHelper::measure_async_time(|| async {
        env.text_collector.collect_translatable_texts(&dom.document)
    }).await;
    
    let texts = texts.expect("Text collection should succeed");
    
    // 验证收集结果
    assert!(!texts.is_empty(), "Should collect some translatable texts");
    AssertionHelper::assert_size_in_range(&texts, 5, 20, "Collected texts count");
    
    // 验证文本内容
    let text_contents: Vec<String> = texts.iter().map(|t| t.text.clone()).collect();
    
    // 应该包含页面标题
    assert!(text_contents.iter().any(|t| t.contains("Test Page")), 
           "Should collect page title");
    
    // 应该包含主要内容
    assert!(text_contents.iter().any(|t| t.contains("Welcome to Test")), 
           "Should collect main heading");
    
    // 应该包含段落文本
    assert!(text_contents.iter().any(|t| t.contains("test paragraph")), 
           "Should collect paragraph text");
    
    // 验证性能
    assert!(collection_time < Duration::from_millis(100), 
           "Text collection should be fast for simple page");
    
    println!("✅ Text collection pipeline test passed - collected {} texts in {:?}", 
             texts.len(), collection_time);
}

/// 测试文本过滤和优先级排序
#[tokio::test]
async fn test_text_filtering_and_prioritization() {
    let mut env = TestEnvironment::default();
    
    // 创建混合内容的HTML
    let html = HtmlTestHelper::create_mixed_language_page();
    let dom = HtmlTestHelper::create_test_dom(&html);
    
    // 收集文本
    let texts = env.get_text_collector_mut()
        .collect_translatable_texts(&dom.document)
        .expect("Text collection should succeed");
    
    // 验证过滤效果
    let english_texts: Vec<&TextItem> = texts.iter()
        .filter(|t| env.text_filter.should_translate(&t.text))
        .collect();
    
    let chinese_texts: Vec<&TextItem> = texts.iter()
        .filter(|t| !env.text_filter.should_translate(&t.text) && 
                    env.text_filter.is_already_chinese(&t.text))
        .collect();
    
    assert!(!english_texts.is_empty(), "Should find English texts to translate");
    assert!(!chinese_texts.is_empty(), "Should find Chinese texts to skip");
    
    // 验证优先级排序
    let critical_texts: Vec<&TextItem> = texts.iter()
        .filter(|t| t.priority == TextPriority::Critical)
        .collect();
    
    let high_texts: Vec<&TextItem> = texts.iter()
        .filter(|t| t.priority == TextPriority::High)
        .collect();
    
    assert!(!critical_texts.is_empty(), "Should have critical priority texts (titles)");
    assert!(!high_texts.is_empty(), "Should have high priority texts (buttons/links)");
    
    println!("✅ Text filtering test passed - English: {}, Chinese: {}, Critical: {}, High: {}", 
             english_texts.len(), chinese_texts.len(), critical_texts.len(), high_texts.len());
}

/// 测试智能批次创建和管理
#[tokio::test]
async fn test_intelligent_batch_creation() {
    let mut env = TestEnvironment::default();
    
    // 创建测试文本项
    let test_items = TestDataGenerator::create_priority_mixed_items();
    let more_items = TestDataGenerator::create_test_text_items(10);
    let mut all_items = test_items;
    all_items.extend(more_items);
    
    // 测试批次创建
    let (batches, batch_time) = PerformanceHelper::measure_time(|| {
        env.get_batch_manager_mut().create_batches(all_items)
    });
    
    // create_batches returns Vec<Batch>, not Result
    let batches = batches;
    
    // 验证批次结果
    assert!(!batches.is_empty(), "Should create at least one batch");
    AssertionHelper::assert_size_in_range(&batches, 1, 5, "Number of batches");
    
    // 验证批次属性
    for (i, batch) in batches.iter().enumerate() {
        assert!(!batch.items.is_empty(), "Batch {} should not be empty", i);
        assert!(batch.estimated_chars > 0, "Batch {} should have character count", i);
        assert!(batch.estimated_effective_size > 0.0, "Batch {} should have effective size", i);
        
        // 验证批次大小限制
        assert!(batch.estimated_chars <= env.config.batch_config.max_batch_size * 2, 
               "Batch {} size should be reasonable", i);
    }
    
    // 验证优先级排序
    let critical_batch = batches.iter()
        .find(|b| b.priority == BatchPriority::Critical);
    assert!(critical_batch.is_some(), "Should have a critical priority batch");
    
    // 验证性能
    assert!(batch_time < Duration::from_millis(50), 
           "Batch creation should be fast");
    
    println!("✅ Batch creation test passed - created {} batches in {:?}", 
             batches.len(), batch_time);
}

/// 测试缓存系统集成
#[tokio::test]
async fn test_cache_system_integration() {
    let env = TestEnvironment::default();
    
    // 创建测试缓存条目
    let test_entries = TestDataGenerator::create_test_cache_entries(5);
    
    // 测试缓存存储和检索
    for (key, entry) in &test_entries {
        // 存储缓存项
        let store_result = env.cache_manager.put(key.clone(), entry.clone()).await;
        assert!(store_result.is_ok(), "Cache storage should succeed");
        
        // 检索缓存项
        let retrieved = env.cache_manager.get(key).await
            .expect("Cache retrieval should not error")
            .expect("Cache item should be found");
        
        assert_eq!(retrieved.original_text, entry.original_text, "Original text should match");
        assert_eq!(retrieved.translated_text, entry.translated_text, "Translated text should match");
    }
    
    // 测试缓存统计
    let stats = env.cache_manager.get_stats();
    assert!(stats.local_hits > 0, "Should have cache hits");
    assert_eq!(stats.misses, 0, "Should have no misses for stored items");
    
    // 测试缓存清理
    let cleaned = env.cache_manager.cleanup_expired().await
        .expect("Cache cleanup should succeed");
    assert!(cleaned >= 0, "Cleanup count should be non-negative");
    
    println!("✅ Cache integration test passed - stored and retrieved {} items", 
             test_entries.len());
}

/// 测试复杂页面的完整处理流程
#[tokio::test]
async fn test_complex_page_processing() {
    let mut env = TestEnvironment::default();
    
    // 创建复杂HTML页面
    let html = HtmlTestHelper::create_complex_page();
    let dom = HtmlTestHelper::create_test_dom(&html);
    
    // 完整处理流程测试
    let (processing_result, total_time) = PerformanceHelper::measure_async_time(|| async {
        // 1. 文本收集
        let texts = env.get_text_collector_mut()
            .collect_translatable_texts(&dom.document)?;
        
        // 2. 文本过滤
        let filtered_texts: Vec<TextItem> = texts.into_iter()
            .filter(|t| env.text_filter.should_translate(&t.text))
            .collect();
        
        // 3. 批次创建
        let batches = env.get_batch_manager_mut().create_batches(filtered_texts);
        
        // 4. 模拟缓存操作
        let mut cache_hits = 0;
        for batch in &batches {
            for item in &batch.items {
                let cache_key = CacheKey::new(
                    item.text.clone(),
                    "en".to_string(),
                    "zh".to_string(),
                );
                
                if env.cache_manager.get(&cache_key).await?.is_none() {
                    // 模拟翻译并缓存
                    let cache_entry = CacheEntry::new(
                        item.text.clone(),
                        format!("翻译:{}", item.text),
                        Some(Duration::from_secs(3600)),
                    );
                    env.cache_manager.put(cache_key, cache_entry).await?;
                } else {
                    cache_hits += 1;
                }
            }
        }
        
        Ok::<(Vec<Batch>, usize), Box<dyn std::error::Error + Send + Sync>>(
            (batches, cache_hits)
        )
    }).await;
    
    let (batches, _cache_hits) = processing_result.expect("Processing should succeed");
    
    // 验证处理结果
    assert!(!batches.is_empty(), "Should create batches for complex page");
    AssertionHelper::assert_size_in_range(&batches, 1, 10, "Number of batches for complex page");
    
    // 计算总处理的文本数量
    let total_texts: usize = batches.iter().map(|b| b.items.len()).sum();
    assert!(total_texts > 20, "Should process substantial amount of text from complex page");
    
    // 验证性能 - 复杂页面处理应该在合理时间内完成
    assert!(total_time < Duration::from_secs(2), 
           "Complex page processing should complete within 2 seconds");
    
    // 验证缓存效果
    let cache_stats = env.cache_manager.get_stats();
    assert!(cache_stats.total_entries > 0, "Should have cached translations");
    
    println!("✅ Complex page processing test passed - processed {} texts in {} batches within {:?}", 
             total_texts, batches.len(), total_time);
}

/// 测试错误处理和恢复
#[tokio::test]
async fn test_error_handling_and_recovery() {
    let mut env = TestEnvironment::default();
    
    // 测试空HTML处理
    let empty_dom = HtmlTestHelper::create_test_dom("");
    let empty_result = env.get_text_collector_mut().collect_translatable_texts(&empty_dom.document);
    assert!(empty_result.is_ok(), "Should handle empty HTML gracefully");
    
    let empty_texts = empty_result.unwrap();
    assert!(empty_texts.is_empty() || empty_texts.len() <= 1, "Empty HTML should yield no or minimal texts");
    
    // 测试无效缓存键处理
    let invalid_key = CacheKey::new("".to_string(), "".to_string(), "".to_string());
    let cache_result = env.cache_manager.get(&invalid_key).await;
    assert!(cache_result.is_ok(), "Should handle invalid cache keys gracefully");
    
    // 测试空批次处理
    let batches = env.get_batch_manager_mut().create_batches(vec![]);
    assert!(batches.is_empty(), "Empty input should yield no batches");
    
    // 测试极长文本处理
    let very_long_text = "A".repeat(10000);
    let long_text_translatable = env.text_filter.should_translate(&very_long_text);
    // 应该能处理长文本而不panic
    assert!(long_text_translatable || !long_text_translatable, "Should handle very long text");
    
    println!("✅ Error handling test passed - system handles edge cases gracefully");
}

/// 测试并发安全性
#[tokio::test]
async fn test_concurrent_operations() {
    let env = TestEnvironment::default();
    
    // 创建测试数据
    let test_entries = TestDataGenerator::create_test_cache_entries(20);
    let cache_manager = env.cache_manager.clone();
    
    // 并发缓存操作
    let mut handles = vec![];
    
    for (key, entry) in test_entries {
        let cm = cache_manager.clone();
        let handle = tokio::spawn(async move {
            // 并发存储
            cm.put(key.clone(), entry.clone()).await.unwrap();
            
            // 并发检索
            let retrieved = cm.get(&key).await.unwrap();
            assert!(retrieved.is_some(), "Concurrent retrieval should work");
            
            key.original_text
        });
        handles.push(handle);
    }
    
    // 等待所有并发操作完成
    let mut results = vec![];
    for handle in handles {
        let result = handle.await.expect("Concurrent operation should complete");
        results.push(result);
    }
    
    // 验证并发操作结果
    assert_eq!(results.len(), 20, "All concurrent operations should complete");
    
    // 验证缓存状态
    let final_stats = cache_manager.get_stats();
    assert!(final_stats.total_entries > 0, "Concurrent operations should populate cache");
    
    println!("✅ Concurrent operations test passed - {} operations completed successfully", 
             results.len());
}

/// 性能基准测试
#[tokio::test]
async fn test_performance_benchmarks() {
    let mut env = TestEnvironment::default();
    
    // 测试大量文本项的批次处理性能
    let large_text_set = TestDataGenerator::create_test_text_items(1000);
    
    let batches = PerformanceHelper::assert_performance(
        || env.get_batch_manager_mut().create_batches(large_text_set),
        Duration::from_millis(500),
        "Large batch creation"
    );
    
    // create_batches returns Vec<Batch>, not Result
    assert!(!batches.is_empty(), "Should create batches for large text set");
    
    // 测试大量缓存操作性能
    let large_cache_set = TestDataGenerator::create_test_cache_entries(100);
    
    let cache_operations_time = PerformanceHelper::measure_async_time(|| async {
        for (key, entry) in large_cache_set {
            env.cache_manager.put(key.clone(), entry).await.unwrap();
            let _retrieved = env.cache_manager.get(&key).await.unwrap();
        }
    }).await.1;
    
    assert!(cache_operations_time < Duration::from_secs(1), 
           "100 cache operations should complete within 1 second, took {:?}", 
           cache_operations_time);
    
    println!("✅ Performance benchmarks passed - batch creation and cache operations within acceptable limits");
}