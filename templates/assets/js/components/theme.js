/**
 * Monolith 主题管理器
 * 负责主题切换、用户偏好管理和主题状态同步
 */

class ThemeManager {
    constructor() {
        this.currentTheme = this.getStoredTheme() || 'light';
        this.themes = new Map();
        this.callbacks = new Set();
        this.systemThemeQuery = window.matchMedia('(prefers-color-scheme: dark)');
        this.autoSwitch = this.getAutoSwitchPreference();
        
        this.init();
    }

    /**
     * 初始化主题管理器
     */
    async init() {
        try {
            // 加载可用主题列表
            await this.loadThemes();
            
            // 初始化主题选择器
            this.initThemeSelector();
            
            // 监听系统主题变化
            this.systemThemeQuery.addEventListener('change', (e) => {
                if (this.autoSwitch) {
                    this.setTheme(e.matches ? 'dark' : 'light');
                }
            });
            
            // 应用当前主题
            await this.applyTheme(this.currentTheme);
            
            // 添加键盘快捷键
            this.initKeyboardShortcuts();
            
            console.log('ThemeManager initialized successfully');
        } catch (error) {
            console.error('Failed to initialize ThemeManager:', error);
        }
    }

    /**
     * 加载可用主题列表
     */
    async loadThemes() {
        try {
            const response = await fetch('/api/theme/list');
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}`);
            }
            
            const result = await response.json();
            if (result.success) {
                this.themes.clear();
                result.data.themes.forEach(theme => {
                    this.themes.set(theme.name, theme);
                });
                
                // 如果当前主题不存在，重置为默认主题
                if (!this.themes.has(this.currentTheme)) {
                    this.currentTheme = result.data.current || 'light';
                }
            }
        } catch (error) {
            console.error('Failed to load themes:', error);
            // 使用默认主题配置
            this.loadDefaultThemes();
        }
    }

    /**
     * 加载默认主题配置（离线fallback）
     */
    loadDefaultThemes() {
        const defaultThemes = [
            {
                name: 'light',
                display_name: '明亮主题',
                description: '清新明亮的默认主题',
                is_dark: false
            },
            {
                name: 'dark',
                display_name: '暗色主题',
                description: '护眼的暗色主题',
                is_dark: true
            },
            {
                name: 'blue',
                display_name: '海洋蓝',
                description: '清新的海洋蓝主题',
                is_dark: false
            },
            {
                name: 'green',
                display_name: '自然绿',
                description: '清新的自然绿主题',
                is_dark: false
            }
        ];
        
        this.themes.clear();
        defaultThemes.forEach(theme => {
            this.themes.set(theme.name, theme);
        });
    }

    /**
     * 初始化主题选择器
     */
    initThemeSelector() {
        const selector = document.getElementById('theme-select');
        if (!selector) return;

        // 清空现有选项
        selector.innerHTML = '';
        
        // 添加主题选项
        this.themes.forEach(theme => {
            const option = document.createElement('option');
            option.value = theme.name;
            option.textContent = theme.display_name;
            option.dataset.isDark = theme.is_dark;
            if (theme.name === this.currentTheme) {
                option.selected = true;
            }
            selector.appendChild(option);
        });

        // 绑定变化事件
        selector.addEventListener('change', (e) => {
            this.setTheme(e.target.value);
        });
    }

    /**
     * 初始化快速主题切换按钮
     */
    initThemeToggle() {
        const toggleBtn = document.getElementById('theme-toggle');
        if (!toggleBtn) return;

        toggleBtn.addEventListener('click', () => {
            this.toggleTheme();
        });

        // 更新按钮图标
        this.updateToggleButton();
    }

    /**
     * 更新主题切换按钮
     */
    updateToggleButton() {
        const toggleBtn = document.getElementById('theme-toggle');
        if (!toggleBtn) return;

        const currentThemeInfo = this.themes.get(this.currentTheme);
        const isDark = currentThemeInfo?.is_dark || false;
        
        // 更新图标
        toggleBtn.innerHTML = isDark ? 
            '<svg viewBox="0 0 24 24"><path fill="currentColor" d="M12 7c-2.76 0-5 2.24-5 5s2.24 5 5 5 5-2.24 5-5-2.24-5-5-5zM2 13h2c.55 0 1-.45 1-1s-.45-1-1-1H2c-.55 0-1 .45-1 1s.45 1 1 1zm18 0h2c.55 0 1-.45 1-1s-.45-1-1-1h-2c-.55 0-1 .45-1 1s.45 1 1 1zM11 2v2c0 .55.45 1 1 1s1-.45 1-1V2c0-.55-.45-1-1-1s-1 .45-1 1zm0 18v2c0 .55.45 1 1 1s1-.45 1-1v-2c0-.55-.45-1-1-1s-1 .45-1 1zM5.99 4.58c-.39-.39-1.03-.39-1.41 0-.39.39-.39 1.03 0 1.41l1.06 1.06c.39.39 1.03.39 1.41 0s.39-1.03 0-1.41L5.99 4.58zm12.37 12.37c-.39-.39-1.03-.39-1.41 0-.39.39-.39 1.03 0 1.41l1.06 1.06c.39.39 1.03.39 1.41 0 .39-.39.39-1.03 0-1.41l-1.06-1.06zm1.06-10.96c.39-.39.39-1.03 0-1.41-.39-.39-1.03-.39-1.41 0l-1.06 1.06c-.39.39-.39 1.03 0 1.41s1.03.39 1.41 0l1.06-1.06zM7.05 18.36c.39-.39.39-1.03 0-1.41-.39-.39-1.03-.39-1.41 0l-1.06 1.06c-.39.39-.39 1.03 0 1.41s1.03.39 1.41 0l1.06-1.06z"/></svg>' :
            '<svg viewBox="0 0 24 24"><path fill="currentColor" d="M9 2c-1.05 0-2.05.16-3 .46 4.06 1.27 7 5.06 7 9.54 0 4.48-2.94 8.27-7 9.54.95.3 1.95.46 3 .46 5.52 0 10-4.48 10-10S14.52 2 9 2z"/></svg>';
        
        // 更新tooltip
        toggleBtn.title = isDark ? '切换到明亮主题' : '切换到暗色主题';
    }

    /**
     * 设置主题
     * @param {string} themeName - 主题名称
     * @param {boolean} savePreference - 是否保存用户偏好
     */
    async setTheme(themeName, savePreference = true) {
        if (!this.themes.has(themeName)) {
            console.warn(`Theme '${themeName}' not found`);
            return;
        }

        const oldTheme = this.currentTheme;
        this.currentTheme = themeName;

        try {
            // 通知服务器切换主题
            const response = await fetch('/api/theme/set', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ theme: themeName })
            });

            if (!response.ok) {
                throw new Error(`HTTP ${response.status}`);
            }

            // 应用主题
            await this.applyTheme(themeName);

            // 保存用户偏好
            if (savePreference) {
                this.storeTheme(themeName);
            }

            // 更新UI元素
            this.updateThemeSelector();
            this.updateToggleButton();

            // 触发主题变化回调
            this.notifyThemeChange(themeName, oldTheme);

            // 显示成功提示
            this.showToast(`已切换到${this.themes.get(themeName).display_name}`, 'success');
            
        } catch (error) {
            console.error('Failed to set theme:', error);
            this.currentTheme = oldTheme; // 恢复旧主题
            this.showToast('主题切换失败', 'error');
        }
    }

    /**
     * 应用主题样式
     * @param {string} themeName - 主题名称
     */
    async applyTheme(themeName) {
        try {
            // 添加主题过渡类
            document.body.classList.add('theme-transition');

            // 获取主题CSS变量
            const response = await fetch(`/api/theme/css/${themeName}`);
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}`);
            }

            const css = await response.text();
            this.updateStyleSheet(css);
            
            // 更新body类名
            document.body.className = document.body.className
                .replace(/theme-\w+/g, '')
                .trim() + ` theme-${themeName}`;

            // 移除过渡类（延迟以确保动画完成）
            setTimeout(() => {
                document.body.classList.remove('theme-transition');
            }, 500);

            // 更新meta theme-color
            this.updateMetaThemeColor(themeName);
            
        } catch (error) {
            console.error('Failed to apply theme:', error);
            throw error;
        }
    }

    /**
     * 更新样式表
     * @param {string} css - CSS内容
     */
    updateStyleSheet(css) {
        let styleElement = document.getElementById('theme-variables');
        if (!styleElement) {
            styleElement = document.createElement('style');
            styleElement.id = 'theme-variables';
            document.head.appendChild(styleElement);
        }
        styleElement.textContent = css;
    }

    /**
     * 更新meta theme-color
     * @param {string} themeName - 主题名称
     */
    updateMetaThemeColor(themeName) {
        let metaThemeColor = document.querySelector('meta[name="theme-color"]');
        if (!metaThemeColor) {
            metaThemeColor = document.createElement('meta');
            metaThemeColor.name = 'theme-color';
            document.head.appendChild(metaThemeColor);
        }

        // 从CSS变量获取主色调
        const primaryColor = getComputedStyle(document.documentElement)
            .getPropertyValue('--color-primary').trim();
        
        if (primaryColor) {
            metaThemeColor.content = primaryColor;
        }
    }

    /**
     * 切换主题（明亮/暗色）
     */
    toggleTheme() {
        const currentThemeInfo = this.themes.get(this.currentTheme);
        const isDark = currentThemeInfo?.is_dark || false;
        
        // 寻找相反类型的主题
        let targetTheme = null;
        for (const [name, theme] of this.themes) {
            if (theme.is_dark !== isDark) {
                targetTheme = name;
                break;
            }
        }
        
        if (targetTheme) {
            this.setTheme(targetTheme);
        } else {
            // 如果没找到相反类型，就在明亮和暗色之间切换
            this.setTheme(isDark ? 'light' : 'dark');
        }
    }

    /**
     * 更新主题选择器状态
     */
    updateThemeSelector() {
        const selector = document.getElementById('theme-select');
        if (selector) {
            selector.value = this.currentTheme;
        }
    }

    /**
     * 获取当前主题
     * @returns {Object|null} 当前主题信息
     */
    getCurrentTheme() {
        return this.themes.get(this.currentTheme) || null;
    }

    /**
     * 获取当前主题是否为暗色
     * @returns {boolean}
     */
    isCurrentThemeDark() {
        const theme = this.getCurrentTheme();
        return theme ? theme.is_dark : false;
    }

    /**
     * 获取所有主题
     * @returns {Array} 主题列表
     */
    getAllThemes() {
        return Array.from(this.themes.values());
    }

    /**
     * 注册主题变化回调
     * @param {Function} callback - 回调函数
     */
    onThemeChange(callback) {
        this.callbacks.add(callback);
    }

    /**
     * 移除主题变化回调
     * @param {Function} callback - 回调函数
     */
    offThemeChange(callback) {
        this.callbacks.delete(callback);
    }

    /**
     * 通知主题变化
     * @param {string} newTheme - 新主题
     * @param {string} oldTheme - 旧主题
     */
    notifyThemeChange(newTheme, oldTheme) {
        this.callbacks.forEach(callback => {
            try {
                callback(newTheme, oldTheme);
            } catch (error) {
                console.error('Theme change callback error:', error);
            }
        });
    }

    /**
     * 获取存储的主题
     * @returns {string|null}
     */
    getStoredTheme() {
        try {
            return localStorage.getItem('monolith-theme');
        } catch (error) {
            console.warn('Failed to read theme from localStorage:', error);
            return null;
        }
    }

    /**
     * 存储主题偏好
     * @param {string} themeName - 主题名称
     */
    storeTheme(themeName) {
        try {
            localStorage.setItem('monolith-theme', themeName);
        } catch (error) {
            console.warn('Failed to save theme to localStorage:', error);
        }
    }

    /**
     * 获取自动切换偏好
     * @returns {boolean}
     */
    getAutoSwitchPreference() {
        try {
            const stored = localStorage.getItem('monolith-auto-theme');
            return stored !== null ? JSON.parse(stored) : true;
        } catch (error) {
            console.warn('Failed to read auto-switch preference:', error);
            return true;
        }
    }

    /**
     * 设置自动切换偏好
     * @param {boolean} enabled - 是否启用自动切换
     */
    setAutoSwitchPreference(enabled) {
        this.autoSwitch = enabled;
        try {
            localStorage.setItem('monolith-auto-theme', JSON.stringify(enabled));
        } catch (error) {
            console.warn('Failed to save auto-switch preference:', error);
        }
    }

    /**
     * 初始化键盘快捷键
     */
    initKeyboardShortcuts() {
        document.addEventListener('keydown', (e) => {
            // Ctrl/Cmd + Shift + T 切换主题
            if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'T') {
                e.preventDefault();
                this.toggleTheme();
            }
        });
    }

    /**
     * 显示提示消息
     * @param {string} message - 消息内容
     * @param {string} type - 消息类型 ('success', 'error', 'info')
     */
    showToast(message, type = 'info') {
        // 移除现有提示
        const existingToasts = document.querySelectorAll('.toast');
        existingToasts.forEach(toast => toast.remove());

        // 创建新提示
        const toast = document.createElement('div');
        toast.className = `toast ${type}-toast show`;
        toast.textContent = message;
        toast.style.cssText = `
            position: fixed;
            top: 80px;
            right: 20px;
            padding: var(--spacing-medium) var(--spacing-large);
            border-radius: var(--border-radius);
            color: white;
            font-weight: 500;
            box-shadow: var(--shadow-large);
            z-index: 1000;
            max-width: 300px;
            animation: slideIn 0.3s ease;
        `;

        // 设置背景色
        switch (type) {
            case 'success':
                toast.style.background = 'var(--color-success)';
                break;
            case 'error':
                toast.style.background = 'var(--color-error)';
                break;
            default:
                toast.style.background = 'var(--color-info)';
        }

        document.body.appendChild(toast);

        // 点击关闭
        toast.addEventListener('click', () => toast.remove());

        // 自动移除
        setTimeout(() => {
            if (toast.parentNode) {
                toast.remove();
            }
        }, 3000);
    }

    /**
     * 预览主题
     * @param {string} themeName - 主题名称
     */
    previewTheme(themeName) {
        if (!this.themes.has(themeName)) {
            console.warn(`Theme '${themeName}' not found`);
            return;
        }

        // 在新窗口中打开主题预览
        const previewUrl = `/api/theme/preview/${themeName}`;
        window.open(previewUrl, '_blank', 'width=800,height=600');
    }

    /**
     * 检测系统主题偏好
     * @returns {string} 'light' 或 'dark'
     */
    getSystemThemePreference() {
        return this.systemThemeQuery.matches ? 'dark' : 'light';
    }

    /**
     * 同步系统主题
     */
    syncWithSystemTheme() {
        if (this.autoSwitch) {
            const systemTheme = this.getSystemThemePreference();
            this.setTheme(systemTheme);
        }
    }

    /**
     * 销毁主题管理器
     */
    destroy() {
        this.callbacks.clear();
        this.systemThemeQuery.removeEventListener('change', this.handleSystemThemeChange);
    }
}

// 导出给全局使用
window.ThemeManager = ThemeManager;

// 自动初始化主题管理器
document.addEventListener('DOMContentLoaded', () => {
    if (!window.themeManager) {
        window.themeManager = new ThemeManager();
    }
});

// 在页面卸载时清理
window.addEventListener('beforeunload', () => {
    if (window.themeManager) {
        window.themeManager.destroy();
    }
});