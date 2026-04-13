/**
 * 笔记预览功能
 * 
 * 功能：
 * - 鼠标悬停在链接上时显示预览卡片
 * - 延迟显示（避免误触）
 * - 智能定位（避免超出视口）
 * - 预加载和缓存优化
 */

const NotePreview = {
    // 配置
    config: {
        showDelay: 500,        // 显示延迟（毫秒）
        hideDelay: 200,        // 隐藏延迟（毫秒）
        maxCacheSize: 50,      // 最大缓存条目数
        cardOffset: 10,        // 卡片与鼠标的距离
    },
    
    // 状态
    state: {
        showTimer: null,       // 显示定时器
        hideTimer: null,       // 隐藏定时器
        currentLink: null,     // 当前悬停的链接
        isCardHovered: false,  // 鼠标是否在卡片上
        isLinkHovered: false,  // 鼠标是否在链接上
    },
    
    // 缓存
    cache: new Map(),
    
    // DOM 元素
    elements: {
        card: null,
        title: null,
        content: null,
    },
    
    /**
     * 初始化预览功能
     */
    init() {
        // 获取 DOM 元素
        this.elements.card = document.getElementById('note-preview-card');
        this.elements.title = document.getElementById('note-preview-title');
        this.elements.content = document.getElementById('note-preview-content');
        
        if (!this.elements.card) {
            console.error('预览卡片元素未找到');
            return;
        }
        
        // 绑定事件
        this.bindEvents();
        
        console.log('笔记预览功能已初始化');
    },
    
    /**
     * 绑定事件监听器
     */
    bindEvents() {
        // 使用事件委托监听所有链接
        document.addEventListener('mouseover', (e) => {
            const link = e.target.closest('a[href^="/doc/"]');
            if (link) {
                this.onLinkEnter(link, e);
            }
        });
        
        document.addEventListener('mouseout', (e) => {
            const link = e.target.closest('a[href^="/doc/"]');
            if (link) {
                this.onLinkLeave(link);
            }
        });
        
        // 卡片悬停事件（防止鼠标移动到卡片上时消失）
        this.elements.card.addEventListener('mouseenter', () => {
            this.state.isCardHovered = true;
            this.cancelHide();
        });
        
        this.elements.card.addEventListener('mouseleave', () => {
            this.state.isCardHovered = false;
            this.scheduleHide();
        });
    },
    
    /**
     * 鼠标进入链接
     */
    onLinkEnter(link, event) {
        this.state.isLinkHovered = true;
        this.state.currentLink = link;
        
        // 取消之前的隐藏计划
        this.cancelHide();
        
        // 延迟显示预览
        this.state.showTimer = setTimeout(() => {
            this.showPreview(link, event);
        }, this.config.showDelay);
    },
    
    /**
     * 鼠标离开链接
     */
    onLinkLeave(link) {
        this.state.isLinkHovered = false;
        
        // 取消显示计划
        if (this.state.showTimer) {
            clearTimeout(this.state.showTimer);
            this.state.showTimer = null;
        }
        
        // 如果鼠标不在卡片上，计划隐藏
        if (!this.state.isCardHovered) {
            this.scheduleHide();
        }
    },
    
    /**
     * 显示预览卡片
     */
    async showPreview(link, event) {
        // 提取笔记路径
        const href = link.getAttribute('href');
        const path = href.replace('/doc/', '');
        
        // 定位卡片
        this.positionCard(event);
        
        // 显示加载状态
        this.showLoading();
        this.elements.card.classList.add('visible');
        
        try {
            // 获取预览内容（使用缓存）
            const preview = await this.fetchPreview(path);
            
            // 更新卡片内容
            this.elements.title.textContent = preview.title;
            this.elements.content.innerHTML = preview.content;
            
        } catch (error) {
            console.error('获取预览失败:', error);
            this.showError();
        }
    },
    
    /**
     * 隐藏预览卡片
     */
    hidePreview() {
        this.elements.card.classList.remove('visible');
        this.state.currentLink = null;
    },
    
    /**
     * 计划隐藏卡片
     */
    scheduleHide() {
        this.state.hideTimer = setTimeout(() => {
            if (!this.state.isLinkHovered && !this.state.isCardHovered) {
                this.hidePreview();
            }
        }, this.config.hideDelay);
    },
    
    /**
     * 取消隐藏计划
     */
    cancelHide() {
        if (this.state.hideTimer) {
            clearTimeout(this.state.hideTimer);
            this.state.hideTimer = null;
        }
    },
    
    /**
     * 定位卡片
     */
    positionCard(event) {
        const card = this.elements.card;
        const offset = this.config.cardOffset;
        
        // 获取视口尺寸
        const viewportWidth = window.innerWidth;
        const viewportHeight = window.innerHeight;
        
        // 获取卡片尺寸（使用默认尺寸，因为此时还未显示）
        const cardWidth = 500;  // 与 CSS 中的 max-width 一致
        const cardHeight = 400; // 与 CSS 中的 max-height 一致
        
        // 计算初始位置（鼠标右下方）
        let left = event.clientX + offset;
        let top = event.clientY + offset;
        
        // 检查右边界
        if (left + cardWidth > viewportWidth) {
            // 移到鼠标左边
            left = event.clientX - cardWidth - offset;
        }
        
        // 检查下边界
        if (top + cardHeight > viewportHeight) {
            // 移到鼠标上方
            top = event.clientY - cardHeight - offset;
        }
        
        // 确保不超出左边界和上边界
        left = Math.max(offset, left);
        top = Math.max(offset, top);
        
        // 应用位置
        card.style.left = `${left}px`;
        card.style.top = `${top}px`;
    },
    
    /**
     * 获取预览内容（带缓存）
     */
    async fetchPreview(path) {
        // 检查缓存
        if (this.cache.has(path)) {
            return this.cache.get(path);
        }
        
        // 请求 API
        const response = await fetch(`/api/preview?path=${encodeURIComponent(path)}`);
        
        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`);
        }
        
        const preview = await response.json();
        
        // 添加到缓存
        this.addToCache(path, preview);
        
        return preview;
    },
    
    /**
     * 添加到缓存
     */
    addToCache(path, preview) {
        // 如果缓存已满，删除最旧的条目
        if (this.cache.size >= this.config.maxCacheSize) {
            const firstKey = this.cache.keys().next().value;
            this.cache.delete(firstKey);
        }
        
        this.cache.set(path, preview);
    },
    
    /**
     * 清除缓存
     */
    clearCache() {
        this.cache.clear();
    },
    
    /**
     * 显示加载状态
     */
    showLoading() {
        this.elements.title.textContent = '加载中...';
        this.elements.content.innerHTML = '<div class="note-preview-loading"></div>';
    },
    
    /**
     * 显示错误状态
     */
    showError() {
        this.elements.title.textContent = '加载失败';
        this.elements.content.innerHTML = `
            <div class="note-preview-error">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="10"></circle>
                    <line x1="12" y1="8" x2="12" y2="12"></line>
                    <line x1="12" y1="16" x2="12.01" y2="16"></line>
                </svg>
                <p>无法加载预览</p>
            </div>
        `;
    },
};

// 在页面加载完成后初始化
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => NotePreview.init());
} else {
    NotePreview.init();
}
