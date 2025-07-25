/**
 * Monolith 网页翻译器 - 应用管理核心
 * 整合自 library-manager.js - 提供统一的应用生命周期管理
 */

class AppManager {
    constructor() {
        this.components = new Map();
        this.initialized = false;
        this.config = {};
        this.eventBus = new EventTarget();
        
        // 绑定全局错误处理
        this.setupGlobalErrorHandling();
    }

    /**
     * 初始化应用
     * @param {Object} config - 应用配置
     */
    async init(config = {}) {
        if (this.initialized) {
            console.warn('应用已初始化');
            return;
        }

        try {
            console.log('开始初始化 Monolith 应用...');
            
            this.config = {
                theme: 'light',
                language: 'zh',
                debug: false,
                ...config
            };

            // 初始化各个组件
            await this.initializeComponents();
            
            // 加载应用配置
            await this.loadAppConfig();
            
            // 设置全局函数
            this.setupGlobalFunctions();
            
            // 绑定全局事件
            this.bindGlobalEvents();
            
            this.initialized = true;
            this.emit('app:initialized', { config: this.config });
            
            console.log('✅ Monolith 应用初始化完成');
        } catch (error) {
            console.error('❌ 应用初始化失败:', error);
            this.emit('app:error', { error, phase: 'initialization' });
            throw error;
        }
    }

    /**
     * 初始化组件
     */
    async initializeComponents() {
        const componentPromises = [];

        // API管理器
        if (window.ApiManager) {
            const apiManager = new window.ApiManager();
            this.registerComponent('api', apiManager);
            componentPromises.push(Promise.resolve());
        }

        // 主题管理器
        if (window.ThemeManager) {
            const themeManager = new window.ThemeManager();
            this.registerComponent('theme', themeManager);
            componentPromises.push(Promise.resolve());
        }

        // 翻译器组件
        if (window.MonolithTranslator) {
            const translator = new window.MonolithTranslator();
            this.registerComponent('translator', translator);
            componentPromises.push(Promise.resolve());
        }

        // 库管理器
        if (window.LibraryManager) {
            const libraryManager = new window.LibraryManager();
            this.registerComponent('library', libraryManager);
            componentPromises.push(libraryManager.init ? libraryManager.init() : Promise.resolve());
        }

        // UI管理器
        if (window.UIManager) {
            const uiManager = new window.UIManager();
            this.registerComponent('ui', uiManager);
            componentPromises.push(Promise.resolve());
        }

        // 等待所有组件初始化完成
        await Promise.all(componentPromises);
        console.log('所有组件初始化完成');
    }

    /**
     * 加载应用配置
     */
    async loadAppConfig() {
        try {
            const response = await fetch('/assets/config/app.json');
            if (response.ok) {
                const appConfig = await response.json();
                this.config = { ...this.config, ...appConfig };
                console.log('应用配置加载完成:', this.config);
            }
        } catch (error) {
            console.warn('加载应用配置失败，使用默认配置:', error);
        }
    }

    /**
     * 注册组件
     * @param {string} name - 组件名称
     * @param {Object} component - 组件实例
     */
    registerComponent(name, component) {
        this.components.set(name, component);
        console.log(`组件注册完成: ${name}`);
        
        // 触发组件注册事件
        this.emit('component:registered', { name, component });
    }

    /**
     * 获取组件
     * @param {string} name - 组件名称
     * @returns {Object|null} 组件实例
     */
    getComponent(name) {
        return this.components.get(name) || null;
    }

    /**
     * 设置全局函数
     */
    setupGlobalFunctions() {
        // 库管理相关的全局函数
        const libraryManager = this.getComponent('library');
        if (libraryManager) {
            window.viewRecord = (id, url) => this.handleViewRecord(id, url);
            window.deleteRecord = (id) => this.handleDeleteRecord(id);
            window.selectAllRecords = (checked) => this.handleSelectAllRecords(checked);
            window.selectRecord = (id, checked) => this.handleSelectRecord(id, checked);
        }

        // 主题相关的全局函数
        const themeManager = this.getComponent('theme');
        if (themeManager) {
            window.switchTheme = (themeName) => this.handleSwitchTheme(themeName);
            window.toggleTheme = () => this.handleToggleTheme();
        }

        // 翻译相关的全局函数
        const translator = this.getComponent('translator');
        if (translator) {
            window.translateUrl = (url, targetLang) => this.handleTranslateUrl(url, targetLang);
        }

        // 通用工具函数
        window.showToast = (message, type) => this.showToast(message, type);
        window.confirmAction = (message) => this.confirmAction(message);
        
        console.log('全局函数设置完成');
    }

    /**
     * 绑定全局事件
     */
    bindGlobalEvents() {
        // 页面卸载事件
        window.addEventListener('beforeunload', () => {
            this.cleanup();
        });

        // 页面可见性变化
        document.addEventListener('visibilitychange', () => {
            if (document.hidden) {
                this.emit('app:hidden');
            } else {
                this.emit('app:visible');
            }
        });

        // 网络状态变化
        window.addEventListener('online', () => {
            this.emit('network:online');
            console.log('网络连接恢复');
        });

        window.addEventListener('offline', () => {
            this.emit('network:offline');
            console.log('网络连接断开');
        });

        // 键盘快捷键
        document.addEventListener('keydown', (e) => {
            this.handleKeyboardShortcuts(e);
        });
        
        console.log('全局事件绑定完成');
    }

    /**
     * 设置全局错误处理
     */
    setupGlobalErrorHandling() {
        // 全局JavaScript错误
        window.addEventListener('error', (event) => {
            console.error('全局JavaScript错误:', event.error);
            this.emit('app:error', { 
                error: event.error, 
                filename: event.filename,
                lineno: event.lineno,
                colno: event.colno 
            });
        });

        // Promise未捕获错误
        window.addEventListener('unhandledrejection', (event) => {
            console.error('未捕获的Promise错误:', event.reason);
            this.emit('app:error', { 
                error: event.reason, 
                type: 'unhandledrejection' 
            });
        });
    }

    /**
     * 处理键盘快捷键
     * @param {KeyboardEvent} e - 键盘事件
     */
    handleKeyboardShortcuts(e) {
        // Ctrl/Cmd + / 显示帮助
        if ((e.ctrlKey || e.metaKey) && e.key === '/') {
            e.preventDefault();
            this.showHelp();
        }

        // Ctrl/Cmd + K 快速搜索
        if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
            e.preventDefault();
            const searchInput = document.getElementById('search-input');
            if (searchInput) {
                searchInput.focus();
            }
        }

        // ESC 关闭模态框
        if (e.key === 'Escape') {
            this.closeModals();
        }
    }

    // ==============================================
    // 全局函数处理器
    // ==============================================

    async handleViewRecord(id, url) {
        try {
            console.log(`查看记录: ${id}`);
            
            if (url && this.isValidUrl(url)) {
                window.open(url, '_blank');
            } else {
                // 通过API获取记录详情
                const apiManager = this.getComponent('api');
                if (apiManager) {
                    const record = await apiManager.apiCall(`/api/v2/library/${id}`);
                    if (record.original_url) {
                        window.open(record.original_url, '_blank');
                    }
                }
            }
            
            this.emit('record:viewed', { id, url });
        } catch (error) {
            console.error('查看记录失败:', error);
            this.showToast('查看记录失败', 'error');
        }
    }

    async handleDeleteRecord(id) {
        try {
            const confirmed = await this.confirmAction('确定要删除这条记录吗？此操作不可恢复。');
            if (!confirmed) return;

            const apiManager = this.getComponent('api');
            if (apiManager) {
                await apiManager.deleteRecord(id);
                this.showToast('记录删除成功', 'success');
                
                // 刷新数据
                const libraryManager = this.getComponent('library');
                if (libraryManager && libraryManager.loadData) {
                    await libraryManager.loadData();
                }
                
                this.emit('record:deleted', { id });
            }
        } catch (error) {
            console.error('删除记录失败:', error);
            this.showToast('删除记录失败', 'error');
        }
    }

    handleSelectAllRecords(checked) {
        const checkboxes = document.querySelectorAll('.row-checkbox');
        checkboxes.forEach(checkbox => {
            checkbox.checked = checked;
        });
        
        this.emit('records:selected', { selectAll: checked });
    }

    handleSelectRecord(id, checked) {
        this.emit('record:selected', { id, checked });
    }

    handleSwitchTheme(themeName) {
        const themeManager = this.getComponent('theme');
        if (themeManager) {
            themeManager.setTheme(themeName);
        }
    }

    handleToggleTheme() {
        const themeManager = this.getComponent('theme');
        if (themeManager) {
            themeManager.toggleTheme();
        }
    }

    async handleTranslateUrl(url, targetLang = 'zh') {
        try {
            const apiManager = this.getComponent('api');
            if (apiManager) {
                this.showToast('开始翻译...', 'info');
                const result = await apiManager.translatePage(url, targetLang);
                this.showToast('翻译完成', 'success');
                return result;
            }
        } catch (error) {
            console.error('翻译失败:', error);
            this.showToast('翻译失败', 'error');
        }
    }

    // ==============================================
    // 工具方法
    // ==============================================

    /**
     * 显示提示消息
     * @param {string} message - 消息内容
     * @param {string} type - 消息类型
     */
    showToast(message, type = 'info') {
        // 移除现有提示
        const existingToasts = document.querySelectorAll('.toast');
        existingToasts.forEach(toast => toast.remove());

        // 创建新提示
        const toast = document.createElement('div');
        toast.className = `toast ${type}-toast`;
        toast.textContent = message;
        toast.style.cssText = `
            position: fixed;
            top: 80px;
            right: 20px;
            padding: 12px 16px;
            border-radius: 8px;
            color: white;
            font-weight: 500;
            box-shadow: 0 4px 12px rgba(0,0,0,0.15);
            z-index: 10000;
            max-width: 300px;
            animation: slideIn 0.3s ease;
        `;

        // 设置背景色
        const colors = {
            success: '#28a745',
            error: '#dc3545',
            warning: '#ffc107',
            info: '#17a2b8'
        };
        toast.style.background = colors[type] || colors.info;

        document.body.appendChild(toast);

        // 自动移除
        setTimeout(() => {
            if (toast.parentNode) {
                toast.style.animation = 'slideOut 0.3s ease';
                setTimeout(() => toast.remove(), 300);
            }
        }, 3000);

        // 点击关闭
        toast.addEventListener('click', () => toast.remove());
    }

    /**
     * 确认对话框
     * @param {string} message - 确认消息
     * @returns {Promise<boolean>} 用户确认结果
     */
    confirmAction(message) {
        return new Promise((resolve) => {
            const result = confirm(message);
            resolve(result);
        });
    }

    /**
     * 显示帮助信息
     */
    showHelp() {
        const helpText = `
Monolith 网页翻译器 - 快捷键说明:

Ctrl/Cmd + K  - 快速搜索
Ctrl/Cmd + /  - 显示帮助
Ctrl/Cmd + Shift + T - 切换主题
ESC - 关闭模态框

更多功能请查看用户文档。
        `;
        
        alert(helpText);
    }

    /**
     * 关闭所有模态框
     */
    closeModals() {
        const modals = document.querySelectorAll('.modal, .dialog, .popup');
        modals.forEach(modal => {
            if (modal.style.display !== 'none') {
                modal.style.display = 'none';
            }
        });
    }

    /**
     * 验证URL格式
     * @param {string} url - URL字符串
     * @returns {boolean} 是否为有效URL
     */
    isValidUrl(url) {
        try {
            new URL(url);
            return true;
        } catch {
            return false;
        }
    }

    // ==============================================
    // 事件系统
    // ==============================================

    /**
     * 触发事件
     * @param {string} eventName - 事件名称
     * @param {Object} data - 事件数据
     */
    emit(eventName, data = {}) {
        const event = new CustomEvent(eventName, { detail: data });
        this.eventBus.dispatchEvent(event);
        
        if (this.config.debug) {
            console.log(`事件触发: ${eventName}`, data);
        }
    }

    /**
     * 监听事件
     * @param {string} eventName - 事件名称
     * @param {Function} callback - 回调函数
     */
    on(eventName, callback) {
        this.eventBus.addEventListener(eventName, callback);
    }

    /**
     * 移除事件监听
     * @param {string} eventName - 事件名称
     * @param {Function} callback - 回调函数
     */
    off(eventName, callback) {
        this.eventBus.removeEventListener(eventName, callback);
    }

    // ==============================================
    // 生命周期管理
    // ==============================================

    /**
     * 清理资源
     */
    cleanup() {
        console.log('开始清理应用资源...');
        
        // 清理各个组件
        this.components.forEach((component, name) => {
            if (component.destroy) {
                try {
                    component.destroy();
                    console.log(`组件清理完成: ${name}`);
                } catch (error) {
                    console.error(`组件清理失败: ${name}`, error);
                }
            }
        });

        // 清理事件监听
        this.eventBus.removeEventListener();
        
        this.emit('app:cleanup');
        console.log('应用资源清理完成');
    }

    /**
     * 重启应用
     */
    async restart() {
        console.log('重启应用...');
        this.cleanup();
        this.initialized = false;
        await this.init(this.config);
    }

    /**
     * 获取应用状态
     * @returns {Object} 应用状态信息
     */
    getStatus() {
        return {
            initialized: this.initialized,
            componentCount: this.components.size,
            config: { ...this.config },
            components: Array.from(this.components.keys())
        };
    }
}

// 创建全局应用实例
window.appManager = new AppManager();

// 导出给其他模块使用
window.AppManager = AppManager;

// 自动初始化（如果DOM已加载）
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        if (window.appManager && !window.appManager.initialized) {
            window.appManager.init();
        }
    });
} else {
    // DOM已经加载完成
    if (window.appManager && !window.appManager.initialized) {
        window.appManager.init();
    }
}