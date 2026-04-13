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
pub enum SortBy {
    Relevance, // 按相关度排序（默认）
    Modified,  // 按修改时间排序
}

impl Default for SortBy {
    fn default() -> Self {
        SortBy::Relevance
    }
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

        // 初始化并缓存 IndexReader，后续搜索直接复用，避免重复创建
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
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
        info!(
            "  └─ 搜索索引增量更新完成：{} 条更新，{} 条删除",
            updated,
            deleted_paths.len()
        );

        Ok(())
    }

    /// 执行高级搜索（带过滤条件）
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
        if let Some(tag_list) = tags {
            if !tag_list.is_empty() {
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
        }

        // 添加文件夹过滤
        if let Some(folder_path) = folder {
            if !folder_path.is_empty() {
                let term = Term::from_field_text(self.folder_field, &folder_path);
                subqueries.push((
                    Occur::Must,
                    Box::new(TermQuery::new(term, Default::default())),
                ));
            }
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

    /// 返回当前 Tantivy 磁盘索引中的文档数量
    ///
    /// 用于判断索引是否已有内容，从而决定是否跳过全量重建。
    /// 返回 0 表示索引为空（需要重建）；> 0 表示索引有内容，可以直接复用。
    pub fn num_docs(&self) -> u64 {
        self.reader.searcher().num_docs()
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

// 生成搜索结果摘要片段
fn generate_snippet(content: &str, search_term: &str, max_len: usize) -> String {
    // 清理内容：移除换行和多余空白
    let cleaned_content = content
        .replace('\n', " ")
        .replace('\r', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let content_lower = cleaned_content.to_lowercase();
    let search_lower = search_term.to_lowercase();

    if let Some(pos) = content_lower.find(&search_lower) {
        // 找到匹配位置，提取上下文
        let start = pos.saturating_sub(50);
        let end = (pos + search_term.len() + 100).min(cleaned_content.len());

        // 安全地提取子串（处理 UTF-8 边界）
        let snippet = safe_substring(&cleaned_content, start, end);

        let prefix = if start > 0 { "..." } else { "" };
        let suffix = if end < cleaned_content.len() {
            "..."
        } else {
            ""
        };

        let result = format!("{}{}{}", prefix, snippet, suffix);

        // 限制总长度
        if result.len() > max_len {
            safe_substring(&result, 0, max_len) + "..."
        } else {
            result
        }
    } else {
        // 如果找不到匹配，返回开头部分
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
