//! 缓存系统集成测试
//!
//! 测试缓存的多层架构和持久化功能

use std::time::Duration;
use std::sync::Arc;

use monolith::translation::storage::cache::{
    CacheManager, CacheConfig, CacheKey, CacheEntry, CacheStats
};

mod common {
    include!("common/mod.rs");
}

use common::{
    TestEnvironment, TestConfigBuilder, TestDataGenerator,
    PerformanceHelper, AssertionHelper
};

/// 测试基本缓存操作
#[tokio::test]
async fn test_basic_cache_operations() {
    let config = CacheConfig {
        enable_local_cache: true,
        local_cache_size: 100,
        default_ttl: Duration::from_secs(300),
        enable_warmup: false,
        cleanup_interval: Duration::from_secs(60),
    };
    
    let cache_manager = CacheManager::new(config);
    
    // 测试基本的存储和检索
    let key = CacheKey::new(
        "Hello world".to_string(),
        "en".to_string(),
        "zh".to_string(),
    );
    
    let entry = CacheEntry::new(
        "Hello world".to_string(),
        "你好世界".to_string(),
        Some(Duration::from_secs(300)),
    );
    
    // 初始状态应该为空
    let initial_result = cache_manager.get(&key).await
        .expect("Cache get should not error");
    assert!(initial_result.is_none(), "Cache should be empty initially");
    
    // 存储条目
    cache_manager.put(key.clone(), entry.clone()).await
        .expect("Cache put should succeed");
    
    // 检索条目
    let retrieved = cache_manager.get(&key).await
        .expect("Cache get should not error")
        .expect("Cache item should be found");
    
    assert_eq!(retrieved.original_text, entry.original_text);
    assert_eq!(retrieved.translated_text, entry.translated_text);
    
    println!("✅ Basic cache operations test passed");
}

/// 测试缓存统计和性能监控
#[tokio::test]
async fn test_cache_statistics_monitoring() {
    let config = CacheConfig {
        enable_local_cache: true,
        local_cache_size: 50,
        default_ttl: Duration::from_secs(300),
        enable_warmup: false,
        cleanup_interval: Duration::from_secs(60),
    };
    
    let cache_manager = CacheManager::new(config);
    
    // 初始统计
    let initial_stats = cache_manager.get_stats();
    assert_eq!(initial_stats.local_hits, 0);
    assert_eq!(initial_stats.misses, 0);
    
    // 生成测试数据
    let test_entries = TestDataGenerator::create_test_cache_entries(10);
    
    // 执行一系列操作并监控统计
    for (key, entry) in &test_entries {
        // 首次访问应该是miss
        let _miss_result = cache_manager.get(key).await.unwrap();
        
        // 存储条目
        cache_manager.put(key.clone(), entry.clone()).await.unwrap();
        
        // 再次访问应该是hit
        let _hit_result = cache_manager.get(key).await.unwrap();
    }
    
    // 验证统计信息
    let final_stats = cache_manager.get_stats();
    assert_eq!(final_stats.misses, test_entries.len());
    assert_eq!(final_stats.local_hits, test_entries.len());
    assert!(final_stats.hit_rate() > 0.0);
    
    // 测试统计重置
    cache_manager.reset_stats();
    let reset_stats = cache_manager.get_stats();
    assert_eq!(reset_stats.local_hits, 0);
    assert_eq!(reset_stats.misses, 0);
    
    println!("✅ Cache statistics monitoring test passed - tracked {} operations", 
             test_entries.len() * 2);
}

/// 测试缓存容量限制和LRU淘汰
#[tokio::test]
async fn test_cache_capacity_and_lru_eviction() {
    let small_capacity = 5;
    let config = CacheConfig {
        enable_local_cache: true,
        local_cache_size: small_capacity,
        default_ttl: Duration::from_secs(300),
        enable_warmup: false,
        cleanup_interval: Duration::from_secs(60),
    };
    
    let cache_manager = CacheManager::new(config);
    
    // 创建超过容量的测试条目
    let test_entries = TestDataGenerator::create_test_cache_entries(small_capacity * 2);
    
    // 填充缓存超过容量
    for (key, entry) in &test_entries {
        cache_manager.put(key.clone(), entry.clone()).await.unwrap();
    }
    
    // 验证缓存大小不超过限制
    let stats = cache_manager.get_stats();
    assert!(stats.total_entries <= small_capacity, 
           "Cache size {} should not exceed capacity {}", 
           stats.total_entries, small_capacity);
    
    // 验证最近使用的项目仍在缓存中
    let recent_keys = &test_entries[test_entries.len() - small_capacity..];
    for (key, _) in recent_keys {
        let result = cache_manager.get(key).await.unwrap();
        assert!(result.is_some(), "Recently added item should still be in cache");
    }
    
    // 验证早期的项目可能已被淘汰
    let early_keys = &test_entries[..test_entries.len() - small_capacity];
    let mut evicted_count = 0;
    for (key, _) in early_keys {
        let result = cache_manager.get(key).await.unwrap();
        if result.is_none() {
            evicted_count += 1;
        }
    }
    
    assert!(evicted_count > 0, "Some early items should have been evicted");
    
    println!("✅ Cache capacity and LRU eviction test passed - evicted {} items", 
             evicted_count);
}

/// 测试缓存过期机制
#[tokio::test]
async fn test_cache_expiration_mechanism() {
    let config = CacheConfig {
        enable_local_cache: true,
        local_cache_size: 100,
        default_ttl: Duration::from_millis(100), // 短过期时间
        enable_warmup: false,
        cleanup_interval: Duration::from_secs(60),
    };
    
    let cache_manager = CacheManager::new(config);
    
    // 创建短过期时间的条目
    let key = CacheKey::new(
        "Expiring item".to_string(),
        "en".to_string(),
        "zh".to_string(),
    );
    
    let entry = CacheEntry::new(
        "Expiring item".to_string(),
        "过期项目".to_string(),
        Some(Duration::from_millis(50)), // 非常短的TTL
    );
    
    // 存储条目
    cache_manager.put(key.clone(), entry.clone()).await.unwrap();
    
    // 立即访问应该成功
    let immediate_result = cache_manager.get(&key).await.unwrap();
    assert!(immediate_result.is_some(), "Item should be available immediately");
    
    // 等待过期
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // 访问过期项目
    let expired_result = cache_manager.get(&key).await.unwrap();
    assert!(expired_result.is_none(), "Expired item should not be available");
    
    // 测试清理过期项目
    let cleanup_count = cache_manager.cleanup_expired().await.unwrap();
    assert!(cleanup_count >= 0, "Cleanup should return non-negative count");
    
    println!("✅ Cache expiration mechanism test passed - cleaned up {} expired items", 
             cleanup_count);
}

/// 测试高并发缓存访问
#[tokio::test]
async fn test_high_concurrency_cache_access() {
    let config = CacheConfig {
        enable_local_cache: true,
        local_cache_size: 200,
        default_ttl: Duration::from_secs(300),
        enable_warmup: false,
        cleanup_interval: Duration::from_secs(60),
    };
    
    let cache_manager = Arc::new(CacheManager::new(config));
    
    // 并发操作参数
    let num_concurrent_tasks = 50;
    let operations_per_task = 10;
    
    // 创建并发任务
    let mut handles = vec![];
    
    for task_id in 0..num_concurrent_tasks {
        let cm = Arc::clone(&cache_manager);
        
        let handle = tokio::spawn(async move {
            let mut local_operations = 0;
            
            for op_id in 0..operations_per_task {
                let key = CacheKey::new(
                    format!("concurrent_item_{}_{}", task_id, op_id),
                    "en".to_string(),
                    "zh".to_string(),
                );
                
                let entry = CacheEntry::new(
                    format!("Item {} {}", task_id, op_id),
                    format!("项目 {} {}", task_id, op_id),
                    Some(Duration::from_secs(300)),
                );
                
                // 并发存储
                cm.put(key.clone(), entry).await.unwrap();
                
                // 并发检索
                let _retrieved = cm.get(&key).await.unwrap();
                
                local_operations += 1;
            }
            
            local_operations
        });
        
        handles.push(handle);
    }
    
    // 等待所有并发任务完成
    let (total_operations, concurrent_time) = PerformanceHelper::measure_async_time(|| async {
        let mut total = 0;
        for handle in handles {
            let ops = handle.await.expect("Concurrent task should complete");
            total += ops;
        }
        total
    }).await;
    
    // 验证并发操作结果
    let expected_operations = num_concurrent_tasks * operations_per_task;
    assert_eq!(total_operations, expected_operations, 
              "All concurrent operations should complete");
    
    // 验证缓存状态
    let final_stats = cache_manager.get_stats();
    assert!(final_stats.total_entries > 0, "Concurrent operations should populate cache");
    
    // 验证性能
    let ops_per_second = total_operations as f64 / concurrent_time.as_secs_f64();
    assert!(ops_per_second > 100.0, 
           "Should achieve reasonable throughput: {} ops/sec", ops_per_second);
    
    println!("✅ High concurrency test passed - {} operations in {:?} at {:.1} ops/sec", 
             total_operations, concurrent_time, ops_per_second);
}

/// 测试缓存键的字符串表示和哈希
#[tokio::test]
async fn test_cache_key_handling() {
    let cache_manager = CacheManager::new(CacheConfig::default());
    
    // 测试不同类型的缓存键
    let test_cases = vec![
        ("Hello world", "en", "zh"),
        ("Complex text with symbols: !@#$%^&*()", "en", "zh"),
        ("", "en", "zh"), // 空文本
        ("Text", "", "zh"), // 空源语言
        ("Text", "en", ""), // 空目标语言
        ("你好世界", "zh", "en"), // 中文文本
        ("Mixed 中英文 text", "auto", "zh"), // 混合语言
        ("🚀 Emoji test 🎉", "en", "zh"), // 包含emoji
    ];
    
    for (text, src_lang, target_lang) in &test_cases {
        let key = CacheKey::new(
            text.to_string(),
            src_lang.to_string(),
            target_lang.to_string(),
        );
        
        let entry = CacheEntry::new(
            text.to_string(),
            format!("translated_{}", text),
            Some(Duration::from_secs(300)),
        );
        
        // 存储和检索应该都成功
        let store_result = cache_manager.put(key.clone(), entry.clone()).await;
        assert!(store_result.is_ok(), "Should handle cache key: {:?}", key);
        
        let retrieve_result = cache_manager.get(&key).await;
        assert!(retrieve_result.is_ok(), "Should retrieve cache key: {:?}", key);
        
        // 验证键的字符串表示
        let key_string = key.to_string();
        assert!(!key_string.is_empty(), "Key string should not be empty");
        assert!(key_string.contains(src_lang), "Key string should contain source language");
        assert!(key_string.contains(target_lang), "Key string should contain target language");
    }
    
    println!("✅ Cache key handling test passed - handled {} different key types", 
             test_cases.len());
}

/// 测试缓存错误处理和恢复
#[tokio::test]
async fn test_cache_error_handling_and_recovery() {
    let config = CacheConfig {
        enable_local_cache: true,
        local_cache_size: 10,
        default_ttl: Duration::from_secs(300),
        enable_warmup: false,
        cleanup_interval: Duration::from_secs(60),
    };
    
    let cache_manager = CacheManager::new(config);
    
    // 测试大量数据的处理
    let large_text = "A".repeat(100000); // 100KB文本
    let large_key = CacheKey::new(
        large_text.clone(),
        "en".to_string(),
        "zh".to_string(),
    );
    
    let large_entry = CacheEntry::new(
        large_text.clone(),
        format!("translated_{}", &large_text[..100]), // 截断翻译以节省内存
        Some(Duration::from_secs(300)),
    );
    
    // 大文本存储应该成功
    let large_store_result = cache_manager.put(large_key.clone(), large_entry).await;
    assert!(large_store_result.is_ok(), "Should handle large text gracefully");
    
    // 大文本检索应该成功
    let large_retrieve_result = cache_manager.get(&large_key).await;
    assert!(large_retrieve_result.is_ok(), "Should retrieve large text gracefully");
    
    // 测试并发访问同一键
    let shared_key = CacheKey::new(
        "shared_item".to_string(),
        "en".to_string(),
        "zh".to_string(),
    );
    
    let mut concurrent_handles = vec![];
    
    for i in 0..10 {
        let cm = cache_manager.clone();
        let key = shared_key.clone();
        
        let handle = tokio::spawn(async move {
            let entry = CacheEntry::new(
                "shared_item".to_string(),
                format!("translation_{}", i),
                Some(Duration::from_secs(300)),
            );
            
            // 并发存储同一键
            cm.put(key.clone(), entry).await.unwrap();
            
            // 并发检索
            let retrieved = cm.get(&key).await.unwrap();
            retrieved.is_some()
        });
        
        concurrent_handles.push(handle);
    }
    
    // 等待所有并发操作完成
    let mut success_count = 0;
    for handle in concurrent_handles {
        let success = handle.await.expect("Concurrent operation should complete");
        if success {
            success_count += 1;
        }
    }
    
    assert_eq!(success_count, 10, "All concurrent operations on same key should succeed");
    
    // 验证最终状态一致性
    let final_result = cache_manager.get(&shared_key).await.unwrap();
    assert!(final_result.is_some(), "Shared key should have a final consistent value");
    
    println!("✅ Cache error handling test passed - handled edge cases and concurrency gracefully");
}

/// 性能压力测试
#[tokio::test]
async fn test_cache_performance_stress() {
    let config = CacheConfig {
        enable_local_cache: true,
        local_cache_size: 1000,
        default_ttl: Duration::from_secs(300),
        enable_warmup: false,
        cleanup_interval: Duration::from_secs(60),
    };
    
    let cache_manager = CacheManager::new(config);
    
    // 大量操作性能测试
    let num_operations = 5000;
    let test_entries = TestDataGenerator::create_test_cache_entries(num_operations);
    
    // 测试批量写入性能
    let (_, write_time) = PerformanceHelper::measure_async_time(|| async {
        for (key, entry) in &test_entries {
            cache_manager.put(key.clone(), entry.clone()).await.unwrap();
        }
    }).await;
    
    let write_ops_per_sec = num_operations as f64 / write_time.as_secs_f64();
    
    // 测试批量读取性能
    let (_, read_time) = PerformanceHelper::measure_async_time(|| async {
        for (key, _) in &test_entries {
            let _result = cache_manager.get(key).await.unwrap();
        }
    }).await;
    
    let read_ops_per_sec = num_operations as f64 / read_time.as_secs_f64();
    
    // 性能断言
    assert!(write_ops_per_sec > 1000.0, 
           "Write performance should exceed 1000 ops/sec, got {:.1}", write_ops_per_sec);
    assert!(read_ops_per_sec > 5000.0, 
           "Read performance should exceed 5000 ops/sec, got {:.1}", read_ops_per_sec);
    
    // 内存使用检查
    let final_stats = cache_manager.get_stats();
    assert!(final_stats.total_entries > 0, "Cache should contain entries after stress test");
    
    println!("✅ Cache performance stress test passed - Write: {:.1} ops/sec, Read: {:.1} ops/sec", 
             write_ops_per_sec, read_ops_per_sec);
}