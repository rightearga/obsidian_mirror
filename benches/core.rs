//! obsidian_mirror 核心性能基准测试
//!
//! 运行方式：
//!   cargo bench                    # 运行全部基准，生成 HTML 报告至 target/criterion/
//!   cargo bench -- markdown        # 只运行名称含 "markdown" 的基准
//!   cargo bench -- search          # 只运行搜索相关基准
//!
//! 说明：
//!   - 每个 group 代表一个性能关注点
//!   - BenchmarkId 用于对比不同规模的性能差异
//!   - black_box 防止编译器优化掉未使用的计算结果

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use std::collections::HashMap;
use std::time::SystemTime;

use obsidian_mirror::domain::{Frontmatter, Note, TocItem};
use obsidian_mirror::indexer::{BacklinkBuilder, TagIndexBuilder};
use obsidian_mirror::markdown::MarkdownProcessor;
use obsidian_mirror::search_engine::{SearchEngine, SortBy};

// ==========================================
// 辅助函数：构造测试数据
// ==========================================

/// 构造测试用 Note（不含 content_text，v1.4.9+）
fn make_note(title: &str, outgoing_links: Vec<&str>, tags: Vec<&str>) -> Note {
    Note {
        path: format!("{}.md", title),
        title: title.to_string(),
        content_html: format!("<h1>{}</h1><p>测试内容</p>", title),
        backlinks: Vec::new(),
        tags: tags.into_iter().map(|s| s.to_string()).collect(),
        toc: Vec::<TocItem>::new(),
        mtime: SystemTime::UNIX_EPOCH,
        frontmatter: Frontmatter(serde_yml::Value::Null),
        outgoing_links: outgoing_links.into_iter().map(|s| s.to_string()).collect(),
    }
}

/// 构造 N 条笔记的笔记库（模拟真实拓扑：每条笔记有 3 条出链）
fn make_notes(n: usize) -> HashMap<String, Note> {
    let mut notes = HashMap::new();
    for i in 0..n {
        let title = format!("笔记{}", i);
        let links: Vec<&str> = Vec::new(); // 使用 outgoing_links
        let tags = if i % 5 == 0 { vec!["rust"] } else { vec!["general"] };
        let mut note = make_note(&title, links, tags);
        // 每条笔记链接到前 3 条（模拟链接拓扑）
        note.outgoing_links = (0..3.min(i))
            .map(|j| format!("笔记{}", j))
            .collect();
        notes.insert(note.path.clone(), note);
    }
    notes
}

// ==========================================
// Group 1: Markdown 处理性能
// ==========================================

fn bench_markdown(c: &mut Criterion) {
    let mut group = c.benchmark_group("markdown");

    // 1-a: 极简笔记（最快情形基线）
    let minimal = "# 标题\n\n正文内容。\n";
    group.bench_function("minimal", |b| {
        b.iter(|| MarkdownProcessor::process(black_box(minimal)))
    });

    // 1-b: 含 WikiLink 的中等复杂度笔记
    let with_links = r#"# 项目笔记

这是一个关于 [[Rust 编程]] 的笔记，参见 [[项目概述|概述文档]]。

主要内容：
- 使用 [[actix-web]] 构建 HTTP 服务
- [[Tantivy]] 提供全文搜索
- 通过 [[Git]] 自动同步笔记库

#技术 #后端

相关笔记：[[架构设计]] | [[部署指南]]
"#;
    group.bench_function("with_wikilinks", |b| {
        b.iter(|| MarkdownProcessor::process(black_box(with_links)))
    });

    // 1-c: 含 Frontmatter + 数学公式 + Callout 的复杂笔记
    let complex = r#"---
title: 复杂笔记
tags: [rust, performance, 测试]
date: 2026-04-13
---
# 主标题

## 数学公式

行内公式 $E = mc^2$，块级公式：

$$\int_0^\infty e^{-x}\,dx = 1$$

## 代码示例

```rust
fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}
```

## 标注块

> [!NOTE] 重要提示
> 这是一个 Callout 块，支持 20+ 种类型。

> [!WARNING]- 可折叠警告
> 内容默认收起。

## WikiLinks 和高亮

参见 [[相关笔记]] 和 [[另一文档|别名链接]]。

==高亮内容== 和 ![[附图.png]]

#tag1 #tag2 #中文标签
"#;
    group.bench_function("complex", |b| {
        b.iter(|| MarkdownProcessor::process(black_box(complex)))
    });

    // 1-d: 超长笔记（5000 字符，模拟大型文档）
    let long_content = format!(
        "# 长文档\n\n{}\n\n{}\n",
        "这是一段测试内容，包含 [[链接]] 和 #标签。\n".repeat(80),
        "**重要段落**：详细描述了性能测试的意义。\n".repeat(40),
    );
    group.bench_function("long_5000chars", |b| {
        b.iter(|| MarkdownProcessor::process(black_box(&long_content)))
    });

    group.finish();
}

// ==========================================
// Group 2: 索引构建性能（BacklinkBuilder + TagIndexBuilder）
// ==========================================

fn bench_indexer(c: &mut Criterion) {
    let mut group = c.benchmark_group("indexer");

    for n in [10, 100, 500, 1000].iter() {
        let notes = make_notes(*n);

        // 2-a: 反向链接构建（全量重建，每次同步都调用）
        group.bench_with_input(
            BenchmarkId::new("backlink_build", n),
            &notes,
            |b, notes| {
                b.iter(|| BacklinkBuilder::build(black_box(notes)))
            },
        );

        // 2-b: 标签索引构建
        group.bench_with_input(
            BenchmarkId::new("tag_index_build", n),
            &notes,
            |b, notes| {
                b.iter(|| TagIndexBuilder::build(black_box(notes)))
            },
        );
    }

    group.finish();
}

// ==========================================
// Group 3: 搜索引擎性能
// ==========================================

fn bench_search(c: &mut Criterion) {
    use tempfile::TempDir;

    // 初始化搜索引擎并索引 200 条文档
    let dir = TempDir::new().unwrap();
    let engine = SearchEngine::new(dir.path()).unwrap();

    let docs: Vec<(String, String, String, SystemTime, Vec<String>)> = (0..200)
        .map(|i| {
            let path = format!("notes/note{}.md", i);
            let title = format!("笔记标题 {}", i);
            let content = format!(
                "这是第 {} 条笔记的内容，包含 Rust 编程相关的技术讨论。\
                 涉及内存安全、所有权系统和性能优化。",
                i
            );
            let mtime = SystemTime::now();
            let tags = if i % 3 == 0 { vec!["rust".to_string()] } else { vec![] };
            (path, title, content, mtime, tags)
        })
        .collect();

    engine.rebuild_index(docs.into_iter()).unwrap();
    engine.reload_reader();

    let mut group = c.benchmark_group("search");
    group.sample_size(50); // 搜索基准样本数较小（防止索引 warming 影响）

    // 3-a: 简单全文搜索（有匹配结果）
    group.bench_function("fulltext_hit", |b| {
        b.iter(|| {
            engine
                .search(black_box("Rust 编程"), 10, SortBy::Relevance)
                .unwrap()
        })
    });

    // 3-b: 简单全文搜索（无匹配结果）
    group.bench_function("fulltext_miss", |b| {
        b.iter(|| {
            engine
                .search(black_box("量子计算"), 10, SortBy::Relevance)
                .unwrap()
        })
    });

    // 3-c: 带标签过滤的高级搜索
    group.bench_function("advanced_tag_filter", |b| {
        b.iter(|| {
            engine
                .advanced_search(
                    black_box("Rust"),
                    10,
                    SortBy::Relevance,
                    Some(vec!["rust".to_string()]),
                    None,
                    None,
                    None,
                )
                .unwrap()
        })
    });

    // 3-d: 按修改时间排序
    group.bench_function("sort_by_modified", |b| {
        b.iter(|| {
            engine
                .search(black_box("笔记"), 10, SortBy::Modified)
                .unwrap()
        })
    });

    group.finish();
}

// ==========================================
// Group 4: 图谱生成性能
// ==========================================

fn bench_graph(c: &mut Criterion) {
    use obsidian_mirror::graph::generate_graph;

    let mut group = c.benchmark_group("graph");

    for n in [50, 200, 500].iter() {
        let notes = make_notes(*n);
        let mut link_index = HashMap::new();
        for (path, note) in &notes {
            link_index.insert(note.title.clone(), path.clone());
        }

        // 4-a: 局部图谱（中心节点 BFS，深度 2）
        group.bench_with_input(
            BenchmarkId::new("local_graph_depth2", n),
            &(&notes, &link_index),
            |b, (notes, link_index)| {
                b.iter(|| {
                    generate_graph(
                        black_box("笔记50"),
                        black_box(notes),
                        black_box(link_index),
                        2,
                    )
                })
            },
        );
    }

    group.finish();
}

// ==========================================
// Group 5: HTML 截断性能（预览功能热路径）
// ==========================================

fn bench_truncate_html(c: &mut Criterion) {
    // 构造不同大小的 HTML 内容
    let small_html = "<p>短内容。</p>".repeat(5);
    let medium_html = "<h2>章节</h2><p>中等长度内容，含有 <strong>加粗</strong> 和 <em>斜体</em>。</p>".repeat(20);
    let large_html = "<div class=\"content\"><h1>标题</h1><p>长篇内容，模拟大型笔记页面的 HTML 输出。</p></div>".repeat(100);

    // 直接使用 truncate_html 等效逻辑进行基准测试
    // （handlers.rs 中的同名函数，提取为本地版本以便测试）
    fn truncate_html_bench(html: &str, max_chars: usize) -> String {
        let mut text = String::with_capacity(html.len());
        let mut in_tag = false;
        for c in html.chars() {
            match c {
                '<' => in_tag = true,
                '>' => { in_tag = false; text.push(' '); }
                _ if !in_tag => text.push(c),
                _ => {}
            }
        }
        let text: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if text.chars().count() <= max_chars {
            return text;
        }
        format!("{}...", text.chars().take(max_chars).collect::<String>())
    }

    let mut group = c.benchmark_group("truncate_html");

    group.bench_function("small_200chars", |b| {
        b.iter(|| truncate_html_bench(black_box(&small_html), 200))
    });
    group.bench_function("medium_500chars", |b| {
        b.iter(|| truncate_html_bench(black_box(&medium_html), 500))
    });
    group.bench_function("large_500chars", |b| {
        b.iter(|| truncate_html_bench(black_box(&large_html), 500))
    });

    group.finish();
}

// ==========================================
// 注册并运行所有 benchmark group
// ==========================================

criterion_group!(
    benches,
    bench_markdown,
    bench_indexer,
    bench_search,
    bench_graph,
    bench_truncate_html,
);
criterion_main!(benches);
