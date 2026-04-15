# obsidian_mirror v1.9.x 代码审查报告

**审查日期：** 2026-04-15  
**审查版本：** v1.9.0（commit `b37e8cb`）  
**审查范围：** v1.9.0 引入的全部新增/修改文件（`src/domain.rs`、`src/graph.rs`、`crates/wasm/src/lib.rs`、`templates/graph_page.html`）  
**严重级别：** 🔴 P0 / 🟠 P1 / 🟡 P2 / 🔵 P3 / ⚪ Info  
**状态标记：** ✅ 已修复（本次）/ 🔜 → 版本 / ⏸ 设计性限制，已知接受

---

## 总体评价

**结论：v1.9.0 图谱影响力功能代码质量高；PageRank 算法实现正确（零溢出风险、收敛性良好）；边权重计算逻辑正确；发现 1 项 P3 文档问题，本次修复。**

| 维度 | 评分 | 说明 |
|------|------|------|
| 架构设计   | ★★★★★ | 服务端计算 + WASM 客户端双路径设计合理；复用已有图谱基础设施 |
| 异步正确性 | ★★★★★ | 无新增异步代码；PageRank 为纯函数，无状态 |
| 安全性     | ★★★★★ | GraphNode.pagerank 为计算结果，无用户输入注入风险 |
| 错误处理   | ★★★★★ | `od.max(1.0)` 防止除零；`n==0` 提前返回；边界完整 |
| 测试覆盖   | ★★★★★ | 4 个 PageRank 测试覆盖链式/孤立/中心节点/空图场景 |
| 代码质量   | ★★★★☆ | WASM 版本注释过期（Q1 修复后升至★5）|

---

## 一、代码质量（Code Quality）

### 🔵 Q1 - WASM 模块版本注释未随 v1.9.0 更新 ✅ 已修复（本次）

**文件：** `crates/wasm/src/lib.rs:1`  
**严重性：** P3

```rust
//! obsidian_mirror WebAssembly 模块（v1.8.6）  // ← 应为 v1.9.0
```

v1.9.0 新增了 `compute_pagerank()` 函数（重要新功能），模块顶部注释应同步更新。

---

## 二、技术验证（v1.9.0 审计重点）

### ✅ PageRank 数值溢出与收敛性

**文件：** `src/graph.rs:21–66`

```rust
let init = 1.0_f32 / n as f32;  // 均匀初始化
let base = (1.0 - damping) / n as f32;  // 阻尼基础分配
let od = *out_deg.get(src).unwrap_or(&1) as f32;
rank += damping * scores.get(src)... / od.max(1.0);  // 防止除零
```

- **初始化**：`1/n` 均匀分配，总和为 1.0，无溢出风险 ✅
- **悬空节点**（无出链）：`base` 保证所有节点每轮至少获得 `(1-d)/n` 的分数，不会出现分数流失至 0 ✅
- **除零防护**：`od.max(1.0)` 确保出度为 0 的节点不触发除零 ✅
- **收敛性**：20 轮迭代 + 阻尼 0.85 对典型笔记库（≤5000 节点）可靠收敛（标准值，多数图在 10–15 轮内达稳态）✅
- **精度**：`f32` 精度（约 7 位有效数字）对归一化分数完全足够 ✅

### ✅ 边权重计算正确性

**文件：** `src/graph.rs:258–267`

```rust
let edge_set: HashSet<(String, String)> = graph_edges.iter()
    .map(|e| (e.from.clone(), e.to.clone()))
    .collect();
for edge in &mut graph_edges {
    if edge_set.contains(&(edge.to.clone(), edge.from.clone())) {
        edge.weight = 2;
    }
}
```

- 逻辑正确：遍历所有边，检查反向边是否存在，若存在则 weight=2 ✅
- 借用安全：先构建 HashSet（不可变借用），再遍历修改（可变借用），避免同时借用 ✅
- 注：内循环中 `edge.to.clone()` 和 `edge.from.clone()` 各分配一个 String 用于 HashSet 查找，对典型图谱规模（<1000条边）无实际性能影响 ⚪ Info

### ✅ GraphNode.pagerank 字段安全性

**文件：** `src/domain.rs`

- `pagerank: f32` 完全由服务端 BFS 图结构计算，不来自任何用户输入 ✅
- `#[serde(default)]` 保证反序列化时旧客户端不破坏 ✅
- 序列化为 JSON 时为普通浮点数，无 XSS/注入风险 ✅

### ✅ WASM compute_pagerank 边界处理

**文件：** `crates/wasm/src/lib.rs`

```rust
let iters = iterations.max(1).min(100) as usize;  // 迭代次数限制
if n == 0 { return "{}".to_string(); }             // 空图提前返回
let key = nd.id.replace('"', "\\\"");              // JSON key 转义
```

- 迭代次数 clamp(1, 100) 防止极端输入 ✅
- 空图返回空对象 ✅
- 路径中双引号转义（Obsidian 路径通常不含双引号，但有兜底）✅

---

## 三、信息性观察（Info）

### ⚪ I1 - 边权重 HashSet 查找时 String 克隆

**文件：** `src/graph.rs:263`  
**严重性：** Info

```rust
if edge_set.contains(&(edge.to.clone(), edge.from.clone())) {
```

每条边执行两次 `clone()`。对典型笔记库（边数 <5000）总分配量约 2×5000×平均路径长度 ≈ 可忽略。接受为已知设计。

### ⚪ I2 - graph.rs PageRank 用 f32，WASM 用 f64

**文件：** `src/graph.rs:25`，`crates/wasm/src/lib.rs`  
**严重性：** Info

两处实现精度不同（服务端 f32，WASM f64），两者对 0–1 归一化分数均足够精确，客户端与服务端结果会有微小浮点差异，不影响功能。接受为设计选择。

---

## 修复状态汇总

| 编号 | 问题 | 级别 | 状态 | 修复版本 |
|------|------|------|------|---------|
| Q1 | WASM 模块版本注释未更新 | 🔵 P3 | ✅ 已修复（本次）| v1.9.1 |
| I1 | 边权重 HashSet String 克隆 | ⚪ Info | ⏸ 典型规模无影响 | — |
| I2 | PageRank f32 vs f64 精度差异 | ⚪ Info | ⏸ 均足够精确 | — |

**修复统计（本次审计 v1.9.1）**：  
已修复 **1 项**（Q1）/ 接受为设计限制 **2 项**

---

## 进入 v1.9.2 前必须解决的问题

无 P0/P1 问题。v1.9.x 系列可继续推进。
