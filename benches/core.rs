//! obsidian_mirror 核心性能基准测试
//!
//! 运行方式：
//!   cargo bench                    # 运行全部基准，生成 HTML 报告至 target/criterion/
//!   cargo bench -- markdown        # 只运行名称含 "markdown" 的基准
//!   cargo bench -- search          # 只运行搜索相关基准
//!   cargo bench -- wasm            # 只运行 WASM 函数基准（v1.6.3 新增）
//!
//! 说明：
//!   - 每个 group 代表一个性能关注点
//!   - BenchmarkId 用于对比不同规模的性能差异
//!   - black_box 防止编译器优化掉未使用的计算结果
//!   - wasm 组：直接调用 WASM crate 的原生目标版本（与浏览器端行为一致）

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use std::collections::HashMap;
use std::time::SystemTime;

use obsidian_mirror::domain::{Frontmatter, Note, TocItem};
use obsidian_mirror::indexer::{BacklinkBuilder, TagIndexBuilder};
use obsidian_mirror::markdown::MarkdownProcessor;
use obsidian_mirror::search_engine::{SearchEngine, SortBy};

// v1.6.3 新增：WASM crate 原生目标直接引用
// （在 bench_wasm 函数内部导入，仅在该 group 使用）

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
        frontmatter: Frontmatter(serde_yaml::Value::Null),
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
// Group 6: WASM 函数性能（v1.6.3 新增）
// ==========================================
//
// 说明：
//   - WASM crate 以原生 x86-64 目标编译运行，无 JS 引擎开销
//   - 实际浏览器端性能因 WASM JIT 略有差异（通常 ±20%）
//   - 主要用于回归检测和跨版本对比

/// 构造 N 条笔记的 JSON（用于 NoteIndex 加载和搜索基准）
fn make_index_json(n: usize) -> String {
    let entries: Vec<String> = (0..n)
        .map(|i| {
            let tags = if i % 3 == 0 { r#"["rust","编程"]"# }
                       else if i % 3 == 1 { r#"["python","数据"]"# }
                       else { r#"["web","前端"]"# };
            format!(
                r#"{{"title":"笔记标题{}","path":"folder/note{}.md","tags":{},"content":"这是第{}条笔记的内容，包含 Rust 编程语言的技术讨论，涉及内存安全和所有权系统。Note content {} discussing performance.","mtime":{}}}"#,
                i, i, tags, i, i, i as i64
            )
        })
        .collect();
    format!("[{}]", entries.join(","))
}

/// 构造图谱节点 JSON
fn make_graph_nodes(n: usize) -> String {
    let nodes: Vec<String> = (0..n).map(|i| format!(r#"{{"id":"node{}"}}"#, i)).collect();
    format!("[{}]", nodes.join(","))
}

/// 构造图谱边 JSON（每个节点连向后续 2 个节点，形成稀疏图）
fn make_graph_edges(n: usize) -> String {
    let edges: Vec<String> = (0..n)
        .flat_map(|i| {
            vec![
                format!(r#"{{"from":"node{}","to":"node{}"}}"#, i, (i + 1) % n),
                format!(r#"{{"from":"node{}","to":"node{}"}}"#, i, (i + 2) % n),
            ]
        })
        .collect();
    format!("[{}]", edges.join(","))
}

fn bench_wasm(c: &mut Criterion) {
    use obsidian_mirror_wasm::{
        compute_graph_layout, filter_notes, generate_toc_from_html,
        render_markdown, NoteIndex,
    };

    let mut group = c.benchmark_group("wasm");
    group.sample_size(50);

    // ─── 6-a: NoteIndex 加载（index.json 反序列化 + 倒排索引构建）──────────
    for n in [100, 500, 1000].iter() {
        let json = make_index_json(*n);
        group.bench_with_input(
            BenchmarkId::new("note_index_load", n),
            &json,
            |b, json| b.iter(|| NoteIndex::load_json(black_box(json)).unwrap()),
        );
    }

    // ─── 6-b: NoteIndex 搜索（ASCII 查询词）─────────────────────────────────
    {
        let json = make_index_json(1000);
        let idx = NoteIndex::load_json(&json).unwrap();
        group.bench_function("note_index_search_ascii_1000", |b| {
            b.iter(|| idx.search_json(black_box("rust programming"), 20))
        });
    }

    // ─── 6-c: NoteIndex 搜索（CJK 查询词，n-gram 分词路径）──────────────────
    {
        let json = make_index_json(1000);
        let idx = NoteIndex::load_json(&json).unwrap();
        group.bench_function("note_index_search_cjk_1000", |b| {
            b.iter(|| idx.search_json(black_box("编程语言"), 20))
        });
    }

    // ─── 6-d: filterNotes（多标签 + 路径前缀过滤）───────────────────────────
    for n in [100, 1000].iter() {
        let json = make_index_json(*n);
        group.bench_with_input(
            BenchmarkId::new("filter_notes_tag", n),
            &json,
            |b, json| b.iter(|| filter_notes(black_box(json), black_box("rust"), black_box(""), 50)),
        );
    }

    // ─── 6-e: compute_graph_layout（Fruchterman-Reingold 布局）──────────────
    for n in [50, 100, 200, 500].iter() {
        let nodes = make_graph_nodes(*n);
        let edges = make_graph_edges(*n);
        let iterations: u32 = if *n > 300 { 15 } else if *n > 100 { 30 } else { 50 };
        group.bench_with_input(
            BenchmarkId::new("graph_layout", n),
            &(nodes, edges, iterations),
            |b, (nodes, edges, iters)| {
                b.iter(|| compute_graph_layout(black_box(nodes), black_box(edges), *iters))
            },
        );
    }

    // ─── 6-f: render_markdown（Markdown → HTML，含 Obsidian 扩展语法）────────
    let md_simple = "# 标题\n\n段落内容，包含 **加粗** 和 *斜体*。\n";
    group.bench_function("render_markdown_simple", |b| {
        b.iter(|| render_markdown(black_box(md_simple)))
    });

    let md_with_wikilinks = "# 项目笔记\n\n这是关于 [[Rust 编程]] 的笔记，参见 [[项目概述|概述]]。\n\n==高亮文字== 和 $E = mc^2$\n\n#技术 #后端\n";
    group.bench_function("render_markdown_wikilinks", |b| {
        b.iter(|| render_markdown(black_box(md_with_wikilinks)))
    });

    let md_complex = "---\ntitle: 复杂笔记\n---\n# 主标题\n\n## 数学公式\n\n$$\\int_0^\\infty e^{-x}\\,dx = 1$$\n\n行内 $E=mc^2$\n\n## WikiLink\n\n参见 [[相关笔记]] 和 [[文档|别名]]。\n\n==高亮==\n\n> [!NOTE]\n> Callout 块。\n\n#tag1 #tag2\n";
    group.bench_function("render_markdown_complex", |b| {
        b.iter(|| render_markdown(black_box(md_complex)))
    });

    // ─── 6-g: generate_toc_from_html（从渲染 HTML 提取目录）──────────────────
    let html_with_headings = (1..=100)
        .map(|i| format!("<h{} id=\"h-{}\">标题 {}</h{}>", (i % 6) + 1, i, i, (i % 6) + 1))
        .collect::<Vec<_>>()
        .join("\n");
    group.bench_function("generate_toc_100_headings", |b| {
        b.iter(|| generate_toc_from_html(black_box(&html_with_headings)))
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
    bench_wasm,
);
criterion_main!(benches);
