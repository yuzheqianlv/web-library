//! 环境变量系统演示
//! 
//! 展示如何使用新的类型安全环境变量管理系统

use monolith::env::{EnvConfig, EnvVar};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Monolith 环境变量系统演示 ===\n");

    // 设置一些示例环境变量
    env::set_var("MONOLITH_MODE", "development");
    env::set_var("MONOLITH_TRANSLATION_ENABLED", "true");
    env::set_var("MONOLITH_TRANSLATION_TARGET_LANG", "zh");
    env::set_var("MONOLITH_WEB_PORT", "8080");
    env::set_var("MONOLITH_CACHE_LOCAL_SIZE", "2000");

    // 1. 单独获取环境变量
    println!("1. 单独获取环境变量:");
    println!("   运行模式: {}", monolith::env::core::Mode::get()?);
    println!("   翻译启用: {}", monolith::env::translation::Enabled::get()?);
    println!("   目标语言: {}", monolith::env::translation::TargetLang::get()?);
    println!("   Web端口: {}", monolith::env::web::Port::get()?);
    println!("   缓存大小: {}", monolith::env::cache::LocalCacheSize::get()?);

    // 2. 批量加载配置
    println!("\n2. 完整环境配置:");
    let config = EnvConfig::from_env()?;
    config.print_summary();

    // 3. 类型安全验证
    println!("\n3. 类型安全验证:");
    
    // 测试无效值
    env::set_var("MONOLITH_WEB_PORT", "invalid");
    match monolith::env::web::Port::get() {
        Ok(port) => println!("   端口: {}", port),
        Err(e) => println!("   端口解析错误: {}", e),
    }

    // 测试超出范围的值
    env::set_var("MONOLITH_TRANSLATION_MAX_REQUESTS_PER_SECOND", "2000");
    match monolith::env::translation::MaxRequestsPerSecond::get() {
        Ok(rate) => println!("   请求速率: {}", rate),
        Err(e) => println!("   请求速率验证失败: {}", e),
    }

    // 4. 默认值演示
    println!("\n4. 默认值演示:");
    env::remove_var("MONOLITH_LOG_LEVEL");
    println!("   日志级别 (默认): {}", monolith::env::core::LogLevel::get()?);

    // 清理测试环境变量
    env::remove_var("MONOLITH_MODE");
    env::remove_var("MONOLITH_TRANSLATION_ENABLED");
    env::remove_var("MONOLITH_TRANSLATION_TARGET_LANG");
    env::remove_var("MONOLITH_WEB_PORT");
    env::remove_var("MONOLITH_CACHE_LOCAL_SIZE");
    env::remove_var("MONOLITH_TRANSLATION_MAX_REQUESTS_PER_SECOND");

    println!("\n=== 演示完成 ===");
    Ok(())
}