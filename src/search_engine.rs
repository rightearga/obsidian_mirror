// 使用 Tantivy 实现的高性能搜索引擎
// 专为海量笔记库（5000+ 文件）优化

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::SystemTime;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument};
use tracing::{info, warn};

// 搜索结果排序方式
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum SortBy {
    #[default]
    Relevance, // 按相关度排序（默认）
    Modified,  // 按修改时间排序
}


// 搜索结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub title: String,
    pub path: String,
    pub snippet: String,
    pub score: f32,
    pub mtime: i64,         // Unix 时间戳（秒）
    pub tags: Vec<String>,  // 笔记标签列表（用于搜索结果卡片展示）
}

// 搜索引擎
pub struct SearchEngine {
    index: Index,
    /// 缓存的 IndexReader，避免每次搜索重复创建，节省初始化开销
    /// ReloadPolicy::OnCommitWithDelay 确保写入提交后自动更新视图
    reader: IndexReader,
    title_field: Field,
    content_field: Field,
    path_field: Field,
    mtime_field: Field,  // 修改时间字段
    tags_field: Field,   // 标签字段
    folder_field: Field, // 文件夹字段
}

impl SearchEngine {
    /// 创建新的搜索引擎实例
    pub fn new(index_dir: &Path) -> Result<Self> {
        // 定义当前版本的 schema
        let mut schema_builder = Schema::builder();

        let title_field = schema_builder.add_text_field("title", TEXT | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let path_field = schema_builder.add_text_field("path", STRING | STORED);
        let mtime_field = schema_builder.add_i64_field("mtime", INDEXED | STORED); // 用于排序
        let tags_field = schema_builder.add_text_field("tags", STRING | STORED); // 标签字段（可多值）
        let folder_field = schema_builder.add_text_field("folder", STRING | STORED); // 文件夹路径

        let schema = schema_builder.build();

        // 创建或打开索引
        let index = if index_dir.exists() {
            // 尝试打开现有索引
            match Index::open_in_dir(index_dir) {
                Ok(existing_index) => {
                    // 检查 schema 是否匹配
                    let existing_schema = existing_index.schema();
                    if Self::schema_matches(&existing_schema, &schema) {
                        info!("  └─ 使用现有索引");
                        existing_index
                    } else {
                        // Schema 不匹配，删除旧索引并重建
                        warn!("  ├─ 检测到 schema 变更，正在删除旧索引...");
                        if let Err(e) = std::fs::remove_dir_all(index_dir) {
                            warn!("  ├─ 删除旧索引失败: {:?}", e);
                        }
                        std::fs::create_dir_all(index_dir)?;
                        info!("  └─ 创建新索引");
                        Index::create_in_dir(index_dir, schema.clone())?
                    }
                }
                Err(e) => {
                    // 打开失败，删除并重建
                    warn!("  ├─ 打开索引失败: {:?}，正在重建...", e);
                    if let Err(e) = std::fs::remove_dir_all(index_dir) {
                        warn!("  ├─ 删除损坏的索引失败: {:?}", e);
                    }
                    std::fs::create_dir_all(index_dir)?;
                    info!("  └─ 创建新索引");
                    Index::create_in_dir(index_dir, schema.clone())?
                }
            }
        } else {
            std::fs::create_dir_all(index_dir)?;
            info!("  └─ 创建新索引");
            Index::create_in_dir(index_dir, schema.clone())?
        };

        // 初始化并缓存 IndexReader，后续搜索直接复用，避免重复创建。
        // 使用 ReloadPolicy::Manual（非 OnCommitWithDelay）：
        //   OnCommitWithDelay 会在 commit 后启动后台线程自动重载，
        //   该后台线程与 IndexWriter 的段合并操作在 Windows 上争抢相同文件的读/写锁，
        //   导致 "拒绝访问（error code 5）"。
        //   Manual 模式下无后台线程，rebuild_index / update_documents 在 commit 后
        //   手动调用 reader.reload()，保证写入完全完成后再开放读取。
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            title_field,
            content_field,
            path_field,
            mtime_field,
            tags_field,
            folder_field,
        })
    }

    /// 公开的 schema_matches 包装，仅供测试使用
    #[cfg(test)]
    pub fn schema_matches_pub(s1: &Schema, s2: &Schema) -> bool {
        Self::schema_matches(s1, s2)
    }

    /// 检查两个 schema 是否匹配（字段名 + 字段类型均需一致）
    ///
    /// 同时比较字段名和字段类型变体（TEXT/STRING/I64 等），
    /// 防止字段类型变更后仍复用旧 schema 导致索引读写错误。
    fn schema_matches(schema1: &Schema, schema2: &Schema) -> bool {
        let fields1: Vec<_> = schema1.fields().collect();
        let fields2: Vec<_> = schema2.fields().collect();

        if fields1.len() != fields2.len() {
            return false;
        }

        for (_field, entry2) in schema2.fields() {
            match schema1.get_field(entry2.name()) {
                Err(_) => return false,
                Ok(f1) => {
                    // 不仅检查字段名，还比较字段类型变体（TEXT vs STRING vs I64 等）
                    let entry1 = schema1.get_field_entry(f1);
                    if std::mem::discriminant(entry1.field_type())
                        != std::mem::discriminant(entry2.field_type())
                    {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// 重建索引
    pub fn rebuild_index<I>(&self, notes: I) -> Result<()>
    where
        I: IntoIterator<Item = (String, String, String, SystemTime, Vec<String>)>, // (path, title, content, mtime, tags)
    {
        info!("  ├─ 获取索引写入器...");

        // 获取写入器
        let mut index_writer: IndexWriter = self.index.writer(50_000_000)?; // 50MB 缓冲

        // 清空现有索引
        info!("  ├─ 清空旧索引...");
        index_writer.delete_all_documents()?;

        let mut count = 0;

        for (path, title, content, mtime, tags) in notes {
            // 转换 SystemTime 为 Unix 时间戳
            let mtime_secs = mtime
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            // 提取文件夹路径（去掉文件名）
            let folder = std::path::Path::new(&path)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string();

            // 构建文档
            let mut doc = doc!(
                self.path_field => path,
                self.title_field => title,
                self.content_field => content,
                self.mtime_field => mtime_secs,
                self.folder_field => folder,
            );

            // 添加标签（支持多个标签）
            for tag in tags {
                doc.add_text(self.tags_field, &tag);
            }

            index_writer.add_document(doc)?;
            count += 1;

            // 每 1000 条提交一次，避免内存占用过高
            if count % 1000 == 0 {
                index_writer.commit()?;
                info!("  ├─ 已索引 {} 条笔记", count);
            }
        }

        // 最终提交
        info!("  ├─ 提交索引...");
        index_writer.commit()?;
        // 手动刷新 IndexReader（Manual 模式下 commit 不自动触发刷新）。
        // 必须在 index_writer drop 之前或 drop 之后均可——writer 锁和 reader 刷新互不干扰。
        if let Err(e) = self.reader.reload() {
            warn!("  ⚠ IndexReader 刷新失败（搜索将返回旧结果直到下次同步）: {:?}", e);
        }
        info!("  └─ 索引重建完成，共 {} 条笔记", count);

        Ok(())
    }

    /// 增量更新搜索索引
    ///
    /// 仅删除 `deleted_paths` 对应的旧文档，并更新 `changed_notes` 对应的文档。
    /// 适用于增量同步场景，避免全量重建 Tantivy 索引的高昂代价。
    pub fn update_documents<I>(
        &self,
        changed_notes: I,
        deleted_paths: &[String],
    ) -> Result<()>
    where
        I: IntoIterator<Item = (String, String, String, SystemTime, Vec<String>)>,
    {
        let mut index_writer: IndexWriter = self.index.writer(50_000_000)?;

        // 删除 deleted 路径对应的文档
        for path in deleted_paths {
            let term = tantivy::Term::from_field_text(self.path_field, path);
            index_writer.delete_term(term);
        }

        let mut updated = 0usize;
        for (path, title, content, mtime, tags) in changed_notes {
            // 先删除该路径的旧文档（幂等）
            let term = tantivy::Term::from_field_text(self.path_field, &path);
            index_writer.delete_term(term);

            // 添加新文档
            let mtime_secs = mtime
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            let folder = std::path::Path::new(&path)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string();

            let mut doc = doc!(
                self.path_field => path,
                self.title_field => title,
                self.content_field => content,
                self.mtime_field => mtime_secs,
                self.folder_field => folder,
            );

            for tag in tags {
                doc.add_text(self.tags_field, &tag);
            }

            index_writer.add_document(doc)?;
            updated += 1;
        }

        index_writer.commit()?;
        // 手动刷新 IndexReader，与 rebuild_index 保持一致
        if let Err(e) = self.reader.reload() {
            warn!("  ⚠ IndexReader 刷新失败（搜索将返回旧结果直到下次同步）: {:?}", e);
        }
        info!(
            "  └─ 搜索索引增量更新完成：{} 条更新，{} 条删除",
            updated,
            deleted_paths.len()
        );

        Ok(())
    }

    /// 执行高级搜索（带过滤条件）
    #[allow(clippy::too_many_arguments)] // 搜索参数较多，引入参数结构体会增加样板代码，暂保持当前设计
    pub fn advanced_search(
        &self,
        query_str: &str,
        limit: usize,
        sort_by: SortBy,
        tags: Option<Vec<String>>, // 标签过滤（支持多个标签）
        folder: Option<String>,    // 文件夹过滤
        date_from: Option<i64>,    // 日期过滤：开始时间（Unix 时间戳）
        date_to: Option<i64>,      // 日期过滤：结束时间（Unix 时间戳）
    ) -> Result<Vec<SearchResult>> {
        // 直接复用缓存的 reader，避免每次搜索重复初始化
        let searcher = self.reader.searcher();

        // 创建查询解析器
        let query_parser =
            QueryParser::for_index(&self.index, vec![self.title_field, self.content_field]);

        // 解析文本查询
        let text_query = if !query_str.trim().is_empty() {
            match query_parser.parse_query(query_str) {
                Ok(q) => Some(q),
                Err(e) => {
                    warn!("查询解析失败: {}, 使用默认查询", e);
                    let escaped = query_str.replace(['\"', '\\'], "");
                    Some(query_parser.parse_query(&escaped)?)
                }
            }
        } else {
            None
        };

        // 构建组合查询（文本 + 过滤条件）
        use tantivy::query::{AllQuery, BooleanQuery, Occur, TermQuery};
        use tantivy::Term;

        let mut subqueries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

        // 添加文本查询
        if let Some(q) = text_query {
            subqueries.push((Occur::Must, q));
        } else {
            // 如果没有文本查询，匹配所有文档
            subqueries.push((Occur::Must, Box::new(AllQuery)));
        }

        // 添加标签过滤
        if let Some(tag_list) = tags
            && !tag_list.is_empty() {
                let mut tag_queries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();
                for tag in tag_list {
                    let term = Term::from_field_text(self.tags_field, &tag);
                    tag_queries.push((
                        Occur::Should,
                        Box::new(TermQuery::new(term, Default::default())),
                    ));
                }
                // 至少匹配一个标签
                subqueries.push((Occur::Must, Box::new(BooleanQuery::new(tag_queries))));
            }

        // 添加文件夹过滤
        if let Some(folder_path) = folder
            && !folder_path.is_empty() {
                let term = Term::from_field_text(self.folder_field, &folder_path);
                subqueries.push((
                    Occur::Must,
                    Box::new(TermQuery::new(term, Default::default())),
                ));
            }

        // 添加日期范围过滤
        if date_from.is_some() || date_to.is_some() {
            use std::ops::Bound;
            use tantivy::query::RangeQuery;
            use tantivy::Term;

            let lower_bound = match date_from {
                Some(ts) => Bound::Included(Term::from_field_i64(self.mtime_field, ts)),
                None => Bound::Unbounded,
            };

            let upper_bound = match date_to {
                Some(ts) => Bound::Included(Term::from_field_i64(self.mtime_field, ts)),
                None => Bound::Unbounded,
            };

            let range_query = RangeQuery::new(lower_bound, upper_bound);
            subqueries.push((Occur::Must, Box::new(range_query)));
        }

        let final_query = BooleanQuery::new(subqueries);

        // 执行搜索（tantivy 0.26：TopDocs 需通过 .order_by_score() 转为 Collector）
        let top_docs = searcher.search(&final_query, &TopDocs::with_limit(limit).order_by_score())?;

        let mut results = Vec::new();

        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;

            // 提取字段值
            let mut title = String::new();
            let mut path = String::new();
            let mut content = String::new();
            let mut mtime = 0i64;
            let mut tags = Vec::new();

            for (field, value) in doc.field_values() {
                if field == self.title_field {
                    if let Some(text) = value.as_str() {
                        title = text.to_string();
                    }
                } else if field == self.path_field {
                    if let Some(text) = value.as_str() {
                        path = text.to_string();
                    }
                } else if field == self.content_field {
                    if let Some(text) = value.as_str() {
                        content = text.to_string();
                    }
                } else if field == self.mtime_field {
                    if let Some(time) = value.as_i64() {
                        mtime = time;
                    }
                } else if field == self.tags_field {
                    // 标签字段支持多值，每个标签单独存储
                    if let Some(text) = value.as_str() {
                        tags.push(text.to_string());
                    }
                }
            }

            // 生成摘要片段
            let snippet = generate_snippet(&content, query_str, 150);

            results.push(SearchResult {
                title,
                path,
                snippet,
                score,
                mtime,
                tags,
            });
        }

        // 根据排序方式排序结果
        match sort_by {
            SortBy::Relevance => {
                // Tantivy 已经按相关度排序，无需额外操作
            }
            SortBy::Modified => {
                // 按修改时间降序排序（最新的在前）
                results.sort_by(|a, b| b.mtime.cmp(&a.mtime));
            }
        }

        Ok(results)
    }

    /// 强制刷新 IndexReader 缓存（测试/基准测试使用，确保 rebuild_index 后立即可搜索）
    pub fn reload_reader(&self) {
        self.reader.reload().expect("reload reader failed");
    }

    /// 返回当前 Tantivy 磁盘索引中的文档数量
    ///
    /// 用于判断索引是否已有内容，从而决定是否跳过全量重建。
    /// 返回 0 表示索引为空（需要重建）；> 0 表示索引有内容，可以直接复用。
    pub fn num_docs(&self) -> u64 {
        self.reader.searcher().num_docs()
    }

    /// 基于 FuzzyTermQuery 在标题字段做容错模糊搜索，返回建议列表。
    ///
    /// 对查询词中的每个 token 构造最多 1 个编辑距离的模糊查询（前缀匹配），
    /// 适合搜索框实时补全场景。返回 `(title, path, score)` 三元组，按相关度降序。
    pub fn fuzzy_suggest(
        &self,
        query_str: &str,
        limit: usize,
    ) -> Result<Vec<(String, String, f32)>> {
        use tantivy::query::{BooleanQuery, FuzzyTermQuery, Occur};
        use tantivy::Term;

        let q = query_str.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }

        let searcher = self.reader.searcher();

        // 对每个空格分隔的词构造 FuzzyTermQuery（编辑距离 1，前缀匹配）
        let tokens: Vec<&str> = q.split_whitespace().collect();
        let mut subqueries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

        for token in &tokens {
            let term = Term::from_field_text(self.title_field, &token.to_lowercase());
            // prefix=true：将查询词作为前缀，编辑距离 1
            let fuzzy_q = FuzzyTermQuery::new(term.clone(), 1, true);
            subqueries.push((Occur::Should, Box::new(fuzzy_q)));
            // 精确匹配优先级更高（额外 Should）
            let exact_term = Term::from_field_text(self.title_field, &token.to_lowercase());
            let exact_q = tantivy::query::TermQuery::new(exact_term, Default::default());
            subqueries.push((Occur::Should, Box::new(exact_q)));
        }

        if subqueries.is_empty() {
            return Ok(Vec::new());
        }

        let final_query = BooleanQuery::new(subqueries);
        let top_docs = searcher.search(&final_query, &TopDocs::with_limit(limit).order_by_score())?;

        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            let mut title = String::new();
            let mut path = String::new();
            for (field, value) in doc.field_values() {
                if field == self.title_field && let Some(t) = value.as_str() {
                    title = t.to_string();
                } else if field == self.path_field && let Some(p) = value.as_str() {
                    path = p.to_string();
                }
            }
            if !title.is_empty() {
                results.push((title, path, score));
            }
        }

        Ok(results)
    }

    /// 执行简单搜索（保持向后兼容）
    pub fn search(
        &self,
        query_str: &str,
        limit: usize,
        sort_by: SortBy,
    ) -> Result<Vec<SearchResult>> {
        // 调用高级搜索，不带过滤条件
        self.advanced_search(query_str, limit, sort_by, None, None, None, None)
    }
}

/// 大小写不敏感子串查找（M1 优化，v1.6.4）：避免分配 `haystack.to_lowercase()`。
///
/// `needle_lower` 必须已预先转换为小写。对 ASCII（字节大小写折叠）和
/// CJK（无大小写，直接字节比较）均正确工作。
/// 返回 haystack 中第一个匹配的**字节偏移**（保证在 UTF-8 字符边界）。
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

/// 生成搜索结果摘要片段，并用 `<mark>` 标签高亮命中词。
///
/// 在内容中定位搜索词，提取 50 字符前缀 + 命中词 + 100 字符后缀，
/// 并将所有匹配项包裹在 `<mark>...</mark>` 中供前端直接渲染为高亮效果。
fn generate_snippet(content: &str, search_term: &str, max_len: usize) -> String {
    // 清理内容：移除换行和多余空白
    let cleaned_content = content
        .replace(['\n', '\r'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // M1 优化：只分配 search_lower（小），用 find_substr_ci 避免分配 content_lower（大）
    let search_lower = search_term.to_lowercase();

    if !search_lower.is_empty()
        && let Some(pos) = find_substr_ci(&cleaned_content, &search_lower) {
        // 找到匹配位置，提取上下文窗口
        let start = pos.saturating_sub(50);
        let end = (pos + search_term.len() + 100).min(cleaned_content.len());
        let snippet = safe_substring(&cleaned_content, start, end);

        let prefix = if start > 0 { "..." } else { "" };
        let suffix = if end < cleaned_content.len() { "..." } else { "" };

        // 在摘要片段中高亮所有匹配项
        let highlighted = highlight_terms(&snippet, search_term);
        let result = format!("{}{}{}", prefix, highlighted, suffix);

        // 粗估 HTML 长度；字符限制适用于可见文本部分
        return result;
    }

    // 未找到匹配，返回开头部分（无高亮）
    if cleaned_content.is_empty() {
        return String::from("(无内容预览)");
    }

    let preview = safe_substring(&cleaned_content, 0, max_len);
    if cleaned_content.len() > max_len {
        format!("{}...", preview)
    } else {
        preview
    }
}

/// 在文本中将所有匹配 `term` 的位置包裹为 `<mark>...</mark>` 高亮标签。
///
/// 大小写不敏感匹配，保留原文大小写。
fn highlight_terms(text: &str, term: &str) -> String {
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
        // 确保在字符边界上
        if !text.is_char_boundary(abs_pos) {
            search_start = abs_pos + 1;
            continue;
        }
        let term_end = abs_pos + term_lower.len(); // 用 term_lower.len() 保持一致性
        if term_end > text.len() || !text.is_char_boundary(term_end) {
            search_start = abs_pos + 1;
            continue;
        }
        // 追加匹配前的文本
        result.push_str(&text[last_end..abs_pos]);
        // 包裹高亮标签
        result.push_str("<mark>");
        result.push_str(&text[abs_pos..term_end]);
        result.push_str("</mark>");
        last_end = term_end;
        search_start = term_end;
    }
    result.push_str(&text[last_end..]);
    result
}

// 安全地提取 UTF-8 字符串的子串，避免在字符边界中间切割
fn safe_substring(s: &str, start: usize, end: usize) -> String {
    let mut actual_start = start.min(s.len());
    let mut actual_end = end.min(s.len());

    // 确保 start 在字符边界上
    while actual_start > 0 && !s.is_char_boundary(actual_start) {
        actual_start -= 1;
    }

    // 确保 end 在字符边界上
    while actual_end < s.len() && !s.is_char_boundary(actual_end) {
        actual_end += 1;
    }

    s[actual_start..actual_end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};
    use tempfile::TempDir;

    /// 构造测试用搜索引擎（使用临时目录）
    fn make_engine() -> (SearchEngine, TempDir) {
        let dir = TempDir::new().unwrap();
        let engine = SearchEngine::new(dir.path()).unwrap();
        (engine, dir)
    }

    /// 构造一条测试文档数据：(path, title, content, mtime, tags)
    fn make_doc(
        path: &str,
        title: &str,
        content: &str,
        mtime_delta_secs: i64,
        tags: Vec<&str>,
    ) -> (String, String, String, SystemTime, Vec<String>) {
        let mtime = if mtime_delta_secs >= 0 {
            SystemTime::now() + Duration::from_secs(mtime_delta_secs as u64)
        } else {
            SystemTime::now() - Duration::from_secs((-mtime_delta_secs) as u64)
        };
        (
            path.to_string(),
            title.to_string(),
            content.to_string(),
            mtime,
            tags.into_iter().map(|s| s.to_string()).collect(),
        )
    }

    #[test]
    fn test_basic_search() {
        // 基本全文搜索：匹配标题和内容
        let (engine, _dir) = make_engine();
        engine
            .rebuild_index(vec![
                make_doc("a.md", "Rust 入门", "学习 Rust 编程语言", 0, vec![]),
                make_doc("b.md", "Python 教程", "Python 是一门动态语言", 0, vec![]),
            ])
            .unwrap();
        engine.reload_reader();

        let results = engine.search("Rust", 10, SortBy::Relevance).unwrap();
        assert!(!results.is_empty(), "应找到 Rust 相关笔记");
        assert!(
            results[0].title.contains("Rust"),
            "最相关的结果应包含 Rust"
        );
    }

    #[test]
    fn test_empty_index_returns_empty() {
        // 空索引搜索应返回空列表
        let (engine, _dir) = make_engine();
        let results = engine.search("anything", 10, SortBy::Relevance).unwrap();
        assert!(results.is_empty(), "空索引应返回空结果");
    }

    #[test]
    fn test_tag_filter() {
        // 标签过滤：只返回包含指定标签的笔记
        let (engine, _dir) = make_engine();
        engine
            .rebuild_index(vec![
                make_doc("a.md", "笔记 A", "内容 A", 0, vec!["rust", "系统"]),
                make_doc("b.md", "笔记 B", "内容 B", 0, vec!["python"]),
                make_doc("c.md", "笔记 C", "内容 C", 0, vec![]),
            ])
            .unwrap();
        engine.reload_reader();

        let results = engine
            .advanced_search("", 10, SortBy::Relevance, Some(vec!["rust".to_string()]), None, None, None)
            .unwrap();

        assert_eq!(results.len(), 1, "应只返回带 rust 标签的笔记");
        assert_eq!(results[0].title, "笔记 A");
    }

    #[test]
    fn test_folder_filter() {
        // 文件夹过滤：只返回指定文件夹下的笔记
        let (engine, _dir) = make_engine();
        engine
            .rebuild_index(vec![
                make_doc("notes/work/a.md", "工作笔记", "内容", 0, vec![]),
                make_doc("notes/personal/b.md", "个人笔记", "内容", 0, vec![]),
            ])
            .unwrap();
        engine.reload_reader();

        let results = engine
            .advanced_search("", 10, SortBy::Relevance, None, Some("notes/work".to_string()), None, None)
            .unwrap();

        assert_eq!(results.len(), 1, "应只返回 notes/work 文件夹下的笔记");
        assert_eq!(results[0].title, "工作笔记");
    }

    #[test]
    fn test_sort_by_modified() {
        // 按修改时间排序：最新的排在前
        let (engine, _dir) = make_engine();
        engine
            .rebuild_index(vec![
                make_doc("old.md", "旧笔记", "内容", -3600, vec![]),
                make_doc("new.md", "新笔记", "内容", 0, vec![]),
            ])
            .unwrap();
        engine.reload_reader();

        let results = engine.search("内容", 10, SortBy::Modified).unwrap();
        assert!(!results.is_empty(), "应有结果");
        // 时间戳较大（更新）的排在前面
        assert!(
            results[0].mtime >= results.last().unwrap().mtime,
            "最新笔记应排在前"
        );
    }

    #[test]
    fn test_num_docs() {
        // num_docs 应返回正确的文档数量
        let (engine, _dir) = make_engine();
        assert_eq!(engine.num_docs(), 0, "初始索引应为空");

        engine
            .rebuild_index(vec![
                make_doc("a.md", "A", "内容", 0, vec![]),
                make_doc("b.md", "B", "内容", 0, vec![]),
            ])
            .unwrap();
        engine.reload_reader();

        assert_eq!(engine.num_docs(), 2, "重建后应有 2 条文档");
    }

    #[test]
    fn test_schema_matches_type_check() {
        // schema_matches 应检测字段类型差异
        let (engine, _dir) = make_engine();
        let schema1 = engine.reader.searcher().index().schema();
        // 构建一个字段名相同但类型不同的 schema（用新 Schema::builder）
        let mut builder = tantivy::schema::Schema::builder();
        // 添加相同字段名但用 INT 类型替代 TEXT，触发不匹配
        builder.add_i64_field("title", tantivy::schema::INDEXED);
        let schema2 = builder.build();
        // schema1 有 TEXT title，schema2 有 I64 title → 不匹配
        assert!(
            !SearchEngine::schema_matches_pub(&schema1, &schema2),
            "字段类型不同时 schema_matches 应返回 false"
        );
    }
}
