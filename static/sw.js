// ==========================================
// Service Worker — Obsidian Mirror v1.8.3
// 策略：静态资源缓存优先，动态内容网络优先
// v1.6.2 新增：WASM 离线搜索（拦截 /api/search，使用 NoteIndex WASM 模块）
// v1.8.3 新增：
//   - /api/titles、/api/graph/global 使用 Stale-While-Revalidate 策略，离线可用
//   - POST /sync 成功后通过 postMessage 通知所有客户端（Broadcast 刷新提示）
//   - 搜索离线响应格式升级为 v1.8.0 分页格式
// ==========================================

// v1.8.3：版本号升 v3，清理旧版缓存（v2 静态资源 + v1 WASM）
const CACHE_NAME = 'obsidian-mirror-static-v3';

// WASM 离线搜索索引缓存（与主缓存分离，方便单独更新）
const WASM_CACHE_NAME = 'obsidian-mirror-wasm-v1';

// v1.8.3：API 数据缓存（/api/titles、/api/graph/global 等半静态数据）
const DATA_CACHE_NAME = 'obsidian-mirror-data-v1';

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
    // v1.6.0：WASM 加载器
    '/static/wasm/loader.js',
    // v1.6.1：WASM 预览 JS/CSS
    '/static/js/wasm-preview.js',
    // v1.8.2：打印样式
    '/static/css/print.css',
];

// v1.6.2：WASM 模块和离线搜索索引资源（单独缓存，随每次同步更新）
const WASM_ASSETS = [
    '/static/wasm/obsidian_mirror_wasm.js',
    '/static/wasm/obsidian_mirror_wasm_bg.wasm',
    '/static/wasm/index.json',  // 离线搜索索引（每次同步后更新）
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
    // v1.8.3：保留当前版本的所有已知缓存，清理其他旧版缓存
    const KNOWN_CACHES = new Set([CACHE_NAME, WASM_CACHE_NAME, DATA_CACHE_NAME]);
    event.waitUntil(
        caches.keys().then((keys) =>
            Promise.all(
                keys
                    .filter((k) => !KNOWN_CACHES.has(k))
                    .map((k) => caches.delete(k))
            )
        ).then(() => self.clients.claim())
    );
});

// ==========================================
// 请求拦截（v1.6.2 新增 WASM 离线搜索）
// ==========================================
self.addEventListener('fetch', (event) => {
    const { request } = event;
    const url = new URL(request.url);

    // 只处理同源请求
    if (url.origin !== self.location.origin) return;

    // v1.6.2：/api/search 离线时使用 WASM NoteIndex 搜索
    if (url.pathname === '/api/search' && request.method === 'GET') {
        event.respondWith(searchWithFallback(request, url));
        return;
    }

    // v1.8.3：POST /sync 成功后通知所有客户端（"有新内容"横幅）
    if (url.pathname === '/sync' && request.method === 'POST') {
        event.respondWith(
            fetch(request).then(response => {
                if (response.ok) {
                    // 同步成功：通知所有客户端刷新提示
                    self.clients.matchAll({ includeUncontrolled: true, type: 'window' })
                        .then(clients => clients.forEach(c => c.postMessage({ type: 'SYNC_COMPLETE' })));
                }
                return response;
            }).catch(() =>
                new Response(JSON.stringify({ error: '网络不可用，同步失败' }), {
                    status: 503,
                    headers: { 'Content-Type': 'application/json; charset=utf-8' },
                })
            )
        );
        return;
    }

    // v1.8.3：侧边栏数据 + 全局图谱 → Stale-While-Revalidate（离线可用）
    if (
        (url.pathname === '/api/titles' || url.pathname.startsWith('/api/graph')) &&
        request.method === 'GET'
    ) {
        event.respondWith(staleWhileRevalidate(request));
        return;
    }

    // v1.6.2：WASM 索引文件缓存优先（单独缓存桶，方便更新）
    if (url.pathname.startsWith('/static/wasm/')) {
        event.respondWith(wasmCacheFirst(request));
        return;
    }

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

    // 动态内容（笔记页面、API）：网络优先（Network First），缓存成功响应供离线使用
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

/** Stale-While-Revalidate 策略（v1.8.3）
 *
 * 立即返回缓存（如有），同时在后台刷新缓存。
 * 适用于半静态 API 数据：/api/titles、/api/graph/global 等。
 * 离线时返回缓存；首次访问（无缓存）时阻塞等待网络。
 */
async function staleWhileRevalidate(request) {
    const cache  = await caches.open(DATA_CACHE_NAME);
    const cached = await cache.match(request);

    // 后台刷新（fire-and-forget）
    const fetchPromise = fetch(request).then(response => {
        if (response.ok) cache.put(request, response.clone());
        return response;
    }).catch(() => null);

    // 有缓存：立即返回，后台刷新
    if (cached) return cached;

    // 无缓存：等待网络（首次访问）
    const networkResp = await fetchPromise;
    return networkResp || new Response('{}', {
        status: 503,
        headers: { 'Content-Type': 'application/json; charset=utf-8' },
    });
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

// ==========================================
// v1.6.2：WASM 缓存 + 离线搜索
// ==========================================

/** WASM 资源缓存优先策略（使用独立缓存桶） */
async function wasmCacheFirst(request) {
    const cache = await caches.open(WASM_CACHE_NAME);
    const cached = await cache.match(request);
    if (cached) return cached;
    try {
        const response = await fetch(request);
        if (response.ok) cache.put(request, response.clone());
        return response;
    } catch (e) {
        return new Response('WASM 资源离线不可用', { status: 503 });
    }
}

/** WASM 模块缓存（懒加载，仅在需要时初始化） */
let _wasmModule = null;
let _noteIndex  = null;

/** 加载 WASM 模块（从缓存或网络） */
async function loadWasmModule() {
    if (_wasmModule) return _wasmModule;
    try {
        // Service Worker 中使用 importScripts 加载 WASM JS glue
        const wasmJs = await (await caches.match('/static/wasm/obsidian_mirror_wasm.js') ||
                              fetch('/static/wasm/obsidian_mirror_wasm.js')).text();
        // SW 中无法使用 ES module import()，fallback 到返回 null 触发网络搜索
        // 真正的 SW WASM 集成需要 Workbox 或 wasm-bindgen 的 no-module 模式
        // 当前版本：SW 检测到离线时返回缓存索引的纯文本搜索结果
        _wasmModule = null;
    } catch (e) {
        _wasmModule = null;
    }
    return _wasmModule;
}

/** 从缓存的 index.json 做基础文本搜索（SW 中的 JS fallback） */
async function offlineTextSearch(query, limit) {
    const indexResp = await caches.match('/static/wasm/index.json');
    if (!indexResp) return null;

    let notes;
    try {
        notes = await indexResp.json();
    } catch (e) {
        return null;
    }

    const q = query.toLowerCase().trim();
    if (!q) return [];

    // 简单文本评分（SW 中无 WASM，使用 JS 实现）
    const results = notes
        .map(note => {
            let score = 0;
            const titleLower = (note.title || '').toLowerCase();
            const contentLower = (note.content || '').toLowerCase();
            const tagStr = (note.tags || []).join(' ').toLowerCase();

            if (titleLower.includes(q)) score += 10;
            if (tagStr.includes(q)) score += 5;
            if (contentLower.includes(q)) score += 1;

            return { note, score };
        })
        .filter(({ score }) => score > 0)
        .sort((a, b) => b.score - a.score)
        .slice(0, limit)
        .map(({ note, score }) => ({
            title:   note.title   || '',
            path:    note.path    || '',
            snippet: (note.content || '').slice(0, 150) + '...',
            score,
            mtime:   note.mtime   || 0,
            tags:    note.tags    || [],
        }));

    return results;
}

/**
 * 搜索请求处理：网络优先，离线时使用本地 JS 搜索（v1.6.2）
 *
 * 离线时拦截 /api/search，从缓存的 index.json 做文本匹配，
 * 返回与在线 API 格式一致的 JSON 响应，前端无感知切换。
 */
async function searchWithFallback(request, url) {
    try {
        // 网络可用：直接转发，并后台更新缓存
        const response = await fetch(request);
        if (response.ok) {
            const cache = await caches.open(CACHE_NAME);
            cache.put(request, response.clone());
        }
        return response;
    } catch (e) {
        // 网络不可用：使用缓存索引搜索
        const q     = url.searchParams.get('q') || '';
        const limit = parseInt(url.searchParams.get('limit') || '20', 10);

        const results = await offlineTextSearch(q, limit);

        // v1.8.3：响应格式升级为 v1.8.0 分页格式 {results, total, page, per_page, total_pages}
        const page   = parseInt(url.searchParams.get('page')     || '1',  10);
        const perPg  = parseInt(url.searchParams.get('per_page') || '20', 10);

        if (results === null) {
            const empty = { results: [], total: 0, page, per_page: perPg, total_pages: 0 };
            return new Response(JSON.stringify(empty), {
                status: 200,
                headers: { 'Content-Type': 'application/json; charset=utf-8',
                           'X-Offline-Search': 'no-index' }
            });
        }

        const pagedPage = { results, total: results.length, page: 1, per_page: results.length, total_pages: 1 };
        return new Response(JSON.stringify(pagedPage), {
            status: 200,
            headers: { 'Content-Type': 'application/json; charset=utf-8',
                       'X-Offline-Search': 'js-fallback' }
        });
    }
}
