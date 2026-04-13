// 侧边栏构建逻辑
use crate::domain::{FlatNode, Note, SidebarNode};
use std::collections::HashMap;

/// 从笔记集合构建侧边栏树形结构
pub fn build_sidebar(notes: &HashMap<String, Note>) -> Vec<SidebarNode> {
    let mut root_nodes: Vec<SidebarNode> = Vec::new();
    let mut keys: Vec<&String> = notes.keys().collect();
    keys.sort(); // 按字母顺序排序

    for path_str in keys {
        let parts: Vec<&str> = path_str.split('/').collect();
        let note = &notes[path_str];

        insert_into_tree(&mut root_nodes, parts.as_slice(), &note.title, path_str);
    }

    root_nodes
}

/// 递归插入节点到树形结构中
fn insert_into_tree(nodes: &mut Vec<SidebarNode>, parts: &[&str], title: &str, full_path: &str) {
    if parts.is_empty() {
        return;
    }

    let current = parts[0];
    let is_file = parts.len() == 1;

    if is_file {
        // 这是一个文件节点
        nodes.push(SidebarNode::new_file(
            title.to_string(),
            full_path.to_string(),
        ));
    } else {
        // 这是一个目录节点
        // 查找目录节点是否已存在
        let mut dir_idx = None;
        for (i, node) in nodes.iter().enumerate() {
            if node.name == current && node.path.is_none() {
                dir_idx = Some(i);
                break;
            }
        }

        if let Some(idx) = dir_idx {
            // 目录已存在，递归插入子节点
            insert_into_tree(&mut nodes[idx].children, &parts[1..], title, full_path);
        } else {
            // 创建新目录节点
            let mut new_dir = SidebarNode::new_dir(current.to_string());
            insert_into_tree(&mut new_dir.children, &parts[1..], title, full_path);
            nodes.push(new_dir);
        }
    }
}

/// 将树形结构扁平化为列表（用于模板渲染）
pub fn flatten_sidebar(nodes: &[SidebarNode]) -> Vec<FlatNode> {
    let mut flat = Vec::new();
    flatten_recursive(nodes, 0, &mut flat);
    flat
}

/// 递归扁平化树形结构
fn flatten_recursive(nodes: &[SidebarNode], depth: usize, out: &mut Vec<FlatNode>) {
    for node in nodes {
        out.push(FlatNode {
            name: node.name.clone(),
            path: node.path.clone(),
            depth,
        });
        flatten_recursive(&node.children, depth + 1, out);
    }
}

/// 在树形结构中查找第一个文件节点
pub fn find_first_file(nodes: &[SidebarNode]) -> Option<&SidebarNode> {
    for node in nodes {
        if node.path.is_some() {
            return Some(node);
        }
        if !node.children.is_empty() {
            if let Some(found) = find_first_file(&node.children) {
                return Some(found);
            }
        }
    }
    None
}
