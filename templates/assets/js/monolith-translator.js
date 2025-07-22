/**
 * Monolith 网页翻译器 - 主 JavaScript 文件
 * 负责处理翻译请求、UI 交互和双语同步滚动
 */

class MonolithTranslator {
    constructor() {
        this.initElements();
        this.bindEvents();
        this.currentMode = 'translated';
        this.navbarVisible = true;
        this.initThemeIntegration();
        this.initScrollSync();
    }

    /**
     * 初始化 DOM 元素引用
     */
    initElements() {
        this.navbar = document.getElementById('navbar');
        this.mainContent = document.getElementById('main-content');
        this.toggleNavBtn = document.getElementById('toggle-nav-btn');
        this.floatingToggle = document.getElementById('floating-toggle');
        this.urlInput = document.getElementById('url-input');
        this.translateBtn = document.getElementById('translate-btn');
        this.modeBtns = document.querySelectorAll('.mode-btn');
        this.loading = document.getElementById('loading');
        this.errorToast = document.getElementById('error-toast');
        this.emptyState = document.getElementById('empty-state');
        this.translatedFrame = document.getElementById('translated-frame');
        this.originalFrame = document.getElementById('original-frame');
        this.bilingualContainer = document.getElementById('bilingual-container');
        this.bilingualTranslated = document.getElementById('bilingual-translated');
        this.bilingualOriginal = document.getElementById('bilingual-original');
        this.syncIndicator = document.getElementById('sync-indicator');
        this.syncStatus = document.getElementById('sync-status');
    }

    /**
     * 绑定事件监听器
     */
    bindEvents() {
        this.toggleNavBtn.addEventListener('click', () => this.toggleNavbar());
        this.floatingToggle.addEventListener('click', () => this.toggleNavbar());
        this.translateBtn.addEventListener('click', () => this.handleTranslate());
        this.urlInput.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') this.handleTranslate();
        });

        this.modeBtns.forEach(btn => {
            btn.addEventListener('click', () => this.switchMode(btn.dataset.mode));
        });

        // 点击错误提示关闭
        this.errorToast.addEventListener('click', () => this.hideError());
        
        // 点击成功提示关闭
        const successToast = document.getElementById('success-toast');
        if (successToast) {
            successToast.addEventListener('click', () => this.hideSuccess());
        }
        
        // 监听来自iframe的链接点击消息
        window.addEventListener('message', (event) => {
            if (event.data && event.data.type === 'link-clicked') {
                this.handleLinkClick(event.data.url);
            }
        });
    }

    /**
     * 切换导航栏显示/隐藏
     */
    toggleNavbar() {
        this.navbarVisible = !this.navbarVisible;
        this.navbar.classList.toggle('hidden', !this.navbarVisible);
        this.mainContent.classList.toggle('nav-hidden', !this.navbarVisible);
        this.floatingToggle.classList.toggle('show', !this.navbarVisible);
    }

    /**
     * 处理翻译请求
     */
    async handleTranslate() {
        const url = this.urlInput.value.trim();
        if (!url) {
            this.showError('请输入有效的 URL');
            return;
        }

        if (!this.isValidUrl(url)) {
            this.showError('请输入正确的 URL 格式');
            return;
        }

        this.showLoading();
        this.hideError();

        try {
            const response = await fetch('/api/translate', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({
                    url: url,
                    target_lang: "zh"
                })
            });

            if (!response.ok) {
                const errorData = await response.json();
                throw new Error(errorData.message || `HTTP ${response.status}`);
            }

            const data = await response.json();
            this.displayContent(data);
        } catch (error) {
            this.showError(`翻译失败: ${error.message}`);
        } finally {
            this.hideLoading();
        }
    }

    /**
     * 显示翻译结果
     * @param {Object} data - 翻译结果数据
     */
    displayContent(data) {
        // 创建 Blob URL 来显示 HTML 内容
        const translatedBlob = new Blob([data.translated_html], { type: 'text/html' });
        const originalBlob = new Blob([data.original_html], { type: 'text/html' });
        
        const translatedUrl = URL.createObjectURL(translatedBlob);
        const originalUrl = URL.createObjectURL(originalBlob);
        
        // 设置所有 iframe 的 src
        this.translatedFrame.src = translatedUrl;
        this.originalFrame.src = originalUrl;
        this.bilingualTranslated.src = translatedUrl;
        this.bilingualOriginal.src = originalUrl;
        
        // 隐藏空状态，显示内容
        this.emptyState.classList.add('hidden');
        this.switchMode(this.currentMode);
        
        // 设置双语模式的同步滚动
        if (this.scrollSyncManager) {
            this.scrollSyncManager.setupBilingualSync();
        } else if (this.simpleScrollSync) {
            this.simpleScrollSync.start();
        } else {
            this.setupBilingualSync();
        }
        
        // 设置链接拦截
        this.setupLinkInterception();
    }

    /**
     * 切换显示模式
     * @param {string} mode - 显示模式：'translated', 'original', 'bilingual'
     */
    switchMode(mode) {
        this.currentMode = mode;
        
        // 更新按钮状态
        this.modeBtns.forEach(btn => {
            btn.classList.toggle('active', btn.dataset.mode === mode);
        });

        // 隐藏所有内容
        this.translatedFrame.classList.remove('active');
        this.originalFrame.classList.remove('active');
        this.bilingualContainer.classList.remove('active');

        // 显示对应模式的内容
        switch (mode) {
            case 'translated':
                this.translatedFrame.classList.add('active');
                this.syncIndicator.classList.remove('show');
                break;
            case 'original':
                this.originalFrame.classList.add('active');
                this.syncIndicator.classList.remove('show');
                break;
            case 'bilingual':
                this.bilingualContainer.classList.add('active');
                // 延迟显示同步指示器，确保同步已设置
                setTimeout(() => {
                    if (this.currentMode === 'bilingual') {
                        this.syncIndicator.classList.add('show');
                    }
                }, 1000);
                break;
        }
    }

    /**
     * 显示加载状态
     */
    showLoading() {
        this.loading.classList.add('show');
        this.translateBtn.disabled = true;
        this.translateBtn.textContent = '处理中...';
    }

    /**
     * 隐藏加载状态
     */
    hideLoading() {
        this.loading.classList.remove('show');
        this.translateBtn.disabled = false;
        this.translateBtn.textContent = '翻译';
    }

    /**
     * 显示错误提示
     * @param {string} message - 错误信息
     */
    showError(message) {
        this.errorToast.textContent = message;
        this.errorToast.classList.add('show');
        
        // 3秒后自动隐藏
        setTimeout(() => this.hideError(), 3000);
    }

    /**
     * 隐藏错误提示
     */
    hideError() {
        this.errorToast.classList.remove('show');
    }

    /**
     * 显示成功提示
     * @param {string} message - 成功信息
     */
    showSuccess(message) {
        const successToast = document.getElementById('success-toast');
        if (successToast) {
            successToast.textContent = message;
            successToast.classList.add('show');
            
            // 3秒后自动隐藏
            setTimeout(() => this.hideSuccess(), 3000);
        }
    }

    /**
     * 隐藏成功提示
     */
    hideSuccess() {
        const successToast = document.getElementById('success-toast');
        if (successToast) {
            successToast.classList.remove('show');
        }
    }

    /**
     * 初始化主题集成
     */
    initThemeIntegration() {
        // 监听主题变化
        if (window.themeManager) {
            window.themeManager.onThemeChange((newTheme, oldTheme) => {
                this.onThemeChanged(newTheme, oldTheme);
            });
        }
    }

    /**
     * 初始化滚动同步管理器
     */
    initScrollSync() {
        if (window.ImprovedScrollSync) {
            this.scrollSyncManager = new window.ImprovedScrollSync(this);
            console.log('高级滚动同步管理器已初始化');
        } else if (window.SimpleScrollSync) {
            this.simpleScrollSync = new window.SimpleScrollSync(this);
            console.log('简单滚动同步管理器已初始化');
        } else {
            console.warn('滚动同步库未加载，将使用原始同步方法');
        }
    }

    /**
     * 主题变化处理
     * @param {string} newTheme - 新主题
     * @param {string} oldTheme - 旧主题
     */
    onThemeChanged(newTheme, oldTheme) {
        // 显示主题切换成功提示
        const themeInfo = window.themeManager?.getCurrentTheme();
        if (themeInfo) {
            this.showSuccess(`已切换到${themeInfo.display_name}`);
        }

        // 更新iframe内容的主题适配
        this.updateIframeThemeAdaptation(newTheme);
        
        // 重新设置同步滚动（可能需要重新初始化）
        if (this.currentMode === 'bilingual') {
            setTimeout(() => {
                if (this.scrollSyncManager) {
                    this.scrollSyncManager.setupBilingualSync();
                } else {
                    this.setupBilingualSync();
                }
            }, 500);
        }
    }

    /**
     * 更新iframe主题适配
     * @param {string} themeName - 主题名称
     */
    updateIframeThemeAdaptation(themeName) {
        const isDarkTheme = window.themeManager?.isCurrentThemeDark() || false;
        
        // 为iframe添加主题相关的类
        const frames = [
            this.translatedFrame,
            this.originalFrame,
            this.bilingualTranslated,
            this.bilingualOriginal
        ];

        frames.forEach(frame => {
            if (frame && frame.contentDocument) {
                try {
                    const frameBody = frame.contentDocument.body;
                    if (frameBody) {
                        frameBody.className = frameBody.className
                            .replace(/theme-\w+/g, '')
                            .trim() + ` theme-${themeName}`;
                    }
                } catch (error) {
                    // 跨域限制，忽略错误
                    console.debug('Cannot access iframe content due to CORS:', error);
                }
            }
        });
    }

    /**
     * 设置双语模式同步滚动
     */
    setupBilingualSync() {
        // 确保 iframe 加载完成后再设置同步
        let loadedCount = 0;
        const totalFrames = 2;
        
        const setupSync = () => {
            loadedCount++;
            if (loadedCount === totalFrames) {
                this.initScrollSync();
            }
        };

        // 监听 iframe 加载完成事件
        this.bilingualTranslated.addEventListener('load', setupSync);
        this.bilingualOriginal.addEventListener('load', setupSync);
    }

    /**
     * 初始化滚动同步功能
     */
    initScrollSync() {
        try {
            const translatedDoc = this.bilingualTranslated.contentDocument || this.bilingualTranslated.contentWindow.document;
            const originalDoc = this.bilingualOriginal.contentDocument || this.bilingualOriginal.contentWindow.document;
            
            if (!translatedDoc || !originalDoc) {
                console.warn('无法访问 iframe 内容，可能是跨域限制');
                return;
            }

            let isTranslatedScrolling = false;
            let isOriginalScrolling = false;
            
            // 获取文档的滚动元素
            const getScrollElement = (doc) => {
                return doc.documentElement || doc.body;
            };
            
            const translatedScrollElement = getScrollElement(translatedDoc);
            const originalScrollElement = getScrollElement(originalDoc);

            // 计算滚动比例
            const getScrollRatio = (element) => {
                const scrollTop = element.scrollTop;
                const scrollHeight = element.scrollHeight - element.clientHeight;
                return scrollHeight > 0 ? scrollTop / scrollHeight : 0;
            };

            // 设置滚动位置
            const setScrollRatio = (element, ratio) => {
                const scrollHeight = element.scrollHeight - element.clientHeight;
                element.scrollTop = scrollHeight * ratio;
            };

            // 译文滚动同步到原文
            const syncTranslatedToOriginal = () => {
                if (isOriginalScrolling) return;
                
                isTranslatedScrolling = true;
                const ratio = getScrollRatio(translatedScrollElement);
                setScrollRatio(originalScrollElement, ratio);
                
                setTimeout(() => {
                    isTranslatedScrolling = false;
                }, 100);
            };

            // 原文滚动同步到译文
            const syncOriginalToTranslated = () => {
                if (isTranslatedScrolling) return;
                
                isOriginalScrolling = true;
                const ratio = getScrollRatio(originalScrollElement);
                setScrollRatio(translatedScrollElement, ratio);
                
                setTimeout(() => {
                    isOriginalScrolling = false;
                }, 100);
            };

            // 添加滚动事件监听器
            translatedScrollElement.addEventListener('scroll', syncTranslatedToOriginal, { passive: true });
            originalScrollElement.addEventListener('scroll', syncOriginalToTranslated, { passive: true });

            // 添加鼠标滚轮事件监听器（更精确的控制）
            const handleWheel = (targetElement, syncFunction) => {
                return (event) => {
                    event.preventDefault();
                    
                    // 计算滚动增量
                    const delta = event.deltaY;
                    const currentScrollTop = targetElement.scrollTop;
                    const newScrollTop = Math.max(0, Math.min(
                        targetElement.scrollHeight - targetElement.clientHeight,
                        currentScrollTop + delta
                    ));
                    
                    targetElement.scrollTop = newScrollTop;
                    syncFunction();
                };
            };

            this.bilingualTranslated.contentWindow.addEventListener('wheel', 
                handleWheel(translatedScrollElement, syncTranslatedToOriginal), 
                { passive: false }
            );
            
            this.bilingualOriginal.contentWindow.addEventListener('wheel', 
                handleWheel(originalScrollElement, syncOriginalToTranslated), 
                { passive: false }
            );

            console.log('双语同步滚动已启用');
            this.updateSyncStatus('同步滚动已启用', false);
            
        } catch (error) {
            console.warn('设置同步滚动失败，可能是跨域限制:', error);
            this.updateSyncStatus('同步滚动不可用', true);
        }
    }

    /**
     * 更新同步状态指示器
     * @param {string} message - 状态信息
     * @param {boolean} isError - 是否为错误状态
     */
    updateSyncStatus(message, isError) {
        this.syncStatus.textContent = message;
        this.syncIndicator.classList.toggle('error', isError);
    }

    /**
     * 设置iframe内的链接拦截
     */
    setupLinkInterception() {
        const frames = [
            this.translatedFrame,
            this.originalFrame,
            this.bilingualTranslated,
            this.bilingualOriginal
        ];

        frames.forEach(frame => {
            if (frame) {
                frame.addEventListener('load', () => {
                    this.injectLinkInterceptionScript(frame);
                });
            }
        });
    }

    /**
     * 向iframe注入链接拦截脚本
     * @param {HTMLIFrameElement} frame - 目标iframe
     */
    injectLinkInterceptionScript(frame) {
        try {
            const frameDoc = frame.contentDocument || frame.contentWindow.document;
            if (!frameDoc) {
                console.warn('无法访问iframe内容，可能是跨域限制');
                return;
            }

            // 创建链接拦截脚本
            const script = frameDoc.createElement('script');
            script.textContent = `
                (function() {
                    // 拦截所有链接点击
                    function interceptLinks() {
                        const links = document.querySelectorAll('a[href]');
                        links.forEach(link => {
                            link.addEventListener('click', function(event) {
                                event.preventDefault();
                                event.stopPropagation();
                                
                                const href = this.getAttribute('href');
                                if (!href || href.startsWith('#') || href.startsWith('javascript:') || href.startsWith('mailto:')) {
                                    return;
                                }
                                
                                // 处理相对URL
                                let fullUrl = href;
                                if (!href.startsWith('http://') && !href.startsWith('https://')) {
                                    const base = window.location.href;
                                    const baseUrl = new URL(base);
                                    fullUrl = new URL(href, baseUrl.origin).href;
                                }
                                
                                // 发送消息到父窗口
                                window.parent.postMessage({
                                    type: 'link-clicked',
                                    url: fullUrl
                                }, '*');
                            });
                        });
                    }
                    
                    // 立即执行和DOM变化时重新执行
                    interceptLinks();
                    
                    // 监听动态添加的链接
                    const observer = new MutationObserver(function(mutations) {
                        mutations.forEach(function(mutation) {
                            mutation.addedNodes.forEach(function(node) {
                                if (node.nodeType === 1) { // Element node
                                    if (node.tagName === 'A' && node.href) {
                                        interceptLink(node);
                                    } else {
                                        const links = node.querySelectorAll && node.querySelectorAll('a[href]');
                                        if (links) {
                                            links.forEach(interceptLink);
                                        }
                                    }
                                }
                            });
                        });
                    });
                    
                    function interceptLink(link) {
                        link.addEventListener('click', function(event) {
                            event.preventDefault();
                            event.stopPropagation();
                            
                            const href = this.getAttribute('href');
                            if (!href || href.startsWith('#') || href.startsWith('javascript:') || href.startsWith('mailto:')) {
                                return;
                            }
                            
                            let fullUrl = href;
                            if (!href.startsWith('http://') && !href.startsWith('https://')) {
                                const base = window.location.href;
                                const baseUrl = new URL(base);
                                fullUrl = new URL(href, baseUrl.origin).href;
                            }
                            
                            window.parent.postMessage({
                                type: 'link-clicked',
                                url: fullUrl
                            }, '*');
                        });
                    }
                    
                    observer.observe(document.body, {
                        childList: true,
                        subtree: true
                    });
                })();
            `;

            // 将脚本注入到iframe的文档头部
            const frameHead = frameDoc.head || frameDoc.getElementsByTagName('head')[0] || frameDoc.documentElement;
            frameHead.appendChild(script);
            
            console.log('链接拦截脚本已注入到iframe');
        } catch (error) {
            console.warn('注入链接拦截脚本失败，可能是跨域限制:', error);
        }
    }

    /**
     * 处理被拦截的链接点击
     * @param {string} url - 被点击的链接URL
     */
    handleLinkClick(url) {
        if (!url || !this.isValidUrl(url)) {
            this.showError('无效的链接地址');
            return;
        }

        // 显示链接拦截提示
        this.showLinkInterceptedNotification(url);
        
        // 自动填入URL并翻译
        this.loadUrlInInput(url);
    }

    /**
     * 显示链接拦截通知
     * @param {string} url - 被拦截的URL
     */
    showLinkInterceptedNotification(url) {
        // 创建通知元素
        const notification = document.createElement('div');
        notification.className = 'link-intercepted-notification';
        notification.innerHTML = `
            <div class="notification-content">
                <span class="notification-icon">🔗</span>
                <span class="notification-text">正在翻译链接: ${this.truncateUrl(url, 50)}</span>
            </div>
        `;
        
        // 添加样式
        notification.style.cssText = `
            position: fixed;
            top: 80px;
            right: 20px;
            background: #4CAF50;
            color: white;
            padding: 12px 20px;
            border-radius: 8px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.2);
            z-index: 10000;
            font-size: 14px;
            max-width: 400px;
            animation: slideInRight 0.3s ease-out;
        `;
        
        // 添加动画样式
        if (!document.querySelector('#link-notification-styles')) {
            const styles = document.createElement('style');
            styles.id = 'link-notification-styles';
            styles.textContent = `
                @keyframes slideInRight {
                    from { transform: translateX(100%); opacity: 0; }
                    to { transform: translateX(0); opacity: 1; }
                }
                .link-intercepted-notification .notification-content {
                    display: flex;
                    align-items: center;
                    gap: 8px;
                }
            `;
            document.head.appendChild(styles);
        }
        
        document.body.appendChild(notification);
        
        // 3秒后自动移除通知
        setTimeout(() => {
            if (notification.parentNode) {
                notification.style.animation = 'slideInRight 0.3s ease-out reverse';
                setTimeout(() => {
                    if (notification.parentNode) {
                        notification.parentNode.removeChild(notification);
                    }
                }, 300);
            }
        }, 3000);
    }

    /**
     * 将URL加载到输入框并触发翻译
     * @param {string} url - 要加载的URL
     */
    loadUrlInInput(url) {
        this.urlInput.value = url;
        // 延迟执行翻译，让用户看到URL已填入
        setTimeout(() => {
            this.handleTranslate();
        }, 500);
    }

    /**
     * 截断URL显示
     * @param {string} url - 要截断的URL
     * @param {number} maxLength - 最大长度
     * @returns {string} 截断后的URL
     */
    truncateUrl(url, maxLength) {
        if (url.length <= maxLength) return url;
        return url.substring(0, maxLength - 3) + '...';
    }

    /**
     * 验证 URL 格式
     * @param {string} string - 要验证的 URL 字符串
     * @returns {boolean} URL 是否有效
     */
    isValidUrl(string) {
        try {
            new URL(string);
            return true;
        } catch (_) {
            return false;
        }
    }
}

// 初始化应用
document.addEventListener('DOMContentLoaded', () => {
    new MonolithTranslator();
});