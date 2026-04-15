// ==========================================
// PWA 注册模块 — v1.4.4
// 注册 Service Worker，启用离线缓存能力
// ==========================================

(function () {
    'use strict';

    /** 注册 Service Worker */
    function registerServiceWorker() {
        if (!('serviceWorker' in navigator)) return;

        window.addEventListener('load', () => {
            navigator.serviceWorker
                .register('/sw.js', { scope: '/' })
                .then((reg) => {
                    console.log('[PWA] Service Worker 注册成功:', reg.scope);

                    // 检测到新版本时提示用户刷新
                    reg.addEventListener('updatefound', () => {
                        const newWorker = reg.installing;
                        if (!newWorker) return;
                        newWorker.addEventListener('statechange', () => {
                            if (
                                newWorker.state === 'installed' &&
                                navigator.serviceWorker.controller
                            ) {
                                showUpdateNotification();
                            }
                        });
                    });
                })
                .catch((err) => {
                    console.warn('[PWA] Service Worker 注册失败:', err);
                });
        });
    }

    /** 显示更新提示 Toast */
    function showUpdateNotification() {
        const toast = document.createElement('div');
        toast.style.cssText = `
            position: fixed; bottom: 20px; left: 50%; transform: translateX(-50%);
            background: var(--primary-color); color: #fff;
            padding: 10px 20px; border-radius: 8px; font-size: 14px;
            z-index: 9999; box-shadow: 0 4px 12px rgba(0,0,0,0.2);
            display: flex; align-items: center; gap: 12px;
        `;
        toast.innerHTML = `
            <span>发现新版本，刷新后生效</span>
            <button onclick="location.reload()" style="
                background:#fff; color:var(--primary-color);
                border:none; border-radius:4px; padding:4px 10px;
                font-size:13px; cursor:pointer; font-weight:500;">
                刷新
            </button>
        `;
        document.body.appendChild(toast);
        setTimeout(() => toast.remove(), 8000);
    }

    registerServiceWorker();
    initNetworkStatus();
    initSyncCompleteListener();
})();

// ==========================================
// v1.8.3：网络状态指示器
// ==========================================

/** 初始化网络状态监听，同步更新状态栏图标和搜索框提示 */
function initNetworkStatus() {
    function update() {
        const online = navigator.onLine;
        const dot  = document.getElementById('network-dot');
        const text = document.getElementById('network-status-text');
        if (dot)  dot.classList.toggle('offline', !online);
        if (text) text.textContent = online ? '在线' : '离线';
        // 离线时在搜索模态框中显示提示
        const hint = document.getElementById('offline-search-hint');
        if (hint) hint.style.display = online ? 'none' : '';
    }

    window.addEventListener('online',  update);
    window.addEventListener('offline', update);
    // DOMContentLoaded 后执行一次（DOM 可能还未就绪）
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', update);
    } else {
        update();
    }
}

// ==========================================
// v1.8.3：Service Worker 同步完成通知监听
// ==========================================

/** 监听 Service Worker 发来的 SYNC_COMPLETE 消息，显示刷新横幅 */
function initSyncCompleteListener() {
    if (!('serviceWorker' in navigator)) return;
    navigator.serviceWorker.addEventListener('message', (event) => {
        if (event.data && event.data.type === 'SYNC_COMPLETE') {
            const banner = document.getElementById('sync-refresh-banner');
            if (banner) banner.classList.add('visible');
        }
    });
}
