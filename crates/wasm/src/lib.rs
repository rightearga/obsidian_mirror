//! obsidian_mirror WebAssembly 模块（v1.6.0）
//!
//! 提供可在浏览器端运行的纯函数实现，与服务端共享同一份逻辑。
//!
//! **编译为 WASM**（需安装 wasm-pack）：
//! ```sh
//! wasm-pack build crates/wasm --target web --out-dir ../../static/wasm
//! ```
//!
//! **在浏览器中使用**：
//! ```html
//! <script type="module" src="/static/wasm/loader.js"></script>
//! <script>
//!   WasmLoader.init().then(() => {
//!     console.log(WasmLoader.highlightTerm("Hello World", "world"));
//!   });
//! </script>
//! ```

use wasm_bindgen::prelude::*;

// ─── 基础设施函数 ────────────────────────────────────────────────────────────

/// 返回当前 WASM 模块版本（与服务端 `obsidian_mirror` 版本一致）
///
/// 用于确认浏览器加载的 WASM 模块版本与服务端匹配。
#[wasm_bindgen]
pub fn wasm_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ─── 文本处理函数（共享逻辑，与服务端保持一致）────────────────────────────

/// 在文本中将所有匹配 `term` 的位置包裹为 `<mark>...</mark>` 高亮标签。
///
/// 大小写不敏感匹配，保留原文大小写。
/// 与服务端 `search_engine::highlight_terms` 逻辑一致，可替换其客户端等价实现。
#[wasm_bindgen]
pub fn highlight_term(text: &str, term: &str) -> String {
    if term.is_empty() {
        return text.to_string();
    }
    let term_lower = term.to_lowercase();
    let text_lower = text.to_lowercase();

    let mut result = String::with_capacity(text.len() + 24);
    let mut last_end = 0;
    let mut search_start = 0;

    while let Some(rel_pos) = text_lower[search_start..].find(&term_lower) {
        let abs_pos = search_start + rel_pos;
        // 确保在 UTF-8 字符边界上
        if !text.is_char_boundary(abs_pos) {
            search_start = abs_pos + 1;
            continue;
        }
        let term_end = abs_pos + term.len();
        if term_end > text.len() || !text.is_char_boundary(term_end) {
            search_start = abs_pos + 1;
            continue;
        }
        result.push_str(&text[last_end..abs_pos]);
        result.push_str("<mark>");
        result.push_str(&text[abs_pos..term_end]);
        result.push_str("</mark>");
        last_end = term_end;
        search_start = term_end;
    }
    result.push_str(&text[last_end..]);
    result
}

/// 从 HTML 中提取纯文本并截取到指定字符数（去除所有 HTML 标签）。
///
/// 与服务端 `handlers::truncate_html` 逻辑一致，可用于客户端预览生成，
/// 减少对 `/api/preview` 接口的依赖。
///
/// # 参数
/// - `html`：输入 HTML 字符串
/// - `max_chars`：最大可见字符数（基于 Unicode 字符，不是字节）
#[wasm_bindgen]
pub fn truncate_html(html: &str, max_chars: usize) -> String {
    // 状态机去除 HTML 标签，提取可见文本
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                text.push(' '); // 标签位置插入空格避免文字粘连
            }
            _ if !in_tag => text.push(c),
            _ => {}
        }
    }
    // 合并多余空白
    let text: String = text.split_whitespace().collect::<Vec<_>>().join(" ");

    if text.chars().count() <= max_chars {
        return text;
    }
    format!("{}...", text.chars().take(max_chars).collect::<String>())
}

// ─── 单元测试（运行于原生目标，无需 wasm-pack）──────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_version_nonempty() {
        // 版本字符串应非空
        assert!(!wasm_version().is_empty(), "wasm_version() 不应返回空字符串");
    }

    #[test]
    fn test_highlight_term_ascii() {
        // ASCII 字符匹配，大小写不敏感
        let result = highlight_term("Hello World", "world");
        assert_eq!(result, "Hello <mark>World</mark>", "应高亮 World");
    }

    #[test]
    fn test_highlight_term_empty_term() {
        // 空 term 原样返回
        let result = highlight_term("Hello", "");
        assert_eq!(result, "Hello", "空 term 应原样返回");
    }

    #[test]
    fn test_highlight_term_no_match() {
        // 无匹配时原样返回
        let result = highlight_term("Hello", "xyz");
        assert_eq!(result, "Hello", "无匹配时应原样返回");
    }

    #[test]
    fn test_highlight_term_multiple() {
        // 多处匹配均高亮
        let result = highlight_term("abc abc abc", "abc");
        assert_eq!(
            result,
            "<mark>abc</mark> <mark>abc</mark> <mark>abc</mark>",
            "所有匹配位置均应高亮"
        );
    }

    #[test]
    fn test_truncate_html_no_tags() {
        // 无标签时直接截断
        let result = truncate_html("Hello World", 5);
        assert_eq!(result, "Hello...", "应截断到 5 字符后加 ...");
    }

    #[test]
    fn test_truncate_html_with_tags() {
        // 剥离标签后截断
        let result = truncate_html("<p>Hello <b>World</b></p>", 5);
        assert_eq!(result, "Hello...", "应去除标签后截断");
    }

    #[test]
    fn test_truncate_html_short() {
        // 内容不超限时不截断
        let result = truncate_html("<p>Hi</p>", 100);
        assert_eq!(result, "Hi", "不超限时不应截断");
    }
}
