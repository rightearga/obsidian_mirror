// 笔记统计模块
(function() {
    /**
     * 加载统计数据
     */
    async function loadStats() {
        try {
            const response = await fetch('/api/stats');
            if (!response.ok) {
                throw new Error('加载统计信息失败');
            }
            
            const stats = await response.json();
            renderStats(stats);
        } catch (e) {
            console.error('加载统计信息失败:', e);
            renderError();
        }
    }

    /**
     * 渲染统计信息
     * @param {Object} stats - 统计数据
     */
    function renderStats(stats) {
        const container = document.getElementById('stats-container');
        if (!container) return;

        const html = `
            <div class="stat-item">
                <div class="stat-icon">
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                        <polyline points="14 2 14 8 20 8"></polyline>
                    </svg>
                </div>
                <div class="stat-content">
                    <div class="stat-value">${stats.total_notes}</div>
                    <div class="stat-label">笔记总数</div>
                </div>
            </div>
            
            <div class="stat-item">
                <div class="stat-icon">
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z"></path>
                        <line x1="7" y1="7" x2="7.01" y2="7"></line>
                    </svg>
                </div>
                <div class="stat-content">
                    <div class="stat-value">${stats.total_tags}</div>
                    <div class="stat-label">标签数量</div>
                </div>
            </div>
            
            <div class="stat-item">
                <div class="stat-icon">
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline>
                    </svg>
                </div>
                <div class="stat-content">
                    <div class="stat-value">${stats.recent_updated}</div>
                    <div class="stat-label">最近更新</div>
                </div>
            </div>
        `;
        
        container.innerHTML = html;
    }

    /**
     * 渲染错误状态
     */
    function renderError() {
        const container = document.getElementById('stats-container');
        if (!container) return;

        container.innerHTML = `
            <div class="stats-error">
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="10"></circle>
                    <line x1="12" y1="8" x2="12" y2="12"></line>
                    <line x1="12" y1="16" x2="12.01" y2="16"></line>
                </svg>
                <span>加载失败</span>
            </div>
        `;
    }

    /**
     * 初始化统计面板
     */
    function init() {
        loadStats();
    }

    // 导出公共接口
    window.Stats = {
        load: loadStats,
        init: init
    };

    // 页面加载完成后初始化
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
