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
            if path.extension().map_or(false, |ext| ext != "md") {
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
            Err(_) => return true, // Should not happen if walking from root
        };

        // Check each component of the path
        for component in relative.components() {
            let s = component.as_os_str().to_string_lossy();

            // 1. Ignore hidden files/dirs (starting with .)
            if s.starts_with('.') {
                return true;
            }

            // 2. Custom ignore patterns
            for pattern in &self.config.ignore_patterns {
                if s.eq_ignore_ascii_case(pattern) {
                    return true;
                }
            }
        }

        false
    }
}
