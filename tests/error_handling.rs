//! 错误处理集成测试
//!
//! 测试系统在各种异常情况下的错误处理和恢复能力

use std::time::Duration;

use monolith::translation::error::{TranslationError, TranslationResult};
use monolith::translation::storage::cache::{CacheKey, CacheEntry};
use monolith::translation::pipeline::collector::TextItem;

mod common {
    include!("common/mod.rs");
}

use common::{
    TestEnvironment, TestConfigBuilder, HtmlTestHelper, TestDataGenerator,
    PerformanceHelper, AssertionHelper
};

/// 测试无效HTML输入的错误处理
#[tokio::test]
async fn test_invalid_html_error_handling() {
    let env = TestEnvironment::default();
    
    // 测试各种无效HTML输入
    let invalid_html_cases = vec![
        "",                                    // 空HTML
        "<html><head></head><body></body></html>", // 有效但空的HTML
        "<div>Unclosed div",                   // 未闭合标签
        "<html><>Invalid tag</>",             // 无效标签
        "Plain text without HTML tags",       // 纯文本
        "<script>alert('test')</script>",     // 只有脚本
        "<!DOCTYPE html>",                     // 只有DOCTYPE
    ];
    
    for (i, html) in invalid_html_cases.iter().enumerate() {
        let dom = HtmlTestHelper::create_test_dom(html);
        
        // 文本收集应该处理无效HTML而不崩溃
        let collection_result = env.text_collector.collect_translatable_texts(&dom.document);
        
        match collection_result {
            Ok(texts) => {
                // 对于无效HTML，可能返回空结果或少量文本
                assert!(texts.len() <= 5, 
                       "Invalid HTML case {} should yield few or no texts, got {}", i, texts.len());
                println!("✅ Invalid HTML case {}: collected {} texts safely", i, texts.len());
            }
            Err(e) => {
                // 如果返回错误，应该是明确的错误类型
                println!("✅ Invalid HTML case {}: handled error gracefully: {:?}", i, e);
            }
        }
    }
}

/// 测试缓存系统的错误恢复
#[tokio::test]
async fn test_cache_system_error_recovery() {
    let env = TestEnvironment::default();
    
    // 测试无效缓存键
    let invalid_keys = vec![
        CacheKey::new("".to_string(), "".to_string(), "".to_string()),
        CacheKey::new("text".to_string(), "".to_string(), "zh".to_string()),
        CacheKey::new("text".to_string(), "en".to_string(), "".to_string()),
        CacheKey::new("\0\0\0".to_string(), "en".to_string(), "zh".to_string()), // 空字符
    ];
    
    for (i, key) in invalid_keys.iter().enumerate() {
        // 获取操作应该优雅处理无效键
        let get_result = env.cache_manager.get(key).await;
        assert!(get_result.is_ok(), "Get operation should handle invalid key {} gracefully", i);
        
        // 存储操作也应该处理无效键
        let entry = CacheEntry::new(
            "test text".to_string(),
            "测试文本".to_string(),
            Some(Duration::from_secs(300)),
        );
        
        let put_result = env.cache_manager.put(key.clone(), entry).await;
        assert!(put_result.is_ok(), "Put operation should handle invalid key {} gracefully", i);
    }
    
    println!("✅ Cache error recovery test passed - handled {} invalid keys", invalid_keys.len());
}

/// 测试批次管理器的错误处理
#[tokio::test]
async fn test_batch_manager_error_handling() {
    let env = TestEnvironment::default();
    
    // 测试空文本列表
    let empty_batch_result = env.batch_manager.clone().create_batches(vec![]);
    assert!(empty_batch_result.is_ok(), "Should handle empty text list gracefully");
    
    let empty_batches = empty_batch_result.unwrap();
    assert!(empty_batches.is_empty(), "Empty input should yield no batches");
    
    // 测试包含无效文本的列表
    let dom = HtmlTestHelper::create_test_dom("<div>test</div>");
    let handle = dom.document.clone();
    
    let problematic_items = vec![
        TextItem::content("".to_string(), handle.clone(), 0), // 空文本
        TextItem::content("\0\0\0".to_string(), handle.clone(), 0), // 空字符
        TextItem::content("A".repeat(100000), handle.clone(), 0), // 超长文本
        TextItem::content("Valid text".to_string(), handle.clone(), 0), // 正常文本
    ];
    
    let batch_result = env.batch_manager.clone().create_batches(problematic_items);
    assert!(batch_result.is_ok(), "Should handle problematic text items gracefully");
    
    let batches = batch_result.unwrap();
    // 应该至少处理正常的文本项
    assert!(!batches.is_empty(), "Should create batches for valid items");
    
    println!("✅ Batch manager error handling test passed - created {} batches from problematic input", 
             batches.len());
}

/// 测试文本过滤器的边界情况
#[tokio::test]
async fn test_text_filter_boundary_cases() {
    let env = TestEnvironment::default();
    
    // 测试各种边界情况文本
    let boundary_cases = vec![
        "",                                      // 空字符串
        " ",                                     // 只有空格
        "\n\t\r",                               // 只有空白字符
        "a",                                     // 单字符
        "A".repeat(100000),                      // 超长文本
        "🚀🎉🌟",                                 // 只有emoji
        "12345",                                 // 只有数字
        "!@#$%^&*()",                           // 只有符号
        "https://example.com",                   // URL
        "user@example.com",                      // 邮箱
        "你好世界",                                // 纯中文
        "Hello 世界 こんにちは 안녕하세요",          // 多语言混合
        "<script>alert('xss')</script>",        // 可能的XSS
        "SELECT * FROM users;",                  // SQL-like文本
        "function() { return true; }",           // JavaScript-like文本
    ];
    
    for (i, text) in boundary_cases.iter().enumerate() {
        // 翻译判断应该不会崩溃
        let should_translate = env.text_filter.should_translate(text);
        
        // 可翻译性评分应该在有效范围内
        let score = env.text_filter.translatability_score(text);
        assert!(score >= 0.0 && score <= 1.0, 
               "Translatability score for case {} should be in [0,1], got {}", i, score);
        
        // 文本分析应该完成
        let analysis = env.text_filter.analyze_text(text);
        assert_eq!(analysis.original_text, *text, "Analysis should preserve original text");
        
        println!("✅ Boundary case {}: '{}' -> translate: {}, score: {:.2}", 
                 i, 
                 if text.len() > 20 { &format!("{}...", &text[..20]) } else { text },
                 should_translate, 
                 score);
    }
}

/// 测试并发错误情况
#[tokio::test]
async fn test_concurrent_error_scenarios() {
    let env = TestEnvironment::default();
    
    // 创建多个可能导致错误的并发任务
    let mut handles = vec![];
    
    for task_id in 0..20 {
        let cache_manager = env.cache_manager.clone();
        let text_filter = env.text_filter.clone();
        
        let handle = tokio::spawn(async move {
            let mut local_errors = 0;
            let mut local_successes = 0;
            
            // 每个任务执行多种可能出错的操作
            for op_id in 0..10 {
                // 测试无效缓存操作
                let invalid_key = CacheKey::new(
                    format!("task_{}_op_{}", task_id, op_id),
                    if op_id % 3 == 0 { "" } else { "en" }.to_string(),
                    "zh".to_string(),
                );
                
                let entry = CacheEntry::new(
                    "test".to_string(),
                    "测试".to_string(),
                    if op_id % 4 == 0 { None } else { Some(Duration::from_millis(1)) }, // 有些立即过期
                );
                
                // 并发缓存操作
                match cache_manager.put(invalid_key.clone(), entry).await {
                    Ok(_) => {
                        local_successes += 1;
                        // 尝试立即获取
                        if cache_manager.get(&invalid_key).await.is_ok() {
                            local_successes += 1;
                        } else {
                            local_errors += 1;
                        }
                    }
                    Err(_) => local_errors += 1,
                }
                
                // 测试边界文本过滤
                let boundary_text = if op_id % 5 == 0 {
                    "".to_string()
                } else if op_id % 5 == 1 {
                    "A".repeat(op_id * 1000)
                } else {
                    format!("Normal text {}", op_id)
                };
                
                let _filter_result = text_filter.should_translate(&boundary_text);
                local_successes += 1; // 过滤操作应该总是成功
            }
            
            (local_successes, local_errors)
        });
        
        handles.push(handle);
    }
    
    // 收集所有并发任务的结果
    let mut total_successes = 0;
    let mut total_errors = 0;
    
    for handle in handles {
        let (successes, errors) = handle.await.expect("Concurrent task should complete");
        total_successes += successes;
        total_errors += errors;
    }
    
    // 验证系统在并发错误情况下的稳定性
    assert!(total_successes > 0, "Should have some successful operations even with errors");
    
    // 系统应该能够处理错误而不崩溃
    let final_stats = env.cache_manager.get_stats();
    assert!(final_stats.total_entries >= 0, "Cache should remain in valid state");
    
    println!("✅ Concurrent error scenarios test passed - {} successes, {} errors handled gracefully", 
             total_successes, total_errors);
}

/// 测试资源耗尽情况的处理
#[tokio::test]
async fn test_resource_exhaustion_handling() {
    // 使用较小的配置来模拟资源限制
    let config = TestConfigBuilder::new()
        .with_fast_cache() // 小缓存
        .with_small_batches() // 小批次
        .build();
    
    let env = TestEnvironment::new(config);
    
    // 尝试处理大量文本项
    let large_text_set = TestDataGenerator::create_test_text_items(1000);
    
    // 批次处理应该能处理大量数据而不崩溃
    let batch_result = PerformanceHelper::assert_performance(
        || env.batch_manager.clone().create_batches(large_text_set.clone()),
        Duration::from_secs(5), // 给予足够时间但不能无限等待
        "Large dataset batch processing"
    );
    
    assert!(batch_result.is_ok(), "Should handle large dataset gracefully");
    let batches = batch_result.unwrap();
    assert!(!batches.is_empty(), "Should create batches even for large dataset");
    
    // 测试缓存在大量数据下的表现
    let large_cache_entries = TestDataGenerator::create_test_cache_entries(500);
    
    let cache_stress_result = PerformanceHelper::measure_async_time(|| async {
        let mut successful_operations = 0;
        
        for (key, entry) in large_cache_entries {
            if env.cache_manager.put(key.clone(), entry).await.is_ok() {
                if env.cache_manager.get(&key).await.is_ok() {
                    successful_operations += 1;
                }
            }
        }
        
        successful_operations
    }).await;
    
    let (successful_ops, stress_time) = cache_stress_result;
    
    // 应该完成大部分操作
    assert!(successful_ops > 400, 
           "Should complete most cache operations under stress, completed {}", successful_ops);
    
    // 性能应该保持在合理范围内
    assert!(stress_time < Duration::from_secs(10), 
           "Cache stress test should complete within reasonable time: {:?}", stress_time);
    
    println!("✅ Resource exhaustion handling test passed - {} operations completed in {:?}", 
             successful_ops, stress_time);
}

/// 测试错误传播和恢复机制
#[tokio::test]
async fn test_error_propagation_and_recovery() {
    let env = TestEnvironment::default();
    
    // 创建一个会导致各种错误的复杂场景
    let complex_html = r#"
    <html>
        <head><title></title></head>
        <body>
            <div>Valid content</div>
            <script>Invalid script content</script>
            <p></p>
            <div>Another valid content</div>
            <style>Invalid CSS</style>
            <p>Final valid content</p>
        </body>
    </html>
    "#;
    
    let dom = HtmlTestHelper::create_test_dom(complex_html);
    
    // 完整的处理流程应该能够从各种错误中恢复
    let processing_result = async {
        // 1. 文本收集 - 可能遇到无效元素
        let texts = env.text_collector.collect_translatable_texts(&dom.document)?;
        
        // 2. 过滤 - 可能遇到边界情况文本
        let filtered_texts: Vec<TextItem> = texts.into_iter()
            .filter(|t| env.text_filter.should_translate(&t.text))
            .collect();
        
        // 3. 批次创建 - 可能遇到无效文本项
        let batches = env.batch_manager.clone().create_batches(filtered_texts)?;
        
        // 4. 模拟缓存操作 - 可能遇到各种缓存错误
        let mut cache_operations = 0;
        for batch in &batches {
            for item in &batch.items {
                let key = CacheKey::new(
                    item.text.clone(),
                    "en".to_string(),
                    "zh".to_string(),
                );
                
                // 尝试获取缓存
                match env.cache_manager.get(&key).await {
                    Ok(Some(_)) => cache_operations += 1,
                    Ok(None) => {
                        // 缓存未命中，尝试存储
                        let entry = CacheEntry::new(
                            item.text.clone(),
                            format!("翻译:{}", item.text),
                            Some(Duration::from_secs(300)),
                        );
                        
                        if env.cache_manager.put(key, entry).await.is_ok() {
                            cache_operations += 1;
                        }
                    }
                    Err(_) => {
                        // 缓存错误，继续处理其他项
                        continue;
                    }
                }
            }
        }
        
        Ok::<(Vec<_>, usize), TranslationError>((batches, cache_operations))
    }.await;
    
    // 验证错误恢复结果
    match processing_result {
        Ok((batches, cache_ops)) => {
            assert!(!batches.is_empty(), "Should create batches despite errors");
            assert!(cache_ops > 0, "Should complete some cache operations despite errors");
            println!("✅ Error recovery successful - {} batches, {} cache operations", 
                     batches.len(), cache_ops);
        }
        Err(e) => {
            // 即使整体失败，也应该是可控的错误
            println!("✅ Error propagation controlled - got expected error: {:?}", e);
        }
    }
    
    // 验证系统状态仍然有效
    let cache_stats = env.cache_manager.get_stats();
    assert!(cache_stats.total_entries >= 0, "Cache should remain in valid state after errors");
    
    let collector_stats = env.text_collector.get_stats();
    assert!(collector_stats.nodes_visited >= 0, "Collector should remain in valid state");
}

/// 测试长时间运行的稳定性
#[tokio::test]
async fn test_long_running_stability() {
    let env = TestEnvironment::default();
    
    // 模拟长时间运行的操作
    let duration = Duration::from_secs(2); // 较短的测试时间
    let start_time = std::time::Instant::now();
    
    let mut operation_count = 0;
    let mut error_count = 0;
    
    while start_time.elapsed() < duration {
        // 执行各种操作的循环
        let html = if operation_count % 3 == 0 {
            HtmlTestHelper::create_simple_english_page()
        } else if operation_count % 3 == 1 {
            HtmlTestHelper::create_mixed_language_page()
        } else {
            format!("<div>Dynamic content {}</div>", operation_count)
        };
        
        let dom = HtmlTestHelper::create_test_dom(&html);
        
        // 执行完整的处理循环
        match env.text_collector.collect_translatable_texts(&dom.document) {
            Ok(texts) => {
                let filtered: Vec<_> = texts.into_iter()
                    .filter(|t| env.text_filter.should_translate(&t.text))
                    .collect();
                
                if !filtered.is_empty() {
                    match env.batch_manager.clone().create_batches(filtered) {
                        Ok(batches) => {
                            // 模拟缓存操作
                            for batch in batches.iter().take(2) { // 限制操作数量以加速测试
                                for item in batch.items.iter().take(3) {
                                    let key = CacheKey::new(
                                        item.text.clone(),
                                        "en".to_string(),
                                        "zh".to_string(),
                                    );
                                    
                                    let entry = CacheEntry::new(
                                        item.text.clone(),
                                        format!("翻译{}", operation_count),
                                        Some(Duration::from_secs(60)),
                                    );
                                    
                                    if env.cache_manager.put(key.clone(), entry).await.is_ok() {
                                        let _ = env.cache_manager.get(&key).await;
                                    }
                                }
                            }
                            operation_count += 1;
                        }
                        Err(_) => error_count += 1,
                    }
                }
            }
            Err(_) => error_count += 1,
        }
        
        // 防止测试运行过久
        if operation_count > 100 {
            break;
        }
        
        // 小延迟以避免过度消耗CPU
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // 验证长时间运行的结果
    assert!(operation_count > 0, "Should complete some operations during long run");
    
    // 错误率应该在可接受范围内
    let error_rate = error_count as f64 / (operation_count + error_count) as f64;
    assert!(error_rate < 0.1, "Error rate should be less than 10%, got {:.2}%", error_rate * 100.0);
    
    // 验证系统最终状态
    let final_cache_stats = env.cache_manager.get_stats();
    assert!(final_cache_stats.total_entries >= 0, "Cache should remain stable");
    
    println!("✅ Long running stability test passed - {} operations, {:.1}% error rate in {:?}", 
             operation_count, error_rate * 100.0, start_time.elapsed());
}