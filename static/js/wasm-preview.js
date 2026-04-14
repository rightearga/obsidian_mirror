// ==========================================
// WASM 实时预览面板（v1.6.1）
// 利用 WASM 模块在浏览器端实时渲染 Markdown，
// 无需服务端 round-trip，目标延迟 < 5ms
// ==========================================

(function () {
    'use strict';

    // 实时预览面板管理器
    const WasmPreview = {
        /** 面板是否激活 */
        active: false,
        /** 防抖定时器 */
        debounceTimer: null,
        /** 防抖延迟（毫秒）*/
        DEBOUNCE_MS: 300,

        /**
         * 初始化：在笔记页面插入「实时预览」按钮
         * 仅当 WASM 模块可用时添加按钮
         */
        init() {
            // 只在笔记正文页面初始化
            const markdownBody = document.querySelector('.markdown-body');
            if (!markdownBody) return;

            // 等待 WASM 加载完成后插入按钮
            window.WasmLoader?.init().then((mod) => {
                if (mod?.render_markdown) {
                    this._insertToggleButton();
                }
            }).catch(() => {});
        },

        /**
         * 在工具栏或正文区域插入「实时预览」切换按钮
         */
        _insertToggleButton() {
            // 若已存在则不重复插入
            if (document.getElementById('wasm-preview-toggle')) return;

            const btn = document.createElement('button');
            btn.id = 'wasm-preview-toggle';
            btn.className = 'wasm-preview-toggle-btn';
            btn.title = '实时预览（由 WebAssembly 驱动）';
            btn.innerHTML = `
                <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24"
                     fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                    <circle cx="12" cy="12" r="3"/>
                </svg>
                实时预览
            `;
            btn.addEventListener('click', () => this.toggle());

            // 尝试插入到状态栏或面包屑附近
            const target = document.querySelector('.breadcrumb, .status-bar, .markdown-body');
            if (target) {
                const wrapper = document.createElement('div');
                wrapper.className = 'wasm-preview-toggle-wrapper';
                wrapper.appendChild(btn);
                target.insertAdjacentElement('beforebegin', wrapper);
            }
        },

        /**
         * 切换实时预览面板的显示状态
         */
        toggle() {
            if (this.active) {
                this._closePanel();
            } else {
                this._openPanel();
            }
        },

        /**
         * 打开实时预览面板
         */
        _openPanel() {
            if (this.active) return;
            this.active = true;

            // 更新按钮状态
            const btn = document.getElementById('wasm-preview-toggle');
            if (btn) btn.classList.add('active');

            // 创建面板容器
            const panel = document.createElement('div');
            panel.id = 'wasm-preview-panel';
            panel.className = 'wasm-preview-panel';
            panel.innerHTML = `
                <div class="wasm-preview-header">
                    <span class="wasm-preview-label">
                        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24"
                             fill="none" stroke="currentColor" stroke-width="2">
                            <polyline points="16 18 22 12 16 6"/>
                            <polyline points="8 6 2 12 8 18"/>
                        </svg>
                        Markdown 输入
                    </span>
                    <span class="wasm-preview-label">
                        <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24"
                             fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
                            <circle cx="12" cy="12" r="3"/>
                        </svg>
                        实时渲染 <small class="wasm-badge">WASM</small>
                    </span>
                    <button class="wasm-preview-close" title="关闭预览">✕</button>
                </div>
                <div class="wasm-preview-body">
                    <textarea
                        id="wasm-preview-input"
                        class="wasm-preview-textarea"
                        placeholder="在此输入 Markdown...

支持：
- [[WikiLink]] 和 [[链接|别名]]
- ![[图片.png]]
- **加粗** / *斜体* / ~~删除线~~
- ==高亮文字==
- $行内数学$ 和 $$块级数学$$
- | 表格 | 列 |
- > [!NOTE] Callout 块
- [^1] 脚注"
                        spellcheck="false"
                    ></textarea>
                    <div id="wasm-preview-output" class="wasm-preview-output markdown-body">
                        <p class="wasm-preview-placeholder">输入 Markdown 后，渲染结果将显示在这里...</p>
                    </div>
                </div>
            `;

            // 插入到页面主内容区
            const main = document.querySelector('#main-content, .content-wrapper, main, .markdown-body');
            if (main) {
                main.insertAdjacentElement('beforebegin', panel);
            } else {
                document.body.appendChild(panel);
            }

            // 绑定关闭按钮
            panel.querySelector('.wasm-preview-close').addEventListener('click', () => this._closePanel());

            // 绑定输入事件（防抖）
            const textarea = document.getElementById('wasm-preview-input');
            textarea.addEventListener('input', (e) => {
                clearTimeout(this.debounceTimer);
                this.debounceTimer = setTimeout(() => this._render(e.target.value), this.DEBOUNCE_MS);
            });

            // 自动聚焦
            textarea.focus();

            // 记录状态
            localStorage.setItem('wasm_preview_open', '1');
        },

        /**
         * 关闭实时预览面板
         */
        _closePanel() {
            this.active = false;
            clearTimeout(this.debounceTimer);

            const panel = document.getElementById('wasm-preview-panel');
            if (panel) panel.remove();

            const btn = document.getElementById('wasm-preview-toggle');
            if (btn) btn.classList.remove('active');

            localStorage.removeItem('wasm_preview_open');
        },

        /**
         * 使用 WASM 渲染 Markdown 并更新预览区
         * @param {string} markdown
         */
        _render(markdown) {
            const output = document.getElementById('wasm-preview-output');
            if (!output) return;

            if (!markdown.trim()) {
                output.innerHTML = '<p class="wasm-preview-placeholder">输入 Markdown 后，渲染结果将显示在这里...</p>';
                return;
            }

            const t0 = performance.now();
            const html = window.WasmLoader?.renderMarkdown(markdown) ?? '';
            const elapsed = (performance.now() - t0).toFixed(1);

            output.innerHTML = html;

            // 触发前端 JS 处理（Callout 解析、KaTeX 渲染等）
            if (window.Callout?.init) window.Callout.init();
            if (window.MermaidManager?.renderAll) window.MermaidManager.renderAll();

            // 更新性能标签
            const badge = document.querySelector('.wasm-badge');
            if (badge) badge.title = `渲染耗时 ${elapsed}ms`;
        }
    };

    // 页面加载后初始化
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', () => WasmPreview.init());
    } else {
        WasmPreview.init();
    }

    window.WasmPreview = WasmPreview;

})();
