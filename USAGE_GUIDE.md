# Monolith 轻量版使用指南

## 📖 快速上手

Monolith 轻量版专注于提供最简单、最便捷的网页保存体验。通过书签脚本工作流，您可以在任意网页上一键保存完整的单文件HTML。

## 🚀 启动服务器

```bash
# 启动轻量Web服务器
cargo run --bin monolith-web --features="web"

# 自定义端口
cargo run --bin monolith-web --features="web" -- --port 8080

# 启用翻译功能
cargo run --bin monolith-web --features="web,translation"
```

服务器启动后访问：`http://localhost:7080`

## 📋 书签脚本使用方法

### 第1步：获取书签脚本
1. 在浏览器中访问 `http://localhost:7080`
2. 您将看到一个精美的主页，包含"Monolith 处理器"按钮

### 第2步：安装书签
1. **方法一（推荐）**：直接拖拽页面上的"Monolith 处理器"按钮到浏览器书签栏
2. **方法二**：右键点击按钮 → "添加到书签" → 选择书签栏

### 第3步：使用书签
1. 打开任意想要保存的网页
2. 点击书签栏中的"Monolith 处理器"书签
3. 浏览器会自动跳转到处理结果页面
4. 获得完整的单文件HTML，包含所有资源

## 🌐 直接预览功能

您也可以直接通过URL预览网页处理结果：

```bash
# 访问预览URL
http://localhost:7080/preview/https://example.com

# 浏览器中直接输入
http://localhost:7080/preview/https://github.com
```

## 🛠️ CLI 命令行使用

```bash
# 保存网页为HTML文件
cargo run --bin monolith --features="cli" -- https://example.com > page.html

# 禁用JavaScript
cargo run --bin monolith --features="cli" -- --no-js https://example.com > page.html

# 禁用图片
cargo run --bin monolith --features="cli" -- --no-images https://example.com > page.html

# 设置超时时间
cargo run --bin monolith --features="cli" -- --timeout 30 https://example.com > page.html
```

## 🌍 翻译功能（可选）

如果启用了翻译功能，您可以：

### 环境变量配置
```bash
# 设置翻译API
export MONOLITH_TRANSLATION_API_KEY="your_api_key"
export MONOLITH_TRANSLATION_SERVICE="openai"

# 启动带翻译的服务器
cargo run --bin monolith-web --features="web,translation"
```

### 使用翻译API
```bash
# 翻译网页内容
curl -X POST http://localhost:7080/api/translate \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com", "target_lang": "zh"}'
```

## 📚 API 端点

轻量版提供以下6个核心API端点：

| 端点 | 方法 | 功能 |
|------|------|------|
| `/` | GET | 主页 - 获取书签脚本 |
| `/preview/{url}` | GET | 直接预览处理后的网页 |
| `/api/translate` | POST | 翻译API |
| `/api/content` | POST | 内容处理API |
| `/api/bookmarklet` | GET | 获取书签脚本代码 |
| `/api/cache/clear` | POST | 清理缓存 |

### API 使用示例

```bash
# 获取书签脚本
curl http://localhost:7080/api/bookmarklet

# 清理缓存
curl -X POST http://localhost:7080/api/cache/clear

# 处理内容
curl -X POST http://localhost:7080/api/content \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com"}'
```

## ⚙️ 配置选项

### 基础配置
```bash
# Web服务器配置
export MONOLITH_WEB_HOST="127.0.0.1"
export MONOLITH_WEB_PORT="7080"

# 核心功能配置
export MONOLITH_TIMEOUT="30"
export MONOLITH_USER_AGENT="Monolith-Lightweight"
```

### 翻译配置（可选）
```bash
export MONOLITH_TRANSLATION_ENABLED="true"
export MONOLITH_TRANSLATION_SERVICE="openai"
export MONOLITH_TRANSLATION_API_KEY="your_key_here"
export MONOLITH_TRANSLATION_TARGET_LANG="zh"
```

## 🎯 最佳实践

### 推荐工作流程
1. **服务器运行**：始终保持Web服务器在后台运行
2. **书签准备**：在所有常用浏览器中安装书签脚本
3. **一键使用**：遇到需要保存的网页时，直接点击书签
4. **批量处理**：对于多个网页，可以使用预览URL功能

### 性能优化
- 启用缓存以提高重复访问速度
- 对于大型网页，适当增加超时时间
- 使用翻译功能时，建议配置API密钥

### 常见问题

**Q: 书签脚本不工作怎么办？**
A: 检查服务器是否运行，确认端口号正确，尝试重新拖拽书签。

**Q: 处理某些网页失败？**
A: 某些网页可能有反爬措施，可以尝试增加超时时间或使用不同的User-Agent。

**Q: 如何备份书签脚本？**
A: 访问 `/api/bookmarklet` 端点可以获取当前的书签脚本代码。

## 🔧 故障排除

### 常见错误及解决方法

1. **连接拒绝错误**
   ```bash
   # 确认服务器正在运行
   curl -I http://localhost:7080/
   ```

2. **端口被占用**
   ```bash
   # 使用不同端口启动
   cargo run --bin monolith-web --features="web" -- --port 8080
   ```

3. **翻译功能不可用**
   ```bash
   # 检查翻译功能是否编译
   cargo build --features="web,translation"
   ```

## 🎉 开始使用

现在您已经了解了 Monolith 轻量版的所有功能！

1. 启动服务器：`cargo run --bin monolith-web --features="web"`
2. 访问主页：`http://localhost:7080`
3. 安装书签脚本
4. 开始一键保存网页！

享受简单、便捷、高效的网页保存体验！ 🚀