/**
 * Library Core - 核心功能模块
 */

class LibraryCore {
    constructor() {
        this.currentPage = 1;
        this.pageSize = 10;
        this.totalRecords = 0;
        this.currentSort = { column: 'created_at', direction: 'desc' };
        this.selectedItems = new Set();
        this.filters = {
            search: '',
            status: '',
            language: ''
        };
    }

    // 工具函数
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

    formatDate(dateString) {
        if (!dateString) return '-';
        const date = new Date(dateString);
        return date.toLocaleString('zh-CN');
    }

    formatFileSize(bytes) {
        if (!bytes) return '0 B';
        const units = ['B', 'KB', 'MB', 'GB'];
        let size = bytes;
        let unitIndex = 0;
        
        while (size >= 1024 && unitIndex < units.length - 1) {
            size /= 1024;
            unitIndex++;
        }
        
        return `${size.toFixed(1)} ${units[unitIndex]}`;
    }

    // 构建V2 API查询参数
    buildQueryParams() {
        const params = new URLSearchParams();
        
        // 分页参数
        params.append('page', this.currentPage);
        params.append('page_size', this.pageSize);
        
        // 排序参数
        if (this.currentSort.column) {
            params.append('sort', this.currentSort.column);
            params.append('order', this.currentSort.direction);
        }
        
        // 筛选参数
        if (this.filters.search) {
            params.append('search', this.filters.search);
        }
        if (this.filters.status) {
            params.append('status', this.filters.status);
        }
        if (this.filters.language) {
            params.append('source_lang', this.filters.language);
        }
        
        // 其他筛选参数
        if (this.filters.domain) {
            params.append('domain', this.filters.domain);
        }
        if (this.filters.date_from) {
            params.append('date_from', this.filters.date_from);
        }
        if (this.filters.date_to) {
            params.append('date_to', this.filters.date_to);
        }
        
        return params.toString();
    }

    // API调用封装
    async apiCall(url, options = {}) {
        const defaultOptions = {
            method: 'GET',
            headers: {
                'Content-Type': 'application/json',
                'Accept': 'application/json'
            }
        };
        
        const mergedOptions = { ...defaultOptions, ...options };
        
        try {
            const response = await fetch(url, mergedOptions);
            
            if (!response.ok) {
                let errorMessage = `HTTP ${response.status}`;
                try {
                    const errorData = await response.json();
                    errorMessage = errorData.message || errorMessage;
                } catch {
                    // 如果无法解析JSON，使用状态文本
                    errorMessage = response.statusText || errorMessage;
                }
                throw new Error(errorMessage);
            }
            
            return await response.json();
        } catch (error) {
            console.error('API调用失败:', error);
            throw error;
        }
    }

    // 获取状态文本
    getStatusText(status) {
        const statusMap = {
            'success': '翻译成功',
            'pending': '处理中',
            'error': '翻译失败'
        };
        return statusMap[status] || '未知状态';
    }

    // 截断文本
    truncateText(text, maxLength) {
        if (!text) return '';
        return text.length > maxLength ? text.substring(0, maxLength) + '...' : text;
    }

    showError(message) {
        // 简单的错误提示，可以替换为更好的UI组件
        alert('错误: ' + message);
    }

    showSuccess(message) {
        // 简单的成功提示，可以替换为更好的UI组件
        console.log('成功: ' + message);
    }
}