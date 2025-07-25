/**
 * Monolith 网页翻译器 - API 管理模块
 * 整合自 library-data.js - 提供统一的API调用和数据管理
 */

class ApiManager {
    constructor() {
        this.baseUrl = '';
        this.defaultTimeout = 30000;
        this.retryAttempts = 3;
        this.retryDelay = 1000;
    }

    /**
     * 通用API调用方法
     * @param {string} endpoint - API端点
     * @param {Object} options - 请求选项
     * @returns {Promise<Object>} API响应数据
     */
    async apiCall(endpoint, options = {}) {
        const {
            method = 'GET',
            body = null,
            headers = {},
            timeout = this.defaultTimeout,
            retry = true
        } = options;

        const defaultHeaders = {
            'Content-Type': 'application/json',
            ...headers
        };

        const requestOptions = {
            method,
            headers: defaultHeaders,
            ...(body && { body: typeof body === 'string' ? body : JSON.stringify(body) })
        };

        let lastError;
        const attempts = retry ? this.retryAttempts : 1;

        for (let attempt = 1; attempt <= attempts; attempt++) {
            try {
                const controller = new AbortController();
                const timeoutId = setTimeout(() => controller.abort(), timeout);

                const response = await fetch(endpoint, {
                    ...requestOptions,
                    signal: controller.signal
                });

                clearTimeout(timeoutId);

                if (!response.ok) {
                    const errorText = await response.text();
                    throw new Error(`HTTP ${response.status}: ${errorText}`);
                }

                const data = await response.json();
                return data;

            } catch (error) {
                lastError = error;
                
                if (attempt < attempts && this.shouldRetry(error)) {
                    console.warn(`API调用失败 (尝试 ${attempt}/${attempts}):`, error.message);
                    await this.delay(this.retryDelay * attempt);
                    continue;
                }
                
                break;
            }
        }

        console.error(`API调用最终失败 (${endpoint}):`, lastError);
        throw lastError;
    }

    /**
     * 判断是否应该重试
     * @param {Error} error - 错误对象
     * @returns {boolean} 是否应该重试
     */
    shouldRetry(error) {
        // 网络错误或5xx服务器错误可以重试
        return error.name === 'TypeError' || 
               error.message.includes('fetch') ||
               error.message.includes('5');
    }

    /**
     * 延迟工具函数
     * @param {number} ms - 延迟毫秒数
     * @returns {Promise<void>}
     */
    delay(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }

    /**
     * 加载翻译库数据
     * @param {Object} params - 查询参数
     * @returns {Promise<Object>} 翻译记录数据
     */
    async loadLibraryData(params = {}) {
        try {
            const queryString = this.buildQueryParams(params);
            const response = await this.apiCall(`/api/v2/library?${queryString}`);
            
            console.log(`数据加载完成: ${response.records?.length || 0} 条记录`);
            if (response.meta) {
                console.log(`查询耗时: ${response.meta.query_time_ms}ms, 应用了 ${response.meta.filters_applied} 个筛选条件`);
            }
            
            return response;
        } catch (error) {
            console.error('加载翻译库数据失败:', error);
            throw new Error(`数据加载失败: ${error.message}`);
        }
    }

    /**
     * 获取统计数据
     * @returns {Promise<Object>} 统计数据
     */
    async getStats() {
        try {
            // 优先使用详细的仪表板统计
            try {
                const stats = await this.apiCall('/api/v2/library/stats/dashboard');
                return {
                    ...stats,
                    source: 'dashboard'
                };
            } catch (dashboardError) {
                console.warn('仪表板统计API失败，降级到基础统计:', dashboardError.message);
                
                // 降级到基础统计
                const basicStats = await this.apiCall('/api/v2/library/stats');
                return {
                    today: {
                        total: basicStats.total || 0,
                        success: basicStats.success || 0,
                        storage_size: basicStats.storage_size || 0
                    },
                    source: 'basic'
                };
            }
        } catch (error) {
            console.error('获取统计数据失败:', error);
            throw new Error(`统计数据获取失败: ${error.message}`);
        }
    }

    /**
     * 删除记录
     * @param {string} id - 记录ID
     * @returns {Promise<Object>} 删除结果
     */
    async deleteRecord(id) {
        try {
            const response = await this.apiCall(`/api/v2/library/${id}`, {
                method: 'DELETE'
            });
            
            console.log(`记录删除成功: ${id}`);
            return response;
        } catch (error) {
            console.error(`删除记录失败 (${id}):`, error);
            throw new Error(`删除失败: ${error.message}`);
        }
    }

    /**
     * 批量删除记录
     * @param {Array<string>} ids - 记录ID数组
     * @returns {Promise<Object>} 删除结果
     */
    async batchDeleteRecords(ids) {
        try {
            const response = await this.apiCall('/api/v2/library/batch/delete', {
                method: 'POST',
                body: { ids }
            });
            
            console.log(`批量删除成功: ${ids.length} 条记录`);
            return response;
        } catch (error) {
            console.error('批量删除失败:', error);
            throw new Error(`批量删除失败: ${error.message}`);
        }
    }

    /**
     * 导出记录
     * @param {Array<string>} ids - 记录ID数组（可选，不提供则导出全部）
     * @param {string} format - 导出格式 ('json', 'csv', 'html')
     * @returns {Promise<Blob>} 导出文件
     */
    async exportRecords(ids = null, format = 'json') {
        try {
            const endpoint = ids ? '/api/v2/library/export/selected' : '/api/v2/library/export';
            const body = ids ? { ids, format } : { format };
            
            const response = await fetch(endpoint, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify(body)
            });

            if (!response.ok) {
                throw new Error(`导出失败: HTTP ${response.status}`);
            }

            const blob = await response.blob();
            console.log(`导出完成: ${format} 格式, ${blob.size} 字节`);
            return blob;
        } catch (error) {
            console.error('导出记录失败:', error);
            throw new Error(`导出失败: ${error.message}`);
        }
    }

    /**
     * 获取缓存统计
     * @returns {Promise<Object>} 缓存统计数据
     */
    async getCacheStats() {
        try {
            const stats = await this.apiCall('/api/cache/stats');
            return stats;
        } catch (error) {
            console.error('获取缓存统计失败:', error);
            throw new Error(`缓存统计获取失败: ${error.message}`);
        }
    }

    /**
     * 清理缓存
     * @param {string} type - 清理类型 ('all', 'expired', 'failed')
     * @returns {Promise<Object>} 清理结果
     */
    async cleanupCache(type = 'expired') {
        try {
            const response = await this.apiCall('/api/cache/cleanup', {
                method: 'POST',
                body: { type }
            });
            
            console.log(`缓存清理完成: ${type} 类型`);
            return response;
        } catch (error) {
            console.error('缓存清理失败:', error);
            throw new Error(`缓存清理失败: ${error.message}`);
        }
    }

    /**
     * 翻译网页
     * @param {string} url - 要翻译的URL
     * @param {string} targetLang - 目标语言
     * @returns {Promise<Object>} 翻译结果
     */
    async translatePage(url, targetLang = 'zh') {
        try {
            const response = await this.apiCall('/api/translate', {
                method: 'POST',
                body: { url, target_lang: targetLang },
                timeout: 60000 // 翻译可能需要更长时间
            });
            
            console.log(`页面翻译完成: ${url}`);
            return response;
        } catch (error) {
            console.error(`页面翻译失败 (${url}):`, error);
            throw new Error(`翻译失败: ${error.message}`);
        }
    }

    /**
     * 构建查询参数字符串
     * @param {Object} params - 参数对象
     * @returns {string} 查询字符串
     */
    buildQueryParams(params) {
        const searchParams = new URLSearchParams();
        
        Object.entries(params).forEach(([key, value]) => {
            if (value !== null && value !== undefined && value !== '') {
                searchParams.append(key, value);
            }
        });
        
        return searchParams.toString();
    }

    /**
     * 格式化文件大小
     * @param {number} bytes - 字节数
     * @returns {string} 格式化后的大小
     */
    formatFileSize(bytes) {
        if (bytes === 0) return '0 B';
        
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

    /**
     * 格式化日期
     * @param {string|Date} date - 日期
     * @returns {string} 格式化后的日期
     */
    formatDate(date) {
        if (!date) return '-';
        
        const d = new Date(date);
        if (isNaN(d.getTime())) return '-';
        
        return d.toLocaleString('zh-CN', {
            year: 'numeric',
            month: '2-digit',
            day: '2-digit',
            hour: '2-digit',
            minute: '2-digit',
            second: '2-digit',
            hour12: false
        });
    }

    /**
     * 截断文本
     * @param {string} text - 原始文本
     * @param {number} maxLength - 最大长度
     * @returns {string} 截断后的文本
     */
    truncateText(text, maxLength = 50) {
        if (!text || text.length <= maxLength) return text || '';
        return text.substring(0, maxLength) + '...';
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
     * 节流函数
     * @param {Function} func - 要节流的函数
     * @param {number} limit - 限制时间
     * @returns {Function} 节流后的函数
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

// 导出给全局使用
window.ApiManager = ApiManager;