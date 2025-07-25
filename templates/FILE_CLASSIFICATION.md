# Templates 文件分类清单

## 📋 文件迁移规划

基于重构计划，对现有文件进行分类并规划迁移路径。

## 🎯 保留并迁移的核心文件

### HTML 模板文件

| 原文件 | 新位置 | 用途 | 操作 |
|--------|--------|------|------|
| `index.html` | `core/index.html` | 主页模板 | 清理并迁移 |
| `library.html` | `core/library.html` | 库管理页面 | 重构后迁移 |
| `bookmarklet.html` | `legacy/bookmarklet.html` | 书签工具 | 直接迁移 |

### HTML 组件文件

| 原文件 | 新位置 | 用途 | 操作 |
|--------|--------|------|------|
| `components/library-header.html` | `components/library/header.html` | 库页面头部 | 重命名迁移 |
| `components/library-navbar.html` | `components/layout/navbar.html` | 导航栏 | 通用化处理 |
| `components/library-pagination.html` | `components/library/pagination.html` | 分页组件 | 重命名迁移 |
| `components/library-table.html` | `components/library/table.html` | 数据表格 | 重命名迁移 |
| `components/library-toolbar.html` | `components/library/toolbar.html` | 工具栏 | 重命名迁移 |

### CSS 样式文件

| 原文件 | 新位置 | 用途 | 操作 |
|--------|--------|------|------|
| `assets/css/main.css` | `assets/css/base.css` | 基础样式 | 重构整合 |
| `assets/css/theme-base.css` | `assets/css/themes.css` | 主题样式 | 重构整合 |
| `assets/css/library.css` | 整合到多个文件 | 库样式 | 拆分整合 |
| `assets/css/library-toolbar.css` | `assets/css/components.css` | 工具栏样式 | 整合 |
| `assets/css/library-table.css` | `assets/css/components.css` | 表格样式 | 整合 |
| `assets/css/library-pagination.css` | `assets/css/components.css` | 分页样式 | 整合 |
| `assets/css/library-responsive.css` | `assets/css/responsive.css` | 响应式 | 重命名迁移 |
| `assets/css/library-mongo.css` | 合并到其他文件 | MongoDB特定 | 整合删除 |

### JavaScript 脚本文件

| 原文件 | 新位置 | 用途 | 操作 |
|--------|--------|------|------|
| `assets/js/monolith-translator.js` | `assets/js/components/translator.js` | 翻译功能 | 重命名迁移 |
| `assets/js/library-core.js` | `assets/js/components/library.js` | 库核心功能 | 重构整合 |
| `assets/js/library-data.js` | `assets/js/core/api.js` | 数据处理 | 重构整合 |
| `assets/js/library-ui.js` | `assets/js/components/ui.js` | UI交互 | 重构整合 |
| `assets/js/library-manager.js` | `assets/js/core/app.js` | 应用管理 | 重构整合 |
| `assets/js/theme-manager.js` | `assets/js/components/theme.js` | 主题管理 | 重命名迁移 |
| `assets/js/scroll-sync-fix.js` | 删除 | 滚动同步修复 | 功能整合后删除 |
| `assets/js/simple-scroll-sync.js` | 删除 | 简单滚动同步 | 功能整合后删除 |

### 配置文件

| 原文件 | 新位置 | 用途 | 操作 |
|--------|--------|------|------|
| `assets/config.json` | `assets/config/app.json` | 应用配置 | 重命名迁移 |

## 🗑️ 删除的冗余文件

### 重复的HTML文件

| 文件 | 删除原因 |
|------|----------|
| `index-backup.html` | 备份文件，已有主备份 |
| `index-new.html` | 实验版本，功能已整合 |
| `index-themed.html` | 主题实验版本，功能已整合 |
| `library-debug.html` | 调试版本，不适合生产 |
| `library-original.html` | 原始版本，已有主备份 |

### 文档文件 (迁移到根目录)

| 文件 | 处理方式 |
|------|----------|
| `README.md` | 更新内容，保留在templates目录 |
| `TEMPLATE_MIGRATION.md` | 移动到docs目录 |
| `LIBRARY_MIGRATION_TODO.md` | 任务完成后删除 |
| `library-modules.md` | 更新后保留 |

## 🔄 迁移执行计划

### Phase 1: 创建基础结构
```bash
# 已完成：创建新目录结构
mkdir -p core components/{layout,library,common} assets/{css,js/{core,components,vendors},config} legacy
```

### Phase 2: 迁移HTML模板文件
```bash
# 迁移核心模板
cp index.html core/
cp library.html core/

# 迁移组件 (需要重命名)
cp components/library-header.html components/library/header.html
cp components/library-navbar.html components/layout/navbar.html
cp components/library-pagination.html components/library/pagination.html
cp components/library-table.html components/library/table.html
cp components/library-toolbar.html components/library/toolbar.html

# 迁移特殊文件
cp bookmarklet.html legacy/
```

### Phase 3: 迁移并整合CSS文件
```bash
# 基础样式重构
# 将main.css重构为base.css
# 将多个library-*.css整合为components.css
# 将theme-base.css重构为themes.css
# 保留library-responsive.css为responsive.css
```

### Phase 4: 迁移并整合JavaScript文件
```bash
# 核心模块
# 重构library-manager.js为core/app.js
# 重构library-data.js为core/api.js
# 创建core/utils.js和core/events.js

# 组件模块  
# 重命名monolith-translator.js为components/translator.js
# 整合library-core.js到components/library.js
# 重构library-ui.js为components/ui.js
# 重命名theme-manager.js为components/theme.js
```

### Phase 5: 迁移配置文件
```bash
# 配置文件重组
cp assets/config.json assets/config/app.json
# 创建assets/config/api.json
# 创建assets/config/features.json
```

### Phase 6: 清理冗余文件
```bash
# 删除重复的HTML文件
rm index-backup.html index-new.html index-themed.html
rm library-debug.html library-original.html

# 删除重复的JavaScript文件  
rm assets/js/scroll-sync-fix.js assets/js/simple-scroll-sync.js

# 删除已整合的CSS文件
rm assets/css/library-mongo.css
# (其他CSS文件在整合后删除)
```

## ✅ 验证检查清单

### 文件完整性检查
- [ ] 所有核心功能文件已迁移
- [ ] 组件文件已正确重命名和分类
- [ ] 配置文件已正确迁移
- [ ] 无重要文件丢失

### 依赖关系检查  
- [ ] HTML模板中的资源引用路径正确
- [ ] CSS文件间的依赖关系保持
- [ ] JavaScript模块依赖关系清晰
- [ ] 配置文件引用路径正确

### 功能保持检查
- [ ] 所有页面功能正常加载
- [ ] 样式渲染正确
- [ ] JavaScript交互正常
- [ ] 主题切换功能正常

## 📊 迁移统计

### 文件数量变化
```
迁移前:
├── HTML: 9个文件 -> 7个保留 (2个删除)
├── CSS: 8个文件 -> 5个整合后文件 (3个删除)  
├── JS: 8个文件 -> 7个重构后文件 (1个删除)
└── Config: 1个文件 -> 3个分离后文件

迁移后:
├── core/: 2个HTML模板
├── components/: 5个HTML组件
├── assets/css/: 5个样式文件
├── assets/js/: 7个脚本文件 (分3个子目录)
├── assets/config/: 3个配置文件
└── legacy/: 1个特殊文件
```

### 代码减少预估
- CSS代码重复率预计减少60%
- JavaScript模块依赖复杂度降低50%  
- HTML组件复用率提升80%
- 整体维护工作量减少40%

---

**创建时间**: 2025-07-25  
**状态**: 规划完成，待执行  
**下一步**: 开始Phase 2文件迁移