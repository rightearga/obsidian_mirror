// 国际化（i18n）多语言支持模块
(function() {
    const STORAGE_KEY = 'obsidian_mirror_language';
    const DEFAULT_LANG = 'zh-CN';
    
    // 语言配置
    const LANGUAGES = {
        'zh-CN': '简体中文',
        'en-US': 'English'
    };
    
    // 翻译字典
    const translations = {
        'zh-CN': {
            // 通用
            'app.name': 'Obsidian Mirror',
            'app.loading': '加载中...',
            'app.error': '错误',
            'app.success': '成功',
            'app.cancel': '取消',
            'app.confirm': '确认',
            'app.close': '关闭',
            'app.apply': '应用',
            'app.reset': '重置',
            'app.save': '保存',
            'app.delete': '删除',
            'app.edit': '编辑',
            'app.search': '搜索',
            'app.filter': '过滤',
            'app.sort': '排序',
            'app.back': '返回',
            'app.next': '下一步',
            'app.previous': '上一步',
            
            // 侧边栏
            'sidebar.files': '文件',
            'sidebar.tab_overview': '概览',
            'sidebar.tab_notes': '笔记',
            'sidebar.sync': '同步',
            'sidebar.syncing': '同步中...',
            'sidebar.sync_success': '同步成功',
            'sidebar.sync_failed': '同步失败',
            'sidebar.tags': '标签列表',
            'sidebar.search': '搜索',
            'sidebar.settings': '设置',
            'sidebar.theme': '切换主题',
            'sidebar.user_menu': '用户菜单',
            'sidebar.logout': '登出',
            'sidebar.change_password': '修改密码',
            
            // 统计面板
            'stats.total_notes': '笔记总数',
            'stats.total_words': '总字数',
            'stats.total_tags': '标签数量',
            'stats.recent_notes': '最近访问',
            'stats.favorites': '收藏夹',
            'stats.word_count': '{count} 字',
            'stats.note_count': '{count} 篇',
            'stats.tag_count': '{count} 个',
            
            // 最近访问
            'recent.title': '最近访问',
            'recent.empty': '暂无访问记录',
            'recent.clear': '清空记录',
            'recent.clear_confirm': '确定要清空所有访问记录吗？',
            
            // 收藏夹
            'favorites.title': '收藏夹',
            'favorites.empty': '暂无收藏',
            'favorites.add': '添加收藏',
            'favorites.remove': '移除收藏',
            'favorites.added': '已添加到收藏夹',
            'favorites.removed': '已从收藏夹移除',
            
            // 搜索
            'search.title': '搜索笔记',
            'search.placeholder': '输入关键词搜索...',
            'search.empty': '输入关键词开始搜索',
            'search.no_results': '没有找到相关笔记',
            'search.results': '找到 {count} 个结果',
            'search.sort_relevance': '相关度',
            'search.sort_modified': '最新修改',
            'search.sort_label': '排序方式：',
            'search.tips': '提示: 使用',
            'search.shortcut_open': '快速打开搜索',
            'search.shortcut_select': '选择',
            'search.shortcut_enter': '打开',
            'search.shortcut_close': '关闭',
            
            // 设置
            'settings.title': '设置',
            'settings.appearance': '外观',
            'settings.language': '语言',
            'settings.language_desc': '选择界面语言',
            'settings.font': '字体',
            'settings.font_desc': '选择阅读字体',
            'settings.font_size': '字体大小',
            'settings.font_size_desc': '调整阅读字体大小',
            'settings.line_height': '行高',
            'settings.line_height_desc': '调整文本行间距',
            'settings.reset_default': '重置默认',
            'settings.reset_confirm': '确定要重置为默认设置吗？',
            'settings.apply_settings': '应用设置',
            'settings.settings_saved': '设置已保存',
            
            // 字体选项
            'font.system': '系统默认',
            'font.serif': '衬线字体',
            'font.sans-serif': '无衬线字体',
            'font.microsoft-yahei': '微软雅黑',
            'font.simsun': '宋体',
            'font.simhei': '黑体',
            'font.kaiti': '楷体',
            'font.fangsong': '仿宋',
            'font.noto-sans': 'Noto Sans CJK',
            'font.source-han-sans': '思源黑体',
            'font.source-han-serif': '思源宋体',
            'font.cascadia': 'Cascadia Code',
            'font.fira-code': 'Fira Code',
            'font.jetbrains-mono': 'JetBrains Mono',
            
            // 笔记页面
            'note.modified': '修改于',
            'note.backlinks': '链接到此页面',
            'note.tags': '标签',
            'note.graph': '关系图谱',
            'note.graph_depth': '显示深度',
            'note.toc': '目录',
            'note.breadcrumb': '面包屑',
            'note.copy_link': '复制链接',
            'note.copy_success': '链接已复制',
            
            // 图谱
            'graph.title': '笔记关系图谱',
            'graph.loading': '正在生成图谱...',
            'graph.depth_label': '关系深度：',
            'graph.depth_1': '1层',
            'graph.depth_2': '2层',
            'graph.depth_3': '3层',
            'graph.current_note': '当前笔记',
            'graph.related_notes': '相关笔记',
            
            // 标签页面
            'tags.title': '标签列表',
            'tags.all_tags': '所有标签',
            'tags.note_count': '{count} 篇笔记',
            'tags.filter_placeholder': '过滤标签...',
            'tags.empty': '暂无标签',
            
            // 状态栏
            'statusbar.sync': '未同步',
            'statusbar.synced': '已同步',
            'statusbar.syncing': '同步中...',
            'statusbar.word_count': '{count} 字',
            'statusbar.line_count': '{count} 行',
            'statusbar.click_stats': '点击查看详细统计',
            'statusbar.time': '时间',
            'statusbar.version': '版本',
            
            // 笔记统计弹窗
            'note_stats.title': '笔记统计',
            'note_stats.characters': '字符数',
            'note_stats.words': '字数',
            'note_stats.lines': '行数',
            'note_stats.paragraphs': '段落数',
            'note_stats.links': '链接数',
            'note_stats.images': '图片数',
            'note_stats.headings': '标题数',
            
            // 认证
            'auth.login': '登录',
            'auth.logout': '登出',
            'auth.username': '用户名',
            'auth.password': '密码',
            'auth.login_button': '登录',
            'auth.login_success': '登录成功',
            'auth.login_failed': '登录失败',
            'auth.logout_success': '已登出',
            'auth.change_password': '修改密码',
            'auth.old_password': '旧密码',
            'auth.new_password': '新密码',
            'auth.confirm_password': '确认密码',
            'auth.password_mismatch': '两次密码不一致',
            'auth.password_changed': '密码修改成功',
            'auth.password_change_failed': '密码修改失败',
            
            // 笔记预览
            'preview.loading': '加载中...',
            'preview.error': '预览加载失败',
            'preview.title': '笔记标题',
        },
        'en-US': {
            // General
            'app.name': 'Obsidian Mirror',
            'app.loading': 'Loading...',
            'app.error': 'Error',
            'app.success': 'Success',
            'app.cancel': 'Cancel',
            'app.confirm': 'Confirm',
            'app.close': 'Close',
            'app.apply': 'Apply',
            'app.reset': 'Reset',
            'app.save': 'Save',
            'app.delete': 'Delete',
            'app.edit': 'Edit',
            'app.search': 'Search',
            'app.filter': 'Filter',
            'app.sort': 'Sort',
            'app.back': 'Back',
            'app.next': 'Next',
            'app.previous': 'Previous',
            
            // Sidebar
            'sidebar.files': 'Files',
            'sidebar.tab_overview': 'Overview',
            'sidebar.tab_notes': 'Notes',
            'sidebar.sync': 'Sync',
            'sidebar.syncing': 'Syncing...',
            'sidebar.sync_success': 'Sync successful',
            'sidebar.sync_failed': 'Sync failed',
            'sidebar.tags': 'Tags',
            'sidebar.search': 'Search',
            'sidebar.settings': 'Settings',
            'sidebar.theme': 'Toggle theme',
            'sidebar.user_menu': 'User menu',
            'sidebar.logout': 'Logout',
            'sidebar.change_password': 'Change password',
            
            // Stats panel
            'stats.total_notes': 'Total notes',
            'stats.total_words': 'Total words',
            'stats.total_tags': 'Total tags',
            'stats.recent_notes': 'Recent',
            'stats.favorites': 'Favorites',
            'stats.word_count': '{count} words',
            'stats.note_count': '{count} notes',
            'stats.tag_count': '{count} tags',
            
            // Recent notes
            'recent.title': 'Recent',
            'recent.empty': 'No recent notes',
            'recent.clear': 'Clear history',
            'recent.clear_confirm': 'Are you sure you want to clear all recent notes?',
            
            // Favorites
            'favorites.title': 'Favorites',
            'favorites.empty': 'No favorites',
            'favorites.add': 'Add to favorites',
            'favorites.remove': 'Remove from favorites',
            'favorites.added': 'Added to favorites',
            'favorites.removed': 'Removed from favorites',
            
            // Search
            'search.title': 'Search notes',
            'search.placeholder': 'Type to search...',
            'search.empty': 'Type to start searching',
            'search.no_results': 'No notes found',
            'search.results': '{count} results found',
            'search.sort_relevance': 'Relevance',
            'search.sort_modified': 'Recently modified',
            'search.sort_label': 'Sort by:',
            'search.tips': 'Tip: Use',
            'search.shortcut_open': 'to open search',
            'search.shortcut_select': 'to select',
            'search.shortcut_enter': 'to open',
            'search.shortcut_close': 'to close',
            
            // Settings
            'settings.title': 'Settings',
            'settings.appearance': 'Appearance',
            'settings.language': 'Language',
            'settings.language_desc': 'Select interface language',
            'settings.font': 'Font',
            'settings.font_desc': 'Select reading font',
            'settings.font_size': 'Font size',
            'settings.font_size_desc': 'Adjust reading font size',
            'settings.line_height': 'Line height',
            'settings.line_height_desc': 'Adjust line spacing',
            'settings.reset_default': 'Reset to default',
            'settings.reset_confirm': 'Are you sure you want to reset to default settings?',
            'settings.apply_settings': 'Apply settings',
            'settings.settings_saved': 'Settings saved',
            
            // Font options
            'font.system': 'System default',
            'font.serif': 'Serif',
            'font.sans-serif': 'Sans-serif',
            'font.microsoft-yahei': 'Microsoft YaHei',
            'font.simsun': 'SimSun',
            'font.simhei': 'SimHei',
            'font.kaiti': 'KaiTi',
            'font.fangsong': 'FangSong',
            'font.noto-sans': 'Noto Sans CJK',
            'font.source-han-sans': 'Source Han Sans',
            'font.source-han-serif': 'Source Han Serif',
            'font.cascadia': 'Cascadia Code',
            'font.fira-code': 'Fira Code',
            'font.jetbrains-mono': 'JetBrains Mono',
            
            // Note page
            'note.modified': 'Modified',
            'note.backlinks': 'Linked to this page',
            'note.tags': 'Tags',
            'note.graph': 'Graph',
            'note.graph_depth': 'Depth',
            'note.toc': 'Table of contents',
            'note.breadcrumb': 'Breadcrumb',
            'note.copy_link': 'Copy link',
            'note.copy_success': 'Link copied',
            
            // Graph
            'graph.title': 'Note graph',
            'graph.loading': 'Generating graph...',
            'graph.depth_label': 'Depth:',
            'graph.depth_1': '1 level',
            'graph.depth_2': '2 levels',
            'graph.depth_3': '3 levels',
            'graph.current_note': 'Current note',
            'graph.related_notes': 'Related notes',
            
            // Tags page
            'tags.title': 'Tags',
            'tags.all_tags': 'All tags',
            'tags.note_count': '{count} notes',
            'tags.filter_placeholder': 'Filter tags...',
            'tags.empty': 'No tags',
            
            // Status bar
            'statusbar.sync': 'Not synced',
            'statusbar.synced': 'Synced',
            'statusbar.syncing': 'Syncing...',
            'statusbar.word_count': '{count} words',
            'statusbar.line_count': '{count} lines',
            'statusbar.click_stats': 'Click for detailed stats',
            'statusbar.time': 'Time',
            'statusbar.version': 'Version',
            
            // Note stats modal
            'note_stats.title': 'Note statistics',
            'note_stats.characters': 'Characters',
            'note_stats.words': 'Words',
            'note_stats.lines': 'Lines',
            'note_stats.paragraphs': 'Paragraphs',
            'note_stats.links': 'Links',
            'note_stats.images': 'Images',
            'note_stats.headings': 'Headings',
            
            // Authentication
            'auth.login': 'Login',
            'auth.logout': 'Logout',
            'auth.username': 'Username',
            'auth.password': 'Password',
            'auth.login_button': 'Login',
            'auth.login_success': 'Login successful',
            'auth.login_failed': 'Login failed',
            'auth.logout_success': 'Logged out',
            'auth.change_password': 'Change password',
            'auth.old_password': 'Old password',
            'auth.new_password': 'New password',
            'auth.confirm_password': 'Confirm password',
            'auth.password_mismatch': 'Passwords do not match',
            'auth.password_changed': 'Password changed successfully',
            'auth.password_change_failed': 'Failed to change password',
            
            // Note preview
            'preview.loading': 'Loading...',
            'preview.error': 'Failed to load preview',
            'preview.title': 'Note title',
        }
    };
    
    // 当前语言
    let currentLanguage = DEFAULT_LANG;
    
    /**
     * 初始化多语言系统
     */
    function init() {
        // 从 localStorage 加载语言设置
        const saved = localStorage.getItem(STORAGE_KEY);
        if (saved && LANGUAGES[saved]) {
            currentLanguage = saved;
        }
        
        // 设置 HTML lang 属性
        document.documentElement.lang = currentLanguage;
        
        // 自动翻译页面
        translatePage();
    }
    
    /**
     * 获取当前语言
     */
    function getCurrentLanguage() {
        return currentLanguage;
    }
    
    /**
     * 获取所有可用语言
     */
    function getAvailableLanguages() {
        return LANGUAGES;
    }
    
    /**
     * 设置语言
     */
    function setLanguage(lang) {
        if (!LANGUAGES[lang]) {
            console.warn('不支持的语言:', lang);
            return false;
        }
        
        currentLanguage = lang;
        localStorage.setItem(STORAGE_KEY, lang);
        document.documentElement.lang = lang;
        
        // 触发语言变更事件
        window.dispatchEvent(new CustomEvent('languageChanged', { detail: { language: lang } }));
        
        return true;
    }
    
    /**
     * 翻译文本
     * @param {string} key - 翻译键
     * @param {object} params - 参数对象（用于替换 {key} 占位符）
     */
    function t(key, params = {}) {
        const langDict = translations[currentLanguage] || translations[DEFAULT_LANG];
        let text = langDict[key] || key;
        
        // 替换参数
        Object.keys(params).forEach(paramKey => {
            text = text.replace(new RegExp(`\\{${paramKey}\\}`, 'g'), params[paramKey]);
        });
        
        return text;
    }
    
    /**
     * 翻译 HTML 元素
     * 使用 data-i18n 属性标记需要翻译的元素
     * 使用 data-i18n-attr 指定要翻译的属性（默认为 textContent）
     */
    function translatePage() {
        document.querySelectorAll('[data-i18n]').forEach(element => {
            const key = element.getAttribute('data-i18n');
            const attr = element.getAttribute('data-i18n-attr') || 'textContent';
            const params = element.getAttribute('data-i18n-params');
            
            let translatedText = t(key);
            
            // 如果有参数，解析并替换
            if (params) {
                try {
                    const paramsObj = JSON.parse(params);
                    translatedText = t(key, paramsObj);
                } catch (e) {
                    console.error('解析 i18n 参数失败:', e);
                }
            }
            
            if (attr === 'textContent') {
                element.textContent = translatedText;
            } else if (attr === 'innerHTML') {
                element.innerHTML = translatedText;
            } else if (attr === 'placeholder') {
                element.placeholder = translatedText;
            } else if (attr === 'title') {
                element.title = translatedText;
            } else {
                element.setAttribute(attr, translatedText);
            }
        });
    }
    
    // 导出公共接口
    window.i18n = {
        init,
        t,
        getCurrentLanguage,
        getAvailableLanguages,
        setLanguage,
        translatePage,
        LANGUAGES
    };
    
    // 自动初始化
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();
