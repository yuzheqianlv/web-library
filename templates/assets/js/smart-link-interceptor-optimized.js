/**
 * Monolith智能链接拦截器 - 性能优化版本
 * 
 * 优化特性：
 * - 虚拟化处理大量链接
 * - 延迟加载和懒执行
 * - 内存使用优化
 * - 批量处理优化
 * - 智能节流和防抖
 * 
 * @version 1.0.0-optimized
 * @author Monolith Team
 */

(function() {
    'use strict';

    // 优化配置
    const OPTIMIZED_CONFIG = {
        // 性能限制
        MAX_LINKS_IMMEDIATE: 50,    // 立即处理的链接数量
        MAX_LINKS_TOTAL: 500,       // 总处理链接数量
        BATCH_SIZE: 10,             // 批量处理大小
        THROTTLE_DELAY: 16,         // 节流延迟 (60fps)
        DEBOUNCE_DELAY: 300,        // 防抖延迟
        
        // 虚拟化配置
        VIEWPORT_BUFFER: 100,       // 视窗缓冲区
        INTERSECTION_THRESHOLD: 0.1, // 交叉观察阈值
        
        // 缓存优化
        MEMORY_CACHE_SIZE: 200,     // 内存缓存大小
        STORAGE_CACHE_SIZE: 1000,   // 存储缓存大小
        
        // 网络优化
        REQUEST_CONCURRENCY: 3,     // 并发请求数
        REQUEST_TIMEOUT: 5000,      // 请求超时
        RETRY_COUNT: 2,             // 重试次数
        
        // 调试
        DEBUG: false,
        PERFORMANCE_MONITORING: true
    };

    /**
     * 性能监控器
     */
    class PerformanceMonitor {
        constructor() {
            this.metrics = {
                initialization: 0,
                linkProcessing: 0,
                batchRequests: 0,
                memoryUsage: [],
                errors: []
            };
            this.startTime = performance.now();
        }

        mark(name) {
            const time = performance.now() - this.startTime;
            this.metrics[name] = time;
            
            if (OPTIMIZED_CONFIG.DEBUG) {
                console.log(`[PerfMonitor] ${name}: ${time.toFixed(2)}ms`);
            }
        }

        recordMemory() {
            if (performance.memory && OPTIMIZED_CONFIG.PERFORMANCE_MONITORING) {
                this.metrics.memoryUsage.push({
                    timestamp: Date.now(),
                    used: performance.memory.usedJSHeapSize,
                    total: performance.memory.totalJSHeapSize
                });
                
                // 保持最近100条记录
                if (this.metrics.memoryUsage.length > 100) {
                    this.metrics.memoryUsage.shift();
                }
            }
        }

        getReport() {
            return {
                ...this.metrics,
                totalTime: performance.now() - this.startTime
            };
        }
    }

    /**
     * 优化的链接管理器
     */
    class OptimizedLinkManager {
        constructor() {
            this.links = new Map();
            this.visibleLinks = new Set();
            this.processingQueue = [];
            this.observer = null;
            this.perfMonitor = new PerformanceMonitor();
            
            this.setupIntersectionObserver();
        }

        /**
         * 设置交叉观察器（用于虚拟化）
         */
        setupIntersectionObserver() {
            if (typeof IntersectionObserver === 'undefined') {
                this.log('IntersectionObserver不可用，使用降级方案');
                return;
            }

            this.observer = new IntersectionObserver((entries) => {
                entries.forEach(entry => {
                    const link = entry.target;
                    const linkId = this.getLinkId(link);
                    
                    if (entry.isIntersecting) {
                        this.visibleLinks.add(linkId);
                        this.processLinkLazy(link);
                    } else {
                        this.visibleLinks.delete(linkId);
                    }
                });
            }, {
                threshold: OPTIMIZED_CONFIG.INTERSECTION_THRESHOLD,
                rootMargin: `${OPTIMIZED_CONFIG.VIEWPORT_BUFFER}px`
            });
        }

        /**
         * 发现并注册链接
         */
        discoverLinks() {
            this.perfMonitor.mark('linkDiscovery_start');
            
            const allLinks = document.querySelectorAll('a[href^="http"]:not([data-monolith-ignore])');
            let processedCount = 0;
            
            // 限制处理的链接数量
            const linksToProcess = Array.from(allLinks).slice(0, OPTIMIZED_CONFIG.MAX_LINKS_TOTAL);
            
            linksToProcess.forEach((link, index) => {
                const linkId = this.getLinkId(link);
                
                if (!this.links.has(linkId)) {
                    this.links.set(linkId, {
                        element: link,
                        status: 'unknown',
                        lastCheck: 0,
                        visible: false,
                        processed: false
                    });

                    // 立即处理前N个链接
                    if (index < OPTIMIZED_CONFIG.MAX_LINKS_IMMEDIATE) {
                        this.processLinkImmediate(link);
                        processedCount++;
                    } else {
                        // 其他链接使用虚拟化
                        this.observer?.observe(link);
                    }
                }
            });
            
            this.perfMonitor.mark('linkDiscovery_end');
            this.log(`发现 ${linksToProcess.length} 个链接，立即处理 ${processedCount} 个`);
            
            return processedCount;
        }

        /**
         * 立即处理链接
         */
        processLinkImmediate(link) {
            this.setupLinkEvents(link);
            this.addStatusIndicator(link);
            this.queueStatusCheck(link);
        }

        /**
         * 延迟处理链接
         */
        processLinkLazy(link) {
            const linkId = this.getLinkId(link);
            const linkData = this.links.get(linkId);
            
            if (linkData && !linkData.processed) {
                this.processLinkImmediate(link);
                linkData.processed = true;
            }
        }

        /**
         * 设置链接事件
         */
        setupLinkEvents(link) {
            // 优化的事件处理 - 使用事件委托的概念
            link.addEventListener('click', this.createClickHandler(link), { passive: false });
            
            // 延迟绑定鼠标悬停事件
            link.addEventListener('mouseenter', this.createHoverHandler(link), { 
                once: true,
                passive: true 
            });
        }

        /**
         * 创建点击处理器
         */
        createClickHandler(link) {
            return (e) => {
                e.preventDefault();
                
                const linkData = this.links.get(this.getLinkId(link));
                const status = linkData?.status || 'unknown';
                
                this.handleLinkNavigation(link.href, status, linkData);
            };
        }

        /**
         * 创建悬停处理器
         */
        createHoverHandler(link) {
            return this.throttle(() => {
                this.prioritizeStatusCheck(link);
            }, OPTIMIZED_CONFIG.THROTTLE_DELAY);
        }

        /**
         * 队列状态检查
         */
        queueStatusCheck(link) {
            this.processingQueue.push(link);
            this.processQueueDebounced();
        }

        /**
         * 优先状态检查
         */
        prioritizeStatusCheck(link) {
            // 将链接移到队列前面
            this.processingQueue = this.processingQueue.filter(l => l !== link);
            this.processingQueue.unshift(link);
            this.processQueueImmediate();
        }

        /**
         * 处理队列（防抖版本）
         */
        processQueueDebounced = this.debounce(() => {
            this.processQueue();
        }, OPTIMIZED_CONFIG.DEBOUNCE_DELAY);

        /**
         * 立即处理队列
         */
        processQueueImmediate = this.throttle(() => {
            this.processQueue();
        }, OPTIMIZED_CONFIG.THROTTLE_DELAY);

        /**
         * 处理状态检查队列
         */
        async processQueue() {
            if (this.processingQueue.length === 0) return;
            
            const batch = this.processingQueue.splice(0, OPTIMIZED_CONFIG.BATCH_SIZE);
            const urls = batch.map(link => link.href);
            
            try {
                this.perfMonitor.mark('batchRequest_start');
                
                const results = await this.batchCheckStatus(urls);
                
                // 更新链接状态
                batch.forEach((link, index) => {
                    const result = results[index];
                    if (result) {
                        this.updateLinkStatus(link, result);
                    }
                });
                
                this.perfMonitor.mark('batchRequest_end');
                this.perfMonitor.recordMemory();
                
            } catch (error) {
                this.log(`批量状态检查失败: ${error.message}`, 'error');
                this.perfMonitor.metrics.errors.push(error.message);
                
                // 降级到单个请求
                batch.forEach(link => {
                    this.checkSingleLinkStatus(link);
                });
            }
        }

        /**
         * 批量检查状态
         */
        async batchCheckStatus(urls) {
            const controller = new AbortController();
            const timeoutId = setTimeout(() => controller.abort(), OPTIMIZED_CONFIG.REQUEST_TIMEOUT);
            
            try {
                const response = await fetch('/api/v2/link-status/batch', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        urls: urls,
                        max_concurrent: OPTIMIZED_CONFIG.REQUEST_CONCURRENCY
                    }),
                    signal: controller.signal
                });
                
                clearTimeout(timeoutId);
                
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}`);
                }
                
                const data = await response.json();
                return data.results || [];
                
            } catch (error) {
                clearTimeout(timeoutId);
                throw error;
            }
        }

        /**
         * 单个链接状态检查
         */
        async checkSingleLinkStatus(link) {
            const url = link.href;
            let retries = OPTIMIZED_CONFIG.RETRY_COUNT;
            
            while (retries > 0) {
                try {
                    const response = await fetch(`/api/v2/link-status?url=${encodeURIComponent(url)}`);
                    
                    if (response.ok) {
                        const result = await response.json();
                        this.updateLinkStatus(link, result);
                        return;
                    }
                    
                } catch (error) {
                    this.log(`单个状态检查失败: ${error.message}`, 'warn');
                }
                
                retries--;
                if (retries > 0) {
                    await this.delay(1000); // 重试延迟
                }
            }
            
            // 所有重试都失败
            this.updateLinkStatus(link, { status: 'error' });
        }

        /**
         * 更新链接状态
         */
        updateLinkStatus(link, result) {
            const linkId = this.getLinkId(link);
            const linkData = this.links.get(linkId);
            
            if (linkData) {
                linkData.status = result.status;
                linkData.lastCheck = Date.now();
                
                // 更新UI（仅对可见链接）
                if (this.visibleLinks.has(linkId) || linkData.processed) {
                    this.updateLinkUI(link, result);
                }
                
                // 缓存结果
                this.cacheResult(link.href, result);
            }
        }

        /**
         * 更新链接UI
         */
        updateLinkUI(link, result) {
            // 使用requestAnimationFrame优化DOM操作
            requestAnimationFrame(() => {
                this.updateStatusIndicator(link, result.status);
                this.updateLinkStyle(link, result.status);
                link.dataset.monolithStatus = result.status;
            });
        }

        /**
         * 处理链接导航
         */
        handleLinkNavigation(url, status, linkData) {
            switch (status) {
                case 'cached':
                    if (linkData?.recordId) {
                        window.location.href = `/library/${linkData.recordId}`;
                    } else {
                        this.navigateToSmart(url);
                    }
                    break;
                    
                case 'processing':
                    this.navigateToSmart(url);
                    break;
                    
                default:
                    this.navigateToSmart(url);
                    break;
            }
        }

        /**
         * 工具方法
         */
        getLinkId(link) {
            return link.href;
        }

        navigateToSmart(url) {
            window.location.href = `/smart-website/${encodeURIComponent(url)}`;
        }

        // 节流函数
        throttle(func, delay) {
            let timeoutId;
            let lastExecTime = 0;
            
            return function(...args) {
                const currentTime = Date.now();
                
                if (currentTime - lastExecTime > delay) {
                    func.apply(this, args);
                    lastExecTime = currentTime;
                } else {
                    clearTimeout(timeoutId);
                    timeoutId = setTimeout(() => {
                        func.apply(this, args);
                        lastExecTime = Date.now();
                    }, delay - (currentTime - lastExecTime));
                }
            };
        }

        // 防抖函数
        debounce(func, delay) {
            let timeoutId;
            
            return function(...args) {
                clearTimeout(timeoutId);
                timeoutId = setTimeout(() => func.apply(this, args), delay);
            };
        }

        delay(ms) {
            return new Promise(resolve => setTimeout(resolve, ms));
        }

        log(message, level = 'log') {
            if (OPTIMIZED_CONFIG.DEBUG) {
                console[level](`[OptimizedInterceptor] ${message}`);
            }
        }

        // 缓存相关方法（简化版本）
        cacheResult(url, result) {
            // 实现简化的缓存逻辑
        }

        addStatusIndicator(link) {
            // 实现状态指示器
        }

        updateStatusIndicator(link, status) {
            // 更新状态指示器
        }

        updateLinkStyle(link, status) {
            // 更新链接样式
        }

        /**
         * 获取性能报告
         */
        getPerformanceReport() {
            return this.perfMonitor.getReport();
        }

        /**
         * 清理资源
         */
        destroy() {
            this.observer?.disconnect();
            this.links.clear();
            this.visibleLinks.clear();
            this.processingQueue = [];
        }
    }

    /**
     * 优化的拦截器主类
     */
    class OptimizedSmartLinkInterceptor {
        constructor() {
            this.linkManager = new OptimizedLinkManager();
            this.initialized = false;
            
            this.initialize();
        }

        async initialize() {
            if (this.initialized) return;

            this.linkManager.perfMonitor.mark('initialization_start');
            
            // 发现和处理链接
            const processedCount = this.linkManager.discoverLinks();
            
            // 创建状态面板
            this.createOptimizedStatusPanel(processedCount);
            
            this.linkManager.perfMonitor.mark('initialization_end');
            this.initialized = true;
            
            this.linkManager.log(`优化拦截器初始化完成，处理了 ${processedCount} 个链接`);
        }

        createOptimizedStatusPanel(processedCount) {
            const panel = document.createElement('div');
            panel.id = 'monolith-optimized-status';
            panel.style.cssText = `
                position: fixed;
                bottom: 20px;
                right: 20px;
                background: rgba(0, 0, 0, 0.8);
                color: white;
                padding: 8px 12px;
                border-radius: 4px;
                font-size: 11px;
                z-index: 9999;
                font-family: monospace;
            `;
            
            panel.innerHTML = `⚡ 优化拦截器: ${processedCount} 链接`;
            document.body.appendChild(panel);
            
            // 3秒后自动隐藏
            setTimeout(() => {
                panel.style.opacity = '0.5';
            }, 3000);
        }

        getStats() {
            return {
                links: this.linkManager.links.size,
                visible: this.linkManager.visibleLinks.size,
                performance: this.linkManager.getPerformanceReport()
            };
        }

        destroy() {
            this.linkManager.destroy();
            this.initialized = false;
        }
    }

    // 自动检测和初始化
    function autoInit() {
        // 检查是否启用优化版本
        const urlParams = new URLSearchParams(window.location.search);
        const shouldUseOptimized = urlParams.get('optimized') === '1' ||
                                  document.querySelector('meta[name="monolith-optimized"]') ||
                                  document.querySelectorAll('a[href^="http"]').length > 100;

        if (shouldUseOptimized && !window.monolithOptimizedInterceptor) {
            window.monolithOptimizedInterceptor = new OptimizedSmartLinkInterceptor();
        }
    }

    // 导出
    window.OptimizedSmartLinkInterceptor = OptimizedSmartLinkInterceptor;
    window.OPTIMIZED_CONFIG = OPTIMIZED_CONFIG;

    // 自动初始化
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', autoInit);
    } else {
        autoInit();
    }

})();