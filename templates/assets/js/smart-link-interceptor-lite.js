/**
 * Monolith智能链接拦截器 - 轻量级版本
 * 
 * 适用于：
 * - 低性能设备
 * - 旧版浏览器
 * - 简单页面
 * 
 * @version 1.0.0-lite
 * @author Monolith Team
 */

(function() {
    'use strict';

    // 简化配置
    const CONFIG = {
        API_BASE: '/api/v2/link-status',
        SMART_ROUTE: '/smart-website',
        CACHE_KEY: 'monolith_lite_cache',
        CACHE_DURATION: 300000, // 5分钟
        MAX_LINKS: 50, // 限制处理的链接数量
        DEBUG: false
    };

    // 状态定义
    const STATUS = {
        UNKNOWN: 'unknown',
        CACHED: 'cached',
        PROCESSING: 'processing',
        NOT_FOUND: 'not_found'
    };

    let initialized = false;
    let cache = {};

    /**
     * 轻量级链接拦截器
     */
    function initLiteInterceptor() {
        if (initialized) return;
        
        log('初始化轻量级链接拦截器');
        
        // 加载缓存
        loadCache();
        
        // 查找链接
        const links = findLinks();
        if (links.length === 0) {
            log('未找到链接');
            return;
        }
        
        log(`处理 ${links.length} 个链接`);
        
        // 处理链接
        links.forEach(processLink);
        
        // 保存缓存
        window.addEventListener('beforeunload', saveCache);
        
        initialized = true;
    }

    /**
     * 查找页面链接
     */
    function findLinks() {
        const links = Array.from(document.querySelectorAll('a[href^="http"]:not([data-monolith-ignore])'));
        
        return links.filter(link => {
            const url = link.href;
            if (!url || url === '#') return false;
            
            try {
                const urlObj = new URL(url);
                // 跳过本地链接
                if (urlObj.hostname === 'localhost' || urlObj.hostname === '127.0.0.1') {
                    return false;
                }
            } catch (e) {
                return false;
            }
            
            return !link.dataset.monolithLite;
        }).slice(0, CONFIG.MAX_LINKS); // 限制数量
    }

    /**
     * 处理单个链接
     */
    function processLink(link) {
        const url = link.href;
        
        // 标记为已处理
        link.dataset.monolithLite = 'true';
        
        // 检查缓存
        const cached = getCached(url);
        if (cached) {
            updateLinkStatus(link, cached);
            return;
        }
        
        // 添加点击拦截
        link.addEventListener('click', function(e) {
            e.preventDefault();
            handleClick(url, link);
        });
        
        // 添加悬停查询
        link.addEventListener('mouseenter', function() {
            if (!link.dataset.monolithStatus) {
                queryStatus(url, link);
            }
        });
        
        // 初始状态
        updateLinkStatus(link, { status: STATUS.UNKNOWN });
    }

    /**
     * 处理点击事件
     */
    function handleClick(url, link) {
        const status = link.dataset.monolithStatus;
        
        switch (status) {
            case STATUS.CACHED:
                const recordId = link.dataset.monolithRecordId;
                if (recordId) {
                    window.location.href = `/library/${recordId}`;
                } else {
                    navigateToSmart(url);
                }
                break;
                
            case STATUS.PROCESSING:
                navigateToSmart(url);
                break;
                
            default:
                navigateToSmart(url);
        }
    }

    /**
     * 跳转到智能路由
     */
    function navigateToSmart(url) {
        window.location.href = `${CONFIG.SMART_ROUTE}/${encodeURIComponent(url)}`;
    }

    /**
     * 查询链接状态
     */
    function queryStatus(url, link) {
        fetch(`${CONFIG.API_BASE}?url=${encodeURIComponent(url)}`)
            .then(response => response.json())
            .then(result => {
                updateLinkStatus(link, result);
                setCache(url, result);
            })
            .catch(error => {
                log(`查询失败: ${error.message}`);
                updateLinkStatus(link, { status: STATUS.UNKNOWN });
            });
    }

    /**
     * 更新链接状态
     */
    function updateLinkStatus(link, result) {
        const status = result.status;
        
        link.dataset.monolithStatus = status;
        if (result.record_id) {
            link.dataset.monolithRecordId = result.record_id;
        }
        
        // 添加视觉指示
        const colors = {
            [STATUS.UNKNOWN]: '#ccc',
            [STATUS.CACHED]: '#4caf50',
            [STATUS.PROCESSING]: '#2196f3',
            [STATUS.NOT_FOUND]: '#ff9800'
        };
        
        const color = colors[status] || colors[STATUS.UNKNOWN];
        link.style.borderLeft = `3px solid ${color}`;
        link.style.paddingLeft = '4px';
        
        // 设置提示
        const titles = {
            [STATUS.UNKNOWN]: '未知状态',
            [STATUS.CACHED]: '已缓存，点击快速访问',
            [STATUS.PROCESSING]: '处理中，点击查看进度',
            [STATUS.NOT_FOUND]: '未缓存，点击开始处理'
        };
        
        link.title = titles[status] || '点击访问智能路由';
    }

    /**
     * 缓存管理
     */
    function loadCache() {
        try {
            const stored = localStorage.getItem(CONFIG.CACHE_KEY);
            if (stored) {
                const data = JSON.parse(stored);
                const now = Date.now();
                
                // 清理过期缓存
                Object.keys(data).forEach(url => {
                    if (now - data[url].timestamp < CONFIG.CACHE_DURATION) {
                        cache[url] = data[url];
                    }
                });
            }
        } catch (e) {
            log('加载缓存失败');
        }
    }

    function saveCache() {
        try {
            localStorage.setItem(CONFIG.CACHE_KEY, JSON.stringify(cache));
        } catch (e) {
            log('保存缓存失败');
        }
    }

    function getCached(url) {
        const cached = cache[url];
        if (cached && Date.now() - cached.timestamp < CONFIG.CACHE_DURATION) {
            return cached;
        }
        return null;
    }

    function setCache(url, result) {
        cache[url] = {
            ...result,
            timestamp: Date.now()
        };
    }

    /**
     * 日志
     */
    function log(message) {
        if (CONFIG.DEBUG) {
            console.log(`[MonolithLite] ${message}`);
        }
    }

    /**
     * 自动初始化
     */
    function autoInit() {
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', initLiteInterceptor);
        } else {
            initLiteInterceptor();
        }
    }

    // 导出
    window.MonolithLiteInterceptor = {
        init: initLiteInterceptor,
        CONFIG: CONFIG
    };

    // 检查是否启用轻量级模式
    if (window.location.search.includes('lite=1') || 
        document.querySelector('meta[name="monolith-lite"]')) {
        autoInit();
    }

})();