// 阅读进度跟踪
const ReadingProgress = {
    currentNotePath: null,
    currentNoteTitle: null,
    lastSavedPosition: 0,
    lastSaveTime: Date.now(),
    readingStartTime: Date.now(),
    saveInterval: null,
    isTracking: false,
    
    // 初始化阅读进度跟踪
    init(notePath, noteTitle) {
        console.log('📖 初始化阅读进度跟踪:', notePath, noteTitle);
        
        this.currentNotePath = notePath;
        this.currentNoteTitle = noteTitle;
        this.readingStartTime = Date.now();
        this.isTracking = true;
        
        // 获取滚动容器
        // 在移动端 (<=768px)，页面使用原生滚动 (window/body)，而不是内层容器
        const isMobile = window.innerWidth <= 768;
        
        if (isMobile) {
            // 移动端使用 window 作为滚动容器的代理
            this.scrollContainer = window;
            this.scrollElement = document.documentElement; // 用于获取 scrollHeight 等属性
            console.log('📱 检测到移动端，使用 window 滚动');
        } else {
            // 桌面端优先使用内层滚动容器
            this.scrollContainer = document.querySelector('.page-scrollable-content') || document.querySelector('.main-scroll-container');
            this.scrollElement = this.scrollContainer;
        }

        if (!this.scrollContainer) {
            console.error('❌ 未找到滚动容器！');
            this.isTracking = false;
            return;
        }
        
        console.log('✅ 找到滚动容器:', this.scrollContainer === window ? 'Window' : this.scrollContainer);
        
        // 加载并恢复上次的阅读位置
        this.loadAndRestoreProgress();
        
        // 开始自动保存（每10秒）
        this.startAutoSave();
        
        // 监听滚动事件（节流）
        this.attachScrollListener();
        
        // 页面卸载时保存
        window.addEventListener('beforeunload', () => {
            this.saveProgress(true); // 同步保存
        });
        
        console.log('✅ 阅读进度跟踪已启动');
    },
    
    // 加载并恢复上次的阅读位置
    async loadAndRestoreProgress() {
        try {
            const encodedPath = encodeURIComponent(this.currentNotePath);
            console.log('📥 正在加载阅读进度:', this.currentNotePath);
            
            const response = await fetch(`/api/reading/progress/${encodedPath}`, {
                credentials: 'include'
            });
            
            console.log('📥 加载进度 API 响应:', response.status, response.statusText);
            
            if (response.ok) {
                const progress = await response.json();
                console.log('✅ 成功加载阅读进度:', progress);
                
                // 设置最后保存的位置，防止立即覆盖
                this.lastSavedPosition = progress.scroll_position;
                
                // 如果上次阅读进度大于5%，显示恢复提示
                if (progress.scroll_percentage > 5 && progress.scroll_percentage < 95) {
                    console.log('📋 显示恢复阅读提示 (进度:', progress.scroll_percentage.toFixed(1) + '%)');
                    this.showRestorePrompt(progress);
                } else if (progress.scroll_position > 0) {
                    // 自动恢复到上次位置（不显示提示）
                    console.log('🔄 自动恢复到上次位置:', progress.scroll_position + 'px');
                    setTimeout(() => {
                        this.doScroll(progress.scroll_position, 'auto');
                    }, 100);
                }
            } else {
                console.log('ℹ️ 未找到阅读进度记录 (状态码:', response.status + ')');
            }
        } catch (error) {
            console.error('❌ 加载阅读进度失败:', error);
        }
    },
    
    // 显示恢复阅读位置的提示
    showRestorePrompt(progress) {
        console.log('🎨 开始创建恢复提示框...');
        const prompt = document.createElement('div');
        prompt.className = 'reading-progress-prompt';
        prompt.innerHTML = `
            <div class="reading-progress-prompt-content">
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M19 21l-7-5-7 5V5a2 2 0 0 1 2-2h10a2 2 0 0 1 2 2z"></path>
                </svg>
                <span>继续阅读 (${Math.round(progress.scroll_percentage)}%)</span>
                <div class="reading-progress-prompt-actions">
                    <button class="btn-restore">继续</button>
                    <button class="btn-dismiss">从头开始</button>
                </div>
            </div>
        `;
        
        console.log('🎨 提示框 HTML 已创建:', prompt);
        document.body.appendChild(prompt);
        console.log('🎨 提示框已添加到 body，当前父元素:', prompt.parentElement);
        
        // 显示动画
        setTimeout(() => {
            prompt.classList.add('show');
            console.log('🎨 提示框已添加 show 类，className:', prompt.className);
        }, 10);
        
        // 继续阅读按钮
        prompt.querySelector('.btn-restore').onclick = () => {
            console.log('👆 点击了"继续"按钮');
            this.doScroll(progress.scroll_position, 'smooth');
            this.hidePrompt(prompt);
        };
        
        // 从头开始按钮
        prompt.querySelector('.btn-dismiss').onclick = () => {
            console.log('👆 点击了"从头开始"按钮');
            this.hidePrompt(prompt);
        };
        
        // 5秒后自动隐藏
        setTimeout(() => {
            if (prompt.parentElement) {
                console.log('⏱️ 5秒已到，自动隐藏提示框');
                this.hidePrompt(prompt);
            }
        }, 5000);
    },
    
    // 隐藏提示
    hidePrompt(prompt) {
        prompt.classList.remove('show');
        setTimeout(() => {
            if (prompt.parentElement) {
                prompt.parentElement.removeChild(prompt);
            }
        }, 300);
    },
    
    // 开始自动保存
    startAutoSave() {
        // 每10秒保存一次
        this.saveInterval = setInterval(() => {
            this.saveProgress();
        }, 10000);
    },
    
    // 停止自动保存
    stopAutoSave() {
        if (this.saveInterval) {
            clearInterval(this.saveInterval);
            this.saveInterval = null;
        }
        this.isTracking = false;
    },
    
    // 附加滚动监听器（使用节流）
    attachScrollListener() {
        let scrollTimeout;
        
        if (!this.scrollContainer) {
            console.error('❌ 无法附加滚动监听器：scrollContainer 未定义');
            return;
        }
        
        this.scrollContainer.addEventListener('scroll', () => {
            if (!this.isTracking) return;
            
            // 清除之前的定时器
            if (scrollTimeout) {
                clearTimeout(scrollTimeout);
            }
            
            // 滚动停止500ms后保存
            scrollTimeout = setTimeout(() => {
                const currentPosition = this.getScrollTop();
                
                // 如果滚动位置变化超过100px，立即保存
                if (Math.abs(currentPosition - this.lastSavedPosition) > 100) {
                    this.saveProgress();
                }
            }, 500);
        });
        
        console.log('✅ 滚动监听器已附加');
    },
    
    // 获取当前滚动位置 (兼容 window 和 element)
    getScrollTop() {
        if (this.scrollContainer === window) {
            return window.scrollY || document.documentElement.scrollTop || document.body.scrollTop;
        }
        return this.scrollContainer.scrollTop;
    },

    // 执行滚动 (兼容 window 和 element)
    doScroll(top, behavior = 'auto') {
        if (this.scrollContainer === window) {
            window.scrollTo({ top, behavior });
        } else {
            this.scrollContainer.scrollTo({ top, behavior });
        }
    },
    
    // 保存阅读进度
    async saveProgress(sync = false) {
        if (!this.isTracking || !this.currentNotePath || !this.scrollContainer) {
            console.log('⚠️ 跳过保存 - isTracking:', this.isTracking, 'currentNotePath:', this.currentNotePath, 'scrollContainer:', !!this.scrollContainer);
            return;
        }
        
        const scrollPosition = Math.round(this.getScrollTop());
        
        // 获取文档高度
        let scrollHeight, clientHeight;
        if (this.scrollContainer === window) {
            scrollHeight = document.documentElement.scrollHeight;
            clientHeight = window.innerHeight;
        } else {
            scrollHeight = this.scrollContainer.scrollHeight;
            clientHeight = this.scrollContainer.clientHeight;
        }
        
        const docHeight = scrollHeight - clientHeight;
        const scrollPercentage = docHeight > 0 ? (scrollPosition / docHeight) * 100 : 0;
        
        console.log('📊 当前滚动信息:', {
            scrollPosition,
            scrollHeight,
            clientHeight,
            docHeight,
            scrollPercentage: scrollPercentage.toFixed(1) + '%',
            lastSavedPosition: this.lastSavedPosition
        });
        
        // 如果位置没有变化，跳过保存
        if (Math.abs(scrollPosition - this.lastSavedPosition) < 50) {
            console.log('⏭️ 位置变化小于50px，跳过保存 (当前:', scrollPosition, 'px, 上次:', this.lastSavedPosition, 'px)');
            return;
        }
        
        const now = Date.now();
        const durationDelta = Math.round((now - this.lastSaveTime) / 1000); // 秒
        
        const data = {
            note_path: this.currentNotePath,
            note_title: this.currentNoteTitle,
            scroll_position: scrollPosition,
            scroll_percentage: Math.round(scrollPercentage * 100) / 100,
            duration_delta: durationDelta
        };
        
        console.log('💾 准备保存阅读进度:', data);
        
        try {
            if (sync) {
                // 同步保存（使用 sendBeacon 或 fetch with keepalive）
                const blob = new Blob([JSON.stringify(data)], { type: 'application/json' });
                navigator.sendBeacon('/api/reading/progress', blob);
                console.log('📤 使用 sendBeacon 保存进度');
            } else {
                // 异步保存
                const response = await fetch('/api/reading/progress', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    credentials: 'include',
                    body: JSON.stringify(data)
                });
                console.log('📤 保存进度 API 响应:', response.status, response.statusText);
            }
            
            this.lastSavedPosition = scrollPosition;
            this.lastSaveTime = now;
            
            console.log(`✅ 阅读进度已保存: ${scrollPercentage.toFixed(1)}% (位置: ${scrollPosition}px)`);
        } catch (error) {
            console.error('❌ 保存阅读进度失败:', error);
        }
    },
    
    // 添加阅读历史记录（在离开页面时）
    async addHistory() {
        if (!this.currentNotePath) return;
        
        const duration = Math.round((Date.now() - this.readingStartTime) / 1000);
        
        // 只记录阅读时间超过5秒的记录
        if (duration < 5) return;
        
        const data = {
            note_path: this.currentNotePath,
            note_title: this.currentNoteTitle,
            duration: duration
        };
        
        try {
            const blob = new Blob([JSON.stringify(data)], { type: 'application/json' });
            navigator.sendBeacon('/api/reading/history', blob);
        } catch (error) {
            console.error('添加阅读历史失败:', error);
        }
    },
    
    // 清理（页面卸载时调用）
    cleanup() {
        this.stopAutoSave();
        this.saveProgress(true);
        this.addHistory();
    }
};

// 页面加载完成后自动初始化
document.addEventListener('DOMContentLoaded', () => {
    // 检查是否在笔记页面
    const notePathMeta = document.querySelector('meta[name="note-path"]');
    const noteTitleMeta = document.querySelector('meta[name="note-title"]');
    
    if (notePathMeta && noteTitleMeta) {
        const notePath = notePathMeta.getAttribute('content');
        const noteTitle = noteTitleMeta.getAttribute('content');
        
        if (notePath && noteTitle) {
            ReadingProgress.init(notePath, noteTitle);
        }
    }
});

// 页面卸载前清理
window.addEventListener('beforeunload', () => {
    ReadingProgress.cleanup();
});

// 页面隐藏时保存（用户切换标签页）
document.addEventListener('visibilitychange', () => {
    if (document.hidden) {
        ReadingProgress.saveProgress(true);
    }
});
