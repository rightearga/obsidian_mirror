// ==========================================
// 侧边栏管理模块
// ==========================================

const STORAGE_KEY = 'obsidian_mirror_collapsed';
const SIDEBAR_KEY = 'obsidian_mirror_sidebar_state';
const SIDEBAR_SCROLL_KEY = 'obsidian_mirror_sidebar_scroll';

// ===== 文件树折叠状态管理 =====

/**
 * 获取已折叠的路径列表
 */
function getCollapsedPaths() {
    try {
        return JSON.parse(localStorage.getItem(STORAGE_KEY) || '[]');
    } catch {
        return [];
    }
}

/**
 * 保存路径的折叠状态
 */
function saveCollapsedPath(path, isCollapsed) {
    let paths = getCollapsedPaths();
    const set = new Set(paths);
    if (isCollapsed) {
        set.add(path);
    } else {
        set.delete(path);
    }
    try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(Array.from(set)));
    } catch (e) {}
}

/**
 * 切换文件夹的展开/折叠状态
 */
function toggleFolderFlat(element) {
    const row = element.parentElement;
    const depth = parseInt(row.getAttribute('data-depth'));
    const icon = element.querySelector('.tree-icon');
    const label = element.querySelector('.label').textContent.trim();
    const pathId = label + '_' + depth;

    if (icon.classList.contains('rotated')) {
        icon.classList.remove('rotated');
        saveCollapsedPath(pathId, true);
        setChildrenVisible(row, depth, false);
    } else {
        icon.classList.add('rotated');
        saveCollapsedPath(pathId, false);
        setChildrenVisible(row, depth, true);
    }
}

/**
 * 设置子节点的可见性
 */
function setChildrenVisible(row, depth, visible) {
    let next = row.nextElementSibling;
    while (next) {
        const nextDepth = parseInt(next.getAttribute('data-depth'));
        if (nextDepth <= depth) break;
        
        if (!visible) {
            next.style.display = 'none';
        } else {
            next.style.display = 'block';
        }
        
        next = next.nextElementSibling;
    }
    
    if (visible) {
        restoreStateForRange(row, depth);
    }
}

/**
 * 恢复指定范围内节点的折叠状态
 */
function restoreStateForRange(startRow, parentDepth) {
    const collapsed = new Set(getCollapsedPaths());
    let next = startRow.nextElementSibling;
    while(next) {
        const nextDepth = parseInt(next.getAttribute('data-depth'));
        if (nextDepth <= parentDepth) break;
        
        const folderBtn = next.querySelector('.tree-row.folder');
        if (folderBtn) {
            const label = folderBtn.querySelector('.label').textContent.trim();
            const pathId = label + '_' + nextDepth;
            if (collapsed.has(pathId)) {
                const icon = folderBtn.querySelector('.tree-icon');
                icon.classList.remove('rotated');
                setChildrenVisible(next, nextDepth, false);
            }
        }
        next = next.nextElementSibling;
    }
}

// ===== 侧边栏显示/隐藏状态管理 =====

/**
 * 保存侧边栏滚动位置
 */
function saveSidebarScroll() {
    const sidebar = document.querySelector('.sidebar');
    if (sidebar) {
        try {
            localStorage.setItem(SIDEBAR_SCROLL_KEY, sidebar.scrollTop.toString());
        } catch(e) {}
    }
}

/**
 * 恢复侧边栏滚动位置
 */
function restoreSidebarScroll() {
    const sidebar = document.querySelector('.sidebar');
    if (sidebar) {
        try {
            const scrollPos = localStorage.getItem(SIDEBAR_SCROLL_KEY);
            if (scrollPos) {
                sidebar.scrollTop = parseInt(scrollPos, 10);
            }
        } catch(e) {}
    }
}

/**
 * 切换侧边栏展开/折叠状态
 */
function toggleSidebarState(e) {
    if (e) {
        e.stopPropagation();
        e.preventDefault();
    }

    const body = document.body;
    const isMobile = window.innerWidth <= 768;

    if (isMobile) {
        const wasExpanded = body.classList.contains('sidebar-expanded');
        body.classList.toggle('sidebar-expanded');
        // 同时给 html 添加类，增强滚动锁定
        document.documentElement.classList.toggle('sidebar-expanded');
        
        const isExpanded = body.classList.contains('sidebar-expanded');
        const state = isExpanded ? 'open' : 'closed';
        try {
            localStorage.setItem(SIDEBAR_KEY, state);
        } catch(e) {}

        // 关闭时保存滚动位置，打开时恢复滚动位置
        if (wasExpanded && !isExpanded) {
            saveSidebarScroll();
        } else if (!wasExpanded && isExpanded) {
            setTimeout(restoreSidebarScroll, 50);
        }
    } else {
        // PC模式：在折叠前保存滚动位置
        const wasCollapsed = body.classList.contains('sidebar-collapsed');
        if (!wasCollapsed) {
            saveSidebarScroll();
        }
        
        body.classList.toggle('sidebar-collapsed');
        const state = body.classList.contains('sidebar-collapsed') ? 'closed' : 'open';
        try {
            localStorage.setItem(SIDEBAR_KEY, state);
        } catch(e) {}

        // PC模式：在展开后恢复滚动位置
        if (wasCollapsed) {
            setTimeout(restoreSidebarScroll, 50);
        }
    }
}

// ===== 同步功能 =====

/**
 * 触发 Git 同步
 */
async function triggerSync() {
    const btn = document.querySelector('.sync-btn');
    const btnText = btn.querySelector('.btn-text');

    if (btn.disabled) return;

    btnText.textContent = '同步中...';
    btn.classList.add('loading');
    btn.disabled = true;
    
    // 触发同步开始事件
    window.dispatchEvent(new Event('sync:start'));

    try {
        const res = await fetch('/sync', { method: 'POST' });
        if (res.ok) {
            // 触发同步成功事件
            window.dispatchEvent(new Event('sync:success'));
            location.reload();
        } else {
            // 触发同步失败事件
            window.dispatchEvent(new Event('sync:error'));
            alert('同步失败');
        }
    } catch (e) {
        // 触发同步失败事件
        window.dispatchEvent(new Event('sync:error'));
        alert('错误：' + e);
    } finally {
        btnText.textContent = '同步';
        btn.classList.remove('loading');
        btn.disabled = false;
    }
}

// ===== 侧边栏拖动调整大小 =====

const SIDEBAR_WIDTH_KEY = 'obsidian_mirror_sidebar_width';
const MIN_SIDEBAR_WIDTH = 200;
const MAX_SIDEBAR_WIDTH = 600;

/**
 * 初始化侧边栏拖动调整大小功能
 */
function initSidebarResize() {
    const sidebar = document.querySelector('.sidebar');
    const resizer = document.getElementById('sidebar-resizer');
    
    if (!sidebar || !resizer) {
        console.error('侧边栏拖动初始化失败:', {
            sidebar: !!sidebar,
            resizer: !!resizer
        });
        return;
    }
    
    console.log('侧边栏拖动功能已初始化');
    
    // 恢复保存的侧边栏宽度
    try {
        const savedWidth = localStorage.getItem(SIDEBAR_WIDTH_KEY);
        if (savedWidth) {
            const width = parseInt(savedWidth, 10);
            if (width >= MIN_SIDEBAR_WIDTH && width <= MAX_SIDEBAR_WIDTH) {
                sidebar.style.width = width + 'px';
                console.log('恢复侧边栏宽度:', width);
            }
        }
    } catch (e) {
        console.error('恢复侧边栏宽度失败:', e);
    }
    
    let isResizing = false;
    let startX = 0;
    let startWidth = 0;
    
    // 鼠标按下开始拖动
    resizer.addEventListener('mousedown', function(e) {
        console.log('开始拖动侧边栏');
        isResizing = true;
        startX = e.clientX;
        startWidth = sidebar.offsetWidth;
        
        sidebar.classList.add('resizing');
        resizer.classList.add('resizing');
        document.body.style.cursor = 'col-resize';
        document.body.style.userSelect = 'none';
        
        e.preventDefault();
    });
    
    // 鼠标移动调整大小
    document.addEventListener('mousemove', function(e) {
        if (!isResizing) return;
        
        const deltaX = e.clientX - startX;
        let newWidth = startWidth + deltaX;
        
        // 限制宽度范围
        newWidth = Math.max(MIN_SIDEBAR_WIDTH, Math.min(MAX_SIDEBAR_WIDTH, newWidth));
        
        sidebar.style.width = newWidth + 'px';
        
        e.preventDefault();
    });
    
    // 鼠标释放停止拖动
    document.addEventListener('mouseup', function(e) {
        if (!isResizing) return;
        
        console.log('停止拖动侧边栏，最终宽度:', sidebar.offsetWidth);
        isResizing = false;
        sidebar.classList.remove('resizing');
        resizer.classList.remove('resizing');
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
        
        // 保存侧边栏宽度
        try {
            localStorage.setItem(SIDEBAR_WIDTH_KEY, sidebar.offsetWidth.toString());
        } catch (e) {
            console.error('保存侧边栏宽度失败:', e);
        }
    });
}

// ==========================================
// 侧边栏页签切换
// ==========================================

/**
 * 切换侧边栏页签
 */
function switchSidebarTab(tabName) {
    // 移除所有页签的 active 状态
    document.querySelectorAll('.sidebar-tab').forEach(tab => {
        tab.classList.remove('active');
    });
    
    // 隐藏所有页签内容
    document.querySelectorAll('.sidebar-tab-content').forEach(content => {
        content.classList.remove('active');
        content.style.display = 'none';
    });
    
    // 激活当前页签
    const activeTab = document.querySelector(`.sidebar-tab[data-tab="${tabName}"]`);
    if (activeTab) {
        activeTab.classList.add('active');
    }
    
    // 显示对应内容
    const activeContent = document.getElementById(`tab-${tabName}`);
    if (activeContent) {
        activeContent.classList.add('active');
        activeContent.style.display = 'flex';
    }
    
    // 保存当前页签选择到 localStorage
    localStorage.setItem('sidebarActiveTab', tabName);
    
    console.log('切换到页签:', tabName);
}

/**
 * 页面加载时恢复上次选择的页签
 */
function initSidebarTabs() {
    // 从 localStorage 读取上次选择的页签
    const savedTab = localStorage.getItem('sidebarActiveTab') || 'notes';
    
    // 切换到保存的页签
    switchSidebarTab(savedTab);
}

// 页面加载完成后初始化页签
document.addEventListener('DOMContentLoaded', initSidebarTabs);
