/**
 * 导航栏切换组件
 * 
 * 功能：
 * - 隐藏/显示导航栏
 * - 浮动切换按钮管理
 * - 响应式适配
 * - 键盘快捷键支持
 * - 动画状态管理
 * 
 * 使用方式：
 * const navToggle = new NavToggleComponent({
 *     navbarSelector: '#navbar',
 *     toggleBtnSelector: '#toggle-nav-btn',
 *     floatingToggleSelector: '#floating-toggle'
 * });
 */

class NavToggleComponent {
    constructor(options = {}) {
        // 配置选项
        this.options = {
            navbarSelector: options.navbarSelector || '#navbar',
            toggleBtnSelector: options.toggleBtnSelector || '#toggle-nav-btn',
            floatingToggleSelector: options.floatingToggleSelector || '#floating-toggle',
            mainContentSelector: options.mainContentSelector || '#main-content',
            
            // 动画配置
            animationDuration: options.animationDuration || 300,
            
            // 响应式配置
            mobileBreakpoint: options.mobileBreakpoint || 768,
            autoHideOnMobile: options.autoHideOnMobile !== false,
            
            // 键盘快捷键
            enableKeyboardShortcuts: options.enableKeyboardShortcuts !== false,
            shortcutKey: options.shortcutKey || 'h', // Ctrl+H
            
            // 回调函数
            onToggle: options.onToggle || null,
            onShow: options.onShow || null,
            onHide: options.onHide || null
        };

        // 状态管理
        this.isVisible = true;
        this.isMobile = false;
        this.isAnimating = false;

        // DOM元素引用
        this.elements = {};

        // 初始化
        this.init();
    }

    /**
     * 初始化组件
     */
    init() {
        this.initElements();
        this.bindEvents();
        this.setupResponsive();
        
        if (this.options.enableKeyboardShortcuts) {
            this.setupKeyboardShortcuts();
        }

        console.log('🎛️ NavToggleComponent 已初始化');
    }

    /**
     * 初始化DOM元素引用
     */
    initElements() {
        this.elements = {
            navbar: document.querySelector(this.options.navbarSelector),
            toggleBtn: document.querySelector(this.options.toggleBtnSelector),
            floatingToggle: document.querySelector(this.options.floatingToggleSelector),
            mainContent: document.querySelector(this.options.mainContentSelector)
        };

        // 验证必需的元素
        if (!this.elements.navbar) {
            console.error('NavToggleComponent: 找不到导航栏元素', this.options.navbarSelector);
            return;
        }

        if (!this.elements.toggleBtn) {
            console.warn('NavToggleComponent: 找不到切换按钮', this.options.toggleBtnSelector);
        }

        if (!this.elements.floatingToggle) {
            console.warn('NavToggleComponent: 找不到浮动按钮', this.options.floatingToggleSelector);
        }
    }

    /**
     * 绑定事件监听器
     */
    bindEvents() {
        // 主切换按钮事件
        if (this.elements.toggleBtn) {
            this.elements.toggleBtn.addEventListener('click', (e) => {
                e.preventDefault();
                this.toggle();
            });
        }

        // 浮动切换按钮事件
        if (this.elements.floatingToggle) {
            this.elements.floatingToggle.addEventListener('click', (e) => {
                e.preventDefault();
                this.show();
            });
        }

        // 窗口大小变化
        window.addEventListener('resize', this.debounce(() => {
            this.handleResize();
        }, 100));

        // 滚动事件（移动端自动隐藏）
        if (this.options.autoHideOnMobile) {
            window.addEventListener('scroll', this.throttle(() => {
                this.handleScroll();
            }, 100));
        }
    }

    /**
     * 设置键盘快捷键
     */
    setupKeyboardShortcuts() {
        document.addEventListener('keydown', (e) => {
            // Ctrl+H 或 Cmd+H
            if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === this.options.shortcutKey) {
                e.preventDefault();
                this.toggle();
            }
        });
    }

    /**
     * 设置响应式处理
     */
    setupResponsive() {
        this.checkMobile();
        
        if (this.isMobile && this.options.autoHideOnMobile) {
            this.hide(false); // 移动端默认隐藏，不触发动画
        }
    }

    /**
     * 切换导航栏显示状态
     */
    toggle() {
        if (this.isAnimating) {
            return;
        }

        if (this.isVisible) {
            this.hide();
        } else {
            this.show();
        }
    }

    /**
     * 显示导航栏
     * @param {boolean} animated - 是否使用动画，默认true
     */
    show(animated = true) {
        if (this.isVisible || this.isAnimating) {
            return;
        }

        this.isAnimating = true;
        this.isVisible = true;

        // 移除隐藏类
        this.elements.navbar.classList.remove('hidden');

        // 隐藏浮动按钮
        if (this.elements.floatingToggle) {
            this.elements.floatingToggle.classList.remove('show');
        }

        // 调整主内容区域
        if (this.elements.mainContent) {
            this.elements.mainContent.classList.remove('nav-hidden');
        }

        // 触发回调
        if (this.options.onShow) {
            this.options.onShow();
        }

        if (this.options.onToggle) {
            this.options.onToggle(this.isVisible);
        }

        // 动画结束后重置状态
        if (animated) {
            setTimeout(() => {
                this.isAnimating = false;
            }, this.options.animationDuration);
        } else {
            this.isAnimating = false;
        }
    }

    /**
     * 隐藏导航栏
     * @param {boolean} animated - 是否使用动画，默认true
     */
    hide(animated = true) {
        if (!this.isVisible || this.isAnimating) {
            return;
        }

        this.isAnimating = true;
        this.isVisible = false;

        // 添加隐藏类
        this.elements.navbar.classList.add('hidden');

        // 显示浮动按钮
        if (this.elements.floatingToggle) {
            this.elements.floatingToggle.classList.add('show');
        }

        // 调整主内容区域
        if (this.elements.mainContent) {
            this.elements.mainContent.classList.add('nav-hidden');
        }

        // 触发回调
        if (this.options.onHide) {
            this.options.onHide();
        }

        if (this.options.onToggle) {
            this.options.onToggle(this.isVisible);
        }

        // 动画结束后重置状态
        if (animated) {
            setTimeout(() => {
                this.isAnimating = false;
            }, this.options.animationDuration);
        } else {
            this.isAnimating = false;
        }
    }

    /**
     * 检查是否为移动设备
     */
    checkMobile() {
        this.isMobile = window.innerWidth <= this.options.mobileBreakpoint;
    }

    /**
     * 处理窗口大小变化
     */
    handleResize() {
        const wasMobile = this.isMobile;
        this.checkMobile();

        // 从桌面端变为移动端
        if (!wasMobile && this.isMobile && this.options.autoHideOnMobile) {
            this.hide(false);
        }
        // 从移动端变为桌面端
        else if (wasMobile && !this.isMobile) {
            this.show(false);
        }
    }

    /**
     * 处理滚动事件（移动端自动隐藏）
     */
    handleScroll() {
        if (!this.isMobile || !this.options.autoHideOnMobile) {
            return;
        }

        const scrollTop = window.pageYOffset || document.documentElement.scrollTop;
        
        // 滚动超过100px时自动隐藏
        if (scrollTop > 100 && this.isVisible) {
            this.hide();
        }
    }

    /**
     * 获取当前状态
     */
    getState() {
        return {
            isVisible: this.isVisible,
            isMobile: this.isMobile,
            isAnimating: this.isAnimating
        };
    }

    /**
     * 更新配置
     */
    updateOptions(newOptions) {
        Object.assign(this.options, newOptions);
    }

    /**
     * 销毁组件
     */
    destroy() {
        // 移除事件监听器
        if (this.elements.toggleBtn) {
            this.elements.toggleBtn.removeEventListener('click', this.toggle);
        }

        if (this.elements.floatingToggle) {
            this.elements.floatingToggle.removeEventListener('click', this.show);
        }

        // 重置DOM状态
        if (this.elements.navbar) {
            this.elements.navbar.classList.remove('hidden');
        }

        if (this.elements.floatingToggle) {
            this.elements.floatingToggle.classList.remove('show');
        }

        if (this.elements.mainContent) {
            this.elements.mainContent.classList.remove('nav-hidden');
        }

        console.log('🎛️ NavToggleComponent 已销毁');
    }

    /**
     * 防抖函数
     */
    debounce(func, wait) {
        let timeout;
        return function executedFunction(...args) {
            const later = () => {
                clearTimeout(timeout);
                func(...args);
            };
            clearTimeout(timeout);
            timeout = setTimeout(later, wait);
        };
    }

    /**
     * 节流函数
     */
    throttle(func, limit) {
        let inThrottle;
        return function executedFunction(...args) {
            if (!inThrottle) {
                func.apply(this, args);
                inThrottle = true;
                setTimeout(() => inThrottle = false, limit);
            }
        };
    }
}

// 自动初始化全局实例（可选）
document.addEventListener('DOMContentLoaded', () => {
    // 检查是否已有其他初始化方式
    if (!window.navToggleComponent) {
        window.navToggleComponent = new NavToggleComponent();
    }
});

// 导出类以供其他模块使用
if (typeof module !== 'undefined' && module.exports) {
    module.exports = NavToggleComponent;
} else {
    window.NavToggleComponent = NavToggleComponent;
}