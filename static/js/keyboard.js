// ==========================================
// 键盘快捷键模块 — v1.4.0
// ==========================================

(function () {
    'use strict';

    // gg 序列检测状态
    const GG_TIMEOUT = 450;    // 双击 g 判定窗口（毫秒）
    let lastKey = null;
    let lastKeyTime = 0;
    let gTimer = null;          // 单击 g → 打开图谱的延迟定时器

    // ==========================================
    // 滚动辅助
    // ==========================================

    /** 获取当前可滚动容器（桌面端为内层 div，移动端为 window） */
    function getScrollContainer() {
        return document.querySelector('.page-scrollable-content') || window;
    }

    /** 平滑滚动指定像素量 */
    function smoothScrollBy(delta) {
        const container = getScrollContainer();
        if (container === window) {
            window.scrollBy({ top: delta, behavior: 'smooth' });
        } else {
            container.scrollBy({ top: delta, behavior: 'smooth' });
        }
    }

    /** 平滑滚动到顶部 */
    function scrollToTop() {
        const container = getScrollContainer();
        if (container === window) {
            window.scrollTo({ top: 0, behavior: 'smooth' });
        } else {
            container.scrollTo({ top: 0, behavior: 'smooth' });
        }
    }

    /** 平滑滚动到底部 */
    function scrollToBottom() {
        const container = getScrollContainer();
        if (container === window) {
            window.scrollTo({ top: document.body.scrollHeight, behavior: 'smooth' });
        } else {
            container.scrollTo({ top: container.scrollHeight, behavior: 'smooth' });
        }
    }

    // ==========================================
    // 笔记导航
    // ==========================================

    /**
     * 获取侧边栏中可见文件链接列表
     * 只包含未折叠（display 不为 none）的文件条目
     */
    function getVisibleFileLinks() {
        return Array.from(document.querySelectorAll('.tree-row.file')).filter(
            (el) => el.offsetHeight > 0
        );
    }

    /** 导航到相邻笔记（direction: -1 上一篇，+1 下一篇） */
    function navigateAdjacentNote(direction) {
        const links = getVisibleFileLinks();
        if (links.length === 0) return;

        const currentHref = decodeURIComponent(window.location.pathname);
        const activeIndex = links.findIndex(
            (l) => decodeURIComponent(l.getAttribute('href') || '') === currentHref
        );

        if (activeIndex === -1) return;

        const targetIndex = activeIndex + direction;
        if (targetIndex < 0 || targetIndex >= links.length) return;

        window.location.href = links[targetIndex].getAttribute('href');
    }

    // ==========================================
    // 图谱与 TOC
    // ==========================================

    /** 触发关系图谱（如果 graph.js 已加载） */
    function openGraph() {
        if (typeof toggleGraphView === 'function') {
            toggleGraphView();
        }
    }

    /** 切换 TOC 展开/收起 */
    function toggleTocPanel() {
        const isMobile = window.innerWidth <= 768;
        if (isMobile) {
            if (typeof toggleMobileTocSidebar === 'function') {
                toggleMobileTocSidebar();
            }
        } else {
            if (typeof toggleDesktopToc === 'function') {
                toggleDesktopToc();
            }
        }
    }

    // ==========================================
    // 帮助面板
    // ==========================================

    /** 切换快捷键帮助面板 */
    function toggleHelpModal() {
        let modal = document.getElementById('keyboard-help-modal');
        if (!modal) {
            modal = createHelpModal();
            document.body.appendChild(modal);
            // 稍后翻译（等待 i18n 初始化）
            requestAnimationFrame(() => {
                if (window.i18n) window.i18n.translatePage();
            });
        }
        if (modal.classList.contains('show')) {
            closeHelpModal();
        } else {
            modal.classList.add('show');
            document.body.style.overflow = 'hidden';
        }
    }

    /** 关闭快捷键帮助面板 */
    function closeHelpModal() {
        const modal = document.getElementById('keyboard-help-modal');
        if (modal && modal.classList.contains('show')) {
            modal.classList.remove('show');
            document.body.style.overflow = '';
        }
    }

    /** 创建帮助面板 DOM */
    function createHelpModal() {
        const modal = document.createElement('div');
        modal.id = 'keyboard-help-modal';
        modal.className = 'keyboard-help-modal';
        modal.setAttribute('role', 'dialog');
        modal.setAttribute('aria-modal', 'true');
        modal.setAttribute('aria-label', '键盘快捷键');

        modal.innerHTML = `
            <div class="keyboard-help-overlay" onclick="window.Keyboard && window.Keyboard.closeHelp()"></div>
            <div class="keyboard-help-content">
                <div class="keyboard-help-header">
                    <span class="keyboard-help-title" data-i18n="keyboard.title">键盘快捷键</span>
                    <button class="keyboard-help-close"
                            onclick="window.Keyboard && window.Keyboard.closeHelp()"
                            aria-label="关闭">×</button>
                </div>
                <div class="keyboard-help-body">
                    <div class="keyboard-help-section">
                        <h4 data-i18n="keyboard.section_nav">导航</h4>
                        <div class="key-row">
                            <div class="key-combo"><kbd>j</kbd> / <kbd>k</kbd></div>
                            <span data-i18n="keyboard.scroll_down_up">向下 / 向上滚动半屏</span>
                        </div>
                        <div class="key-row">
                            <div class="key-combo"><kbd>g</kbd><kbd>g</kbd></div>
                            <span data-i18n="keyboard.go_top">跳转到页面顶部</span>
                        </div>
                        <div class="key-row">
                            <div class="key-combo"><kbd>G</kbd></div>
                            <span data-i18n="keyboard.go_bottom">跳转到页面底部</span>
                        </div>
                        <div class="key-row">
                            <div class="key-combo"><kbd>[</kbd> / <kbd>]</kbd></div>
                            <span data-i18n="keyboard.prev_next_note">前 / 后一篇笔记</span>
                        </div>
                        <div class="key-row">
                            <div class="key-combo"><kbd>b</kbd></div>
                            <span data-i18n="keyboard.back">返回上一页</span>
                        </div>
                    </div>
                    <div class="keyboard-help-section">
                        <h4 data-i18n="keyboard.section_feature">功能</h4>
                        <div class="key-row">
                            <div class="key-combo"><kbd>g</kbd></div>
                            <span data-i18n="keyboard.open_graph">打开关系图谱（单击 g，不跟第二下）</span>
                        </div>
                        <div class="key-row">
                            <div class="key-combo"><kbd>t</kbd></div>
                            <span data-i18n="keyboard.toggle_toc">切换目录（TOC）展开/收起</span>
                        </div>
                        <div class="key-row">
                            <div class="key-combo"><kbd>r</kbd></div>
                            <span data-i18n="keyboard.random_note">随机跳转到一篇笔记</span>
                        </div>
                        <div class="key-row">
                            <div class="key-combo"><kbd>Ctrl</kbd>+<kbd>K</kbd></div>
                            <span data-i18n="keyboard.open_search">打开搜索</span>
                        </div>
                        <div class="key-row">
                            <div class="key-combo"><kbd>?</kbd></div>
                            <span data-i18n="keyboard.show_help">显示 / 关闭本帮助面板</span>
                        </div>
                        <div class="key-row">
                            <div class="key-combo"><kbd>Esc</kbd></div>
                            <span data-i18n="keyboard.close_modal">关闭弹窗</span>
                        </div>
                    </div>
                </div>
                <div class="keyboard-help-footer" data-i18n="keyboard.footer_hint">
                    在输入框中快捷键自动停用
                </div>
            </div>
        `;
        return modal;
    }

    // ==========================================
    // 判断是否应忽略快捷键
    // ==========================================

    /** 当焦点在输入类元素时，大部分快捷键应停用 */
    function shouldIgnore(e) {
        const tag = (document.activeElement.tagName || '').toLowerCase();
        if (['input', 'textarea', 'select'].includes(tag)) return true;
        if (document.activeElement.isContentEditable) return true;
        // 若任意模态弹窗（搜索/设置/图谱）处于活跃状态（除帮助面板外），也忽略
        const modals = [
            document.getElementById('search-modal'),
            document.getElementById('settings-dialog'),
            document.getElementById('graph-modal'),
        ];
        for (const m of modals) {
            if (m && (m.style.display === 'flex' || m.classList.contains('show'))) {
                return true;
            }
        }
        return false;
    }

    // ==========================================
    // 主键盘处理器
    // ==========================================

    function handleKeydown(e) {
        // 带 Ctrl/Alt/Meta 修饰键的由浏览器或搜索模块处理
        if (e.ctrlKey || e.altKey || e.metaKey) return;

        const key = e.key;

        // Esc 特殊处理：总是关闭帮助面板（不受 shouldIgnore 影响）
        if (key === 'Escape') {
            closeHelpModal();
            return;
        }

        if (shouldIgnore(e)) return;

        const now = Date.now();

        switch (key) {
            // ---- 滚动 ----
            case 'j':
                smoothScrollBy(window.innerHeight * 0.45);
                e.preventDefault();
                break;

            case 'k':
                smoothScrollBy(-window.innerHeight * 0.45);
                e.preventDefault();
                break;

            // ---- 底部（Shift+G） ----
            case 'G':
                scrollToBottom();
                e.preventDefault();
                break;

            // ---- g / gg ----
            case 'g': {
                clearTimeout(gTimer);
                if (lastKey === 'g' && now - lastKeyTime < GG_TIMEOUT) {
                    // gg → 顶部
                    scrollToTop();
                    lastKey = null;
                    lastKeyTime = 0;
                } else {
                    // 等待下一次 g；超时后打开图谱
                    lastKey = 'g';
                    lastKeyTime = now;
                    gTimer = setTimeout(() => {
                        openGraph();
                        lastKey = null;
                        lastKeyTime = 0;
                        gTimer = null;
                    }, GG_TIMEOUT);
                }
                e.preventDefault();
                return; // 不重置 lastKey
            }

            // ---- 前/后一篇 ----
            case '[':
                navigateAdjacentNote(-1);
                e.preventDefault();
                break;

            case ']':
                navigateAdjacentNote(1);
                e.preventDefault();
                break;

            // ---- 回退 ----
            case 'b':
                window.history.back();
                e.preventDefault();
                break;

            // ---- TOC ----
            case 't':
                toggleTocPanel();
                e.preventDefault();
                break;

            // ---- 随机笔记 ----
            case 'r':
                window.location.href = '/random';
                e.preventDefault();
                break;

            // ---- 帮助面板 ----
            case '?':
                toggleHelpModal();
                e.preventDefault();
                break;

            default:
                break;
        }

        // 重置 gg 状态（仅当本次按键不是 g）
        if (key !== 'g') {
            lastKey = null;
            lastKeyTime = 0;
            clearTimeout(gTimer);
            gTimer = null;
        }
    }

    // ==========================================
    // 初始化
    // ==========================================

    function init() {
        document.addEventListener('keydown', handleKeydown);
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }

    // 导出公共接口
    window.Keyboard = {
        toggleHelp: toggleHelpModal,
        closeHelp: closeHelpModal,
    };
})();
