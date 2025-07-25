# Environment Variables Configuration

本文档描述了Monolith项目的环境变量配置系统，该系统基于翻译组件重构的经验，提供类型安全且可验证的环境变量管理。

## 系统特性

- **类型安全**: 所有环境变量都有明确的类型定义和验证规则
- **默认值**: 每个变量都有合理的默认值，确保系统可以在未设置环境变量时正常运行
- **验证机制**: 自动验证环境变量的格式、范围和有效性
- **统一接口**: 提供一致的API来访问所有环境变量
- **文档化**: 每个变量都有详细的描述和用法说明

## 使用方式

### 基本用法

```rust
use monolith::env::{EnvVar, translation, web, cache};

// 获取单个环境变量
let target_lang = translation::TargetLang::get()?;
let web_port = web::Port::get()?;
let cache_size = cache::LocalCacheSize::get()?;

// 使用默认值
let mode = core::Mode::get_or_default("production".to_string());
```

### 批量加载配置

```rust
use monolith::env::EnvConfig;

// 加载所有环境变量配置
let config = EnvConfig::from_env()?;

// 验证配置
config.validate()?;

// 打印配置摘要
config.print_summary();
```

## 环境变量列表

### 核心配置 (Core Configuration)

| 变量名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `MONOLITH_MODE` | String | "production" | 应用运行模式: development, staging, production |
| `MONOLITH_LOG_LEVEL` | String | "info" | 日志级别: trace, debug, info, warn, error |
| `NO_COLOR` | Boolean | false | 禁用彩色输出 |

### 翻译配置 (Translation Configuration)

| 变量名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `MONOLITH_TRANSLATION_ENABLED` | Boolean | true | 启用翻译功能 |
| `MONOLITH_TRANSLATION_TARGET_LANG` | String | "zh" | 目标语言 (ISO 639-1 代码) |
| `MONOLITH_TRANSLATION_SOURCE_LANG` | String | "auto" | 源语言 ('auto' 表示自动检测) |
| `MONOLITH_TRANSLATION_API_URL` | String | "http://localhost:1188/translate" | 翻译API端点URL |
| `MONOLITH_TRANSLATION_MAX_REQUESTS_PER_SECOND` | Float | 5.0 | 每秒最大请求数 (1.0-1000.0) |
| `MONOLITH_TRANSLATION_MAX_CONCURRENT_REQUESTS` | Integer | 10 | 最大并发请求数 (1-100) |
| `MONOLITH_TRANSLATION_BATCH_SIZE` | Integer | 9000 | 批次大小 (100-50000) |
| `MONOLITH_TRANSLATION_BATCH_TIMEOUT` | Integer | 30 | 批次超时时间(秒) (1-300) |

### 缓存配置 (Cache Configuration)

| 变量名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `MONOLITH_CACHE_ENABLED` | Boolean | true | 启用缓存功能 |
| `MONOLITH_CACHE_LOCAL_SIZE` | Integer | 1000 | 本地缓存大小 (10-100000) |
| `MONOLITH_CACHE_TTL` | Integer | 3600 | 缓存TTL(秒) (60-604800) |
| `MONOLITH_CACHE_WARMUP_ENABLED` | Boolean | false | 启用缓存预热 |

### Web服务器配置 (Web Server Configuration)

| 变量名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `MONOLITH_WEB_BIND_ADDRESS` | String | "127.0.0.1" | Web服务器绑定地址 |
| `MONOLITH_WEB_PORT` | Integer | 7080 | Web服务器端口 |
| `MONOLITH_WEB_STATIC_DIR` | String | "static" | 静态文件目录 |

### MongoDB配置 (MongoDB Configuration)

| 变量名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `MONGODB_URL` | String | "mongodb://localhost:27017" | MongoDB连接字符串 |
| `MONGODB_DATABASE` | String | "monolith" | 数据库名称 |
| `MONGODB_COLLECTION` | String | "html_cache" | 集合名称 |

### 性能配置 (Performance Configuration)

| 变量名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `MONOLITH_WORKER_THREADS` | Integer | (CPU cores) | 工作线程数 (1-128) |
| `MONOLITH_MAX_MEMORY_MB` | Integer | 512 | 最大内存使用(MB) (64-32768) |
| `MONOLITH_PARALLEL_ENABLED` | Boolean | true | 启用并行处理 |

### 安全配置 (Security Configuration)

| 变量名 | 类型 | 默认值 | 描述 |
|--------|------|--------|------|
| `MONOLITH_API_KEY` | String | (无) | API认证密钥 (最少8字符) |
| `MONOLITH_CORS_ORIGINS` | String | "*" | CORS允许的源 (逗号分隔) |

## 布尔值格式

布尔类型环境变量支持多种格式：

- **True值**: `true`, `1`, `yes`, `on`, `enabled`
- **False值**: `false`, `0`, `no`, `off`, `disabled`

大小写不敏感。

## 配置文件集成

环境变量系统与现有的配置文件系统完美集成：

1. **加载顺序**: 配置文件 → 环境变量 → 命令行参数
2. **覆盖机制**: 后加载的配置会覆盖先加载的配置
3. **向下兼容**: 保持与现有配置接口的兼容性

### 示例配置

```bash
# 开发环境配置
export MONOLITH_MODE=development
export MONOLITH_LOG_LEVEL=debug
export MONOLITH_TRANSLATION_ENABLED=true
export MONOLITH_TRANSLATION_TARGET_LANG=zh
export MONOLITH_WEB_PORT=8080
export MONOLITH_CACHE_LOCAL_SIZE=2000

# 生产环境配置
export MONOLITH_MODE=production
export MONOLITH_LOG_LEVEL=warn
export MONOLITH_TRANSLATION_MAX_REQUESTS_PER_SECOND=10.0
export MONOLITH_CACHE_TTL=7200
export MONOLITH_WEB_BIND_ADDRESS=0.0.0.0
export MONOLITH_WEB_PORT=80
```

## 错误处理

环境变量系统提供详细的错误信息：

```rust
use monolith::env::web;

match web::Port::get() {
    Ok(port) => println!("端口: {}", port),
    Err(e) => {
        // 错误信息格式: "Environment variable 'MONOLITH_WEB_PORT': <详细错误>"
        eprintln!("配置错误: {}", e);
    }
}
```

### 常见错误类型

1. **类型错误**: 无法解析为指定类型
2. **范围错误**: 数值超出允许范围
3. **格式错误**: 不符合预期格式（如URL格式）
4. **必需变量缺失**: 没有默认值的必需变量未设置

## 最佳实践

### 1. 分层配置

```bash
# .env.development
MONOLITH_MODE=development
MONOLITH_LOG_LEVEL=debug

# .env.production  
MONOLITH_MODE=production
MONOLITH_LOG_LEVEL=error
```

### 2. 容器化部署

```dockerfile
# Dockerfile
ENV MONOLITH_MODE=production
ENV MONOLITH_WEB_BIND_ADDRESS=0.0.0.0
ENV MONOLITH_WEB_PORT=80
```

### 3. Kubernetes配置

```yaml
# ConfigMap
apiVersion: v1
kind: ConfigMap
metadata:
  name: monolith-config
data:
  MONOLITH_MODE: "production"
  MONOLITH_TRANSLATION_ENABLED: "true"
  MONOLITH_CACHE_LOCAL_SIZE: "5000"
```

### 4. 敏感信息处理

```bash
# 使用Secret管理敏感信息
export MONOLITH_API_KEY=$(cat /run/secrets/api_key)
export MONGODB_URL=$(cat /run/secrets/mongodb_url)
```

## 开发指南

### 添加新的环境变量

1. **定义结构体和实现**:

```rust
pub mod my_module {
    use super::*;
    
    pub struct MyVariable;
    impl EnvVar<String> for MyVariable {
        const NAME: &'static str = "MONOLITH_MY_VARIABLE";
        const DEFAULT: Option<String> = None;
        const DESCRIPTION: &'static str = "My custom variable";
        
        fn get() -> EnvResult<String> {
            match env::var(Self::NAME) {
                Ok(value) => Self::parse(&value),
                Err(_) => Ok("default_value".to_string()),
            }
        }
        
        fn parse(value: &str) -> EnvResult<String> {
            // 添加验证逻辑
            if value.is_empty() {
                return Err(EnvError {
                    variable: Self::NAME.to_string(),
                    message: "Value cannot be empty".to_string(),
                });
            }
            Ok(value.to_string())
        }
    }
}
```

2. **更新EnvConfig结构体**:

```rust
pub struct EnvConfig {
    // ... 现有字段
    pub my_variable: String,
}

impl EnvConfig {
    pub fn from_env() -> EnvResult<Self> {
        Ok(Self {
            // ... 现有字段
            my_variable: my_module::MyVariable::get()?,
        })
    }
}
```

3. **添加测试**:

```rust
#[test]
fn test_my_variable() {
    env::set_var("MONOLITH_MY_VARIABLE", "test_value");
    assert_eq!(my_module::MyVariable::get().unwrap(), "test_value");
    env::remove_var("MONOLITH_MY_VARIABLE");
}
```

### 测试环境变量

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[test]
    fn test_environment_variables() {
        // 设置测试值
        env::set_var("MONOLITH_MODE", "development");
        env::set_var("MONOLITH_WEB_PORT", "8080");
        
        // 测试解析
        assert_eq!(core::Mode::get().unwrap(), "development");
        assert_eq!(web::Port::get().unwrap(), 8080);
        
        // 清理
        env::remove_var("MONOLITH_MODE");
        env::remove_var("MONOLITH_WEB_PORT");
    }
}
```

## 迁移指南

### 从手动env::var()迁移

**之前**:
```rust
let port = std::env::var("WEB_PORT")
    .unwrap_or_else(|_| "7080".to_string())
    .parse::<u16>()
    .unwrap_or(7080);
```

**之后**:
```rust
use monolith::env::{web, EnvVar};

let port = web::Port::get_or_default(7080);
```

### 从配置文件迁移

**之前**:
```rust
let config = load_config_file()?;
let port = config.web.port;
```

**之后**:
```rust
use monolith::env::web;

let port = web::Port::get()?; // 自动处理默认值和验证
```

## 故障排除

### 常见问题

1. **环境变量未生效**
   - 检查变量名是否正确（区分大小写）
   - 确认环境变量已正确设置
   - 重启应用程序

2. **类型转换失败**
   - 检查变量值格式是否正确
   - 查看错误信息了解具体要求
   - 使用合法的布尔值格式

3. **权限错误**
   - 检查端口是否需要特殊权限
   - 使用大于1024的端口号

### 调试工具

```rust
// 打印所有环境变量配置
let config = EnvConfig::from_env()?;
config.print_summary();

// 生成环境变量文档
let docs = monolith::env::generate_env_docs();
println!("{}", docs);
```

## 版本历史

- **v1.0**: 初始版本，基于翻译组件重构经验设计
- 支持核心、翻译、缓存、Web、MongoDB、性能和安全配置
- 提供类型安全的访问接口和详细的验证机制