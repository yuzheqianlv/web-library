/**
 * Library UI - 用户界面交互模块
 */

class LibraryUI extends LibraryData {
    constructor() {
        super();
    }

    // 初始化界面元素
    initElements() {
        console.log('正在初始化UI元素...');
        
        // 导航栏相关
        this.navbar = document.getElementById('navbar');
        this.toggleNavBtn = document.getElementById('toggle-nav-btn');
        this.floatingToggle = document.getElementById('floating-toggle');
        
        // 搜索和筛选
        this.searchInput = document.getElementById('search-input');
        this.statusFilter = document.getElementById('status-filter');
        this.languageFilter = document.getElementById('language-filter');
        
        // 表格相关
        this.dataTable = document.getElementById('data-table');
        this.tableBody = document.getElementById('table-body');
        this.selectAllCheckbox = document.getElementById('select-all');
        this.selectedCountSpan = document.getElementById('selected-count');
        this.emptyState = document.getElementById('empty-state');
        
        // 按钮
        this.refreshBtn = document.getElementById('refresh-btn');
        this.exportBtn = document.getElementById('export-btn');
        this.deleteSelectedBtn = document.getElementById('delete-selected-btn');
        
        // 分页
        this.paginationInfo = document.getElementById('pagination-info');
        this.pageNumbers = document.getElementById('page-numbers');
        this.prevPageBtn = document.getElementById('prev-page');
        this.nextPageBtn = document.getElementById('next-page');
        
        // 统计数据
        this.totalCountSpan = document.getElementById('total-count');
        this.successCountSpan = document.getElementById('success-count');
        this.todayCountSpan = document.getElementById('today-count');
        this.storageSizeSpan = document.getElementById('storage-size');
    }

    // 绑定事件
    bindEvents() {
        // 导航栏事件
        this.toggleNavBtn?.addEventListener('click', () => this.toggleNavbar());
        this.floatingToggle?.addEventListener('click', () => this.toggleNavbar());
        
        // 搜索和筛选事件
        this.searchInput?.addEventListener('input', this.debounce(() => {
            this.filters.search = this.searchInput.value.trim();
            this.currentPage = 1;
            this.loadData();
        }, 500));
        
        this.statusFilter?.addEventListener('change', () => {
            this.filters.status = this.statusFilter.value;
            this.currentPage = 1;
            this.loadData();
        });
        
        this.languageFilter?.addEventListener('change', () => {
            this.filters.language = this.languageFilter.value;
            this.currentPage = 1;
            this.loadData();
        });
        
        // 表格事件
        this.selectAllCheckbox?.addEventListener('change', () => this.toggleSelectAll());
        
        // 排序事件 - 使用事件委托
        this.dataTable?.addEventListener('click', (e) => {
            if (e.target.classList.contains('sortable') || e.target.closest('.sortable')) {
                const column = e.target.dataset.column || e.target.closest('.sortable').dataset.column;
                this.handleSort(column);
            }
        });
        
        // 按钮事件
        this.refreshBtn?.addEventListener('click', () => {
            this.loadData();
            this.updateStats();
        });
        this.exportBtn?.addEventListener('click', () => this.exportSelected());
        this.deleteSelectedBtn?.addEventListener('click', () => this.deleteSelected());
        
        // 分页事件
        this.prevPageBtn?.addEventListener('click', () => this.changePage(this.currentPage - 1));
        this.nextPageBtn?.addEventListener('click', () => this.changePage(this.currentPage + 1));
        
        // 页面大小选择事件
        const pageSizeSelect = document.getElementById('page-size');
        pageSizeSelect?.addEventListener('change', () => {
            this.pageSize = parseInt(pageSizeSelect.value);
            this.currentPage = 1;
            this.loadData();
        });
    }

    // 导航栏切换
    toggleNavbar() {
        const navbar = this.navbar;
        const floatingToggle = this.floatingToggle;
        
        navbar.classList.toggle('hidden');
        floatingToggle.classList.toggle('show');
    }

    // 选择管理
    updateSelection() {
        const checkboxes = this.tableBody?.querySelectorAll('.row-checkbox') || [];
        this.selectedItems.clear();
        
        checkboxes.forEach(checkbox => {
            if (checkbox.checked) {
                this.selectedItems.add(checkbox.value);
            }
        });
        
        if (this.selectedCountSpan) {
            this.selectedCountSpan.textContent = `已选择 ${this.selectedItems.size} 项`;
        }
        
        if (this.selectAllCheckbox) {
            this.selectAllCheckbox.checked = checkboxes.length > 0 && this.selectedItems.size === checkboxes.length;
            this.selectAllCheckbox.indeterminate = this.selectedItems.size > 0 && this.selectedItems.size < checkboxes.length;
        }
        
        // 更新操作按钮状态
        const hasSelection = this.selectedItems.size > 0;
        if (this.deleteSelectedBtn) this.deleteSelectedBtn.disabled = !hasSelection;
        if (this.exportBtn) this.exportBtn.disabled = !hasSelection;
    }

    toggleSelectAll() {
        const checkboxes = this.tableBody?.querySelectorAll('.row-checkbox') || [];
        const shouldCheck = this.selectAllCheckbox?.checked || false;
        
        checkboxes.forEach(checkbox => {
            checkbox.checked = shouldCheck;
        });
        
        this.updateSelection();
    }

    // 排序处理
    handleSort(column) {
        if (this.currentSort.column === column) {
            this.currentSort.direction = this.currentSort.direction === 'asc' ? 'desc' : 'asc';
        } else {
            this.currentSort = { column, direction: 'asc' };
        }
        
        // 更新排序图标
        this.dataTable.querySelectorAll('.sortable').forEach(header => {
            header.classList.remove('sort-asc', 'sort-desc');
            if (header.dataset.column === column) {
                header.classList.add(`sort-${this.currentSort.direction}`);
            }
        });
        
        this.currentPage = 1;
        this.loadData();
    }

    // 分页相关
    changePage(page) {
        const maxPage = Math.ceil(this.totalRecords / this.pageSize);
        if (page < 1 || page > maxPage) {
            return;
        }
        this.currentPage = page;
        this.loadData();
    }

    updatePagination(pagination) {
        if (!pagination) return;
        
        const { current_page, total_pages, total_records, page_size } = pagination;
        
        // 更新分页信息
        const start = (current_page - 1) * page_size + 1;
        const end = Math.min(current_page * page_size, total_records);
        if (this.paginationInfo) {
            this.paginationInfo.textContent = `显示第 ${start}-${end} 条，共 ${total_records} 条记录`;
        }
        
        // 更新按钮状态
        if (this.prevPageBtn) this.prevPageBtn.disabled = current_page <= 1;
        if (this.nextPageBtn) this.nextPageBtn.disabled = current_page >= total_pages;
        
        // 生成页码按钮
        this.generatePageNumbers(current_page, total_pages);
    }

    generatePageNumbers(currentPage, totalPages) {
        if (!this.pageNumbers || totalPages <= 1) {
            return;
        }
        
        this.pageNumbers.innerHTML = '';
        
        const showPages = 5; // 显示的页码数量
        let startPage = Math.max(1, currentPage - Math.floor(showPages / 2));
        let endPage = Math.min(totalPages, startPage + showPages - 1);
        
        if (endPage - startPage < showPages - 1) {
            startPage = Math.max(1, endPage - showPages + 1);
        }
        
        // 如果不是从第1页开始，添加首页和省略号
        if (startPage > 1) {
            this.addPageButton(1, currentPage);
            if (startPage > 2) {
                this.addEllipsis();
            }
        }
        
        // 添加页码按钮
        for (let i = startPage; i <= endPage; i++) {
            this.addPageButton(i, currentPage);
        }
        
        // 如果不是到最后一页结束，添加省略号和末页
        if (endPage < totalPages) {
            if (endPage < totalPages - 1) {
                this.addEllipsis();
            }
            this.addPageButton(totalPages, currentPage);
        }
    }
    
    addPageButton(pageNum, currentPage) {
        const pageBtn = document.createElement('button');
        pageBtn.className = `page-btn ${pageNum === currentPage ? 'active' : ''}`;
        pageBtn.textContent = pageNum;
        pageBtn.addEventListener('click', () => this.changePage(pageNum));
        this.pageNumbers.appendChild(pageBtn);
    }
    
    addEllipsis() {
        const ellipsis = document.createElement('span');
        ellipsis.className = 'page-ellipsis';
        ellipsis.textContent = '...';
        this.pageNumbers.appendChild(ellipsis);
    }
}