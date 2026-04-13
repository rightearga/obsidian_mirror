use pulldown_cmark::{html, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use regex::Regex;
use lazy_static::lazy_static;
use crate::domain::TocItem;
use crate::tags;

lazy_static! {
    /// 匹配图片/文件类 WikiLink：![[文件路径]] 或 ![[文件路径|显示文本]]
    static ref IMAGE_WIKI_REGEX: Regex =
        Regex::new(r"!\[\[(.*?)(?:\|(.*?))?\]\]").unwrap();

    /// 匹配普通笔记 WikiLink：[[笔记]] 或 [[笔记|别名]]
    static ref WIKI_REGEX: Regex =
        Regex::new(r"\[\[(.*?)(?:\|(.*?))?\]\]").unwrap();

    /// 匹配标准 Markdown 图片语法：![alt](path)
    static ref MD_IMAGE_REGEX: Regex =
        Regex::new(r"!\[([^\]]*)\]\(([^)]+)\)").unwrap();

    /// 匹配标准 Markdown 链接语法：[text](path)
    static ref MD_LINK_REGEX: Regex =
        Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();

    /// 匹配 YAML Frontmatter 块（支持 \r\n 和 \n）
    static ref FRONTMATTER_REGEX: Regex =
        Regex::new(r"(?s)^---\s*\r?\n(.*?)\r?\n---\s*\r?\n(.*)$").unwrap();
}

pub struct MarkdownProcessor;

impl MarkdownProcessor {
    /// 处理 Markdown 内容，返回 (HTML, 链接列表, 标签列表, Frontmatter, TOC)
    pub fn process(
        content: &str,
    ) -> (
        String,
        Vec<String>,
        Vec<String>,
        serde_yml::Value,
        Vec<TocItem>,
    ) {
        // 1. Extract Frontmatter
        let (content_body, frontmatter) = Self::extract_frontmatter(content);

        // 2. Pre-process Image/File WikiLinks (处理 ![[...]] 语法)
        let mut processed_content = IMAGE_WIKI_REGEX
            .replace_all(&content_body, |caps: &regex::Captures| {
                let target = caps.get(1).map_or("", |m| m.as_str()).trim();
                let alt_text = caps.get(2).map_or("", |m| m.as_str()).trim();

                // 检查是否是图片文件
                let is_image = target.to_lowercase().ends_with(".png")
                    || target.to_lowercase().ends_with(".jpg")
                    || target.to_lowercase().ends_with(".jpeg")
                    || target.to_lowercase().ends_with(".gif")
                    || target.to_lowercase().ends_with(".svg")
                    || target.to_lowercase().ends_with(".webp");

                // URL 编码文件路径
                let url_encoded = percent_encoding::utf8_percent_encode(
                    target,
                    percent_encoding::NON_ALPHANUMERIC,
                )
                .to_string();

                if is_image {
                    // 图片:生成标准 Markdown 图片语法
                    // 如果用户没有指定 alt 文本,使用空字符串而不是文件名
                    // 在图片前后添加换行符,确保图片单独成行
                    format!("\n\n![{}](/assets/{})\n\n", alt_text, url_encoded)
                } else {
                    // 非图片文件(PDF、文档等):生成链接
                    // 链接文本使用 alt_text,如果为空则使用文件名
                    let link_text = if alt_text.is_empty() {
                        target
                    } else {
                        alt_text
                    };
                    format!("[{}](/assets/{})", link_text, url_encoded)
                }
            })
            .to_string();

        // 3. Pre-process WikiLinks (处理普通 [[...]] 链接)
        let mut links = Vec::new();
        processed_content = WIKI_REGEX
            .replace_all(&processed_content, |caps: &regex::Captures| {
                let target = caps.get(1).map_or("", |m| m.as_str()).trim();
                let label = caps.get(2).map_or(target, |m| m.as_str()).trim();

                // 检查是否是文件链接（PDF、图片等）
                let is_file = target.ends_with(".pdf")
                    || target.ends_with(".PDF")
                    || target.ends_with(".png")
                    || target.ends_with(".jpg")
                    || target.ends_with(".jpeg")
                    || target.ends_with(".gif")
                    || target.ends_with(".svg")
                    || target.ends_with(".webp");

                let url_encoded = percent_encoding::utf8_percent_encode(
                    target,
                    percent_encoding::NON_ALPHANUMERIC,
                )
                .to_string();

                if is_file {
                    // 文件链接，指向 /assets
                    format!("[{}](/assets/{})", label, url_encoded)
                } else {
                    // 笔记链接，指向 /doc
                    links.push(target.to_string());
                    format!("[{}]({}{})", label, "/doc/", url_encoded)
                }
            })
            .to_string();

        // 4. 处理标准 Markdown 图片相对路径 ![alt](path)
        // 将相对路径转换为 /assets/path
        processed_content = MD_IMAGE_REGEX
            .replace_all(&processed_content, |caps: &regex::Captures| {
                let alt_text = caps.get(1).map_or("", |m| m.as_str());
                let path = caps.get(2).map_or("", |m| m.as_str()).trim();

                // 如果路径不是绝对路径（不以 http:// 或 https:// 或 / 开头）
                if !path.starts_with("http://")
                    && !path.starts_with("https://")
                    && !path.starts_with('/')
                {
                    let url_encoded = percent_encoding::utf8_percent_encode(
                        path,
                        percent_encoding::NON_ALPHANUMERIC,
                    )
                    .to_string();
                    format!("![{}](/assets/{})", alt_text, url_encoded)
                } else {
                    // 保持原样
                    format!("![{}]({})", alt_text, path)
                }
            })
            .to_string();

        // 5. 处理标准 Markdown 链接中的文件相对路径 [text](file.pdf)
        processed_content = MD_LINK_REGEX
            .replace_all(&processed_content, |caps: &regex::Captures| {
                let text = caps.get(1).map_or("", |m| m.as_str());
                let path = caps.get(2).map_or("", |m| m.as_str()).trim();

                // 检查是否是文件链接
                let is_file = path.ends_with(".pdf")
                    || path.ends_with(".PDF")
                    || path.ends_with(".doc")
                    || path.ends_with(".docx")
                    || path.ends_with(".zip")
                    || path.ends_with(".rar");

                // 如果是相对路径的文件链接
                if is_file
                    && !path.starts_with("http://")
                    && !path.starts_with("https://")
                    && !path.starts_with('/')
                {
                    let url_encoded = percent_encoding::utf8_percent_encode(
                        path,
                        percent_encoding::NON_ALPHANUMERIC,
                    )
                    .to_string();
                    format!("[{}](/assets/{})", text, url_encoded)
                } else {
                    // 保持原样
                    format!("[{}]({})", text, path)
                }
            })
            .to_string();

        let final_markdown = processed_content;

        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);

        // 第一遍解析：生成 HTML 并收集标题
        let mut toc = Vec::new();
        let mut current_heading: Option<(HeadingLevel, String)> = None;
        let mut heading_counter = 0;

        // 使用双遍解析：第一遍提取 TOC，第二遍生成 HTML
        let events: Vec<_> = Parser::new_ext(&final_markdown, options.clone()).collect();

        // 第一遍：提取标题生成 TOC
        let mut i = 0;
        while i < events.len() {
            match &events[i] {
                Event::Start(Tag::Heading { level, .. }) => {
                    current_heading = Some((*level, String::new()));
                }
                Event::Text(text) if current_heading.is_some() => {
                    if let Some((_, ref mut heading_text)) = current_heading {
                        heading_text.push_str(text);
                    }
                }
                Event::End(TagEnd::Heading(_)) if current_heading.is_some() => {
                    if let Some((level, text)) = current_heading.take() {
                        let level_num = match level {
                            HeadingLevel::H1 => 1,
                            HeadingLevel::H2 => 2,
                            HeadingLevel::H3 => 3,
                            HeadingLevel::H4 => 4,
                            HeadingLevel::H5 => 5,
                            HeadingLevel::H6 => 6,
                        };

                        // 生成 ID
                        heading_counter += 1;
                        let id = Self::generate_heading_id(&text, heading_counter);

                        toc.push(TocItem {
                            level: level_num,
                            text: text.clone(),
                            id: id.clone(),
                        });
                    }
                }
                _ => {}
            }
            i += 1;
        }

        // 第二遍：生成带锚点的 HTML，并处理 Mermaid 代码块
        let parser = Parser::new_ext(&final_markdown, options);
        let mut html_output = String::new();
        let mut heading_counter = 0;
        let mut current_heading_text = String::new();
        let mut in_heading = false;
        let mut in_code_block = false;
        let mut code_block_lang = String::new();
        let mut code_block_content = String::new();

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level: _, .. }) => {
                    in_heading = true;
                    current_heading_text.clear();
                    // 暂不输出，等收集完文本后一起输出
                }
                Event::Text(ref text) if in_heading => {
                    current_heading_text.push_str(text);
                }
                Event::End(TagEnd::Heading(level)) if in_heading => {
                    heading_counter += 1;
                    let id = Self::generate_heading_id(&current_heading_text, heading_counter);

                    let level_num = match level {
                        HeadingLevel::H1 => 1,
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        HeadingLevel::H4 => 4,
                        HeadingLevel::H5 => 5,
                        HeadingLevel::H6 => 6,
                    };

                    // 输出带 ID 的标题（标题文本必须转义，防止 XSS 注入）
                    html_output.push_str(&format!(
                        "<h{} id=\"{}\">{}</h{}>\n",
                        level_num, id, Self::html_escape(&current_heading_text), level_num
                    ));

                    in_heading = false;
                }
                // 处理代码块（特别是 Mermaid）
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code_block = true;
                    code_block_content.clear();

                    // 提取语言标识
                    code_block_lang = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                        pulldown_cmark::CodeBlockKind::Indented => String::new(),
                    };
                }
                Event::Text(ref text) if in_code_block => {
                    code_block_content.push_str(text);
                }
                Event::End(TagEnd::CodeBlock) if in_code_block => {
                    // 检查是否为 Mermaid 代码块
                    if code_block_lang == "mermaid" {
                        // 生成 Mermaid 专用的 div，由前端 JavaScript 渲染
                        html_output.push_str("<div class=\"mermaid\">");
                        // HTML 转义内容以防止 XSS
                        let escaped = code_block_content
                            .replace('&', "&amp;")
                            .replace('<', "&lt;")
                            .replace('>', "&gt;");
                        html_output.push_str(&escaped);
                        html_output.push_str("</div>\n");
                    } else {
                        // 普通代码块，使用默认渲染
                        html_output.push_str("<pre><code");
                        if !code_block_lang.is_empty() {
                            html_output
                                .push_str(&format!(" class=\"language-{}\"", code_block_lang));
                        }
                        html_output.push_str(">");
                        let escaped = code_block_content
                            .replace('&', "&amp;")
                            .replace('<', "&lt;")
                            .replace('>', "&gt;");
                        html_output.push_str(&escaped);
                        html_output.push_str("</code></pre>\n");
                    }

                    in_code_block = false;
                    code_block_lang.clear();
                }
                _ => {
                    if !in_heading && !in_code_block {
                        // 正常处理其他事件
                        let mut temp_events = vec![event];
                        html::push_html(&mut html_output, temp_events.drain(..));
                    }
                }
            }
        }

        // 6. 提取标签
        let tags = tags::extract_tags(&content_body, &frontmatter);

        (html_output, links, tags, frontmatter, toc)
    }

    /// 对字符串进行 HTML 转义，防止 XSS 注入
    fn html_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    }

    /// 生成标题 ID（用于锚点）
    fn generate_heading_id(text: &str, counter: usize) -> String {
        // 生成基于标题文本的 ID，支持中文字符
        // 策略：保留字母、数字、中文、下划线和连字符，其他字符转换为连字符
        let sanitized = text
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    // 保留字母、数字、连字符、下划线（包括中文字符）
                    c
                } else if c.is_whitespace() {
                    // 空格转换为连字符
                    '-'
                } else {
                    // 其他特殊字符也转换为连字符
                    '-'
                }
            })
            .collect::<String>()
            .trim_matches('-') // 移除首尾的连字符
            .to_string();

        if sanitized.is_empty() {
            format!("heading-{}", counter)
        } else {
            // 为了确保唯一性，仍然附加计数器
            format!("{}-{}", sanitized, counter)
        }
    }

    fn extract_frontmatter(content: &str) -> (String, serde_yml::Value) {
        if let Some(caps) = FRONTMATTER_REGEX.captures(content) {
            let yaml_text = caps.get(1).map_or("", |m| m.as_str());
            let body = caps.get(2).map_or("", |m| m.as_str());

            match serde_yml::from_str(yaml_text) {
                Ok(fm) => return (body.to_string(), fm),
                Err(_e) => {
                    // tracing::warn!("Failed to parse YAML frontmatter: {}", e);
                    return (body.to_string(), serde_yml::Value::Null);
                }
            }
        }

        // Try simpler check if regex fails (e.g. file is ONLY frontmatter?)
        // Or if the file doesn't start with ---
        (content.to_string(), serde_yml::Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_markdown_to_html() {
        let content = "# Hello World\n\nThis is a **test**.";
        let (html, links, _tags, frontmatter, toc) = MarkdownProcessor::process(content);

        assert!(html.contains("<h1"));
        assert!(html.contains("Hello World</h1>"));
        assert!(html.contains("<strong>test</strong>"));
        assert!(links.is_empty());
        assert!(frontmatter.is_null());
        assert_eq!(toc.len(), 1);
        assert_eq!(toc[0].level, 1);
        assert_eq!(toc[0].text, "Hello World");
    }

    #[test]
    fn test_wikilink_basic() {
        let content = "This links to [[Target Note]].";
        let (html, links, _tags, _, _toc) = MarkdownProcessor::process(content);

        assert!(
            html.contains("/doc/Target%20Note"),
            "HTML 应包含编码后的链接"
        );
        assert_eq!(links.len(), 1);
        assert_eq!(links[0], "Target Note");
    }

    #[test]
    fn test_wikilink_with_label() {
        let content = "Check out [[Target Note|this amazing note]].";
        let (html, links, _tags, _, _toc) = MarkdownProcessor::process(content);

        assert!(html.contains("this amazing note"), "HTML 应包含自定义标签");
        assert!(
            html.contains("/doc/Target%20Note"),
            "HTML 应包含编码后的目标"
        );
        assert_eq!(links.len(), 1);
        assert_eq!(links[0], "Target Note");
    }

    #[test]
    fn test_wikilink_multiple() {
        let content = "Link to [[Note A]] and [[Note B]].";
        let (html, links, _tags, _, _toc) = MarkdownProcessor::process(content);

        assert_eq!(links.len(), 2);
        assert_eq!(links[0], "Note A");
        assert_eq!(links[1], "Note B");
        assert!(html.contains("/doc/Note%20A"));
        assert!(html.contains("/doc/Note%20B"));
    }

    #[test]
    fn test_wikilink_chinese_characters() {
        let content = "参考 [[中文笔记]] 和 [[Another 笔记|显示文本]].";
        let (html, links, _tags, _, _toc) = MarkdownProcessor::process(content);

        assert_eq!(links.len(), 2);
        assert_eq!(links[0], "中文笔记");
        assert_eq!(links[1], "Another 笔记");
        assert!(
            html.contains("%E4%B8%AD%E6%96%87%E7%AC%94%E8%AE%B0"),
            "中文应正确编码"
        );
    }

    #[test]
    fn test_image_wikilink() {
        let content = "![[image.png]]";
        let (html, links, _tags, _, _toc) = MarkdownProcessor::process(content);

        assert!(html.contains("<img"), "应生成 img 标签");
        assert!(html.contains("/assets/image%2Epng"), "图片应指向 /assets");
        assert!(links.is_empty(), "图片链接不应被记录为笔记链接");
    }

    #[test]
    fn test_image_wikilink_with_alt() {
        let content = "![[screenshot.jpg|My Screenshot]]";
        let (html, links, _tags, _, _toc) = MarkdownProcessor::process(content);

        assert!(html.contains("My Screenshot"), "应包含自定义 alt 文本");
        assert!(html.contains("/assets/screenshot%2Ejpg"));
        assert!(links.is_empty());
    }

    #[test]
    fn test_file_wikilink_pdf() {
        let content = "[[document.pdf|Read this PDF]]";
        let (html, links, _tags, _, _toc) = MarkdownProcessor::process(content);

        assert!(
            html.contains("/assets/document%2Epdf"),
            "PDF 应指向 /assets"
        );
        assert!(html.contains("Read this PDF"));
        assert!(links.is_empty(), "文件链接不应被记录为笔记链接");
    }

    #[test]
    fn test_markdown_image_relative_path() {
        let content = "![My Image](images/photo.png)";
        let (html, _links, _tags, _, _toc) = MarkdownProcessor::process(content);

        assert!(
            html.contains("/assets/images%2Fphoto%2Epng"),
            "相对路径应转换为 /assets"
        );
    }

    #[test]
    fn test_markdown_image_absolute_url() {
        let content = "![Remote Image](https://example.com/image.png)";
        let (html, _links, _tags, _, _toc) = MarkdownProcessor::process(content);

        assert!(
            html.contains("https://example.com/image.png"),
            "绝对 URL 应保持不变"
        );
        assert!(!html.contains("/assets/"), "绝对 URL 不应转换为 /assets");
    }

    #[test]
    fn test_markdown_link_to_pdf() {
        let content = "[Download PDF](files/report.pdf)";
        let (html, _links, _tags, _, _toc) = MarkdownProcessor::process(content);

        assert!(
            html.contains("/assets/files%2Freport%2Epdf"),
            "PDF 链接应转换为 /assets"
        );
    }

    #[test]
    fn test_frontmatter_extraction() {
        let content = r#"---
title: My Note
tags: [test, example]
date: 2024-01-01
---

# Content Here"#;

        let (html, _links, _tags, frontmatter, toc) = MarkdownProcessor::process(content);

        assert!(!frontmatter.is_null(), "应提取到 frontmatter");
        assert!(html.contains("Content Here</h1>"), "正文应正确解析");
        assert!(
            !html.contains("---"),
            "frontmatter 分隔符不应出现在 HTML 中"
        );
        assert_eq!(toc.len(), 1);
        assert_eq!(toc[0].text, "Content Here");

        // 验证 frontmatter 内容
        if let serde_yml::Value::Mapping(map) = frontmatter {
            assert!(map.contains_key(&serde_yml::Value::String("title".to_string())));
        } else {
            panic!("frontmatter 应该是一个 mapping");
        }
    }

    #[test]
    fn test_frontmatter_with_windows_line_endings() {
        let content = "---\r\ntitle: Test\r\n---\r\n\r\nContent";
        let (html, _links, _tags, frontmatter, _toc) = MarkdownProcessor::process(content);

        assert!(!frontmatter.is_null(), "应处理 Windows 行结尾");
        assert!(html.contains("Content"));
    }

    #[test]
    fn test_no_frontmatter() {
        let content = "Just regular content without frontmatter.";
        let (_html, _links, _tags, frontmatter, _toc) = MarkdownProcessor::process(content);

        assert!(frontmatter.is_null(), "无 frontmatter 时应返回 Null");
    }

    #[test]
    fn test_invalid_frontmatter() {
        let content = r#"---
this is not: valid: yaml::
---

Content"#;

        let (html, _links, _tags, frontmatter, _toc) = MarkdownProcessor::process(content);

        assert!(frontmatter.is_null(), "无效的 YAML 应返回 Null");
        assert!(html.contains("Content"), "正文仍应正常解析");
    }

    #[test]
    fn test_markdown_extensions() {
        // 测试表格
        let table_content = r#"| Header 1 | Header 2 |
|----------|----------|
| Cell 1   | Cell 2   |"#;
        let (html, _tags, _, _, _toc) = MarkdownProcessor::process(table_content);
        assert!(html.contains("<table"), "应支持表格");

        // 测试删除线
        let strikethrough_content = "~~deleted text~~";
        let (html, _tags, _, _, _toc) = MarkdownProcessor::process(strikethrough_content);
        assert!(html.contains("<del>deleted text</del>"), "应支持删除线");

        // 测试任务列表
        let tasklist_content = "- [ ] Todo item\n- [x] Done item";
        let (html, _tags, _, _, _toc) = MarkdownProcessor::process(tasklist_content);
        assert!(html.contains("checkbox"), "应支持任务列表");
    }

    #[test]
    fn test_mixed_wikilinks_and_markdown() {
        let content = r#"# My Note

Check [[Related Note]] and see ![screenshot](images/test.png).

Also read [[Another Note|this one]]."#;

        let (html, links, _tags, _, _toc) = MarkdownProcessor::process(content);

        assert_eq!(links.len(), 2);
        assert!(html.contains("My Note</h1>"));
        assert!(html.contains("/doc/Related%20Note"));
        assert!(html.contains("/doc/Another%20Note"));
        assert!(html.contains("/assets/images%2Ftest%2Epng"));
    }

    #[test]
    fn test_special_characters_in_wikilinks() {
        let content = "[[Note (with) [brackets] & special]]";
        let (html, links, _tags, _, _toc) = MarkdownProcessor::process(content);

        assert_eq!(links.len(), 1);
        assert_eq!(links[0], "Note (with) [brackets] & special");
        // URL 编码应处理特殊字符
        assert!(html.contains("/doc/"));
    }

    #[test]
    fn test_empty_content() {
        let content = "";
        let (html, links, _tags, frontmatter, toc) = MarkdownProcessor::process(content);

        assert!(html.is_empty() || html.trim().is_empty());
        assert!(links.is_empty());
        assert!(frontmatter.is_null());
        assert!(toc.is_empty());
    }

    #[test]
    fn test_url_encoding() {
        let content = "[[File Name With Spaces]]";
        let (html, links, _tags, _, _toc) = MarkdownProcessor::process(content);

        assert_eq!(links[0], "File Name With Spaces");
        assert!(
            html.contains("File%20Name%20With%20Spaces"),
            "空格应编码为 %20"
        );
    }

    #[test]
    fn test_image_formats() {
        let formats = vec![
            ("image.png", true),
            ("photo.jpg", true),
            ("pic.jpeg", true),
            ("animated.gif", true),
            ("icon.svg", true),
            ("modern.webp", true),
            ("document.pdf", false),
        ];

        for (filename, is_image) in formats {
            let content = format!("![[{}]]", filename);
            let (html, _links, _tags, _, _toc) = MarkdownProcessor::process(&content);

            if is_image {
                assert!(html.contains("<img"), "{} 应被识别为图片", filename);
            } else {
                assert!(html.contains("<a "), "{} 不应被识别为图片", filename);
            }
        }
    }
}

#[cfg(test)]
mod test_exact_case {
    use super::*;

    #[test]
    fn test_real_world_image() {
        let content = r"## 第四章節 角色攻略（2）

### 美月姐姐

![[冬日狂想曲全角色攻略 附图41.jpg]]

美月姐姐第一階段";

        let (html, _, _, _, _) = MarkdownProcessor::process(content);

        println!("\n=== 输入 ===\n{}\n", content);
        println!("=== 输出 HTML ===\n{}\n", html);

        // 检查 HTML 中不应该有裸露的文件名
        let cleaned = html.replace("src=\"", "").replace("alt=\"", "");
        assert!(
            !cleaned.contains("冬日狂想曲全角色攻略 附图41.jpg"),
            "HTML 中不应该有裸露的文件名文本"
        );
    }

    #[test]
    fn test_mermaid_code_block() {
        let content = r#"# 流程图测试

```mermaid
graph TD
    A[开始] --> B{判断条件}
    B -->|是| C[执行操作]
    B -->|否| D[结束]
    C --> D
```

这是一个简单的流程图。"#;

        let (html, _links, _tags, _, _toc) = MarkdownProcessor::process(content);

        println!("\n=== Mermaid 测试输出 ===\n{}\n", html);

        // 检查 HTML 中应该包含 mermaid class
        assert!(
            html.contains("<div class=\"mermaid\">"),
            "应生成 mermaid div"
        );
        assert!(html.contains("graph TD"), "应包含 Mermaid 图表定义");
        assert!(html.contains("开始"), "应保留中文内容");
        assert!(!html.contains("```"), "不应包含代码块标记");
        assert!(
            !html.contains("<pre><code"),
            "Mermaid 代码块不应渲染为普通代码块"
        );
    }

    #[test]
    fn test_regular_code_block() {
        let content = r#"# 代码示例

```rust
fn main() {
    println!("Hello, world!");
}
```

普通代码块测试。"#;

        let (html, _links, _tags, _, _toc) = MarkdownProcessor::process(content);

        println!("\n=== 普通代码块测试输出 ===\n{}\n", html);

        // 检查普通代码块应该正常渲染
        assert!(html.contains("<pre><code"), "应生成普通代码块");
        assert!(html.contains("language-rust"), "应包含语言标识");
        assert!(html.contains("fn main()"), "应包含代码内容");
        assert!(
            !html.contains("<div class=\"mermaid\">"),
            "不应渲染为 mermaid"
        );
    }

    #[test]
    fn test_heading_xss_escape() {
        // pulldown-cmark 将 markdown 中的 HTML 实体（如 &lt;）解码为文本字符 <，
        // 这些字符会出现在 Event::Text 中，插入 HTML 前必须再次转义，防止 XSS。
        // 例如：# Note &lt;tag&gt; → Event::Text("Note <tag>") → 需转义为 &lt;tag&gt;
        let content = "# Note &lt;injected&gt; & \"test\"";
        let (html, _links, _tags, _, toc) = MarkdownProcessor::process(content);

        // 转义后的 HTML 不应含有原始的 < 或 > 或未转义的 &
        assert!(
            !html.contains("<injected>"),
            "标题中解码后的 < > 字符应被 html_escape 重新转义"
        );
        // 应该包含转义后的形式
        assert!(
            html.contains("&lt;injected&gt;"),
            "< > 应被转义为 &lt; &gt;"
        );
        assert!(
            html.contains("&amp;"),
            "& 应被转义为 &amp;"
        );
        // TOC 中保留解码后的原始文本（供模板自行处理）
        assert!(
            toc[0].text.contains('<'),
            "TOC 保留原始文本，不二次转义"
        );
    }
}
