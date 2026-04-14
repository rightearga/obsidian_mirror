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

// ─── v1.6.2：离线搜索相关导入 ───────────────────────────────────────────────

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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

// ─── v1.6.2：离线全文搜索（WASM NoteIndex）──────────────────────────────────

/// 离线搜索索引条目（与服务端 SearchIndexDump 格式一致）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteEntry {
    /// 笔记标题
    pub title: String,
    /// 相对路径（如 "folder/note.md"）
    pub path: String,
    /// 标签列表
    #[serde(default)]
    pub tags: Vec<String>,
    /// 笔记内容摘要（前 300 字符，用于搜索匹配和摘要展示）
    #[serde(default)]
    pub content: String,
    /// 修改时间（Unix 时间戳秒，用于排序提示）
    #[serde(default)]
    pub mtime: i64,
}

/// 搜索结果（格式与服务端 `/api/search` 响应一致，前端无感知切换）
#[derive(Debug, Serialize)]
struct SearchResult {
    title: String,
    path: String,
    snippet: String,
    score: f32,
    mtime: i64,
    tags: Vec<String>,
}

/// 轻量笔记全文索引（WASM 版本，v1.6.2）
///
/// 使用 n-gram 分词 + TF 评分实现客户端离线搜索。
/// 通过 `load_json` 从服务端生成的 `index.json` 初始化，
/// 通过 `search_json` 返回与在线 API 格式一致的 JSON 搜索结果。
#[wasm_bindgen]
pub struct NoteIndex {
    /// 所有笔记条目
    notes: Vec<NoteEntry>,
    /// 倒排索引：token → [note_index...]（加速搜索）
    inverted: HashMap<String, Vec<usize>>,
}

#[wasm_bindgen]
impl NoteIndex {
    /// 从服务端 index.json 的 JSON 字符串加载索引。
    ///
    /// index.json 格式：`[{title, path, tags, content, mtime}, ...]`
    #[wasm_bindgen(js_name = loadJson)]
    pub fn load_json(json: &str) -> Result<NoteIndex, String> {
        let notes: Vec<NoteEntry> = serde_json::from_str(json)
            .map_err(|e| format!("索引解析失败: {}", e))?;

        // 构建倒排索引
        let mut inverted: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, note) in notes.iter().enumerate() {
            // 标题 tokens（权重最高）
            for token in tokenize_text(&note.title) {
                inverted.entry(token).or_default().push(idx);
            }
            // 标签 tokens
            for tag in &note.tags {
                for token in tokenize_text(tag) {
                    inverted.entry(token).or_default().push(idx);
                }
            }
            // 内容 tokens
            for token in tokenize_text(&note.content) {
                inverted.entry(token).or_default().push(idx);
            }
        }

        Ok(NoteIndex { notes, inverted })
    }

    /// 返回索引中的笔记总数
    #[wasm_bindgen(js_name = noteCount)]
    pub fn note_count(&self) -> usize {
        self.notes.len()
    }

    /// 搜索笔记，返回 JSON 格式结果（与服务端 `/api/search` 响应格式一致）。
    ///
    /// # 评分规则
    /// - 标题完全匹配每个 token：+10 分
    /// - 标签匹配每个 token：+5 分
    /// - 内容摘要匹配每个 token：+1 分
    ///
    /// # 返回格式
    /// ```json
    /// [{"title":"...","path":"...","snippet":"...","score":15.0,"mtime":0,"tags":["..."]}]
    /// ```
    #[wasm_bindgen(js_name = searchJson)]
    pub fn search_json(&self, query: &str, limit: u32) -> String {
        if query.trim().is_empty() {
            return "[]".to_string();
        }

        let query_tokens: HashSet<String> = tokenize_text(query).into_iter().collect();
        if query_tokens.is_empty() {
            return "[]".to_string();
        }

        // 通过倒排索引快速找到候选笔记
        let mut candidate_indices: HashSet<usize> = HashSet::new();
        for token in &query_tokens {
            if let Some(indices) = self.inverted.get(token) {
                candidate_indices.extend(indices);
            }
        }

        // 对候选笔记评分
        let mut scored: Vec<(f32, &NoteEntry)> = candidate_indices.iter()
            .filter_map(|&idx| self.notes.get(idx).map(|note| idx_with_note(idx, note)))
            .map(|(_, note)| {
                let title_tokens: HashSet<String> = tokenize_text(&note.title).into_iter().collect();
                let tag_tokens: HashSet<String> = note.tags.iter()
                    .flat_map(|t| tokenize_text(t))
                    .collect();
                let content_tokens: HashSet<String> = tokenize_text(&note.content).into_iter().collect();

                let score: f32 = query_tokens.iter().map(|token| {
                    let mut s = 0.0f32;
                    if title_tokens.contains(token)   { s += 10.0; }
                    if tag_tokens.contains(token)     { s += 5.0; }
                    if content_tokens.contains(token) { s += 1.0; }
                    s
                }).sum();

                (score, note)
            })
            .filter(|(score, _)| *score > 0.0)
            .collect();

        // 按分数降序，同分按 mtime 降序
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.1.mtime.cmp(&a.1.mtime))
        });
        scored.truncate(limit as usize);

        let results: Vec<SearchResult> = scored.into_iter().map(|(score, note)| {
            // 生成包含查询词上下文的摘要
            let snippet = make_snippet(&note.content, &query_tokens, 150);
            SearchResult {
                title: note.title.clone(),
                path: note.path.clone(),
                snippet,
                score,
                mtime: note.mtime,
                tags: note.tags.clone(),
            }
        }).collect();

        serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string())
    }
}

/// 辅助：将 note 索引与引用配对
fn idx_with_note(idx: usize, note: &NoteEntry) -> (usize, &NoteEntry) {
    (idx, note)
}

/// n-gram 分词器（v1.6.2）
///
/// - ASCII：以非字母数字为分隔符切分单词（小写化）
/// - CJK：生成单字 unigram + 相邻双字 bigram
/// 支持中、日、韩文及 ASCII 混合文本的基本搜索。
fn tokenize_text(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut tokens = Vec::new();

    // ASCII 词切分（忽略单字符 token）
    for word in lower.split(|c: char| !c.is_alphanumeric()) {
        if word.len() >= 2 {
            tokens.push(word.to_string());
        }
    }

    // CJK unigram + bigram
    let chars: Vec<char> = lower.chars().collect();
    for i in 0..chars.len() {
        let c = chars[i];
        if is_cjk(c) {
            tokens.push(c.to_string()); // unigram
            if i + 1 < chars.len() && is_cjk(chars[i + 1]) {
                let bigram: String = [c, chars[i + 1]].iter().collect();
                tokens.push(bigram); // bigram（提高精确度）
            }
        }
    }

    tokens
}

/// 判断字符是否属于 CJK 范围（包含中文、日文假名、韩文）
fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}'  // 中文基本汉字
        | '\u{3400}'..='\u{4DBF}' // 扩展汉字A
        | '\u{3040}'..='\u{309F}' // 平假名
        | '\u{30A0}'..='\u{30FF}' // 片假名
        | '\u{AC00}'..='\u{D7AF}' // 韩文音节
    )
}

/// 在内容中找到第一个 query token 的位置，提取上下文摘要
fn make_snippet(content: &str, query_tokens: &HashSet<String>, max_len: usize) -> String {
    let content_lower = content.to_lowercase();

    // 找到第一个匹配位置
    let first_match = query_tokens.iter()
        .filter_map(|token| content_lower.find(token.as_str()))
        .min();

    let start = first_match
        .map(|pos| pos.saturating_sub(30))
        .unwrap_or(0);
    let end = (start + max_len).min(content.len());

    // 安全切断 UTF-8 边界
    let mut s = start;
    let mut e = end;
    while s > 0 && !content.is_char_boundary(s) { s -= 1; }
    while e < content.len() && !content.is_char_boundary(e) { e += 1; }

    let snippet = &content[s..e];
    let prefix = if s > 0 { "..." } else { "" };
    let suffix = if e < content.len() { "..." } else { "" };
    format!("{}{}{}", prefix, snippet, suffix)
}

// ─── 单元测试（运行于原生目标，无需 wasm-pack）──────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── NoteIndex 测试（v1.6.2）──────────────────────────────────────────────

    #[test]
    fn test_note_index_load_and_search() {
        let json = r#"[
            {"title":"Rust 编程指南","path":"rust.md","tags":["rust","编程"],"content":"Rust 是一门系统编程语言，强调安全性和性能。","mtime":0},
            {"title":"Python 教程","path":"python.md","tags":["python"],"content":"Python 是一门简洁的脚本语言，广泛应用于数据科学。","mtime":0}
        ]"#;
        let idx = NoteIndex::load_json(json).expect("加载失败");
        assert_eq!(idx.note_count(), 2, "应加载 2 条笔记");

        let results: serde_json::Value = serde_json::from_str(&idx.search_json("rust", 10)).unwrap();
        assert!(results.as_array().map_or(0, |a| a.len()) >= 1, "应找到 Rust 相关笔记");
    }

    #[test]
    fn test_note_index_empty_query() {
        let json = r#"[{"title":"笔记A","path":"a.md","tags":[],"content":"内容A","mtime":0}]"#;
        let idx = NoteIndex::load_json(json).expect("加载失败");
        let results_str = idx.search_json("", 10);
        assert_eq!(results_str, "[]", "空查询应返回空数组");
    }

    #[test]
    fn test_note_index_cjk_search() {
        let json = r#"[
            {"title":"笔记索引优化","path":"notes.md","tags":["优化"],"content":"笔记管理和索引优化是提高效率的关键。","mtime":0},
            {"title":"Python 基础","path":"py.md","tags":[],"content":"基础语法和函数定义。","mtime":0}
        ]"#;
        let idx = NoteIndex::load_json(json).expect("加载失败");
        let results: serde_json::Value = serde_json::from_str(&idx.search_json("优化", 10)).unwrap();
        let arr = results.as_array().unwrap();
        assert!(!arr.is_empty(), "应找到包含'优化'的笔记");
    }

    #[test]
    fn test_tokenize_text() {
        let tokens = tokenize_text("Hello World");
        assert!(tokens.contains(&"hello".to_string()), "应包含 hello");
        assert!(tokens.contains(&"world".to_string()), "应包含 world");

        let cjk_tokens = tokenize_text("中文搜索");
        assert!(cjk_tokens.contains(&"中".to_string()), "应包含单字 unigram");
        assert!(cjk_tokens.contains(&"中文".to_string()), "应包含双字 bigram");
    }

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
