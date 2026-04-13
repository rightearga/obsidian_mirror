// 收藏夹管理模块
(function() {
    const STORAGE_KEY = 'obsidian_mirror_favorites';

    /**
     * 获取收藏列表
     * @returns {Array} 收藏笔记数组，格式: [{title, path, timestamp}]
     */
    function getFavorites() {
        try {
            const data = localStorage.getItem(STORAGE_KEY);
            if (!data) return [];
            const favorites = JSON.parse(data);
            // 按添加时间倒序排序（最新的在前面）
            return favorites.sort((a, b) => b.timestamp - a.timestamp);
        } catch (e) {
            console.error('读取收藏列表失败:', e);
            return [];
        }
    }

    /**
     * 保存收藏列表
     * @param {Array} favorites - 收藏笔记数组
     */
    function saveFavorites(favorites) {
        try {
            localStorage.setItem(STORAGE_KEY, JSON.stringify(favorites));
        } catch (e) {
            console.error('保存收藏列表失败:', e);
        }
    }

    /**
     * 检查笔记是否已收藏
     * @param {string} path - 笔记路径
     * @returns {boolean} 是否已收藏
     */
    function isFavorited(path) {
        const favorites = getFavorites();
        return favorites.some(fav => fav.path === path);
    }

    /**
     * 添加笔记到收藏夹
     * @param {string} title - 笔记标题
     * @param {string} path - 笔记路径
     * @returns {boolean} 是否添加成功
     */
    function addFavorite(title, path) {
        if (!title || !path) return false;

        let favorites = getFavorites();
        
        // 检查是否已收藏
        if (favorites.some(fav => fav.path === path)) {
            console.log('笔记已在收藏夹中');
            return false;
        }
        
        // 添加到收藏夹
        favorites.unshift({
            title: title,
            path: path,
            timestamp: Date.now()
        });
        
        saveFavorites(favorites);
        
        // 更新 UI
        renderFavorites();
        updateFavoriteButton(path, true);
        
        // 显示提示
        showToast('已添加到收藏夹');
        
        return true;
    }

    /**
     * 从收藏夹移除笔记
     * @param {string} path - 笔记路径
     * @returns {boolean} 是否移除成功
     */
    function removeFavorite(path) {
        if (!path) return false;

        let favorites = getFavorites();
        const initialLength = favorites.length;
        
        // 移除指定笔记
        favorites = favorites.filter(fav => fav.path !== path);
        
        if (favorites.length === initialLength) {
            console.log('笔记不在收藏夹中');
            return false;
        }
        
        saveFavorites(favorites);
        
        // 更新 UI
        renderFavorites();
        updateFavoriteButton(path, false);
        
        // 显示提示
        showToast('已从收藏夹移除');
        
        return true;
    }

    /**
     * 切换笔记的收藏状态
     * @param {string} title - 笔记标题
     * @param {string} path - 笔记路径
     */
    function toggleFavorite(title, path) {
        if (isFavorited(path)) {
            removeFavorite(path);
        } else {
            addFavorite(title, path);
        }
    }

    /**
     * 清空收藏夹
     */
    function clearFavorites() {
        if (!confirm('确定要清空所有收藏吗？')) {
            return;
        }
        
        try {
            localStorage.removeItem(STORAGE_KEY);
            renderFavorites();
            
            // 更新当前页面的收藏按钮
            const currentPath = getCurrentPath();
            if (currentPath) {
                updateFavoriteButton(currentPath, false);
            }
            
            showToast('已清空收藏夹');
        } catch (e) {
            console.error('清空收藏夹失败:', e);
        }
    }

    /**
     * 渲染收藏夹列表到侧边栏
     */
    function renderFavorites() {
        const container = document.getElementById('favorites-container');
        if (!container) return;

        const favorites = getFavorites();
        
        if (favorites.length === 0) {
            container.innerHTML = `
                <div class="favorites-empty">
                    <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                        <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"></polygon>
                    </svg>
                    <p>暂无收藏</p>
                </div>
            `;
            return;
        }

        let html = '<div class="favorites-list">';
        favorites.forEach((fav, index) => {
            const addedTime = formatTimeAgo(fav.timestamp);
            html += `
                <div class="favorite-item">
                    <a href="/doc/${fav.path}" class="favorite-link" title="${fav.title}">
                        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="2">
                            <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"></polygon>
                        </svg>
                        <div class="favorite-content">
                            <div class="favorite-title">${escapeHtml(fav.title)}</div>
                            <div class="favorite-time">${addedTime}</div>
                        </div>
                    </a>
                    <button class="favorite-remove" onclick="Favorites.remove('${escapeAttribute(fav.path)}')" title="取消收藏">
                        <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <line x1="18" y1="6" x2="6" y2="18"></line>
                            <line x1="6" y1="6" x2="18" y2="18"></line>
                        </svg>
                    </button>
                </div>
            `;
        });
        html += '</div>';
        
        // 添加清空按钮
        html += `
            <button class="favorites-clear" onclick="Favorites.clear()">
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="3 6 5 6 21 6"></polyline>
                    <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                </svg>
                清空收藏
            </button>
        `;
        
        container.innerHTML = html;
    }

    /**
     * 更新笔记页面的收藏按钮状态
     * @param {string} path - 笔记路径
     * @param {boolean} favorited - 是否已收藏
     */
    function updateFavoriteButton(path, favorited) {
        const button = document.getElementById('favorite-button');
        if (!button) return;

        const icon = button.querySelector('.favorite-icon');
        const text = button.querySelector('.favorite-text');
        
        if (favorited) {
            button.classList.add('favorited');
            if (icon) icon.innerHTML = `
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="2">
                    <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"></polygon>
                </svg>
            `;
            if (text) text.textContent = '已收藏';
        } else {
            button.classList.remove('favorited');
            if (icon) icon.innerHTML = `
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"></polygon>
                </svg>
            `;
            if (text) text.textContent = '收藏';
        }
    }

    /**
     * 初始化收藏按钮
     */
    function initFavoriteButton() {
        const currentPath = getCurrentPath();
        if (!currentPath) return;

        const favorited = isFavorited(currentPath);
        updateFavoriteButton(currentPath, favorited);
    }

    /**
     * 获取当前页面的笔记路径
     * @returns {string|null} 笔记路径
     */
    function getCurrentPath() {
        const pathname = window.location.pathname;
        if (pathname.startsWith('/doc/')) {
            return pathname.replace('/doc/', '');
        }
        return null;
    }

    /**
     * 获取当前页面的笔记标题
     * @returns {string} 笔记标题
     */
    function getCurrentTitle() {
        const titleElement = document.querySelector('h1, .page-title, .markdown-content h1');
        if (titleElement) {
            return titleElement.textContent.trim();
        }
        return document.title.split(' - ')[0] || '未命名笔记';
    }

    /**
     * 切换收藏夹面板的展开/收起状态
     */
    function toggleFavoritesPanel() {
        const panel = document.getElementById('favorites-panel');
        if (!panel) return;
        
        panel.classList.toggle('collapsed');
        
        // 保存折叠状态
        const isCollapsed = panel.classList.contains('collapsed');
        try {
            localStorage.setItem('favorites_collapsed', isCollapsed ? 'true' : 'false');
        } catch (e) {
            console.error('保存折叠状态失败:', e);
        }
    }

    /**
     * 恢复折叠状态
     */
    function restoreCollapseState() {
        try {
            const isCollapsed = localStorage.getItem('favorites_collapsed') === 'true';
            const panel = document.getElementById('favorites-panel');
            if (panel && isCollapsed) {
                panel.classList.add('collapsed');
            }
        } catch (e) {
            console.error('恢复折叠状态失败:', e);
        }
    }

    /**
     * 显示提示消息
     * @param {string} message - 提示消息
     */
    function showToast(message) {
        // 查找或创建 toast 容器
        let toast = document.getElementById('toast-message');
        if (!toast) {
            toast = document.createElement('div');
            toast.id = 'toast-message';
            toast.className = 'toast-message';
            document.body.appendChild(toast);
        }
        
        toast.textContent = message;
        toast.classList.add('show');
        
        // 3 秒后自动隐藏
        setTimeout(() => {
            toast.classList.remove('show');
        }, 3000);
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
     * 属性值转义
     * @param {string} text - 原始文本
     * @returns {string} 转义后的文本
     */
    function escapeAttribute(text) {
        return text.replace(/'/g, "\\'").replace(/"/g, '&quot;');
    }

    /**
     * 初始化收藏夹功能
     */
    function init() {
        // 渲染收藏列表
        renderFavorites();
        
        // 恢复折叠状态
        restoreCollapseState();
        
        // 如果当前页面是笔记页面，初始化收藏按钮
        if (getCurrentPath()) {
            initFavoriteButton();
        }
    }

    // 导出公共接口
    window.Favorites = {
        add: addFavorite,
        remove: removeFavorite,
        toggle: toggleFavorite,
        isFavorited: isFavorited,
        get: getFavorites,
        clear: clearFavorites,
        togglePanel: toggleFavoritesPanel,
        init: init
    };

    // 页面加载完成后初始化
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
