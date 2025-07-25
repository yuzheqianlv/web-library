# Monolith Web 模板文件结构

本目录包含 Monolith 网页翻译器的前端模板文件，已重构为模块化结构便于维护。

## 📁 文件结构

```
templates/
├── README.md                    # 本说明文件
├── index.html                   # 原始单文件模板（保留作为备用）
├── index-new.html              # 新的模块化模板（引用外部资源）
├── assets/
│   ├── config.json             # 配置文件
│   ├── css/
│   │   └── main.css           # 主样式文件
│   └── js/
│       └── monolith-translator.js  # 主 JavaScript 文件
```

## 🔧 使用方式

### 开发模式
使用 `index-new.html` 作为模板，外部引用 CSS 和 JavaScript 文件：
- 便于调试和热重载
- 文件结构清晰
- 支持浏览器缓存

### 生产模式
使用 Rust 的 `HtmlBuilder` 动态构建单文件 HTML：
- 所有资源内联，无外部依赖
- 部署简单，只需一个 HTML 文件
- 性能优化，减少 HTTP 请求

## 📦 模块说明

### CSS 模块 (`assets/css/main.css`)
包含所有样式定义：
- 响应式布局
- 导航栏样式
- 双语模式布局
- 加载和错误状态
- 动画效果

### JavaScript 模块 (`assets/js/monolith-translator.js`)
核心功能实现：
- `MonolithTranslator` 主类
- 翻译请求处理
- UI 交互管理
- 双语同步滚动
- 错误处理

### 配置文件 (`assets/config.json`)
应用配置信息：
- API 端点配置
- 功能开关
- 资源路径配置
- 版本信息

## 🛠️ 开发指南

### 修改样式
直接编辑 `assets/css/main.css`，修改会在下次访问时生效。

### 修改 JavaScript
编辑 `assets/js/monolith-translator.js`，支持：
- ES6+ 语法
- 模块化开发
- 详细注释

### 添加新功能
1. 在 JavaScript 中添加新方法
2. 在 CSS 中添加对应样式
3. 更新配置文件（如需要）

### 测试
```bash
# 开发模式：直接在浏览器中打开 index-new.html
# 生产模式：通过 Rust web 服务器访问

# 编译测试
cargo build --features="web,translation" --release
./target/release/monolith-web
```

## 🎯 构建流程

1. **开发阶段**：使用外部引用的 `index-new.html`
2. **构建阶段**：`HtmlBuilder` 读取所有资源文件
3. **内联处理**：CSS/JS 内容嵌入到 HTML 中
4. **输出结果**：生成单文件 HTML 响应

## 🔄 版本管理

- `index.html` - 保留原始版本作为备份
- `index-new.html` - 新的模块化版本
- 通过 `config.json` 管理版本信息
- 支持渐进式迁移

## 🚀 性能优化

### 内联资源优势
- 减少 HTTP 请求数量
- 避免资源加载延迟
- 提高首屏渲染速度

### 开发体验优化
- 文件结构清晰
- 便于代码维护
- 支持版本控制
- 易于团队协作

## 📝 注意事项

1. **文件路径**：确保 `HtmlBuilder` 能正确找到模板文件
2. **编码格式**：所有文件使用 UTF-8 编码
3. **兼容性**：保持与原版 `index.html` 的功能一致性
4. **性能**：避免在 CSS/JS 中使用过大的资源文件

## 🔮 未来扩展

- 支持多主题切换
- 添加插件系统
- 国际化支持
- PWA 功能
- 离线缓存