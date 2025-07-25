/**
 * Library Data - 数据管理模块
 */

class LibraryData extends LibraryCore {
    constructor() {
        super();
    }

    // 加载数据
    async loadData() {
        try {
            // 显示加载状态
            this.tableBody.innerHTML = '<tr><td colspan="9" style="text-align: center; padding: 2rem;">加载中...</td></tr>';
            
            // 构建查询参数
            const params = this.buildQueryParams();
            
            // 发送请求到V2 API
            const response = await this.apiCall(`/api/v2/library?${params}`);
            
            this.renderTable(response.records);
            this.updatePagination(response.pagination);
            this.totalRecords = response.pagination.total;
            
            // 显示查询元数据
            if (response.meta) {
                console.log(`查询耗时: ${response.meta.query_time_ms}ms, 应用了 ${response.meta.filters_applied} 个筛选条件`);
            }
            
        } catch (error) {
            console.error('加载数据失败:', error);
            this.showError('加载数据失败，请稍后重试');
            this.tableBody.innerHTML = '<tr><td colspan="9" style="text-align: center; padding: 2rem; color: #dc3545;">加载失败</td></tr>';
        }
    }

    // 更新统计数据
    async updateStats() {
        try {
            // 使用V2 API的综合统计面板
            const stats = await this.apiCall('/api/v2/library/stats/dashboard');
            
            this.totalCountSpan.textContent = stats.today.total || 0;
            this.successCountSpan.textContent = stats.today.success || 0;
            this.todayCountSpan.textContent = stats.today.total || 0;
            this.storageSizeSpan.textContent = this.formatFileSize(stats.today.storage_size || 0);
            
            // 显示成功率
            if (stats.today && typeof stats.today.success_rate === 'number') {
                const successRate = (stats.today.success_rate * 100).toFixed(1);
                console.log(`今日翻译成功率: ${successRate}%`);
            } else {
                // 计算成功率：有翻译内容的记录数/总记录数
                const successCount = stats.today?.success || 0;
                const totalCount = stats.today?.total || 0;
                const successRate = totalCount > 0 ? ((successCount / totalCount) * 100).toFixed(1) : '0.0';
                console.log(`今日翻译成功率: ${successRate}%`);
            }
            
        } catch (error) {
            console.error('更新统计数据失败:', error);
            // 降级到基础统计API
            try {
                const basicStats = await this.apiCall('/api/v2/library/stats');
                this.totalCountSpan.textContent = basicStats.total || 0;
                this.successCountSpan.textContent = basicStats.success || 0;
                this.todayCountSpan.textContent = basicStats.today || 0;
                this.storageSizeSpan.textContent = this.formatFileSize(basicStats.storage_size || 0);
            } catch (fallbackError) {
                console.error('基础统计API也失败:', fallbackError);
            }
        }
    }

    // 渲染表格 - MongoDB风格
    renderTable(records) {
        if (!records || records.length === 0) {
            this.tableBody.innerHTML = '';
            this.emptyState.style.display = 'block';
            return;
        }
        
        this.emptyState.style.display = 'none';
        
        // 更新记录计数
        const recordCount = document.getElementById('record-count');
        if (recordCount) {
            recordCount.textContent = `${records.length} documents`;
        }
        
        this.tableBody.innerHTML = records.map(record => `
            <tr data-id="${record.id}" class="mongo-row">
                <td class="fixed-left">
                    <input type="checkbox" class="mongo-checkbox row-checkbox" value="${record.id}">
                </td>
                <td>
                    <span class="object-id" title="${record.id}">
                        ObjectId('${record.id.substring(0, 8)}...')
                    </span>
                </td>
                <td>
                    <a href="${record.url}" class="url-text" target="_blank" title="${record.url}">
                        ${this.truncateText(record.url, 35)}
                    </a>
                </td>
                <td>
                    <span class="title-text" title="${record.title || ''}">
                        ${this.truncateText(record.title || 'null', 25)}
                    </span>
                </td>
                <td>
                    <div class="html-cell" title="点击'查看'按钮预览完整页面">
                        ${this.truncateText(record.translated_html || 'null', 40)}
                    </div>
                </td>
                <td>
                    <span class="lang-tag">${record.source_lang || 'auto'}</span>
                </td>
                <td>
                    <span class="lang-tag">${record.target_lang || 'zh'}</span>
                </td>
                <td>
                    <span class="date-text">${this.formatMongoDate(record.created_at)}</span>
                </td>
                <td class="fixed-right">
                    <div class="record-actions">
                        <button class="action-icon-btn view" onclick="window.viewRecord && window.viewRecord('${record.id}', '${record.url.replace(/'/g, "\\\'")}')" title="查看">
                            <svg viewBox="0 0 16 16" width="14" height="14" fill="currentColor">
                                <path d="M8 1C4.318 1 1.204 3.618.293 7.5a.5.5 0 0 0 0 1C1.204 12.382 4.318 15 8 15s6.796-2.618 7.707-6.5a.5.5 0 0 0 0-1C14.796 3.618 11.682 1 8 1zM8 12.5a4.5 4.5 0 1 1 0-9 4.5 4.5 0 0 1 0 9z"/>
                                <path d="M8 5.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5z"/>
                            </svg>
                        </button>
                        <button class="action-icon-btn delete" onclick="window.deleteRecord && window.deleteRecord('${record.id}')" title="删除">
                            <svg viewBox="0 0 16 16" width="14" height="14" fill="currentColor">
                                <path d="M6.5 1h3a.5.5 0 0 1 .5.5v1H6v-1a.5.5 0 0 1 .5-.5zM11 2.5v-1A1.5 1.5 0 0 0 9.5 0h-3A1.5 1.5 0 0 0 5 1.5v1H2.506a.58.58 0 0 0-.01 0H1.5a.5.5 0 0 0 0 1h.538l.853 10.66A2 2 0 0 0 4.885 16h6.23a2 2 0 0 0 1.994-1.84L13.962 3.5H14.5a.5.5 0 0 0 0-1h-1.004a.58.58 0 0 0-.01 0H11z"/>
                            </svg>
                        </button>
                    </div>
                </td>
            </tr>
        `).join('');
        
        // 绑定行选择事件
        this.tableBody.querySelectorAll('.row-checkbox').forEach(checkbox => {
            checkbox.addEventListener('change', () => this.updateSelection());
        });
    }
    
    // 格式化MongoDB风格的日期
    formatMongoDate(timestamp) {
        if (!timestamp) return 'null';
        
        try {
            const date = new Date(timestamp * 1000); // 假设是Unix时间戳
            const year = date.getFullYear();
            const month = String(date.getMonth() + 1).padStart(2, '0');
            const day = String(date.getDate()).padStart(2, '0');
            const hours = String(date.getHours()).padStart(2, '0');
            const minutes = String(date.getMinutes()).padStart(2, '0');
            
            return `${year}-${month}-${day} ${hours}:${minutes}`;
        } catch (e) {
            return 'Invalid Date';
        }
    }

    // 单个记录操作  
    async viewRecord(id, url) {
        console.log(`查看记录: ID=${id}, URL=${url}`);
        
        if (!url) {
            this.showError('缺少原始URL信息');
            return;
        }
        
        // 跳转到website路由，传递原始URL
        const websiteUrl = `/website/${encodeURIComponent(url)}`;
        console.log(`跳转到website路由: ${websiteUrl}`);
        window.open(websiteUrl, '_blank');
    }

    async downloadRecord(id, downloadType = 'translated') {
        try {
            // 使用V2 API的下载接口，支持多种下载类型
            const params = new URLSearchParams({ 
                content_type: downloadType,
                inline: false 
            });
            const response = await fetch(`/api/v2/library/${id}/download?${params}`);
            
            if (!response.ok) {
                const error = await response.json();
                throw new Error(error.message || '下载失败');
            }
            
            const blob = await response.blob();
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            
            // 根据下载类型设置文件名
            const extension = downloadType === 'both' ? 'zip' : 'html';
            a.download = `translation-${id}-${downloadType}.${extension}`;
            a.click();
            URL.revokeObjectURL(url);
            
            this.showSuccess('文件下载已开始');
        } catch (error) {
            this.showError('下载失败: ' + error.message);
        }
    }

    async deleteRecord(id) {
        if (!confirm('确定要删除这条记录吗？此操作不可撤销。')) {
            return;
        }
        
        try {
            // 使用V2 API的删除接口
            const response = await fetch(`/api/v2/library/${id}`, { 
                method: 'DELETE',
                headers: {
                    'Content-Type': 'application/json'
                }
            });
            
            if (!response.ok) {
                const error = await response.json();
                throw new Error(error.message || '删除失败');
            }
            
            const result = await response.json();
            this.loadData();
            this.updateStats();
            
            // 显示删除结果，包括释放的存储空间
            const freedSpace = this.formatFileSize(result.freed_bytes || 0);
            this.showSuccess(`记录已删除，释放存储空间: ${freedSpace}`);
            
        } catch (error) {
            this.showError('删除失败: ' + error.message);
        }
    }

    // 批量操作
    async deleteSelected() {
        if (this.selectedItems.size === 0) {
            this.showError('请先选择要删除的记录');
            return;
        }
        
        if (!confirm(`确定要删除选中的 ${this.selectedItems.size} 条记录吗？此操作不可撤销。`)) {
            return;
        }
        
        try {
            // 使用V2 API的批量删除接口
            const response = await this.apiCall('/api/v2/library/batch-delete', {
                method: 'DELETE',
                body: JSON.stringify({ ids: Array.from(this.selectedItems) })
            });
            
            this.selectedItems.clear();
            this.loadData();
            this.updateStats();
            
            // 显示删除结果，包括释放的存储空间
            const freedSpace = this.formatFileSize(response.freed_bytes || 0);
            this.showSuccess(`已删除 ${response.deleted_count || this.selectedItems.size} 条记录，释放存储空间: ${freedSpace}`);
        } catch (error) {
            this.showError('批量删除失败: ' + error.message);
        }
    }

    async exportSelected() {
        if (this.selectedItems.size === 0) {
            this.showError('请先选择要导出的记录');
            return;
        }
        
        try {
            // 使用V2 API的批量导出接口
            const response = await fetch('/api/v2/library/export', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ 
                    ids: Array.from(this.selectedItems),
                    format: 'zip',
                    include_originals: true 
                })
            });
            
            if (!response.ok) {
                const error = await response.json();
                throw new Error(error.message || '导出失败');
            }
            
            const blob = await response.blob();
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `library-export-${new Date().toISOString().split('T')[0]}.zip`;
            a.click();
            URL.revokeObjectURL(url);
            
            this.showSuccess(`已导出 ${this.selectedItems.size} 条记录`);
        } catch (error) {
            this.showError('导出失败: ' + error.message);
        }
    }
}