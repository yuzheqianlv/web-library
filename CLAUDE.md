# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Monolith 轻量版是一个精简的 Rust 库和 CLI 工具，专注于通过书签脚本提供便捷的网页保存功能。项目已经过重大轻量化改造，移除复杂功能，专注于核心的书签脚本工作流程。

## 轻量化架构

### 核心组件

- **Core Processing**: `src/core.rs` - 主要文档处理逻辑，保持 `create_monolithic_document()` 核心功能
- **Environment Management**: `src/env.rs` - 简化的环境变量系统
- **Network Layer**: `src/network/` - HTTP 客户端、缓存、会话管理
- **Parsers**: `src/parsers/` - HTML、CSS、JavaScript 解析器
  - `src/parsers/html/` - HTML 处理和 DOM 操作
  - `src/parsers/link_rewriter.rs` - URL 重写和资源嵌入
- **Translation System** (可选): `src/translation/` - 简化的翻译功能
  - `src/translation/core/` - 翻译引擎核心
  - `src/translation/pipeline/` - 文本处理管道
- **轻量Web服务器**: `src/web/` - 基于 Axum 的精简 Web 界面
  - `src/web/handlers/pages.rs` - 主页和预览处理器
  - `src/web/handlers/api/` - 6个核心API端点
  - `src/web/routes.rs` - 简化的路由配置

### 执行模式

1. **CLI Tool** (`src/main.rs`): 命令行界面
2. **轻量Web服务器** (`src/web_main.rs`): 专注书签脚本的HTTP服务器

### 简化的功能标志

项目使用 Cargo features 实现可选功能：

- `cli`: 命令行工具 (默认)
- `web`: 轻量Web服务器 (移除了MongoDB依赖)
- `translation`: 可选的翻译功能
- `vendored-openssl`: 静态 OpenSSL 链接

### 已移除的组件

为实现轻量化，以下组件已被移除：
- ~~MongoDB 数据库支持~~
- ~~复杂的库管理系统 (`src/web/library/`)~~
- ~~批量翻译UI组件~~
- ~~V2 API系统~~
- ~~统计系统~~
- ~~复杂的前端CSS/JS组件~~

## 开发命令

### 构建

```bash
# 标准 CLI 构建
cargo build --features="cli"

# 轻量Web服务器
cargo build --bin monolith-web --features="web"

# 包含翻译功能的Web服务器
cargo build --bin monolith-web --features="web,translation"

# 发布版本构建
cargo build --release --features="web"
```

### 测试

```bash
# 运行所有测试
cargo test

# 运行特定功能的测试
cargo test --features="web"
cargo test --features="web,translation"

# 运行核心功能测试
cargo test core::
cargo test --test integration
```

### 代码质量

```bash
# 格式化代码
cargo fmt --all

# 代码检查
cargo clippy

# 清理构建产物
cargo clean
```

### 运行应用

```bash
# CLI 工具
cargo run --bin monolith --features="cli" -- <URL>

# 轻量Web服务器 (推荐)
cargo run --bin monolith-web --features="web"

# 自定义端口运行
cargo run --bin monolith-web --features="web" -- --port 8080

# 包含翻译功能的服务器
cargo run --bin monolith-web --features="web,translation"
```

## Key Dependencies

- **html5ever**: HTML parsing and DOM manipulation
- **reqwest**: HTTP client for network requests
- **axum**: Web framework (web feature)
# GUI framework druid removed
- **clap**: Command-line parsing (CLI feature)
- **markdown-translator**: Translation integration (translation feature)
- **mongodb**: Database support (web feature)

## Working with Features

Always use appropriate feature flags when building or testing specific functionality:

```bash
# For translation work
cargo build --features="translation"
cargo test --features="translation"

# For web development  
cargo build --features="web,translation"
cargo test --features="web"

# GUI functionality removed
```

## Translation System

When working with the translation functionality:

- Configuration files use TOML format with .env support
- Translation API integration in `src/translation/service.rs`
- DOM text extraction in `src/translation/collector.rs`
- Caching system in `src/translation/cache.rs`
- Batch processing in `src/translation/batch.rs`

## Web Interface

The web server provides:

- REST API endpoints in `src/web/handlers/api/`
- HTML templates in `templates/`
- Theme management system with CSS variables
- MongoDB integration for persistent storage
- Static asset serving

## Testing Structure

### Test Organization
- **Unit tests**: Inline `#[cfg(test)]` modules in source files
- **Integration tests**: `tests/` directory organized by functionality
- **Test data**: `tests/_data_/` contains realistic HTML, CSS, and JS test files
- **Feature-specific tests**: Use appropriate feature flags for testing optional functionality

### Test Data Categories
- `tests/_data_/basic/` - Basic HTML and resource files
- `tests/_data_/css/` - CSS parsing and embedding tests
- `tests/_data_/unusual_encodings/` - Character encoding tests (GB2312, ISO-8859-1)
- `tests/_data_/svg/` - SVG and vector graphics tests
- `tests/_data_/integrity/` - Resource integrity verification tests

### Test Modules
- `tests/cli/` - Command-line interface functionality
- `tests/core/` - Core processing logic tests
- `tests/html/` - HTML parsing and manipulation tests
- `tests/translation_pipeline.rs` - End-to-end translation functionality
- `tests/integration/translation/` - Translation system integration tests

## Error Handling

The codebase uses custom error types:
- `MonolithError` for core processing errors
- Translation errors in `src/translation/error.rs`
- Consistent error handling patterns throughout

## Code Style

- Uses rustfmt with configuration in `rustfmt.toml` (max_width=100, tab_spaces=4)
- Extensive documentation comments for public APIs
- Conditional compilation with `#[cfg(feature = "...")]`
- Follows standard Rust naming conventions
- Chinese comments preferred (使用中文编写注释)

## Project Structure

### Binary Targets
- `monolith` (CLI): Requires `cli` feature, entry point at `src/main.rs`
- `monolith-web` (Web server): Requires `web` feature, entry point at `src/web_main.rs`

### Environment Configuration
- `cargo/config.toml` - Cargo build configuration (OPENSSL_NO_VENDOR=1)
- `rustfmt.toml` - Code formatting rules
- `Makefile` - Development automation scripts

### Important Files
- `src/CLAUDE.md` - Rust coding standards and project guidelines
- `src/translation_legacy.rs` - Legacy translation support for backward compatibility

## Development Environment

### Prerequisites
- Rust 1.70+ (edition 2021)
- For web features: MongoDB (optional, for persistent storage)
- For translation features: External translation API access

### Environment Variables
The project supports extensive environment configuration through `src/env.rs`:
- Translation API keys and endpoints
- Web server configuration (host, port, theme)
- Cache settings and timeouts
- Debug and logging options

### OpenSSL Configuration
- Uses system OpenSSL by default (`OPENSSL_NO_VENDOR=1`)
- Use `vendored-openssl` feature for static linking when needed