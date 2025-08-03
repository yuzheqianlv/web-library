/**
 * Monolith智能链接拦截器兼容性测试套件
 * 
 * 功能：
 * - 跨浏览器兼容性测试
 * - 性能基准测试
 * - 功能完整性验证
 * - 错误处理测试
 * 
 * @version 1.0.0
 * @author Monolith Team
 */

(function() {
    'use strict';

    /**
     * 浏览器兼容性测试
     */
    class BrowserCompatibilityTester {
        constructor() {
            this.results = {
                browser: this.detectBrowser(),
                features: {},
                performance: {},
                errors: []
            };
        }

        /**
         * 运行所有兼容性测试
         */
        async runAllTests() {
            console.log('🧪 开始运行兼容性测试...');
            
            // 基础API兼容性测试
            this.testBasicAPIs();
            
            // ES6特性测试
            this.testES6Features();
            
            // DOM API测试
            this.testDOMAPIs();
            
            // 网络API测试
            await this.testNetworkAPIs();
            
            // 存储API测试
            this.testStorageAPIs();
            
            // 性能测试
            this.runPerformanceTests();
            
            // 生成报告
            this.generateReport();
            
            return this.results;
        }

        /**
         * 检测浏览器信息
         */
        detectBrowser() {
            const ua = navigator.userAgent;
            const browser = {
                name: 'Unknown',
                version: 'Unknown',
                engine: 'Unknown'
            };

            // Chrome
            if (ua.includes('Chrome')) {
                browser.name = 'Chrome';
                browser.version = ua.match(/Chrome\/(\d+)/)?.[1] || 'Unknown';
                browser.engine = 'Blink';
            }
            // Firefox
            else if (ua.includes('Firefox')) {
                browser.name = 'Firefox';
                browser.version = ua.match(/Firefox\/(\d+)/)?.[1] || 'Unknown';
                browser.engine = 'Gecko';
            }
            // Safari
            else if (ua.includes('Safari') && !ua.includes('Chrome')) {
                browser.name = 'Safari';
                browser.version = ua.match(/Version\/(\d+)/)?.[1] || 'Unknown';
                browser.engine = 'WebKit';
            }
            // Edge
            else if (ua.includes('Edg')) {
                browser.name = 'Edge';
                browser.version = ua.match(/Edg\/(\d+)/)?.[1] || 'Unknown';
                browser.engine = 'Blink';
            }
            // IE
            else if (ua.includes('Trident')) {
                browser.name = 'Internet Explorer';
                browser.version = ua.match(/rv:(\d+)/)?.[1] || 'Unknown';
                browser.engine = 'Trident';
            }

            return browser;
        }

        /**
         * 测试基础API
         */
        testBasicAPIs() {
            const tests = [
                ['JSON', () => typeof JSON !== 'undefined'],
                ['Promise', () => typeof Promise !== 'undefined'],
                ['fetch', () => typeof fetch !== 'undefined'],
                ['URL', () => typeof URL !== 'undefined'],
                ['URLSearchParams', () => typeof URLSearchParams !== 'undefined'],
                ['Object.assign', () => typeof Object.assign === 'function'],
                ['Array.from', () => typeof Array.from === 'function'],
                ['Map', () => typeof Map !== 'undefined'],
                ['Set', () => typeof Set !== 'undefined'],
                ['Symbol', () => typeof Symbol !== 'undefined']
            ];

            tests.forEach(([name, test]) => {
                try {
                    this.results.features[name] = test();
                } catch (e) {
                    this.results.features[name] = false;
                    this.results.errors.push(`${name}: ${e.message}`);
                }
            });
        }

        /**
         * 测试ES6特性
         */
        testES6Features() {
            const tests = [
                ['Arrow Functions', () => {
                    try {
                        eval('(() => true)()');
                        return true;
                    } catch (e) {
                        return false;
                    }
                }],
                ['Template Literals', () => {
                    try {
                        eval('`test ${1}`');
                        return true;
                    } catch (e) {
                        return false;
                    }
                }],
                ['Destructuring', () => {
                    try {
                        eval('const {a} = {a: 1}');
                        return true;
                    } catch (e) {
                        return false;
                    }
                }],
                ['Spread Operator', () => {
                    try {
                        eval('[...Array(1)]');
                        return true;
                    } catch (e) {
                        return false;
                    }
                }],
                ['Classes', () => {
                    try {
                        eval('class Test {}');
                        return true;
                    } catch (e) {
                        return false;
                    }
                }],
                ['Async/Await', () => {
                    try {
                        eval('async function test() { await Promise.resolve(); }');
                        return true;
                    } catch (e) {
                        return false;
                    }
                }]
            ];

            tests.forEach(([name, test]) => {
                try {
                    this.results.features[name] = test();
                } catch (e) {
                    this.results.features[name] = false;
                    this.results.errors.push(`${name}: ${e.message}`);
                }
            });
        }

        /**
         * 测试DOM API
         */
        testDOMAPIs() {
            const tests = [
                ['querySelector', () => typeof document.querySelector === 'function'],
                ['querySelectorAll', () => typeof document.querySelectorAll === 'function'],
                ['addEventListener', () => typeof document.addEventListener === 'function'],
                ['removeEventListener', () => typeof document.removeEventListener === 'function'],
                ['createElement', () => typeof document.createElement === 'function'],
                ['classList', () => {
                    const el = document.createElement('div');
                    return el.classList && typeof el.classList.add === 'function';
                }],
                ['dataset', () => {
                    const el = document.createElement('div');
                    return typeof el.dataset === 'object';
                }],
                ['MutationObserver', () => typeof MutationObserver !== 'undefined'],
                ['IntersectionObserver', () => typeof IntersectionObserver !== 'undefined']
            ];

            tests.forEach(([name, test]) => {
                try {
                    this.results.features[name] = test();
                } catch (e) {
                    this.results.features[name] = false;
                    this.results.errors.push(`${name}: ${e.message}`);
                }
            });
        }

        /**
         * 测试网络API
         */
        async testNetworkAPIs() {
            // 测试fetch基本功能
            try {
                const response = await fetch('/api/v2/link-status?url=https://example.com');
                this.results.features['fetch_basic'] = response instanceof Response;
            } catch (e) {
                this.results.features['fetch_basic'] = false;
                this.results.errors.push(`fetch_basic: ${e.message}`);
            }

            // 测试AbortController
            try {
                const controller = new AbortController();
                this.results.features['AbortController'] = typeof controller.abort === 'function';
            } catch (e) {
                this.results.features['AbortController'] = false;
                this.results.errors.push(`AbortController: ${e.message}`);
            }
        }

        /**
         * 测试存储API
         */
        testStorageAPIs() {
            const tests = [
                ['localStorage', () => {
                    try {
                        localStorage.setItem('test', 'test');
                        localStorage.removeItem('test');
                        return true;
                    } catch (e) {
                        return false;
                    }
                }],
                ['sessionStorage', () => {
                    try {
                        sessionStorage.setItem('test', 'test');
                        sessionStorage.removeItem('test');
                        return true;
                    } catch (e) {
                        return false;
                    }
                }],
                ['IndexedDB', () => typeof indexedDB !== 'undefined'],
                ['Cache API', () => typeof caches !== 'undefined']
            ];

            tests.forEach(([name, test]) => {
                try {
                    this.results.features[name] = test();
                } catch (e) {
                    this.results.features[name] = false;
                    this.results.errors.push(`${name}: ${e.message}`);
                }
            });
        }

        /**
         * 运行性能测试
         */
        runPerformanceTests() {
            // DOM查询性能
            const startQuery = performance.now();
            const links = document.querySelectorAll('a[href]');
            const endQuery = performance.now();
            this.results.performance.domQuery = {
                time: endQuery - startQuery,
                count: links.length
            };

            // 事件绑定性能
            const testLinks = Array.from(links).slice(0, 100); // 限制数量
            const startBind = performance.now();
            const handlers = testLinks.map(link => {
                const handler = () => {};
                link.addEventListener('click', handler);
                return { link, handler };
            });
            const endBind = performance.now();
            
            // 清理事件监听器
            handlers.forEach(({ link, handler }) => {
                link.removeEventListener('click', handler);
            });
            
            this.results.performance.eventBinding = {
                time: endBind - startBind,
                count: testLinks.length
            };

            // 内存使用（如果可用）
            if (performance.memory) {
                this.results.performance.memory = {
                    used: performance.memory.usedJSHeapSize,
                    total: performance.memory.totalJSHeapSize,
                    limit: performance.memory.jsHeapSizeLimit
                };
            }
        }

        /**
         * 生成兼容性报告
         */
        generateReport() {
            const report = {
                summary: this.generateSummary(),
                details: this.results,
                recommendations: this.generateRecommendations(),
                timestamp: new Date().toISOString()
            };

            // 输出到控制台
            console.group('🧪 兼容性测试报告');
            console.log('浏览器信息:', this.results.browser);
            console.log('功能支持:', this.results.features);
            console.log('性能指标:', this.results.performance);
            
            if (this.results.errors.length > 0) {
                console.warn('发现的问题:', this.results.errors);
            }
            
            console.log('建议:', report.recommendations);
            console.groupEnd();

            // 保存到window对象供外部访问
            window.monolithCompatReport = report;

            return report;
        }

        /**
         * 生成摘要
         */
        generateSummary() {
            const features = this.results.features;
            const total = Object.keys(features).length;
            const supported = Object.values(features).filter(Boolean).length;
            
            return {
                browser: `${this.results.browser.name} ${this.results.browser.version}`,
                compatibility: `${supported}/${total} (${Math.round(supported/total*100)}%)`,
                grade: this.calculateGrade(supported / total),
                critical_issues: this.results.errors.length
            };
        }

        /**
         * 计算兼容性等级
         */
        calculateGrade(ratio) {
            if (ratio >= 0.95) return 'A+';
            if (ratio >= 0.90) return 'A';
            if (ratio >= 0.85) return 'B+';
            if (ratio >= 0.80) return 'B';
            if (ratio >= 0.70) return 'C';
            return 'D';
        }

        /**
         * 生成建议
         */
        generateRecommendations() {
            const recommendations = [];
            const features = this.results.features;

            // 关键功能检查
            if (!features.fetch) {
                recommendations.push('建议使用polyfill支持fetch API');
            }
            if (!features.Promise) {
                recommendations.push('建议使用polyfill支持Promise');
            }
            if (!features.localStorage) {
                recommendations.push('localStorage不可用，将使用内存存储');
            }
            if (!features['Arrow Functions']) {
                recommendations.push('不支持ES6语法，建议使用轻量级版本');
            }

            // 性能建议
            const queryTime = this.results.performance.domQuery?.time || 0;
            if (queryTime > 50) {
                recommendations.push('DOM查询性能较慢，建议减少查询次数');
            }

            // 浏览器特定建议
            const browser = this.results.browser;
            if (browser.name === 'Internet Explorer') {
                recommendations.push('IE浏览器兼容性有限，强烈建议使用轻量级版本');
            }

            if (recommendations.length === 0) {
                recommendations.push('兼容性良好，可以使用完整功能');
            }

            return recommendations;
        }
    }

    /**
     * 拦截器功能测试
     */
    class InterceptorFunctionTester {
        constructor() {
            this.results = {
                initialization: false,
                linkDetection: false,
                statusQuery: false,
                eventHandling: false,
                caching: false,
                performance: {},
                errors: []
            };
        }

        /**
         * 测试拦截器功能
         */
        async testInterceptorFeatures() {
            console.log('🔗 开始测试拦截器功能...');

            // 测试初始化
            this.testInitialization();

            // 测试链接检测
            this.testLinkDetection();

            // 测试状态查询
            await this.testStatusQuery();

            // 测试事件处理
            this.testEventHandling();

            // 测试缓存功能
            this.testCaching();

            // 性能测试
            this.testPerformance();

            return this.results;
        }

        testInitialization() {
            try {
                // 检查拦截器类是否存在
                this.results.initialization = typeof SmartLinkInterceptor !== 'undefined';
            } catch (e) {
                this.results.errors.push(`初始化测试失败: ${e.message}`);
                this.results.initialization = false;
            }
        }

        testLinkDetection() {
            try {
                // 创建测试链接
                const testLink = document.createElement('a');
                testLink.href = 'https://example.com';
                testLink.textContent = 'Test Link';
                document.body.appendChild(testLink);

                // 检查是否能检测到链接
                const links = document.querySelectorAll('a[href^="http"]');
                this.results.linkDetection = links.length > 0;

                // 清理
                document.body.removeChild(testLink);
            } catch (e) {
                this.results.errors.push(`链接检测测试失败: ${e.message}`);
                this.results.linkDetection = false;
            }
        }

        async testStatusQuery() {
            try {
                // 测试API调用
                const response = await fetch('/api/v2/link-status?url=https://example.com');
                this.results.statusQuery = response.ok;
            } catch (e) {
                this.results.errors.push(`状态查询测试失败: ${e.message}`);
                this.results.statusQuery = false;
            }
        }

        testEventHandling() {
            try {
                // 创建测试元素
                const testElement = document.createElement('div');
                document.body.appendChild(testElement);

                // 测试事件绑定
                let eventFired = false;
                const handler = () => { eventFired = true; };
                testElement.addEventListener('click', handler);

                // 触发事件
                testElement.click();
                this.results.eventHandling = eventFired;

                // 清理
                testElement.removeEventListener('click', handler);
                document.body.removeChild(testElement);
            } catch (e) {
                this.results.errors.push(`事件处理测试失败: ${e.message}`);
                this.results.eventHandling = false;
            }
        }

        testCaching() {
            try {
                // 测试localStorage
                const testKey = 'monolith_test_cache';
                const testData = { test: true, timestamp: Date.now() };

                localStorage.setItem(testKey, JSON.stringify(testData));
                const retrieved = JSON.parse(localStorage.getItem(testKey));

                this.results.caching = retrieved && retrieved.test === true;

                // 清理
                localStorage.removeItem(testKey);
            } catch (e) {
                this.results.errors.push(`缓存测试失败: ${e.message}`);
                this.results.caching = false;
            }
        }

        testPerformance() {
            // 创建多个测试链接
            const linkCount = 100;
            const testLinks = [];

            const startCreate = performance.now();
            for (let i = 0; i < linkCount; i++) {
                const link = document.createElement('a');
                link.href = `https://example${i}.com`;
                link.textContent = `Test Link ${i}`;
                testLinks.push(link);
                document.body.appendChild(link);
            }
            const endCreate = performance.now();

            // 测试批量处理性能
            const startProcess = performance.now();
            testLinks.forEach(link => {
                link.dataset.monolithProcessed = 'true';
                link.addEventListener('click', () => {});
            });
            const endProcess = performance.now();

            this.results.performance = {
                creation: {
                    time: endCreate - startCreate,
                    count: linkCount
                },
                processing: {
                    time: endProcess - startProcess,
                    count: linkCount
                }
            };

            // 清理
            testLinks.forEach(link => {
                if (link.parentNode) {
                    link.parentNode.removeChild(link);
                }
            });
        }
    }

    /**
     * 主测试运行器
     */
    async function runCompatibilityTests() {
        const compatTester = new BrowserCompatibilityTester();
        const functionTester = new InterceptorFunctionTester();

        try {
            // 运行兼容性测试
            const compatResults = await compatTester.runAllTests();

            // 运行功能测试
            const functionResults = await functionTester.testInterceptorFeatures();

            // 合并结果
            const finalResults = {
                compatibility: compatResults,
                functionality: functionResults,
                timestamp: new Date().toISOString(),
                userAgent: navigator.userAgent
            };

            // 显示结果
            displayTestResults(finalResults);

            return finalResults;
        } catch (e) {
            console.error('测试运行失败:', e);
            return null;
        }
    }

    /**
     * 显示测试结果
     */
    function displayTestResults(results) {
        // 创建结果显示元素
        const resultDiv = document.createElement('div');
        resultDiv.id = 'monolith-test-results';
        resultDiv.style.cssText = `
            position: fixed;
            top: 10px;
            left: 10px;
            width: 400px;
            max-height: 600px;
            overflow-y: auto;
            background: white;
            border: 2px solid #333;
            border-radius: 8px;
            padding: 15px;
            font-family: monospace;
            font-size: 12px;
            z-index: 10000;
            box-shadow: 0 4px 12px rgba(0,0,0,0.3);
        `;

        const summary = results.compatibility.summary;
        const compat = results.compatibility.details.features;
        const func = results.functionality;

        resultDiv.innerHTML = `
            <h3 style="margin-top: 0; color: #333;">🧪 兼容性测试报告</h3>
            
            <div style="margin-bottom: 15px;">
                <strong>浏览器:</strong> ${summary.browser}<br>
                <strong>兼容性:</strong> ${summary.compatibility}<br>
                <strong>等级:</strong> <span style="color: ${getGradeColor(summary.grade)}">${summary.grade}</span><br>
                <strong>问题数:</strong> ${summary.critical_issues}
            </div>

            <details>
                <summary><strong>功能支持详情</strong></summary>
                <div style="margin: 10px 0;">
                    ${Object.entries(compat).map(([name, supported]) => 
                        `<div>${supported ? '✅' : '❌'} ${name}</div>`
                    ).join('')}
                </div>
            </details>

            <details>
                <summary><strong>拦截器功能测试</strong></summary>
                <div style="margin: 10px 0;">
                    ${Object.entries(func).filter(([key]) => key !== 'errors' && key !== 'performance')
                        .map(([name, result]) => 
                        `<div>${result ? '✅' : '❌'} ${name}</div>`
                    ).join('')}
                </div>
            </details>

            <div style="margin-top: 15px; text-align: center;">
                <button onclick="this.parentElement.parentElement.remove()" 
                        style="padding: 5px 10px; cursor: pointer;">关闭</button>
                <button onclick="console.log(window.monolithCompatReport)" 
                        style="padding: 5px 10px; cursor: pointer; margin-left: 5px;">查看详情</button>
            </div>
        `;

        document.body.appendChild(resultDiv);
    }

    function getGradeColor(grade) {
        const colors = {
            'A+': '#4caf50',
            'A': '#8bc34a',
            'B+': '#cddc39',
            'B': '#ffeb3b',
            'C': '#ff9800',
            'D': '#f44336'
        };
        return colors[grade] || '#999';
    }

    // 导出到全局
    window.MonolithCompatTester = {
        runTests: runCompatibilityTests,
        BrowserCompatibilityTester,
        InterceptorFunctionTester
    };

    // 自动运行测试（如果URL包含test参数）
    if (window.location.search.includes('compat_test=1')) {
        document.addEventListener('DOMContentLoaded', () => {
            setTimeout(runCompatibilityTests, 1000);
        });
    }

})();