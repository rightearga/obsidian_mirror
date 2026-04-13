// ==========================================
// 图片灯箱模块 — v1.4.1
// 点击 .markdown-body 内的图片显示全屏放大效果
// ==========================================

(function () {
    'use strict';

    let imageList = [];   // 当前页面所有笔记图片
    let currentIndex = 0; // 当前显示的图片索引

    // ==========================================
    // 初始化
    // ==========================================

    function initLightbox() {
        const body = document.querySelector('.markdown-body');
        if (!body) return;

        // 收集所有图片（排除无 src 的图片）
        imageList = Array.from(body.querySelectorAll('img[src]')).filter(
            (img) => !img.closest('.callout-icon') // 排除 callout 图标
        );
        if (imageList.length === 0) return;

        // 创建模态 DOM（只创建一次）
        if (!document.getElementById('lightbox-modal')) {
            createModal();
        }

        // 为每张图片绑定点击
        imageList.forEach((img, i) => {
            img.style.cursor = 'zoom-in';
            img.addEventListener('click', () => open(i));
        });
    }

    // ==========================================
    // DOM 创建
    // ==========================================

    function createModal() {
        const modal = document.createElement('div');
        modal.id = 'lightbox-modal';
        modal.className = 'lightbox-modal';
        modal.setAttribute('role', 'dialog');
        modal.setAttribute('aria-modal', 'true');
        modal.setAttribute('aria-label', '图片放大查看');

        modal.innerHTML = `
            <div class="lightbox-overlay" id="lightbox-overlay"></div>
            <div class="lightbox-container">
                <img class="lightbox-img" id="lightbox-img" src="" alt="" draggable="false">
                <div class="lightbox-caption" id="lightbox-caption"></div>
                <div class="lightbox-counter" id="lightbox-counter"></div>
            </div>
            <button class="lightbox-btn lightbox-prev" id="lightbox-prev" aria-label="上一张">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="15 18 9 12 15 6"/></svg>
            </button>
            <button class="lightbox-btn lightbox-next" id="lightbox-next" aria-label="下一张">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="9 18 15 12 9 6"/></svg>
            </button>
            <button class="lightbox-btn lightbox-close" id="lightbox-close" aria-label="关闭">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
            </button>
        `;
        document.body.appendChild(modal);

        // 绑定事件
        document.getElementById('lightbox-overlay').addEventListener('click', close);
        document.getElementById('lightbox-close').addEventListener('click', close);
        document.getElementById('lightbox-prev').addEventListener('click', (e) => { e.stopPropagation(); prev(); });
        document.getElementById('lightbox-next').addEventListener('click', (e) => { e.stopPropagation(); next(); });

        // 键盘导航
        document.addEventListener('keydown', handleKeydown);
    }

    // ==========================================
    // 显示控制
    // ==========================================

    function open(index) {
        currentIndex = index;
        renderImage();
        document.getElementById('lightbox-modal').classList.add('show');
        document.body.style.overflow = 'hidden';
    }

    function close() {
        const modal = document.getElementById('lightbox-modal');
        if (modal) {
            modal.classList.remove('show');
            document.body.style.overflow = '';
        }
    }

    function prev() {
        currentIndex = (currentIndex - 1 + imageList.length) % imageList.length;
        renderImage();
    }

    function next() {
        currentIndex = (currentIndex + 1) % imageList.length;
        renderImage();
    }

    function renderImage() {
        const img = imageList[currentIndex];
        const lightboxImg = document.getElementById('lightbox-img');
        const caption = document.getElementById('lightbox-caption');
        const counter = document.getElementById('lightbox-counter');
        const prevBtn = document.getElementById('lightbox-prev');
        const nextBtn = document.getElementById('lightbox-next');

        if (!lightboxImg) return;

        lightboxImg.src = img.src;
        lightboxImg.alt = img.alt || '';
        if (caption) caption.textContent = img.alt || '';
        if (counter) {
            counter.textContent = imageList.length > 1
                ? `${currentIndex + 1} / ${imageList.length}`
                : '';
        }
        // 仅有多张图时显示前后按钮
        const multi = imageList.length > 1;
        if (prevBtn) prevBtn.style.display = multi ? '' : 'none';
        if (nextBtn) nextBtn.style.display = multi ? '' : 'none';
    }

    // ==========================================
    // 键盘处理
    // ==========================================

    function handleKeydown(e) {
        const modal = document.getElementById('lightbox-modal');
        if (!modal || !modal.classList.contains('show')) return;

        switch (e.key) {
            case 'Escape':    close(); break;
            case 'ArrowLeft': prev();  break;
            case 'ArrowRight':next();  break;
        }
    }

    // ==========================================
    // 初始化入口
    // ==========================================

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initLightbox);
    } else {
        initLightbox();
    }

    window.Lightbox = { open, close, prev, next, init: initLightbox };
})();
