// ==========================================
// Callout 块解析模块 — v1.4.1
// 将 > [!TYPE] Title 语法的 blockquote 转换为样式化 callout
// ==========================================

(function () {
    'use strict';

    // Callout 类型对应的默认标题
    const CALLOUT_TITLES = {
        NOTE: '注意', TIP: '提示', INFO: '信息', WARNING: '警告',
        DANGER: '危险', CAUTION: '注意', ERROR: '错误',
        SUCCESS: '成功', CHECK: '完成', DONE: '完成',
        QUESTION: '问题', FAQ: '常见问题', HELP: '帮助',
        QUOTE: '引用', CITE: '引用', ABSTRACT: '摘要',
        SUMMARY: '总结', TLDR: '概要',
        BUG: '缺陷', FAILURE: '失败', FAIL: '失败',
        MISSING: '缺失', EXAMPLE: '示例', EXPERIMENT: '实验',
        TODO: '待办',
    };

    // Callout 类型对应的内联 SVG 图标
    const CALLOUT_ICONS = {
        NOTE:     '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>',
        TIP:      '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2a7 7 0 0 1 7 7c0 2.38-1.19 4.47-3 5.74V17a2 2 0 0 1-2 2H10a2 2 0 0 1-2-2v-2.26C6.19 13.47 5 11.38 5 9a7 7 0 0 1 7-7z"/><line x1="10" y1="21" x2="14" y2="21"/></svg>',
        INFO:     '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/></svg>',
        WARNING:  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>',
        DANGER:   '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>',
        SUCCESS:  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg>',
        QUESTION: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>',
        QUOTE:    '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 21c3 0 7-1 7-8V5c0-1.25-.756-2.017-2-2H4c-1.25 0-2 .75-2 1.972V11c0 1.25.75 2 2 2 1 0 1 0 1 1v1c0 1-1 2-2 2s-1 .008-1 1.031V20c0 1 0 1 1 1z"/><path d="M15 21c3 0 7-1 7-8V5c0-1.25-.757-2.017-2-2h-4c-1.25 0-2 .75-2 1.972V11c0 1.25.75 2 2 2h.75c0 2.25.25 4-2.75 4v3c0 1 0 1 1 1z"/></svg>',
        BUG:      '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="8" y="6" width="8" height="14" rx="4"/><path d="M4 14h2m12 0h2M6 9l-2-2m16 2-2-2M9 6V3m6 3V3"/></svg>',
        TODO:     '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="5" width="6" height="6" rx="1"/><path d="m3 17 2 2 4-4"/><line x1="13" y1="6" x2="21" y2="6"/><line x1="13" y1="12" x2="21" y2="12"/><line x1="13" y1="18" x2="21" y2="18"/></svg>',
        EXAMPLE:  '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/><line x1="10" y1="9" x2="8" y2="9"/></svg>',
        ABSTRACT: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="21" y1="6" x2="3" y2="6"/><line x1="17" y1="12" x2="3" y2="12"/><line x1="13" y1="18" x2="3" y2="18"/></svg>',
    };

    /** 获取类型对应的图标 SVG，找不到时返回通用信息图标 */
    function getIcon(type) {
        return CALLOUT_ICONS[type] || CALLOUT_ICONS.INFO;
    }

    /** 将 markdown-body 内所有 blockquote 转换为 callout div */
    function initCallouts() {
        const body = document.querySelector('.markdown-body');
        if (!body) return;

        const blockquotes = body.querySelectorAll('blockquote');
        blockquotes.forEach(transformBlockquote);
    }

    /**
     * 检测并转换单个 blockquote
     * Obsidian callout 格式：第一个 <p> 的文本以 [!TYPE] 开头
     */
    function transformBlockquote(bq) {
        const firstP = bq.querySelector('p:first-child');
        if (!firstP) return;

        // 获取第一行文本（只取第一个文本节点，排除后续换行内容）
        const rawText = firstP.textContent;
        const firstLine = rawText.split('\n')[0].trim();
        const match = firstLine.match(/^\[!([A-Za-z]+)\]([+-]?)\s*(.*)/);
        if (!match) return;

        const typeRaw = match[1].toUpperCase();
        const foldChar = match[2]; // '-' = 默认收起，'+' = 默认展开，'' = 展开（不可折叠）
        const titleText = match[3].trim() || CALLOUT_TITLES[typeRaw] || typeRaw;
        const isFoldable = foldChar !== '';
        const isCollapsed = foldChar === '-';

        // 构建 callout 容器
        const callout = document.createElement('div');
        callout.className = `callout callout-${typeRaw.toLowerCase()}`;
        if (isFoldable) callout.classList.add('callout-foldable');
        if (isCollapsed) callout.classList.add('callout-collapsed');

        // 构建标题行
        const header = document.createElement('div');
        header.className = 'callout-header';
        header.innerHTML = `
            <span class="callout-icon">${getIcon(typeRaw)}</span>
            <span class="callout-title">${escapeHtml(titleText)}</span>
            ${isFoldable ? '<span class="callout-fold-icon">▾</span>' : ''}
        `;
        if (isFoldable) {
            header.style.cursor = 'pointer';
            header.addEventListener('click', () => {
                callout.classList.toggle('callout-collapsed');
            });
        }

        // 构建内容区
        const content = document.createElement('div');
        content.className = 'callout-content';

        // 将 bq 中的所有子节点移入内容区
        // 先移除 firstP 中的第一行（[!TYPE]...）文本，保留其余内容
        removeFirstLineFromParagraph(firstP, firstLine);

        // 若 firstP 处理后无内容，则删除它
        if (!firstP.textContent.trim() && firstP.childNodes.length === 0) {
            firstP.remove();
        }

        // 将 bq 的子节点移入 content
        while (bq.firstChild) {
            content.appendChild(bq.firstChild);
        }

        callout.appendChild(header);
        callout.appendChild(content);
        bq.parentNode.replaceChild(callout, bq);
    }

    /**
     * 从 <p> 元素中移除第一行文本（[!TYPE] 那行）
     * 保留段落中的后续内容
     */
    function removeFirstLineFromParagraph(p, firstLine) {
        // 遍历所有文本节点，找到包含 firstLine 的那个，截取掉它
        const children = Array.from(p.childNodes);
        let remaining = firstLine;

        for (const node of children) {
            if (remaining.length === 0) break;
            if (node.nodeType === Node.TEXT_NODE) {
                const text = node.textContent;
                if (text.includes(firstLine)) {
                    // 替换第一次出现的 firstLine（含可能的换行）
                    const idx = text.indexOf(firstLine);
                    const after = text.slice(idx + firstLine.length).replace(/^\n/, '');
                    node.textContent = after;
                    remaining = '';
                    break;
                }
            }
        }
    }

    /** 对字符串进行 HTML 转义 */
    function escapeHtml(str) {
        return str
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;');
    }

    // 初始化：DOM 加载完成后执行
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initCallouts);
    } else {
        initCallouts();
    }

    window.Callout = { init: initCallouts };
})();
