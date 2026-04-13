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
    pub fn build(
        notes: &HashMap<String, Note>,
        temp_links: HashMap<String, Vec<String>>,
    ) -> HashMap<String, Vec<String>> {
        info!("  ├─ 构建反向链接...");
        let mut backlinks = HashMap::new();

        for (source_path, targets) in temp_links {
            let source_title = notes
                .get(&source_path)
                .map(|n| n.title.clone())
                .unwrap_or_default();

            for target in targets {
                backlinks
                    .entry(target)
                    .or_insert_with(Vec::new)
                    .push(source_title.clone());
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
            if path.is_file() && !path.extension().map_or(false, |ext| ext == "md") {
                if let Some(filename) = path.file_name() {
                    let filename_str = filename.to_string_lossy().to_string();
                    let relative_path = pathdiff::diff_paths(&path, local_path)
                        .unwrap_or(path.to_path_buf())
                        .to_string_lossy()
                        .replace("\\", "/");
                    file_index.insert(filename_str, relative_path);
                }
            }
        }

        info!("  └─ 资源文件索引构建完成，共 {} 个文件", file_index.len());
        file_index
    }
}

/// 索引更新器
pub struct IndexUpdater;

impl IndexUpdater {
    /// 更新所有索引（notes、link_index、backlinks）
    ///
    /// 返回临时链接映射，用于后续构建反向链接
    pub fn update_notes_and_links(
        notes_map: &mut HashMap<String, Note>,
        link_index: &mut HashMap<String, String>,
        processed_notes: Vec<ProcessedNote>,
    ) -> HashMap<String, Vec<String>> {
        // ✅ 增量更新：只更新/添加新处理的笔记，不删除未处理的笔记
        // 注意：删除操作在 sync.rs 中的 files_to_delete 单独处理

        // 只清理需要更新的笔记的旧 link_index 条目
        let updating_paths: HashSet<String> = processed_notes
            .iter()
            .map(|(path, _, _)| path.clone())
            .collect();

        // 删除旧的 link_index 条目（通过值查找键）
        link_index.retain(|_, path| !updating_paths.contains(path));

        // 临时存储链接关系
        let mut temp_links: HashMap<String, Vec<String>> = HashMap::new();

        // 插入或更新笔记
        for (relative_path, note, links) in processed_notes {
            notes_map.insert(relative_path.clone(), note.clone());
            link_index.insert(note.title, relative_path.clone());
            if !links.is_empty() {
                temp_links.insert(relative_path, links);
            }
        }

        temp_links
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
                    .or_insert_with(Vec::new)
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
