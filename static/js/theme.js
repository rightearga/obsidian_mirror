// ==========================================
// 主题管理模块 — v1.4.0 扩展
// ==========================================

const THEME_KEY          = 'obsidian_mirror_theme';
const THEME_PRESET_KEY   = 'obsidian_mirror_theme_preset';
const ACCENT_COLOR_KEY   = 'obsidian_mirror_accent_color';
const CODE_THEME_KEY     = 'obsidian_mirror_code_theme';

// 代码块可用主题（与 layout.html 中 <link id="hljs-opt-*"> 对应）
const CODE_THEMES = {
    'auto':         null,                  // 随深/浅色自动切换
    'atom-one-dark':  'atom-one-dark',
    'atom-one-light': 'atom-one-light',
    'github':       'github',
    'github-dark':  'github-dark',
    'dracula':      'dracula',
    'monokai':      'monokai',
};

/**
 * 更新代码高亮主题
 * 优先使用用户选择的固定代码主题；若为 auto，则跟随深/浅色模式。
 */
function updateHighlightTheme(darkOrLight) {
    let codeThemePref = 'auto';
    try { codeThemePref = localStorage.getItem(CODE_THEME_KEY) || 'auto'; } catch (e) {}

    // 禁用所有可选主题
    document.querySelectorAll('[id^="hljs-opt-"]').forEach(el => { el.disabled = true; });

    if (codeThemePref !== 'auto' && CODE_THEMES[codeThemePref]) {
        // 用户选择了固定主题
        const el = document.getElementById('hljs-opt-' + codeThemePref);
        if (el) { el.disabled = false; }
        // 同时关闭默认的 dark/light 链接
        const darkEl  = document.getElementById('highlight-theme-dark');
        const lightEl = document.getElementById('highlight-theme-light');
        if (darkEl)  darkEl.disabled  = true;
        if (lightEl) lightEl.disabled = true;
    } else {
        // auto：跟随深/浅色
        const darkEl  = document.getElementById('highlight-theme-dark');
        const lightEl = document.getElementById('highlight-theme-light');
        if (darkOrLight === 'dark') {
            if (darkEl)  darkEl.disabled  = false;
            if (lightEl) lightEl.disabled = true;
        } else {
            if (darkEl)  darkEl.disabled  = true;
            if (lightEl) lightEl.disabled = false;
        }
    }
}

/**
 * 应用主题预设（在 <html> 上设置 data-theme-preset 属性）
 */
function applyThemePreset(preset) {
    const html = document.documentElement;
    if (preset && preset !== 'none') {
        html.setAttribute('data-theme-preset', preset);
    } else {
        html.removeAttribute('data-theme-preset');
    }
    try { localStorage.setItem(THEME_PRESET_KEY, preset || 'none'); } catch (e) {}
}

/**
 * 初始化主题预设（从 localStorage 恢复）
 */
function initThemePreset() {
    let saved = 'none';
    try { saved = localStorage.getItem(THEME_PRESET_KEY) || 'none'; } catch (e) {}
    applyThemePreset(saved);
}

/**
 * 应用自定义强调色
 */
function applyAccentColor(color) {
    const html = document.documentElement;
    if (color) {
        html.style.setProperty('--accent-color', color);
        html.style.setProperty('--link-color', color);
        html.style.setProperty('--primary-color', color);
        html.setAttribute('data-accent', '1');
        try { localStorage.setItem(ACCENT_COLOR_KEY, color); } catch (e) {}
    } else {
        html.style.removeProperty('--accent-color');
        html.style.removeProperty('--link-color');
        html.style.removeProperty('--primary-color');
        html.removeAttribute('data-accent');
        try { localStorage.removeItem(ACCENT_COLOR_KEY); } catch (e) {}
    }
}

/**
 * 初始化强调色（从 localStorage 恢复）
 */
function initAccentColor() {
    let saved = '';
    try { saved = localStorage.getItem(ACCENT_COLOR_KEY) || ''; } catch (e) {}
    if (saved) applyAccentColor(saved);
}

/**
 * 应用代码块主题（更新高亮 CSS 链接）
 */
function applyCodeTheme(theme) {
    try { localStorage.setItem(CODE_THEME_KEY, theme || 'auto'); } catch (e) {}
    const current = document.documentElement.getAttribute('data-theme') || 'light';
    updateHighlightTheme(current);
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
    // 同步更新 PWA meta theme-color（深色/浅色切换时跟随）
    const metaThemeColor = document.getElementById('meta-theme-color');
    if (metaThemeColor) {
        metaThemeColor.content = newTheme === 'dark' ? '#202020' : '#6a5acd';
    }
    
    // 通知 Mermaid 主题已切换
    if (window.MermaidManager) {
        window.MermaidManager.switchTheme(newTheme);
    }
}

// 立即初始化主题（避免页面闪烁）
initTheme();
// 初始化主题预设和强调色
initThemePreset();
initAccentColor();
