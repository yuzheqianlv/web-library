/**
 * Monolith 网页翻译器 - 库管理组件
 * 整合自 library-core.js, library-ui.js - 提供完整的翻译库管理功能
 */

class LibraryManager {
    constructor() {
        this.apiManager = null;
        this.elements = {};
        this.state = {
            currentPage: 1,
            pageSize: 20,
            totalRecords: 0,
            selectedRecords: new Set(),
            sortField: 'created_at',
            sortOrder: 'desc',
            filters: {
                search: '',
                status: '',
                language: ''
            }
        };
        
        this.loadingStates = new Set();
    }

    /**
     * 初始化库管理器
     */
    async init() {
        try {
            console.log('初始化库管理器...');
            
            // 获取API管理器
            this.apiManager = window.appManager?.getComponent('api') || new window.ApiManager();
            
            // 初始化DOM元素
            this.initElements();
            
            // 绑定事件
            this.bindEvents();
            
            // 加载初始数据
            await this.loadData();
            await this.updateStats();
            
            console.log('✅ 库管理器初始化完成');
        } catch (error) {
            console.error('❌ 库管理器初始化失败:', error);
            this.showError('库管理器初始化失败');
        }
    }

    /**
     * 初始化DOM元素引用
     */
    initElements() {
        // 表格相关元素
        this.elements.tableBody = document.getElementById('table-body');
        this.elements.emptyState = document.getElementById('empty-state');
        this.elements.loadingState = document.getElementById('loading-state');
        
        // 统计元素
        this.elements.totalCountSpan = document.getElementById('total-count');
        this.elements.successCountSpan = document.getElementById('success-count');
        this.elements.todayCountSpan = document.getElementById('today-count');
        this.elements.storageSizeSpan = document.getElementById('storage-size');
        
        // 搜索和筛选元素
        this.elements.searchInput = document.getElementById('search-input');
        this.elements.statusFilter = document.getElementById('status-filter');
        this.elements.languageFilter = document.getElementById('language-filter');
        
        // 分页元素
        this.elements.paginationInfo = document.getElementById('pagination-info');
        this.elements.pageNumbers = document.getElementById('page-numbers');
        this.elements.prevPageBtn = document.getElementById('prev-page');
        this.elements.nextPageBtn = document.getElementById('next-page');
        this.elements.pageSize = document.getElementById('page-size');
        
        // 批量操作元素
        this.elements.selectAllCheckbox = document.getElementById('select-all');
        this.elements.exportBtn = document.getElementById('export-btn');
        this.elements.deleteSelectedBtn = document.getElementById('delete-selected-btn');
        this.elements.refreshBtn = document.getElementById('refresh-btn');
        
        console.log('DOM元素初始化完成');
    }

    /**
     * 绑定事件监听器
     */
    bindEvents() {
        // 搜索和筛选事件
        if (this.elements.searchInput) {
            const debouncedSearch = this.debounce(() => {
                this.state.filters.search = this.elements.searchInput.value.trim();
                this.state.currentPage = 1;
                this.loadData();
            }, 500);
            
            this.elements.searchInput.addEventListener('input', debouncedSearch);
        }

        if (this.elements.statusFilter) {
            this.elements.statusFilter.addEventListener('change', () => {
                this.state.filters.status = this.elements.statusFilter.value;
                this.state.currentPage = 1;
                this.loadData();
            });
        }

        if (this.elements.languageFilter) {
            this.elements.languageFilter.addEventListener('change', () => {
                this.state.filters.language = this.elements.languageFilter.value;
                this.state.currentPage = 1;
                this.loadData();
            });
        }

        // 分页事件
        if (this.elements.pageSize) {
            this.elements.pageSize.addEventListener('change', () => {
                this.state.pageSize = parseInt(this.elements.pageSize.value);
                this.state.currentPage = 1;
                this.loadData();
            });
        }

        if (this.elements.prevPageBtn) {
            this.elements.prevPageBtn.addEventListener('click', () => {
                if (this.state.currentPage > 1) {
                    this.state.currentPage--;
                    this.loadData();
                }
            });
        }

        if (this.elements.nextPageBtn) {
            this.elements.nextPageBtn.addEventListener('click', () => {
                const maxPage = Math.ceil(this.state.totalRecords / this.state.pageSize);
                if (this.state.currentPage < maxPage) {
                    this.state.currentPage++;
                    this.loadData();
                }
            });
        }

        // 批量操作事件
        if (this.elements.selectAllCheckbox) {
            this.elements.selectAllCheckbox.addEventListener('change', (e) => {
                this.handleSelectAll(e.target.checked);
            });
        }

        if (this.elements.exportBtn) {
            this.elements.exportBtn.addEventListener('click', () => {
                this.handleExport();
            });
        }

        if (this.elements.deleteSelectedBtn) {
            this.elements.deleteSelectedBtn.addEventListener('click', () => {
                this.handleBatchDelete();
            });
        }

        if (this.elements.refreshBtn) {
            this.elements.refreshBtn.addEventListener('click', () => {
                this.handleRefresh();
            });
        }

        // 表格排序事件
        document.addEventListener('click', (e) => {
            if (e.target.classList.contains('sortable')) {
                this.handleSort(e.target.dataset.field);
            }
        });

        console.log('事件绑定完成');
    }

    /**
     * 加载数据
     */
    async loadData() {
        if (this.loadingStates.has('data')) return;
        
        try {
            this.loadingStates.add('data');
            this.showLoading();

            const params = {
                page: this.state.currentPage,
                limit: this.state.pageSize,
                sort_field: this.state.sortField,
                sort_order: this.state.sortOrder,
                ...this.state.filters
            };

            const response = await this.apiManager.loadLibraryData(params);
            
            this.renderTable(response.records || []);
            this.updatePagination(response.pagination || {});
            this.state.totalRecords = response.pagination?.total || 0;

        } catch (error) {
            console.error('加载数据失败:', error);
            this.showError('加载数据失败，请稍后重试');
            this.renderTable([]);
        } finally {
            this.loadingStates.delete('data');
            this.hideLoading();
        }
    }

    /**
     * 更新统计数据
     */
    async updateStats() {
        if (this.loadingStates.has('stats')) return;
        
        try {
            this.loadingStates.add('stats');
            
            const stats = await this.apiManager.getStats();
            
            if (this.elements.totalCountSpan) {
                this.elements.totalCountSpan.textContent = stats.today?.total || 0;
            }
            if (this.elements.successCountSpan) {
                this.elements.successCountSpan.textContent = stats.today?.success || 0;
            }
            if (this.elements.todayCountSpan) {
                this.elements.todayCountSpan.textContent = stats.today?.total || 0;
            }
            if (this.elements.storageSizeSpan) {
                this.elements.storageSizeSpan.textContent = this.apiManager.formatFileSize(stats.today?.storage_size || 0);
            }

            // 计算成功率
            const successRate = stats.today?.total > 0 ? 
                ((stats.today.success / stats.today.total) * 100).toFixed(1) : '0.0';
            console.log(`翻译成功率: ${successRate}%`);

        } catch (error) {
            console.error('更新统计数据失败:', error);
        } finally {
            this.loadingStates.delete('stats');
        }
    }

    /**
     * 渲染表格
     * @param {Array} records - 记录数组
     */
    renderTable(records) {
        if (!this.elements.tableBody) return;

        if (!records || records.length === 0) {
            this.elements.tableBody.innerHTML = '';
            if (this.elements.emptyState) {
                this.elements.emptyState.style.display = 'block';
            }
            return;
        }

        if (this.elements.emptyState) {
            this.elements.emptyState.style.display = 'none';
        }

        this.elements.tableBody.innerHTML = records.map(record => this.renderTableRow(record)).join('');
        
        // 绑定行内事件
        this.bindTableRowEvents();
    }

    /**
     * 渲染表格行
     * @param {Object} record - 记录对象
     * @returns {string} HTML字符串
     */
    renderTableRow(record) {
        const statusBadge = this.getStatusBadge(record);
        const languageTag = this.getLanguageTag(record.source_language);
        const htmlPreview = this.getHtmlPreview(record.translated_html);
        
        return `
            <tr data-id="${record.id}" class="mongo-row ${this.state.selectedRecords.has(record.id) ? 'selected' : ''}">
                <td class="fixed-left checkbox-col" data-label="选择">
                    <input type="checkbox" class="mongo-checkbox row-checkbox" 
                           value="${record.id}" ${this.state.selectedRecords.has(record.id) ? 'checked' : ''}>
                </td>
                <td class="id-col" data-label="ID">
                    <span class="object-id" title="${record.id}">
                        ObjectId('${record.id.substring(0, 8)}...')
                    </span>
                </td>
                <td class="url-col" data-label="URL">
                    <div class="url-cell">
                        <a href="${record.original_url}" target="_blank" class="url-link" 
                           title="${record.original_url}">
                            ${this.apiManager.truncateText(record.original_url, 40)}
                        </a>
                    </div>
                </td>
                <td class="title-col" data-label="标题">
                    ${this.apiManager.truncateText(record.title || '无标题', 30)}
                </td>
                <td class="html-col" data-label="翻译内容">
                    ${htmlPreview}
                </td>
                <td class="lang-col" data-label="语言">
                    ${languageTag}
                </td>
                <td class="date-col" data-label="创建时间">
                    ${this.apiManager.formatDate(record.created_at)}
                </td>
                <td class="date-col" data-label="更新时间">
                    ${this.apiManager.formatDate(record.updated_at)}
                </td>
                <td class="fixed-right actions-col" data-label="操作">
                    <div class="record-actions">
                        <button class="action-icon-btn view" onclick="viewRecord('${record.id}', '${record.original_url}')" 
                                title="查看">
                            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                                <path d="M12 4.5C7.305 4.5 3.306 7.479 1.5 12c1.806 4.521 5.805 7.5 10.5 7.5s8.694-2.979 10.5-7.5c-1.806-4.521-5.805-7.5-10.5-7.5zM12 17c-2.761 0-5-2.239-5-5s2.239-5 5-5 5 2.239 5 5-2.239 5-5 5zm0-8c-1.657 0-3 1.343-3 3s1.343 3 3 3 3-1.343 3-3-1.343-3-3-3z"/>
                            </svg>
                        </button>
                        <button class="action-icon-btn delete" onclick="deleteRecord('${record.id}')" 
                                title="删除">
                            <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
                                <path d="M19 4h-3.5l-1-1h-5l-1 1H5v2h14M6 19a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V7H6v12z"/>
                            </svg>
                        </button>
                    </div>
                </td>
            </tr>
        `;
    }

    /**
     * 获取状态徽章
     * @param {Object} record - 记录对象
     * @returns {string} 状态徽章HTML
     */
    getStatusBadge(record) {
        if (record.translated_html && record.translated_html.trim()) {
            return '<span class="status-badge status-success">成功</span>';
        } else if (record.error_message) {
            return '<span class="status-badge status-error">失败</span>';
        } else {
            return '<span class="status-badge status-pending">处理中</span>';
        }
    }

    /**
     * 获取语言标签
     * @param {string} language - 语言代码
     * @returns {string} 语言标签HTML
     */
    getLanguageTag(language) {
        const languageMap = {
            'en': '英语',
            'ja': '日语',
            'ko': '韩语',
            'fr': '法语',
            'de': '德语',
            'es': '西班牙语',
            'zh': '中文'
        };
        
        const displayName = languageMap[language] || language || '未知';
        return `<span class="language-tag">${displayName}</span>`;
    }

    /**
     * 获取HTML预览
     * @param {string} html - HTML内容
     * @returns {string} 预览HTML
     */
    getHtmlPreview(html) {
        if (!html || !html.trim()) {
            return '<span class="html-preview empty">暂无翻译</span>';
        }
        
        const preview = this.apiManager.truncateText(html, 50);
        return `<span class="html-preview" title="点击查看完整内容">${preview}</span>`;
    }

    /**
     * 绑定表格行事件
     */
    bindTableRowEvents() {
        // 行选择事件
        const checkboxes = this.elements.tableBody.querySelectorAll('.row-checkbox');
        checkboxes.forEach(checkbox => {
            checkbox.addEventListener('change', (e) => {
                const recordId = e.target.value;
                if (e.target.checked) {
                    this.state.selectedRecords.add(recordId);
                    e.target.closest('tr').classList.add('selected');
                } else {
                    this.state.selectedRecords.delete(recordId);
                    e.target.closest('tr').classList.remove('selected');
                }
                
                this.updateBatchActionsState();
            });
        });

        // HTML预览点击事件
        const htmlPreviews = this.elements.tableBody.querySelectorAll('.html-preview:not(.empty)');
        htmlPreviews.forEach(preview => {
            preview.addEventListener('click', () => {
                this.showHtmlPreview(preview.title || preview.textContent);
            });
        });
    }

    /**
     * 更新分页信息
     * @param {Object} pagination - 分页数据
     */
    updatePagination(pagination) {
        const { page = 1, limit = 20, total = 0 } = pagination;
        const totalPages = Math.ceil(total / limit);
        const startRecord = total > 0 ? (page - 1) * limit + 1 : 0;
        const endRecord = Math.min(page * limit, total);

        // 更新分页信息
        if (this.elements.paginationInfo) {
            this.elements.paginationInfo.textContent = `显示第 ${startRecord}-${endRecord} 条，共 ${total} 条记录`;
        }

        // 更新分页控件状态
        if (this.elements.prevPageBtn) {
            this.elements.prevPageBtn.disabled = page <= 1;
        }
        if (this.elements.nextPageBtn) {
            this.elements.nextPageBtn.disabled = page >= totalPages;
        }

        // 生成页码按钮
        this.generatePageNumbers(page, totalPages);
    }

    /**
     * 生成页码按钮
     * @param {number} currentPage - 当前页码
     * @param {number} totalPages - 总页数
     */
    generatePageNumbers(currentPage, totalPages) {
        if (!this.elements.pageNumbers) return;

        let html = '';
        const maxVisiblePages = 7;
        let startPage = Math.max(1, currentPage - Math.floor(maxVisiblePages / 2));
        let endPage = Math.min(totalPages, startPage + maxVisiblePages - 1);

        // 调整起始页
        if (endPage - startPage + 1 < maxVisiblePages) {
            startPage = Math.max(1, endPage - maxVisiblePages + 1);
        }

        // 第一页
        if (startPage > 1) {
            html += `<button class="page-btn" onclick="libraryManager.goToPage(1)">1</button>`;
            if (startPage > 2) {
                html += `<span class="page-ellipsis">...</span>`;
            }
        }

        // 页码范围
        for (let i = startPage; i <= endPage; i++) {
            const activeClass = i === currentPage ? 'active' : '';
            html += `<button class="page-btn ${activeClass}" onclick="libraryManager.goToPage(${i})">${i}</button>`;
        }

        // 最后一页
        if (endPage < totalPages) {
            if (endPage < totalPages - 1) {
                html += `<span class="page-ellipsis">...</span>`;
            }
            html += `<button class="page-btn" onclick="libraryManager.goToPage(${totalPages})">${totalPages}</button>`;
        }

        this.elements.pageNumbers.innerHTML = html;
    }

    // ==============================================
    // 事件处理器
    // ==============================================

    handleSelectAll(checked) {
        const checkboxes = this.elements.tableBody.querySelectorAll('.row-checkbox');
        checkboxes.forEach(checkbox => {
            checkbox.checked = checked;
            const recordId = checkbox.value;
            const row = checkbox.closest('tr');
            
            if (checked) {
                this.state.selectedRecords.add(recordId);
                row.classList.add('selected');
            } else {
                this.state.selectedRecords.delete(recordId);
                row.classList.remove('selected');
            }
        });
        
        this.updateBatchActionsState();
    }

    async handleExport() {
        try {
            const selectedIds = Array.from(this.state.selectedRecords);
            const format = 'json'; // 可以扩展为用户选择
            
            this.showLoading('导出中...');
            const blob = await this.apiManager.exportRecords(selectedIds.length > 0 ? selectedIds : null, format);
            
            // 下载文件
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `monolith-export-${new Date().toISOString().slice(0, 10)}.${format}`;
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            URL.revokeObjectURL(url);
            
            this.showSuccess('导出完成');
        } catch (error) {
            console.error('导出失败:', error);
            this.showError('导出失败');
        } finally {
            this.hideLoading();
        }
    }

    async handleBatchDelete() {
        const selectedIds = Array.from(this.state.selectedRecords);
        if (selectedIds.length === 0) {
            this.showError('请先选择要删除的记录');
            return;
        }

        const confirmed = confirm(`确定要删除选中的 ${selectedIds.length} 条记录吗？此操作不可恢复。`);
        if (!confirmed) return;

        try {
            this.showLoading('删除中...');
            await this.apiManager.batchDeleteRecords(selectedIds);
            
            this.state.selectedRecords.clear();
            this.updateBatchActionsState();
            
            await this.loadData();
            await this.updateStats();
            
            this.showSuccess(`成功删除 ${selectedIds.length} 条记录`);
        } catch (error) {
            console.error('批量删除失败:', error);
            this.showError('批量删除失败');
        } finally {
            this.hideLoading();
        }
    }

    async handleRefresh() {
        try {
            await Promise.all([
                this.loadData(),
                this.updateStats()
            ]);
            this.showSuccess('数据刷新完成');
        } catch (error) {
            console.error('刷新失败:', error);
            this.showError('数据刷新失败');
        }
    }

    handleSort(field) {
        if (this.state.sortField === field) {
            this.state.sortOrder = this.state.sortOrder === 'asc' ? 'desc' : 'asc';
        } else {
            this.state.sortField = field;
            this.state.sortOrder = 'desc';
        }
        
        this.state.currentPage = 1;
        this.loadData();
    }

    goToPage(page) {
        this.state.currentPage = page;
        this.loadData();
    }

    // ==============================================
    // UI 辅助方法
    // ==============================================

    updateBatchActionsState() {
        const selectedCount = this.state.selectedRecords.size;
        
        if (this.elements.deleteSelectedBtn) {
            this.elements.deleteSelectedBtn.disabled = selectedCount === 0;
        }
        if (this.elements.exportBtn) {
            this.elements.exportBtn.textContent = selectedCount > 0 ? 
                `导出选中 (${selectedCount})` : '导出全部';
        }
        if (this.elements.selectAllCheckbox) {
            const totalCheckboxes = this.elements.tableBody.querySelectorAll('.row-checkbox').length;
            this.elements.selectAllCheckbox.checked = selectedCount > 0 && selectedCount === totalCheckboxes;
            this.elements.selectAllCheckbox.indeterminate = selectedCount > 0 && selectedCount < totalCheckboxes;
        }
    }

    showLoading(message = '加载中...') {
        if (this.elements.loadingState) {
            this.elements.loadingState.style.display = 'block';
            this.elements.loadingState.textContent = message;
        }
    }

    hideLoading() {
        if (this.elements.loadingState) {
            this.elements.loadingState.style.display = 'none';
        }
    }

    showError(message) {
        window.appManager?.showToast(message, 'error');
    }

    showSuccess(message) {
        window.appManager?.showToast(message, 'success');
    }

    showHtmlPreview(content) {
        // 创建模态框显示HTML内容
        const modal = document.createElement('div');
        modal.className = 'html-preview-modal';
        modal.innerHTML = `
            <div class="modal-backdrop">
                <div class="modal-content">
                    <div class="modal-header">
                        <h3>HTML 内容预览</h3>
                        <button class="modal-close">&times;</button>
                    </div>
                    <div class="modal-body">
                        <pre><code>${this.escapeHtml(content)}</code></pre>
                    </div>
                </div>
            </div>
        `;
        
        modal.style.cssText = `
            position: fixed; top: 0; left: 0; right: 0; bottom: 0;
            background: rgba(0,0,0,0.5); z-index: 10000;
            display: flex; align-items: center; justify-content: center;
        `;
        
        document.body.appendChild(modal);
        
        // 绑定关闭事件
        modal.querySelector('.modal-close').addEventListener('click', () => {
            modal.remove();
        });
        modal.addEventListener('click', (e) => {
            if (e.target === modal || e.target.classList.contains('modal-backdrop')) {
                modal.remove();
            }
        });
    }

    escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

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
     * 销毁库管理器
     */
    destroy() {
        this.state.selectedRecords.clear();
        this.loadingStates.clear();
        console.log('库管理器已销毁');
    }
}

// 全局实例
window.LibraryManager = LibraryManager;

// 创建全局实例（延迟初始化）
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => {
        if (window.appManager) {
            window.libraryManager = new LibraryManager();
        }
    });
} else {
    if (window.appManager) {
        window.libraryManager = new LibraryManager();
    }
}