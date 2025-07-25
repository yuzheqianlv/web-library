/**
 * 简化的滚动同步实现
 * 作为主同步方案的备用选择
 */

class SimpleScrollSync {
    constructor(translator) {
        this.translator = translator;
        this.isActive = false;
        this.lastSyncSource = null;
        this.syncDelay = 50; // 简单的防抖延迟
    }

    /**
     * 启动简单同步
     */
    start() {
        console.log('启动简单滚动同步...');
        
        try {
            // 获取iframe元素
            const translatedFrame = this.translator.bilingualTranslated;
            const originalFrame = this.translator.bilingualOriginal;
            
            if (!translatedFrame || !originalFrame) {
                console.error('无法找到iframe元素');
                return false;
            }

            // 等待iframe加载
            this.waitForLoad(translatedFrame, originalFrame).then(() => {
                this.setupSimpleSync(translatedFrame, originalFrame);
                this.isActive = true;
                this.translator.updateSyncStatus('简单同步已启用', false);
            }).catch((error) => {
                console.error('简单同步设置失败:', error);
                this.translator.updateSyncStatus('简单同步失败', true);
            });

            return true;
        } catch (error) {
            console.error('简单同步启动失败:', error);
            return false;
        }
    }

    /**
     * 等待iframe加载完成
     */
    waitForLoad(frame1, frame2) {
        return new Promise((resolve) => {
            let loadCount = 0;
            const checkLoad = () => {
                loadCount++;
                if (loadCount >= 2) {
                    // 额外等待确保内容渲染
                    setTimeout(resolve, 300);
                }
            };

            // 检查是否已加载
            if (frame1.complete || frame1.readyState === 'complete') {
                checkLoad();
            } else {
                frame1.addEventListener('load', checkLoad, { once: true });
            }

            if (frame2.complete || frame2.readyState === 'complete') {
                checkLoad();
            } else {
                frame2.addEventListener('load', checkLoad, { once: true });
            }
        });
    }

    /**
     * 设置简单同步机制
     */
    setupSimpleSync(translatedFrame, originalFrame) {
        console.log('设置简单同步机制...');

        // 方案1: 使用iframe窗口的滚动事件（如果可以访问）
        this.tryWindowSync(translatedFrame, originalFrame) ||
        // 方案2: 使用容器滚动事件
        this.tryContainerSync(translatedFrame, originalFrame) ||
        // 方案3: 使用定时检查
        this.usePollingSync(translatedFrame, originalFrame);
    }

    /**
     * 尝试窗口滚动同步
     */
    tryWindowSync(translatedFrame, originalFrame) {
        try {
            const translatedWindow = translatedFrame.contentWindow;
            const originalWindow = originalFrame.contentWindow;

            if (!translatedWindow || !originalWindow) {
                return false;
            }

            // 简单的防抖同步
            let syncTimeout;
            const sync = (sourceWindow, targetWindow, direction) => {
                if (this.lastSyncSource === sourceWindow) return;
                
                clearTimeout(syncTimeout);
                syncTimeout = setTimeout(() => {
                    try {
                        this.lastSyncSource = sourceWindow;
                        const sourceDoc = sourceWindow.document;
                        const targetDoc = targetWindow.document;
                        
                        if (sourceDoc && targetDoc) {
                            const sourceScroll = sourceDoc.documentElement.scrollTop || sourceDoc.body.scrollTop;
                            const sourceMax = (sourceDoc.documentElement.scrollHeight || sourceDoc.body.scrollHeight) - sourceWindow.innerHeight;
                            
                            if (sourceMax > 0) {
                                const ratio = sourceScroll / sourceMax;
                                const targetMax = (targetDoc.documentElement.scrollHeight || targetDoc.body.scrollHeight) - targetWindow.innerHeight;
                                const targetScroll = targetMax * ratio;
                                
                                if (targetDoc.documentElement.scrollTo) {
                                    targetDoc.documentElement.scrollTo(0, targetScroll);
                                } else {
                                    targetDoc.documentElement.scrollTop = targetScroll;
                                }
                            }
                        }
                        
                        setTimeout(() => {
                            this.lastSyncSource = null;
                        }, this.syncDelay);
                        
                        console.debug(`简单同步: ${direction}`);
                    } catch (error) {
                        console.debug('同步执行错误:', error);
                    }
                }, 16); // 约60fps
            };

            // 绑定滚动事件
            translatedWindow.addEventListener('scroll', () => {
                sync(translatedWindow, originalWindow, '译文→原文');
            }, { passive: true });

            originalWindow.addEventListener('scroll', () => {
                sync(originalWindow, translatedWindow, '原文→译文');
            }, { passive: true });

            console.log('窗口滚动同步设置成功');
            return true;

        } catch (error) {
            console.debug('窗口滚动同步设置失败:', error);
            return false;
        }
    }

    /**
     * 尝试容器滚动同步
     */
    tryContainerSync(translatedFrame, originalFrame) {
        try {
            // 使用iframe的父容器进行同步
            const containers = [translatedFrame.parentElement, originalFrame.parentElement];
            
            if (!containers[0] || !containers[1]) {
                return false;
            }

            let syncLock = false;
            const sync = (sourceIndex) => {
                if (syncLock) return;
                syncLock = true;

                const source = containers[sourceIndex];
                const target = containers[1 - sourceIndex];
                
                try {
                    const sourceRatio = source.scrollTop / (source.scrollHeight - source.clientHeight);
                    const targetScroll = (target.scrollHeight - target.clientHeight) * sourceRatio;
                    target.scrollTop = targetScroll;
                } catch (error) {
                    console.debug('容器同步错误:', error);
                }

                setTimeout(() => {
                    syncLock = false;
                }, this.syncDelay);
            };

            containers[0].addEventListener('scroll', () => sync(0), { passive: true });
            containers[1].addEventListener('scroll', () => sync(1), { passive: true });

            console.log('容器滚动同步设置成功');
            return true;

        } catch (error) {
            console.debug('容器滚动同步设置失败:', error);
            return false;
        }
    }

    /**
     * 使用轮询同步（最后的备用方案）
     */
    usePollingSync(translatedFrame, originalFrame) {
        console.log('使用轮询同步方案');

        let lastScrollPositions = [0, 0];
        let userScrolling = false;
        let scrollDirection = 0; // 0: none, 1: translated, 2: original

        const pollSync = () => {
            if (!this.isActive) return;

            try {
                const frames = [translatedFrame, originalFrame];
                const currentPositions = [0, 0];

                // 获取当前滚动位置
                frames.forEach((frame, index) => {
                    try {
                        const doc = frame.contentDocument || frame.contentWindow?.document;
                        if (doc) {
                            currentPositions[index] = doc.documentElement.scrollTop || doc.body.scrollTop || 0;
                        }
                    } catch (error) {
                        // 跨域时静默处理
                    }
                });

                // 检测滚动变化
                const changes = currentPositions.map((pos, index) => pos !== lastScrollPositions[index]);
                
                if (changes[0] && !changes[1] && scrollDirection !== 1) {
                    // 译文滚动，同步到原文
                    scrollDirection = 1;
                    this.syncPosition(frames[0], frames[1]);
                } else if (changes[1] && !changes[0] && scrollDirection !== 2) {
                    // 原文滚动，同步到译文
                    scrollDirection = 2;
                    this.syncPosition(frames[1], frames[0]);
                }

                // 如果都没有变化，重置方向
                if (!changes[0] && !changes[1]) {
                    scrollDirection = 0;
                }

                lastScrollPositions = [...currentPositions];

            } catch (error) {
                console.debug('轮询同步错误:', error);
            }

            // 继续轮询
            setTimeout(pollSync, 100); // 10fps的轮询频率
        };

        // 开始轮询
        pollSync();
        return true;
    }

    /**
     * 同步滚动位置
     */
    syncPosition(sourceFrame, targetFrame) {
        try {
            const sourceDoc = sourceFrame.contentDocument || sourceFrame.contentWindow?.document;
            const targetDoc = targetFrame.contentDocument || targetFrame.contentWindow?.document;

            if (!sourceDoc || !targetDoc) return;

            const sourceScroll = sourceDoc.documentElement.scrollTop || sourceDoc.body.scrollTop;
            const sourceMax = (sourceDoc.documentElement.scrollHeight || sourceDoc.body.scrollHeight) - 
                             (sourceFrame.contentWindow?.innerHeight || sourceDoc.documentElement.clientHeight);

            if (sourceMax > 0) {
                const ratio = sourceScroll / sourceMax;
                const targetMax = (targetDoc.documentElement.scrollHeight || targetDoc.body.scrollHeight) - 
                                (targetFrame.contentWindow?.innerHeight || targetDoc.documentElement.clientHeight);
                const targetScroll = Math.max(0, Math.min(targetMax, targetMax * ratio));

                // 设置目标滚动位置
                if (targetDoc.documentElement.scrollTo) {
                    targetDoc.documentElement.scrollTo({
                        top: targetScroll,
                        behavior: 'auto'
                    });
                } else {
                    targetDoc.documentElement.scrollTop = targetScroll;
                    if (targetDoc.body) {
                        targetDoc.body.scrollTop = targetScroll;
                    }
                }
            }

        } catch (error) {
            console.debug('位置同步错误:', error);
        }
    }

    /**
     * 停止同步
     */
    stop() {
        this.isActive = false;
        console.log('简单滚动同步已停止');
    }

    /**
     * 手动同步
     */
    manualSync() {
        if (!this.isActive) {
            console.warn('简单同步未激活');
            return;
        }

        try {
            const translatedFrame = this.translator.bilingualTranslated;
            const originalFrame = this.translator.bilingualOriginal;

            if (translatedFrame && originalFrame) {
                // 以译文为准同步到原文
                this.syncPosition(translatedFrame, originalFrame);
                console.log('手动简单同步完成');
            }
        } catch (error) {
            console.error('手动简单同步失败:', error);
        }
    }

    /**
     * 获取状态
     */
    getStatus() {
        return {
            isActive: this.isActive,
            type: 'simple',
            lastSyncSource: this.lastSyncSource ? 'active' : 'none'
        };
    }
}

// 导出供使用
window.SimpleScrollSync = SimpleScrollSync;