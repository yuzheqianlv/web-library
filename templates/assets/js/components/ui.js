// Monolith UI 交互组件

/**
 * UI 管理器类
 */
class UIManager {
    constructor() {
        this.isNavbarVisible = true;
        this.currentViewMode = 'translated';
        this.init();
    }

    /**
     * 初始化 UI 管理器
     */
    init() {
        this.setupScrollSync();
        this.setupKeyboardShortcuts();
        this.setupResponsiveHandling();
        this.bindEvents();
    }

    /**
     * 绑定事件
     */
    bindEvents() {
        // 窗口大小变化
        window.addEventListener('resize', () => {
            this.handleResize();
        });

        // 滚动事件
        window.addEventListener('scroll', () => {
            this.handleScroll();
        });

        // 双语模式的滚动同步
        this.setupBilingualScrollSync();
    }

    /**
     * 设置双语模式滚动同步
     */
    setupBilingualScrollSync() {
        const bilingualTranslated = document.getElementById('bilingual-translated');
        const bilingualOriginal = document.getElementById('bilingual-original');

        if (!bilingualTranslated || !bilingualOriginal) return;

        let isScrolling = false;

        const syncScroll = (source, target) => {
            if (isScrolling) return;
            isScrolling = true;

            try {
                const sourceDoc = source.contentDocument || source.contentWindow.document;
                const targetDoc = target.contentDocument || target.contentWindow.document;

                if (sourceDoc && targetDoc) {
                    const sourceHeight = sourceDoc.documentElement.scrollHeight;
                    const targetHeight = targetDoc.documentElement.scrollHeight;
                    const scrollRatio = sourceDoc.documentElement.scrollTop / (sourceHeight - sourceDoc.documentElement.clientHeight);

                    targetDoc.documentElement.scrollTop = scrollRatio * (targetHeight - targetDoc.documentElement.clientHeight);
                }
            } catch (error) {
                console.warn('滚动同步失败:', error);
            }

            setTimeout(() => {
                isScrolling = false;
            }, 100);
        };

        // 为两个iframe添加滚动事件监听
        bilingualTranslated.addEventListener('load', () => {
            try {
                const doc = bilingualTranslated.contentDocument || bilingualTranslated.contentWindow.document;
                doc.addEventListener('scroll', () => {
                    syncScroll(bilingualTranslated, bilingualOriginal);
                });
            } catch (error) {
                console.warn('无法绑定滚动事件:', error);
            }
        });

        bilingualOriginal.addEventListener('load', () => {
            try {
                const doc = bilingualOriginal.contentDocument || bilingualOriginal.contentWindow.document;
                doc.addEventListener('scroll', () => {
                    syncScroll(bilingualOriginal, bilingualTranslated);
                });
            } catch (error) {
                console.warn('无法绑定滚动事件:', error);
            }
        });
    }

    /**
     * 设置键盘快捷键
     */
    setupKeyboardShortcuts() {
        document.addEventListener('keydown', (e) => {
            // Ctrl+Enter 或 Cmd+Enter: 处理当前URL
            if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
                e.preventDefault();
                this.triggerProcessing();
            }

            // Ctrl+1/2/3: 切换查看模式
            if ((e.ctrlKey || e.metaKey) && ['1', '2', '3'].includes(e.key)) {
                e.preventDefault();
                const modes = ['translated', 'original', 'bilingual'];
                const modeIndex = parseInt(e.key) - 1;
                if (modes[modeIndex]) {
                    this.switchViewMode(modes[modeIndex]);
                }
            }

            // Ctrl+H: 切换导航栏显示
            if ((e.ctrlKey || e.metaKey) && e.key === 'h') {
                e.preventDefault();
                this.toggleNavbar();
            }

            // Escape: 隐藏提示框
            if (e.key === 'Escape') {
                this.hideAllToasts();
            }
        });
    }

    /**
     * 设置响应式处理
     */
    setupResponsiveHandling() {
        // 检测移动设备
        this.isMobile = window.innerWidth <= 768;
        
        // 移动端自动隐藏导航栏
        if (this.isMobile) {
            this.hideNavbarOnMobile();
        }
    }

    /**
     * 处理窗口大小变化
     */
    handleResize() {
        const wasMobile = this.isMobile;
        this.isMobile = window.innerWidth <= 768;

        // 如果从桌面端变为移动端
        if (!wasMobile && this.isMobile) {
            this.hideNavbarOnMobile();
        }
        // 如果从移动端变为桌面端
        else if (wasMobile && !this.isMobile) {
            this.showNavbarOnDesktop();
        }

        // 调整双语模式布局
        this.adjustBilingualLayout();
    }

    /**
     * 处理滚动事件
     */
    handleScroll() {
        // 在滚动时可以添加一些UI效果
        const scrollTop = window.pageYOffset || document.documentElement.scrollTop;
        
        // 滚动时隐藏导航栏（移动端）
        if (this.isMobile && scrollTop > 100) {
            this.autoHideNavbar();
        }
    }

    /**
     * 触发处理
     */
    triggerProcessing() {
        const translateBtn = document.getElementById('translate-btn');
        if (translateBtn && !translateBtn.disabled) {
            translateBtn.click();
        }
    }

    /**
     * 切换查看模式
     * @param {string} mode 模式名称
     */
    switchViewMode(mode) {
        if (window.monolithApp) {
            window.monolithApp.switchMode(mode);
        }
        this.currentViewMode = mode;
    }

    /**
     * 切换导航栏显示
     */
    toggleNavbar() {
        if (window.monolithApp) {
            window.monolithApp.toggleNavbar();
        }
        this.isNavbarVisible = !this.isNavbarVisible;
    }

    /**
     * 移动端隐藏导航栏
     */
    hideNavbarOnMobile() {
        if (this.isMobile && this.isNavbarVisible) {
            this.toggleNavbar();
        }
    }

    /**
     * 桌面端显示导航栏
     */
    showNavbarOnDesktop() {
        if (!this.isMobile && !this.isNavbarVisible) {
            this.toggleNavbar();
        }
    }

    /**
     * 自动隐藏导航栏
     */
    autoHideNavbar() {
        if (this.isNavbarVisible) {
            setTimeout(() => {
                if (this.isMobile) {
                    this.toggleNavbar();
                }
            }, 200);
        }
    }

    /**
     * 调整双语模式布局
     */
    adjustBilingualLayout() {
        const bilingualContainer = document.getElementById('bilingual-container');
        if (!bilingualContainer) return;

        if (this.isMobile) {
            // 移动端垂直布局
            bilingualContainer.style.flexDirection = 'column';
            const panes = bilingualContainer.querySelectorAll('.bilingual-pane');
            panes.forEach(pane => {
                pane.style.width = '100%';
                pane.style.height = '50%';
            });
        } else {
            // 桌面端水平布局
            bilingualContainer.style.flexDirection = 'row';
            const panes = bilingualContainer.querySelectorAll('.bilingual-pane');
            panes.forEach(pane => {
                pane.style.width = '50%';
                pane.style.height = '100%';
            });
        }
    }

    /**
     * 隐藏所有提示框
     */
    hideAllToasts() {
        const toasts = document.querySelectorAll('.error-toast, .success-toast');
        toasts.forEach(toast => {
            toast.classList.remove('show');
        });
    }

    /**
     * 显示确认对话框
     * @param {string} message 消息内容
     * @returns {boolean} 用户确认结果
     */
    showConfirm(message) {
        return confirm(message);
    }

    /**
     * 创建加载指示器
     * @param {HTMLElement} container 容器元素
     * @returns {HTMLElement} 加载指示器元素
     */
    createLoadingIndicator(container) {
        const loader = document.createElement('div');
        loader.className = 'loading-indicator';
        loader.innerHTML = `
            <div class="spinner"></div>
            <span>处理中...</span>
        `;
        
        if (container) {
            container.appendChild(loader);
        }
        
        return loader;
    }

    /**
     * 移除加载指示器
     * @param {HTMLElement} loader 加载指示器元素
     */
    removeLoadingIndicator(loader) {
        if (loader && loader.parentNode) {
            loader.parentNode.removeChild(loader);
        }
    }

    /**
     * 设置滚动同步
     */
    setupScrollSync() {
        // 这个方法为将来的滚动同步功能预留
        console.log('滚动同步已设置');
    }

    /**
     * 获取当前UI状态
     * @returns {Object} UI状态对象
     */
    getUIState() {
        return {
            isNavbarVisible: this.isNavbarVisible,
            currentViewMode: this.currentViewMode,
            isMobile: this.isMobile
        };
    }

    /**
     * 重置UI状态
     */
    resetUIState() {
        this.isNavbarVisible = true;
        this.currentViewMode = 'translated';
        this.hideAllToasts();
        
        // 重置导航栏显示
        const navbar = document.getElementById('navbar');
        const floatingToggle = document.getElementById('floating-toggle');
        
        if (navbar) {
            navbar.classList.remove('hidden');
        }
        if (floatingToggle) {
            floatingToggle.classList.remove('show');
        }
    }
}

// 创建全局UI管理器实例
document.addEventListener('DOMContentLoaded', () => {
    window.uiManager = new UIManager();
});