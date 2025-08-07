// Monolith 应用管理器

/**
 * 应用管理器类
 */
class MonolithApp {
    constructor() {
        this.currentMode = 'translated';
        this.isProcessing = false;
        this.initializeElements();
        this.bindEvents();
        this.loadSettings();
    }

    /**
     * 初始化DOM元素引用
     */
    initializeElements() {
        this.elements = {
            urlInput: document.getElementById('url-input'),
            translateBtn: document.getElementById('translate-btn'),
            modeButtons: document.querySelectorAll('.mode-btn'),
            contentFrames: {
                translated: document.getElementById('translated-frame'),
                original: document.getElementById('original-frame'),
                bilingual: document.getElementById('bilingual-container')
            },
            emptyState: document.getElementById('empty-state'),
            loading: document.getElementById('loading'),
            errorToast: document.getElementById('error-toast'),
            successToast: document.getElementById('success-toast')
            // 注意：导航栏切换相关元素已移至 NavToggleComponent 管理
        };
    }

    /**
     * 绑定事件监听器
     */
    bindEvents() {
        // URL输入框回车事件
        this.elements.urlInput?.addEventListener('keypress', (e) => {
            if (e.key === 'Enter') {
                this.processUrl();
            }
        });

        // 翻译按钮点击事件
        this.elements.translateBtn?.addEventListener('click', () => {
            this.processUrl();
        });

        // 模式切换按钮事件
        this.elements.modeButtons?.forEach(btn => {
            btn.addEventListener('click', () => {
                this.switchMode(btn.dataset.mode);
            });
        });

        // 注意：导航栏切换事件现在由 NavToggleComponent 处理

        // 页面加载时检查URL参数
        this.checkUrlParams();
    }

    /**
     * 检查URL参数自动处理
     */
    checkUrlParams() {
        const urlParams = new URLSearchParams(window.location.search);
        const autoUrl = urlParams.get('url');
        if (autoUrl && this.elements.urlInput) {
            this.elements.urlInput.value = autoUrl;
            this.processUrl();
        }
    }

    /**
     * 处理URL
     */
    async processUrl() {
        const url = this.elements.urlInput?.value?.trim();
        if (!url) {
            this.showError('请输入要处理的网页URL');
            return;
        }

        if (this.isProcessing) {
            return;
        }

        this.setProcessing(true);
        this.hideContent();
        this.showLoading();

        try {
            const data = {
                url: url,
                translate: true,
                target_lang: 'zh',
                options: {
                    no_css: false,
                    no_js: true, // 默认禁用JS以提高安全性
                    no_images: false,
                    no_fonts: false,
                    timeout: 30
                }
            };

            const result = await window.monolithAPI.process(data);

            if (result.status === 'success') {
                this.displayContent(result);
                this.showSuccess('网页处理完成');
            } else {
                throw new Error(result.error || '处理失败');
            }
        } catch (error) {
            console.error('处理失败:', error);
            this.showError(`处理失败: ${error.message}`);
        } finally {
            this.setProcessing(false);
            this.hideLoading();
        }
    }

    /**
     * 显示处理结果
     */
    displayContent(result) {
        // 隐藏空状态
        this.elements.emptyState?.classList.add('hidden');

        // 更新原文iframe
        if (this.elements.contentFrames.original && result.original_html) {
            this.elements.contentFrames.original.srcdoc = result.original_html;
        }

        // 更新译文iframe
        if (this.elements.contentFrames.translated && result.translated_html) {
            this.elements.contentFrames.translated.srcdoc = result.translated_html;
        } else if (this.elements.contentFrames.translated && result.original_html) {
            // 如果没有译文，显示原文
            this.elements.contentFrames.translated.srcdoc = result.original_html;
        }

        // 更新双语对照
        if (this.elements.contentFrames.bilingual) {
            const bilingualTranslated = document.getElementById('bilingual-translated');
            const bilingualOriginal = document.getElementById('bilingual-original');
            
            if (bilingualTranslated && result.translated_html) {
                bilingualTranslated.srcdoc = result.translated_html;
            } else if (bilingualTranslated && result.original_html) {
                bilingualTranslated.srcdoc = result.original_html;
            }
            
            if (bilingualOriginal && result.original_html) {
                bilingualOriginal.srcdoc = result.original_html;
            }
        }

        // 显示当前模式的内容
        this.showCurrentMode();
    }

    /**
     * 切换显示模式
     */
    switchMode(mode) {
        this.currentMode = mode;
        
        // 更新按钮状态
        this.elements.modeButtons?.forEach(btn => {
            btn.classList.toggle('active', btn.dataset.mode === mode);
        });

        // 显示对应内容
        this.showCurrentMode();
    }

    /**
     * 显示当前模式的内容
     */
    showCurrentMode() {
        // 隐藏所有内容
        Object.values(this.elements.contentFrames).forEach(frame => {
            if (frame) {
                frame.classList.remove('active');
                frame.style.display = 'none';
            }
        });

        // 显示当前模式的内容
        const currentFrame = this.elements.contentFrames[this.currentMode];
        if (currentFrame) {
            currentFrame.classList.add('active');
            currentFrame.style.display = this.currentMode === 'bilingual' ? 'flex' : 'block';
        }
    }

    /**
     * 注意：导航栏切换功能已移至 NavToggleComponent
     * 如需访问导航栏切换功能，请使用 window.navToggleComponent
     */

    /**
     * 设置处理状态
     */
    setProcessing(processing) {
        this.isProcessing = processing;
        if (this.elements.translateBtn) {
            this.elements.translateBtn.disabled = processing;
            this.elements.translateBtn.textContent = processing ? '处理中...' : '翻译';
        }
    }

    /**
     * 显示/隐藏内容
     */
    hideContent() {
        Object.values(this.elements.contentFrames).forEach(frame => {
            if (frame) {
                frame.style.display = 'none';
            }
        });
    }

    showLoading() {
        this.elements.loading?.classList.add('active');
        this.elements.emptyState?.classList.add('hidden');
    }

    hideLoading() {
        this.elements.loading?.classList.remove('active');
    }

    /**
     * 显示消息
     */
    showError(message) {
        this.showToast(this.elements.errorToast, message);
    }

    showSuccess(message) {
        this.showToast(this.elements.successToast, message);
    }

    showToast(element, message) {
        if (!element) return;
        
        element.textContent = message;
        element.classList.add('show');
        
        setTimeout(() => {
            element.classList.remove('show');
        }, 3000);
    }

    /**
     * 加载设置
     */
    loadSettings() {
        try {
            const savedTheme = localStorage.getItem('monolith-theme') || 'light';
            document.body.className = `theme-${savedTheme}`;
            
            const themeSelect = document.getElementById('theme-select');
            if (themeSelect) {
                themeSelect.value = savedTheme;
            }
        } catch (error) {
            console.warn('加载设置失败:', error);
        }
    }
}

// 当DOM加载完成时初始化应用
document.addEventListener('DOMContentLoaded', () => {
    window.monolithApp = new MonolithApp();
    console.log('🚀 Monolith 应用已初始化');
});