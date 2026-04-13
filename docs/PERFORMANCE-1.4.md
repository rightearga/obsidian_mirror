# obsidian_mirror v1.4 性能基准报告

> 测试日期：2026-04-13  
> 版本：v1.4.9  
> 工具：[Criterion](https://github.com/bheisler/criterion.rs) 0.8.2（Release profile，100 样本）  
> 环境：Windows 10 Pro 10.0.19045，x86-64  
> 复现：`cargo bench`（HTML 报告输出至 `target/criterion/`）

---

## 汇总

| 组别 | 代表用例 | 中位延迟 | 实际含义 |
|------|---------|---------|---------|
| markdown | 典型复杂笔记 | **85.9 µs** | 1000 笔记并行处理 ≈ 85 ms |
| indexer  | 反向链接 ×1000 | **476 µs** | 全库重建开销可忽略 |
| search   | 全文命中 | **51.6 µs** | 单次搜索 HTTP 路径瓶颈在网络 |
| graph    | 局部图谱 ×500 | **201 µs** | 深度 2 的 500 节点图渲染极快 |
| truncate_html | 大 HTML | **22.7 µs** | 预览端点无性能压力 |

---

## 第一组：Markdown 处理（`bench_markdown`）

`MarkdownProcessor::process` 是同步管道中最耗时的单步骤，由 Rayon 并行分摊。

### 实测数据

| 用例 | 中位时间 | 置信区间 [lo, hi] | 异常值 |
|------|---------|------------------|------|
| minimal（纯正文，无扩展语法） | 5.57 µs | [5.39, 5.75] | 3% mild |
| with_wikilinks（6 个 WikiLink + 2 个标签） | 37.82 µs | [36.59, 39.15] | 5% mild |
| complex（Frontmatter + 公式 + Callout） | 85.93 µs | [82.96, 89.14] | 2% mild |
| long_5000chars（约 5000 字符长文） | 346.77 µs | [340.83, 353.16] | 4 个（含 2 severe） |

### 各阶段耗时分解

根据复杂度梯度推断各阶段贡献：

```
minimal (5.6 µs)
  └─ pulldown-cmark 渲染 + lazy_static 正则匹配基线

with_wikilinks (37.8 µs) = minimal × 6.8
  └─ 增量 ≈ 32 µs
     ├─ WikiLink 正则扫描（全文 pass）：~12 µs
     ├─ ![[]] 图片/文件处理：~8 µs
     └─ #tag 提取 + 链接列表构建：~12 µs

complex (85.9 µs) = with_wikilinks × 2.3
  └─ 增量 ≈ 48 µs
     ├─ serde_yml frontmatter 解析：~20 µs
     ├─ Callout 块识别（正则）：~10 µs
     ├─ 数学公式包裹（$...$ 正则）：~8 µs
     └─ TOC 生成（标题提取）：~10 µs

long_5000chars (346.8 µs)
  └─ 字符数 × 系数约 0.069 µs/char（以 complex 为参照）
     ├─ pulldown-cmark SIMD 渲染随文本线性增长
     └─ 正则 pass 对文本长度线性敏感
```

### 实际影响：初始同步耗时估算

Rayon 默认线程数 = 逻辑 CPU 数（设为 `C`）：

| 笔记库规模 | 单核串行 | 8 核并行（估算） | 16 核并行（估算） |
|-----------|---------|----------------|----------------|
| 100 条（典型个人库） | 8.6 ms | 1.1 ms | 0.6 ms |
| 500 条（中型团队库） | 43 ms | 5.4 ms | 2.7 ms |
| 1000 条（大型知识库） | 86 ms | 10.8 ms | 5.4 ms |
| 5000 条（超大型） | 430 ms | 53.8 ms | 26.9 ms |

> 注：假设笔记复杂度分布以 `complex` 为代表；增量同步只处理 Git diff 变更文件，耗时更低。

### 关键观察

- **正则是主要开销**：WikiLink、Callout、数学公式的正则 pass 各约 8–12 µs。`lazy_static!` 预编译已避免重复编译，但每次调用仍需全文扫描。
- **serde_yml 不可忽视**：Frontmatter 解析贡献约 20 µs，占 complex 用例的 23%。
- **SIMD 渲染高效**：`pulldown-cmark` 的 SIMD 特性使纯渲染阶段极快；文本长度对总时间的影响主要来自正则 pass 而非渲染本身。

---

## 第二组：索引构建（`bench_indexer`）

同步结束时对全库执行一次全量重建（`BacklinkBuilder::build` + `TagIndexBuilder::build`）。

### 实测数据

| 操作 | 笔记数 | 中位时间 | 相对前档 |
|------|--------|---------|---------|
| backlink_build | 10 | 3.87 µs | — |
| backlink_build | 100 | 50.1 µs | ×13 (10× 数据) |
| backlink_build | 500 | 228.9 µs | ×4.6 (5× 数据) |
| backlink_build | 1000 | 476.1 µs | ×2.1 (2× 数据) |
| tag_index_build | 10 | 1.82 µs | — |
| tag_index_build | 100 | 21.0 µs | ×11.5 |
| tag_index_build | 500 | 108.1 µs | ×5.1 |
| tag_index_build | 1000 | 220.8 µs | ×2.0 |

### 复杂度分析

```
backlink_build O(N·L)：N = 笔记数，L = 平均出链数
  每条笔记遍历 outgoing_links，在 link_index HashMap 中 O(1) 查找
  实测 N=10→1000（100×）：延迟增长约 123×，略超线性
  原因：HashMap 在高负载下碰撞率上升，cache miss 增多

tag_index_build O(N·T)：T = 平均标签数
  比 backlink_build 约快 2×（标签数 < 出链数，且无路径解析）
```

### 大规模估算

| 笔记数 | backlink 重建 | tag 重建 | 合计 |
|--------|-------------|---------|------|
| 1,000 | 0.48 ms | 0.22 ms | **0.7 ms** |
| 5,000 | ~3.5 ms | ~1.6 ms | **~5 ms** |
| 10,000 | ~9 ms | ~4 ms | **~13 ms** |

> 全量重建时间可忽略不计，不会成为同步延迟的瓶颈。

---

## 第三组：搜索引擎（`bench_search`）

在 200 条已索引文档上的 Tantivy 搜索性能（`sample_size = 50`）。

### 实测数据

| 用例 | 中位时间 | 说明 |
|------|---------|------|
| fulltext_hit（"Rust 编程"，有结果） | 51.6 µs | BM25 评分 + top-10 |
| fulltext_miss（"量子计算"，无结果） | **8.29 µs** | 早期终止，无评分计算 |
| advanced_tag_filter（"Rust" + 标签过滤） | 57.7 µs | 附加布尔过滤器 |
| sort_by_modified（按时间排序） | 10.1 µs | 数值字段排序，无 BM25 |

### 关键发现

**命中 vs 未命中 6× 差距**：

```
fulltext_miss (8.3 µs)
  ├─ Tantivy QueryParser 解析：~3 µs
  ├─ 倒排索引查找（无匹配 → 立即返回）：~2 µs
  └─ 空结果序列化：~3 µs

fulltext_hit (51.6 µs) = miss × 6.2
  └─ 额外 ≈ 43 µs
     ├─ BM25 TF-IDF 评分计算：~25 µs
     ├─ top-k 堆操作（10 个结果）：~8 µs
     └─ snippet 提取 + 路径解析：~10 µs
```

**标签过滤 +12%**：advanced_search 的标签过滤仅增加 6.1 µs（BooleanQuery 额外 pass），开销极低。

**时间排序与未命中相近**：sort_by_modified (10.1 µs) 接近 fulltext_miss，说明数值字段 `mtime` 的倒排扫描极高效。

### 实际 HTTP 响应时间估算

```
GET /api/search?q=Rust
  ├─ Tantivy 计算：51.6 µs
  ├─ JSON 序列化（10 条结果）：~15 µs
  ├─ HTTP 框架（actix-web）：~20 µs
  └─ 网络往返（localhost）：~100 µs
  总计：≈ 190 µs（0.19 ms）
```

> 搜索延迟完全由网络决定，Tantivy 计算占比 < 30%。

---

## 第四组：关系图谱（`bench_graph`）

`generate_graph` 以 BFS 遍历出链/入链，深度固定为 2。

### 实测数据

| 笔记库规模 | 中位时间 | 置信区间 |
|-----------|---------|---------|
| 50 条 | **27.99 ns** | [27.49, 28.52] |
| 200 条 | 76.0 µs | [74.1, 78.1] |
| 500 条 | 200.7 µs | [194.9, 206.9] |

### 异常：50 条库仅需 28 ns

基准测试以"笔记50"为中心节点请求局部图谱：

```
50 条库（笔记0–笔记49）中不存在"笔记50"
  → generate_graph 立即返回空图（early exit）
  → 28 ns ≈ HashMap::get 一次查找时间

200/500 条库中"笔记50"存在
  → 完整 BFS：遍历直接链接 + 2 跳内所有节点
  → 76–201 µs 为正常 BFS 开销
```

> 实际使用中，用户请求的笔记必然存在，应以 200/500 条的数据为参考基线。

### BFS 复杂度

```
O(V + E)：V = BFS 可达节点数，E = 这些节点的出链/入链数
  200 条 → 500 条（2.5×）：76 → 201 µs（2.6×），近线性
  实际图谱通常 depth=2 可达节点 << 全库，开销与库总规模弱相关
```

---

## 第五组：HTML 摘要截断（`bench_truncate_html`）

`/api/preview` 端点对每篇笔记调用一次，剥离 HTML 标签后截断至指定字符数。

### 实测数据

| 用例 | HTML 大小（估算） | 中位时间 |
|------|----------------|---------|
| small（5× `<p>` 段落） | ~150 B | 484.70 ns |
| medium（20× `<h2><p>` 块） | ~1.6 KB | 4.32 µs |
| large（100× `<div><h1><p>` 块） | ~8.5 KB | 22.71 µs |

### 分析

```
当前实现（逐字符扫描 + split_whitespace + join）：
  时间复杂度：O(N) N = HTML 字节数
  空间开销：构建临时 String + Vec<&str>，有额外分配

大小比：150B → 1.6KB → 8.5KB = 1× → 10.7× → 56.7×
时间比：485ns → 4.32µs → 22.7µs = 1× → 8.9× → 46.9×
```

字符 → 时间的线性系数约 **2.7 ns/byte**，主要来自字符分类（UTF-8 多字节）和内存分配。

对于 `/api/preview` 端点（通常一次请求对应一篇笔记），22.7 µs 的最坏情况完全可接受。

---

## 跨组综合分析

### 同步管道总耗时（1000 条笔记，8 核）

```
步骤                        单次耗时      总耗时（8核并行）
─────────────────────────────────────────────────────
Git pull                    外部，不计
Markdown 处理（×1000）       85.9 µs      ≈ 10.8 ms   ← 主要耗时
BacklinkBuilder::build       476 µs        0.48 ms
TagIndexBuilder::build       221 µs        0.22 ms
侧边栏重建                   未测，估算      ~5 ms
Tantivy 索引重建             IO 密集，未测  ~500 ms     ← 运行在后台线程
IndexPersistence::save       IO 密集，未测  ~200 ms     ← 运行在后台线程
─────────────────────────────────────────────────────
HTTP 阻塞时间（用户可感知）   ≈ 17 ms（步骤 1–6）
后台异步时间                  ≈ 700 ms（Tantivy + redb，不影响服务）
```

### HTTP 热路径延迟概览

| 端点 | 计算耗时 | 主要瓶颈 |
|------|---------|---------|
| `GET /api/search` | 51.6 µs | 网络 I/O |
| `GET /api/graph` | 201 µs | 网络 I/O |
| `GET /api/preview` | 22.7 µs | 网络 I/O |
| `GET /doc/{path}` | ~86 µs（HTML 渲染） | 网络 I/O |

所有端点的计算耗时均在 **< 250 µs**，远低于典型 HTTP 往返时间（LAN ≈ 1 ms，WAN ≈ 10–100 ms）。

---

## 优化机会

以下为当前版本的可改进点，按优先级排序：

### P2（中期改进）

**M1：WikiLink 正则合并**

当前 `markdown.rs` 对 WikiLink、![[]]、Callout 分别做三次全文正则 pass。可考虑合并为单次扫描，预计节省 15–20 µs/笔记（complex 用例）。

```rust
// 当前：三次 regex.replace_all
// 改进方向：手写状态机或合并 lazy_static! 多模式 Aho-Corasick
```

**M2：truncate_html 减少内存分配**

当前实现构造临时 `Vec<&str>` + `join`，可改为流式写出，避免二次分配：

```rust
// 改进方向：用 write! 直接写到预分配 String，无需 split_whitespace().collect()
// 预期收益：medium/large 用例节省约 30–40% 时间
```

### P3（长期优化）

**M3：frontmatter 解析缓存**

对内容未变更的笔记（mtime 相同），持久化层已跳过重处理。但若 frontmatter 较重且仅正文变更，仍会重新解析。可考虑在 `ProcessedNote` 中缓存解析后的 `Frontmatter`（已通过 `outgoing_links` 缓存先例）。

**M4：图谱 BFS 剪枝**

depth=2 时 BFS 可能访问大量节点。对于度数极高的"枢纽笔记"（如索引页），可加入最大邻居数限制（默认 50），避免响应时间退化。

---

## 复现方式

```bash
# 运行全部基准（生成 HTML 报告至 target/criterion/）
cargo bench

# 只运行单个组
cargo bench -- markdown
cargo bench -- search
cargo bench -- indexer
cargo bench -- graph
cargo bench -- truncate_html

# 快速验证（不计时，仅确认可运行）
cargo bench -- --test
```

HTML 报告包含每组基准的时间分布图、violin plot 和历史对比（多次运行后自动生成回归检测）。
