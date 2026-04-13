// ==========================================
// 工具函数模块
// ==========================================

/**
 * URL 百分号解码
 */
function percentDecode(str) {
    try {
        return decodeURIComponent(str);
    } catch (e) {
        return str;
    }
}

/**
 * HTML 转义（防止 XSS）
 */
function escapeHtml(text) {
    if (!text) return '';
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

/**
 * 正则表达式转义
 */
function escapeRegex(str) {
    return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * 防抖函数 - 延迟执行函数调用
 * @param {Function} func - 要防抖的函数
 * @param {number} wait - 延迟时间（毫秒）
 * @returns {Function} 防抖后的函数
 */
function debounce(func, wait) {
    let timeout;
    return function executedFunction(...args) {
        const later = () => {
            clearTimeout(timeout);
            func(...args);
        };
        clearTimeout(timeout);
        timeout = setTimeout(later, wait);
    };
}
