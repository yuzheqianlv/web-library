# Monolith 主题管理系统文档

## 概述

Monolith 主题管理系统提供了一个完整的、规范化的主题切换和管理解决方案，支持多种预设主题和自定义主题，为用户提供个性化的界面体验。

## 🎨 功能特性

### 核心功能
- **多主题支持**: 内置明亮、暗色、海洋蓝、自然绿四种主题
- **实时切换**: 无刷新页面即时切换主题
- **用户偏好**: 自动保存用户主题选择
- **系统同步**: 支持跟随系统主题自动切换
- **响应式设计**: 完美适配桌面端和移动端
- **键盘快捷键**: 支持快捷键快速切换主题

### 技术特性
- **CSS变量系统**: 基于CSS自定义属性实现主题变量
- **模块化架构**: 主题逻辑与业务逻辑完全分离
- **RESTful API**: 提供完整的主题管理API接口
- **类型安全**: 使用Rust强类型系统确保代码安全
- **缓存优化**: 智能缓存主题资源，提升性能

## 📁 文件结构

```
monolith/
├── src/web/
│   ├── theme.rs                     # 主题管理核心模块
│   └── handlers/api/theme.rs        # 主题API处理器
├── templates/
│   ├── index-themed.html            # 集成主题功能的主页模板
│   └── assets/
│       ├── css/
│       │   ├── theme-base.css       # 基于CSS变量的主题样式
│       │   └── main.css             # 原始样式（已弃用）
│       └── js/
│           ├── theme-manager.js     # 主题管理器JavaScript
│           └── monolith-translator.js # 翻译器主逻辑（已更新）
└── docs/
    └── THEME_SYSTEM.md              # 本文档
```

## 🚀 快速开始

### 1. 启用主题功能

主题功能默认集成在web feature中，启动web服务即可使用：

```bash
# 启动web服务器，包含主题功能
cargo run --bin monolith-web --features="web"
```

### 2. 访问主题化界面

打开浏览器访问：`http://localhost:7080`，即可看到集成了主题功能的界面。

### 3. 切换主题

- **使用选择器**: 点击导航栏中的主题选择器
- **快捷切换**: 点击主题切换按钮（月亮/太阳图标）
- **键盘快捷键**: 按 `Ctrl/Cmd + Shift + T`

## 🎯 主题配置

### 预设主题

| 主题名称 | 说明 | 类型 |
|---------|------|------|
| `light` | 明亮主题 | 明亮 |
| `dark` | 暗色主题 | 暗色 |
| `blue` | 海洋蓝主题 | 明亮 |
| `green` | 自然绿主题 | 明亮 |

### 主题配置结构

```rust
pub struct ThemeConfig {
    pub name: String,              // 主题名称
    pub display_name: String,      // 显示名称
    pub description: String,       // 主题描述
    pub is_dark: bool,            // 是否为暗色主题
    pub colors: ThemeColors,       // 颜色配置
    pub fonts: ThemeFonts,         // 字体配置
    pub spacing: ThemeSpacing,     // 间距配置
    pub shadows: ThemeShadows,     // 阴影配置
    pub borders: ThemeBorders,     // 边框配置
    pub animations: ThemeAnimations, // 动画配置
}
```

## 🔧 API 接口

### 主题管理API

| 方法 | 端点 | 说明 |
|------|------|------|
| GET | `/api/theme/list` | 获取所有可用主题 |
| GET | `/api/theme/current` | 获取当前主题CSS |
| POST | `/api/theme/set` | 设置当前主题 |
| GET | `/api/theme/css/:theme` | 获取指定主题CSS |
| GET | `/api/theme/preview/:theme` | 预览主题 |
| POST | `/api/theme/register` | 注册自定义主题 |
| POST | `/api/theme/delete/:theme` | 删除自定义主题 |

### 使用示例

#### 获取主题列表
```javascript
const response = await fetch('/api/theme/list');
const result = await response.json();
console.log(result.data.themes);
```

#### 切换主题
```javascript
const response = await fetch('/api/theme/set', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ theme: 'dark' })
});
```

#### 注册自定义主题
```javascript
const customTheme = {
    name: "purple",
    display_name: "紫色主题",
    description: "优雅的紫色主题",
    is_dark: false,
    colors: {
        primary: "#8b5cf6",
        primary_hover: "#7c3aed",
        // ... 其他配置
    },
    // ... 其他配置项
};

await fetch('/api/theme/register', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(customTheme)
});
```

## 🎨 自定义主题

### 创建自定义主题

1. **定义主题配置**:

```rust
use crate::web::theme::*;

let custom_theme = ThemeConfig {
    name: "custom".to_string(),
    display_name: "自定义主题".to_string(),
    description: "我的专属主题".to_string(),
    is_dark: false,
    colors: ThemeColors {
        primary: "#ff6b6b".to_string(),
        primary_hover: "#ff5252".to_string(),
        // ... 其他颜色配置
    },
    // ... 其他配置
};
```

2. **注册主题**:

```rust
let mut theme_manager = ThemeManager::new();
theme_manager.register_theme(custom_theme);
```

### CSS变量系统

主题系统基于CSS自定义属性（变量）实现，所有主题相关的样式都应使用变量：

```css
/* 使用主题变量 */
.my-component {
    background: var(--color-surface);
    color: var(--color-text-primary);
    border: var(--border-width) var(--border-style) var(--color-border);
    border-radius: var(--border-radius);
    padding: var(--spacing-medium);
    box-shadow: var(--shadow-medium);
    transition: all var(--animation-duration) var(--animation-easing);
}

.my-component:hover {
    background: var(--color-primary);
    color: white;
}
```

### 可用的CSS变量

#### 颜色变量
- `--color-primary` / `--color-primary-hover`
- `--color-secondary` / `--color-secondary-hover`
- `--color-background` / `--color-surface`
- `--color-text-primary` / `--color-text-secondary`
- `--color-border`
- `--color-success` / `--color-warning` / `--color-error` / `--color-info`

#### 字体变量
- `--font-family` / `--font-mono-family`
- `--font-base-size` / `--font-line-height`

#### 间距变量
- `--spacing-small` / `--spacing-medium` / `--spacing-large` / `--spacing-xlarge`

#### 阴影变量
- `--shadow-small` / `--shadow-medium` / `--shadow-large` / `--shadow-xlarge`

#### 边框变量
- `--border-width` / `--border-style`
- `--border-radius` / `--border-radius-large`

#### 动画变量
- `--animation-duration` / `--animation-easing`
- `--animation-duration-fast` / `--animation-duration-slow`

## 🔧 开发指南

### 添加新主题

1. **在主题管理器中注册**:

```rust
// 在 ThemeManager::register_default_themes() 中添加
let new_theme = ThemeConfig {
    name: "red".to_string(),
    display_name: "红色主题".to_string(),
    // ... 配置
};
self.themes.insert("red".to_string(), new_theme);
```

2. **更新主题选择器**:

前端会自动从API获取主题列表，无需手动更新。

### 扩展主题功能

1. **添加新的主题属性**:

```rust
// 在 ThemeConfig 中添加新字段
pub struct ThemeConfig {
    // ... 现有字段
    pub effects: ThemeEffects,  // 新增效果配置
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeEffects {
    pub blur: String,
    pub opacity: String,
}
```

2. **更新CSS变量生成**:

```rust
// 在 generate_css_variables() 中添加新变量
format!(
    r#":root {{
  /* ... 现有变量 */
  --effect-blur: {};
  --effect-opacity: {};
}}"#,
    // ... 现有变量
    theme.effects.blur,
    theme.effects.opacity
)
```

### 主题持久化

主题偏好自动保存在localStorage中，键名为：
- `monolith-theme`: 当前选择的主题
- `monolith-auto-theme`: 是否启用自动切换

如需服务端持久化，可以扩展`UserThemePreference`结构并实现相应的存储逻辑。

## 🧪 测试

### 主题功能测试

1. **基础切换测试**:
   - 验证所有预设主题可以正常切换
   - 检查主题变化是否实时生效
   - 确认用户偏好是否正确保存

2. **API测试**:
   ```bash
   # 获取主题列表
   curl http://localhost:7080/api/theme/list
   
   # 切换主题
   curl -X POST http://localhost:7080/api/theme/set \
        -H "Content-Type: application/json" \
        -d '{"theme":"dark"}'
   
   # 获取主题CSS
   curl http://localhost:7080/api/theme/css/dark
   ```

3. **响应式测试**:
   - 在不同屏幕尺寸下测试主题切换
   - 验证移动端主题显示效果

## 🐛 故障排除

### 常见问题

1. **主题切换不生效**:
   - 检查CSS变量是否正确加载
   - 验证JavaScript控制台是否有错误
   - 确认API响应是否正常

2. **自定义主题显示异常**:
   - 检查主题配置是否完整
   - 验证颜色值格式是否正确
   - 确认所有必需字段都已设置

3. **性能问题**:
   - 检查主题切换是否有不必要的DOM操作
   - 优化CSS变量的使用
   - 确认没有内存泄漏

### 调试工具

1. **开发者工具**:
   ```javascript
   // 在浏览器控制台中调试
   console.log(window.themeManager.getCurrentTheme());
   console.log(window.themeManager.getAllThemes());
   ```

2. **网络监控**:
   - 检查主题API请求响应
   - 监控CSS资源加载时间

## 📈 性能优化

### 最佳实践

1. **CSS变量缓存**: 主题CSS在客户端缓存，减少重复请求
2. **懒加载**: 主题资源按需加载
3. **防抖处理**: 快速切换主题时进行防抖处理
4. **动画优化**: 使用CSS transforms而非layout属性

### 性能监控

```javascript
// 监控主题切换性能
const start = performance.now();
await themeManager.setTheme('dark');
const end = performance.now();
console.log(`Theme switch took ${end - start} milliseconds`);
```

## 🔮 未来规划

### 计划功能

1. **主题编辑器**: 可视化主题自定义工具
2. **主题市场**: 主题分享和下载平台
3. **动态主题**: 基于时间、地理位置的自动主题切换
4. **主题动画**: 更丰富的主题切换动画效果
5. **无障碍功能**: 高对比度、大字体等无障碍主题

### 技术改进

1. **性能优化**: 进一步优化主题切换性能
2. **兼容性**: 增强旧浏览器兼容性
3. **国际化**: 多语言主题名称支持
4. **插件系统**: 支持第三方主题插件

## 📝 更新日志

### v1.0.0 (2025-07-21)
- ✨ 初始版本发布
- 🎨 四种预设主题：明亮、暗色、海洋蓝、自然绿
- 🔧 完整的主题管理API
- 💾 用户偏好本地存储
- ⌨️ 键盘快捷键支持
- 📱 响应式设计适配

---

## 📞 技术支持

如有问题或建议，请提交Issue或联系开发团队。

**文档版本**: v1.0.0  
**最后更新**: 2025-07-21  
**维护者**: Monolith开发团队