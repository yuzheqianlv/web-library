// Monolith 翻译器组件

/**
 * 翻译器管理类
 */
class TranslatorManager {
    constructor() {
        this.supportedLanguages = {
            'zh': '中文',
            'en': 'English',
            'ja': '日本語',
            'ko': '한국어',
            'es': 'Español',
            'fr': 'Français',
            'de': 'Deutsch',
            'ru': 'Русский'
        };
        this.defaultTargetLang = 'zh';
        this.init();
    }

    /**
     * 初始化翻译器
     */
    init() {
        this.setupLanguageDetection();
        this.bindEvents();
    }

    /**
     * 绑定事件
     */
    bindEvents() {
        // 监听应用的处理事件
        document.addEventListener('beforeprocess', (e) => {
            this.onBeforeProcess(e.detail);
        });

        document.addEventListener('afterprocess', (e) => {
            this.onAfterProcess(e.detail);
        });
    }

    /**
     * 设置语言检测
     */
    setupLanguageDetection() {
        // 简单的语言检测逻辑
        this.detectedLanguage = this.detectBrowserLanguage();
    }

    /**
     * 检测浏览器语言
     * @returns {string} 语言代码
     */
    detectBrowserLanguage() {
        const lang = navigator.language || navigator.userLanguage || 'en';
        const langCode = lang.split('-')[0];
        return this.supportedLanguages[langCode] ? langCode : 'en';
    }

    /**
     * 检测页面内容语言
     * @param {string} text 页面文本内容
     * @returns {string} 检测到的语言代码
     */
    detectContentLanguage(text) {
        if (!text || text.length < 10) {
            return this.detectedLanguage;
        }

        // 简单的语言检测规则
        const chineseRegex = /[\u4e00-\u9fff]/;
        const japaneseRegex = /[\u3040-\u309f\u30a0-\u30ff]/;
        const koreanRegex = /[\uac00-\ud7af]/;
        const russianRegex = /[\u0400-\u04ff]/;

        if (chineseRegex.test(text)) {
            return 'zh';
        } else if (japaneseRegex.test(text)) {
            return 'ja';
        } else if (koreanRegex.test(text)) {
            return 'ko';
        } else if (russianRegex.test(text)) {
            return 'ru';
        }

        return 'en'; // 默认为英文
    }

    /**
     * 获取目标语言
     * @param {string} sourceLanguage 源语言
     * @returns {string} 目标语言
     */
    getTargetLanguage(sourceLanguage) {
        // 智能选择目标语言
        if (sourceLanguage === 'zh') {
            return this.detectedLanguage === 'zh' ? 'en' : this.detectedLanguage;
        } else {
            return this.defaultTargetLang;
        }
    }

    /**
     * 处理前事件
     * @param {Object} data 处理数据
     */
    onBeforeProcess(data) {
        // 可以在这里添加预处理逻辑
        console.log('翻译前处理:', data);
    }

    /**
     * 处理后事件
     * @param {Object} result 处理结果
     */
    onAfterProcess(result) {
        // 可以在这里添加后处理逻辑
        console.log('翻译后处理:', result);
        
        if (result.translated_html) {
            this.analyzeTranslationQuality(result);
        }
    }

    /**
     * 分析翻译质量
     * @param {Object} result 翻译结果
     */
    analyzeTranslationQuality(result) {
        // 简单的翻译质量分析
        const originalLength = result.original_html ? result.original_html.length : 0;
        const translatedLength = result.translated_html ? result.translated_html.length : 0;
        
        if (translatedLength === 0) {
            console.warn('翻译结果为空');
            return;
        }

        const lengthRatio = translatedLength / originalLength;
        
        if (lengthRatio < 0.3) {
            console.warn('翻译结果可能不完整');
        } else if (lengthRatio > 3.0) {
            console.warn('翻译结果可能包含错误');
        } else {
            console.log('翻译质量检查通过');
        }
    }

    /**
     * 创建语言选择器
     * @param {HTMLElement} container 容器元素
     */
    createLanguageSelector(container) {
        if (!container) return;

        const selector = document.createElement('select');
        selector.className = 'language-selector';
        selector.title = '选择目标语言';

        Object.entries(this.supportedLanguages).forEach(([code, name]) => {
            const option = document.createElement('option');
            option.value = code;
            option.textContent = name;
            if (code === this.defaultTargetLang) {
                option.selected = true;
            }
            selector.appendChild(option);
        });

        selector.addEventListener('change', (e) => {
            this.defaultTargetLang = e.target.value;
            this.saveSettings();
        });

        container.appendChild(selector);
        return selector;
    }

    /**
     * 保存设置
     */
    saveSettings() {
        try {
            const settings = {
                defaultTargetLang: this.defaultTargetLang,
                detectedLanguage: this.detectedLanguage
            };
            localStorage.setItem('monolith-translator-settings', JSON.stringify(settings));
        } catch (error) {
            console.warn('保存翻译设置失败:', error);
        }
    }

    /**
     * 加载设置
     */
    loadSettings() {
        try {
            const saved = localStorage.getItem('monolith-translator-settings');
            if (saved) {
                const settings = JSON.parse(saved);
                this.defaultTargetLang = settings.defaultTargetLang || this.defaultTargetLang;
                this.detectedLanguage = settings.detectedLanguage || this.detectedLanguage;
            }
        } catch (error) {
            console.warn('加载翻译设置失败:', error);
        }
    }

    /**
     * 获取支持的语言列表
     * @returns {Object} 语言映射
     */
    getSupportedLanguages() {
        return { ...this.supportedLanguages };
    }

    /**
     * 检查是否支持某种语言
     * @param {string} langCode 语言代码
     * @returns {boolean} 是否支持
     */
    isLanguageSupported(langCode) {
        return langCode in this.supportedLanguages;
    }
}

// 创建全局翻译器管理器实例
document.addEventListener('DOMContentLoaded', () => {
    window.translatorManager = new TranslatorManager();
});