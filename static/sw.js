// ==========================================
// Service Worker — Obsidian Mirror v1.4.4
// 策略：静态资源缓存优先，动态内容网络优先
// ==========================================

const CACHE_NAME = 'obsidian-mirror-static-v1';

// 需要预缓存的静态资源（CSS / JS，不含内容页面）
const PRECACHE_ASSETS = [
    '/static/css/variables.css',
    '/static/css/layout.css',
    '/static/css/sidebar.css',
    '/static/css/search.css',
    '/static/css/markdown.css',
    '/static/css/toc.css',
    '/static/css/graph.css',
    '/static/css/settings.css',
    '/static/css/animations.css',
    '/static/css/themes.css',
    '/static/css/keyboard.css',
    '/static/css/callout.css',
    '/static/css/lightbox.css',
    '/static/css/math.css',
    '/static/css/accessibility.css',
    '/static/js/i18n.js',
    '/static/js/utils.js',
    '/static/js/theme.js',
    '/static/js/sidebar.js',
    '/static/js/search.js',
    '/static/js/toc.js',
    '/static/js/graph.js',
    '/static/js/settings.js',
    '/static/js/keyboard.js',
    '/static/js/callout.js',
    '/static/js/lightbox.js',
    '/static/js/katex-init.js',
    '/static/js/gestures.js',
    '/static/js/init.js',
    '/static/manifest.json',
];

// ==========================================
// 安装阶段：预缓存静态资源
// ==========================================
self.addEventListener('install', (event) => {
    event.waitUntil(
        caches.open(CACHE_NAME)
            .then((cache) => {
                // 逐个尝试缓存，失败不阻止安装
                return Promise.allSettled(
                    PRECACHE_ASSETS.map(url =>
                        cache.add(url).catch(() => {
                            console.warn('[SW] 预缓存失败（可忽略）:', url);
                        })
                    )
                );
            })
            .then(() => self.skipWaiting())
    );
});

// ==========================================
// 激活阶段：清理旧缓存
// ==========================================
self.addEventListener('activate', (event) => {
    event.waitUntil(
        caches.keys().then((keys) =>
            Promise.all(
                keys
                    .filter((k) => k !== CACHE_NAME)
                    .map((k) => caches.delete(k))
            )
        ).then(() => self.clients.claim())
    );
});

// ==========================================
// 请求拦截
// ==========================================
self.addEventListener('fetch', (event) => {
    const { request } = event;
    const url = new URL(request.url);

    // 只处理同源请求
    if (url.origin !== self.location.origin) return;

    // 静态资源：缓存优先（Cache First）
    if (url.pathname.startsWith('/static/')) {
        event.respondWith(cacheFirst(request));
        return;
    }

    // manifest.json：缓存优先
    if (url.pathname === '/manifest.json' || url.pathname === '/sw.js') {
        event.respondWith(cacheFirst(request));
        return;
    }

    // 动态内容（笔记页面、API）：网络优先（Network First）
    // 离线时提供友好提示
    event.respondWith(networkFirst(request));
});

/** 缓存优先策略 */
async function cacheFirst(request) {
    const cached = await caches.match(request);
    if (cached) return cached;
    try {
        const response = await fetch(request);
        if (response.ok) {
            const cache = await caches.open(CACHE_NAME);
            cache.put(request, response.clone());
        }
        return response;
    } catch (e) {
        return new Response('离线模式：静态资源不可用', { status: 503 });
    }
}

/** 网络优先策略 */
async function networkFirst(request) {
    try {
        const response = await fetch(request);
        // 成功后更新缓存（背景刷新）
        if (response.ok && request.method === 'GET') {
            const cache = await caches.open(CACHE_NAME);
            cache.put(request, response.clone());
        }
        return response;
    } catch (e) {
        // 网络失败，尝试从缓存提供
        const cached = await caches.match(request);
        if (cached) return cached;
        // 返回离线页面
        return new Response(
            `<!DOCTYPE html><html lang="zh-CN"><head><meta charset="UTF-8">
            <title>离线 - Obsidian Mirror</title>
            <style>body{font-family:system-ui;text-align:center;padding:60px;color:#666}
            h1{color:#6a5acd}a{color:#6a5acd}</style></head>
            <body><h1>📴 当前处于离线状态</h1>
            <p>无法连接到服务器，请检查网络连接后刷新页面。</p>
            <p><a href="/">尝试返回首页</a></p></body></html>`,
            { status: 200, headers: { 'Content-Type': 'text/html; charset=utf-8' } }
        );
    }
}
