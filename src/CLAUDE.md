# CLAUDE.md - Rust项目编码规范

> **项目名称**: [项目名称]
> **版本**: 1.0
> **更新日期**: 2025-07-20
> **维护团队**: [团队名称]

---

## 目录

- [1. 项目结构规范](#1-项目结构规范)
- [2. 代码格式规范](#2-代码格式规范)
- [3. 命名约定](#3-命名约定)
- [4. 错误处理规范](#4-错误处理规范)
- [5. 文档和注释规范](#5-文档和注释规范)
- [6. 性能和安全实践](#6-性能和安全实践)
- [7. 测试规范](#7-测试规范)
- [8. 依赖管理规范](#8-依赖管理规范)
- [9. Git提交规范](#9-git提交规范)
- [10. 代码审查检查清单](#10-代码审查检查清单)

---

## 1. 项目结构规范

### 1.1 标准项目结构

```
project_name/
├── Cargo.toml                 # 项目配置
├── Cargo.lock                 # 依赖锁定文件
├── README.md                  # 项目说明
├── LICENSE                    # 许可证
├── .gitignore                 # Git忽略文件
├── .github/                   # GitHub配置
│   └── workflows/             # CI/CD工作流
├── src/                       # 源代码
│   ├── lib.rs                 # 库入口
│   ├── main.rs                # 可执行程序入口
│   ├── bin/                   # 多个可执行程序
│   ├── modules/               # 模块目录
│   └── utils/                 # 工具函数
├── tests/                     # 集成测试
├── benches/                   # 基准测试
├── examples/                  # 示例代码
├── docs/                      # 项目文档
└── assets/                    # 资源文件
```

### 1.2 模块组织原则

- **单一职责**: 每个模块专注于一个功能领域
- **松耦合**: 减少模块间的依赖关系
- **高内聚**: 相关功能放在同一模块内
- **清晰接口**: 明确定义公共API

### 1.3 文件命名规范

| 文件类型 | 命名规范 | 示例 |
|---------|---------|------|
| Rust源文件 | ```snake_case.rs``` | ```user_service.rs``` |
| 测试文件 | ```snake_case_test.rs``` | ```user_service_test.rs``` |
| 模块目录 | ```snake_case/``` | ```auth_module/``` |
| 配置文件 | ```kebab-case``` | ```rust-toolchain.toml``` |

---

## 2. 代码格式规范

### 2.1 基本格式设置

```toml
# rustfmt.toml
edition = "2021"
max_width = 100
tab_spaces = 4
newline_style = "Unix"
use_small_heuristics = "Default"
reorder_imports = true
reorder_modules = true
remove_nested_parens = true
```

### 2.2 缩进和空格

- **缩进**: 使用4个空格，禁止使用制表符
- **行宽**: 最大100字符
- **尾随逗号**: 多行结构使用尾随逗号

```rust
// ✅ 正确
let config = Config {
    host: "localhost".to_string(),
    port: 8080,
    timeout: Duration::from_secs(30),  // 尾随逗号
};

// ❌ 错误
let config = Config {
        host: "localhost".to_string(),  // 错误的缩进
        port: 8080,
        timeout: Duration::from_secs(30)   // 缺少尾随逗号
    };
```

### 2.3 空行使用规范

```rust
// ✅ 正确的空行使用
use std::collections::HashMap;
use std::fs::File;

use serde::{Deserialize, Serialize};

const MAX_RETRIES: u32 = 3;

#[derive(Debug, Clone)]
pub struct User {
    pub id: u64,
    pub name: String,
}

impl User {
    pub fn new(id: u64, name: String) -> Self {
        Self { id, name }
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        // 实现逻辑
    }
}
```

---

## 3. 命名约定

### 3.1 命名规范表

| 构造类型 | 约定 | 示例 |
|---------|------|------|
| 变量 | ```snake_case``` | ```user_count```, ```is_valid``` |
| 函数 | ```snake_case``` | ```calculate_tax()```, ```send_email()``` |
| 结构体 | ```PascalCase``` | ```UserAccount```, ```HttpResponse``` |
| 枚举 | ```PascalCase``` | ```Status```, ```ErrorKind``` |
| 特征 | ```PascalCase``` | ```Serialize```, ```Clone``` |
| 常量 | ```SCREAMING_SNAKE_CASE``` | ```MAX_CONNECTIONS```, ```API_VERSION``` |
| 静态变量 | ```SCREAMING_SNAKE_CASE``` | ```GLOBAL_CONFIG``` |
| 模块 | ```snake_case``` | ```user_service```, ```http_client``` |
| 宏 | ```snake_case``` | ```assert_eq!```, ```println!``` |
| 生命周期 | ```'lowercase``` | ```'a```, ```'static``` |
| 泛型参数 | ```PascalCase``` | ```T```, ```TResult```, ```TError``` |

### 3.2 命名最佳实践

#### 布尔值命名

```rust
// ✅ 推荐
is_valid: bool
has_permission: bool
can_execute: bool
should_retry: bool

// ❌ 不推荐
valid: bool
permission: bool
execute: bool
retry: bool
```

#### 集合命名

```rust
// ✅ 推荐 - 使用复数形式
users: Vec<User>
error_messages: HashMap<String, String>
active_connections: HashSet<ConnectionId>

// ❌ 不推荐
user_list: Vec<User>
error_message_map: HashMap<String, String>
```

#### 函数命名

```rust
// ✅ 推荐 - 动词开头，描述行为
pub fn calculate_discount(price: f64, rate: f64) -> f64 { }
pub fn validate_email(email: &str) -> Result<(), ValidationError> { }
pub fn send_notification(user: &User, message: &str) -> Result<(), NotificationError> { }

// ❌ 不推荐
pub fn discount(price: f64, rate: f64) -> f64 { }
pub fn email_validation(email: &str) -> Result<(), ValidationError> { }
```

---

## 4. 错误处理规范

### 4.1 错误处理原则

1. **使用类型系统**: 利用```Result```和```Option```处理错误
2. **尽早返回**: 在错误发生点立即处理
3. **明确错误类型**: 定义具体的错误类型
4. **避免panic**: 生产代码中避免使用```unwrap()```和```expect()```

### 4.2 自定义错误类型

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UserError {
    #[error("User not found: {id}")]
    NotFound { id: u64 },

    #[error("Invalid email format: {email}")]
    InvalidEmail { email: String },

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

pub type UserResult<T> = Result<T, UserError>;
```

### 4.3 错误处理模式

#### 使用?操作符

```rust
// ✅ 推荐
pub fn get_user_profile(id: u64) -> UserResult<UserProfile> {
    let user = database::get_user(id)?;
    let profile = ProfileService::fetch_profile(&user)?;
    Ok(profile)
}

// ❌ 不推荐
pub fn get_user_profile(id: u64) -> UserResult<UserProfile> {
    match database::get_user(id) {
        Ok(user) => {
            match ProfileService::fetch_profile(&user) {
                Ok(profile) => Ok(profile),
                Err(e) => Err(e.into()),
            }
        }
        Err(e) => Err(e.into()),
    }
}
```

#### 上下文错误处理

```rust
use anyhow::{Context, Result};

pub fn process_config_file(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;

    Ok(config)
}
```

### 4.4 日志记录

```rust
use tracing::{error, warn, info, debug};

pub fn handle_request(request: Request) -> Result<Response, ServerError> {
    debug!("Processing request: {}", request.id());

    match process_request(request) {
        Ok(response) => {
            info!("Request processed successfully: {}", response.status());
            Ok(response)
        }
        Err(e) => {
            error!("Request processing failed: {}", e);
            Err(e)
        }
    }
}
```

---

## 5. 文档和注释规范

### 5.1 文档注释

```rust
/// 计算两个数字的最大公约数
///
/// # Arguments
///
/// * `a` - 第一个数字
/// * `b` - 第二个数字
///
/// # Returns
///
/// 返回两个数字的最大公约数
///
/// # Examples
///
/// ```
/// use mylib::math::gcd;
///
/// assert_eq!(gcd(48, 18), 6);
/// assert_eq!(gcd(17, 13), 1);
/// ```
///
/// # Panics
///
/// 当两个参数都为0时会panic
pub fn gcd(mut a: u64, mut b: u64) -> u64 {
    assert!(a != 0 || b != 0, "Both arguments cannot be zero");

    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}
```

### 5.2 模块文档

```rust
//! # 用户管理模块
//!
//! 这个模块提供了用户管理的核心功能，包括：
//!
//! - 用户注册和认证
//! - 用户信息管理
//! - 权限控制
//!
//! # 基本用法
//!
//! ```rust
//! use user_management::{User, UserService};
//!
//! let service = UserService::new();
//! let user = service.create_user("john@example.com", "password123")?;
//! ```

use crate::auth::AuthService;
use crate::database::Database;
```

### 5.3 代码注释规范

```rust
pub struct ConnectionPool {
    // 活跃连接列表
    active_connections: Vec<Connection>,
    // 最大连接数限制
    max_connections: usize,
    // 连接超时设置（秒）
    timeout: Duration,
}

impl ConnectionPool {
    pub fn get_connection(&mut self) -> Result<Connection, PoolError> {
        // 检查是否有可用连接
        if let Some(conn) = self.find_available_connection() {
            return Ok(conn);
        }

        // 如果没有可用连接且未达到最大限制，创建新连接
        if self.active_connections.len() < self.max_connections {
            self.create_new_connection()
        } else {
            // 等待连接可用或超时
            self.wait_for_connection()
        }
    }
}
```

---

## 6. 性能和安全实践

### 6.1 内存管理

#### 避免不必要的克隆

```rust
// ✅ 推荐 - 使用借用
fn process_data(data: &[u8]) -> ProcessResult {
    // 处理数据而不获取所有权
}

// ❌ 不推荐 - 不必要的克隆
fn process_data(data: Vec<u8>) -> ProcessResult {
    // 强制获取所有权，可能导致不必要的内存复制
}
```

#### 合理使用字符串类型

```rust
// ✅ 推荐 - 根据使用场景选择类型
fn format_message(template: &str, name: &str) -> String {
    format!("Hello, {}! {}", name, template)
}

fn log_message(message: &str) {
    println!("{}", message);
}

// ❌ 不推荐 - 过度使用String
fn log_message(message: String) {
    println!("{}", message);
}
```

### 6.2 并发安全

```rust
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::Semaphore;

// ✅ 推荐 - 读写锁用于读多写少场景
pub struct UserCache {
    users: Arc<RwLock<HashMap<u64, User>>>,
}

impl UserCache {
    pub async fn get_user(&self, id: u64) -> Option<User> {
        let users = self.users.read().unwrap();
        users.get(&id).cloned()
    }

    pub async fn update_user(&self, id: u64, user: User) {
        let mut users = self.users.write().unwrap();
        users.insert(id, user);
    }
}

// ✅ 推荐 - 使用信号量控制并发数
pub struct ApiClient {
    client: reqwest::Client,
    semaphore: Arc<Semaphore>,
}

impl ApiClient {
    pub async fn make_request(&self, url: &str) -> Result<Response, ApiError> {
        let _permit = self.semaphore.acquire().await?;
        let response = self.client.get(url).send().await?;
        Ok(response)
    }
}
```

### 6.3 unsafe代码使用

```rust
// ✅ 推荐 - 明确文档化unsafe代码
/// # Safety
///
/// 调用者必须保证：
/// - `ptr` 是有效的内存地址
/// - `len` 不超过实际分配的内存大小
/// - 在函数执行期间，没有其他代码修改这块内存
unsafe fn read_memory(ptr: *const u8, len: usize) -> Vec<u8> {
    std::slice::from_raw_parts(ptr, len).to_vec()
}

// ✅ 推荐 - 封装unsafe操作
pub struct SafeBuffer {
    ptr: *mut u8,
    len: usize,
    capacity: usize,
}

impl SafeBuffer {
    pub fn write(&mut self, offset: usize, data: &[u8]) -> Result<(), BufferError> {
        if offset + data.len() > self.capacity {
            return Err(BufferError::OutOfBounds);
        }

        unsafe {
            std::ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.ptr.add(offset),
                data.len(),
            );
        }
        Ok(())
    }
}
```

---

## 7. 测试规范

### 7.1 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_discount_valid_input() {
        let result = calculate_discount(100.0, 0.1);
        assert_eq!(result, 10.0);
    }

    #[test]
    fn test_calculate_discount_zero_rate() {
        let result = calculate_discount(100.0, 0.0);
        assert_eq!(result, 0.0);
    }

    #[test]
    #[should_panic(expected = "Rate cannot be negative")]
    fn test_calculate_discount_negative_rate() {
        calculate_discount(100.0, -0.1);
    }

    #[tokio::test]
    async fn test_async_user_creation() {
        let service = UserService::new();
        let user = service.create_user("test@example.com", "password123").await.unwrap();
        assert_eq!(user.email, "test@example.com");
    }
}
```

### 7.2 集成测试

```rust
// tests/integration_test.rs
use myapp::{App, Config};
use tokio_test;

#[tokio::test]
async fn test_full_user_workflow() {
    let config = Config::test_default();
    let app = App::new(config).await.expect("Failed to create app");

    // 测试用户注册
    let user_data = json!({
        "email": "integration@test.com",
        "password": "test123"
    });

    let response = app.register_user(user_data).await.unwrap();
    assert_eq!(response.status, "success");

    // 测试用户登录
    let login_data = json!({
        "email": "integration@test.com",
        "password": "test123"
    });

    let auth_response = app.login_user(login_data).await.unwrap();
    assert!(auth_response.token.is_some());
}
```

### 7.3 测试辅助工具

```rust
// 测试数据构建器
pub struct UserBuilder {
    id: Option<u64>,
    email: Option<String>,
    name: Option<String>,
}

impl UserBuilder {
    pub fn new() -> Self {
        Self {
            id: None,
            email: None,
            name: None,
        }
    }

    pub fn with_id(mut self, id: u64) -> Self {
        self.id = Some(id);
        self
    }

    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    pub fn build(self) -> User {
        User {
            id: self.id.unwrap_or(1),
            email: self.email.unwrap_or_else(|| "test@example.com".to_string()),
            name: self.name.unwrap_or_else(|| "Test User".to_string()),
        }
    }
}

// 使用示例
#[test]
fn test_with_builder() {
    let user = UserBuilder::new()
        .with_id(123)
        .with_email("custom@test.com")
        .build();

    assert_eq!(user.id, 123);
    assert_eq!(user.email, "custom@test.com");
}
```

---

## 8. 依赖管理规范

### 8.1 Cargo.toml配置

```toml
[package]
name = "project-name"
version = "0.1.0"
authors = ["Your Name <your.email@example.com>"]
edition = "2021"
rust-version = "1.70"
description = "A brief description of your project"
license = "MIT OR Apache-2.0"
repository = "https://github.com/username/project-name"
homepage = "https://github.com/username/project-name"
documentation = "https://docs.rs/project-name"
readme = "README.md"
keywords = ["keyword1", "keyword2"]
categories = ["category1", "category2"]

[dependencies]
# 核心依赖
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"

# 可选依赖
redis = { version = "0.23", optional = true }
postgres = { version = "0.19", optional = true }

[dev-dependencies]
tokio-test = "0.4"
mockall = "0.11"
tempfile = "3.0"

[features]
default = []
redis-support = ["redis"]
postgres-support = ["postgres"]
full = ["redis-support", "postgres-support"]

[[bin]]
name = "server"
path = "src/bin/server.rs"

[[example]]
name = "basic_usage"
path = "examples/basic_usage.rs"
```

### 8.2 依赖选择原则

| 依赖类型 | 选择原则 | 推荐库 |
|---------|---------|--------|
| 序列化 | 性能好、功能全 | ```serde``` |
| 异步运行时 | 生态丰富、稳定 | ```tokio``` |
| 错误处理 | 易用、信息丰富 | ```anyhow```, ```thiserror``` |
| 日志 | 结构化、高性能 | ```tracing``` |
| HTTP客户端 | 异步、功能全 | ```reqwest``` |
| 数据库 | 类型安全、异步 | ```sqlx``` |
| 测试工具 | 表达力强、维护好 | ```mockall```, ```proptest``` |

### 8.3 版本管理策略

```toml
# ✅ 推荐 - 使用具体版本范围
serde = "1.0.136"              # 只接受补丁更新
tokio = "~1.20.0"              # 接受1.20.x版本
reqwest = "^0.11.12"           # 接受0.11.x版本

# ❌ 不推荐 - 过于宽松的版本要求
serde = "*"                    # 接受任何版本
tokio = ">=1.0"               # 可能引入破坏性变更
```

---

## 9. Git提交规范

### 9.1 提交信息格式

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### 9.2 提交类型

| 类型 | 描述 | 示例 |
|------|------|------|
| ```feat``` | 新功能 | ```feat(auth): add JWT token validation``` |
| ```fix``` | Bug修复 | ```fix(database): resolve connection pool leak``` |
| ```docs``` | 文档更新 | ```docs(api): update endpoint documentation``` |
| ```style``` | 代码格式 | ```style: run rustfmt on all files``` |
| ```refactor``` | 重构 | ```refactor(user): simplify user creation logic``` |
| ```perf``` | 性能优化 | ```perf(query): optimize database query performance``` |
| ```test``` | 测试 | ```test(user): add integration tests for user service``` |
| ```chore``` | 构建/工具 | ```chore(deps): update tokio to version 1.21``` |

### 9.3 分支命名规范

```
# 功能分支
feature/user-authentication
feature/payment-integration

# 修复分支
fix/memory-leak-in-worker
hotfix/critical-security-vulnerability

# 发布分支
release/v1.2.0

# 实验分支
experiment/new-caching-strategy
```

### 9.4 Pull Request模板

```markdown
## 变更描述
简要描述本次变更的内容和目的。

## 变更类型
- [ ] 新功能 (feature)
- [ ] Bug修复 (fix)
- [ ] 破坏性变更 (breaking change)
- [ ] 文档更新 (docs)
- [ ] 性能优化 (perf)
- [ ] 重构 (refactor)
- [ ] 测试 (test)
- [ ] 构建/工具 (chore)

## 测试
- [ ] 已添加单元测试
- [ ] 已添加集成测试
- [ ] 手动测试通过
- [ ] CI/CD管道通过

## 检查清单
- [ ] 代码遵循项目编码规范
- [ ] 自测试通过
- [ ] 已添加必要的文档
- [ ] 已更新CHANGELOG.md
- [ ] 没有遗留的TODO或FIXME注释
```

---

## 10. 代码审查检查清单

### 10.1 代码质量检查

#### 基础检查
- [ ] 代码编译通过，无警告
- [ ] 所有测试通过
- [ ] ```cargo clippy```检查通过
- [ ] ```cargo fmt```格式化完成
- [ ] 无未使用的导入和变量

#### 设计检查
- [ ] 代码结构清晰，职责分离
- [ ] 函数长度适中（一般不超过50行）
- [ ] 避免深度嵌套（一般不超过4层）
- [ ] 合理使用设计模式
- [ ] 接口设计简洁明了

#### 错误处理检查
- [ ] 所有可能的错误都得到适当处理
- [ ] 避免使用```unwrap()```和```expect()```在生产代码中
- [ ] 错误信息清晰有用
- [ ] 使用适当的错误类型

#### 性能检查
- [ ] 避免不必要的内存分配
- [ ] 合理使用借用而非所有权转移
- [ ] 避免不必要的克隆操作
- [ ] 考虑使用迭代器替代循环

#### 安全检查
- [ ] ```unsafe```代码有充分的安全注释
- [ ] 输入验证完善
- [ ] 敏感信息不在日志中泄露
- [ ] 并发安全考虑充分

### 10.2 文档检查

- [ ] 公共API有完整的文档注释
- [ ] 复杂算法有必要的解释
- [ ] 示例代码可以运行
- [ ] README.md信息准确完整

### 10.3 测试检查

- [ ] 核心功能有单元测试覆盖
- [ ] 边界条件有测试
- [ ] 错误情况有测试
- [ ] 测试命名清晰描述测试意图
- [ ] 无忽略的测试（除非有充分理由）

### 10.4 依赖检查

- [ ] 新依赖是必要的
- [ ] 依赖版本合理
- [ ] 许可证兼容
- [ ] 依赖来源可信

---

## 附录

### A. 推荐工具配置

#### VS Code配置 (settings.json)

```json
{
    "rust-analyzer.check.command": "clippy",
    "rust-analyzer.cargo.features": "all",
    "editor.formatOnSave": true,
    "[rust]": {
        "editor.defaultFormatter": "rust-lang.rust-analyzer",
        "editor.tabSize": 4,
        "editor.insertSpaces": true
    },
    "files.exclude": {
        "**/target": true,
        "**/Cargo.lock": true
    }
}
```

#### Pre-commit配置 (.pre-commit-config.yaml)

```yaml
repos:
  - repo: local
    hooks:
      - id: cargo-check
        name: cargo check
        entry: cargo check
        language: system
        files: \.rs$
        pass_filenames: false

      - id: cargo-test
        name: cargo test
        entry: cargo test
        language: system
        files: \.rs$
        pass_filenames: false

      - id: cargo-fmt
        name: cargo fmt
        entry: cargo fmt -- --check
        language: system
        files: \.rs$
        pass_filenames: false

      - id: cargo-clippy
        name: cargo clippy
        entry: cargo clippy -- -D warnings
        language: system
        files: \.rs$
        pass_filenames: false
```

### B. 持续集成配置

#### GitHub Actions (.github/workflows/ci.yml)

```yaml
name: CI

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Test
    runs-on: ubuntu-latest
    strategy:
      matrix:
        rust: [stable, beta, nightly]
    steps:
    - uses: actions/checkout@v3
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: ${{ matrix.rust }}
        override: true
        components: rustfmt, clippy

    - name: Cache cargo registry
      uses: actions/cache@v3
      with:
        path: ~/.cargo/registry
        key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

    - name: Cache cargo index
      uses: actions/cache@v3
      with:
        path: ~/.cargo/git
        key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}

    - name: Cache cargo build
      uses: actions/cache@v3
      with:
        path: target
        key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}

    - name: Check formatting
      run: cargo fmt -- --check

    - name: Run clippy
      run: cargo clippy -- -D warnings

    - name: Run tests
      run: cargo test --verbose

    - name: Run doc tests
      run: cargo test --doc
```

---

**文档维护**: 本规范应定期审查和更新，以反映团队的最佳实践和Rust生态系统的发展。

**问题反馈**: 如有疑问或建议，请提交Issue或联系维护团队。
