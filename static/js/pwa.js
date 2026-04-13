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
})();
