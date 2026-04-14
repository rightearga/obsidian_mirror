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
