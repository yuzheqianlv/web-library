// Monolith 主题管理组件

/**
 * 主题管理器类
 */
class ThemeManager {
    constructor() {
        this.themes = ['light', 'dark', 'blue', 'green'];
        this.currentTheme = 'light';
        this.init();
    }

    /**
     * 初始化主题管理器
     */
    init() {
        this.loadTheme();
        this.bindEvents();
    }

    /**
     * 绑定事件
     */
    bindEvents() {
        const themeSelect = document.getElementById('theme-select');
        if (themeSelect) {
            themeSelect.addEventListener('change', (e) => {
                this.setTheme(e.target.value);
            });
        }

        // 键盘快捷键：Ctrl+Shift+T 切换主题
        document.addEventListener('keydown', (e) => {
            if (e.ctrlKey && e.shiftKey && e.key === 'T') {
                e.preventDefault();
                this.cycleTheme();
            }
        });
    }

    /**
     * 设置主题
     * @param {string} theme 主题名称
     */
    setTheme(theme) {
        if (!this.themes.includes(theme)) {
            console.warn(`未知主题: ${theme}`);
            return;
        }

        // 移除当前主题类
        document.body.className = document.body.className
            .replace(/theme-\w+/g, '')
            .trim();

        // 添加新主题类
        document.body.classList.add(`theme-${theme}`);
        
        this.currentTheme = theme;
        this.saveTheme();

        // 更新选择器
        const themeSelect = document.getElementById('theme-select');
        if (themeSelect && themeSelect.value !== theme) {
            themeSelect.value = theme;
        }

        // 触发主题变更事件
        this.dispatchThemeChangeEvent(theme);
    }

    /**
     * 循环切换主题
     */
    cycleTheme() {
        const currentIndex = this.themes.indexOf(this.currentTheme);
        const nextIndex = (currentIndex + 1) % this.themes.length;
        this.setTheme(this.themes[nextIndex]);
    }

    /**
     * 获取当前主题
     * @returns {string} 当前主题名称
     */
    getCurrentTheme() {
        return this.currentTheme;
    }

    /**
     * 保存主题到本地存储
     */
    saveTheme() {
        try {
            localStorage.setItem('monolith-theme', this.currentTheme);
        } catch (error) {
            console.warn('保存主题失败:', error);
        }
    }

    /**
     * 从本地存储加载主题
     */
    loadTheme() {
        try {
            const savedTheme = localStorage.getItem('monolith-theme');
            if (savedTheme && this.themes.includes(savedTheme)) {
                this.setTheme(savedTheme);
            } else {
                this.setTheme('light'); // 默认主题
            }
        } catch (error) {
            console.warn('加载主题失败:', error);
            this.setTheme('light');
        }
    }

    /**
     * 触发主题变更事件
     * @param {string} theme 新主题名称
     */
    dispatchThemeChangeEvent(theme) {
        const event = new CustomEvent('themechange', {
            detail: { theme }
        });
        document.dispatchEvent(event);
    }

    /**
     * 获取主题颜色值
     * @param {string} property CSS变量名
     * @returns {string} 颜色值
     */
    getThemeColor(property) {
        return getComputedStyle(document.documentElement)
            .getPropertyValue(property)
            .trim();
    }

    /**
     * 检测系统主题偏好
     * @returns {string} 'dark' 或 'light'
     */
    detectSystemTheme() {
        if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
            return 'dark';
        }
        return 'light';
    }

    /**
     * 设置自动主题（跟随系统）
     */
    setAutoTheme() {
        const systemTheme = this.detectSystemTheme();
        this.setTheme(systemTheme);

        // 监听系统主题变化
        if (window.matchMedia) {
            const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
            mediaQuery.addEventListener('change', (e) => {
                this.setTheme(e.matches ? 'dark' : 'light');
            });
        }
    }
}

// 创建全局主题管理器实例
document.addEventListener('DOMContentLoaded', () => {
    window.themeManager = new ThemeManager();
});