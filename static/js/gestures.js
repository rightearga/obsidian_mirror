// ==========================================
// 触屏手势模块 — v1.4.4
// 侧边栏边缘滑动开关 + 内容区翻页手势
// ==========================================

(function () {
    'use strict';

    // ==========================================
    // 配置常量
    // ==========================================
    const EDGE_TRIGGER_WIDTH = 30;      // 左边缘触发区域宽度（px）
    const MIN_SWIPE_DISTANCE = 60;      // 最小有效滑动距离（px）
    const PAGE_FLIP_THRESHOLD = 100;    // 翻页手势触发阈值（px）
    const MAX_VERTICAL_DRIFT = 60;      // 水平滑动允许的最大垂直偏移（px）

    // ==========================================
    // 状态
    // ==========================================
    let touchStartX = 0;
    let touchStartY = 0;
    let touchStartTime = 0;
    let isSidebarGesture = false;   // 是否是侧边栏手势
    let isPageFlipGesture = false;  // 是否是翻页手势
    let atScrollTop = false;        // 内容区是否在顶部
    let atScrollBottom = false;     // 内容区是否在底部

    // ==========================================
    // 初始化
    // ==========================================
    function init() {
        document.addEventListener('touchstart', onTouchStart, { passive: true });
        document.addEventListener('touchmove', onTouchMove, { passive: false });
        document.addEventListener('touchend', onTouchEnd, { passive: true });

        // 监听内容区滚动位置
        const scrollable = getScrollContainer();
        if (scrollable) {
            scrollable.addEventListener('scroll', updateScrollState, { passive: true });
            updateScrollState();
        }
    }

    /** 获取内容滚动容器 */
    function getScrollContainer() {
        return document.querySelector('.page-scrollable-content') || null;
    }

    /** 更新滚动位置状态 */
    function updateScrollState() {
        const el = getScrollContainer();
        if (!el) return;
        const threshold = 10;
        atScrollTop = el.scrollTop <= threshold;
        atScrollBottom = (el.scrollHeight - el.scrollTop - el.clientHeight) <= threshold;
    }

    // ==========================================
    // 触摸事件处理
    // ==========================================

    function onTouchStart(e) {
        if (e.touches.length !== 1) return;
        const touch = e.touches[0];
        touchStartX = touch.clientX;
        touchStartY = touch.clientY;
        touchStartTime = Date.now();
        isSidebarGesture = false;
        isPageFlipGesture = false;

        // 判断是否从左边缘开始（侧边栏开启手势）
        if (touchStartX <= EDGE_TRIGGER_WIDTH) {
            isSidebarGesture = true;
        }

        // 判断是否在内容区（翻页手势）
        const scrollable = getScrollContainer();
        if (scrollable) {
            updateScrollState();
            const rect = scrollable.getBoundingClientRect();
            const inContent = (
                touch.clientX >= rect.left && touch.clientX <= rect.right &&
                touch.clientY >= rect.top && touch.clientY <= rect.bottom
            );
            isPageFlipGesture = inContent;
        }
    }

    function onTouchMove(e) {
        if (e.touches.length !== 1) return;
        const touch = e.touches[0];
        const deltaX = touch.clientX - touchStartX;
        const deltaY = touch.clientY - touchStartY;

        // 侧边栏开启手势：阻止默认滚动防止抖动
        if (isSidebarGesture && Math.abs(deltaX) > Math.abs(deltaY)) {
            e.preventDefault();
        }
    }

    function onTouchEnd(e) {
        if (e.changedTouches.length !== 1) return;
        const touch = e.changedTouches[0];
        const deltaX = touch.clientX - touchStartX;
        const deltaY = touch.clientY - touchStartY;
        const duration = Date.now() - touchStartTime;
        const absDeltaX = Math.abs(deltaX);
        const absDeltaY = Math.abs(deltaY);

        // 速度过慢（>600ms）不触发
        if (duration > 600) return;

        // ---- 侧边栏手势 ----
        if (isSidebarGesture && absDeltaX >= MIN_SWIPE_DISTANCE && absDeltaX > absDeltaY) {
            if (deltaX > 0) {
                openSidebar();
            }
            return;
        }

        // 侧边栏任意位置左滑关闭
        const isSidebarOpen = document.body.classList.contains('sidebar-expanded');
        if (isSidebarOpen && deltaX < -MIN_SWIPE_DISTANCE && absDeltaX > absDeltaY) {
            closeSidebar();
            return;
        }

        // ---- 翻页手势 ----
        if (!isPageFlipGesture) return;
        if (absDeltaY < PAGE_FLIP_THRESHOLD || absDeltaX > MAX_VERTICAL_DRIFT) return;

        if (deltaY < 0 && atScrollBottom) {
            // 向上滑（已在底部） → 下一篇笔记
            navigateAdjacentNote(1);
        } else if (deltaY > 0 && atScrollTop) {
            // 向下滑（已在顶部） → 上一篇笔记
            navigateAdjacentNote(-1);
        }
    }

    // ==========================================
    // 侧边栏开关
    // ==========================================

    function openSidebar() {
        if (document.body.classList.contains('sidebar-expanded')) return;
        document.body.classList.add('sidebar-expanded');
        try { localStorage.setItem('obsidian_mirror_sidebar_state', 'open'); } catch (e) {}
    }

    function closeSidebar() {
        if (!document.body.classList.contains('sidebar-expanded')) return;
        document.body.classList.remove('sidebar-expanded');
        try { localStorage.setItem('obsidian_mirror_sidebar_state', 'closed'); } catch (e) {}
    }

    // ==========================================
    // 笔记导航（复用 keyboard.js 的逻辑）
    // ==========================================

    function navigateAdjacentNote(direction) {
        const links = Array.from(document.querySelectorAll('.tree-row.file')).filter(
            (el) => el.offsetHeight > 0
        );
        if (links.length === 0) return;

        const currentHref = decodeURIComponent(window.location.pathname);
        const activeIndex = links.findIndex(
            (l) => decodeURIComponent(l.getAttribute('href') || '') === currentHref
        );
        if (activeIndex === -1) return;

        const targetIndex = activeIndex + direction;
        if (targetIndex < 0 || targetIndex >= links.length) return;

        window.location.href = links[targetIndex].getAttribute('href');
    }

    // ==========================================
    // 启动
    // ==========================================
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }

    window.Gestures = { init };
})();
