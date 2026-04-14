//! obsidian_mirror WebAssembly 模块（v1.6.1）
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

// ─── v1.6.1：Markdown 渲染相关导入 ──────────────────────────────────────────

use lazy_static::lazy_static;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use pulldown_cmark::{html, Options, Parser};
use regex::Regex;

lazy_static! {
    /// 匹配图片/文件类 WikiLink：![[文件路径]] 或 ![[文件路径|显示文本]]
    static ref IMAGE_WIKI_RE: Regex =
        Regex::new(r"!\[\[(.*?)(?:\|(.*?))?\]\]").unwrap();

    /// 匹配普通笔记 WikiLink：[[笔记]] 或 [[笔记|别名]]
    static ref WIKI_RE: Regex =
        Regex::new(r"\[\[(.*?)(?:\|(.*?))?\]\]").unwrap();

    /// 匹配块级数学公式：$$ ... $$（可跨行，非贪婪）
    static ref MATH_BLOCK_RE: Regex =
        Regex::new(r"\$\$([\s\S]*?)\$\$").unwrap();

    /// 匹配行内数学公式：$...$（不跨行，首尾非空格非$）
    static ref MATH_INLINE_RE: Regex =
        Regex::new(r"\$(?P<content>[^\s\$\n\r][^\$\n\r]*[^\s\$\n\r]|[^\s\$\n\r])\$").unwrap();

    /// 匹配高亮语法：==文本==（不跨行）
    static ref HIGHLIGHT_RE: Regex =
        Regex::new(r"==([^=\n\r]+)==").unwrap();
}

/// 图片扩展名列表（用于判断 ![[...]] 是否为图片）
const IMAGE_EXTS: &[&str] = &[".png", ".jpg", ".jpeg", ".gif", ".svg", ".webp"];

/// 判断目标路径是否为图片文件
fn is_image_ext(target: &str) -> bool {
    let lower = target.to_lowercase();
    IMAGE_EXTS.iter().any(|ext| lower.ends_with(ext))
}

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

// ─── v1.6.1：Markdown 渲染（WASM 版本）──────────────────────────────────────

/// 将 Markdown 渲染为 HTML，处理完整的 Obsidian 扩展语法（v1.6.1）。
///
/// 处理顺序与服务端 `MarkdownProcessor::process` 保持一致：
/// 1. 预处理 `![[...]]` 图片/笔记内嵌（图片 → `<img>`，其他 → 链接）
/// 2. 预处理 `[[...]]` WikiLink（→ `/doc/...` HTML 链接）
/// 3. 预处理块级数学公式 `$$...$$`（→ `<div class="math-block">` 占位）
/// 4. 预处理行内数学公式 `$...$`（→ `<span class="math-inline">` 占位）
/// 5. 预处理高亮语法 `==text==`（→ `<mark>text</mark>`）
/// 6. pulldown-cmark 渲染（开启 Tables/Strikethrough/Tasklists/Footnotes）
///
/// **注意**：Callout 块由客户端 `callout.js` 处理，此函数无需单独处理。
/// **注意**：不处理 YAML Frontmatter（实时预览场景通常不需要）。
#[wasm_bindgen]
pub fn render_markdown(content: &str) -> String {
    // 步骤 1：预处理图片/文件 WikiLink（![[...]]）
    let s = IMAGE_WIKI_RE.replace_all(content, |caps: &regex::Captures| {
        let target = caps.get(1).map_or("", |m| m.as_str()).trim();
        let alt    = caps.get(2).map_or("", |m| m.as_str()).trim();
        let encoded = utf8_percent_encode(target, NON_ALPHANUMERIC).to_string();

        if is_image_ext(target) {
            // 图片：生成 img 标签
            format!("\n\n![{}](/assets/{})\n\n", alt, encoded)
        } else if target.to_lowercase().ends_with(".md") || !target.contains('.') {
            // 笔记内嵌：渲染为链接（WASM 环境无法递归加载，用链接代替展开）
            let link_text = if alt.is_empty() { target } else { alt };
            format!("[{}](/doc/{})", html_escape(link_text), encoded)
        } else {
            // 其他文件：链接到 /assets/
            let link_text = if alt.is_empty() { target } else { alt };
            format!("[{}](/assets/{})", link_text, encoded)
        }
    });

    // 步骤 2：预处理普通 WikiLink（[[...]]）
    let s = WIKI_RE.replace_all(&s, |caps: &regex::Captures| {
        let target = caps.get(1).map_or("", |m| m.as_str()).trim();
        let label  = caps.get(2).map_or(target, |m| m.as_str()).trim();
        let encoded = utf8_percent_encode(target, NON_ALPHANUMERIC).to_string();
        format!("[{}](/doc/{})", html_escape(label), encoded)
    });

    // 步骤 3：预处理块级数学公式 $$...$$
    let s = MATH_BLOCK_RE.replace_all(&s, |caps: &regex::Captures| {
        let formula = caps.get(1).map_or("", |m| m.as_str());
        let encoded = html_escape(formula);
        format!(
            r#"<div class="math-block" data-math="{}">{}</div>"#,
            html_escape_attr(formula), encoded
        )
    });

    // 步骤 4：预处理行内数学公式 $...$
    let s = MATH_INLINE_RE.replace_all(&s, |caps: &regex::Captures| {
        let formula = caps.name("content").map_or("", |m| m.as_str());
        format!(
            r#"<span class="math-inline" data-math="{}">{}</span>"#,
            html_escape_attr(formula), html_escape(formula)
        )
    });

    // 步骤 5：预处理高亮语法 ==text==
    let s = HIGHLIGHT_RE.replace_all(&s, |caps: &regex::Captures| {
        let text = caps.get(1).map_or("", |m| m.as_str());
        format!("<mark>{}</mark>", html_escape(text))
    });

    // 步骤 6：pulldown-cmark 渲染
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(&s, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    html_output
}

/// HTML 内容转义（防止 XSS）
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// HTML 属性值转义（用于 data-math 属性）
fn html_escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\n', "&#10;")
}

// ─── 单元测试（运行于原生目标，无需 wasm-pack）──────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── render_markdown 测试 ───────────────────────────────────────────────

    #[test]
    fn test_render_markdown_basic() {
        let html = render_markdown("# 标题\n\n段落内容");
        assert!(html.contains("<h1>"), "应生成 h1 标签");
        assert!(html.contains("段落内容"), "应包含段落内容");
    }

    #[test]
    fn test_render_markdown_wikilink() {
        let html = render_markdown("参见 [[笔记A]]");
        assert!(html.contains("href=\"/doc/"), "WikiLink 应转换为 /doc/ 路径");
        assert!(html.contains("笔记A"), "应保留链接文本");
    }

    #[test]
    fn test_render_markdown_wikilink_alias() {
        let html = render_markdown("参见 [[笔记A|别名]]");
        assert!(html.contains("别名"), "应使用别名作为链接文本");
    }

    #[test]
    fn test_render_markdown_image_wikilink() {
        let html = render_markdown("![[图片.png]]");
        assert!(html.contains("/assets/"), "图片 WikiLink 应指向 /assets/");
        assert!(html.contains("<img") || html.contains("!["), "应生成图片元素");
    }

    #[test]
    fn test_render_markdown_highlight() {
        let html = render_markdown("这是 ==高亮文字== 内容");
        assert!(html.contains("<mark>高亮文字</mark>"), "应渲染高亮语法");
    }

    #[test]
    fn test_render_markdown_math_inline() {
        let html = render_markdown("行内公式 $E=mc^2$ 内容");
        assert!(html.contains("math-inline"), "应包含行内数学占位符");
    }

    #[test]
    fn test_render_markdown_math_block() {
        let html = render_markdown("$$\na + b = c\n$$");
        assert!(html.contains("math-block"), "应包含块级数学占位符");
    }

    #[test]
    fn test_render_markdown_table() {
        let html = render_markdown("| A | B |\n|---|---|\n| 1 | 2 |");
        assert!(html.contains("<table>"), "应渲染表格");
    }

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
