/**
 * Monolith智能链接拦截器
 * 
 * 功能特性：
 * - 自动检测和拦截页面链接
 * - 实时查询链接状态
 * - 智能路由决策
 * - 状态可视化指示器
 * - 批量处理优化
 * - 跨浏览器兼容性
 * 
 * @version 1.0.0
 * @author Monolith Team
 */

(function() {
    'use strict';

    // 配置常量
    const CONFIG = {
        // API端点
        API_BASE: '/api/v2',
        LINK_STATUS_API: '/api/v2/link-status',
        BATCH_STATUS_API: '/api/v2/link-status/batch',
        SMART_ROUTE_BASE: '/smart-website',
        
        // 批量处理配置
        BATCH_SIZE: 20,
        BATCH_DELAY: 100, // ms
        MAX_CONCURRENT: 5,
        
        // 缓存配置
        CACHE_DURATION: 300000, // 5分钟
        STORAGE_KEY: 'monolith_link_cache',
        
        // UI配置
        STATUS_UPDATE_INTERVAL: 3000, // 3秒
        ANIMATION_DURATION: 300,
        
        // 选择器配置
        LINK_SELECTOR: 'a[href^="http"]:not([data-monolith-ignore])',
        EXCLUDE_DOMAINS: ['localhost', '127.0.0.1'],
        
        // 调试模式
        DEBUG: false,
    };

    // 状态常量
    const LINK_STATUS = {
        UNKNOWN: 'unknown',
        CHECKING: 'checking',
        CACHED: 'cached',
        PROCESSING: 'processing',
        EXPIRED: 'expired',
        FAILED: 'failed',
        NOT_FOUND: 'not_found',
        ERROR: 'error'
    };

    // 全局状态
    let interceptorState = {
        initialized: false,
        cache: new Map(),
        pendingRequests: new Map(),
        batchQueue: [],
        batchTimer: null,
        statsInterval: null,
        totalLinks: 0,
        processedLinks: 0
    };

    /**
     * 智能链接拦截器主类
     */
    class SmartLinkInterceptor {
        constructor() {
            this.log('初始化智能链接拦截器');
            this.bindEvents();
            this.loadCache();
            this.initializeUI();
        }

        /**
         * 绑定事件监听器
         */
        bindEvents() {
            // DOM ready事件
            if (document.readyState === 'loading') {
                document.addEventListener('DOMContentLoaded', () => this.initialize());
            } else {
                this.initialize();
            }

            // 页面可见性变化
            document.addEventListener('visibilitychange', () => {
                if (!document.hidden) {
                    this.refreshLinkStatus();
                }
            });

            // 窗口焦点事件
            window.addEventListener('focus', () => {
                this.refreshLinkStatus();
            });

            // 页面卸载时保存缓存
            window.addEventListener('beforeunload', () => {
                this.saveCache();
            });
        }

        /**
         * 初始化拦截器
         */
        initialize() {
            if (interceptorState.initialized) return;

            this.log('开始初始化链接拦截');
            
            // 查找所有链接
            const links = this.findLinks();
            interceptorState.totalLinks = links.length;
            
            if (links.length === 0) {
                this.log('未找到可拦截的链接');
                return;
            }

            this.log(`找到 ${links.length} 个链接，开始处理`);

            // 为链接添加拦截器
            this.interceptLinks(links);
            
            // 批量查询链接状态
            this.batchCheckLinkStatus(links);
            
            // 启动状态更新定时器
            this.startStatusUpdater();
            
            interceptorState.initialized = true;
            this.updateStats();
            
            this.log('链接拦截器初始化完成');
        }

        /**
         * 查找页面中的链接
         */
        findLinks() {
            const links = Array.from(document.querySelectorAll(CONFIG.LINK_SELECTOR));
            
            return links.filter(link => {
                const url = link.href;
                
                // 跳过空链接
                if (!url || url === '#') return false;
                
                // 跳过排除的域名
                try {
                    const urlObj = new URL(url);
                    if (CONFIG.EXCLUDE_DOMAINS.includes(urlObj.hostname)) {
                        return false;
                    }
                } catch (e) {
                    return false;
                }
                
                // 跳过已处理的链接
                if (link.dataset.monolithProcessed) return false;
                
                return true;
            });
        }

        /**
         * 为链接添加拦截器
         */
        interceptLinks(links) {
            links.forEach(link => {
                // 标记为已处理
                link.dataset.monolithProcessed = 'true';
                link.dataset.monolithStatus = LINK_STATUS.UNKNOWN;
                
                // 添加点击拦截器
                link.addEventListener('click', (e) => this.handleLinkClick(e, link));
                
                // 添加悬停效果
                link.addEventListener('mouseenter', () => this.handleLinkHover(link));
                
                // 添加状态指示器
                this.addStatusIndicator(link);
            });
        }

        /**
         * 处理链接点击事件
         */
        handleLinkClick(event, link) {
            event.preventDefault();
            event.stopPropagation();
            
            const url = link.href;
            const status = link.dataset.monolithStatus;
            
            this.log(`链接点击: ${url}, 状态: ${status}`);
            
            // 根据状态决定跳转策略
            switch (status) {
                case LINK_STATUS.CACHED:
                    // 直接跳转到缓存版本
                    const recordId = link.dataset.monolithRecordId;
                    if (recordId) {
                        window.location.href = `/library/${recordId}`;
                    } else {
                        this.navigateToSmartRoute(url);
                    }
                    break;
                    
                case LINK_STATUS.PROCESSING:
                    // 跳转到等待页面
                    this.navigateToSmartRoute(url);
                    break;
                    
                case LINK_STATUS.EXPIRED:
                case LINK_STATUS.FAILED:
                    // 显示选择对话框
                    this.showChoiceDialog(url, link);
                    break;
                    
                case LINK_STATUS.NOT_FOUND:
                case LINK_STATUS.UNKNOWN:
                default:
                    // 跳转到智能路由进行处理
                    this.navigateToSmartRoute(url);
                    break;
            }
        }

        /**
         * 处理链接悬停事件
         */
        handleLinkHover(link) {
            const url = link.href;
            
            // 如果还没有查询过状态，立即查询
            if (link.dataset.monolithStatus === LINK_STATUS.UNKNOWN) {
                this.checkSingleLinkStatus(url, link);
            }
            
            // 显示状态提示
            this.showStatusTooltip(link);
        }

        /**
         * 批量查询链接状态
         */
        async batchCheckLinkStatus(links) {
            if (links.length === 0) return;
            
            // 显示处理通知
            if (links.length > 10) {
                this.createProgressNotification(
                    `正在检查 ${links.length} 个链接的状态...`,
                    'info'
                );
            }
            
            // 将链接分批处理
            const batches = this.chunkArray(links, CONFIG.BATCH_SIZE);
            
            for (let i = 0; i < batches.length; i++) {
                const batch = batches[i];
                
                // 延迟处理，避免过载
                if (i > 0) {
                    await this.delay(CONFIG.BATCH_DELAY);
                }
                
                this.processBatch(batch);
            }
        }

        /**
         * 处理单个批次
         */
        async processBatch(links) {
            const urls = links.map(link => link.href);
            
            try {
                this.log(`批量查询 ${urls.length} 个链接状态`);
                
                const response = await fetch(CONFIG.BATCH_STATUS_API, {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({
                        urls: urls,
                        max_concurrent: CONFIG.MAX_CONCURRENT
                    })
                });
                
                if (!response.ok) {
                    throw new Error(`批量查询失败: ${response.status}`);
                }
                
                const data = await response.json();
                this.processBatchResponse(data, links);
                
            } catch (error) {
                this.log(`批量查询错误: ${error.message}`, 'error');
                
                // 降级到单个查询
                links.forEach(link => {
                    this.checkSingleLinkStatus(link.href, link);
                });
            }
        }

        /**
         * 处理批量查询响应
         */
        processBatchResponse(data, links) {
            const { results, summary } = data;
            
            this.log(`批量查询完成: 总数=${summary.total_urls}, 成功=${summary.successful_queries}, 缓存=${summary.cached_urls}`);
            
            // 显示结果通知
            if (summary.cached_urls > 0) {
                this.createProgressNotification(
                    `发现 ${summary.cached_urls} 个可用缓存链接`,
                    'success'
                );
            }
            
            // 创建URL到结果的映射
            const resultMap = new Map();
            results.forEach(result => {
                resultMap.set(result.url, result);
            });
            
            // 更新链接状态
            links.forEach(link => {
                const result = resultMap.get(link.href);
                if (result) {
                    this.updateLinkStatus(link, result);
                    this.cacheResult(link.href, result);
                } else {
                    this.updateLinkStatus(link, { status: LINK_STATUS.ERROR });
                }
            });
            
            this.updateStats();
        }

        /**
         * 查询单个链接状态
         */
        async checkSingleLinkStatus(url, link) {
            // 检查缓存
            const cached = this.getCachedResult(url);
            if (cached) {
                this.updateLinkStatus(link, cached);
                return;
            }
            
            // 检查是否正在请求中
            if (interceptorState.pendingRequests.has(url)) {
                return;
            }
            
            interceptorState.pendingRequests.set(url, true);
            this.updateLinkStatus(link, { status: LINK_STATUS.CHECKING });
            
            try {
                const response = await fetch(`${CONFIG.LINK_STATUS_API}?url=${encodeURIComponent(url)}`);
                
                if (!response.ok) {
                    throw new Error(`查询失败: ${response.status}`);
                }
                
                const result = await response.json();
                this.updateLinkStatus(link, result);
                this.cacheResult(url, result);
                
            } catch (error) {
                this.log(`单个查询错误: ${error.message}`, 'error');
                this.updateLinkStatus(link, { status: LINK_STATUS.ERROR });
            } finally {
                interceptorState.pendingRequests.delete(url);
            }
        }

        /**
         * 更新链接状态
         */
        updateLinkStatus(link, result) {
            const status = result.status;
            const prevStatus = link.dataset.monolithStatus;
            
            // 更新数据属性
            link.dataset.monolithStatus = status;
            if (result.record_id) {
                link.dataset.monolithRecordId = result.record_id;
            }
            if (result.suggested_action) {
                link.dataset.monolithAction = result.suggested_action;
            }
            
            // 更新状态指示器
            this.updateStatusIndicator(link, status, prevStatus);
            
            // 更新链接样式
            this.updateLinkStyle(link, status);
            
            // 增加处理计数
            if (prevStatus === LINK_STATUS.UNKNOWN) {
                interceptorState.processedLinks++;
            }
        }

        /**
         * 添加状态指示器
         */
        addStatusIndicator(link) {
            // 避免重复添加
            if (link.querySelector('.monolith-status-indicator')) {
                return;
            }
            
            const indicator = document.createElement('span');
            indicator.className = 'monolith-status-indicator';
            indicator.setAttribute('aria-hidden', 'true');
            indicator.innerHTML = '●';
            
            // 使用CSS类而非内联样式以利用外部CSS文件
            link.appendChild(indicator);
            link.dataset.monolithProcessed = 'true';
            
            // 添加初始状态
            this.updateStatusIndicator(link, LINK_STATUS.UNKNOWN);
            
            // 添加悬停增强效果
            this.addHoverEffects(link);
        }

        /**
         * 更新状态指示器
         */
        updateStatusIndicator(link, status, prevStatus = null) {
            const indicator = link.querySelector('.monolith-status-indicator');
            if (!indicator) return;
            
            // 移除之前的状态类
            indicator.classList.remove(
                'animate-pulse', 'animate-spin', 'monolith-status-unknown',
                'monolith-status-checking', 'monolith-status-cached',
                'monolith-status-processing', 'monolith-status-expired',
                'monolith-status-failed', 'monolith-status-error'
            );
            
            // 添加新的状态类
            const statusClassMap = {
                [LINK_STATUS.UNKNOWN]: 'monolith-status-unknown',
                [LINK_STATUS.CHECKING]: 'monolith-status-checking animate-pulse',
                [LINK_STATUS.CACHED]: 'monolith-status-cached',
                [LINK_STATUS.PROCESSING]: 'monolith-status-processing animate-spin',
                [LINK_STATUS.EXPIRED]: 'monolith-status-expired',
                [LINK_STATUS.FAILED]: 'monolith-status-failed',
                [LINK_STATUS.NOT_FOUND]: 'monolith-status-unknown',
                [LINK_STATUS.ERROR]: 'monolith-status-error'
            };
            
            const statusClasses = statusClassMap[status] || statusClassMap[LINK_STATUS.UNKNOWN];
            indicator.className = `monolith-status-indicator ${statusClasses}`;
            
            // 添加动画效果
            if (prevStatus && prevStatus !== status) {
                indicator.classList.add('animate-bounce');
                setTimeout(() => {
                    indicator.classList.remove('animate-bounce');
                }, CONFIG.ANIMATION_DURATION);
            }
            
            // 创建高级工具提示
            this.createAdvancedTooltip(link, status);
        }

        /**
         * 更新链接样式
         */
        updateLinkStyle(link, status) {
            // 移除之前的状态类
            link.classList.remove(
                'monolith-cached', 'monolith-processing', 
                'monolith-expired', 'monolith-failed', 
                'monolith-checking'
            );
            
            // 添加新的状态类
            const statusClasses = {
                [LINK_STATUS.CACHED]: 'monolith-cached',
                [LINK_STATUS.PROCESSING]: 'monolith-processing',
                [LINK_STATUS.EXPIRED]: 'monolith-expired',
                [LINK_STATUS.FAILED]: 'monolith-failed',
                [LINK_STATUS.CHECKING]: 'monolith-checking'
            };
            
            const className = statusClasses[status];
            if (className) {
                link.classList.add(className);
            }
        }

        /**
         * 显示选择对话框
         */
        showChoiceDialog(url, link) {
            const recordId = link.dataset.monolithRecordId;
            
            const dialog = document.createElement('div');
            dialog.className = 'monolith-choice-dialog';
            dialog.innerHTML = `
                <div class="monolith-dialog-overlay">
                    <div class="monolith-dialog-content">
                        <h3>选择处理方式</h3>
                        <p>链接: <code>${this.truncateUrl(url)}</code></p>
                        <div class="monolith-dialog-actions">
                            ${recordId ? `<button class="btn-cache" data-action="cache">查看缓存版本</button>` : ''}
                            <button class="btn-reprocess" data-action="reprocess">重新处理</button>
                            <button class="btn-cancel" data-action="cancel">取消</button>
                        </div>
                    </div>
                </div>
            `;
            
            // 添加样式
            this.addDialogStyles(dialog);
            
            // 绑定事件
            dialog.addEventListener('click', (e) => {
                const action = e.target.dataset.action;
                
                switch (action) {
                    case 'cache':
                        window.location.href = `/library/${recordId}`;
                        break;
                    case 'reprocess':
                        this.navigateToSmartRoute(url, true);
                        break;
                    case 'cancel':
                        document.body.removeChild(dialog);
                        break;
                }
                
                if (action) {
                    document.body.removeChild(dialog);
                }
            });
            
            document.body.appendChild(dialog);
        }

        /**
         * 跳转到智能路由
         */
        navigateToSmartRoute(url, forceReprocess = false) {
            let smartUrl = `${CONFIG.SMART_ROUTE_BASE}/${encodeURIComponent(url)}`;
            
            if (forceReprocess) {
                smartUrl += '?force_reprocess=true';
            }
            
            window.location.href = smartUrl;
        }

        /**
         * 显示状态提示
         */
        showStatusTooltip(link) {
            // 简化实现，使用浏览器原生tooltip
            const status = link.dataset.monolithStatus;
            const action = link.dataset.monolithAction;
            
            let tooltip = `状态: ${this.getStatusText(status)}`;
            if (action) {
                tooltip += `\n建议: ${this.getActionText(action)}`;
            }
            
            link.title = tooltip;
        }

        /**
         * 启动状态更新器
         */
        startStatusUpdater() {
            if (interceptorState.statsInterval) {
                clearInterval(interceptorState.statsInterval);
            }
            
            interceptorState.statsInterval = setInterval(() => {
                this.updateStats();
                this.refreshProcessingLinks();
            }, CONFIG.STATUS_UPDATE_INTERVAL);
        }

        /**
         * 刷新处理中的链接状态
         */
        refreshProcessingLinks() {
            const processingLinks = document.querySelectorAll(
                `a[data-monolith-status="${LINK_STATUS.PROCESSING}"]`
            );
            
            processingLinks.forEach(link => {
                this.checkSingleLinkStatus(link.href, link);
            });
        }

        /**
         * 刷新所有链接状态
         */
        refreshLinkStatus() {
            this.log('刷新链接状态');
            
            const allLinks = document.querySelectorAll('a[data-monolith-processed]');
            this.batchCheckLinkStatus(Array.from(allLinks));
        }

        /**
         * 初始化UI
         */
        initializeUI() {
            this.addGlobalStyles();
            this.createStatusPanel();
        }

        /**
         * 添加全局样式
         */
        addGlobalStyles() {
            if (document.getElementById('monolith-interceptor-styles')) {
                return;
            }
            
            const styles = document.createElement('style');
            styles.id = 'monolith-interceptor-styles';
            styles.textContent = `
                /* 链接状态样式 */
                a.monolith-cached {
                    border-left: 3px solid #4caf50 !important;
                    padding-left: 4px !important;
                }
                
                a.monolith-processing {
                    border-left: 3px solid #2196f3 !important;
                    padding-left: 4px !important;
                    animation: monolith-pulse 1.5s infinite;
                }
                
                a.monolith-expired {
                    border-left: 3px solid #ff5722 !important;
                    padding-left: 4px !important;
                }
                
                a.monolith-failed {
                    border-left: 3px solid #f44336 !important;
                    padding-left: 4px !important;
                }
                
                a.monolith-checking {
                    opacity: 0.7;
                    animation: monolith-checking 1s infinite;
                }
                
                /* 动画 */
                @keyframes monolith-pulse {
                    0%, 100% { opacity: 1; }
                    50% { opacity: 0.6; }
                }
                
                @keyframes monolith-checking {
                    0%, 100% { opacity: 0.7; }
                    50% { opacity: 1; }
                }
                
                /* 对话框样式 */
                .monolith-choice-dialog .monolith-dialog-overlay {
                    position: fixed;
                    top: 0;
                    left: 0;
                    width: 100%;
                    height: 100%;
                    background: rgba(0, 0, 0, 0.5);
                    display: flex;
                    align-items: center;
                    justify-content: center;
                    z-index: 10000;
                }
                
                .monolith-choice-dialog .monolith-dialog-content {
                    background: white;
                    padding: 20px;
                    border-radius: 8px;
                    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
                    max-width: 500px;
                    width: 90%;
                }
                
                .monolith-choice-dialog h3 {
                    margin-top: 0;
                    color: #333;
                }
                
                .monolith-choice-dialog code {
                    background: #f5f5f5;
                    padding: 2px 4px;
                    border-radius: 3px;
                    word-break: break-all;
                }
                
                .monolith-dialog-actions {
                    margin-top: 15px;
                    text-align: right;
                }
                
                .monolith-dialog-actions button {
                    margin-left: 8px;
                    padding: 8px 16px;
                    border: none;
                    border-radius: 4px;
                    cursor: pointer;
                    font-size: 14px;
                }
                
                .monolith-dialog-actions .btn-cache {
                    background: #4caf50;
                    color: white;
                }
                
                .monolith-dialog-actions .btn-reprocess {
                    background: #2196f3;
                    color: white;
                }
                
                .monolith-dialog-actions .btn-cancel {
                    background: #f5f5f5;
                    color: #333;
                }
                
                .monolith-dialog-actions button:hover {
                    opacity: 0.9;
                }
                
                /* 状态面板样式 */
                .monolith-status-panel {
                    position: fixed;
                    bottom: 20px;
                    right: 20px;
                    background: rgba(0, 0, 0, 0.8);
                    color: white;
                    padding: 10px;
                    border-radius: 6px;
                    font-size: 12px;
                    z-index: 9999;
                    transition: opacity ${CONFIG.ANIMATION_DURATION}ms ease;
                }
                
                .monolith-status-panel.hidden {
                    opacity: 0;
                    pointer-events: none;
                }
            `;
            
            document.head.appendChild(styles);
        }

        /**
         * 创建状态面板
         */
        createStatusPanel() {
            if (document.getElementById('monolith-status-panel')) {
                return;
            }
            
            const panel = document.createElement('div');
            panel.id = 'monolith-status-panel';
            panel.className = 'monolith-status-panel';
            
            document.body.appendChild(panel);
            this.updateStats();
        }

        /**
         * 更新统计信息
         */
        updateStats() {
            const panel = document.getElementById('monolith-status-panel');
            if (!panel) return;
            
            const { totalLinks, processedLinks } = interceptorState;
            const progress = totalLinks > 0 ? Math.round((processedLinks / totalLinks) * 100) : 0;
            
            // 统计各种状态的链接数
            const statusCounts = this.getStatusCounts();
            
            panel.innerHTML = `
                <div class="panel-header">
                    <span>🔗 Monolith智能拦截器</span>
                    <span class="status-badge">${processedLinks}/${totalLinks}</span>
                </div>
                <div class="panel-content">
                    <div class="progress-bar">
                        <div class="progress-fill" style="width: ${progress}%"></div>
                    </div>
                    <div class="status-summary">
                        ${statusCounts.cached > 0 ? `✅ 已缓存: ${statusCounts.cached}` : ''}
                        ${statusCounts.processing > 0 ? `🔄 处理中: ${statusCounts.processing}` : ''}
                        ${statusCounts.checking > 0 ? `🔍 检查中: ${statusCounts.checking}` : ''}
                        ${statusCounts.failed > 0 ? `❌ 失败: ${statusCounts.failed}` : ''}
                    </div>
                </div>
            `;
            
            // 处理完成后隐藏面板
            if (processedLinks >= totalLinks && totalLinks > 0) {
                setTimeout(() => {
                    panel.classList.add('hidden');
                }, 3000);
            }
        }

        /**
         * 获取状态统计
         */
        getStatusCounts() {
            const counts = {
                cached: 0,
                processing: 0,
                checking: 0,
                failed: 0,
                expired: 0,
                unknown: 0
            };
            
            const allLinks = document.querySelectorAll('a[data-monolith-status]');
            allLinks.forEach(link => {
                const status = link.dataset.monolithStatus;
                if (status === LINK_STATUS.CACHED) counts.cached++;
                else if (status === LINK_STATUS.PROCESSING) counts.processing++;
                else if (status === LINK_STATUS.CHECKING) counts.checking++;
                else if (status === LINK_STATUS.FAILED) counts.failed++;
                else if (status === LINK_STATUS.EXPIRED) counts.expired++;
                else counts.unknown++;
            });
            
            return counts;
        }

        /**
         * 缓存相关方法
         */
        loadCache() {
            try {
                const cached = localStorage.getItem(CONFIG.STORAGE_KEY);
                if (cached) {
                    const data = JSON.parse(cached);
                    const now = Date.now();
                    
                    // 清理过期缓存
                    Object.keys(data).forEach(url => {
                        if (now - data[url].timestamp < CONFIG.CACHE_DURATION) {
                            interceptorState.cache.set(url, data[url]);
                        }
                    });
                }
            } catch (e) {
                this.log('加载缓存失败', 'error');
            }
        }

        saveCache() {
            try {
                const cacheObj = {};
                interceptorState.cache.forEach((value, key) => {
                    cacheObj[key] = value;
                });
                localStorage.setItem(CONFIG.STORAGE_KEY, JSON.stringify(cacheObj));
            } catch (e) {
                this.log('保存缓存失败', 'error');
            }
        }

        cacheResult(url, result) {
            interceptorState.cache.set(url, {
                ...result,
                timestamp: Date.now()
            });
        }

        getCachedResult(url) {
            const cached = interceptorState.cache.get(url);
            if (cached && Date.now() - cached.timestamp < CONFIG.CACHE_DURATION) {
                return cached;
            }
            return null;
        }

        /**
         * 工具方法
         */
        chunkArray(array, size) {
            const chunks = [];
            for (let i = 0; i < array.length; i += size) {
                chunks.push(array.slice(i, i + size));
            }
            return chunks;
        }

        delay(ms) {
            return new Promise(resolve => setTimeout(resolve, ms));
        }

        truncateUrl(url, maxLength = 50) {
            return url.length > maxLength ? url.substring(0, maxLength) + '...' : url;
        }

        getStatusText(status) {
            const texts = {
                [LINK_STATUS.UNKNOWN]: '未知',
                [LINK_STATUS.CHECKING]: '检查中',
                [LINK_STATUS.CACHED]: '已缓存',
                [LINK_STATUS.PROCESSING]: '处理中',
                [LINK_STATUS.EXPIRED]: '已过期',
                [LINK_STATUS.FAILED]: '失败',
                [LINK_STATUS.NOT_FOUND]: '未找到',
                [LINK_STATUS.ERROR]: '错误'
            };
            return texts[status] || '未知';
        }

        getActionText(action) {
            const texts = {
                'use_cache': '使用缓存',
                'wait': '等待处理',
                'reprocess': '重新处理',
                'process': '开始处理',
                'retry': '重试'
            };
            return texts[action] || action;
        }

        /**
         * 添加悬停增强效果
         */
        addHoverEffects(link) {
            link.addEventListener('mouseenter', () => {
                link.classList.add('monolith-link-hover');
                this.showAdvancedTooltip(link);
            });
            
            link.addEventListener('mouseleave', () => {
                link.classList.remove('monolith-link-hover');
                this.hideAdvancedTooltip(link);
            });
        }

        /**
         * 创建高级工具提示
         */
        createAdvancedTooltip(link, status) {
            const tooltipId = `monolith-tooltip-${Math.random().toString(36).substr(2, 9)}`;
            link.dataset.tooltipId = tooltipId;
            
            const statusInfo = this.getStatusInfo(status);
            const action = link.dataset.monolithAction;
            
            let tooltipContent = `
                <div class="tooltip-header">
                    <strong>${statusInfo.title}</strong>
                    <span class="status-badge status-${status}">${statusInfo.shortText}</span>
                </div>
                <div class="tooltip-body">
                    ${statusInfo.description}
                </div>
            `;
            
            if (action) {
                tooltipContent += `
                    <div class="tooltip-action">
                        <strong>建议:</strong> ${this.getActionText(action)}
                    </div>
                `;
            }
            
            // 存储工具提示内容供后续显示
            link.dataset.tooltipContent = tooltipContent;
        }

        /**
         * 显示高级工具提示
         */
        showAdvancedTooltip(link) {
            // 移除现有工具提示
            this.hideAllTooltips();
            
            const tooltipContent = link.dataset.tooltipContent;
            if (!tooltipContent) return;
            
            const tooltip = document.createElement('div');
            tooltip.className = 'monolith-tooltip';
            tooltip.innerHTML = tooltipContent;
            
            // 计算位置
            const rect = link.getBoundingClientRect();
            const tooltipRect = { width: 300, height: 100 }; // 估算大小
            
            let top = rect.bottom + 8;
            let left = rect.left + (rect.width / 2) - (tooltipRect.width / 2);
            
            // 边界检查
            if (left < 10) left = 10;
            if (left + tooltipRect.width > window.innerWidth - 10) {
                left = window.innerWidth - tooltipRect.width - 10;
            }
            if (top + tooltipRect.height > window.innerHeight - 10) {
                top = rect.top - tooltipRect.height - 8;
            }
            
            tooltip.style.cssText = `
                position: fixed;
                top: ${top}px;
                left: ${left}px;
                z-index: 10000;
            `;
            
            document.body.appendChild(tooltip);
            
            // 动画显示
            requestAnimationFrame(() => {
                tooltip.classList.add('show');
            });
            
            link.dataset.activeTooltip = 'true';
        }

        /**
         * 隐藏高级工具提示
         */
        hideAdvancedTooltip(link) {
            const tooltip = document.querySelector('.monolith-tooltip.show');
            if (tooltip) {
                tooltip.classList.remove('show');
                setTimeout(() => {
                    if (tooltip.parentNode) {
                        tooltip.parentNode.removeChild(tooltip);
                    }
                }, CONFIG.ANIMATION_DURATION);
            }
            
            if (link) {
                delete link.dataset.activeTooltip;
            }
        }

        /**
         * 隐藏所有工具提示
         */
        hideAllTooltips() {
            const tooltips = document.querySelectorAll('.monolith-tooltip');
            tooltips.forEach(tooltip => {
                if (tooltip.parentNode) {
                    tooltip.parentNode.removeChild(tooltip);
                }
            });
        }

        /**
         * 获取状态信息
         */
        getStatusInfo(status) {
            const statusInfoMap = {
                [LINK_STATUS.UNKNOWN]: {
                    title: '未知状态',
                    shortText: '未知',
                    description: '尚未检查此链接的缓存状态，点击链接将自动检查并处理。'
                },
                [LINK_STATUS.CHECKING]: {
                    title: '正在检查',
                    shortText: '检查中',
                    description: '正在查询链接的缓存状态，请稍候。这通常只需要几秒钟。'
                },
                [LINK_STATUS.CACHED]: {
                    title: '已缓存',
                    shortText: '可用',
                    description: '此链接已被处理并缓存，点击可快速访问本地版本。'
                },
                [LINK_STATUS.PROCESSING]: {
                    title: '处理中',
                    shortText: '处理中',
                    description: '正在处理此链接的内容，包括下载、翻译和优化。完成后将自动可用。'
                },
                [LINK_STATUS.EXPIRED]: {
                    title: '缓存过期',
                    shortText: '过期',
                    description: '缓存版本已过期，可以查看旧版本或重新处理获取最新内容。'
                },
                [LINK_STATUS.FAILED]: {
                    title: '处理失败',
                    shortText: '失败',
                    description: '处理此链接时出现错误，可以重试或查看错误详情。'
                },
                [LINK_STATUS.NOT_FOUND]: {
                    title: '未找到',
                    shortText: '未找到',
                    description: '未找到此链接的缓存记录，将开始新的处理流程。'
                },
                [LINK_STATUS.ERROR]: {
                    title: '查询错误',
                    shortText: '错误',
                    description: '查询链接状态时发生错误，可能是网络问题或服务暂时不可用。'
                }
            };
            
            return statusInfoMap[status] || statusInfoMap[LINK_STATUS.UNKNOWN];
        }

        /**
         * 创建进度通知
         */
        createProgressNotification(message, type = 'info') {
            const notification = document.createElement('div');
            notification.className = `monolith-notification ${type}`;
            
            notification.innerHTML = `
                <div class="notification-header">
                    <span class="notification-icon">${this.getNotificationIcon(type)}</span>
                    <span class="notification-title">${this.getNotificationTitle(type)}</span>
                    <button class="notification-close" onclick="this.parentElement.parentElement.remove()">×</button>
                </div>
                <div class="notification-body">${message}</div>
            `;
            
            document.body.appendChild(notification);
            
            // 自动移除
            setTimeout(() => {
                if (notification.parentNode) {
                    notification.classList.add('slide-out');
                    setTimeout(() => {
                        if (notification.parentNode) {
                            notification.parentNode.removeChild(notification);
                        }
                    }, CONFIG.ANIMATION_DURATION);
                }
            }, 5000);
            
            return notification;
        }

        /**
         * 获取通知图标
         */
        getNotificationIcon(type) {
            const icons = {
                'info': 'ℹ️',
                'success': '✅',
                'warning': '⚠️',
                'error': '❌'
            };
            return icons[type] || icons['info'];
        }

        /**
         * 获取通知标题
         */
        getNotificationTitle(type) {
            const titles = {
                'info': '信息',
                'success': '成功',
                'warning': '警告',
                'error': '错误'
            };
            return titles[type] || titles['info'];
        }

        /**
         * 创建加载器
         */
        createLoader(type = 'spinner') {
            const loader = document.createElement('div');
            loader.className = 'monolith-loader';
            
            if (type === 'spinner') {
                loader.innerHTML = '<div class="monolith-loader-spinner"></div>';
            } else if (type === 'dots') {
                loader.innerHTML = `
                    <div class="monolith-loader-dots">
                        <div></div>
                        <div></div>
                        <div></div>
                    </div>
                `;
            }
            
            return loader;
        }

        addDialogStyles(dialog) {
            // 样式已在全局样式中定义
        }

        log(message, level = 'info') {
            if (CONFIG.DEBUG) {
                console[level](`[MonolithInterceptor] ${message}`);
            }
        }
    }

    /**
     * 自动初始化
     */
    function autoInit() {
        // 检查是否已经初始化
        if (window.monolithInterceptor) {
            return;
        }

        // 检查是否在monolith页面中
        if (document.querySelector('meta[name="monolith-page"]') || 
            location.pathname.startsWith('/library/') ||
            location.pathname.startsWith('/smart-website/')) {
            
            window.monolithInterceptor = new SmartLinkInterceptor();
        }
    }

    // 导出到全局
    window.SmartLinkInterceptor = SmartLinkInterceptor;
    window.MONOLITH_CONFIG = CONFIG;

    // 自动初始化
    autoInit();

})();