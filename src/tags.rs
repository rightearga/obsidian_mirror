// 标签提取和处理模块

use regex::Regex;

/// 从 frontmatter 和正文中提取标签
pub fn extract_tags(content: &str, frontmatter: &serde_yml::Value) -> Vec<String> {
    let mut tags = Vec::new();

    // 1. 从 frontmatter 中提取标签
    if let serde_yml::Value::Mapping(map) = frontmatter {
        // 尝试获取 "tags" 字段
        if let Some(tags_value) = map.get(&serde_yml::Value::String("tags".to_string())) {
            extract_tags_from_yaml_value(tags_value, &mut tags);
        }

        // 也尝试 "tag" 字段（单数形式）
        if let Some(tag_value) = map.get(&serde_yml::Value::String("tag".to_string())) {
            extract_tags_from_yaml_value(tag_value, &mut tags);
        }
    }

    // 2. 从正文中提取 #标签 语法
    // 匹配 #标签 但不匹配 ## 标题
    // 支持中英文字符、数字、下划线、连字符
    let hashtag_regex = Regex::new(r"(?:^|[^#\w])#([\w\u4e00-\u9fa5_-]+)").unwrap();
    for caps in hashtag_regex.captures_iter(content) {
        if let Some(tag_match) = caps.get(1) {
            let tag = tag_match.as_str().to_string();
            if !tag.is_empty() && !tags.contains(&tag) {
                tags.push(tag);
            }
        }
    }

    tags
}

/// 从 YAML 值中提取标签
fn extract_tags_from_yaml_value(value: &serde_yml::Value, tags: &mut Vec<String>) {
    match value {
        // tags: [tag1, tag2, tag3]
        serde_yml::Value::Sequence(seq) => {
            for item in seq {
                if let serde_yml::Value::String(tag) = item {
                    let tag = tag.trim().to_string();
                    if !tag.is_empty() && !tags.contains(&tag) {
                        tags.push(tag);
                    }
                }
            }
        }
        // tags: tag1
        serde_yml::Value::String(tag) => {
            let tag = tag.trim().to_string();
            if !tag.is_empty() && !tags.contains(&tag) {
                tags.push(tag);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashtag_extraction() {
        let content = "This is a note with #tag1 and #tag2 in the text.";
        let frontmatter = serde_yml::Value::Null;
        let tags = extract_tags(content, &frontmatter);

        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&"tag1".to_string()));
        assert!(tags.contains(&"tag2".to_string()));
    }

    #[test]
    fn test_hashtag_chinese() {
        let content = "这是一个带有 #中文标签 和 #测试 的笔记。";
        let frontmatter = serde_yml::Value::Null;
        let tags = extract_tags(content, &frontmatter);

        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&"中文标签".to_string()));
        assert!(tags.contains(&"测试".to_string()));
    }

    #[test]
    fn test_hashtag_with_numbers_and_hyphens() {
        let content = "Tags: #tag-123, #my_tag, #test2024";
        let frontmatter = serde_yml::Value::Null;
        let tags = extract_tags(content, &frontmatter);

        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"tag-123".to_string()));
        assert!(tags.contains(&"my_tag".to_string()));
        assert!(tags.contains(&"test2024".to_string()));
    }

    #[test]
    fn test_hashtag_not_markdown_heading() {
        let content = "## This is a heading\n\nThis has #tag but not the heading.";
        let frontmatter = serde_yml::Value::Null;
        let tags = extract_tags(content, &frontmatter);

        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0], "tag");
    }

    #[test]
    fn test_frontmatter_tags_array() {
        let yaml_str = r#"
tags: [rust, programming, obsidian]
"#;
        let frontmatter: serde_yml::Value = serde_yml::from_str(yaml_str).unwrap();
        let content = "Content here.";

        let tags = extract_tags(content, &frontmatter);

        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"programming".to_string()));
        assert!(tags.contains(&"obsidian".to_string()));
    }

    #[test]
    fn test_frontmatter_tags_single() {
        let yaml_str = r#"
tags: important
"#;
        let frontmatter: serde_yml::Value = serde_yml::from_str(yaml_str).unwrap();
        let content = "Content here.";

        let tags = extract_tags(content, &frontmatter);

        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0], "important");
    }

    #[test]
    fn test_frontmatter_tag_singular() {
        let yaml_str = r#"
tag: [test, demo]
"#;
        let frontmatter: serde_yml::Value = serde_yml::from_str(yaml_str).unwrap();
        let content = "Content here.";

        let tags = extract_tags(content, &frontmatter);

        assert_eq!(tags.len(), 2);
        assert!(tags.contains(&"test".to_string()));
        assert!(tags.contains(&"demo".to_string()));
    }

    #[test]
    fn test_combined_tags_frontmatter_and_hashtags() {
        let yaml_str = r#"
tags: [frontmatter-tag]
"#;
        let frontmatter: serde_yml::Value = serde_yml::from_str(yaml_str).unwrap();
        let content = "# My Note\n\nThis has #inline-tag and #another-tag.";

        let tags = extract_tags(content, &frontmatter);

        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"frontmatter-tag".to_string()));
        assert!(tags.contains(&"inline-tag".to_string()));
        assert!(tags.contains(&"another-tag".to_string()));
    }

    #[test]
    fn test_duplicate_tags() {
        let yaml_str = r#"
tags: [duplicate]
"#;
        let frontmatter: serde_yml::Value = serde_yml::from_str(yaml_str).unwrap();
        let content = "This has #duplicate again.";

        let tags = extract_tags(content, &frontmatter);

        // 应该去重，只有一个
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0], "duplicate");
    }

    #[test]
    fn test_empty_tags() {
        let yaml_str = r#"
tags: []
"#;
        let frontmatter: serde_yml::Value = serde_yml::from_str(yaml_str).unwrap();
        let content = "No tags in content either.";

        let tags = extract_tags(content, &frontmatter);

        assert!(tags.is_empty());
    }
}
