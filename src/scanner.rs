use crate::config::AppConfig;
use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::{debug, info};
use walkdir::WalkDir;

pub struct VaultScanner {
    config: AppConfig,
}

impl VaultScanner {
    pub fn new(config: AppConfig) -> Self {
        Self { config }
    }

    pub fn scan(&self) -> Result<Vec<PathBuf>> {
        let root = &self.config.local_path;
        let mut files = Vec::new();

        info!("Scanning vault at {:?}", root);

        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();

            // Skip directories for the file list (but we walked them)
            if path.is_dir() {
                continue;
            }

            // Must be markdown
            if path.extension().is_some_and(|ext| ext != "md") {
                continue;
            }

            // Check ignore rules
            if self.should_ignore(path, root) {
                continue;
            }

            files.push(path.to_path_buf());
        }

        debug!("Found {} markdown files", files.len());
        Ok(files)
    }

    fn should_ignore(&self, path: &Path, root: &Path) -> bool {
        let relative = match path.strip_prefix(root) {
            Ok(p) => p,
            Err(_) => return true,
        };

        // 相对路径字符串（统一用 / 作为分隔符，用于 glob 匹配）
        let relative_str = relative.to_string_lossy().replace('\\', "/");

        // 检查每个路径分量和完整路径
        for component in relative.components() {
            let s = component.as_os_str().to_string_lossy();

            // 1. 隐藏文件/目录（以 . 开头）忽略
            if s.starts_with('.') {
                return true;
            }

            // 2. 自定义忽略模式
            for pattern in &self.config.ignore_patterns {
                if is_glob_pattern(pattern) {
                    // 对完整相对路径做 glob 匹配
                    if glob_matches(pattern, &relative_str) {
                        return true;
                    }
                } else {
                    // 非 glob：对单个路径分量做大小写不敏感精确匹配
                    if s.eq_ignore_ascii_case(pattern) {
                        return true;
                    }
                }
            }
        }

        false
    }
} // impl VaultScanner

/// 判断模式字符串是否含有 glob 特殊字符
fn is_glob_pattern(pattern: &str) -> bool {
    pattern.contains('*') || pattern.contains('?') || pattern.contains('[')
}

/// 简单 glob 匹配（支持 `*`、`**`、`?`、字面量）
///
/// - `*` 匹配除 `/` 以外的任意字符序列
/// - `**` 匹配包含 `/` 在内的任意字符序列（跨目录）
/// - `?` 匹配除 `/` 以外的单个字符
fn glob_matches(pattern: &str, text: &str) -> bool {
    glob_match_recursive(pattern.as_bytes(), text.as_bytes())
}

fn glob_match_recursive(pat: &[u8], txt: &[u8]) -> bool {
    match (pat, txt) {
        ([], []) => true,
        ([], _) => false,
        // ** 匹配零个或多个任意字符（包含 /）
        ([b'*', b'*', rest @ ..], _) => {
            if glob_match_recursive(rest, txt) {
                return true;
            }
            for i in 0..=txt.len() {
                if glob_match_recursive(rest, &txt[i..]) {
                    return true;
                }
            }
            false
        }
        // * 匹配零个或多个非 / 字符
        ([b'*', rest @ ..], _) => {
            if glob_match_recursive(rest, txt) {
                return true;
            }
            for i in 0..txt.len() {
                if txt[i] == b'/' {
                    break;
                }
                if glob_match_recursive(rest, &txt[i + 1..]) {
                    return true;
                }
            }
            false
        }
        // ? 匹配一个非 / 字符
        ([b'?', rest_p @ ..], [c, rest_t @ ..]) if *c != b'/' => {
            glob_match_recursive(rest_p, rest_t)
        }
        ([b'?', ..], _) => false,
        // 字面量匹配（大小写不敏感）
        ([p, rest_p @ ..], [t, rest_t @ ..]) if p.eq_ignore_ascii_case(t) => {
            glob_match_recursive(rest_p, rest_t)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_literal_case_insensitive() {
        assert!(glob_matches("Draft", "Draft"));
        assert!(glob_matches("Draft", "draft"));
        assert!(glob_matches("draft", "DRAFT"));
    }

    #[test]
    fn test_glob_star_matches_within_component() {
        // * 不跨越 /
        assert!(glob_matches("*.tmp", "file.tmp"));
        assert!(!glob_matches("*.tmp", "dir/file.tmp"));
        assert!(glob_matches("note_*", "note_2024"));
    }

    #[test]
    fn test_glob_double_star_matches_across_dirs() {
        // ** 可以跨越 /
        assert!(glob_matches("draft/**", "draft/2024/note.md"));
        assert!(glob_matches("draft/**", "draft/note.md"));
        assert!(!glob_matches("draft/**", "other/note.md"));
    }

    #[test]
    fn test_glob_question_mark() {
        assert!(glob_matches("file?.md", "file1.md"));
        assert!(!glob_matches("file?.md", "file12.md"));
        assert!(!glob_matches("file?.md", "file/.md"));
    }

    #[test]
    fn test_is_glob_pattern() {
        assert!(is_glob_pattern("*.tmp"));
        assert!(is_glob_pattern("draft/**"));
        assert!(is_glob_pattern("file?.md"));
        assert!(!is_glob_pattern("Draft"));
        assert!(!is_glob_pattern(".obsidian"));
    }
}
