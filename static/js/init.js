// ==========================================
// 页面初始化模块
// ==========================================

/**
 * 初始化代码高亮
 */
function initCodeHighlight() {
    hljs.configure({
        ignoreUnescapedHTML: true,
        languages: ['glsl', 'rust', 'python', 'cpp', 'java', 'go', 'bash', 'json', 'yaml', 'sql', 'javascript', 'typescript', 'html', 'css']
    });

    document.querySelectorAll('pre code').forEach((el) => {
        hljs.highlightElement(el);
    });
}

/**
 * 为表格添加包装器，防止溢出
 */
function wrapTables() {
    const tables = document.querySelectorAll('.markdown-body > table');
    tables.forEach(table => {
        // 检查是否已经被包装
        if (table.parentElement.classList.contains('table-wrapper')) {
            return;
        }
        
        // 创建包装器
        const wrapper = document.createElement('div');
        wrapper.className = 'table-wrapper';
        
        // 将表格包装起来
        table.parentNode.insertBefore(wrapper, table);
        wrapper.appendChild(table);
    });
}

/**
 * 初始化侧边栏状态
 */
function initSidebarState() {
    const isMobile = window.innerWidth <= 768;
    let sidebarState = null;
    
    try {
        sidebarState = localStorage.getItem(SIDEBAR_KEY);
    } catch(e) {}

    if (isMobile) {
        if (sidebarState === 'open') {
            document.body.classList.add('sidebar-expanded');
            setTimeout(restoreSidebarScroll, 50);
        }

        // 点击主内容区域时关闭侧边栏
        const sidebar = document.querySelector('.sidebar');
        const mainContent = document.querySelector('.main-content');

        if (sidebar) {
            sidebar.addEventListener('click', (e) => e.stopPropagation());
        }

        if (mainContent) {
            mainContent.addEventListener('click', (e) => {
                if (e.target.closest('.sidebar-toggle')) return;

                if (document.body.classList.contains('sidebar-expanded')) {
                    saveSidebarScroll();
                    document.body.classList.remove('sidebar-expanded');
                    try {
                        localStorage.setItem(SIDEBAR_KEY, 'closed');
                    } catch(e) {}
                }
            });
        }
    } else {
        if (sidebarState === 'closed') {
            document.body.classList.add('sidebar-collapsed');
        } else {
            setTimeout(restoreSidebarScroll, 50);
        }
    }
}

/**
 * 初始化文件树状态
 */
function initFileTree() {
    const collapsed = new Set(getCollapsedPaths());
    const folders = document.querySelectorAll('.tree-row.folder');
    
    const AUTO_COLLAPSE_DEPTH = 2;
    const totalNodes = document.querySelectorAll('.tree-item').length;
    const isLargeTree = totalNodes > 500;
    
    if (isLargeTree) {
        console.log(`检测到大型文件树（${totalNodes} 个节点），启用性能优化`);
    }
    
    folders.forEach(folder => {
        const row = folder.parentElement;
        const label = folder.querySelector('.label').textContent.trim();
        const depth = parseInt(row.getAttribute('data-depth'));
        const pathId = label + '_' + depth;
        const icon = folder.querySelector('.tree-icon');
        
        let shouldCollapse = collapsed.has(pathId);
        
        if (isLargeTree && depth >= AUTO_COLLAPSE_DEPTH && !collapsed.has(pathId)) {
            const allCollapsed = getCollapsedPaths();
            const wasExplicitlyExpanded = false;
            if (!wasExplicitlyExpanded) {
                shouldCollapse = true;
            }
        }
        
        if (shouldCollapse) {
            icon.classList.remove('rotated');
            setChildrenVisible(row, depth, false);
        } else {
            icon.classList.add('rotated');
        }
    });
}

/**
 * 初始化当前文件高亮和父目录展开
 */
function initActiveFile() {
    const currentPath = window.location.pathname;
    const isMobile = window.innerWidth <= 768;
    const links = document.querySelectorAll('.tree-row.file');
    
    links.forEach(link => {
        if (link.getAttribute('href') === percentDecode(currentPath)) {
            link.classList.add('active');

            // 展开所有父目录
            let prev = link.parentElement.previousElementSibling;
            const myDepth = parseInt(link.parentElement.getAttribute('data-depth'));
            let currentDepth = myDepth;

            while (prev) {
                const prevDepth = parseInt(prev.getAttribute('data-depth'));
                if (prevDepth < currentDepth) {
                    const folderRow = prev.querySelector('.tree-row.folder');
                    if (folderRow) {
                        const icon = folderRow.querySelector('.tree-icon');
                        if (!icon.classList.contains('rotated')) {
                            toggleFolderFlat(folderRow);
                        }
                    }
                    currentDepth = prevDepth;
                }
                prev = prev.previousElementSibling;
            }
        }

        // 点击文件链接时保存滚动位置
        if (isMobile) {
            link.addEventListener('click', (e) => {
                saveSidebarScroll();
                document.body.classList.remove('sidebar-expanded');
                try {
                    localStorage.setItem(SIDEBAR_KEY, 'closed');
                } catch(e) {}
            });
        } else {
            link.addEventListener('click', (e) => {
                saveSidebarScroll();
            });
        }
    });
}

// ===== DOM 加载完成后的初始化 =====
document.addEventListener('DOMContentLoaded', () => {
    // 确保 i18n 已初始化后再初始化其他模块
    if (window.i18n) {
        window.i18n.init();
        window.i18n.translatePage();
    }

    // 初始化代码高亮
    initCodeHighlight();
    
    // 为表格添加包装器
    wrapTables();

    // 初始化侧边栏切换按钮
    const sidebarToggleBtn = document.querySelector('.sidebar-toggle');
    if (sidebarToggleBtn) {
        sidebarToggleBtn.addEventListener('click', toggleSidebarState);
    }

    // 恢复侧边栏状态
    initSidebarState();
    
    // 初始化侧边栏拖动调整大小
    initSidebarResize();

    // 恢复文件树状态
    initFileTree();

    // 高亮当前文件并展开父目录
    initActiveFile();

    // 初始化搜索弹窗
    initSearchModal();

    // 初始化 TOC 功能
    if (typeof initToc === 'function') {
        initToc();
    }
    
    // 如果存在内层滚动区域，则标记页面类型，交给内层处理滚动条
    if (document.querySelector('.page-scrollable-content')) {
        document.body.classList.add('page-has-inner-scroll');
    }
    
    // 监听语言变更事件，重新翻译页面
    window.addEventListener('languageChanged', () => {
        window.i18n.translatePage();
    });
});
