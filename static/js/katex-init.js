// ==========================================
// KaTeX 数学公式渲染模块 — v1.4.1
// 渲染后端预处理生成的 .math-inline 和 .math-block 元素
// ==========================================

(function () {
    'use strict';

    /**
     * 渲染页面中所有数学公式元素
     * 元素格式：<span class="math-inline" data-math="LaTeX"></span>
     *           <div  class="math-block"  data-math="LaTeX"></div>
     */
    function renderMath() {
        if (typeof katex === 'undefined') {
            // KaTeX 未加载（离线场景），显示原始 LaTeX 文本作为降级
            document.querySelectorAll('[data-math]').forEach((el) => {
                const latex = el.getAttribute('data-math') || '';
                el.textContent = el.classList.contains('math-block')
                    ? `$$${latex}$$`
                    : `$${latex}$`;
                el.classList.add('math-fallback');
            });
            return;
        }

        // 渲染行内公式
        document.querySelectorAll('.math-inline[data-math]').forEach((el) => {
            const latex = el.getAttribute('data-math') || '';
            try {
                katex.render(latex, el, {
                    displayMode: false,
                    throwOnError: false,
                    output: 'html',
                });
            } catch (e) {
                el.textContent = `$${latex}$`;
                el.classList.add('math-error');
            }
        });

        // 渲染块级公式
        document.querySelectorAll('.math-block[data-math]').forEach((el) => {
            const latex = el.getAttribute('data-math') || '';
            try {
                katex.render(latex, el, {
                    displayMode: true,
                    throwOnError: false,
                    output: 'html',
                });
            } catch (e) {
                el.textContent = `$$${latex}$$`;
                el.classList.add('math-error');
            }
        });
    }

    // 页面加载后渲染
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', renderMath);
    } else {
        renderMath();
    }

    window.KaTeXRenderer = { render: renderMath };
})();
