/**
 * 底部状态栏管理器
 */

const Statusbar = {
    // 同步状态
    syncStatus: 'idle', // idle, syncing, synced, error
    lastSyncTime: null,
    
    // 统计数据
    stats: {
        wordCount: 0,
        charCount: 0,
        lineCount: 0
    },
    
    /**
     * 初始化状态栏
     */
    init() {
        this.loadLastSyncTime();
        this.updateStats();
        this.updateTime();
        this.loadVersion();
        
        // 定时更新时间
        setInterval(() => this.updateTime(), 1000);
        
        // 监听同步事件
        this.setupSyncListener();
    },
    
    /**
     * 加载上次同步时间
     */
    loadLastSyncTime() {
        const lastSync = localStorage.getItem('last_sync_time');
        console.log('[Statusbar] 加载上次同步时间:', lastSync);
        if (lastSync) {
            this.lastSyncTime = new Date(lastSync);
            console.log('[Statusbar] 解析同步时间:', this.lastSyncTime);
            this.updateSyncStatus();
        } else {
            console.log('[Statusbar] 没有找到同步记录，显示"未同步"');
        }
    },
    
    /**
     * 保存同步时间
     */
    saveSyncTime() {
        this.lastSyncTime = new Date();
        localStorage.setItem('last_sync_time', this.lastSyncTime.toISOString());
        console.log('[Statusbar] 保存同步时间:', this.lastSyncTime.toISOString());
        this.updateSyncStatus();
    },
    
    /**
     * 更新同步状态
     */
    updateSyncStatus() {
        const indicator = document.getElementById('statusbar-sync-indicator');
        const text = document.getElementById('statusbar-sync-text');
        
        if (!indicator || !text) return;
        
        // 移除所有状态类
        indicator.className = 'statusbar-sync-indicator';
        
        const t = window.i18n ? window.i18n.t : (key) => key;
        
        console.log('[Statusbar] 更新同步状态:', this.syncStatus, 'lastSyncTime:', this.lastSyncTime);
        
        switch (this.syncStatus) {
            case 'syncing':
                indicator.classList.add('syncing');
                text.textContent = t('statusbar.syncing');
                break;
            case 'synced':
                indicator.classList.add('synced');
                text.textContent = t('statusbar.synced');
                break;
            case 'error':
                indicator.classList.add('error');
                text.textContent = t('sidebar.sync_failed');
                break;
            default:
                if (this.lastSyncTime) {
                    const timeAgo = this.getTimeAgo(this.lastSyncTime);
                    text.textContent = `${timeAgo}前同步`;
                    console.log('[Statusbar] 显示同步时间:', `${timeAgo}前同步`);
                } else {
                    text.textContent = t('statusbar.sync');
                    console.log('[Statusbar] 显示"未同步"');
                }
        }
    },
    
    /**
     * 计算时间差
     */
    getTimeAgo(date) {
        const now = new Date();
        const diff = Math.floor((now - date) / 1000); // 秒
        
        if (diff < 60) return '刚刚';
        if (diff < 3600) return `${Math.floor(diff / 60)}分钟`;
        if (diff < 86400) return `${Math.floor(diff / 3600)}小时`;
        return `${Math.floor(diff / 86400)}天`;
    },
    
    /**
     * 设置同步监听器
     */
    setupSyncListener() {
        // 监听全局同步事件
        window.addEventListener('sync:start', () => {
            this.syncStatus = 'syncing';
            this.updateSyncStatus();
        });
        
        window.addEventListener('sync:success', () => {
            this.syncStatus = 'synced';
            this.saveSyncTime();
            
            // 3秒后恢复为空闲状态
            setTimeout(() => {
                this.syncStatus = 'idle';
                this.updateSyncStatus();
            }, 3000);
        });
        
        window.addEventListener('sync:error', () => {
            this.syncStatus = 'error';
            this.updateSyncStatus();
            
            // 5秒后恢复为空闲状态
            setTimeout(() => {
                this.syncStatus = 'idle';
                this.updateSyncStatus();
            }, 5000);
        });
    },
    
    /**
     * 更新内容统计
     */
    updateStats() {
        const content = document.querySelector('.markdown-body');
        if (!content) return;
        
        const text = content.innerText || '';
        
        // 字数统计（中文字符 + 英文单词）
        const chineseChars = text.match(/[\u4e00-\u9fa5]/g) || [];
        const englishWords = text.match(/[a-zA-Z]+/g) || [];
        this.stats.wordCount = chineseChars.length + englishWords.length;
        
        // 字符统计
        this.stats.charCount = text.length;
        
        // 行数统计
        this.stats.lineCount = text.split('\n').length;
        
        this.updateStatsDisplay();
    },
    
    /**
     * 更新统计显示
     */
    updateStatsDisplay() {
        const t = window.i18n ? window.i18n.t : (key, params) => {
            // 如果 i18n 未加载，使用默认中文格式
            if (key === 'statusbar.word_count') return `${params.count} 字`;
            if (key === 'statusbar.line_count') return `${params.count} 行`;
            return key;
        };
        
        const wordCountEl = document.getElementById('statusbar-wordcount');
        if (wordCountEl) {
            wordCountEl.textContent = t('statusbar.word_count', { count: this.stats.wordCount });
        }
        
        const lineCountEl = document.getElementById('statusbar-linecount');
        if (lineCountEl) {
            lineCountEl.textContent = t('statusbar.line_count', { count: this.stats.lineCount });
        }
    },
    
    /**
     * 更新当前时间
     */
    updateTime() {
        const timeEl = document.getElementById('statusbar-time');
        if (!timeEl) return;
        
        const now = new Date();
        const hours = String(now.getHours()).padStart(2, '0');
        const minutes = String(now.getMinutes()).padStart(2, '0');
        timeEl.textContent = `${hours}:${minutes}`;
    },
    
    /**
     * 加载版本号
     */
    async loadVersion() {
        try {
            const response = await fetch('/health');
            const data = await response.json();
            if (data.version) {
                const versionEl = document.getElementById('statusbar-version');
                if (versionEl) {
                    versionEl.textContent = `v${data.version}`;
                }
            }
        } catch (error) {
            console.error('获取版本号失败:', error);
        }
    },
    
    /**
     * 显示笔记统计信息
     */
    showNoteStats() {
        const t = window.i18n ? window.i18n.t : (key, params) => key;
        
        // 可以显示一个弹窗，展示详细的统计信息
        const currentLang = window.i18n ? window.i18n.getCurrentLanguage() : 'zh-CN';
        
        if (currentLang === 'zh-CN') {
            alert(`${t('note_stats.title')}：
字数：${this.stats.wordCount}
字符：${this.stats.charCount}
行数：${this.stats.lineCount}`);
        } else {
            alert(`${t('note_stats.title')}:
${t('note_stats.words')}: ${this.stats.wordCount}
${t('note_stats.characters')}: ${this.stats.charCount}
${t('note_stats.lines')}: ${this.stats.lineCount}`);
        }
    }
};

// 页面加载完成后初始化
document.addEventListener('DOMContentLoaded', () => {
    Statusbar.init();
    
    // 监听语言变更事件，重新更新显示
    window.addEventListener('languageChanged', () => {
        Statusbar.updateSyncStatus();
        Statusbar.updateStatsDisplay();
    });
});

// 内容变化时更新统计
if (typeof MutationObserver !== 'undefined') {
    const observer = new MutationObserver(() => {
        Statusbar.updateStats();
    });
    
    // 监听 markdown-body 的变化
    const contentObserver = () => {
        const content = document.querySelector('.markdown-body');
        if (content) {
            observer.observe(content, {
                childList: true,
                subtree: true,
                characterData: true
            });
        } else {
            // 如果还没有加载，等待一会再试
            setTimeout(contentObserver, 500);
        }
    };
    
    contentObserver();
}
