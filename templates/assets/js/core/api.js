// Monolith API 核心模块

/**
 * API 客户端类
 */
class MonolithAPI {
    constructor() {
        this.baseUrl = window.location.origin;
    }

    /**
     * 处理网页请求
     * @param {Object} data 请求数据
     * @returns {Promise<Object>} 处理结果
     */
    async process(data) {
        try {
            const response = await fetch(`${this.baseUrl}/api/process`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(data)
            });

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            return await response.json();
        } catch (error) {
            console.error('API请求失败:', error);
            throw error;
        }
    }

    /**
     * 获取内容
     * @param {string} url 要获取的URL
     * @returns {Promise<Object>} 内容数据
     */
    async getContent(url) {
        try {
            const response = await fetch(`${this.baseUrl}/api/content`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ url })
            });

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            return await response.json();
        } catch (error) {
            console.error('获取内容失败:', error);
            throw error;
        }
    }

    /**
     * 清理缓存
     * @returns {Promise<Object>} 清理结果
     */
    async clearCache() {
        try {
            const response = await fetch(`${this.baseUrl}/api/cache/clear`, {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({})
            });

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            return await response.json();
        } catch (error) {
            console.error('清理缓存失败:', error);
            throw error;
        }
    }

    /**
     * 获取书签脚本
     * @returns {Promise<Object>} 书签脚本数据
     */
    async getBookmarklet() {
        try {
            const response = await fetch(`${this.baseUrl}/api/bookmarklet`);

            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            return await response.json();
        } catch (error) {
            console.error('获取书签脚本失败:', error);
            throw error;
        }
    }
}

// 创建全局API实例
window.monolithAPI = new MonolithAPI();