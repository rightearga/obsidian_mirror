// 索引构建模块
// 负责构建和管理各种索引：链接索引、反向链接、文件索引、搜索索引

use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::info;
use walkdir::WalkDir;

use crate::domain::Note;

/// 处理后的笔记数据（包含路径、笔记对象和链接）
pub type ProcessedNote = (String, Note, Vec<String>);

/// 反向链接构建器
pub struct BacklinkBuilder;

impl BacklinkBuilder {
    /// 构建反向链接：目标笔记标题 -> 链接到它的笔记标题列表
    ///
    /// 基于全量 notes 的 outgoing_links 字段重建，不依赖增量 temp_links，
    /// 确保增量同步时未变更笔记的反向链接不会丢失。
    pub fn build(notes: &HashMap<String, Note>) -> HashMap<String, Vec<String>> {
        info!("  ├─ 构建反向链接...");
        let mut backlinks: HashMap<String, Vec<String>> = HashMap::new();

        for note in notes.values() {
            for target in &note.outgoing_links {
                backlinks
                    .entry(target.clone())
                    .or_default()
                    .push(note.title.clone());
            }
        }

        info!(
            "  ├─ 反向链接构建完成，共 {} 个笔记有反向链接",
            backlinks.len()
        );
        backlinks
    }
}

/// 文件索引构建器（用于图片、PDF 等资源文件）
pub struct FileIndexBuilder;

impl FileIndexBuilder {
    /// 构建文件索引：文件名 -> 完整相对路径
    pub fn build(local_path: &Path) -> HashMap<String, String> {
        info!("  ├─ 扫描资源文件...");
        let mut file_index = HashMap::new();

        for entry in WalkDir::new(local_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            // 只索引非 Markdown 文件
            if path.is_file() && path.extension().is_none_or(|ext| ext != "md")
                && let Some(filename) = path.file_name() {
                    let filename_str = filename.to_string_lossy().to_string();
                    let relative_path = pathdiff::diff_paths(path, local_path)
                        .unwrap_or(path.to_path_buf())
                        .to_string_lossy()
                        .replace("\\", "/");
                    file_index.insert(filename_str, relative_path);
                }
        }

        info!("  └─ 资源文件索引构建完成，共 {} 个文件", file_index.len());
        file_index
    }
}

/// 索引更新器
pub struct IndexUpdater;

impl IndexUpdater {
    /// 增量更新笔记和链接索引
    ///
    /// 只更新/添加本次处理的笔记，不删除未处理的笔记。
    /// 删除操作由 sync.rs 中的 files_to_delete 单独处理。
    /// 反向链接由 BacklinkBuilder::build 基于全量 notes.outgoing_links 重建。
    pub fn update_notes_and_links(
        notes_map: &mut HashMap<String, Note>,
        link_index: &mut HashMap<String, String>,
        processed_notes: Vec<ProcessedNote>,
    ) {
        // 只清理需要更新的笔记的旧 link_index 条目
        let updating_paths: HashSet<String> = processed_notes
            .iter()
            .map(|(path, _, _)| path.clone())
            .collect();

        // 删除旧的 link_index 条目（通过值查找键）
        link_index.retain(|_, path| !updating_paths.contains(path));

        // 插入或更新笔记
        for (relative_path, note, _links) in processed_notes {
            notes_map.insert(relative_path.clone(), note.clone());
            link_index.insert(note.title, relative_path);
        }
    }
}

/// 标签索引构建器
pub struct TagIndexBuilder;

impl TagIndexBuilder {
    /// 构建标签索引：标签 -> 包含该标签的笔记列表（按标题）
    pub fn build(notes: &HashMap<String, Note>) -> HashMap<String, Vec<String>> {
        info!("  ├─ 构建标签索引...");
        let mut tag_index: HashMap<String, Vec<String>> = HashMap::new();

        for note in notes.values() {
            for tag in &note.tags {
                tag_index
                    .entry(tag.clone())
                    .or_default()
                    .push(note.title.clone());
            }
        }

        // 对每个标签的笔记列表按标题排序
        for note_list in tag_index.values_mut() {
            note_list.sort();
        }

        info!("  ├─ 标签索引构建完成，共 {} 个标签", tag_index.len());
        tag_index
    }
}

/// 搜索索引数据提取器
pub struct SearchIndexDataExtractor;

impl SearchIndexDataExtractor {
    /// 从笔记集合中提取搜索索引所需的数据
    pub fn extract(
        notes: &HashMap<String, Note>,
    ) -> Vec<(String, String, String, std::time::SystemTime, Vec<String>)> {
        notes
            .iter()
            .map(|(path, note)| {
                (
                    path.clone(),
                    note.title.clone(),
                    note.content_text.clone(),
                    note.mtime,
                    note.tags.clone(), // 包含标签信息
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Frontmatter, TocItem};
    use std::time::SystemTime;

    /// 构造测试用 Note
    fn make_note(title: &str, outgoing_links: Vec<&str>) -> Note {
        Note {
            path: format!("{}.md", title),
            title: title.to_string(),
            content_html: String::new(),
            content_text: String::new(),
            backlinks: Vec::new(),
            tags: Vec::new(),
            toc: Vec::<TocItem>::new(),
            mtime: SystemTime::UNIX_EPOCH,
            frontmatter: Frontmatter(serde_yml::Value::Null),
            outgoing_links: outgoing_links.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_backlink_builder_full_sync() {
        // 全量同步：A->B, B->C，反向链接应正确构建
        let mut notes = HashMap::new();
        notes.insert("A.md".to_string(), make_note("A", vec!["B"]));
        notes.insert("B.md".to_string(), make_note("B", vec!["C"]));
        notes.insert("C.md".to_string(), make_note("C", vec![]));

        let backlinks = BacklinkBuilder::build(&notes);

        assert_eq!(backlinks.get("B").map(|v| v.as_slice()), Some(&["A".to_string()][..]));
        assert_eq!(backlinks.get("C").map(|v| v.as_slice()), Some(&["B".to_string()][..]));
        assert!(backlinks.get("A").is_none(), "A 没有反向链接");
    }

    #[test]
    fn test_backlink_builder_incremental_sync_no_loss() {
        // 增量同步场景验证：A 更新（A->B 改为 A->C），B 不变（B->C）
        // BacklinkBuilder::build 基于全量 outgoing_links 重建，B 对 C 的反向链接不应丢失
        let mut notes = HashMap::new();
        notes.insert("A.md".to_string(), make_note("A", vec!["C"])); // A 已更新
        notes.insert("B.md".to_string(), make_note("B", vec!["C"])); // B 未变，outgoing_links 来自上次处理
        notes.insert("C.md".to_string(), make_note("C", vec![]));

        let backlinks = BacklinkBuilder::build(&notes);

        // C 应同时有来自 A 和 B 的反向链接
        let c_backlinks = backlinks.get("C").expect("C 应有反向链接");
        assert!(c_backlinks.contains(&"A".to_string()), "C 应有来自 A 的反向链接");
        assert!(c_backlinks.contains(&"B".to_string()), "增量同步后 C 仍应保留来自 B 的反向链接");
        // B 不再有来自 A 的反向链接
        assert!(backlinks.get("B").is_none(), "A 已不再链接到 B，B 的反向链接应为空");
    }

    #[test]
    fn test_backlink_builder_empty_notes() {
        // 空笔记集合不应报错
        let notes = HashMap::new();
        let backlinks = BacklinkBuilder::build(&notes);
        assert!(backlinks.is_empty());
    }

    #[test]
    fn test_tag_index_builder_basic() {
        // TagIndexBuilder 基本功能验证
        let mut notes = HashMap::new();
        let mut note_a = make_note("A", vec![]);
        note_a.tags = vec!["rust".to_string(), "系统".to_string()];
        let mut note_b = make_note("B", vec![]);
        note_b.tags = vec!["rust".to_string()];
        notes.insert("A.md".to_string(), note_a);
        notes.insert("B.md".to_string(), note_b);

        let tag_index = TagIndexBuilder::build(&notes);

        let rust_notes = tag_index.get("rust").expect("应有 rust 标签");
        assert_eq!(rust_notes.len(), 2, "rust 标签应有 2 篇笔记");
        assert!(tag_index.contains_key("系统"), "系统标签应存在");
    }
}
