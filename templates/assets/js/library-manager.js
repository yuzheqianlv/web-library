/**
 * Library Manager - 主管理类
 */

class LibraryManager extends LibraryUI {
    constructor() {
        super();
        this.init();
    }

    // 初始化
    async init() {
        this.initElements();
        this.bindEvents();
        this.setupGlobalFunctions();
        await this.loadData();
        await this.updateStats();
    }

    // 设置全局函数供HTML onclick使用
    setupGlobalFunctions() {
        window.viewRecord = (id, url) => this.viewRecord(id, url);
        window.deleteRecord = (id) => this.deleteRecord(id);
    }

}

// 全局实例
let libraryManager;

// 注意：初始化现在由主页面的组件加载器控制
// 不再在这里自动初始化