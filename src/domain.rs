use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::time::SystemTime;

/// Frontmatter 包装类型，用于支持 postcard 序列化
#[derive(Debug, Clone)]
pub struct Frontmatter(pub serde_yml::Value);

impl Serialize for Frontmatter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 将 serde_yml::Value 转换为 JSON 字符串
        let json_str = serde_json::to_string(&self.0).unwrap_or_else(|_| "null".to_string());
        serializer.serialize_str(&json_str)
    }
}

impl<'de> Deserialize<'de> for Frontmatter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // 从 JSON 字符串反序列化为 serde_yml::Value
        let json_str = String::deserialize(deserializer)?;
        let value: serde_yml::Value =
            serde_json::from_str(&json_str).map_err(serde::de::Error::custom)?;
        Ok(Frontmatter(value))
    }
}

/// 目录项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocItem {
    pub level: u8,    // 标题级别 (1-6)
    pub text: String, // 标题文本
    pub id: String,   // 用于锚点的 ID
}

/// 面包屑导航项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreadcrumbItem {
    pub name: String,         // 显示名称
    pub path: Option<String>, // 路径（最后一项为 None，表示当前页面）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub path: String, // Relative path string (e.g., "folder/note.md")
    pub title: String,
    pub content_html: String,
    // content_text 已在 v1.4.9 移除：原始 Markdown 文本不再驻留内存。
    // 搜索索引由 Tantivy 磁盘索引维持，全量/增量同步时直接从文件读取内容传给引擎。
    pub backlinks: Vec<String>,
    pub tags: Vec<String>, // 标签列表
    pub toc: Vec<TocItem>, // 目录列表
    pub mtime: SystemTime,
    pub frontmatter: Frontmatter, // Frontmatter 数据
    /// 当前笔记的出链（指向的其他笔记标题列表），用于构建全量反向链接索引
    #[serde(default)]
    pub outgoing_links: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarNode {
    pub name: String,
    pub path: Option<String>, // Some(path) for files, None for directories
    pub children: Vec<SidebarNode>,
}

#[derive(Debug, Clone)]
pub struct FlatNode {
    pub name: String,
    pub path: Option<String>,
    pub depth: usize,
}

impl SidebarNode {
    pub fn new_dir(name: String) -> Self {
        Self {
            name,
            path: None,
            children: Vec::new(),
        }
    }

    pub fn new_file(name: String, path: String) -> Self {
        Self {
            name,
            path: Some(path),
            children: Vec::new(),
        }
    }
}

/// 图谱节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,    // 节点 ID（通常是笔记路径）
    pub label: String, // 节点显示标签（笔记标题）
    pub title: String, // 悬停提示
    /// 笔记所属标签列表（用于节点颜色分组，取第一个标签决定颜色）
    #[serde(default)]
    pub tags: Vec<String>,
    /// 笔记最后修改时间（Unix 时间戳秒，v1.8.4 图谱热力图使用）
    #[serde(default)]
    pub mtime: i64,
}

/// 图谱边（连接）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String, // 源节点 ID
    pub to: String,   // 目标节点 ID
}

/// 图谱数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}
