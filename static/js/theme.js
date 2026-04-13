// ==========================================
// 主题管理模块
// ==========================================

const THEME_KEY = 'obsidian_mirror_theme';

/**
 * 更新代码高亮主题
 */
function updateHighlightTheme(theme) {
    const darkTheme = document.getElementById('highlight-theme-dark');
    const lightTheme = document.getElementById('highlight-theme-light');

    if (theme === 'dark') {
        if (darkTheme) darkTheme.disabled = false;
        if (lightTheme) lightTheme.disabled = true;
    } else {
        if (darkTheme) darkTheme.disabled = true;
        if (lightTheme) lightTheme.disabled = false;
    }
}

/**
 * 更新主题切换按钮图标
 */
function updateThemeIcon(theme) {
    const sun = document.querySelector('.icon-sun');
    const moon = document.querySelector('.icon-moon');
    if (!sun || !moon) return;

    if (theme === 'dark') {
        sun.style.display = 'block';
        moon.style.display = 'none';
    } else {
        sun.style.display = 'none';
        moon.style.display = 'block';
    }

    updateHighlightTheme(theme);
}

/**
 * 初始化主题（根据本地存储或系统偏好）
 */
function initTheme() {
    let savedTheme = null;
    try {
        savedTheme = localStorage.getItem(THEME_KEY);
    } catch (e) {
        console.warn('localStorage access failed:', e);
    }

    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;

    let theme = savedTheme;
    if (!theme) {
        theme = prefersDark ? 'dark' : 'light';
    }

    document.documentElement.setAttribute('data-theme', theme);
    updateThemeIcon(theme);
    updateHighlightTheme(theme);
}

/**
 * 切换主题（亮色/暗色）
 */
function toggleTheme() {
    const current = document.documentElement.getAttribute('data-theme');
    const newTheme = current === 'dark' ? 'light' : 'dark';

    document.documentElement.setAttribute('data-theme', newTheme);
    try {
        localStorage.setItem(THEME_KEY, newTheme);
    } catch (e) {}
    updateThemeIcon(newTheme);
    
    // 通知 Mermaid 主题已切换
    if (window.MermaidManager) {
        window.MermaidManager.switchTheme(newTheme);
    }
}

// 立即初始化主题（避免页面闪烁）
initTheme();
