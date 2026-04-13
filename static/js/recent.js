// 最近访问笔记管理模块
(function() {
    const STORAGE_KEY = 'obsidian_mirror_recent_notes';
    const MAX_RECENT_NOTES = 10; // 最多保存 10 条最近访问记录

    /**
     * 获取最近访问笔记列表
     * @returns {Array} 最近访问笔记数组，格式: [{title, path, timestamp}]
     */
    function getRecentNotes() {
        try {
            const data = localStorage.getItem(STORAGE_KEY);
            if (!data) return [];
            const notes = JSON.parse(data);
            // 按时间戳倒序排序（最新的在前面）
            return notes.sort((a, b) => b.timestamp - a.timestamp);
        } catch (e) {
            console.error('读取最近访问记录失败:', e);
            return [];
        }
    }

    /**
     * 保存最近访问笔记列表
     * @param {Array} notes - 最近访问笔记数组
     */
    function saveRecentNotes(notes) {
        try {
            localStorage.setItem(STORAGE_KEY, JSON.stringify(notes));
        } catch (e) {
            console.error('保存最近访问记录失败:', e);
        }
    }

    /**
     * 添加笔记到最近访问列表
     * @param {string} title - 笔记标题
     * @param {string} path - 笔记路径
     */
    function addRecentNote(title, path) {
        if (!title || !path) return;

        let notes = getRecentNotes();
        
        // 移除已存在的同一笔记（避免重复）
        notes = notes.filter(note => note.path !== path);
        
        // 添加新笔记到列表开头
        notes.unshift({
            title: title,
            path: path,
            timestamp: Date.now()
        });
        
        // 限制列表长度
        if (notes.length > MAX_RECENT_NOTES) {
            notes = notes.slice(0, MAX_RECENT_NOTES);
        }
        
        saveRecentNotes(notes);
        
        // 更新 UI
        renderRecentNotes();
    }

    /**
     * 清空最近访问列表
     */
    function clearRecentNotes() {
        try {
            localStorage.removeItem(STORAGE_KEY);
            renderRecentNotes();
        } catch (e) {
            console.error('清空最近访问记录失败:', e);
        }
    }

    /**
     * 渲染最近访问笔记列表到侧边栏
     */
    function renderRecentNotes() {
        const container = document.getElementById('recent-notes-container');
        if (!container) return;

        const notes = getRecentNotes();
        
        if (notes.length === 0) {
            container.innerHTML = `
                <div class="recent-notes-empty">
                    <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                        <path d="M12 2v20M2 12h20"></path>
                    </svg>
                    <p>暂无访问记录</p>
                </div>
            `;
            return;
        }

        let html = '<div class="recent-notes-list">';
        notes.forEach((note, index) => {
            const timeAgo = formatTimeAgo(note.timestamp);
            html += `
                <a href="/doc/${note.path}" class="recent-note-item" title="${note.title}">
                    <span class="recent-note-index">${index + 1}</span>
                    <div class="recent-note-content">
                        <div class="recent-note-title">${escapeHtml(note.title)}</div>
                        <div class="recent-note-time">${timeAgo}</div>
                    </div>
                </a>
            `;
        });
        html += '</div>';
        
        // 添加清除按钮
        html += `
            <button class="recent-notes-clear" onclick="RecentNotes.clear()">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="3 6 5 6 21 6"></polyline>
                    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                </svg>
                清空记录
            </button>
        `;
        
        container.innerHTML = html;
    }

    /**
     * 格式化时间为相对时间
     * @param {number} timestamp - 时间戳
     * @returns {string} 格式化的时间字符串
     */
    function formatTimeAgo(timestamp) {
        const now = Date.now();
        const diff = now - timestamp;
        
        const seconds = Math.floor(diff / 1000);
        const minutes = Math.floor(seconds / 60);
        const hours = Math.floor(minutes / 60);
        const days = Math.floor(hours / 24);
        
        if (days > 0) return `${days} 天前`;
        if (hours > 0) return `${hours} 小时前`;
        if (minutes > 0) return `${minutes} 分钟前`;
        return '刚刚';
    }

    /**
     * HTML 转义
     * @param {string} text - 原始文本
     * @returns {string} 转义后的文本
     */
    function escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    /**
     * 切换最近访问面板的展开/收起状态
     */
    function toggleRecentPanel() {
        const panel = document.getElementById('recent-notes-panel');
        if (!panel) return;
        
        panel.classList.toggle('collapsed');
        
        // 保存折叠状态
        const isCollapsed = panel.classList.contains('collapsed');
        try {
            localStorage.setItem('recent_notes_collapsed', isCollapsed ? 'true' : 'false');
        } catch (e) {
            console.error('保存折叠状态失败:', e);
        }
    }

    /**
     * 恢复折叠状态
     */
    function restoreCollapseState() {
        try {
            const isCollapsed = localStorage.getItem('recent_notes_collapsed') === 'true';
            const panel = document.getElementById('recent-notes-panel');
            if (panel && isCollapsed) {
                panel.classList.add('collapsed');
            }
        } catch (e) {
            console.error('恢复折叠状态失败:', e);
        }
    }

    /**
     * 初始化最近访问功能
     */
    function init() {
        // 渲染最近访问列表
        renderRecentNotes();
        
        // 恢复折叠状态
        restoreCollapseState();
        
        // 如果当前页面是笔记页面，记录访问历史
        const currentPath = window.location.pathname;
        if (currentPath.startsWith('/doc/')) {
            // 从页面标题中提取笔记名称
            const titleElement = document.querySelector('h1, .page-title');
            const title = titleElement ? titleElement.textContent.trim() : document.title.split(' - ')[0];
            const path = currentPath.replace('/doc/', '');
            
            // 延迟记录，避免页面跳转时记录不准确
            setTimeout(() => {
                addRecentNote(title, path);
            }, 500);
        }
    }

    // 导出公共接口
    window.RecentNotes = {
        add: addRecentNote,
        get: getRecentNotes,
        clear: clearRecentNotes,
        toggle: toggleRecentPanel,
        init: init
    };

    // 页面加载完成后初始化
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
