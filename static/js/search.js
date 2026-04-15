// ==========================================
// 搜索功能模块
// ==========================================

const SEARCH_HISTORY_KEY = 'obsidian_mirror_search_history';
const MAX_HISTORY_SIZE = 20; // 最多保存 20 条历史记录
const TITLES_CACHE_KEY = 'obsidian_mirror_titles_cache';
const TITLES_CACHE_TTL = 5 * 60 * 1000; // 标题缓存 5 分钟（毫秒）

// 搜索状态
let searchTimeout = null;
let currentSearchResults = [];
let selectedResultIndex = -1;
let isSearchModalOpen = false;
let searchModal, searchModalInput, searchModalResults, searchModalClear;
let currentSortBy = 'relevance'; // 当前排序方式：relevance 或 modified

// v1.8.0 分页状态
let searchCurrentPage   = 1;
let searchTotalPages    = 1;
let searchTotalResults  = 0;
let searchLastQuery     = '';   // 上次搜索的 query 字符串
let searchLastFilters   = null; // 上次搜索的 filters 对象
let searchLoadingMore   = false;

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
            <div class="search-history-item" data-index="${index}">
                <div class="search-history-main" onclick="selectHistoryItem('${escapeHtml(query).replace(/'/g, "\\'")}')">
                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="10"></circle>
                        <polyline points="12 6 12 12 16 14"></polyline>
                    </svg>
                    <span class="search-history-text">${escapeHtml(query)}</span>
                </div>
                <button class="search-history-delete" title="删除此记录"
                        onclick="deleteHistoryItem(${index})" aria-label="删除">×</button>
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
 * 删除单条搜索历史记录
 */
function deleteHistoryItem(index) {
    try {
        let history = getSearchHistory();
        history.splice(index, 1);
        localStorage.setItem(SEARCH_HISTORY_KEY, JSON.stringify(history));
    } catch (e) {
        console.warn('删除历史记录失败:', e);
    }
    displaySearchHistory();
}

// ===== 搜索自动补全 =====

/**
 * 获取缓存的标题和标签数据（带 TTL）
 */
async function getCachedTitles() {
    try {
        const cached = sessionStorage.getItem(TITLES_CACHE_KEY);
        if (cached) {
            const { data, time } = JSON.parse(cached);
            if (Date.now() - time < TITLES_CACHE_TTL) {
                return data;
            }
        }
    } catch (e) { /* 忽略缓存错误 */ }

    // 从服务器获取
    try {
        const res = await fetch('/api/titles');
        if (res.ok) {
            const data = await res.json();
            sessionStorage.setItem(TITLES_CACHE_KEY, JSON.stringify({ data, time: Date.now() }));
            return data;
        }
    } catch (e) {
        console.warn('获取标题列表失败:', e);
    }
    return { titles: [], tags: [] };
}

/**
 * 显示搜索自动补全建议
 */
async function showSearchSuggestions(query) {
    if (!query || query.length < 1) {
        displaySearchHistory();
        return;
    }

    const { titles, tags } = await getCachedTitles();
    const lowerQ = query.toLowerCase();

    // 标题匹配（前缀优先，再包含）
    const matchedTitles = titles
        .filter(t => t.toLowerCase().includes(lowerQ))
        .sort((a, b) => {
            const aStart = a.toLowerCase().startsWith(lowerQ) ? 0 : 1;
            const bStart = b.toLowerCase().startsWith(lowerQ) ? 0 : 1;
            return aStart - bStart || a.localeCompare(b);
        })
        .slice(0, 5);

    // 标签匹配（带 # 前缀时触发）
    const matchedTags = query.startsWith('#')
        ? tags.filter(t => t.toLowerCase().includes(lowerQ.slice(1))).slice(0, 4)
        : tags.filter(t => t.toLowerCase().includes(lowerQ)).slice(0, 3);

    if (matchedTitles.length === 0 && matchedTags.length === 0) return;

    let html = '<div class="search-suggestions">';
    if (matchedTitles.length > 0) {
        html += '<div class="suggestion-section-title">笔记</div>';
        matchedTitles.forEach(title => {
            const safeTitle = escapeHtml(title);
            html += `<div class="search-suggestion-item" onclick="selectSuggestion('${safeTitle.replace(/'/g, "\\'")}')">
                <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/>
                    <polyline points="14 2 14 8 20 8"/>
                </svg>
                <span class="suggestion-text">${safeTitle}</span>
            </div>`;
        });
    }
    if (matchedTags.length > 0) {
        html += '<div class="suggestion-section-title">标签</div>';
        matchedTags.forEach(tag => {
            const safeTag = escapeHtml(tag);
            html += `<div class="search-suggestion-item" onclick="selectSuggestion('#${safeTag.replace(/'/g, "\\'")}')">
                <svg xmlns="http://www.w3.org/2000/svg" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M20.59 13.41l-7.17 7.17a2 2 0 0 1-2.83 0L2 12V2h10l8.59 8.59a2 2 0 0 1 0 2.82z"/>
                    <line x1="7" y1="7" x2="7.01" y2="7"/>
                </svg>
                <span class="suggestion-text">#${safeTag}</span>
            </div>`;
        });
    }
    html += '</div>';

    if (searchModalResults) {
        searchModalResults.innerHTML = html;
    }
}

/**
 * 选择自动补全建议
 */
function selectSuggestion(value) {
    if (searchModalInput) {
        searchModalInput.value = value;
        searchModalInput.focus();
        performModalSearch(value);
    }
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
        displaySearchHistory();
        return;
    }
    if (query) saveSearchHistory(query);

    // 重置分页状态（全新搜索）
    searchCurrentPage  = 1;
    searchLastQuery    = query || '';
    searchLastFilters  = getActiveFilters();
    searchLoadingMore  = false;

    searchModalResults.innerHTML = '<div class="search-modal-loading">搜索中...</div>';
    await _fetchAndRenderSearch(1, false);
}

/**
 * 执行搜索并渲染结果（内部方法，支持分页）
 * @param {number} page      - 要加载的页码（1-based）
 * @param {boolean} append   - true=追加到现有列表，false=替换
 */
async function _fetchAndRenderSearch(page, append) {
    try {
        const searchParams = new URLSearchParams({
            q:        searchLastQuery,
            sort_by:  currentSortBy,
            page:     page,
            per_page: 20,
        });
        if (searchLastFilters) {
            if (searchLastFilters.tags)      searchParams.append('tags',      searchLastFilters.tags);
            if (searchLastFilters.folder)    searchParams.append('folder',    searchLastFilters.folder);
            if (searchLastFilters.date_from) searchParams.append('date_from', searchLastFilters.date_from);
            if (searchLastFilters.date_to)   searchParams.append('date_to',   searchLastFilters.date_to);
        }

        const response = await fetch(`/api/search?${searchParams.toString()}`);
        if (!response.ok) throw new Error(`HTTP ${response.status}`);

        // v1.8.3：Service Worker 离线搜索时设置 X-Offline-Search 响应头
        const isOfflineSearch = response.headers.get('X-Offline-Search');
        const offlineHint = document.getElementById('offline-search-hint');
        if (offlineHint && isOfflineSearch) {
            offlineHint.style.display = '';  // 离线时显示提示
        }

        const data = await response.json();
        // v1.8.0：响应格式为 {results, total, page, per_page, total_pages}
        const results    = data.results    ?? data;    // 向后兼容旧格式（纯数组）
        const total      = data.total      ?? results.length;
        const totalPages = data.total_pages ?? 1;

        searchCurrentPage  = page;
        searchTotalPages   = totalPages;
        searchTotalResults = total;

        if (append) {
            _appendSearchResults(results, searchLastQuery);
        } else {
            displayModalSearchResults(results, searchLastQuery, total, totalPages);
        }
        searchLoadingMore = false;
    } catch (error) {
        console.error('搜索错误:', error);
        if (!append) {
            searchModalResults.innerHTML = `<div class="search-modal-no-results">搜索出错: ${error.message}</div>`;
        }
        searchLoadingMore = false;
    }
}

/**
 * 构建单条搜索结果的 HTML 字符串
 */
function _buildResultHtml(result, index, queryLower) {
    const titleHtml   = highlightText(result.title, queryLower);
    const snippetHtml = highlightText(result.snippet, queryLower);
    const pathParts   = (result.path || '').split('/');
    pathParts.pop();
    const folderPath  = pathParts.join(' / ');
    const tags        = (result.tags || []).slice(0, 3);
    const tagsHtml    = tags.length > 0
        ? `<span class="result-tags">${tags.map(t => `<span class="result-tag">#${escapeHtml(t)}</span>`).join('')}</span>`
        : '';
    const mtimeHtml   = result.mtime ? `<span class="result-mtime">${formatRelativeTime(result.mtime)}</span>` : '';
    return `<a href="/doc/${encodeURIComponent(result.path)}"
               class="search-modal-result-item"
               data-index="${index}"
               onmouseenter="highlightModalSearchResult(${index})"
               onclick="closeSearchModal()">
                <div class="search-modal-result-title">${titleHtml}</div>
                ${folderPath ? `<div class="result-path">${escapeHtml(folderPath)}</div>` : ''}
                <div class="search-modal-result-snippet">${snippetHtml}</div>
                <div class="result-meta">${tagsHtml}${mtimeHtml}</div>
            </a>`;
}

/**
 * 显示搜索结果（替换模式）
 * @param {Array}  results     - 当前页结果列表
 * @param {string} query       - 搜索词
 * @param {number} total       - 命中总数
 * @param {number} totalPages  - 总页数
 */
function displayModalSearchResults(results, query, total, totalPages) {
    currentSearchResults = results;
    selectedResultIndex  = -1;

    if (results.length === 0) {
        searchModalResults.innerHTML = `
            <div class="search-modal-no-results">
                <p>未找到匹配的笔记</p>
                <p style="font-size: 12px; margin-top: 8px; color: var(--text-faint);">尝试使用不同的关键词</p>
            </div>`;
        return;
    }

    const queryLower = (query || '').toLowerCase();
    let html = '';

    // 结果总数提示（v1.8.0）
    if (total > 0) {
        html += `<div class="search-result-summary">共 ${total} 条结果${total > results.length ? `，当前显示前 ${results.length} 条` : ''}</div>`;
    }

    results.forEach((result, index) => {
        html += _buildResultHtml(result, index, queryLower);
    });

    // "加载更多"按钮（v1.8.0 分页）
    if (totalPages > 1) {
        html += _buildLoadMoreBtn(totalPages);
    }

    searchModalResults.innerHTML = html;

    // 错峰动画
    searchModalResults.querySelectorAll('.search-modal-result-item').forEach((item, i) => {
        item.style.animationDelay = Math.min(i, 7) * 20 + 'ms';
    });
}

/**
 * 追加更多搜索结果到现有列表（"加载更多"）
 */
function _appendSearchResults(results, query) {
    if (!results || results.length === 0) return;
    const queryLower   = (query || '').toLowerCase();
    const startIndex   = currentSearchResults.length;
    currentSearchResults = currentSearchResults.concat(results);

    // 移除旧的"加载更多"按钮和总数提示（如有）
    const oldBtn     = searchModalResults.querySelector('.search-load-more-btn');
    const oldSummary = searchModalResults.querySelector('.search-result-summary');
    if (oldBtn)     oldBtn.remove();

    let html = '';
    results.forEach((result, i) => {
        html += _buildResultHtml(result, startIndex + i, queryLower);
    });

    // 如果还有更多页，重新添加按钮
    if (searchCurrentPage < searchTotalPages) {
        html += _buildLoadMoreBtn(searchTotalPages);
    }

    searchModalResults.insertAdjacentHTML('beforeend', html);

    // 更新总数
    if (oldSummary) {
        oldSummary.textContent = `共 ${searchTotalResults} 条结果，已显示 ${currentSearchResults.length} 条`;
    }
}

/**
 * 构建"加载更多"按钮的 HTML
 */
function _buildLoadMoreBtn(totalPages) {
    return `<button class="search-load-more-btn"
        onclick="loadMoreSearchResults()"
        style="display:block;width:100%;padding:10px;margin-top:8px;
               background:var(--bg-secondary,#1e1e2e);border:1px solid var(--border-color,#313244);
               border-radius:6px;color:var(--text-muted,#6c7086);cursor:pointer;font-size:0.85rem;">
        加载更多（第 ${searchCurrentPage}/${totalPages} 页）
    </button>`;
}

/**
 * 加载下一页搜索结果（"加载更多"按钮触发）
 */
async function loadMoreSearchResults() {
    if (searchLoadingMore || searchCurrentPage >= searchTotalPages) return;
    searchLoadingMore = true;

    const btn = searchModalResults.querySelector('.search-load-more-btn');
    if (btn) btn.textContent = '加载中...';

    await _fetchAndRenderSearch(searchCurrentPage + 1, true);
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
 * 将 Unix 时间戳（秒）转换为相对时间字符串
 * 如：刚刚 / 5分钟前 / 3小时前 / 2天前 / 2026-04-01
 */
function formatRelativeTime(timestampSec) {
    if (!timestampSec) return '';
    const now = Math.floor(Date.now() / 1000);
    const diff = now - timestampSec;

    if (diff < 60)  return '刚刚';
    if (diff < 3600) return `${Math.floor(diff / 60)}分钟前`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}小时前`;
    if (diff < 86400 * 30) return `${Math.floor(diff / 86400)}天前`;

    const d = new Date(timestampSec * 1000);
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
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

            // 短延迟时先显示自动补全建议，300ms 后再发起正式搜索
            showSearchSuggestions(query);

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
