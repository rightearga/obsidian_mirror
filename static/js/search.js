// ==========================================
// 搜索功能模块
// ==========================================

const SEARCH_HISTORY_KEY = 'obsidian_mirror_search_history';
const MAX_HISTORY_SIZE = 10; // 最多保存 10 条历史记录

// 搜索状态
let searchTimeout = null;
let currentSearchResults = [];
let selectedResultIndex = -1;
let isSearchModalOpen = false;
let searchModal, searchModalInput, searchModalResults, searchModalClear;
let currentSortBy = 'relevance'; // 当前排序方式：relevance 或 modified

// ===== 搜索历史管理 =====

/**
 * 获取搜索历史记录
 */
function getSearchHistory() {
    try {
        return JSON.parse(localStorage.getItem(SEARCH_HISTORY_KEY) || '[]');
    } catch {
        return [];
    }
}

/**
 * 保存搜索历史记录
 */
function saveSearchHistory(query) {
    if (!query || query.trim().length === 0) return;
    
    let history = getSearchHistory();
    
    // 移除重复项（如果已存在，提到最前面）
    history = history.filter(item => item !== query);
    
    // 添加到开头
    history.unshift(query);
    
    // 限制历史记录数量
    if (history.length > MAX_HISTORY_SIZE) {
        history = history.slice(0, MAX_HISTORY_SIZE);
    }
    
    try {
        localStorage.setItem(SEARCH_HISTORY_KEY, JSON.stringify(history));
    } catch (e) {
        console.warn('保存搜索历史失败:', e);
    }
}

/**
 * 清除搜索历史记录
 */
function clearSearchHistory() {
    try {
        localStorage.removeItem(SEARCH_HISTORY_KEY);
        console.log('搜索历史已清除');
    } catch (e) {
        console.warn('清除搜索历史失败:', e);
    }
}

/**
 * 显示搜索历史记录
 */
function displaySearchHistory() {
    const history = getSearchHistory();
    
    if (history.length === 0) {
        resetSearchResults();
        return;
    }
    
    let html = '<div class="search-history-section">';
    html += '<div class="search-history-header">';
    html += '<span class="search-history-title">最近搜索</span>';
    html += '<button class="search-history-clear" onclick="clearSearchHistoryUI()">清除</button>';
    html += '</div>';
    
    history.forEach((query, index) => {
        html += `
            <div class="search-history-item" data-index="${index}" onclick="selectHistoryItem('${escapeHtml(query)}')">
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="12" r="10"></circle>
                    <polyline points="12 6 12 12 16 14"></polyline>
                </svg>
                <span class="search-history-text">${escapeHtml(query)}</span>
            </div>
        `;
    });
    
    html += '</div>';
    searchModalResults.innerHTML = html;
}

/**
 * 清除搜索历史并更新 UI
 */
function clearSearchHistoryUI() {
    clearSearchHistory();
    displaySearchHistory();
}

/**
 * 选择历史记录项
 */
function selectHistoryItem(query) {
    if (searchModalInput) {
        searchModalInput.value = query;
        searchModalInput.focus();
        // 触发搜索
        performModalSearch(query);
    }
}

/**
 * 切换搜索弹窗的显示/隐藏
 */
function toggleSearchModal() {
    console.log('toggleSearchModal called, isSearchModalOpen:', isSearchModalOpen);
    console.log('searchModal:', searchModal);
    if (isSearchModalOpen) {
        closeSearchModal();
    } else {
        openSearchModal();
    }
}

/**
 * 打开搜索弹窗
 */
function openSearchModal() {
    console.log('openSearchModal called');
    if (!searchModal) {
        console.error('searchModal element not found!');
        return;
    }
    searchModal.style.display = 'flex';
    isSearchModalOpen = true;
    
    // 显示搜索历史
    displaySearchHistory();
    
    setTimeout(() => {
        if (searchModalInput) {
            searchModalInput.focus();
            searchModalInput.select();
        }
    }, 100);
    document.body.style.overflow = 'hidden'; // 防止背景滚动
}

/**
 * 关闭搜索弹窗
 */
function closeSearchModal() {
    console.log('closeSearchModal called');
    if (!searchModal) return;
    searchModal.style.display = 'none';
    isSearchModalOpen = false;
    if (searchModalInput) {
        searchModalInput.value = '';
    }
    if (searchModalClear) {
        searchModalClear.style.display = 'none';
    }
    resetSearchResults();
    document.body.style.overflow = ''; // 恢复滚动
}

/**
 * 清除搜索输入
 */
function clearSearchModal() {
    if (searchModalInput) {
        searchModalInput.value = '';
        searchModalInput.focus();
    }
    if (searchModalClear) {
        searchModalClear.style.display = 'none';
    }
    displaySearchHistory(); // 显示历史记录而不是空状态
}

/**
 * 重置搜索结果显示为初始状态
 */
function resetSearchResults() {
    if (!searchModalResults) return;
    searchModalResults.innerHTML = `
        <div class="search-modal-empty">
            <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" style="opacity: 0.3;">
                <circle cx="11" cy="11" r="8"></circle>
                <path d="m21 21-4.35-4.35"></path>
            </svg>
            <p>输入关键词开始搜索</p>
            <div class="search-modal-tips">
                <span>提示: 使用</span>
                <kbd>Ctrl</kbd> + <kbd>K</kbd>
                <span>快速打开搜索</span>
            </div>
        </div>
    `;
    currentSearchResults = [];
    selectedResultIndex = -1;
}

/**
 * 切换搜索排序方式
 */
function changeSearchSort(sortBy) {
    if (currentSortBy === sortBy) return; // 避免重复切换
    
    currentSortBy = sortBy;
    
    // 更新按钮状态
    document.querySelectorAll('.search-sort-btn').forEach(btn => {
        if (btn.dataset.sort === sortBy) {
            btn.classList.add('active');
        } else {
            btn.classList.remove('active');
        }
    });
    
    // 如果有搜索查询，重新执行搜索
    if (searchModalInput && searchModalInput.value.trim()) {
        performModalSearch(searchModalInput.value.trim());
    }
}

/**
 * 执行搜索请求
 */
async function performModalSearch(query) {
    if (!query && !hasActiveFilters()) {
        displaySearchHistory(); // 显示历史记录而不是空状态
        return;
    }

    // 保存搜索历史
    if (query) {
        saveSearchHistory(query);
    }

    // 显示加载状态
    searchModalResults.innerHTML = '<div class="search-modal-loading">搜索中...</div>';

    try {
        // 构建搜索 URL，包含过滤参数
        const searchParams = new URLSearchParams({
            q: query || '',
            sort_by: currentSortBy
        });

        // 添加过滤参数
        const filters = getActiveFilters();
        if (filters.tags) {
            searchParams.append('tags', filters.tags);
        }
        if (filters.folder) {
            searchParams.append('folder', filters.folder);
        }
        if (filters.date_from) {
            searchParams.append('date_from', filters.date_from);
        }
        if (filters.date_to) {
            searchParams.append('date_to', filters.date_to);
        }

        const response = await fetch(`/api/search?${searchParams.toString()}`);
        if (!response.ok) {
            const errorText = await response.text();
            console.error('搜索响应错误:', response.status, errorText);
            throw new Error(`搜索请求失败: ${response.status}`);
        }

        const results = await response.json();
        console.log('搜索结果:', results);
        displayModalSearchResults(results, query);
    } catch (error) {
        console.error('搜索错误:', error);
        searchModalResults.innerHTML = `<div class="search-modal-no-results">搜索出错: ${error.message}</div>`;
    }
}

/**
 * 显示搜索结果
 */
function displayModalSearchResults(results, query) {
    currentSearchResults = results;
    selectedResultIndex = -1;

    if (results.length === 0) {
        searchModalResults.innerHTML = `
            <div class="search-modal-no-results">
                <p>未找到匹配的笔记</p>
                <p style="font-size: 12px; margin-top: 8px; color: var(--text-faint);">尝试使用不同的关键词</p>
            </div>
        `;
        return;
    }

    const queryLower = query.toLowerCase();
    let html = '';

    results.forEach((result, index) => {
        const titleHtml = highlightText(result.title, queryLower);
        const snippetHtml = highlightText(result.snippet, queryLower);

        html += `
            <a href="/doc/${encodeURIComponent(result.path)}" 
               class="search-modal-result-item" 
               data-index="${index}"
               onmouseenter="highlightModalSearchResult(${index})"
               onclick="closeSearchModal()">
                <div class="search-modal-result-title">${titleHtml}</div>
                <div class="search-modal-result-snippet">${snippetHtml}</div>
            </a>
        `;
    });

    searchModalResults.innerHTML = html;

    // 为每个结果项设置错峰进入动画延迟（最多 8 项，之后延迟不再增加避免等待过长）
    const items = searchModalResults.querySelectorAll('.search-modal-result-item');
    items.forEach((item, i) => {
        item.style.animationDelay = Math.min(i, 7) * 20 + 'ms';
    });
}

/**
 * 高亮显示匹配的文本
 */
function highlightText(text, query) {
    if (!text) return '';
    if (!query) return escapeHtml(text);
    
    const escapedText = escapeHtml(text);
    const regex = new RegExp(`(${escapeRegex(query)})`, 'gi');
    
    return escapedText.replace(regex, '<span class="highlight">$1</span>');
}

/**
 * 更新选中的搜索结果
 */
function updateModalSelectedResult() {
    const items = searchModalResults.querySelectorAll('.search-modal-result-item');
    items.forEach((item, index) => {
        if (index === selectedResultIndex) {
            item.classList.add('active');
            item.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
        } else {
            item.classList.remove('active');
        }
    });
}

/**
 * 高亮搜索结果（鼠标悬停）
 */
function highlightModalSearchResult(index) {
    selectedResultIndex = index;
    updateModalSelectedResult();
}

/**
 * 导航到选中的搜索结果
 */
function navigateToModalResult(result) {
    if (result && result.path) {
        window.location.href = `/doc/${encodeURIComponent(result.path)}`;
    }
}

/**
 * 初始化搜索弹窗事件监听器
 */
function initSearchModal() {
    searchModal = document.getElementById('search-modal');
    searchModalInput = document.getElementById('search-modal-input');
    searchModalResults = document.getElementById('search-modal-results');
    searchModalClear = document.getElementById('search-modal-clear');

    console.log('搜索弹窗初始化:', {
        searchModal: !!searchModal,
        searchModalInput: !!searchModalInput,
        searchModalResults: !!searchModalResults,
        searchModalClear: !!searchModalClear
    });

    // 搜索输入事件
    if (searchModalInput) {
        searchModalInput.addEventListener('input', (e) => {
            const query = e.target.value.trim();
            
            if (query.length > 0) {
                searchModalClear.style.display = 'flex';
            } else {
                searchModalClear.style.display = 'none';
                displaySearchHistory(); // 显示历史记录
                return;
            }

            if (searchTimeout) {
                clearTimeout(searchTimeout);
            }

            searchTimeout = setTimeout(() => {
                performModalSearch(query);
            }, 300);
        });

        // 键盘导航
        searchModalInput.addEventListener('keydown', (e) => {
            const hasResults = currentSearchResults.length > 0;
            
            if (e.key === 'ArrowDown' && hasResults) {
                e.preventDefault();
                selectedResultIndex = Math.min(selectedResultIndex + 1, currentSearchResults.length - 1);
                updateModalSelectedResult();
            } else if (e.key === 'ArrowUp' && hasResults) {
                e.preventDefault();
                selectedResultIndex = Math.max(selectedResultIndex - 1, -1);
                updateModalSelectedResult();
            } else if (e.key === 'Enter' && hasResults) {
                e.preventDefault();
                if (selectedResultIndex >= 0 && selectedResultIndex < currentSearchResults.length) {
                    navigateToModalResult(currentSearchResults[selectedResultIndex]);
                }
            } else if (e.key === 'Escape') {
                e.preventDefault();
                closeSearchModal();
            }
        });
    }

    // 实现对话框拖拽调整大小
    initSearchDialogResize();
}

/**
 * 初始化搜索对话框的拖拽调整大小功能
 */
function initSearchDialogResize() {
    const dialog = document.querySelector('.search-modal-dialog');
    const handle = document.querySelector('.search-modal-resize-handle');
    
    if (!dialog || !handle) return;
    
    let isResizing = false;
    let startX, startY, startWidth, startHeight;
    
    handle.addEventListener('mousedown', (e) => {
        isResizing = true;
        startX = e.clientX;
        startY = e.clientY;
        startWidth = dialog.offsetWidth;
        startHeight = dialog.offsetHeight;
        
        e.preventDefault();
        document.body.style.cursor = 'se-resize';
    });
    
    document.addEventListener('mousemove', (e) => {
        if (!isResizing) return;
        
        const deltaX = e.clientX - startX;
        const deltaY = e.clientY - startY;
        
        const newWidth = Math.max(400, Math.min(1200, startWidth + deltaX));
        const newHeight = Math.max(300, Math.min(window.innerHeight * 0.8, startHeight + deltaY));
        
        dialog.style.width = newWidth + 'px';
        dialog.style.maxWidth = newWidth + 'px';
        dialog.style.height = newHeight + 'px';
        dialog.style.maxHeight = newHeight + 'px';
    });
    
    document.addEventListener('mouseup', () => {
        if (isResizing) {
            isResizing = false;
            document.body.style.cursor = '';
        }
    });
}

// ===== 全局快捷键 =====
document.addEventListener('keydown', (e) => {
    // Ctrl+K (Windows/Linux) 或 Cmd+K (Mac)
    if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
        e.preventDefault();
        toggleSearchModal();
    }
});

// ==========================================
// 高级过滤功能
// ==========================================

/**
 * 切换过滤面板显示/隐藏
 */
function toggleSearchFilters() {
    const panel = document.getElementById('search-filters-panel');
    const toggle = document.querySelector('.search-filters-toggle');
    
    if (panel.style.display === 'none') {
        panel.style.display = 'block';
        toggle.classList.add('expanded');
    } else {
        panel.style.display = 'none';
        toggle.classList.remove('expanded');
    }
}

/**
 * 应用搜索过滤（输入改变时触发）
 */
function applySearchFilters() {
    const searchInput = document.getElementById('search-modal-input');
    performModalSearch(searchInput.value.trim());
}

/**
 * 获取当前激活的过滤条件
 */
function getActiveFilters() {
    const filters = {};
    
    // 标签过滤
    const tagsInput = document.getElementById('search-filter-tags');
    if (tagsInput && tagsInput.value.trim()) {
        filters.tags = tagsInput.value.trim();
    }
    
    // 文件夹过滤
    const folderInput = document.getElementById('search-filter-folder');
    if (folderInput && folderInput.value.trim()) {
        filters.folder = folderInput.value.trim();
    }
    
    // 日期范围过滤
    const dateFromInput = document.getElementById('search-filter-date-from');
    const dateToInput = document.getElementById('search-filter-date-to');
    
    if (dateFromInput && dateFromInput.value) {
        // 转换日期为 Unix 时间戳（秒）
        const dateFrom = new Date(dateFromInput.value);
        filters.date_from = Math.floor(dateFrom.getTime() / 1000);
    }
    
    if (dateToInput && dateToInput.value) {
        // 转换日期为 Unix 时间戳（秒），并设置为当天的结束时间
        const dateTo = new Date(dateToInput.value);
        dateTo.setHours(23, 59, 59, 999);
        filters.date_to = Math.floor(dateTo.getTime() / 1000);
    }
    
    return filters;
}

/**
 * 检查是否有激活的过滤条件
 */
function hasActiveFilters() {
    const filters = getActiveFilters();
    return Object.keys(filters).length > 0;
}

/**
 * 清除所有过滤条件
 */
function clearSearchFilters() {
    const tagsInput = document.getElementById('search-filter-tags');
    const folderInput = document.getElementById('search-filter-folder');
    const dateFromInput = document.getElementById('search-filter-date-from');
    const dateToInput = document.getElementById('search-filter-date-to');
    
    if (tagsInput) tagsInput.value = '';
    if (folderInput) folderInput.value = '';
    if (dateFromInput) dateFromInput.value = '';
    if (dateToInput) dateToInput.value = '';
    
    // 重新执行搜索
    const searchInput = document.getElementById('search-modal-input');
    performModalSearch(searchInput.value.trim());
}
