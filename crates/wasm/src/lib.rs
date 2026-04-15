//! obsidian_mirror WebAssembly 模块（v1.8.0）
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
use std::collections::HashMap;
use std::collections::HashSet;

// ─── v1.6.5：Bitset —— 位图候选集（替换 HashSet<usize>）────────────────────

/// 位图结构，用于在 NoteIndex.search_json 中存储候选笔记索引集合。
///
/// 相比 `HashSet<usize>`：
/// - 无哈希计算开销，位或赋值（`|=`）即可完成合并
/// - 1000 条笔记仅需 16 个 u64 = 128 字节（完全 L1 缓存友好）
/// - 无堆再分配，posting list 合并为零分配操作
struct Bitset {
    /// u64 数组，第 i 位代表 note 索引 i 是否为候选
    bits: Vec<u64>,
}

impl Bitset {
    /// 创建空位图，容量为 note 总数
    fn new(note_count: usize) -> Self {
        Bitset { bits: vec![0u64; (note_count + 63) / 64] }
    }

    /// 将 posting list 中的所有索引加入候选集（位或运算，零分配）
    fn union_with_slice(&mut self, indices: &[usize]) {
        for &idx in indices {
            let word = idx / 64;
            if word < self.bits.len() {
                self.bits[word] |= 1u64 << (idx % 64);
            }
        }
    }

    /// 遍历所有置位的索引（使用 trailing_zeros 位扫描，比迭代快）
    fn iter_ones(&self) -> BitsetIter<'_> {
        BitsetIter { bits: &self.bits, word_idx: 0, word: self.bits.first().copied().unwrap_or(0) }
    }
}

/// 位图迭代器：依次返回所有置位的 note 索引
struct BitsetIter<'a> {
    bits: &'a [u64],
    word_idx: usize,
    word: u64,
}

impl Iterator for BitsetIter<'_> {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        // 跳过值为 0 的字，直到找到下一个置位
        while self.word == 0 {
            self.word_idx += 1;
            if self.word_idx >= self.bits.len() { return None; }
            self.word = self.bits[self.word_idx];
        }
        // trailing_zeros 找到最低置位，然后清除它
        let bit = self.word.trailing_zeros() as usize;
        self.word &= self.word - 1; // 清除最低置位（Brian Kernighan 技巧）
        Some(self.word_idx * 64 + bit)
    }
}

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

// ─── M1 共享助手：大小写不敏感子串查找（v1.6.4）────────────────────────────

/// 大小写不敏感子串查找，避免分配 `haystack.to_lowercase()`。
///
/// `needle_lower` 必须已预先转换为小写。
/// ASCII 字节做 `to_ascii_lowercase()` 比较；非 ASCII（CJK 等）直接字节比较。
fn find_substr_ci(haystack: &str, needle_lower: &str) -> Option<usize> {
    if needle_lower.is_empty() { return Some(0); }
    let h = haystack.as_bytes();
    let n = needle_lower.as_bytes();
    let hlen = h.len();
    let nlen = n.len();
    if hlen < nlen { return None; }

    'outer: for i in 0..=(hlen - nlen) {
        if !haystack.is_char_boundary(i) { continue; }
        for j in 0..nlen {
            let hb = if h[i + j] < 128 { h[i + j].to_ascii_lowercase() } else { h[i + j] };
            if hb != n[j] { continue 'outer; }
        }
        return Some(i);
    }
    None
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
    // M1 优化：只分配 term_lower（小），用 find_substr_ci 避免分配 text_lower（大）
    let term_lower = term.to_lowercase();

    let mut result = String::with_capacity(text.len() + 24);
    let mut last_end = 0;
    let mut search_start = 0;

    while let Some(rel_pos) = find_substr_ci(&text[search_start..], &term_lower) {
        let abs_pos = search_start + rel_pos;
        let term_end = abs_pos + term_lower.len();
        if term_end > text.len() || !text.is_char_boundary(abs_pos) || !text.is_char_boundary(term_end) {
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
/// 预计算的笔记 token 集合（load_json 时生成，搜索时 O(1) 查询）
///
/// 将 tokenize_text 从每次搜索移到加载阶段，消除 O(N × content_len) 的重复计算，
/// 使 search_json 从 ~10ms → < 1ms（1000 条笔记）。
struct TokenCache {
    title_tokens:   HashSet<String>,
    tag_tokens:     HashSet<String>,
    content_tokens: HashSet<String>,
}

#[wasm_bindgen]
pub struct NoteIndex {
    /// 所有笔记条目（原始数据，用于返回结果）
    notes: Vec<NoteEntry>,
    /// 预计算 token 集合（与 notes 按 idx 对应，搜索时直接查询）
    token_cache: Vec<TokenCache>,
    /// 倒排索引：token → [note_index...]（加速候选筛选）
    inverted: HashMap<String, Vec<usize>>,
}

#[wasm_bindgen]
impl NoteIndex {
    /// 从服务端 index.json 的 JSON 字符串加载索引。
    ///
    /// index.json 格式：`[{title, path, tags, content, mtime}, ...]`
    ///
    /// **性能优化（v1.6.3+）**：加载时一次性分词并缓存所有字段的 token 集合，
    /// 搜索时直接查询缓存，消除重复分词开销。
    #[wasm_bindgen(js_name = loadJson)]
    pub fn load_json(json: &str) -> Result<NoteIndex, String> {
        let notes: Vec<NoteEntry> = serde_json::from_str(json)
            .map_err(|e| format!("索引解析失败: {}", e))?;

        let n = notes.len();
        let mut inverted: HashMap<String, Vec<usize>> = HashMap::new();
        let mut token_cache: Vec<TokenCache> = Vec::with_capacity(n);

        for (idx, note) in notes.iter().enumerate() {
            // 一次性分词，结果既用于倒排索引也缓存到 token_cache
            let title_tokens: HashSet<String> = tokenize_text(&note.title).into_iter().collect();
            let tag_tokens: HashSet<String>   = note.tags.iter()
                .flat_map(|t| tokenize_text(t))
                .collect();
            let content_tokens: HashSet<String> = tokenize_text(&note.content).into_iter().collect();

            // 构建倒排索引（直接从缓存集合迭代，无需重新分词）
            for token in &title_tokens {
                inverted.entry(token.clone()).or_default().push(idx);
            }
            for token in &tag_tokens {
                inverted.entry(token.clone()).or_default().push(idx);
            }
            for token in &content_tokens {
                inverted.entry(token.clone()).or_default().push(idx);
            }

            token_cache.push(TokenCache { title_tokens, tag_tokens, content_tokens });
        }

        Ok(NoteIndex { notes, inverted, token_cache })
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

        let all_tokens: HashSet<String> = tokenize_text(query).into_iter().collect();
        if all_tokens.is_empty() {
            return "[]".to_string();
        }

        // M3 优化（v1.6.4）：限制查询 token 数为最多 8 个，优先选取最稀有（命中数最少）的 token。
        // CJK 查询会产生大量 unigram + bigram token，不加限制时候选集过大导致搜索变慢。
        // "最稀有" = 倒排索引中命中文档数最少 → 最具区分度。
        let query_tokens: Vec<String> = {
            let mut tokens: Vec<String> = all_tokens.into_iter().collect();
            tokens.sort_by_key(|t| self.inverted.get(t).map_or(0, |v| v.len()));
            tokens.truncate(8);
            tokens
        };

        // v1.6.5 M3-续：用 Bitset 替换 HashSet<usize> 候选集
        // 位或运算合并 posting list，零分配，128 字节完全 L1 缓存友好（1000 条笔记）
        let n = self.notes.len();
        let mut candidate_bits = Bitset::new(n);
        for token in &query_tokens {
            if let Some(indices) = self.inverted.get(token) {
                candidate_bits.union_with_slice(indices);
            }
        }

        // M3：预计算每个 query token 的标题权重（在候选循环外执行，每次搜索只算一次）
        // CJK bigram（精确双字）命中标题得 15 分，其他得 10 分
        let title_weights: Vec<f32> = query_tokens.iter()
            .map(|t| if is_cjk_bigram(t) { 15.0_f32 } else { 10.0 })
            .collect();

        // v1.8.6：回退 M4，恢复原始 HashSet.contains() 评分策略。
        //
        // M4（v1.7.0）的位掩码方案在 content_tokens 集合较大时产生逆效应：
        //   迭代所有 content_tokens 建位掩码 = O(n_content_tokens)
        //   vs 原方案 HashSet.contains() = O(1)
        // 基准测试（PERFORMANCE-1.8）显示 ASCII 搜索 200µs → 1855µs（+828%）。
        //
        // 当前方案（v1.8.6）：对每个 query_token（≤8）直接查三个 HashSet，
        //   总复杂度 = O(8 × 3) = O(24) 次 O(1) 哈希查询，不受 content_tokens 规模影响。
        let mut scored: Vec<(f32, &NoteEntry)> = candidate_bits.iter_ones()
            .filter_map(|idx| {
                let note   = self.notes.get(idx)?;
                let cached = self.token_cache.get(idx)?;

                // 对每个 query_token 查三个预缓存 HashSet，O(1) 每次
                let score: f32 = query_tokens.iter().zip(title_weights.iter()).map(|(token, &tw)| {
                    let mut s = 0.0f32;
                    if cached.title_tokens.contains(token.as_str())   { s += tw; }
                    if cached.tag_tokens.contains(token.as_str())     { s += 5.0; }
                    if cached.content_tokens.contains(token.as_str()) { s += 1.0; }
                    s
                }).sum();

                if score > 0.0 { Some((score, note)) } else { None }
            })
            .collect();

        // 按分数降序，同分按 mtime 降序
        scored.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b.1.mtime.cmp(&a.1.mtime))
        });
        scored.truncate(limit as usize);

        // make_snippet 接受 HashSet，在迭代器外转换（仅执行一次）
        let query_token_set: HashSet<String> = query_tokens.iter().cloned().collect();

        let results: Vec<SearchResult> = scored.into_iter().map(|(score, note)| {
            let snippet = make_snippet(&note.content, &query_token_set, 150);
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

/// 判断 token 是否为精确 CJK bigram（2 个 CJK 字符，用于 M3 权重提升）
/// 不分配堆内存：用 chars() 迭代器判断恰好 2 个 CJK 字符
fn is_cjk_bigram(token: &str) -> bool {
    let mut chars = token.chars();
    match (chars.next(), chars.next(), chars.next()) {
        (Some(c1), Some(c2), None) => is_cjk(c1) && is_cjk(c2),
        _ => false,
    }
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

// ─── v1.6.3：图谱布局（Force-Directed）──────────────────────────────────────

/// 图谱节点输入格式
#[derive(Debug, Deserialize)]
struct GraphNode {
    /// 节点唯一 ID（与 Vis.js 节点 id 一致）
    id: String,
}

/// 图谱边输入格式
#[derive(Debug, Deserialize)]
struct GraphEdge {
    /// 起始节点 ID
    from: String,
    /// 终止节点 ID
    to: String,
}

/// 节点布局位置输出格式
#[derive(Serialize)]
struct NodePosition {
    id: String,
    x: f64,
    y: f64,
}

/// 使用 Fruchterman-Reingold 算法计算力导向图谱布局坐标（v1.6.3）。
///
/// 性能目标：500 节点 30 次迭代 < 200ms（WASM Release 模式）。
///
/// # 参数
/// - `nodes_json`：`[{"id":"..."},...]` 格式的节点 JSON
/// - `edges_json`：`[{"from":"...","to":"..."},...]` 格式的边 JSON
/// - `iterations`：迭代次数（建议：<100节点→50次，<300节点→30次，>300节点→15次）
///
/// # 返回
/// `[{"id":"...","x":100.0,"y":-50.0},...]` 格式的 JSON 字符串
// ─── M2：Barnes-Hut 四叉树（v1.6.4）────────────────────────────────────────
//
// 用于加速 compute_graph_layout 中的排斥力计算：
// - O(n²) Fruchterman-Reingold → O(n log n) Barnes-Hut
// - 适用于 n > 100 的图，小图仍使用精确 O(n²) 计算

/// 四叉树节点，实现 Barnes-Hut 近似排斥力计算
enum QuadTree {
    /// 空节点
    Empty,
    /// 叶子节点（含一个点）
    Leaf { idx: usize, x: f64, y: f64 },
    /// 内部节点（含多个点，用质心聚合）
    Internal {
        /// 区域范围 [x_min, y_min, x_max, y_max]
        bounds: [f64; 4],
        /// 区域内点的数量（用于质心权重）
        total_mass: f64,
        /// 质心 x 坐标
        cx: f64,
        /// 质心 y 坐标
        cy: f64,
        /// 四个子象限 [SW, SE, NW, NE]
        children: Box<[QuadTree; 4]>,
    },
}

impl QuadTree {
    /// 将一个点插入四叉树
    fn insert(&mut self, idx: usize, x: f64, y: f64, bounds: [f64; 4]) {
        match self {
            QuadTree::Empty => {
                *self = QuadTree::Leaf { idx, x, y };
            }
            QuadTree::Leaf { .. } => {
                // 取出旧叶子，升级为内部节点后重新插入
                if let QuadTree::Leaf { idx: oi, x: ox, y: oy } = std::mem::replace(self, QuadTree::Empty) {
                    let mx = (bounds[0] + bounds[2]) / 2.0;
                    let my = (bounds[1] + bounds[3]) / 2.0;
                    *self = QuadTree::Internal {
                        bounds,
                        total_mass: 0.0,
                        cx: mx, cy: my,
                        children: Box::new([QuadTree::Empty, QuadTree::Empty,
                                            QuadTree::Empty, QuadTree::Empty]),
                    };
                    self.insert(oi, ox, oy, bounds);
                    self.insert(idx, x, y, bounds);
                }
            }
            QuadTree::Internal { bounds: b, total_mass, cx, cy, children } => {
                // 更新质心（增量公式）
                *total_mass += 1.0;
                *cx = (*cx * (*total_mass - 1.0) + x) / *total_mass;
                *cy = (*cy * (*total_mass - 1.0) + y) / *total_mass;
                // 决定插入哪个象限并递归
                let mx = (b[0] + b[2]) / 2.0;
                let my = (b[1] + b[3]) / 2.0;
                let quad = usize::from(x >= mx) + 2 * usize::from(y >= my);
                let cb = [
                    if quad & 1 == 0 { b[0] } else { mx },
                    if quad & 2 == 0 { b[1] } else { my },
                    if quad & 1 == 0 { mx   } else { b[2] },
                    if quad & 2 == 0 { my   } else { b[3] },
                ];
                children[quad].insert(idx, x, y, cb);
            }
        }
    }

    /// 计算来自此子树的排斥力（Barnes-Hut 近似）。
    ///
    /// `px, py`：查询点坐标；`self_idx`：查询点索引（跳过自身）
    /// `k_sq`：最优距离平方；`theta_sq`：近似阈值平方（θ²，默认 0.81）
    fn repulsion_force(&self, px: f64, py: f64, k_sq: f64, theta_sq: f64, self_idx: usize) -> (f64, f64) {
        match self {
            QuadTree::Empty => (0.0, 0.0),
            QuadTree::Leaf { idx, x, y } => {
                if *idx == self_idx { return (0.0, 0.0); }
                let dx = px - x;
                let dy = py - y;
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                let rep = k_sq / dist;
                (dx / dist * rep, dy / dist * rep)
            }
            QuadTree::Internal { bounds, total_mass, cx, cy, children } => {
                let dx = px - cx;
                let dy = py - cy;
                let dist_sq = (dx * dx + dy * dy).max(1.0);
                // s = 区域最大边长
                let s = (bounds[2] - bounds[0]).max(bounds[3] - bounds[1]);
                // Barnes-Hut 条件：s²/d² < θ² → 用质心近似
                if s * s < theta_sq * dist_sq {
                    let dist = dist_sq.sqrt();
                    let rep = k_sq * total_mass / dist;
                    (dx / dist * rep, dy / dist * rep)
                } else {
                    // 递归到子节点
                    let (mut rfx, mut rfy) = (0.0, 0.0);
                    for child in children.iter() {
                        let (cx, cy) = child.repulsion_force(px, py, k_sq, theta_sq, self_idx);
                        rfx += cx;
                        rfy += cy;
                    }
                    (rfx, rfy)
                }
            }
        }
    }
}

#[wasm_bindgen(js_name = computeGraphLayout)]
pub fn compute_graph_layout(nodes_json: &str, edges_json: &str, iterations: u32) -> String {
    let nodes: Vec<GraphNode> = match serde_json::from_str(nodes_json) {
        Ok(v) => v,
        Err(_) => return "[]".to_string(),
    };
    let edges: Vec<GraphEdge> = serde_json::from_str(edges_json).unwrap_or_default();

    let n = nodes.len();
    if n == 0 { return "[]".to_string(); }
    if n == 1 {
        let r = serde_json::json!([{"id": nodes[0].id, "x": 0.0, "y": 0.0}]);
        return r.to_string();
    }

    // 初始化位置：圆形均匀分布，避免重叠
    let init_r = 500.0_f64 * (n as f64).sqrt();
    let mut pos_x: Vec<f64> = (0..n)
        .map(|i| init_r * (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos())
        .collect();
    let mut pos_y: Vec<f64> = (0..n)
        .map(|i| init_r * (2.0 * std::f64::consts::PI * i as f64 / n as f64).sin())
        .collect();

    // 建立节点 ID → 索引映射
    let id_to_idx: HashMap<&str, usize> = nodes.iter()
        .enumerate()
        .map(|(i, node)| (node.id.as_str(), i))
        .collect();

    // 解析有效边（仅保留两端都存在于节点列表中的边）
    let adj_edges: Vec<(usize, usize)> = edges.iter()
        .filter_map(|e| {
            let i = id_to_idx.get(e.from.as_str())?;
            let j = id_to_idx.get(e.to.as_str())?;
            if i != j { Some((*i, *j)) } else { None }
        })
        .collect();

    // Fruchterman-Reingold 参数
    let area = 2000.0_f64 * 2000.0_f64;
    let k = (area / n as f64).sqrt();
    let k_sq = k * k;
    let mut temp = 200.0_f64;
    let cooling = 0.85_f64;

    // M2（v1.6.4）：n > 100 时使用 Barnes-Hut O(n log n) 近似代替 O(n²) 全对排斥力
    let use_barnes_hut = n > 100;
    // M5（v1.7.0）：θ 自适应——前 60% 迭代用较大 θ 快速收敛，后 40% 用较小 θ 精细布局。
    //   θ = 1.2（前期）：s/d < 1.2 时用质心近似，允许更多近似 → 每迭代更快
    //   θ = 0.7（后期）：s/d < 0.7 时用质心近似，精度更高 → 布局抖动更小
    //   θ_sq = θ² 避免每次迭代重复乘法
    let theta_early_sq = 1.2_f64 * 1.2_f64; // 前 60% 迭代
    let theta_late_sq  = 0.7_f64 * 0.7_f64; // 后 40% 迭代
    let warmup_iters = (iterations as f64 * 0.6).round() as u32;

    for iter_idx in 0..iterations {
        let mut fx = vec![0.0_f64; n];
        let mut fy = vec![0.0_f64; n];

        if use_barnes_hut {
            // Barnes-Hut：每轮构建四叉树，每节点 O(log n) 查询排斥力
            let pad = init_r * 0.05 + 1.0;
            let x_min = pos_x.iter().cloned().fold(f64::INFINITY, f64::min) - pad;
            let y_min = pos_y.iter().cloned().fold(f64::INFINITY, f64::min) - pad;
            let x_max = pos_x.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + pad;
            let y_max = pos_y.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + pad;
            let bounds = [x_min, y_min, x_max, y_max];

            let mut tree = QuadTree::Empty;
            for i in 0..n {
                tree.insert(i, pos_x[i], pos_y[i], bounds);
            }

            // M5：根据迭代阶段选择 θ（前期快速收敛，后期精细布局）
            let theta_sq = if iter_idx < warmup_iters { theta_early_sq } else { theta_late_sq };
            for i in 0..n {
                let (rfx, rfy) = tree.repulsion_force(pos_x[i], pos_y[i], k_sq, theta_sq, i);
                fx[i] += rfx;
                fy[i] += rfy;
            }
        } else {
            // 小图：标准 O(n²) Fruchterman-Reingold（精确）
            for i in 0..n {
                for j in (i + 1)..n {
                    let dx = pos_x[i] - pos_x[j];
                    let dy = pos_y[i] - pos_y[j];
                    let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                    let repulsion = k_sq / dist;
                    let rx = dx / dist * repulsion;
                    let ry = dy / dist * repulsion;
                    fx[i] += rx;
                    fy[i] += ry;
                    fx[j] -= rx;
                    fy[j] -= ry;
                }
            }
        }

        // 吸引力（连边节点对，O(E)，两种模式相同）
        for &(i, j) in &adj_edges {
            let dx = pos_x[j] - pos_x[i];
            let dy = pos_y[j] - pos_y[i];
            let dist = (dx * dx + dy * dy).sqrt().max(1.0);
            let attraction = dist * dist / k;
            let ax = dx / dist * attraction;
            let ay = dy / dist * attraction;
            fx[i] += ax;
            fy[i] += ay;
            fx[j] -= ax;
            fy[j] -= ay;
        }

        // 应用合力（受温度限制最大位移）
        for i in 0..n {
            let magnitude = (fx[i] * fx[i] + fy[i] * fy[i]).sqrt().max(1.0);
            let displacement = magnitude.min(temp);
            pos_x[i] += fx[i] / magnitude * displacement;
            pos_y[i] += fy[i] / magnitude * displacement;
        }

        temp *= cooling;
    }

    // 居中（将重心移到原点）
    let cx = pos_x.iter().sum::<f64>() / n as f64;
    let cy = pos_y.iter().sum::<f64>() / n as f64;
    let result: Vec<NodePosition> = nodes.iter().enumerate()
        .map(|(i, node)| NodePosition {
            id: node.id.clone(),
            x: (pos_x[i] - cx).round(),
            y: (pos_y[i] - cy).round(),
        })
        .collect();

    serde_json::to_string(&result).unwrap_or_else(|_| "[]".to_string())
}

// ─── v1.6.3：本地搜索过滤 ────────────────────────────────────────────────────

/// 轻量笔记过滤条目（来自 /api/titles 的 note_items 字段）
#[derive(Debug, Deserialize, Serialize, Clone)]
struct FilterNote {
    title: String,
    path: String,
    #[serde(default)]
    tags: Vec<String>,
}

/// 本地 WASM 笔记过滤（v1.6.3）。
///
/// 从前端缓存的 `note_items` 列表中快速过滤，支持多标签交集匹配和路径前缀过滤。
/// 与服务端搜索互补：WASM 先给出本地建议，服务端异步补充全文搜索结果。
///
/// # 参数
/// - `notes_json`：`[{"title":"...","path":"...","tags":["..."]}]` 格式的 JSON
/// - `tags_filter`：逗号分隔的标签列表（全部匹配，OR 用多次调用实现）
/// - `folder_filter`：文件夹路径前缀（空字符串 = 不过滤）
/// - `limit`：最大返回条数
///
/// # 返回
/// 过滤后的 `[{"title":"...","path":"...","tags":[...]}]` JSON
#[wasm_bindgen(js_name = filterNotes)]
pub fn filter_notes(notes_json: &str, tags_filter: &str, folder_filter: &str, limit: u32) -> String {
    let notes: Vec<FilterNote> = match serde_json::from_str(notes_json) {
        Ok(v) => v,
        Err(_) => return "[]".to_string(),
    };

    let required_tags: Vec<&str> = tags_filter
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();

    let folder_lower = folder_filter.to_lowercase();

    let results: Vec<&FilterNote> = notes.iter()
        .filter(|note| {
            // 路径前缀过滤（大小写不敏感）
            if !folder_lower.is_empty() && !note.path.to_lowercase().starts_with(&folder_lower) {
                return false;
            }
            // 多标签交集过滤（ALL 语义）
            if !required_tags.is_empty() {
                let note_tags_lower: Vec<String> = note.tags.iter().map(|t| t.to_lowercase()).collect();
                for req_tag in &required_tags {
                    if !note_tags_lower.iter().any(|t| t == req_tag) {
                        return false;
                    }
                }
            }
            true
        })
        .take(limit as usize)
        .collect();

    serde_json::to_string(&results).unwrap_or_else(|_| "[]".to_string())
}

// ─── v1.6.3：客户端 TOC 生成 ─────────────────────────────────────────────────

/// TOC 条目
#[derive(Serialize)]
struct TocEntry {
    level: u8,
    text: String,
    id: String,
}

/// 从服务端渲染的 HTML 中提取目录（TOC），用于客户端快速刷新（v1.6.3）。
///
/// 扫描 `<h1>...<h6>` 标题元素，提取 `id` 属性和文本内容，
/// 生成与服务端 `Note.toc` 格式兼容的 JSON 数组。
///
/// 目标：100 个标题 < 1ms（替代服务端 TOC 字段，支持本地预览实时更新）。
///
/// # 参数
/// - `html`：渲染后的 HTML 字符串（来自 `render_markdown` 或服务端）
///
/// # 返回
/// `[{"level":2,"text":"标题","id":"anchor-id"}]` 格式的 JSON
#[wasm_bindgen(js_name = generateTocFromHtml)]
pub fn generate_toc_from_html(html: &str) -> String {
    lazy_static! {
        // 匹配 <h1 id="anchor">文本</h1> 等，捕获 id 属性和标题文本
        static ref HEADING_RE: Regex = Regex::new(
            r#"(?i)<h([1-6])(?:[^>]*?\bid=['"]([\w\-]+)['"][^>]*)?>([^<]*(?:<[^/][^>]*>[^<]*</[^>]+>[^<]*)*)</h[1-6]>"#
        ).unwrap();
        // 剥离内嵌标签（如 <code>、<em>）
        static ref TAG_STRIP_RE: Regex = Regex::new(r"<[^>]+>").unwrap();
    }

    let entries: Vec<TocEntry> = HEADING_RE.captures_iter(html)
        .filter_map(|caps| {
            let level: u8 = caps.get(1)?.as_str().parse().ok()?;
            let id = caps.get(2).map_or("", |m| m.as_str()).to_string();
            let raw_text = caps.get(3).map_or("", |m| m.as_str());
            // 剥离内嵌标签后获取纯文本
            let text = TAG_STRIP_RE.replace_all(raw_text, "")
                .trim()
                .to_string();
            if text.is_empty() { return None; }
            Some(TocEntry { level, text, id })
        })
        .collect();

    serde_json::to_string(&entries).unwrap_or_else(|_| "[]".to_string())
}

// ─── 单元测试（运行于原生目标，无需 wasm-pack）──────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── v1.6.5 测试：Bitset 位图 ────────────────────────────────────────────────

    #[test]
    fn test_bitset_basic() {
        let mut bs = Bitset::new(200);
        bs.union_with_slice(&[0, 63, 64, 127, 128, 199]);
        let ones: Vec<usize> = bs.iter_ones().collect();
        assert_eq!(ones, vec![0, 63, 64, 127, 128, 199], "应按升序返回所有置位索引");
    }

    #[test]
    fn test_bitset_empty() {
        let bs = Bitset::new(100);
        assert_eq!(bs.iter_ones().count(), 0, "空位图应无置位");
    }

    #[test]
    fn test_bitset_union_dedup() {
        let mut bs = Bitset::new(100);
        bs.union_with_slice(&[5, 10, 5, 10, 5]);
        let ones: Vec<usize> = bs.iter_ones().collect();
        assert_eq!(ones, vec![5, 10], "重复索引应自动去重（位图天然去重）");
    }

    #[test]
    fn test_bitset_search_correctness() {
        // 验证 Bitset 搜索结果与预期一致
        let json = r#"[
            {"title":"Rust 编程","path":"rust.md","tags":["rust"],"content":"系统编程语言","mtime":0},
            {"title":"Python 教程","path":"py.md","tags":["python"],"content":"脚本语言","mtime":0}
        ]"#;
        let idx = NoteIndex::load_json(json).expect("加载失败");
        let result_json = idx.search_json("rust", 10);
        let arr: serde_json::Value = serde_json::from_str(&result_json).unwrap();
        assert!(!arr.as_array().unwrap().is_empty(), "应找到 Rust 相关笔记");
        assert_eq!(arr.as_array().unwrap()[0]["title"], "Rust 编程");
    }

    // ─── v1.6.4 测试：M1/M2/M3 ──────────────────────────────────────────────────

    #[test]
    fn test_find_substr_ci_ascii() {
        // needle_lower 必须已预先转为小写
        assert_eq!(find_substr_ci("Hello World", "world"), Some(6));
        assert_eq!(find_substr_ci("Hello World", "hello"), Some(0)); // 搜索小写 hello → 匹配 Hello
        assert_eq!(find_substr_ci("Hello", "xyz"), None);
        assert_eq!(find_substr_ci("", "abc"), None);
        assert_eq!(find_substr_ci("Rust 编程", "rust"), Some(0));
    }

    #[test]
    fn test_find_substr_ci_cjk() {
        // CJK 无大小写，直接字节匹配
        assert_eq!(find_substr_ci("中文搜索", "搜索"), Some(6)); // "中文" = 6 bytes (2×3)
        assert_eq!(find_substr_ci("中文搜索", "中"), Some(0));
    }

    #[test]
    fn test_graph_layout_barnes_hut_large() {
        // n > 100 时自动使用 Barnes-Hut，结果应包含正确数量的节点
        let nodes: Vec<_> = (0..200).map(|i| format!(r#"{{"id":"n{}"}}"#, i)).collect();
        let nodes_json = format!("[{}]", nodes.join(","));
        let edges_json = "[]";
        let result = compute_graph_layout(&nodes_json, edges_json, 10);
        let arr: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(arr.as_array().unwrap().len(), 200, "Barnes-Hut 布局应返回 200 个节点位置");
    }

    #[test]
    fn test_m3_cjk_token_limit() {
        // 构造足够多笔记，验证 CJK 查询不会超时（行为测试）
        let entries: Vec<String> = (0..500).map(|i| format!(
            r#"{{"title":"笔记{}","path":"n{}.md","tags":[],"content":"内容{}","mtime":0}}"#, i, i, i
        )).collect();
        let json = format!("[{}]", entries.join(","));
        let idx = NoteIndex::load_json(&json).unwrap();
        // "编程语言" 会生成多个 token，M3 应限制为最多 8 个
        let result = idx.search_json("这是一个包含很多词语的长查询语句测试", 10);
        let arr: serde_json::Value = serde_json::from_str(&result).unwrap();
        // 只要不 panic/超时，结果可以为空
        assert!(arr.is_array(), "长 CJK 查询应正常返回数组");
    }

    // ─── v1.6.3 测试：图谱布局 + 搜索过滤 + TOC ────────────────────────────────

    #[test]
    fn test_compute_graph_layout_basic() {
        let nodes = r#"[{"id":"a"},{"id":"b"},{"id":"c"}]"#;
        let edges = r#"[{"from":"a","to":"b"},{"from":"b","to":"c"}]"#;
        let result = compute_graph_layout(nodes, edges, 10);
        let positions: serde_json::Value = serde_json::from_str(&result).unwrap();
        let arr = positions.as_array().unwrap();
        assert_eq!(arr.len(), 3, "应返回 3 个节点位置");
        // 每个节点有 id, x, y 字段
        for pos in arr {
            assert!(pos.get("id").is_some(), "应有 id 字段");
            assert!(pos.get("x").is_some(), "应有 x 字段");
            assert!(pos.get("y").is_some(), "应有 y 字段");
        }
    }

    #[test]
    fn test_compute_graph_layout_empty() {
        let result = compute_graph_layout("[]", "[]", 10);
        assert_eq!(result, "[]", "空图应返回空数组");
    }

    #[test]
    fn test_filter_notes_by_tag() {
        let notes = r#"[
            {"title":"Rust 笔记","path":"rust.md","tags":["rust","编程"]},
            {"title":"Python 笔记","path":"python.md","tags":["python"]},
            {"title":"Rust Web","path":"web/rust.md","tags":["rust","web"]}
        ]"#;
        let result = filter_notes(notes, "rust", "", 10);
        let arr: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(arr.as_array().unwrap().len(), 2, "应过滤出 2 条 rust 标签笔记");
    }

    #[test]
    fn test_filter_notes_by_folder() {
        let notes = r#"[
            {"title":"根目录笔记","path":"root.md","tags":[]},
            {"title":"子目录笔记","path":"web/note.md","tags":[]}
        ]"#;
        let result = filter_notes(notes, "", "web/", 10);
        let arr: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(arr.as_array().unwrap().len(), 1, "应只返回 web/ 路径下的笔记");
    }

    #[test]
    fn test_generate_toc_basic() {
        let html = r#"<h1 id="h-1">主标题</h1><p>段落</p><h2 id="h-2">小节</h2>"#;
        let result = generate_toc_from_html(html);
        let toc: serde_json::Value = serde_json::from_str(&result).unwrap();
        let arr = toc.as_array().unwrap();
        assert_eq!(arr.len(), 2, "应提取 2 个标题");
        assert_eq!(arr[0]["level"], 1, "第一个应为 h1");
        assert_eq!(arr[0]["text"], "主标题");
        assert_eq!(arr[0]["id"], "h-1");
    }

    #[test]
    fn test_generate_toc_empty() {
        let result = generate_toc_from_html("<p>无标题段落</p>");
        assert_eq!(result, "[]", "无标题时应返回空数组");
    }

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

    // ─── v1.7.0 测试：M4 评分 Bitset ──────────────────────────────────────────

    /// M4 验证：多 query token 命中标题/标签/正文时分数应与期望一致
    #[test]
    fn test_m4_scoring_correctness() {
        use serde_json::json;
        // 构建包含 1 条笔记的 NoteIndex
        let notes_json = serde_json::to_string(&json!([
            {
                "title": "Rust 编程指南",
                "path": "guide.md",
                "tags": ["rust"],
                "content": "介绍 Rust 语言基础知识",
                "mtime": 0
            }
        ])).unwrap();
        let idx = NoteIndex::load_json(&notes_json).expect("load_json 应成功");

        // 搜索 "rust"：应在标题和标签中命中，分数 = title_weight(10) + tag(5) = 15
        let result_json = idx.search_json("rust", 10);
        let results: Vec<serde_json::Value> = serde_json::from_str(&result_json).unwrap();
        assert_eq!(results.len(), 1, "应找到 1 条笔记");
        let score = results[0]["score"].as_f64().unwrap();
        // 标题命中 10 + 标签命中 5 + 内容未含 "rust"（title/tag 已覆盖），score ≥ 10
        assert!(score >= 10.0, "命中标题应得分 ≥ 10，实际={}", score);
    }

    /// M4 验证：不相关查询应返回空结果
    #[test]
    fn test_m4_no_match_returns_empty() {
        use serde_json::json;
        let notes_json = serde_json::to_string(&json!([
            {"title":"笔记 A","path":"a.md","tags":[],"content":"hello world","mtime":0}
        ])).unwrap();
        let idx = NoteIndex::load_json(&notes_json).expect("load_json 应成功");
        let result = idx.search_json("完全不存在的词语xyz", 10);
        assert_eq!(result, "[]", "无命中应返回空数组");
    }

    // ─── v1.7.0 测试：M5 θ 自适应 Barnes-Hut ──────────────────────────────────

    /// M5 验证：θ 自适应不影响布局结果的有效性（节点数量与坐标有限）
    #[test]
    fn test_m5_theta_adaptive_layout_valid() {
        // 150 节点触发 Barnes-Hut，验证 M5 自适应 θ 不产生 NaN/Inf 坐标
        let nodes: Vec<_> = (0..150).map(|i| format!(r#"{{"id":"n{}"}}"#, i)).collect();
        let nodes_json = format!("[{}]", nodes.join(","));
        let result = compute_graph_layout(&nodes_json, "[]", 30);
        let arr: serde_json::Value = serde_json::from_str(&result).unwrap();
        let positions = arr.as_array().unwrap();
        assert_eq!(positions.len(), 150, "应返回 150 个节点位置");
        // 所有坐标应为有限值
        for pos in positions {
            let x = pos["x"].as_f64().unwrap_or(f64::NAN);
            let y = pos["y"].as_f64().unwrap_or(f64::NAN);
            assert!(x.is_finite(), "x 坐标不应为 NaN/Inf：{}", x);
            assert!(y.is_finite(), "y 坐标不应为 NaN/Inf：{}", y);
        }
    }

    /// M5 验证：小图（≤ 100 节点）不使用 Barnes-Hut，M5 不影响其路径
    #[test]
    fn test_m5_small_graph_unaffected() {
        let nodes: Vec<_> = (0..50).map(|i| format!(r#"{{"id":"s{}"}}"#, i)).collect();
        let nodes_json = format!("[{}]", nodes.join(","));
        let result = compute_graph_layout(&nodes_json, "[]", 20);
        let arr: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(arr.as_array().unwrap().len(), 50, "小图应返回 50 个节点位置");
    }
}
