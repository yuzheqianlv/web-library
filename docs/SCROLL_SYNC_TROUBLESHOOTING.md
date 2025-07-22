# 同步滚动问题排查指南

## 🔍 问题识别

### 常见症状
1. **滚动不同步**: 一侧滚动，另一侧不跟随
2. **延迟同步**: 滚动后延迟1-2秒才同步
3. **跳跃性滚动**: 滚动不平滑，出现跳跃
4. **单向同步**: 只有一个方向能同步
5. **同步失效**: 开始能同步，后来失效

## 🛠️ 问题排查步骤

### 1. 基础检查

#### 检查控制台输出
```javascript
// 在浏览器控制台执行
console.log('滚动同步管理器:', window.translator?.scrollSyncManager);
console.log('同步状态:', window.translator?.scrollSyncManager?.checkSyncStatus());
```

#### 检查iframe加载状态
```javascript
// 检查iframe是否正常加载
const frames = [
    document.getElementById('bilingual-translated'),
    document.getElementById('bilingual-original')
];
frames.forEach((frame, index) => {
    console.log(`Frame ${index}:`, {
        src: frame.src,
        loaded: frame.complete,
        readyState: frame.readyState,
        contentDocument: !!frame.contentDocument
    });
});
```

### 2. 跨域问题检查

#### 症状
- 控制台显示 "无法访问iframe内容，可能是跨域限制"
- 同步状态显示 "同步滚动(受限模式)"

#### 解决方案
1. **检查Content-Security-Policy**
   ```bash
   # 检查响应头
   curl -I http://localhost:7080/api/translate
   ```

2. **验证iframe内容来源**
   ```javascript
   // 检查iframe的URL来源
   frames.forEach((frame, index) => {
       try {
           console.log(`Frame ${index} URL:`, frame.contentWindow.location.href);
       } catch (e) {
           console.log(`Frame ${index} CORS blocked:`, e.message);
       }
   });
   ```

### 3. 滚动容器检查

#### 检查滚动容器是否正确识别
```javascript
// 手动检查滚动容器
if (window.translator?.scrollSyncManager) {
    const frames = [
        document.getElementById('bilingual-translated'),
        document.getElementById('bilingual-original')
    ];
    
    frames.forEach((frame, index) => {
        try {
            const doc = frame.contentDocument;
            if (doc) {
                const candidates = [
                    doc.documentElement,
                    doc.body,
                    doc.querySelector('.main-content'),
                    doc.querySelector('main')
                ];
                
                candidates.forEach((el, i) => {
                    if (el) {
                        console.log(`Frame ${index} 候选容器 ${i}:`, {
                            tagName: el.tagName,
                            scrollHeight: el.scrollHeight,
                            clientHeight: el.clientHeight,
                            scrollable: el.scrollHeight > el.clientHeight,
                            overflow: getComputedStyle(el).overflow
                        });
                    }
                });
            }
        } catch (e) {
            console.log(`Frame ${index} 容器检查失败:`, e.message);
        }
    });
}
```

### 4. 事件监听器检查

#### 检查事件是否正确绑定
```javascript
// 检查事件监听器
const checkEventListeners = () => {
    try {
        const frames = [
            document.getElementById('bilingual-translated'),
            document.getElementById('bilingual-original')
        ];
        
        frames.forEach((frame, index) => {
            const doc = frame.contentDocument;
            if (doc && doc.documentElement) {
                // 触发一个测试滚动事件
                const scrollTop = doc.documentElement.scrollTop;
                doc.documentElement.scrollTop = scrollTop + 1;
                doc.documentElement.scrollTop = scrollTop;
                console.log(`Frame ${index} 滚动测试完成`);
            }
        });
    } catch (e) {
        console.error('事件监听器检查失败:', e);
    }
};

// 延迟执行检查
setTimeout(checkEventListeners, 1000);
```

## 🔧 快速修复方案

### 方案1: 手动同步
```javascript
// 按 Ctrl+Shift+S 手动触发同步
// 或在控制台执行:
if (window.translator?.scrollSyncManager) {
    window.translator.scrollSyncManager.manualSync();
}
```

### 方案2: 重新初始化
```javascript
// 重新设置同步滚动
if (window.translator?.scrollSyncManager) {
    window.translator.scrollSyncManager.cleanup();
    setTimeout(() => {
        window.translator.scrollSyncManager.setupBilingualSync();
    }, 500);
}
```

### 方案3: 降级到原始同步
```javascript
// 如果改进版不工作，回退到原始版本
if (window.translator) {
    window.translator.scrollSyncManager = null;
    window.translator.setupBilingualSync();
}
```

## 🐛 常见问题解决

### 问题1: iframe内容无法访问
**原因**: 跨域安全限制
**解决**: 
1. 确保翻译后的HTML不包含跨域限制
2. 检查CSP头设置
3. 使用fallback同步方案

### 问题2: 滚动事件不触发
**原因**: 滚动容器识别错误
**解决**:
```javascript
// 强制指定滚动容器
const forceScrollContainer = (frameIndex, selector) => {
    const frame = document.getElementById(frameIndex === 0 ? 'bilingual-translated' : 'bilingual-original');
    const doc = frame.contentDocument;
    const container = doc.querySelector(selector) || doc.documentElement;
    
    // 添加调试事件
    container.addEventListener('scroll', (e) => {
        console.log(`Frame ${frameIndex} 滚动:`, e.target.scrollTop);
    });
};

// 使用示例
forceScrollContainer(0, 'html');
forceScrollContainer(1, 'html');
```

### 问题3: 同步延迟过大
**原因**: 节流设置过高或事件处理过慢
**解决**:
```javascript
// 调整节流时间
if (window.translator?.scrollSyncManager) {
    window.translator.scrollSyncManager.syncThrottle = 8; // 降低到8ms
}
```

### 问题4: 滚动比例计算错误
**原因**: 内容高度计算错误
**解决**:
```javascript
// 检查高度计算
const checkScrollMetrics = () => {
    const frames = [
        document.getElementById('bilingual-translated'),
        document.getElementById('bilingual-original')
    ];
    
    frames.forEach((frame, index) => {
        try {
            const doc = frame.contentDocument;
            const el = doc.documentElement;
            console.log(`Frame ${index} 滚动指标:`, {
                scrollTop: el.scrollTop,
                scrollHeight: el.scrollHeight,
                clientHeight: el.clientHeight,
                maxScroll: el.scrollHeight - el.clientHeight,
                ratio: el.scrollTop / (el.scrollHeight - el.clientHeight)
            });
        } catch (e) {
            console.error(`Frame ${index} 指标检查失败:`, e);
        }
    });
};

setInterval(checkScrollMetrics, 2000); // 每2秒检查一次
```

## 🔬 高级调试

### 启用详细日志
```javascript
// 在控制台执行以启用详细日志
window.SCROLL_SYNC_DEBUG = true;

// 修改同步管理器以输出更多信息
if (window.translator?.scrollSyncManager) {
    const original = window.translator.scrollSyncManager.getScrollRatio;
    window.translator.scrollSyncManager.getScrollRatio = function(element) {
        const ratio = original.call(this, element);
        if (window.SCROLL_SYNC_DEBUG) {
            console.debug('滚动比例计算:', {
                scrollTop: element.scrollTop,
                scrollHeight: element.scrollHeight,
                clientHeight: element.clientHeight,
                ratio: ratio
            });
        }
        return ratio;
    };
}
```

### 性能监控
```javascript
// 监控同步性能
const monitorSyncPerformance = () => {
    let syncCount = 0;
    let totalTime = 0;
    
    const originalSync = window.translator?.scrollSyncManager?.setupSyncListeners;
    if (originalSync) {
        window.translator.scrollSyncManager.setupSyncListeners = function(...args) {
            const start = performance.now();
            const result = originalSync.apply(this, args);
            const end = performance.now();
            
            syncCount++;
            totalTime += (end - start);
            
            console.log(`同步设置性能: ${end - start}ms, 平均: ${totalTime / syncCount}ms`);
            return result;
        };
    }
};

monitorSyncPerformance();
```

## 📋 故障报告模板

如果问题无法解决，请收集以下信息：

```javascript
// 收集故障信息
const collectDiagnosticInfo = () => {
    const info = {
        userAgent: navigator.userAgent,
        viewport: {
            width: window.innerWidth,
            height: window.innerHeight
        },
        translator: !!window.translator,
        scrollSyncManager: !!window.translator?.scrollSyncManager,
        currentMode: window.translator?.currentMode,
        syncStatus: window.translator?.scrollSyncManager?.checkSyncStatus(),
        frames: []
    };
    
    // 检查iframe状态
    const frames = [
        document.getElementById('bilingual-translated'),
        document.getElementById('bilingual-original')
    ];
    
    frames.forEach((frame, index) => {
        const frameInfo = {
            exists: !!frame,
            src: frame?.src,
            loaded: frame?.complete,
            readyState: frame?.readyState,
            hasContentDocument: !!frame?.contentDocument,
            dimensions: frame ? {
                width: frame.offsetWidth,
                height: frame.offsetHeight
            } : null
        };
        
        try {
            if (frame?.contentDocument) {
                const doc = frame.contentDocument;
                frameInfo.documentReady = doc.readyState;
                frameInfo.scrollMetrics = {
                    scrollTop: doc.documentElement.scrollTop,
                    scrollHeight: doc.documentElement.scrollHeight,
                    clientHeight: doc.documentElement.clientHeight
                };
            }
        } catch (e) {
            frameInfo.accessError = e.message;
        }
        
        info.frames.push(frameInfo);
    });
    
    console.log('诊断信息:', JSON.stringify(info, null, 2));
    return info;
};

// 执行诊断
const diagnosticInfo = collectDiagnosticInfo();
```

## 🚀 性能优化建议

1. **减少节流时间**: 将同步频率调整到16ms(60fps)
2. **使用requestAnimationFrame**: 对于平滑动画效果
3. **避免强制同步**: 使用异步同步减少阻塞
4. **优化DOM查询**: 缓存滚动容器引用
5. **监控内存泄漏**: 及时清理事件监听器

## 📞 获取帮助

如果以上方法都无法解决问题，请：

1. 复制诊断信息 (`collectDiagnosticInfo()`)
2. 记录具体的重现步骤
3. 提供浏览器版本和操作系统信息
4. 提交Issue到项目仓库

---

**最后更新**: 2025-07-21  
**版本**: v1.0.0