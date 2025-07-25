# Monolith

[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)
[![License: CC0-1.0](https://img.shields.io/badge/License-CC0%201.0-lightgrey.svg)](http://creativecommons.org/publicdomain/zero/1.0/)
[![Version](https://img.shields.io/badge/version-2.11.0-green.svg)](https://github.com/Y2Z/monolith)

一个强大的 CLI 工具和 Rust 库，用于将网页保存为单一的自包含 HTML 文件，包含所有资源（图片、CSS、JavaScript 等）。支持智能翻译、Web 服务器模式和高度可配置的处理选项。

## ✨ 特性

### 🚀 核心功能
- **完整页面保存** - 将网页及其所有资源嵌入到单个 HTML 文件中
- **资源内联** - 自动处理图片、CSS、JavaScript、字体等外部资源
- **智能解析** - 支持复杂的 HTML5、CSS3 和现代 JavaScript
- **网络优化** - 内置缓存和会话管理，支持 Cookie 和认证

### 🌍 翻译功能 (可选)
- **智能文本识别** - 自动识别可翻译内容，过滤代码和特殊元素
- **批量处理** - 高效的批次翻译，支持优先级排序
- **多层缓存** - 本地 LRU 缓存 + MongoDB 持久化存储
- **并发安全** - 支持多线程并发翻译处理
- **API 集成** - 集成主流翻译服务提供商

### 🌐 Web 服务器模式 (可选)
- **REST API** - 完整的 HTTP API 接口
- **Web 界面** - 现代化的 Web 管理界面
- **主题系统** - 可定制的界面主题
- **库管理** - 保存和管理处理过的页面

### ⚙️ 高级配置
- **环境变量管理** - 类型安全的配置系统，支持 42+ 配置项
- **功能标志** - 模块化架构，按需启用功能
- **性能优化** - 智能批处理和内存管理
- **错误处理** - 完善的错误处理和恢复机制

## 📦 安装

### 从源码构建

```bash
# 克隆项目
git clone https://github.com/Y2Z/monolith.git
cd monolith

# 基础 CLI 工具
cargo build --release --features="cli"

# 包含翻译功能
cargo build --release --features="cli,translation"

# 完整功能 (CLI + Web + 翻译)
cargo build --release --features="cli,web,translation"
```

### 使用 Cargo

```bash
# 安装 CLI 工具
cargo install monolith --features="cli"

# 或安装完整版本
cargo install monolith --features="cli,web,translation"
```

## 🚀 快速开始

### 基础使用

```bash
# 保存网页为单个 HTML 文件
monolith https://example.com > page.html

# 保存并禁用 JavaScript
monolith --no-js https://example.com > page.html

# 保存远程图片为 data URLs
monolith --no-images https://example.com > page.html
```

### 翻译功能

```bash
# 保存并翻译页面内容
monolith --translate https://example.com > translated_page.html

# 指定翻译服务
MONOLITH_TRANSLATION_SERVICE=openai monolith --translate https://example.com > page.html
```

### Web 服务器模式

```bash
# 启动 Web 服务器
cargo run --bin monolith-web --features="web"

# 自定义端口和配置
MONOLITH_WEB_PORT=8080 cargo run --bin monolith-web --features="web"
```

## 🏗️ 项目架构

```
monolith/
├── src/
│   ├── core.rs              # 核心处理逻辑
│   ├── env.rs               # 环境变量管理
│   ├── parsers/             # 资源解析器
│   │   ├── html/            # HTML 解析和处理
│   │   ├── css.rs           # CSS 解析
│   │   └── js.rs            # JavaScript 处理
│   ├── network/             # 网络层
│   │   ├── cache.rs         # HTTP 缓存
│   │   ├── session.rs       # 会话管理
│   │   └── cookies.rs       # Cookie 处理
│   ├── translation/         # 翻译模块 (可选)
│   │   ├── core/            # 翻译引擎
│   │   ├── pipeline/        # 处理管道
│   │   ├── storage/         # 缓存存储
│   │   └── config/          # 配置管理
│   └── web/                 # Web 服务器 (可选)
│       ├── handlers/        # HTTP 处理器
│       ├── templates/       # HTML 模板
│       └── library/         # 页面库管理
├── tests/                   # 测试套件
├── templates/               # Web 界面模板
└── docs/                    # 项目文档
```

## 🛠️ 配置

### 环境变量

项目支持 40+ 环境变量配置，分为以下模块：

```bash
# 核心功能
MONOLITH_OUTPUT_FORMAT=html
MONOLITH_USER_AGENT="Custom Agent"
MONOLITH_TIMEOUT=30

# 翻译功能
MONOLITH_TRANSLATION_ENABLED=true
MONOLITH_TRANSLATION_SERVICE=openai
MONOLITH_TRANSLATION_API_KEY=your_key_here
MONOLITH_TRANSLATION_SOURCE_LANG=en
MONOLITH_TRANSLATION_TARGET_LANG=zh

# Web 服务器
MONOLITH_WEB_HOST=127.0.0.1
MONOLITH_WEB_PORT=3000
MONOLITH_WEB_THEME=dark

# 缓存配置
MONOLITH_CACHE_ENABLED=true
MONOLITH_CACHE_SIZE=1000
MONOLITH_CACHE_TTL=3600
```

### 配置文件

支持 TOML 格式的配置文件：

```toml
# config.toml
[core]
timeout = 30
user_agent = "Monolith Bot"
output_format = "html"

[translation]
enabled = true
service = "openai"
source_lang = "en" 
target_lang = "zh"
batch_size = 50

[web]
host = "0.0.0.0"
port = 3000
theme = "auto"

[cache]
enabled = true
size = 1000
ttl = 3600
```

## 📚 API 使用

### Rust 库

```rust
use monolith::{create_monolithic_document, MonolithOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = MonolithOptions {
        target_url: "https://example.com".to_string(),
        no_css: false,
        no_fonts: false,
        no_frames: false,
        no_images: false,
        no_js: false,
        insecure: false,
        isolate: false,
        output: None,
        silent: false,
        timeout: 60,
        user_agent: "Monolith".to_string(),
        ..Default::default()
    };

    let html = create_monolithic_document(&options).await?;
    println!("{}", html);
    Ok(())
}
```

### 翻译功能

```rust
use monolith::translation::{TranslationEngine, TranslationConfig};

#[tokio::main] 
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = TranslationConfig::default();
    let engine = TranslationEngine::new(config);
    
    let translated_html = engine.translate_html(
        "<p>Hello, world!</p>",
        "en",
        "zh"
    ).await?;
    
    println!("{}", translated_html);
    Ok(())
}
```

### Web API

```bash
# 保存页面
curl -X POST http://localhost:3000/api/process \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com", "translate": true}'

# 获取保存的页面列表
curl http://localhost:3000/api/library

# 获取特定页面
curl http://localhost:3000/api/library/{id}
```

## 🧪 测试

```bash
# 运行所有测试
make test

# 运行特定功能的测试
cargo test --features="translation"
cargo test --features="web"

# 运行集成测试
cargo test --test integration

# 性能基准测试
cargo bench
```

## 🤝 开发

### 开发环境设置

```bash
# 克隆项目
git clone https://github.com/Y2Z/monolith.git
cd monolith

# 安装开发依赖
cargo build --all-features

# 运行开发服务器 (需要 MongoDB)
docker-compose up -d mongodb
cargo run --bin monolith-web --features="web,translation"
```

### 代码质量

```bash
# 格式化代码
make format

# 运行 Clippy 检查
make lint

# 检查格式
make format-check

# 运行所有检查
make check
```

### 贡献指南

1. Fork 项目并创建功能分支
2. 编写测试覆盖新功能
3. 确保所有测试通过
4. 遵循项目的代码风格
5. 提交 Pull Request

## 📈 性能

### 基准测试结果

- **页面处理速度**: ~500ms/页面 (普通页面)
- **翻译处理**: ~50ms/句子 (批量处理)
- **内存使用**: <100MB (大部分场景)
- **并发支持**: 1000+ 同时连接 (Web 模式)

### 优化建议

- 启用缓存以提高重复访问性能
- 使用批量翻译减少 API 调用
- 调整并发限制以平衡性能和资源使用
- 对大型页面启用流式处理

## 🗺️ 路线图

### v2.12.0 (计划中)
- [ ] 支持更多翻译服务提供商
- [ ] WebAssembly 支持
- [ ] 插件系统
- [ ] 增强的错误处理

### v2.13.0 (计划中)
- [ ] 实时协作功能
- [ ] 高级页面分析
- [ ] 自定义处理规则
- [ ] 性能监控面板

## 📄 许可证

本项目采用 [CC0 1.0 Universal](LICENSE) 许可证 - 详见 LICENSE 文件。

## 🙏 致谢

感谢所有贡献者和以下开源项目：

- [html5ever](https://github.com/servo/html5ever) - HTML 解析
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP 客户端
- [tokio](https://github.com/tokio-rs/tokio) - 异步运行时
- [axum](https://github.com/tokio-rs/axum) - Web 框架
- [MongoDB](https://www.mongodb.com/) - 数据存储

## 📞 支持

- 📖 [文档](docs/)
- 🐛 [问题报告](https://github.com/Y2Z/monolith/issues)
- 💬 [讨论](https://github.com/Y2Z/monolith/discussions)
- 📧 联系: snshn@tutanota.com

---

**Monolith** - 让网页保存变得简单、智能、强大！ 🚀