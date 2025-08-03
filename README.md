# Monolith - 轻量版

[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)
[![License: CC0-1.0](https://img.shields.io/badge/License-CC0%201.0-lightgrey.svg)](http://creativecommons.org/publicdomain/zero/1.0/)
[![Version](https://img.shields.io/badge/version-2.11.0--lightweight-green.svg)](https://github.com/Y2Z/monolith)

一个精简、快速的网页处理工具，专注于通过书签脚本提供便捷的网页保存和预览功能。

## ✨ 核心特性

### 🚀 简洁高效
- **一键处理** - 通过书签脚本直接处理任意网页
- **资源内嵌** - 自动处理图片、CSS、JavaScript等外部资源
- **即时预览** - 快速预览处理后的单文件HTML
- **轻量设计** - 移除复杂功能，专注核心体验

### 📖 书签脚本工作流
1. 访问轻量化主页
2. 拖拽书签脚本到浏览器书签栏
3. 在任意网页点击书签一键处理
4. 获得完整的单文件HTML结果

### 🌍 可选翻译功能
- **智能翻译** - 可选启用翻译功能处理多语言内容
- **简化API** - 精简的翻译接口，专注实用性

## 📦 安装与构建

### 从源码构建

```bash
# 克隆项目
git clone https://github.com/Y2Z/monolith.git
cd monolith

# 基础 CLI 工具
cargo build --release --features="cli"

# 轻量Web服务器（推荐）
cargo build --release --features="web"

# 包含翻译功能的Web服务器
cargo build --release --features="web,translation"
```

## 🚀 快速开始

### CLI 使用

```bash
# 保存网页为单个 HTML 文件
monolith https://example.com > page.html

# 保存并禁用特定资源
monolith --no-js https://example.com > page.html
```

### Web 服务器模式（推荐）

```bash
# 启动轻量Web服务器
cargo run --bin monolith-web --features="web"

# 自定义端口
cargo run --bin monolith-web --features="web" -- --port 8080

# 访问 http://localhost:7080 获取书签脚本
```

### 书签脚本使用

1. **获取书签脚本**：访问 `http://localhost:7080`
2. **安装书签**：将页面上的"Monolith 处理器"按钮拖拽到浏览器书签栏
3. **一键使用**：在任意网页点击书签，自动跳转到处理结果页面

## 🏗️ 轻量化架构

```
monolith/
├── src/
│   ├── core.rs              # 核心处理逻辑
│   ├── parsers/             # 资源解析器
│   │   ├── html/            # HTML 解析和处理
│   │   ├── css.rs           # CSS 解析
│   │   └── js.rs            # JavaScript 处理
│   ├── network/             # 网络层
│   ├── translation/         # 翻译模块 (可选)
│   └── web/                 # 轻量Web服务器
│       ├── handlers/        # 简化的HTTP处理器
│       │   ├── pages.rs     # 主页和预览处理
│       │   └── api/         # 核心API端点
│       └── routes.rs        # 6个核心路由
├── templates/
│   └── core/
│       └── simple_index.html # 简化主页
└── tests/                   # 测试套件
```

## 🛠️ 配置

### 基础环境变量

```bash
# Web 服务器配置
MONOLITH_WEB_HOST=127.0.0.1
MONOLITH_WEB_PORT=7080

# 翻译功能 (可选)
MONOLITH_TRANSLATION_ENABLED=true
MONOLITH_TRANSLATION_SERVICE=openai
MONOLITH_TRANSLATION_API_KEY=your_key_here
```

## 📚 API 端点

### 轻量化路由

```
GET  /                    # 主页 - 书签脚本获取页面
GET  /preview/{url}       # 直接预览处理网页
POST /api/translate       # 翻译API
POST /api/content         # 内容处理API  
GET  /api/bookmarklet     # 书签脚本生成
POST /api/cache/clear     # 缓存清理
```

### 使用示例

```bash
# 直接预览网页
curl http://localhost:7080/preview/https://example.com

# 获取书签脚本
curl http://localhost:7080/api/bookmarklet
```

## 🧪 测试

```bash
# 运行所有测试
cargo test

# 测试Web功能
cargo test --features="web"

# 测试翻译功能
cargo test --features="web,translation"
```

## 🎯 轻量化优势

### 性能提升
- **路由数量减少87%** - 从47个路由精简到6个核心路由
- **启动速度更快** - 移除MongoDB等重型依赖
- **内存占用更少** - 精简的状态管理和缓存策略

### 用户体验
- **操作简化** - 一键书签脚本工作流
- **界面清爽** - 专注功能的简洁设计
- **即开即用** - 无需复杂配置即可使用

### 维护便利
- **代码精简** - 移除复杂的库管理和统计系统
- **依赖减少** - 精简的依赖树
- **部署轻量** - 更小的二进制文件和更少的资源文件

## 📈 性能基准

- **页面处理速度**: ~300ms/页面 (轻量化优化)
- **内存使用**: <50MB (大部分场景)
- **启动时间**: <1秒
- **二进制大小**: 显著减小

## 🗺️ 发展方向

### 持续优化
- [ ] 进一步精简依赖
- [ ] 提升处理速度
- [ ] 优化用户界面

### 功能增强
- [ ] 更多翻译服务支持
- [ ] 批量处理优化
- [ ] 自定义书签脚本

## 📄 许可证

本项目采用 [CC0 1.0 Universal](LICENSE) 许可证。

## 🙏 致谢

基于原版 [Monolith](https://github.com/Y2Z/monolith) 项目，感谢以下开源项目：

- [html5ever](https://github.com/servo/html5ever) - HTML 解析
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP 客户端  
- [tokio](https://github.com/tokio-rs/tokio) - 异步运行时
- [axum](https://github.com/tokio-rs/axum) - Web 框架

---

**Monolith 轻量版** - 简单、便捷、高效的网页处理工具！ 🚀