/**
 * obsidian_mirror WASM 模块加载器（v1.6.0）
 *
 * 异步加载 WASM 模块，提供渐进增强支持：
 * - 支持 WebAssembly 时：加载 .wasm 并暴露 Rust 函数
 * - 不支持或加载失败时：自动 fallback 到 JavaScript 等价实现，不影响现有功能
 *
 * 使用方式：
 *   // 在页面中引入（已在 layout.html 中加载）
 *   // 初始化后可通过 window.WasmLoader 使用：
 *   WasmLoader.init().then(() => {
 *     const highlighted = WasmLoader.highlightTerm("Hello World", "world");
 *     console.log(highlighted); // "Hello <mark>World</mark>"
 *   });
 */

(function () {
    'use strict';

    const WasmLoader = {
        /** 已加载的 WASM 模块实例（null 表示未加载或加载失败）*/
        module: null,
        /** WASM 是否已成功加载 */
        loaded: false,
        /** 加载是否进行中 */
        loading: false,
        /** 加载耗时（毫秒），用于性能基准比对 */
        loadTime: null,
        /** 加载 Promise，防止并发初始化 */
        _initPromise: null,

        /**
         * 初始化并加载 WASM 模块。
         * 幂等：多次调用只加载一次。
         * @returns {Promise<object|null>} WASM 模块实例，失败时为 null
         */
        async init() {
            if (this.loaded) return this.module;
            if (this._initPromise) return this._initPromise;

            this._initPromise = this._doInit();
            return this._initPromise;
        },

        async _doInit() {
            if (!this._supportsWasm()) {
                console.warn('[WASM] 浏览器不支持 WebAssembly，使用 JavaScript fallback');
                return null;
            }

            this.loading = true;
            const t0 = performance.now();

            try {
                // 异步导入 wasm-pack 生成的 ES module 胶水代码
                const wasmModule = await import('/static/wasm/obsidian_mirror_wasm.js');
                // 初始化 WASM 实例（加载 .wasm 二进制文件）
                await wasmModule.default('/static/wasm/obsidian_mirror_wasm_bg.wasm');

                this.module = wasmModule;
                this.loaded = true;
                this.loadTime = performance.now() - t0;

                const version = wasmModule.wasm_version?.() || 'unknown';
                console.log(`[WASM] 模块加载完成，耗时 ${this.loadTime.toFixed(1)}ms，版本 ${version}`);

                // v1.6.2：WASM 加载完成后异步加载离线搜索索引
                this.loadIndex().catch(() => {});
            } catch (e) {
                // 加载失败：记录警告但不中断应用（渐进增强）
                console.warn('[WASM] 模块加载失败，使用 JavaScript fallback:', e.message || e);
                this.module = null;
            } finally {
                this.loading = false;
            }

            return this.module;
        },

        /**
         * 检查浏览器是否支持 WebAssembly。
         * @returns {boolean}
         */
        _supportsWasm() {
            try {
                return typeof WebAssembly === 'object'
                    && typeof WebAssembly.instantiate === 'function'
                    && typeof WebAssembly.instantiateStreaming === 'function';
            } catch {
                return false;
            }
        },

        // ─── 公共函数（WASM 优先，fallback 到 JS 实现）──────────────────────

        // ─── v1.6.2：离线搜索 ─────────────────────────────────────────────────

        /** 已加载的 NoteIndex 实例（null 表示未加载或不可用）*/
        noteIndex: null,
        /** index.json 是否已加载 */
        indexLoaded: false,

        /**
         * 加载离线搜索索引（从 /static/wasm/index.json）。
         * 在 WASM 模块初始化完成后自动调用。
         * @returns {Promise<boolean>} 是否成功加载
         */
        async loadIndex() {
            if (this.indexLoaded) return this.noteIndex !== null;
            if (!this.loaded || !this.module?.NoteIndex) return false;

            try {
                const resp = await fetch('/static/wasm/index.json');
                if (!resp.ok) return false;

                const json = await resp.text();
                this.noteIndex = this.module.NoteIndex.loadJson(json);
                this.indexLoaded = true;

                const count = this.noteIndex?.noteCount?.() ?? 0;
                console.log(`[WASM] 离线搜索索引加载完成，共 ${count} 条笔记`);
                return true;
            } catch (e) {
                console.warn('[WASM] 离线搜索索引加载失败:', e.message || e);
                return false;
            }
        },

        /**
         * 使用 WASM NoteIndex 搜索（离线可用，v1.6.2）。
         * 返回与服务端 /api/search 格式一致的结果数组。
         * 仅在 WASM 加载完成且索引已加载时有效，否则返回 null（调用方应 fallback 到服务端）。
         *
         * @param {string} query 搜索关键词
         * @param {number} limit 最大结果数（默认 20）
         * @returns {Array|null} 搜索结果数组，或 null 表示不可用
         */
        search(query, limit = 20) {
            if (!this.loaded || !this.noteIndex) return null;
            try {
                const json = this.noteIndex.searchJson(query, limit);
                return JSON.parse(json);
            } catch (e) {
                console.warn('[WASM] 搜索失败:', e);
                return null;
            }
        },

        /**
         * 在文本中高亮关键词，用 <mark>...</mark> 包裹（大小写不敏感）。
         * WASM 版本与服务端 `search_engine::highlight_terms` 逻辑一致。
         *
         * @param {string} text 原始文本
         * @param {string} term 要高亮的词
         * @returns {string} 带 <mark> 标签的 HTML
         */
        highlightTerm(text, term) {
            const t0 = performance.now();
            let result;

            if (this.loaded && this.module?.highlight_term) {
                result = this.module.highlight_term(text, term);
                this._logPerf('highlightTerm', 'wasm', t0);
            } else {
                // JavaScript fallback
                result = this._highlightTermFallback(text, term);
                this._logPerf('highlightTerm', 'js', t0);
            }

            return result;
        },

        _highlightTermFallback(text, term) {
            if (!term) return text;
            // 转义正则特殊字符
            const escaped = term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
            return text.replace(new RegExp(`(${escaped})`, 'gi'), '<mark>$1</mark>');
        },

        /**
         * 将 Markdown 渲染为 HTML（WASM 优先，v1.6.1）。
         * 支持 WikiLink、数学公式占位、高亮语法等 Obsidian 扩展语法。
         * WASM 目标 < 5ms（vs 服务端 HTTP round-trip ~50ms）。
         *
         * @param {string} markdown 输入 Markdown 字符串
         * @returns {string} 渲染后的 HTML
         */
        renderMarkdown(markdown) {
            const t0 = performance.now();
            let result;
            if (this.loaded && this.module?.render_markdown) {
                result = this.module.render_markdown(markdown);
                this._logPerf('renderMarkdown', 'wasm', t0);
            } else {
                // JavaScript fallback：基本段落化
                result = '<p>' + markdown
                    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
                    .replace(/\n\n+/g, '</p><p>').replace(/\n/g, '<br>') + '</p>';
                this._logPerf('renderMarkdown', 'js-fallback', t0);
            }
            return result;
        },

        /**
         * 从 HTML 中提取纯文本并截取（去除所有 HTML 标签）。
         * WASM 版本与服务端 `handlers::truncate_html` 逻辑一致。
         *
         * @param {string} html 输入 HTML 字符串
         * @param {number} maxChars 最大字符数
         * @returns {string} 截断后的纯文本，超限时末尾加 "..."
         */
        truncateHtml(html, maxChars) {
            if (this.loaded && this.module?.truncate_html) {
                return this.module.truncate_html(html, maxChars);
            }
            // JavaScript fallback
            const text = html.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim();
            return text.length <= maxChars ? text : text.slice(0, maxChars) + '...';
        },

        // ─── v1.6.3：图谱布局 + 搜索过滤 + TOC ─────────────────────────────────

        /**
         * 使用 WASM Fruchterman-Reingold 算法计算图谱布局坐标（v1.6.3）。
         * 性能目标：500 节点 < 200ms（vs Vis.js 物理引擎 ~2s）。
         *
         * @param {Array<{id:string}>} nodes 节点数组
         * @param {Array<{from:string,to:string}>} edges 边数组
         * @param {number} [iterations] 迭代次数（自动按节点数调整）
         * @returns {Array<{id:string,x:number,y:number}>|null} 不可用时返回 null
         */
        computeGraphLayout(nodes, edges, iterations) {
            if (!this.loaded || !this.module?.computeGraphLayout) return null;
            try {
                const t0 = performance.now();
                const n = nodes.length;
                const iter = iterations ?? (n > 300 ? 15 : n > 100 ? 30 : 50);
                const result = this.module.computeGraphLayout(
                    JSON.stringify(nodes), JSON.stringify(edges), iter
                );
                this._logPerf('computeGraphLayout', 'wasm', t0);
                return JSON.parse(result);
            } catch (e) {
                console.warn('[WASM] 图谱布局计算失败:', e);
                return null;
            }
        },

        /**
         * 知识地图布局计算（v1.9.5 / v1.9.7）。
         * 基于标签相似度（Jaccard）+ F-R 力导向布局 + K-means 聚类。
         * 比 JS Worker 实现快 10-100×。
         *
         * @param {Array<{id,title,path,tags,pagerank}>} notes 笔记列表
         * @returns {Array<{id,x,y,tags,cluster_id,pagerank}>|null} 布局结果，不可用时返回 null
         */
        computeKnowledgeMap(notes) {
            if (!this.loaded || !this.module?.computeKnowledgeMap) return null;
            try {
                const t0 = performance.now();
                const result = this.module.computeKnowledgeMap(JSON.stringify(notes));
                this._logPerf('computeKnowledgeMap', 'wasm', t0);
                return JSON.parse(result);
            } catch (e) {
                console.warn('[WASM] 知识地图布局计算失败:', e);
                return null;
            }
        },

        /**
         * PageRank 影响力分数计算（v1.9.0）。
         * 归一化 0.0–1.0，阻尼因子 0.85，20 轮迭代。
         *
         * @param {Array<{id}>} nodes 节点数组
         * @param {Array<{from,to}>} edges 边数组
         * @param {number} [iterations=20] 迭代次数
         * @returns {Object|null} {node_id: score} 字典，不可用时返回 null
         */
        computePagerank(nodes, edges, iterations = 20) {
            if (!this.loaded || !this.module?.computePagerank) return null;
            try {
                const t0 = performance.now();
                const result = this.module.computePagerank(
                    JSON.stringify(nodes), JSON.stringify(edges), iterations
                );
                this._logPerf('computePagerank', 'wasm', t0);
                return JSON.parse(result);
            } catch (e) {
                console.warn('[WASM] PageRank 计算失败:', e);
                return null;
            }
        },

        /**
         * 本地 WASM 笔记过滤（v1.6.3）。
         * 多标签交集 + 路径前缀过滤，< 5ms（1000 条笔记）。
         *
         * @param {Array<{title,path,tags}>} notes 笔记列表
         * @param {string} tagsFilter 逗号分隔的必须标签（ALL 语义）
         * @param {string} folderFilter 路径前缀（空字符串 = 不过滤）
         * @param {number} [limit=50] 最大返回条数
         * @returns {Array<{title,path,tags}>}
         */
        filterNotes(notes, tagsFilter = '', folderFilter = '', limit = 50) {
            if (this.loaded && this.module?.filterNotes) {
                try {
                    const t0 = performance.now();
                    const result = this.module.filterNotes(JSON.stringify(notes), tagsFilter, folderFilter, limit);
                    this._logPerf('filterNotes', 'wasm', t0);
                    return JSON.parse(result);
                } catch (e) { /* fallthrough to JS */ }
            }
            // JavaScript fallback
            const required = tagsFilter.split(',').map(t => t.trim()).filter(Boolean);
            return notes.filter(note => {
                if (folderFilter && !note.path.toLowerCase().startsWith(folderFilter.toLowerCase())) return false;
                if (required.length > 0) {
                    const ntags = (note.tags || []).map(t => t.toLowerCase());
                    return required.every(tag => ntags.includes(tag.toLowerCase()));
                }
                return true;
            }).slice(0, limit);
        },

        /**
         * 从 HTML 中提取目录（TOC）（v1.6.3）。
         * 配合实时预览使用，< 1ms（100 个标题）。
         *
         * @param {string} html 渲染后的 HTML 字符串
         * @returns {Array<{level:number,text:string,id:string}>}
         */
        generateToc(html) {
            if (this.loaded && this.module?.generateTocFromHtml) {
                try {
                    const t0 = performance.now();
                    const result = this.module.generateTocFromHtml(html);
                    this._logPerf('generateToc', 'wasm', t0);
                    return JSON.parse(result);
                } catch (e) { /* fallthrough */ }
            }
            // JavaScript fallback
            const items = [];
            const re = /<h([1-6])(?:[^>]*id=["']([\w-]+)["'][^>]*)?>([^<]+)<\/h[1-6]>/gi;
            let m;
            while ((m = re.exec(html)) !== null) {
                items.push({ level: parseInt(m[1]), text: m[3].trim(), id: m[2] || '' });
            }
            return items;
        },

        /**
         * 性能日志：记录 WASM vs JS 耗时，用于基准比对。
         * 仅在开发模式（localStorage.debug_wasm=true）下输出。
         */
        _logPerf(fn, impl, t0) {
            if (localStorage.getItem('debug_wasm') === 'true') {
                const elapsed = (performance.now() - t0).toFixed(3);
                console.debug(`[WASM:perf] ${fn} [${impl}] ${elapsed}ms`);
            }
        }
    };

    // 暴露到全局
    window.WasmLoader = WasmLoader;

    // 页面加载后自动初始化（非阻塞，不影响页面渲染）
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', () => WasmLoader.init());
    } else {
        WasmLoader.init();
    }

})();
