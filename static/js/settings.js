// 设置模块
(function() {
    const STORAGE_KEY = 'obsidian_mirror_settings';
    
    // 默认配置
    const DEFAULT_SETTINGS = {
        fontSize: 16,
        lineHeight: 1.6,
        fontFamily: 'system-ui',  // 默认使用系统字体
        language: 'zh-CN',        // 默认语言
        themePreset: 'none',      // 主题预设（none/warm/eye-care/high-contrast）
        accentColor: '',          // 自定义强调色（空字符串 = 使用主题默认）
        codeTheme: 'auto',        // 代码块主题（auto 跟随深/浅色）
        animations: true,         // 是否启用交互动画
    };
    
    // 常用字体列表
    const FONT_OPTIONS = [
        { value: 'system-ui', label: '系统默认', family: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif' },
        { value: 'serif', label: '衬线字体', family: 'Georgia, "Times New Roman", "Songti SC", "SimSun", serif' },
        { value: 'sans-serif', label: '无衬线字体', family: 'Arial, "Helvetica Neue", Helvetica, sans-serif' },
        { value: 'microsoft-yahei', label: '微软雅黑', family: '"Microsoft YaHei", "微软雅黑", sans-serif' },
        { value: 'simsun', label: '宋体', family: 'SimSun, "宋体", serif' },
        { value: 'simhei', label: '黑体', family: 'SimHei, "黑体", sans-serif' },
        { value: 'kaiti', label: '楷体', family: 'KaiTi, "楷体", serif' },
        { value: 'fangsong', label: '仿宋', family: 'FangSong, "仿宋", serif' },
        { value: 'noto-sans', label: 'Noto Sans CJK', family: '"Noto Sans CJK SC", "Noto Sans SC", sans-serif' },
        { value: 'source-han-sans', label: '思源黑体', family: '"Source Han Sans SC", "思源黑体", sans-serif' },
        { value: 'source-han-serif', label: '思源宋体', family: '"Source Han Serif SC", "思源宋体", serif' },
        { value: 'cascadia', label: 'Cascadia Code', family: '"Cascadia Code", "Cascadia Mono", monospace' },
        { value: 'fira-code', label: 'Fira Code', family: '"Fira Code", monospace' },
        { value: 'jetbrains-mono', label: 'JetBrains Mono', family: '"JetBrains Mono", monospace' },
    ];
    
    // 当前配置
    let currentSettings = { ...DEFAULT_SETTINGS };
    
    /**
     * 加载设置
     */
    function loadSettings() {
        try {
            const saved = localStorage.getItem(STORAGE_KEY);
            if (saved) {
                currentSettings = { ...DEFAULT_SETTINGS, ...JSON.parse(saved) };
            }
        } catch (e) {
            console.error('加载设置失败:', e);
        }
        return currentSettings;
    }
    
    /**
     * 保存设置
     */
    function saveSettings(settings) {
        try {
            currentSettings = { ...currentSettings, ...settings };
            localStorage.setItem(STORAGE_KEY, JSON.stringify(currentSettings));
        } catch (e) {
            console.error('保存设置失败:', e);
        }
    }
    
    /**
     * 应用设置
     */
    function applySettings(settings) {
        const root = document.documentElement;
        
        // 应用语言
        if (settings.language && window.i18n) {
            window.i18n.setLanguage(settings.language);
            window.i18n.translatePage();
        }

        // 应用主题预设
        if ('themePreset' in settings && typeof applyThemePreset === 'function') {
            applyThemePreset(settings.themePreset);
        }

        // 应用自定义强调色
        if ('accentColor' in settings && typeof applyAccentColor === 'function') {
            applyAccentColor(settings.accentColor || '');
        }

        // 应用代码块主题
        if ('codeTheme' in settings && typeof applyCodeTheme === 'function') {
            applyCodeTheme(settings.codeTheme);
        }

        // 应用动画开关
        if ('animations' in settings) {
            if (settings.animations) {
                document.documentElement.removeAttribute('data-animations');
            } else {
                document.documentElement.setAttribute('data-animations', 'off');
            }
        }

        // 应用字体
        if (settings.fontFamily) {
            const selectedFont = FONT_OPTIONS.find(f => f.value === settings.fontFamily);
            if (selectedFont) {
                root.style.setProperty('--font-text', selectedFont.family);
            }
        }
        
        // 应用字体大小
        if (settings.fontSize) {
            root.style.setProperty('--font-size-base', settings.fontSize + 'px');
            root.style.setProperty('--font-size-small', (settings.fontSize - 2) + 'px');
            root.style.setProperty('--font-size-large', (settings.fontSize + 2) + 'px');
            // UI 字体大小：保持在 12-16px 范围内，与正文成比例但不会太小
            const uiSize = Math.max(12, Math.min(16, settings.fontSize - 2));
            root.style.setProperty('--font-size-ui', uiSize + 'px');
        }
        
        // 应用行高
        if (settings.lineHeight) {
            root.style.setProperty('--line-height', settings.lineHeight);
        }
    }
    
    /**
     * 打开设置对话框
     */
    function openSettings() {
        let dialog = document.getElementById('settings-dialog');
        
        if (!dialog) {
            dialog = createSettingsDialog();
            document.body.appendChild(dialog);
        }
        
        // 更新表单值
        updateFormValues();
        
        dialog.classList.add('show');
        document.body.style.overflow = 'hidden';
    }
    
    /**
     * 关闭设置对话框
     */
    function closeSettings() {
        const dialog = document.getElementById('settings-dialog');
        if (dialog) {
            dialog.classList.remove('show');
            document.body.style.overflow = '';
        }
    }
    
    /**
     * 创建设置对话框
     */
    function createSettingsDialog() {
        const dialog = document.createElement('div');
        dialog.id = 'settings-dialog';
        dialog.className = 'settings-dialog';
        
        // 获取语言选项 HTML
        const languageOptions = window.i18n ? 
            Object.entries(window.i18n.LANGUAGES).map(([code, name]) => `
                <option value="${code}" ${currentSettings.language === code ? 'selected' : ''}>
                    ${name}
                </option>
            `).join('') : '';
        
        dialog.innerHTML = `
            <div class="settings-overlay" onclick="Settings.close()"></div>
            <div class="settings-content">
                <div class="settings-header">
                    <h2 data-i18n="settings.title">设置</h2>
                    <button class="settings-close" onclick="Settings.close()" data-i18n-attr="title" data-i18n="app.close">
                        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <line x1="18" y1="6" x2="6" y2="18"></line>
                            <line x1="6" y1="6" x2="18" y2="18"></line>
                        </svg>
                    </button>
                </div>
                
                <div class="settings-body">
                    <form id="settings-form">
                        <!-- 语言选择 -->
                        <div class="setting-group">
                            <label class="setting-label">
                                <span class="label-text" data-i18n="settings.language">语言</span>
                                <span class="label-desc" data-i18n="settings.language_desc">选择界面语言</span>
                            </label>
                            <div class="setting-control">
                                <select id="language-select" name="language" class="font-select">
                                    ${languageOptions}
                                </select>
                            </div>
                        </div>
                        
                        <!-- 字体选择 -->
                        <div class="setting-group">
                            <label class="setting-label">
                                <span class="label-text" data-i18n="settings.font">字体</span>
                                <span class="label-desc" data-i18n="settings.font_desc">选择阅读字体</span>
                            </label>
                            <div class="setting-control">
                                <select id="font-family" name="fontFamily" class="font-select">
                                    ${FONT_OPTIONS.map(font => `
                                        <option value="${font.value}" ${currentSettings.fontFamily === font.value ? 'selected' : ''} data-i18n="font.${font.value}">
                                            ${font.label}
                                        </option>
                                    `).join('')}
                                </select>
                            </div>
                        </div>
                        
                        <!-- 字体大小 -->
                        <div class="setting-group">
                            <label class="setting-label">
                                <span class="label-text" data-i18n="settings.font_size">字体大小</span>
                                <span class="label-desc" data-i18n="settings.font_size_desc">调整阅读字体大小</span>
                            </label>
                            <div class="setting-control">
                                <input type="range" id="font-size" name="fontSize" min="12" max="24" step="1" value="${currentSettings.fontSize}">
                                <span class="range-value" id="font-size-value">${currentSettings.fontSize}px</span>
                            </div>
                        </div>
                        
                        <!-- 行高 -->
                        <div class="setting-group">
                            <label class="setting-label">
                                <span class="label-text" data-i18n="settings.line_height">行高</span>
                                <span class="label-desc" data-i18n="settings.line_height_desc">调整文本行间距</span>
                            </label>
                            <div class="setting-control">
                                <input type="range" id="line-height" name="lineHeight" min="1.2" max="2.0" step="0.1" value="${currentSettings.lineHeight}">
                                <span class="range-value" id="line-height-value">${currentSettings.lineHeight}</span>
                            </div>
                        </div>

                        <!-- 主题预设 -->
                        <div class="setting-group">
                            <label class="setting-label">
                                <span class="label-text" data-i18n="settings.theme_preset">主题预设</span>
                                <span class="label-desc" data-i18n="settings.theme_preset_desc">叠加在深色/浅色基础上的配色方案</span>
                            </label>
                            <div class="setting-control">
                                <div class="theme-preset-options" id="theme-preset-options">
                                    <button class="theme-preset-btn ${currentSettings.themePreset === 'none' ? 'active' : ''}" data-preset="none" data-i18n="settings.preset_none">默认</button>
                                    <button class="theme-preset-btn ${currentSettings.themePreset === 'warm' ? 'active' : ''}" data-preset="warm" data-i18n="settings.preset_warm">暖色</button>
                                    <button class="theme-preset-btn ${currentSettings.themePreset === 'eye-care' ? 'active' : ''}" data-preset="eye-care" data-i18n="settings.preset_eye_care">护眼</button>
                                    <button class="theme-preset-btn ${currentSettings.themePreset === 'high-contrast' ? 'active' : ''}" data-preset="high-contrast" data-i18n="settings.preset_high_contrast">高对比</button>
                                </div>
                            </div>
                        </div>

                        <!-- 强调色 -->
                        <div class="setting-group">
                            <label class="setting-label">
                                <span class="label-text" data-i18n="settings.accent_color">强调色</span>
                                <span class="label-desc" data-i18n="settings.accent_color_desc">影响链接、活跃标签和高亮颜色</span>
                            </label>
                            <div class="setting-control">
                                <div class="accent-color-row">
                                    <input type="color" id="accent-color" class="accent-color-input"
                                           value="${currentSettings.accentColor || '#6a5acd'}">
                                    <button class="accent-color-reset" onclick="Settings.resetAccentColor()" data-i18n="settings.accent_color_reset">
                                        重置默认
                                    </button>
                                </div>
                            </div>
                        </div>

                        <!-- 代码块主题 -->
                        <div class="setting-group">
                            <label class="setting-label">
                                <span class="label-text" data-i18n="settings.code_theme">代码块主题</span>
                                <span class="label-desc" data-i18n="settings.code_theme_desc">选择代码语法高亮配色</span>
                            </label>
                            <div class="setting-control">
                                <select id="code-theme" name="codeTheme" class="font-select">
                                    <option value="auto" ${currentSettings.codeTheme === 'auto' ? 'selected' : ''} data-i18n="settings.code_theme_auto">自动（跟随深/浅色）</option>
                                    <option value="atom-one-dark"  ${currentSettings.codeTheme === 'atom-one-dark'  ? 'selected' : ''}>Atom One Dark</option>
                                    <option value="atom-one-light" ${currentSettings.codeTheme === 'atom-one-light' ? 'selected' : ''}>Atom One Light</option>
                                    <option value="github"      ${currentSettings.codeTheme === 'github'      ? 'selected' : ''}>GitHub Light</option>
                                    <option value="github-dark" ${currentSettings.codeTheme === 'github-dark' ? 'selected' : ''}>GitHub Dark</option>
                                    <option value="dracula"     ${currentSettings.codeTheme === 'dracula'     ? 'selected' : ''}>Dracula</option>
                                    <option value="monokai"     ${currentSettings.codeTheme === 'monokai'     ? 'selected' : ''}>Monokai</option>
                                </select>
                            </div>
                        </div>

                        <!-- 交互动画 -->
                        <div class="setting-group">
                            <label class="setting-label">
                                <span class="label-text" data-i18n="settings.animations">交互动画</span>
                                <span class="label-desc" data-i18n="settings.animations_desc">页面淡入、搜索结果错峰进入等动画效果</span>
                            </label>
                            <div class="setting-control">
                                <label class="toggle-switch">
                                    <input type="checkbox" id="animations-toggle" ${currentSettings.animations !== false ? 'checked' : ''}>
                                    <span class="toggle-slider"></span>
                                </label>
                            </div>
                        </div>
                    </form>
                </div>
                
                <div class="settings-footer">
                    <button type="button" class="btn btn-secondary" onclick="Settings.reset()" data-i18n="settings.reset_default">重置默认</button>
                    <button type="button" class="btn btn-primary" onclick="Settings.apply()" data-i18n="settings.apply_settings">应用设置</button>
                </div>
            </div>
        `;
        
        // 绑定实时预览事件
        setTimeout(() => {
            bindPreviewEvents();
            // 翻译对话框内容
            if (window.i18n) {
                window.i18n.translatePage();
            }
        }, 0);
        
        return dialog;
    }
    
    /**
     * 绑定实时预览事件
     */
    function bindPreviewEvents() {
        const form = document.getElementById('settings-form');
        if (!form) return;
        
        // 语言选择
        const languageSelect = document.getElementById('language-select');
        if (languageSelect) {
            languageSelect.addEventListener('change', (e) => {
                const value = e.target.value;
                currentSettings.language = value;
                applySettings({ language: value });
            });
        }
        
        // 字体选择
        const fontFamilySelect = document.getElementById('font-family');
        if (fontFamilySelect) {
            fontFamilySelect.addEventListener('change', (e) => {
                const value = e.target.value;
                currentSettings.fontFamily = value;
                applySettings({ fontFamily: value });
            });
        }
        
        // 字体大小滑块
        const fontSizeInput = document.getElementById('font-size');
        const fontSizeValue = document.getElementById('font-size-value');
        if (fontSizeInput && fontSizeValue) {
            fontSizeInput.addEventListener('input', (e) => {
                const value = parseInt(e.target.value);
                fontSizeValue.textContent = value + 'px';
                currentSettings.fontSize = value;
                applySettings({ fontSize: value });
            });
        }
        
        // 行高滑块
        const lineHeightInput = document.getElementById('line-height');
        const lineHeightValue = document.getElementById('line-height-value');
        if (lineHeightInput && lineHeightValue) {
            lineHeightInput.addEventListener('input', (e) => {
                const value = parseFloat(e.target.value);
                lineHeightValue.textContent = value.toFixed(1);
                currentSettings.lineHeight = value;
                applySettings({ lineHeight: value });
            });
        }

        // 主题预设按钮
        const presetContainer = document.getElementById('theme-preset-options');
        if (presetContainer) {
            presetContainer.addEventListener('click', (e) => {
                const btn = e.target.closest('.theme-preset-btn');
                if (!btn) return;
                const preset = btn.getAttribute('data-preset');
                presetContainer.querySelectorAll('.theme-preset-btn').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
                currentSettings.themePreset = preset;
                applySettings({ themePreset: preset });
            });
        }

        // 强调色选择器
        const accentInput = document.getElementById('accent-color');
        if (accentInput) {
            accentInput.addEventListener('input', (e) => {
                currentSettings.accentColor = e.target.value;
                applySettings({ accentColor: e.target.value });
            });
        }

        // 代码块主题选择
        const codeThemeSelect = document.getElementById('code-theme');
        if (codeThemeSelect) {
            codeThemeSelect.addEventListener('change', (e) => {
                currentSettings.codeTheme = e.target.value;
                applySettings({ codeTheme: e.target.value });
            });
        }

        // 动画开关
        const animToggle = document.getElementById('animations-toggle');
        if (animToggle) {
            animToggle.addEventListener('change', (e) => {
                currentSettings.animations = e.target.checked;
                applySettings({ animations: e.target.checked });
            });
        }
    }
    
    /**
     * 更新表单值
     */
    function updateFormValues() {
        const languageSelect = document.getElementById('language-select');
        const fontFamilySelect = document.getElementById('font-family');
        const fontSizeInput = document.getElementById('font-size');
        const fontSizeValue = document.getElementById('font-size-value');
        const lineHeightInput = document.getElementById('line-height');
        const lineHeightValue = document.getElementById('line-height-value');
        
        if (languageSelect) languageSelect.value = currentSettings.language;
        if (fontFamilySelect) fontFamilySelect.value = currentSettings.fontFamily;
        if (fontSizeInput) fontSizeInput.value = currentSettings.fontSize;
        if (fontSizeValue) fontSizeValue.textContent = currentSettings.fontSize + 'px';
        if (lineHeightInput) lineHeightInput.value = currentSettings.lineHeight;
        if (lineHeightValue) lineHeightValue.textContent = currentSettings.lineHeight.toFixed(1);
    }
    
    /**
     * 应用设置并保存
     */
    function apply() {
        saveSettings(currentSettings);
        closeSettings();
        
        // 显示提示
        const toast = document.getElementById('toast-message') || createToast();
        toast.textContent = window.i18n ? window.i18n.t('settings.settings_saved') : '设置已保存';
        toast.classList.add('show');
        setTimeout(() => toast.classList.remove('show'), 2000);
    }
    
    /**
     * 创建 Toast 元素
     */
    function createToast() {
        const toast = document.createElement('div');
        toast.id = 'toast-message';
        toast.className = 'toast-message';
        document.body.appendChild(toast);
        return toast;
    }
    
    /**
     * 重置为默认设置
     */
    function reset() {
        const confirmMsg = window.i18n ? window.i18n.t('settings.reset_confirm') : '确定要重置为默认设置吗？';
        if (!confirm(confirmMsg)) {
            return;
        }
        
        currentSettings = { ...DEFAULT_SETTINGS };
        applySettings(currentSettings);
        updateFormValues();
        saveSettings(currentSettings);
    }
    
    /**
     * 初始化设置
     */
    function init() {
        // 加载保存的配置
        loadSettings();
        
        // 应用配置
        applySettings(currentSettings);
    }
    
    /**
     * 重置强调色为主题默认
     */
    function resetAccentColor() {
        currentSettings.accentColor = '';
        applySettings({ accentColor: '' });
        const accentInput = document.getElementById('accent-color');
        if (accentInput) accentInput.value = '#6a5acd';
    }

    // 导出公共接口
    window.Settings = {
        open: openSettings,
        close: closeSettings,
        apply: apply,
        reset: reset,
        init: init,
        resetAccentColor: resetAccentColor,
    };
    
    // 页面加载完成后初始化
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
