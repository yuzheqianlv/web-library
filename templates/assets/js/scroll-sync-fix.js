/**
 * 双语模式同步滚动修复和优化
 * 解决跨域、时序和性能问题
 */

class ImprovedScrollSync {
    constructor(translator) {
        this.translator = translator;
        this.isSetup = false;
        this.syncLock = false;
        this.lastSyncTime = 0;
        this.syncThrottle = 16; // 约60fps的节流
        this.retryCount = 0;
        this.maxRetries = 5;
        this.retryDelay = 1000;
    }

    /**
     * 设置双语模式同步滚动（改进版）
     */
    setupBilingualSync() {
        console.log('开始设置双语同步滚动...');
        
        // 重置状态
        this.isSetup = false;
        this.retryCount = 0;
        
        // 清理旧的事件监听器
        this.cleanup();
        
        // 等待iframe加载完成
        this.waitForFramesLoad().then(() => {
            this.initScrollSync();
        }).catch((error) => {
            console.error('同步滚动设置失败:', error);
            this.translator.updateSyncStatus('同步滚动设置失败', true);
        });
    }

    /**
     * 等待iframe加载完成
     */
    waitForFramesLoad() {
        return new Promise((resolve, reject) => {
            const frames = [
                this.translator.bilingualTranslated,
                this.translator.bilingualOriginal
            ];
            
            let loadedCount = 0;
            const totalFrames = frames.length;
            let timeout;

            const checkLoad = () => {
                loadedCount++;
                console.log(`iframe ${loadedCount}/${totalFrames} 加载完成`);
                
                if (loadedCount === totalFrames) {
                    clearTimeout(timeout);
                    // 额外等待以确保内容完全渲染
                    setTimeout(resolve, 500);
                }
            };

            // 设置超时
            timeout = setTimeout(() => {
                reject(new Error('iframe加载超时'));
            }, 10000);

            // 监听load事件
            frames.forEach((frame, index) => {
                if (frame.complete || frame.readyState === 'complete') {
                    // 已经加载完成
                    checkLoad();
                } else {
                    // 监听load事件
                    const onLoad = () => {
                        frame.removeEventListener('load', onLoad);
                        checkLoad();
                    };
                    frame.addEventListener('load', onLoad);
                }
            });
        });
    }

    /**
     * 初始化滚动同步功能（改进版）
     */
    initScrollSync() {
        try {
            console.log('正在初始化滚动同步...');
            
            // 获取iframe文档
            const translatedDoc = this.getFrameDocument(this.translator.bilingualTranslated);
            const originalDoc = this.getFrameDocument(this.translator.bilingualOriginal);
            
            if (!translatedDoc || !originalDoc) {
                this.handleCORSFallback();
                return;
            }

            // 获取滚动容器
            const translatedContainer = this.getScrollContainer(translatedDoc);
            const originalContainer = this.getScrollContainer(originalDoc);

            if (!translatedContainer || !originalContainer) {
                console.error('无法找到滚动容器');
                this.translator.updateSyncStatus('滚动容器不可用', true);
                return;
            }

            // 设置同步监听器
            this.setupSyncListeners(translatedContainer, originalContainer);
            
            // 设置成功
            this.isSetup = true;
            console.log('双语同步滚动设置成功');
            this.translator.updateSyncStatus('同步滚动已启用', false);
            
        } catch (error) {
            console.error('初始化滚动同步失败:', error);
            this.handleCORSFallback();
        }
    }

    /**
     * 获取iframe文档对象
     */
    getFrameDocument(frame) {
        try {
            // 尝试多种方式获取文档
            return frame.contentDocument || 
                   frame.contentWindow?.document || 
                   frame.document;
        } catch (error) {
            console.warn('无法访问iframe文档，可能是跨域限制:', error);
            return null;
        }
    }

    /**
     * 获取滚动容器
     */
    getScrollContainer(doc) {
        try {
            // 尝试多种滚动容器
            const candidates = [
                doc.documentElement,
                doc.body,
                doc.querySelector('.main-content'),
                doc.querySelector('main'),
                doc.querySelector('[data-scroll]')
            ];

            for (const candidate of candidates) {
                if (candidate && this.isScrollable(candidate)) {
                    return candidate;
                }
            }

            // 如果都不可滚动，返回documentElement作为fallback
            return doc.documentElement || doc.body;
        } catch (error) {
            console.error('获取滚动容器失败:', error);
            return null;
        }
    }

    /**
     * 检查元素是否可滚动
     */
    isScrollable(element) {
        try {
            if (!element) return false;
            
            const style = getComputedStyle(element);
            const hasScroll = element.scrollHeight > element.clientHeight;
            const canScroll = ['auto', 'scroll'].includes(style.overflowY) || 
                             ['auto', 'scroll'].includes(style.overflow);
            
            return hasScroll || canScroll;
        } catch (error) {
            return false;
        }
    }

    /**
     * 设置同步监听器
     */
    setupSyncListeners(translatedContainer, originalContainer) {
        // 节流函数
        const throttle = (func, delay) => {
            let timeoutId;
            let lastCallTime = 0;
            
            return (...args) => {
                const now = Date.now();
                const timeSinceLastCall = now - lastCallTime;
                
                const callFunction = () => {
                    lastCallTime = now;
                    func.apply(this, args);
                };
                
                if (timeSinceLastCall >= delay) {
                    callFunction();
                } else {
                    clearTimeout(timeoutId);
                    timeoutId = setTimeout(callFunction, delay - timeSinceLastCall);
                }
            };
        };

        // 同步函数
        const syncScroll = throttle((sourceContainer, targetContainer, direction) => {
            if (this.syncLock) return;
            
            this.syncLock = true;
            
            try {
                const sourceRatio = this.getScrollRatio(sourceContainer);
                this.setScrollRatio(targetContainer, sourceRatio);
                
                // 更新同步状态指示器
                const indicator = this.translator.syncIndicator;
                if (indicator) {
                    indicator.classList.add('show');
                    indicator.classList.remove('error');
                }
                
                console.debug(`同步滚动: ${direction}, 比例: ${sourceRatio.toFixed(3)}`);
            } catch (error) {
                console.error('同步滚动执行失败:', error);
            } finally {
                // 延迟解锁以避免循环触发
                setTimeout(() => {
                    this.syncLock = false;
                }, 50);
            }
        }, this.syncThrottle);

        // 添加滚动事件监听器
        const translatedScrollHandler = () => syncScroll(translatedContainer, originalContainer, '译文→原文');
        const originalScrollHandler = () => syncScroll(originalContainer, translatedContainer, '原文→译文');

        translatedContainer.addEventListener('scroll', translatedScrollHandler, { passive: true });
        originalContainer.addEventListener('scroll', originalScrollHandler, { passive: true });

        // 保存清理函数
        this.cleanupFunctions = [
            () => translatedContainer.removeEventListener('scroll', translatedScrollHandler),
            () => originalContainer.removeEventListener('scroll', originalScrollHandler)
        ];

        // 添加鼠标滚轮监听（可选）
        this.setupWheelListeners(translatedContainer, originalContainer, syncScroll);
    }

    /**
     * 设置鼠标滚轮监听器
     */
    setupWheelListeners(translatedContainer, originalContainer, syncScroll) {
        try {
            // 获取iframe窗口
            const translatedWindow = this.translator.bilingualTranslated.contentWindow;
            const originalWindow = this.translator.bilingualOriginal.contentWindow;

            if (!translatedWindow || !originalWindow) return;

            // 滚轮处理函数
            const handleWheel = (sourceContainer, targetContainer, direction) => {
                return throttle((event) => {
                    // 不阻止默认行为，让浏览器处理滚动
                    // event.preventDefault();
                    
                    // 延迟同步以确保滚动位置已更新
                    setTimeout(() => {
                        syncScroll(sourceContainer, targetContainer, direction);
                    }, 0);
                }, this.syncThrottle);
            };

            const translatedWheelHandler = handleWheel(translatedContainer, originalContainer, '译文→原文');
            const originalWheelHandler = handleWheel(originalContainer, translatedContainer, '原文→译文');

            translatedWindow.addEventListener('wheel', translatedWheelHandler, { passive: true });
            originalWindow.addEventListener('wheel', originalWheelHandler, { passive: true });

            // 添加到清理函数
            this.cleanupFunctions.push(
                () => translatedWindow.removeEventListener('wheel', translatedWheelHandler),
                () => originalWindow.removeEventListener('wheel', originalWheelHandler)
            );

        } catch (error) {
            console.warn('设置滚轮监听器失败:', error);
        }
    }

    /**
     * 计算滚动比例
     */
    getScrollRatio(element) {
        try {
            const scrollTop = element.scrollTop;
            const scrollHeight = element.scrollHeight - element.clientHeight;
            
            if (scrollHeight <= 0) return 0;
            return Math.max(0, Math.min(1, scrollTop / scrollHeight));
        } catch (error) {
            console.error('计算滚动比例失败:', error);
            return 0;
        }
    }

    /**
     * 设置滚动比例
     */
    setScrollRatio(element, ratio) {
        try {
            const scrollHeight = element.scrollHeight - element.clientHeight;
            const newScrollTop = Math.max(0, Math.min(scrollHeight, scrollHeight * ratio));
            
            // 使用平滑滚动或直接设置
            if (element.scrollTo && typeof element.scrollTo === 'function') {
                element.scrollTo({
                    top: newScrollTop,
                    behavior: 'auto' // 使用auto避免动画干扰
                });
            } else {
                element.scrollTop = newScrollTop;
            }
        } catch (error) {
            console.error('设置滚动位置失败:', error);
        }
    }

    /**
     * 处理跨域情况的fallback
     */
    handleCORSFallback() {
        console.warn('检测到跨域限制，使用fallback同步方案');
        
        // 方案1: 使用iframe容器的滚动事件
        this.setupContainerSync();
        
        // 方案2: 使用PostMessage通信（如果页面支持）
        this.setupPostMessageSync();
        
        this.translator.updateSyncStatus('同步滚动(受限模式)', false);
    }

    /**
     * 容器同步方案
     */
    setupContainerSync() {
        try {
            // 监听iframe容器的父元素滚动
            const containers = [
                this.translator.bilingualTranslated.parentElement,
                this.translator.bilingualOriginal.parentElement
            ];

            if (containers[0] && containers[1]) {
                let isSync = false;
                
                const syncContainers = throttle((sourceIndex) => {
                    if (isSync) return;
                    isSync = true;
                    
                    const source = containers[sourceIndex];
                    const target = containers[1 - sourceIndex];
                    
                    if (source && target) {
                        const ratio = this.getScrollRatio(source);
                        this.setScrollRatio(target, ratio);
                    }
                    
                    setTimeout(() => { isSync = false; }, 50);
                }, this.syncThrottle);

                containers[0].addEventListener('scroll', () => syncContainers(0), { passive: true });
                containers[1].addEventListener('scroll', () => syncContainers(1), { passive: true });
            }
        } catch (error) {
            console.error('容器同步设置失败:', error);
        }
    }

    /**
     * PostMessage同步方案
     */
    setupPostMessageSync() {
        try {
            // 向iframe发送同步脚本
            const syncScript = `
                (function() {
                    let lastScrollTop = 0;
                    const checkScroll = () => {
                        const currentScrollTop = document.documentElement.scrollTop || document.body.scrollTop;
                        if (currentScrollTop !== lastScrollTop) {
                            lastScrollTop = currentScrollTop;
                            const scrollHeight = document.documentElement.scrollHeight - window.innerHeight;
                            const ratio = scrollHeight > 0 ? currentScrollTop / scrollHeight : 0;
                            parent.postMessage({
                                type: 'scroll-sync',
                                source: '${Date.now()}',
                                ratio: ratio
                            }, '*');
                        }
                        requestAnimationFrame(checkScroll);
                    };
                    checkScroll();
                })();
            `;

            // 尝试注入脚本到iframe
            [this.translator.bilingualTranslated, this.translator.bilingualOriginal].forEach((frame, index) => {
                try {
                    const doc = frame.contentDocument || frame.contentWindow.document;
                    if (doc) {
                        const script = doc.createElement('script');
                        script.textContent = syncScript.replace('${Date.now()}', `frame-${index}`);
                        doc.head.appendChild(script);
                    }
                } catch (e) {
                    // 静默处理跨域错误
                }
            });

            // 监听PostMessage
            window.addEventListener('message', (event) => {
                if (event.data.type === 'scroll-sync') {
                    // 处理同步消息
                    console.debug('收到滚动同步消息:', event.data);
                }
            });

        } catch (error) {
            console.error('PostMessage同步设置失败:', error);
        }
    }

    /**
     * 清理所有事件监听器
     */
    cleanup() {
        if (this.cleanupFunctions) {
            this.cleanupFunctions.forEach(cleanup => {
                try {
                    cleanup();
                } catch (error) {
                    console.warn('清理监听器失败:', error);
                }
            });
            this.cleanupFunctions = [];
        }
        this.isSetup = false;
    }

    /**
     * 手动触发同步
     */
    manualSync() {
        if (!this.isSetup) {
            console.warn('同步滚动未设置，无法手动同步');
            return;
        }

        try {
            const translatedDoc = this.getFrameDocument(this.translator.bilingualTranslated);
            const originalDoc = this.getFrameDocument(this.translator.bilingualOriginal);
            
            if (translatedDoc && originalDoc) {
                const translatedContainer = this.getScrollContainer(translatedDoc);
                const originalContainer = this.getScrollContainer(originalDoc);
                
                if (translatedContainer && originalContainer) {
                    // 以译文为准同步到原文
                    const ratio = this.getScrollRatio(translatedContainer);
                    this.setScrollRatio(originalContainer, ratio);
                    console.log('手动同步完成，比例:', ratio);
                }
            }
        } catch (error) {
            console.error('手动同步失败:', error);
        }
    }

    /**
     * 检查同步状态
     */
    checkSyncStatus() {
        return {
            isSetup: this.isSetup,
            syncLock: this.syncLock,
            retryCount: this.retryCount,
            lastSyncTime: this.lastSyncTime
        };
    }
}

// 节流函数工具
function throttle(func, delay) {
    let timeoutId;
    let lastCallTime = 0;
    
    return function(...args) {
        const now = Date.now();
        const timeSinceLastCall = now - lastCallTime;
        
        const callFunction = () => {
            lastCallTime = now;
            func.apply(this, args);
        };
        
        if (timeSinceLastCall >= delay) {
            callFunction();
        } else {
            clearTimeout(timeoutId);
            timeoutId = setTimeout(callFunction, delay - timeSinceLastCall);
        }
    };
}

// 导出供主程序使用
window.ImprovedScrollSync = ImprovedScrollSync;