// ==========================================
// 触屏手势模块 — v1.4.4
// 侧边栏边缘滑动开关
// （翻页手势已移除：在移动端滚动位置检测不可靠，容易误触发导航）
// ==========================================

(function () {
    'use strict';

    const EDGE_TRIGGER_WIDTH = 30;  // 左边缘触发区域宽度（px）
    const MIN_SWIPE_DISTANCE = 60;  // 最小有效滑动距离（px）
    const MAX_VERTICAL_DRIFT = 60;  // 水平滑动允许的最大垂直偏移（px）

    let touchStartX = 0;
    let touchStartY = 0;
    let touchStartTime = 0;
    let isSidebarGesture = false;

    function init() {
        document.addEventListener('touchstart', onTouchStart, { passive: true });
        document.addEventListener('touchmove', onTouchMove, { passive: false });
        document.addEventListener('touchend', onTouchEnd, { passive: true });
    }

    function onTouchStart(e) {
        if (e.touches.length !== 1) return;
        const touch = e.touches[0];
        touchStartX = touch.clientX;
        touchStartY = touch.clientY;
        touchStartTime = Date.now();
        isSidebarGesture = touchStartX <= EDGE_TRIGGER_WIDTH;
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

        if (duration > 600) return;

        // ---- 左边缘右滑：打开侧边栏 ----
        if (isSidebarGesture && absDeltaX >= MIN_SWIPE_DISTANCE && absDeltaX > absDeltaY) {
            if (deltaX > 0) openSidebar();
            return;
        }

        // ---- 侧边栏任意位置左滑：关闭侧边栏 ----
        if (document.body.classList.contains('sidebar-expanded')
            && deltaX < -MIN_SWIPE_DISTANCE && absDeltaX > absDeltaY) {
            closeSidebar();
        }
    }

    function openSidebar() {
        if (document.body.classList.contains('sidebar-expanded')) return;
        document.body.classList.add('sidebar-expanded');
        document.documentElement.classList.add('sidebar-expanded');
        try { localStorage.setItem('obsidian_mirror_sidebar_state', 'open'); } catch (e) {}
    }

    function closeSidebar() {
        if (!document.body.classList.contains('sidebar-expanded')) return;
        document.body.classList.remove('sidebar-expanded');
        document.documentElement.classList.remove('sidebar-expanded');
        try { localStorage.setItem('obsidian_mirror_sidebar_state', 'closed'); } catch (e) {}
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }

    window.Gestures = { init, openSidebar, closeSidebar };
})();
