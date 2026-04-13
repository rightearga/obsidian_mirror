/* ==========================================
   目录 (TOC) 交互逻辑
   ========================================== */

/**
 * 初始化 TOC 功能
 */
function initToc() {
    const rightSidebar = document.querySelector('.right-sidebar');
    if (!rightSidebar) return;

    // 初始化移动端状态
    initMobileTocState();
    
    // 初始化桌面端状态
    initDesktopTocState();

    // 添加滚动监听
    initScrollSpy();

    // 添加平滑滚动
    initSmoothScroll();
    
    // 初始化拖动调整宽度功能（仅桌面端）
    if (window.innerWidth > 768) {
        initTocResize();
    }
}

/**
 * 初始化移动端 TOC 状态
 */
function initMobileTocState() {
    const tocWrapper = document.querySelector('.toc-mobile .toc-wrapper');
    if (!tocWrapper) return;

    // 如果是移动端，默认收起 TOC
    if (window.innerWidth <= 768) {
        tocWrapper.classList.add('collapsed');
        const toggle = document.querySelector('.toc-mobile .toc-toggle');
        if (toggle) {
            toggle.classList.add('collapsed');
        }
    }
}

/**
 * 初始化桌面端 TOC 状态
 */
function initDesktopTocState() {
    const rightSidebar = document.querySelector('.right-sidebar');
    if (!rightSidebar) return;

    const STORAGE_KEY = 'obsidian_mirror_toc_collapsed';
    const savedState = localStorage.getItem(STORAGE_KEY);

    // 如果之前保存了收起状态，则恢复
    if (savedState === 'true') {
        rightSidebar.classList.add('collapsed');
    }
}

/**
 * 切换移动端 TOC 侧边栏
 */
function toggleMobileTocSidebar() {
    const sidebar = document.getElementById('toc-sidebar-mobile');
    const backdrop = document.getElementById('toc-sidebar-backdrop');
    
    if (!sidebar || !backdrop) return;
    
    sidebar.classList.toggle('active');
    backdrop.classList.toggle('active');
    
    // 防止背景滚动
    if (sidebar.classList.contains('active')) {
        document.body.classList.add('toc-expanded');
        document.documentElement.classList.add('toc-expanded'); // 同时锁定 html
    } else {
        document.body.classList.remove('toc-expanded');
        document.documentElement.classList.remove('toc-expanded');
    }
}

/**
 * 关闭移动端 TOC 侧边栏
 */
function closeMobileTocSidebar() {
    const sidebar = document.getElementById('toc-sidebar-mobile');
    const backdrop = document.getElementById('toc-sidebar-backdrop');
    
    if (!sidebar || !backdrop) return;
    
    sidebar.classList.remove('active');
    backdrop.classList.remove('active');
    document.body.classList.remove('toc-expanded');
    document.documentElement.classList.remove('toc-expanded');
}

/**
 * 处理移动端 TOC 链接点击
 */
function handleMobileTocClick(event) {
    // 获取目标元素
    const link = event.currentTarget;
    const targetId = link.getAttribute('data-target');
    const targetElement = document.getElementById(targetId);
    
    if (!targetElement) return;
    
    // 阻止默认行为
    event.preventDefault();
    
    // 关闭侧边栏
    closeMobileTocSidebar();
    
    // 等待侧边栏关闭动画完成后再滚动
    setTimeout(() => {
        const offset = 60; // 移动端头部高度偏移 + 缓冲
        // 移动端使用 window 滚动
        const targetTop = targetElement.offsetTop - offset;

        window.scrollTo({
            top: targetTop,
            behavior: 'smooth'
        });

        // 更新 URL hash
        if (history.pushState) {
            history.pushState(null, null, `#${targetId}`);
        }
    }, 300);
}


/**
 * 切换移动端 TOC 展开/收起
 */
function toggleToc() {
    const tocWrapper = document.querySelector('.toc-mobile .toc-wrapper');
    const toggle = document.querySelector('.toc-mobile .toc-toggle');
    
    if (!tocWrapper || !toggle) return;

    tocWrapper.classList.toggle('collapsed');
    toggle.classList.toggle('collapsed');
}

/**
 * 切换桌面端 TOC 展开/收起
 */
function toggleDesktopToc() {
    const rightSidebar = document.querySelector('.right-sidebar');
    if (!rightSidebar) return;

    const STORAGE_KEY = 'obsidian_mirror_toc_collapsed';
    
    rightSidebar.classList.toggle('collapsed');
    
    // 保存状态到 localStorage
    const isCollapsed = rightSidebar.classList.contains('collapsed');
    localStorage.setItem(STORAGE_KEY, isCollapsed.toString());
}

/**
 * 切换桌面端 TOC 页签（目录 / 反向链接）
 */
function switchTocTab(tabName) {
    // 切换页签按钮状态
    const tabs = document.querySelectorAll('.toc-tab');
    tabs.forEach(tab => {
        if (tab.getAttribute('data-tab') === tabName) {
            tab.classList.add('active');
        } else {
            tab.classList.remove('active');
        }
    });
    
    // 切换内容显示
    const contents = document.querySelectorAll('.toc-tab-content');
    contents.forEach(content => {
        if (content.getAttribute('data-tab-content') === tabName) {
            content.style.display = 'block';
        } else {
            content.style.display = 'none';
        }
    });
}

/**
 * 切换移动端 TOC 页签（目录 / 反向链接）
 */
function switchMobileTocTab(tabName) {
    // 切换页签按钮状态
    const tabs = document.querySelectorAll('.toc-mobile-tab');
    tabs.forEach(tab => {
        if (tab.getAttribute('data-tab') === tabName) {
            tab.classList.add('active');
        } else {
            tab.classList.remove('active');
        }
    });
    
    // 切换内容显示
    const contents = document.querySelectorAll('.toc-mobile-tab-content');
    contents.forEach(content => {
        if (content.getAttribute('data-tab-content') === tabName) {
            content.style.display = 'block';
        } else {
            content.style.display = 'none';
        }
    });
}

/**
 * 滚动监听 - 高亮当前可见的标题
 */
function initScrollSpy() {
    const tocLinks = document.querySelectorAll('.toc-link');
    if (tocLinks.length === 0) return;

    // 获取所有标题元素
    const headings = Array.from(tocLinks).map(link => {
        const target = link.getAttribute('data-target');
        return document.getElementById(target);
    }).filter(Boolean);

    if (headings.length === 0) return;

    // 获取实际的滚动容器
    // 移动端 (<=768px) 使用 window 滚动
    const isMobile = window.innerWidth <= 768;
    const scrollContainer = isMobile ? window : (document.querySelector('.page-scrollable-content') || document.querySelector('.main-scroll-container'));
    
    if (!scrollContainer) return;

    // 滚动处理函数（带防抖）
    let ticking = false;
    function onScroll() {
        if (!ticking) {
            window.requestAnimationFrame(() => {
                updateActiveLink(headings, tocLinks, scrollContainer);
                ticking = false;
            });
            ticking = true;
        }
    }

    // 添加滚动监听
    scrollContainer.addEventListener('scroll', onScroll, { passive: true });

    // 初始化时执行一次
    updateActiveLink(headings, tocLinks, scrollContainer);
}

/**
 * 更新激活的 TOC 链接
 */
function updateActiveLink(headings, tocLinks, scrollContainer) {
    const isWindow = scrollContainer === window;
    const scrollTop = isWindow ? (window.scrollY || document.documentElement.scrollTop) : scrollContainer.scrollTop;
    const scrollPos = scrollTop + 50; // 偏移量
    
    // 容器的顶部偏移量
    let containerOffsetTop = 0;
    if (!isWindow) {
        containerOffsetTop = scrollContainer.offsetTop;
    }

    // 找到当前可见的标题
    let activeIndex = -1;
    for (let i = 0; i < headings.length; i++) {
        const heading = headings[i];
        // 计算标题相对于滚动容器顶部的距离
        // 如果是 window 滚动，heading.offsetTop 就是相对于文档顶部的距离
        // 如果是容器滚动，需减去容器的 offsetTop
        const headingTop = heading.offsetTop - containerOffsetTop;
        
        if (headingTop <= scrollPos) {
            activeIndex = i;
        } else {
            break;
        }
    }

    // 更新激活状态
    tocLinks.forEach((link, index) => {
        if (index === activeIndex) {
            link.classList.add('active');
        } else {
            link.classList.remove('active');
        }
    });
}

/**
 * 平滑滚动到目标标题
 */
function initSmoothScroll() {
    const tocLinks = document.querySelectorAll('.toc-link');
    
    // 移动端检测
    const isMobile = window.innerWidth <= 768;
    const scrollContainer = isMobile ? window : (document.querySelector('.page-scrollable-content') || document.querySelector('.main-scroll-container'));
    
    if (!scrollContainer) return;
    
    tocLinks.forEach(link => {
        link.addEventListener('click', (e) => {
            // 如果绑定了 onclick (如移动端 handleMobileTocClick)，则跳过这里
            if (link.getAttribute('onclick')) return;

            e.preventDefault();
            
            const targetId = link.getAttribute('data-target');
            const targetElement = document.getElementById(targetId);
            
            if (targetElement) {
                const offset = 10;
                
                if (isMobile) {
                    const targetTop = targetElement.offsetTop - offset;
                    window.scrollTo({
                        top: targetTop,
                        behavior: 'smooth'
                    });
                } else {
                    // 容器内滚动
                    // 计算目标元素相对于滚动容器的位置
                    // 容器可能不是 relative 定位，offsetTop 是相对于 offsetParent
                    // 使用 getBoundingClientRect 更可靠
                    const containerRect = scrollContainer === window ? { top: 0 } : scrollContainer.getBoundingClientRect();
                    const targetRect = targetElement.getBoundingClientRect();
                    
                    // 当前 scrollTop
                    const currentScrollTop = scrollContainer.scrollTop;
                    
                    // 目标位置 = 当前位置 + (目标视口位置 - 容器视口位置) - 偏移
                    const scrollTop = currentScrollTop + targetRect.top - containerRect.top - offset;

                    scrollContainer.scrollTo({
                        top: scrollTop,
                        behavior: 'smooth'
                    });
                }

                // 更新 URL hash（不触发滚动）
                if (history.pushState) {
                    history.pushState(null, null, `#${targetId}`);
                }
            }
        });
    });
}

/**
 * 响应窗口大小变化
 */
window.addEventListener('resize', debounce(() => {
    const tocWrapperMobile = document.querySelector('.toc-mobile .toc-wrapper');
    const tocDesktop = document.querySelector('.toc-desktop');
    
    // 如果从移动端切换到桌面端，展开移动端 TOC
    if (window.innerWidth > 768 && tocWrapperMobile && tocWrapperMobile.classList.contains('collapsed')) {
        tocWrapperMobile.classList.remove('collapsed');
        const toggle = document.querySelector('.toc-mobile .toc-toggle');
        if (toggle) {
            toggle.classList.remove('collapsed');
        }
    }
    
    // 重新初始化拖动调整宽度功能（仅桌面端）
    if (window.innerWidth > 768 && tocDesktop && !tocDesktop.classList.contains('collapsed')) {
        // 仅恢复宽度，避免重复绑定事件
        restoreTocWidth();
        // 如果未初始化过事件监听，则初始化
        if (!document.getElementById('right-sidebar').dataset.resizeInitialized) {
            initTocResize();
        }
    }
}, 250));

/**
 * 恢复 TOC 宽度
 */
function restoreTocWidth() {
    const rightSidebar = document.getElementById('right-sidebar');
    if (!rightSidebar) return;

    const STORAGE_KEY = 'obsidian_mirror_toc_width';
    const MIN_WIDTH = 200;
    const MAX_WIDTH = 500;
    
    const savedWidth = localStorage.getItem(STORAGE_KEY);
    if (savedWidth) {
        const width = parseInt(savedWidth);
        if (width >= MIN_WIDTH && width <= MAX_WIDTH) {
            rightSidebar.style.width = width + 'px';
        }
    }
}

/**
 * 初始化 TOC 宽度调整功能
 */
function initTocResize() {
    const rightSidebar = document.getElementById('right-sidebar');
    const resizeHandle = document.getElementById('right-sidebar-resizer');
    
    if (!rightSidebar || !resizeHandle) return;
    
    // 标记已初始化
    rightSidebar.dataset.resizeInitialized = 'true';
    
    const STORAGE_KEY = 'obsidian_mirror_toc_width';
    const MIN_WIDTH = 200;
    const MAX_WIDTH = 500;
    
    // 首次初始化时尝试恢复宽度
    restoreTocWidth();
    
    let isDragging = false;
    let startX = 0;
    let startWidth = 0;
    
    // 鼠标按下
    resizeHandle.addEventListener('mousedown', (e) => {
        isDragging = true;
        startX = e.clientX;
        startWidth = rightSidebar.offsetWidth;
        
        rightSidebar.classList.add('resizing'); // 添加 resizing 类以禁用动画
        resizeHandle.classList.add('resizing');
        document.body.style.cursor = 'ew-resize';
        document.body.style.userSelect = 'none';
        
        e.preventDefault();
    });
    
    // 鼠标移动
    document.addEventListener('mousemove', (e) => {
        if (!isDragging) return;
        
        // 计算新宽度（注意：右侧栏在右侧，向左拖动增加宽度）
        const deltaX = startX - e.clientX;
        let newWidth = startWidth + deltaX;
        
        // 限制宽度范围
        newWidth = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, newWidth));
        
        // 应用新宽度
        rightSidebar.style.width = newWidth + 'px';
    });
    
    // 鼠标释放
    document.addEventListener('mouseup', () => {
        if (!isDragging) return;
        
        isDragging = false;
        rightSidebar.classList.remove('resizing'); // 移除 resizing 类
        resizeHandle.classList.remove('resizing');
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
        
        // 保存宽度到 localStorage
        // 优先使用 style.width 以避免 CSS 动画导致的计算滞后，回退到 offsetWidth
        let widthToSave = rightSidebar.style.width ? parseInt(rightSidebar.style.width) : rightSidebar.offsetWidth;
        
        // 确保保存的值是有效的数字
        if (isNaN(widthToSave)) {
            widthToSave = rightSidebar.offsetWidth;
        }
        
        localStorage.setItem(STORAGE_KEY, widthToSave.toString());
    });
}

// 将函数导出到全局，以便在 HTML 中调用
if (typeof window !== 'undefined') {
    window.initToc = initToc;
    window.toggleToc = toggleToc;
    window.toggleDesktopToc = toggleDesktopToc;
    window.switchTocTab = switchTocTab;
    window.switchMobileTocTab = switchMobileTocTab;
    window.toggleMobileTocSidebar = toggleMobileTocSidebar;
    window.closeMobileTocSidebar = closeMobileTocSidebar;
    window.handleMobileTocClick = handleMobileTocClick;
}
