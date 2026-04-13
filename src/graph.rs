// 图谱生成逻辑
use crate::domain::{GraphData, GraphEdge, GraphNode, Note};
use std::collections::{HashMap, HashSet, VecDeque};

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
    let _current_note = match notes.get(&current_path) {
        Some(note) => note,
        None => {
            return GraphData {
                nodes: vec![],
                edges: vec![],
            }
        }
    };

    // 添加中心节点
    graph_nodes.insert(
        current_path.clone(),
        GraphNode {
            id: current_path.clone(),
            label: current_note_title.to_string(),
            title: current_note_title.to_string(),
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
                    graph_nodes.insert(
                        linked_path.clone(),
                        GraphNode {
                            id: linked_path.clone(),
                            label: linked_title.clone(),
                            title: linked_title.clone(),
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
            // 添加反向链接节点
            graph_nodes.insert(
                path.clone(),
                GraphNode {
                    id: path.clone(),
                    label: note.title.clone(),
                    title: note.title.clone(),
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

/// 从笔记内容中提取所有链接的标题
fn extract_links_from_note(note: &Note) -> HashSet<String> {
    let mut links = HashSet::new();

    // 使用正则表达式提取 WikiLinks
    let re = regex::Regex::new(r"\[\[([^\]|]+)(?:\|[^\]]+)?\]\]").unwrap();

    // 从 HTML 内容中提取（因为 content 是已经转换的 HTML）
    // 我们需要从原始文本中提取，但 Note 结构中只有 content_text
    for cap in re.captures_iter(&note.content_text) {
        if let Some(target) = cap.get(1) {
            let target_str = target.as_str().trim().to_string();
            if !target_str.is_empty() {
                links.insert(target_str);
            }
        }
    }

    links
}
