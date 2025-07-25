/**
 * Monolith 网页翻译器 - UI 管理组件
 * 整合自 library-ui.js - 提供统一的UI交互和状态管理
 */

class UIManager {
    constructor() {
        this.modals = new Map();
        this.notifications = [];
        this.loadingElements = new Set();
        this.animations = new Map();
        
        this.init();
    }

    /**
     * 初始化UI管理器
     */
    init() {
        this.setupGlobalStyles();
        this.bindGlobalEvents();
        this.initializeAnimations();
        
        console.log('UI管理器初始化完成');
    }

    /**
     * 设置全局样式
     */
    setupGlobalStyles() {
        // 注入必要的CSS样式
        const styleSheet = document.createElement('style');
        styleSheet.textContent = `
            /* 模态框样式 */
            .ui-modal {
                position: fixed;
                top: 0;
                left: 0;
                right: 0;
                bottom: 0;
                background: rgba(0, 0, 0, 0.5);
                z-index: 10000;
                display: flex;
                align-items: center;
                justify-content: center;
                opacity: 0;
                transition: opacity 0.3s ease;
            }
            
            .ui-modal.show {
                opacity: 1;
            }
            
            .ui-modal-content {
                background: var(--color-surface, white);
                border-radius: var(--border-radius, 8px);
                box-shadow: var(--shadow-large, 0 4px 20px rgba(0,0,0,0.15));
                max-width: 90vw;
                max-height: 90vh;
                overflow: auto;
                transform: scale(0.9);
                transition: transform 0.3s ease;
            }
            
            .ui-modal.show .ui-modal-content {
                transform: scale(1);
            }
            
            .ui-modal-header {
                padding: 1.5rem;
                border-bottom: 1px solid var(--color-border, #e1e5e9);
                display: flex;
                align-items: center;
                justify-content: space-between;
            }
            
            .ui-modal-title {
                font-size: 1.25rem;
                font-weight: 600;
                color: var(--color-text-primary, #333);
                margin: 0;
            }
            
            .ui-modal-close {
                background: none;
                border: none;
                font-size: 1.5rem;
                cursor: pointer;
                color: var(--color-text-secondary, #666);
                transition: color 0.2s ease;
            }
            
            .ui-modal-close:hover {
                color: var(--color-text-primary, #333);
            }
            
            .ui-modal-body {
                padding: 1.5rem;
            }
            
            .ui-modal-footer {
                padding: 1rem 1.5rem;
                border-top: 1px solid var(--color-border, #e1e5e9);
                display: flex;
                gap: 0.75rem;
                justify-content: flex-end;
            }
            
            /* 加载指示器 */
            .ui-loading {
                position: relative;
                overflow: hidden;
            }
            
            .ui-loading::after {
                content: '';
                position: absolute;
                top: 0;
                left: -100%;
                width: 100%;
                height: 100%;
                background: linear-gradient(90deg, transparent, rgba(255,255,255,0.4), transparent);
                animation: shimmer 1.5s infinite;
            }
            
            @keyframes shimmer {
                0% { left: -100%; }
                100% { left: 100%; }
            }
            
            .ui-spinner {
                display: inline-block;
                width: 20px;
                height: 20px;
                border: 2px solid var(--color-border, #e1e5e9);
                border-top: 2px solid var(--color-primary, #667eea);
                border-radius: 50%;
                animation: spin 1s linear infinite;
            }
            
            @keyframes spin {
                0% { transform: rotate(0deg); }
                100% { transform: rotate(360deg); }
            }
            
            /* 通知样式 */
            .ui-notification {
                position: fixed;
                top: 80px;
                right: 20px;
                min-width: 300px;
                max-width: 400px;
                padding: 1rem 1.25rem;
                border-radius: var(--border-radius, 8px);
                box-shadow: var(--shadow-large, 0 4px 16px rgba(0,0,0,0.1));
                z-index: 10001;
                transform: translateX(100%);
                transition: transform 0.3s ease;
                cursor: pointer;
            }
            
            .ui-notification.show {
                transform: translateX(0);
            }
            
            .ui-notification-success {
                background: var(--color-success, #28a745);
                color: white;
            }
            
            .ui-notification-error {
                background: var(--color-error, #dc3545);
                color: white;
            }
            
            .ui-notification-warning {
                background: var(--color-warning, #ffc107);
                color: #333;
            }
            
            .ui-notification-info {
                background: var(--color-info, #17a2b8);
                color: white;
            }
            
            /* 工具提示 */
            .ui-tooltip {
                position: absolute;
                background: rgba(0, 0, 0, 0.8);
                color: white;
                padding: 0.5rem 0.75rem;
                border-radius: 4px;
                font-size: 0.875rem;
                white-space: nowrap;
                z-index: 10002;
                opacity: 0;
                transition: opacity 0.2s ease;
                pointer-events: none;
            }
            
            .ui-tooltip.show {
                opacity: 1;
            }
            
            /* 确认对话框 */
            .ui-confirm {
                width: 400px;
            }
            
            .ui-confirm-message {
                margin-bottom: 1.5rem;
                color: var(--color-text-primary, #333);
                line-height: 1.5;
            }
            
            /* 响应式设计 */
            @media (max-width: 768px) {
                .ui-modal-content {
                    width: 95vw;
                    margin: 1rem;
                }
                
                .ui-notification {
                    left: 10px;
                    right: 10px;
                    max-width: none;
                    min-width: auto;
                }
                
                .ui-confirm {
                    width: auto;
                }
            }
        `;
        
        document.head.appendChild(styleSheet);
    }

    /**
     * 绑定全局事件
     */
    bindGlobalEvents() {
        // ESC 键关闭模态框
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape') {
                this.closeTopModal();
            }
        });

        // 点击背景关闭模态框
        document.addEventListener('click', (e) => {
            if (e.target.classList.contains('ui-modal')) {
                this.closeModal(e.target.dataset.modalId);
            }
        });
    }

    /**
     * 初始化动画
     */
    initializeAnimations() {
        this.animations.set('fadeIn', 'opacity 0.3s ease');
        this.animations.set('slideIn', 'transform 0.3s ease');
        this.animations.set('scale', 'transform 0.3s ease');
    }

    // ==============================================
    // 模态框管理
    // ==============================================

    /**
     * 显示模态框
     * @param {Object} options - 模态框选项
     * @returns {string} 模态框ID
     */
    showModal(options = {}) {
        const {
            title = '提示',
            content = '',
            size = 'medium',
            closable = true,
            backdrop = true,
            buttons = [],
            onShow = null,
            onHide = null
        } = options;

        const modalId = this.generateId();
        
        const modal = document.createElement('div');
        modal.className = 'ui-modal';
        modal.dataset.modalId = modalId;
        
        modal.innerHTML = `
            <div class="ui-modal-content ui-modal-${size}">
                <div class="ui-modal-header">
                    <h3 class="ui-modal-title">${title}</h3>
                    ${closable ? '<button class="ui-modal-close">&times;</button>' : ''}
                </div>
                <div class="ui-modal-body">
                    ${content}
                </div>
                ${buttons.length > 0 ? `
                    <div class="ui-modal-footer">
                        ${buttons.map(btn => `
                            <button class="btn ${btn.class || 'btn-secondary'}" 
                                    data-action="${btn.action || 'close'}">
                                ${btn.text}
                            </button>
                        `).join('')}
                    </div>
                ` : ''}
            </div>
        `;

        document.body.appendChild(modal);

        // 绑定事件
        if (closable) {
            const closeBtn = modal.querySelector('.ui-modal-close');
            if (closeBtn) {
                closeBtn.addEventListener('click', () => this.closeModal(modalId));
            }
        }

        // 绑定按钮事件
        const actionBtns = modal.querySelectorAll('[data-action]');
        actionBtns.forEach(btn => {
            btn.addEventListener('click', () => {
                const action = btn.dataset.action;
                if (action === 'close') {
                    this.closeModal(modalId);
                } else if (typeof options[action] === 'function') {
                    options[action](modalId);
                }
            });
        });

        // 显示动画
        requestAnimationFrame(() => {
            modal.classList.add('show');
        });

        this.modals.set(modalId, {
            element: modal,
            options,
            onShow,
            onHide
        });

        if (onShow) onShow(modalId);
        
        return modalId;
    }

    /**
     * 关闭模态框
     * @param {string} modalId - 模态框ID
     */
    closeModal(modalId) {
        const modalInfo = this.modals.get(modalId);
        if (!modalInfo) return;

        const { element, onHide } = modalInfo;
        
        element.classList.remove('show');
        
        setTimeout(() => {
            if (element.parentNode) {
                element.parentNode.removeChild(element);
            }
            this.modals.delete(modalId);
            
            if (onHide) onHide(modalId);
        }, 300);
    }

    /**
     * 关闭最顶层模态框
     */
    closeTopModal() {
        const modalIds = Array.from(this.modals.keys());
        if (modalIds.length > 0) {
            this.closeModal(modalIds[modalIds.length - 1]);
        }
    }

    /**
     * 关闭所有模态框
     */
    closeAllModals() {
        Array.from(this.modals.keys()).forEach(modalId => {
            this.closeModal(modalId);
        });
    }

    // ==============================================
    // 确认对话框
    // ==============================================

    /**
     * 显示确认对话框
     * @param {string} message - 确认消息
     * @param {Object} options - 选项
     * @returns {Promise<boolean>} 用户选择结果
     */
    confirm(message, options = {}) {
        return new Promise((resolve) => {
            const {
                title = '确认',
                confirmText = '确定',
                cancelText = '取消',
                type = 'warning'
            } = options;

            const modalId = this.showModal({
                title,
                content: `<div class="ui-confirm-message">${message}</div>`,
                size: 'small',
                buttons: [
                    { text: cancelText, action: 'cancel', class: 'btn-secondary' },
                    { text: confirmText, action: 'confirm', class: `btn-${type === 'danger' ? 'danger' : 'primary'}` }
                ],
                cancel: () => {
                    this.closeModal(modalId);
                    resolve(false);
                },
                confirm: () => {
                    this.closeModal(modalId);
                    resolve(true);
                }
            });
        });
    }

    /**
     * 显示警告对话框
     * @param {string} message - 警告消息
     * @param {Object} options - 选项
     * @returns {Promise<void>}
     */
    alert(message, options = {}) {
        return new Promise((resolve) => {
            const {
                title = '提示',
                buttonText = '确定'
            } = options;

            const modalId = this.showModal({
                title,
                content: `<div class="ui-confirm-message">${message}</div>`,
                size: 'small',
                buttons: [
                    { text: buttonText, action: 'ok', class: 'btn-primary' }
                ],
                ok: () => {
                    this.closeModal(modalId);
                    resolve();
                }
            });
        });
    }

    // ==============================================
    // 通知系统
    // ==============================================

    /**
     * 显示通知
     * @param {string} message - 通知消息
     * @param {string} type - 通知类型
     * @param {Object} options - 选项
     * @returns {string} 通知ID
     */
    notify(message, type = 'info', options = {}) {
        const {
            duration = 4000,
            closable = true,
            position = 'top-right'
        } = options;

        const notificationId = this.generateId();
        
        const notification = document.createElement('div');
        notification.className = `ui-notification ui-notification-${type}`;
        notification.dataset.notificationId = notificationId;
        notification.textContent = message;

        // 设置位置
        this.setNotificationPosition(notification, position);

        document.body.appendChild(notification);

        // 显示动画
        requestAnimationFrame(() => {
            notification.classList.add('show');
        });

        // 自动关闭
        const timer = setTimeout(() => {
            this.closeNotification(notificationId);
        }, duration);

        // 点击关闭
        if (closable) {
            notification.addEventListener('click', () => {
                clearTimeout(timer);
                this.closeNotification(notificationId);
            });
        }

        this.notifications.push({
            id: notificationId,
            element: notification,
            timer
        });

        return notificationId;
    }

    /**
     * 设置通知位置
     * @param {HTMLElement} element - 通知元素
     * @param {string} position - 位置
     */
    setNotificationPosition(element, position) {
        const positions = {
            'top-right': { top: '80px', right: '20px' },
            'top-left': { top: '80px', left: '20px' },
            'bottom-right': { bottom: '20px', right: '20px' },
            'bottom-left': { bottom: '20px', left: '20px' },
            'top-center': { top: '80px', left: '50%', transform: 'translateX(-50%)' },
            'bottom-center': { bottom: '20px', left: '50%', transform: 'translateX(-50%)' }
        };

        const positionStyle = positions[position] || positions['top-right'];
        Object.assign(element.style, positionStyle);
    }

    /**
     * 关闭通知
     * @param {string} notificationId - 通知ID
     */
    closeNotification(notificationId) {
        const notificationIndex = this.notifications.findIndex(n => n.id === notificationId);
        if (notificationIndex === -1) return;

        const { element, timer } = this.notifications[notificationIndex];
        
        clearTimeout(timer);
        element.classList.remove('show');
        
        setTimeout(() => {
            if (element.parentNode) {
                element.parentNode.removeChild(element);
            }
        }, 300);

        this.notifications.splice(notificationIndex, 1);
    }

    // ==============================================
    // 加载状态管理
    // ==============================================

    /**
     * 显示加载状态
     * @param {HTMLElement|string} target - 目标元素或选择器
     * @param {Object} options - 选项
     */
    showLoading(target, options = {}) {
        const element = typeof target === 'string' ? document.querySelector(target) : target;
        if (!element) return;

        const {
            type = 'shimmer', // shimmer, spinner, overlay
            message = '',
            disableElement = true
        } = options;

        if (disableElement && element.tagName === 'BUTTON') {
            element.disabled = true;
        }

        if (type === 'shimmer') {
            element.classList.add('ui-loading');
        } else if (type === 'spinner') {
            const spinner = document.createElement('span');
            spinner.className = 'ui-spinner';
            spinner.dataset.loadingSpinner = 'true';
            element.appendChild(spinner);
        } else if (type === 'overlay') {
            const overlay = document.createElement('div');
            overlay.className = 'ui-loading-overlay';
            overlay.innerHTML = `
                <div class="ui-spinner"></div>
                ${message ? `<div class="ui-loading-message">${message}</div>` : ''}
            `;
            overlay.style.cssText = `
                position: absolute;
                top: 0; left: 0; right: 0; bottom: 0;
                background: rgba(255,255,255,0.8);
                display: flex; flex-direction: column;
                align-items: center; justify-content: center;
                z-index: 1000;
            `;
            
            element.style.position = 'relative';
            element.appendChild(overlay);
        }

        this.loadingElements.add(element);
    }

    /**
     * 隐藏加载状态
     * @param {HTMLElement|string} target - 目标元素或选择器
     */
    hideLoading(target) {
        const element = typeof target === 'string' ? document.querySelector(target) : target;
        if (!element) return;

        // 移除shimmer效果
        element.classList.remove('ui-loading');

        // 移除spinner
        const spinners = element.querySelectorAll('[data-loading-spinner]');
        spinners.forEach(spinner => spinner.remove());

        // 移除overlay
        const overlays = element.querySelectorAll('.ui-loading-overlay');
        overlays.forEach(overlay => overlay.remove());

        // 重新启用元素
        if (element.tagName === 'BUTTON') {
            element.disabled = false;
        }

        this.loadingElements.delete(element);
    }

    // ==============================================
    // 工具提示
    // ==============================================

    /**
     * 显示工具提示
     * @param {HTMLElement} target - 目标元素
     * @param {string} text - 提示文本
     * @param {Object} options - 选项
     */
    showTooltip(target, text, options = {}) {
        const {
            position = 'top',
            delay = 500
        } = options;

        const tooltip = document.createElement('div');
        tooltip.className = 'ui-tooltip';
        tooltip.textContent = text;
        
        document.body.appendChild(tooltip);

        const targetRect = target.getBoundingClientRect();
        const tooltipRect = tooltip.getBoundingClientRect();
        
        let top, left;
        
        switch (position) {
            case 'top':
                top = targetRect.top - tooltipRect.height - 8;
                left = targetRect.left + (targetRect.width - tooltipRect.width) / 2;
                break;
            case 'bottom':
                top = targetRect.bottom + 8;
                left = targetRect.left + (targetRect.width - tooltipRect.width) / 2;
                break;
            case 'left':
                top = targetRect.top + (targetRect.height - tooltipRect.height) / 2;
                left = targetRect.left - tooltipRect.width - 8;
                break;
            case 'right':
                top = targetRect.top + (targetRect.height - tooltipRect.height) / 2;
                left = targetRect.right + 8;
                break;
        }

        tooltip.style.top = Math.max(8, top) + 'px';
        tooltip.style.left = Math.max(8, Math.min(window.innerWidth - tooltipRect.width - 8, left)) + 'px';

        setTimeout(() => {
            tooltip.classList.add('show');
        }, delay);

        // 自动移除
        target.addEventListener('mouseleave', () => {
            tooltip.remove();
        }, { once: true });
    }

    // ==============================================
    // 实用工具
    // ==============================================

    /**
     * 生成唯一ID
     * @returns {string} 唯一ID
     */
    generateId() {
        return 'ui_' + Math.random().toString(36).substr(2, 9) + Date.now().toString(36);
    }

    /**
     * 平滑滚动到元素
     * @param {HTMLElement|string} target - 目标元素或选择器
     * @param {Object} options - 选项
     */
    scrollTo(target, options = {}) {
        const element = typeof target === 'string' ? document.querySelector(target) : target;
        if (!element) return;

        const {
            behavior = 'smooth',
            block = 'start',
            inline = 'nearest',
            offset = 0
        } = options;

        const elementRect = element.getBoundingClientRect();
        const targetTop = window.pageYOffset + elementRect.top - offset;

        window.scrollTo({
            top: targetTop,
            behavior
        });
    }

    /**
     * 复制文本到剪贴板
     * @param {string} text - 要复制的文本
     * @returns {Promise<boolean>} 复制是否成功
     */
    async copyToClipboard(text) {
        try {
            if (navigator.clipboard && window.isSecureContext) {
                await navigator.clipboard.writeText(text);
                return true;
            } else {
                // 降级方案
                const textArea = document.createElement('textarea');
                textArea.value = text;
                textArea.style.cssText = 'position: fixed; top: -1000px; left: -1000px;';
                document.body.appendChild(textArea);
                textArea.focus();
                textArea.select();
                
                const successful = document.execCommand('copy');
                document.body.removeChild(textArea);
                return successful;
            }
        } catch (error) {
            console.error('复制到剪贴板失败:', error);
            return false;
        }
    }

    /**
     * 检查元素是否在视口中
     * @param {HTMLElement} element - 要检查的元素
     * @param {number} threshold - 阈值（0-1）
     * @returns {boolean} 是否在视口中
     */
    isInViewport(element, threshold = 0) {
        const rect = element.getBoundingClientRect();
        const windowHeight = window.innerHeight || document.documentElement.clientHeight;
        const windowWidth = window.innerWidth || document.documentElement.clientWidth;
        
        const vertInView = (rect.top + rect.height * threshold) <= windowHeight && 
                          (rect.top + rect.height * (1 - threshold)) >= 0;
        const horInView = (rect.left + rect.width * threshold) <= windowWidth && 
                         (rect.left + rect.width * (1 - threshold)) >= 0;
        
        return vertInView && horInView;
    }

    /**
     * 防抖函数
     * @param {Function} func - 要防抖的函数
     * @param {number} wait - 等待时间
     * @returns {Function} 防抖后的函数
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
     * 销毁UI管理器
     */
    destroy() {
        this.closeAllModals();
        this.notifications.forEach(n => this.closeNotification(n.id));
        this.loadingElements.forEach(element => this.hideLoading(element));
        
        console.log('UI管理器已销毁');
    }
}

// 导出给全局使用
window.UIManager = UIManager;