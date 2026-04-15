// 图谱生成逻辑
use crate::domain::{GraphData, GraphEdge, GraphNode, Note};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::UNIX_EPOCH;

/// 从笔记的 mtime 获取 Unix 秒时间戳
fn note_mtime_secs(note: &Note) -> i64 {
    note.mtime.duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// 生成笔记的关系图谱数据
///
/// # 参数
/// * `current_note_title` - 当前笔记标题（图谱中心节点）
/// * `notes` - 所有笔记的映射
/// * `link_index` - 标题到路径的映射
/// * `depth` - 图谱深度（1-3 层）
pub fn generate_graph(
    current_note_title: &str,
    notes: &HashMap<String, Note>,
    link_index: &HashMap<String, String>,
    depth: usize,
) -> GraphData {
    let mut graph_nodes = HashMap::new();
    let mut graph_edges = Vec::new();
    let mut visited = HashSet::new();

    // 找到当前笔记的路径
    let current_path = match link_index.get(current_note_title) {
        Some(path) => path.clone(),
        None => {
            return GraphData {
                nodes: vec![],
                edges: vec![],
            }
        }
    };

    // 找到当前笔记
    let current_note = match notes.get(&current_path) {
        Some(note) => note,
        None => {
            return GraphData {
                nodes: vec![],
                edges: vec![],
            }
        }
    };

    // 添加中心节点（携带标签信息，用于前端节点颜色分组）
    graph_nodes.insert(
        current_path.clone(),
        GraphNode {
            id:    current_path.clone(),
            label: current_note_title.to_string(),
            title: current_note_title.to_string(),
            tags:  current_note.tags.clone(),
            mtime: note_mtime_secs(current_note),
        },
    );
    visited.insert(current_path.clone());

    // 广度优先搜索构建图谱
    let mut queue = VecDeque::new();
    queue.push_back((current_path.clone(), 0));

    while let Some((note_path, current_depth)) = queue.pop_front() {
        if current_depth >= depth {
            continue;
        }

        let note = match notes.get(&note_path) {
            Some(n) => n,
            None => continue,
        };

        // 提取笔记中的所有链接
        let linked_titles = extract_links_from_note(note);

        for linked_title in linked_titles {
            if let Some(linked_path) = link_index.get(&linked_title) {
                // 添加节点（如果还未添加）
                if !graph_nodes.contains_key(linked_path) {
                    let linked_tags = notes
                        .get(linked_path)
                        .map(|n| n.tags.clone())
                        .unwrap_or_default();
                    graph_nodes.insert(
                        linked_path.clone(),
                        GraphNode {
                            id: linked_path.clone(),
                            label: linked_title.clone(),
                            title: linked_title.clone(),
                            tags:  linked_tags,
                            mtime: notes.get(linked_path).map(note_mtime_secs).unwrap_or(0),
                        },
                    );
                }

                // 添加边
                graph_edges.push(GraphEdge {
                    from: note_path.clone(),
                    to: linked_path.clone(),
                });

                // 将未访问的节点加入队列
                if !visited.contains(linked_path) {
                    visited.insert(linked_path.clone());
                    queue.push_back((linked_path.clone(), current_depth + 1));
                }
            }
        }
    }

    // 收集所有链接到当前笔记的反向链接
    for (path, note) in notes {
        if visited.contains(path) {
            continue; // 已经处理过的节点
        }

        let linked_titles = extract_links_from_note(note);
        if linked_titles.contains(current_note_title) {
            // 添加反向链接节点（携带标签）
            graph_nodes.insert(
                path.clone(),
                GraphNode {
                    id:    path.clone(),
                    label: note.title.clone(),
                    title: note.title.clone(),
                    tags:  note.tags.clone(),
                    mtime: note_mtime_secs(note),
                },
            );

            // 添加反向链接边
            graph_edges.push(GraphEdge {
                from: path.clone(),
                to: current_path.clone(),
            });
        }
    }

    let nodes: Vec<GraphNode> = graph_nodes.into_values().collect();

    GraphData {
        nodes,
        edges: graph_edges,
    }
}

/// 生成全库关系图谱数据
///
/// 包含笔记库中所有笔记及其链接关系。
/// 当节点数超过 500 时自动降采样，仅保留有至少一条链接的笔记。
///
/// # 参数
/// * `notes` - 所有笔记的映射
/// * `link_index` - 标题到路径的映射（用于快速查找目标笔记路径）
/// * `hide_isolated` - 是否隐藏孤立节点（无入链也无出链）
pub fn generate_global_graph(
    notes: &HashMap<String, Note>,
    link_index: &HashMap<String, String>,
    hide_isolated: bool,
) -> GraphData {
    const MAX_NODES: usize = 500;

    // v1.8.6：预计算全部笔记的 mtime 秒，避免在节点构建热路径重复调用 duration_since()。
    // 在遍历所有笔记（第二遍构建节点时）直接从此 map 读取，节省逐节点的时间戳转换开销。
    let mtime_map: HashMap<&str, i64> = notes.iter()
        .map(|(path, note)| (path.as_str(), note_mtime_secs(note)))
        .collect();

    let mut graph_nodes: HashMap<String, GraphNode> = HashMap::new();
    let mut graph_edges: Vec<GraphEdge> = Vec::new();
    let mut connected: HashSet<String> = HashSet::new();

    // 第一遍：构建所有边，并标记有连接的节点
    for note in notes.values() {
        for linked_title in &note.outgoing_links {
            if let Some(linked_path) = link_index.get(linked_title)
                && notes.contains_key(linked_path)
            {
                connected.insert(note.path.clone());
                connected.insert(linked_path.clone());
                graph_edges.push(GraphEdge {
                    from: note.path.clone(),
                    to: linked_path.clone(),
                });
            }
        }
    }

    // 确定是否需要降采样（节点超 500 时只保留有链接的节点）
    let total = notes.len();
    let should_downsample = total > MAX_NODES;

    // 第二遍：构建节点列表
    for note in notes.values() {
        let is_isolated = !connected.contains(&note.path);

        // 需要隐藏孤立节点，或降采样时跳过孤立节点
        if (hide_isolated || should_downsample) && is_isolated {
            continue;
        }

        graph_nodes.insert(
            note.path.clone(),
            GraphNode {
                id:    note.path.clone(),
                label: note.title.clone(),
                title: note.title.clone(),
                tags:  note.tags.clone(),
                // v1.8.6：从预计算 mtime_map 读取，避免重复调用 duration_since()
                mtime: mtime_map.get(note.path.as_str()).copied().unwrap_or(0),
            },
        );
    }

    // 过滤掉引用了已被移除节点的边
    let edges = graph_edges
        .into_iter()
        .filter(|e| graph_nodes.contains_key(&e.from) && graph_nodes.contains_key(&e.to))
        .collect();

    GraphData {
        nodes: graph_nodes.into_values().collect(),
        edges,
    }
}

/// 从笔记中提取所有出链标题
///
/// 直接使用 Note.outgoing_links 字段（构建期已预计算），
/// 无需重新解析 content_text，消除热路径上的正则表达式开销。
fn extract_links_from_note(note: &Note) -> HashSet<String> {
    note.outgoing_links.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Frontmatter, TocItem};
    use std::collections::HashMap;
    use std::time::SystemTime;

    /// 构造测试用 Note
    fn make_note(title: &str, outgoing_links: Vec<&str>) -> Note {
        Note {
            path: format!("{}.md", title),
            title: title.to_string(),
            content_html: String::new(),
            backlinks: Vec::new(),
            tags: Vec::new(),
            toc: Vec::<TocItem>::new(),
            mtime: SystemTime::UNIX_EPOCH,
            frontmatter: Frontmatter(serde_yaml::Value::Null),
            outgoing_links: outgoing_links.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    /// 构建笔记集合和链接索引
    fn build_notes_and_index(
        specs: &[(&str, Vec<&str>)],
    ) -> (HashMap<String, Note>, HashMap<String, String>) {
        let mut notes = HashMap::new();
        let mut link_index = HashMap::new();
        for (title, links) in specs {
            let note = make_note(title, links.clone());
            link_index.insert(title.to_string(), note.path.clone());
            notes.insert(note.path.clone(), note);
        }
        (notes, link_index)
    }

    #[test]
    fn test_graph_nonexistent_note_returns_empty() {
        // 中心笔记不存在时应返回空图谱
        let (notes, link_index) = build_notes_and_index(&[("A", vec![])]);
        let graph = generate_graph("不存在", &notes, &link_index, 2);
        assert!(graph.nodes.is_empty(), "不存在的笔记应返回空节点");
        assert!(graph.edges.is_empty(), "不存在的笔记应返回空边");
    }

    #[test]
    fn test_graph_depth_1_includes_only_direct_links() {
        // A→B→C，深度 1：图谱应包含 A 和 B，不包含 C
        let (notes, link_index) =
            build_notes_and_index(&[("A", vec!["B"]), ("B", vec!["C"]), ("C", vec![])]);

        let graph = generate_graph("A", &notes, &link_index, 1);
        let node_labels: Vec<&str> = graph.nodes.iter().map(|n| n.label.as_str()).collect();

        assert!(node_labels.contains(&"A"), "中心节点 A 应在图谱中");
        assert!(node_labels.contains(&"B"), "直接链接 B 应在图谱中（深度 1）");
        assert!(!node_labels.contains(&"C"), "间接链接 C 不应在深度 1 的图谱中");
    }

    #[test]
    fn test_graph_depth_2_includes_two_hops() {
        // A→B→C，深度 2：图谱应包含 A、B、C
        let (notes, link_index) =
            build_notes_and_index(&[("A", vec!["B"]), ("B", vec!["C"]), ("C", vec![])]);

        let graph = generate_graph("A", &notes, &link_index, 2);
        let node_labels: Vec<&str> = graph.nodes.iter().map(|n| n.label.as_str()).collect();

        assert!(node_labels.contains(&"A"), "中心节点 A 应在图谱中");
        assert!(node_labels.contains(&"B"), "一跳节点 B 应在图谱中");
        assert!(node_labels.contains(&"C"), "二跳节点 C 应在深度 2 的图谱中");
    }

    #[test]
    fn test_graph_backlinks_included() {
        // D→A（反向链接），A 的图谱应包含 D
        let (notes, link_index) = build_notes_and_index(&[
            ("A", vec![]),
            ("D", vec!["A"]), // D 链接到 A（D 是 A 的反向链接）
        ]);

        let graph = generate_graph("A", &notes, &link_index, 1);
        let node_labels: Vec<&str> = graph.nodes.iter().map(|n| n.label.as_str()).collect();

        assert!(node_labels.contains(&"D"), "反向链接节点 D 应出现在图谱中");
        // 应有 D→A 方向的边
        let has_edge = graph.edges.iter().any(|e| e.from.contains("D") && e.to.contains("A"));
        assert!(has_edge, "应有从 D 到 A 的反向链接边");
    }

    #[test]
    fn test_graph_isolated_node_not_included() {
        // E 与 A 无任何链接关系，不应出现在 A 的图谱中
        let (notes, link_index) = build_notes_and_index(&[
            ("A", vec!["B"]),
            ("B", vec![]),
            ("E", vec![]), // 孤立节点
        ]);

        let graph = generate_graph("A", &notes, &link_index, 2);
        let node_labels: Vec<&str> = graph.nodes.iter().map(|n| n.label.as_str()).collect();

        assert!(!node_labels.contains(&"E"), "孤立节点 E 不应出现在 A 的图谱中");
    }

    #[test]
    fn test_graph_isolated_center_has_only_self() {
        // 中心笔记没有出链，且没有人链接到它 → 图谱只有中心节点
        let (notes, link_index) = build_notes_and_index(&[("Solo", vec![])]);

        let graph = generate_graph("Solo", &notes, &link_index, 2);

        assert_eq!(graph.nodes.len(), 1, "孤立中心笔记图谱应只有 1 个节点");
        assert!(graph.edges.is_empty(), "孤立中心笔记图谱不应有边");
    }

    #[test]
    fn test_global_graph_includes_all_notes() {
        // 全局图谱应包含所有笔记（不隐藏孤立节点时）
        let (notes, link_index) =
            build_notes_and_index(&[("A", vec!["B"]), ("B", vec![]), ("Solo", vec![])]);

        let graph = generate_global_graph(&notes, &link_index, false);
        let labels: Vec<&str> = graph.nodes.iter().map(|n| n.label.as_str()).collect();

        assert!(labels.contains(&"A"), "A 应在全局图谱中");
        assert!(labels.contains(&"B"), "B 应在全局图谱中");
        assert!(labels.contains(&"Solo"), "孤立节点 Solo 应在全局图谱中（未隐藏孤立节点）");
    }

    #[test]
    fn test_global_graph_hide_isolated() {
        // hide_isolated=true 时，孤立节点不应出现
        let (notes, link_index) =
            build_notes_and_index(&[("A", vec!["B"]), ("B", vec![]), ("Solo", vec![])]);

        let graph = generate_global_graph(&notes, &link_index, true);
        let labels: Vec<&str> = graph.nodes.iter().map(|n| n.label.as_str()).collect();

        assert!(labels.contains(&"A"), "A 应在图谱中（有出链）");
        assert!(labels.contains(&"B"), "B 应在图谱中（有入链）");
        assert!(!labels.contains(&"Solo"), "孤立节点 Solo 应被隐藏");
    }

    #[test]
    fn test_global_graph_contains_edges() {
        // 全局图谱应包含所有链接边
        let (notes, link_index) =
            build_notes_and_index(&[("A", vec!["B"]), ("B", vec!["C"]), ("C", vec![])]);

        let graph = generate_global_graph(&notes, &link_index, false);
        assert!(graph.edges.len() >= 2, "全局图谱应包含 A→B 和 B→C 的边");
    }

    #[test]
    fn test_graph_node_carries_tags() {
        // 图谱节点应携带标签信息
        let mut notes = HashMap::new();
        let mut link_index = HashMap::new();
        let mut note_a = make_note("A", vec!["B"]);
        note_a.tags = vec!["rust".to_string()];
        let note_b = make_note("B", vec![]);
        link_index.insert("A".to_string(), note_a.path.clone());
        link_index.insert("B".to_string(), note_b.path.clone());
        notes.insert(note_a.path.clone(), note_a);
        notes.insert(note_b.path.clone(), note_b);

        let graph = generate_global_graph(&notes, &link_index, false);
        let a_node = graph.nodes.iter().find(|n| n.label == "A");
        assert!(a_node.is_some(), "A 节点应存在");
        assert!(
            a_node.unwrap().tags.contains(&"rust".to_string()),
            "A 节点应携带 rust 标签"
        );
    }
}
