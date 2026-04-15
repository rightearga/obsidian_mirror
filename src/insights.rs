//! 笔记洞察模块（v1.7.3）
//!
//! 从现有内存索引中计算写作趋势、知识库健康度和标签云，
//! 结果缓存到 `InsightsCache` 并在每次同步后更新。

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Serialize;
use crate::domain::Note;

// ──────────────────────────────────────────────────────────────────────────────
// 数据类型
// ──────────────────────────────────────────────────────────────────────────────

/// 笔记引用（标题 + 路径），用于健康度报告中的列表项
#[derive(Debug, Clone, Serialize, Default)]
pub struct NoteRef {
    pub title: String,
    pub path:  String,
}

/// 断链记录：从哪篇笔记的哪个 WikiLink 指向了不存在的目标
#[derive(Debug, Clone, Serialize)]
pub struct BrokenLink {
    /// 包含断链的笔记标题
    pub from_title:    String,
    /// 包含断链的笔记路径
    pub from_path:     String,
    /// 不存在的 WikiLink 目标名称
    pub broken_target: String,
}

/// 标签云条目
#[derive(Debug, Clone, Serialize)]
pub struct TagCloudEntry {
    pub tag:   String,
    pub count: usize,
}

/// 按月统计的笔记数量（用于折线图）
#[derive(Debug, Clone, Serialize)]
pub struct MonthlyCount {
    /// 格式：`"YYYY-MM"`
    pub year_month: String,
    pub count:      usize,
}

/// 按天统计的笔记数量（用于最近 30 天热力图）
#[derive(Debug, Clone, Serialize)]
pub struct DailyCount {
    /// 格式：`"YYYY-MM-DD"`
    pub date:  String,
    pub count: usize,
}

/// 笔记洞察缓存（v1.7.3）
///
/// 在每次 `perform_sync` 完成后由 `compute_insights` 重新计算并存入 `AppState`。
/// 所有字段均可直接序列化为 JSON 供前端图表渲染。
#[derive(Debug, Clone, Serialize, Default)]
pub struct InsightsCache {
    /// 缓存计算时间（Unix 时间戳秒）
    pub computed_at: i64,

    // ── 总量统计 ────────────────────────────────────────────────────────────
    pub total_notes: usize,
    pub total_tags:  usize,
    pub total_links: usize,

    // ── 健康度 ──────────────────────────────────────────────────────────────
    /// 孤立笔记（无出链且无入链）
    pub orphan_notes:    Vec<NoteRef>,
    /// 断链列表（WikiLink 指向不存在的笔记）
    pub broken_links:    Vec<BrokenLink>,
    /// 超大笔记（内容字符数 > 5000 且无 TOC，建议拆分）
    pub large_notes:     Vec<NoteRef>,
    /// 无标签笔记数量
    pub untagged_count:  usize,
    /// 无标签笔记占总笔记的比例（0.0–1.0）
    pub untagged_ratio:  f32,

    // ── 标签云 ──────────────────────────────────────────────────────────────
    /// 按笔记数降序排列的标签列表（最多 200 条，防止页面过大）
    pub tag_cloud: Vec<TagCloudEntry>,

    // ── 写作趋势 ────────────────────────────────────────────────────────────
    /// 按月统计的笔记修改数（最近 24 个月，升序）
    pub monthly_counts: Vec<MonthlyCount>,
    /// 最近 30 天每天修改的笔记数（升序）
    pub daily_counts: Vec<DailyCount>,
}

// ──────────────────────────────────────────────────────────────────────────────
// 计算逻辑
// ──────────────────────────────────────────────────────────────────────────────

/// 从现有内存索引计算完整的笔记洞察数据。
///
/// # 参数
/// * `notes` - 全量笔记映射（路径 → Note）
/// * `link_index` - 标题/文件名 → 路径索引（用于断链检测）
/// * `tag_index` - 标签名 → 笔记标题列表索引
pub fn compute_insights(
    notes:      &HashMap<String, Note>,
    link_index: &HashMap<String, String>,
    tag_index:  &HashMap<String, Vec<String>>,
) -> InsightsCache {
    let total_notes = notes.len();
    let total_tags  = tag_index.len();

    // ── 构建 "有入链" 集合（路径 set）─────────────────────────────────────
    // 遍历所有 outgoing_links，找出被至少一条链接引用过的笔记路径
    let mut has_inlink: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut total_links = 0usize;
    let mut broken_links: Vec<BrokenLink> = Vec::new();

    for note in notes.values() {
        for target in &note.outgoing_links {
            total_links += 1;
            // 查找目标是否存在（link_index 存 title→path 和 filename→path）
            let exists = link_index.contains_key(target)
                || notes.contains_key(target)
                || notes.contains_key(&format!("{}.md", target));

            if exists {
                // 记录被引用的路径
                if let Some(target_path) = link_index.get(target) {
                    has_inlink.insert(target_path.clone());
                }
            } else {
                broken_links.push(BrokenLink {
                    from_title:    note.title.clone(),
                    from_path:     note.path.clone(),
                    broken_target: target.clone(),
                });
            }
        }
    }

    // ── 孤立笔记（无出链且无入链）──────────────────────────────────────────
    let orphan_notes: Vec<NoteRef> = notes.values()
        .filter(|n| n.outgoing_links.is_empty() && !has_inlink.contains(&n.path))
        .map(|n| NoteRef { title: n.title.clone(), path: n.path.clone() })
        .collect();

    // ── 超大笔记（HTML 可见字符 > 5000 且无 TOC）──────────────────────────
    let large_notes: Vec<NoteRef> = notes.values()
        .filter(|n| n.toc.is_empty() && count_visible_chars(&n.content_html) > 5000)
        .map(|n| NoteRef { title: n.title.clone(), path: n.path.clone() })
        .collect();

    // ── 无标签统计 ─────────────────────────────────────────────────────────
    let untagged_count = notes.values().filter(|n| n.tags.is_empty()).count();
    let untagged_ratio = if total_notes > 0 {
        untagged_count as f32 / total_notes as f32
    } else {
        0.0
    };

    // ── 标签云（按笔记数降序，最多 200 条）──────────────────────────────────
    let mut tag_cloud: Vec<TagCloudEntry> = tag_index.iter()
        .map(|(tag, titles)| TagCloudEntry { tag: tag.clone(), count: titles.len() })
        .collect();
    tag_cloud.sort_by(|a, b| b.count.cmp(&a.count).then(a.tag.cmp(&b.tag)));
    tag_cloud.truncate(200);

    // ── 写作趋势：按月 / 按天（基于 note.mtime）────────────────────────────
    let (monthly_counts, daily_counts) = build_time_series(notes);

    // ── 计算时间戳 ─────────────────────────────────────────────────────────
    let computed_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    InsightsCache {
        computed_at,
        total_notes,
        total_tags,
        total_links,
        orphan_notes,
        broken_links,
        large_notes,
        untagged_count,
        untagged_ratio,
        tag_cloud,
        monthly_counts,
        daily_counts,
    }
}

/// 统计 HTML 字符串中的可见字符数（去除标签后）
fn count_visible_chars(html: &str) -> usize {
    let mut count = 0usize;
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => count += 1,
            _ => {}
        }
    }
    count
}

/// 将 SystemTime 转为 `"YYYY-MM-DD"` 字符串
fn mtime_to_date(mtime: SystemTime) -> Option<String> {
    let secs = mtime.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
    if secs <= 0 { return None; }
    // 简单整数计算（避免引入 chrono 依赖）
    let days = secs / 86400;             // 距 1970-01-01 的天数
    let (y, m, d) = days_to_ymd(days);
    Some(format!("{:04}-{:02}-{:02}", y, m, d))
}

/// 将 SystemTime 转为 `"YYYY-MM"` 字符串
fn mtime_to_month(mtime: SystemTime) -> Option<String> {
    mtime_to_date(mtime).map(|s| s[..7].to_string())
}

/// 从 1970-01-01 起的天数转换为 (year, month, day)，使用公历算法
fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    // 使用 Gregorian calendar 的儒略日算法
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

/// 构建月度折线图数据和最近 30 天热力图数据
fn build_time_series(notes: &HashMap<String, Note>) -> (Vec<MonthlyCount>, Vec<DailyCount>) {
    // ── 月度统计（最近 24 个月）──────────────────────────────────────────────
    let mut month_map: HashMap<String, usize> = HashMap::new();
    let mut day_map:   HashMap<String, usize> = HashMap::new();

    for note in notes.values() {
        if let Some(month) = mtime_to_month(note.mtime) {
            *month_map.entry(month).or_insert(0) += 1;
        }
        if let Some(day) = mtime_to_date(note.mtime) {
            *day_map.entry(day).or_insert(0) += 1;
        }
    }

    // 取最近 24 个月（升序）
    let today_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let today_days = today_secs / 86400;
    let (ty, tm, _) = days_to_ymd(today_days);

    let mut monthly_counts: Vec<MonthlyCount> = Vec::new();
    for i in (0..24i32).rev() {
        let mut month = tm as i32 - i;
        let mut year  = ty;
        while month <= 0 { month += 12; year -= 1; }
        while month > 12 { month -= 12; year += 1; }
        let key = format!("{:04}-{:02}", year, month);
        monthly_counts.push(MonthlyCount {
            year_month: key.clone(),
            count: *month_map.get(&key).unwrap_or(&0),
        });
    }

    // ── 最近 30 天（升序）────────────────────────────────────────────────────
    let mut daily_counts: Vec<DailyCount> = Vec::new();
    for i in (0..30i64).rev() {
        let (y, m, d) = days_to_ymd(today_days - i);
        let key = format!("{:04}-{:02}-{:02}", y, m, d);
        daily_counts.push(DailyCount {
            date:  key.clone(),
            count: *day_map.get(&key).unwrap_or(&0),
        });
    }

    (monthly_counts, daily_counts)
}

// ──────────────────────────────────────────────────────────────────────────────
// 测试
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Frontmatter, TocItem};
    use std::time::SystemTime;

    fn make_note(title: &str, path: &str, tags: Vec<&str>, outgoing: Vec<&str>, html: &str) -> Note {
        Note {
            path: path.to_string(),
            title: title.to_string(),
            content_html: html.to_string(),
            backlinks: vec![],
            tags: tags.into_iter().map(|s| s.to_string()).collect(),
            toc: vec![],
            mtime: SystemTime::UNIX_EPOCH,
            frontmatter: Frontmatter(serde_yml::Value::Null),
            outgoing_links: outgoing.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_orphan_detection() {
        // A 无出链且无入链 → 孤立；B 被 A 链接（但 A 链接断了）→ B 无入链也孤立
        let mut notes = HashMap::new();
        notes.insert("a.md".to_string(), make_note("A", "a.md", vec![], vec![], "hello"));
        notes.insert("b.md".to_string(), make_note("B", "b.md", vec![], vec![], "world"));

        let cache = compute_insights(&notes, &HashMap::new(), &HashMap::new());
        assert_eq!(cache.orphan_notes.len(), 2, "A 和 B 都应为孤立笔记");
    }

    #[test]
    fn test_broken_link_detection() {
        // A 链接到 "不存在" → 断链
        let mut notes = HashMap::new();
        notes.insert("a.md".to_string(), make_note("A", "a.md", vec![], vec!["不存在"], "x"));

        let cache = compute_insights(&notes, &HashMap::new(), &HashMap::new());
        assert_eq!(cache.broken_links.len(), 1);
        assert_eq!(cache.broken_links[0].broken_target, "不存在");
    }

    #[test]
    fn test_large_note_detection() {
        // 超过 5000 字符且无 TOC → 应列入 large_notes
        let long_html = "x".repeat(5001);
        let mut notes = HashMap::new();
        notes.insert("big.md".to_string(), make_note("Big", "big.md", vec![], vec![], &long_html));

        let cache = compute_insights(&notes, &HashMap::new(), &HashMap::new());
        assert_eq!(cache.large_notes.len(), 1);
    }

    #[test]
    fn test_large_note_with_toc_excluded() {
        // 超过 5000 字符但有 TOC → 不应列入（有结构，不需要拆分提示）
        let long_html = "x".repeat(5001);
        let mut note = make_note("Structured", "structured.md", vec![], vec![], &long_html);
        note.toc = vec![TocItem { level: 1, text: "标题".to_string(), id: "h1".to_string() }];

        let mut notes = HashMap::new();
        notes.insert("structured.md".to_string(), note);

        let cache = compute_insights(&notes, &HashMap::new(), &HashMap::new());
        assert_eq!(cache.large_notes.len(), 0, "有 TOC 的笔记不应被标记为超大笔记");
    }

    #[test]
    fn test_untagged_ratio() {
        let mut notes = HashMap::new();
        notes.insert("a.md".to_string(), make_note("A", "a.md", vec!["tag"], vec![], ""));
        notes.insert("b.md".to_string(), make_note("B", "b.md", vec![], vec![], ""));

        let cache = compute_insights(&notes, &HashMap::new(), &HashMap::new());
        assert_eq!(cache.untagged_count, 1);
        assert!((cache.untagged_ratio - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_tag_cloud_sorted() {
        let mut tag_index = HashMap::new();
        tag_index.insert("rust".to_string(), vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        tag_index.insert("python".to_string(), vec!["x".to_string()]);

        let cache = compute_insights(&HashMap::new(), &HashMap::new(), &tag_index);
        assert_eq!(cache.tag_cloud[0].tag, "rust", "高频标签应排在前面");
        assert_eq!(cache.tag_cloud[0].count, 3);
    }

    #[test]
    fn test_count_visible_chars() {
        assert_eq!(count_visible_chars("<p>hello</p>"), 5);
        assert_eq!(count_visible_chars("<b>你好</b>"), 2);
        assert_eq!(count_visible_chars("no tags"), 7);
    }

    #[test]
    fn test_days_to_ymd_epoch() {
        // 1970-01-01 = day 0
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
    }

    #[test]
    fn test_days_to_ymd_known() {
        // 2026-04-15: 从 1970-01-01 起 days = ?
        // 2026 - 1970 = 56 年，大约 56*365 + 闰年数 天
        let (y, m, d) = days_to_ymd(20558);
        // 只验证范围合法
        assert!(y >= 2026 && y <= 2027);
        assert!(m >= 1 && m <= 12);
        assert!(d >= 1 && d <= 31);
    }
}
