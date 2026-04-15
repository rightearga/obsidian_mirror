use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, warn};
use anyhow::{Result, Context};
use serde::Serialize;

#[derive(Debug, Clone)]
pub enum SyncResult {
    /// 首次克隆，所有文件都是新的
    InitialClone,
    /// 增量更新，包含变更的文件列表（相对路径）
    IncrementalUpdate {
        changed: Vec<PathBuf>,
        deleted: Vec<PathBuf>,
    },
    /// 无变更
    NoChange,
}

/// 单次 Git 提交的元信息（v1.7.2：文件历史功能）
#[derive(Debug, Clone, Serialize)]
pub struct CommitInfo {
    /// 完整提交 hash（40 位十六进制）
    pub hash: String,
    /// 缩短的 hash（前 8 位，用于 UI 显示）
    pub hash_short: String,
    /// 作者邮箱
    pub author: String,
    /// 提交日期（ISO 8601，如 "2026-04-15 10:00:00 +0800"）
    pub date: String,
    /// 提交说明（首行摘要）
    pub subject: String,
}

pub struct GitClient;

impl GitClient {
    /// 同步 Git 仓库并返回变更信息
    pub async fn sync(repo_url: &str, local_path: &Path) -> Result<SyncResult> {
        let clean_url = repo_url.trim();
        
        // 1. 目录已存在时的处理
        if local_path.exists() {
            if local_path.join(".git").exists() {
                // 已是 git 仓库 → 执行 pull
                return Self::pull_and_diff(local_path).await;
            }

            // 目录存在但不是 git 仓库 —— 根据内容决定处理方式
            //
            // 应用启动时搜索引擎会在 local_path/.search_index/ 创建索引目录，
            // 导致 local_path 在首次克隆前就变成"非空"目录。
            // 判断规则：若目录内只有隐藏子目录（以 '.' 开头，如 .search_index），
            // 则视为"应用自动创建"，安全删除后重新克隆；
            // 若包含用户可见文件/目录则报错，避免误删数据。
            let has_user_content = std::fs::read_dir(local_path)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .any(|e| {
                            !e.file_name()
                                .to_string_lossy()
                                .starts_with('.')
                        })
                })
                .unwrap_or(false);

            if !has_user_content {
                // 仅有隐藏目录（如 .search_index）：安全删除整个目录后重新克隆
                warn!(
                    "  ├─ {:?} 存在但不是 git 仓库（仅含隐藏目录），自动清理后重新克隆",
                    local_path
                );
                tokio::fs::remove_dir_all(local_path)
                    .await
                    .context("清理非 git 目录失败")?;
                // fallthrough 到下方克隆逻辑
            } else {
                // 含用户可见内容：拒绝操作，避免误删数据
                return Err(anyhow::anyhow!(
                    "目录 {:?} 已存在且不是 git 仓库（目录含用户文件）。\n\
                     请手动删除该目录后重启服务，程序将自动重新克隆。",
                    local_path
                ));
            }
        }

        // 2. 目录不存在（或上面已删除空目录）→ 执行 clone
        Self::clone_repo(clean_url, local_path).await
    }
    
    /// 执行 git pull 并返回变更信息
    async fn pull_and_diff(local_path: &Path) -> Result<SyncResult> {
        info!("  ├─ 记录同步前的提交...");
        let old_commit = Self::get_current_commit(local_path).await?;

        info!("  ├─ 拉取最新更改: {}", local_path.display());
        let status = Command::new("git")
            .current_dir(local_path)
            .arg("pull")
            .status()
            .await
            .context("Failed to execute git pull")?;

        if !status.success() {
            warn!("  ├─ Git pull 失败，尝试 stash 后重新拉取...");
            Command::new("git")
                .current_dir(local_path)
                .args(["stash"])
                .status()
                .await?;

            let status = Command::new("git")
                .current_dir(local_path)
                .arg("pull")
                .status()
                .await?;

            if !status.success() {
                return Err(anyhow::anyhow!("Git pull failed even after stash"));
            }
            info!("  └─ Stash 后拉取成功");
        } else {
            info!("  └─ 拉取成功");
        }

        let new_commit = Self::get_current_commit(local_path).await?;

        if old_commit == new_commit {
            info!("  └─ 无变更");
            return Ok(SyncResult::NoChange);
        }

        info!("  ├─ 检测文件变更: {} -> {}", &old_commit[..8], &new_commit[..8]);
        let (changed, deleted) = Self::get_changed_files(local_path, &old_commit, &new_commit).await?;
        info!("  └─ 发现 {} 个修改，{} 个删除", changed.len(), deleted.len());

        Ok(SyncResult::IncrementalUpdate { changed, deleted })
    }

    /// 执行 git clone
    async fn clone_repo(repo_url: &str, local_path: &Path) -> Result<SyncResult> {
        info!("  ├─ 克隆仓库: {}", repo_url);
        info!("  ├─ 目标路径: {}", local_path.display());
        let parent = local_path.parent().unwrap_or(Path::new("."));
        if !parent.exists() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let status = Command::new("git")
            .arg("clone")
            .arg(repo_url)
            .arg(local_path)
            .status()
            .await
            .context("Failed to execute git clone")?;

        if !status.success() {
            return Err(anyhow::anyhow!("Git clone failed"));
        }
        info!("  └─ 克隆成功");

        Ok(SyncResult::InitialClone)
    }

    /// 获取当前 Git 提交 hash（公共接口，供 sync/handlers/main 共用）
    pub async fn get_current_commit(local_path: &Path) -> Result<String> {
        let output = Command::new("git")
            .current_dir(local_path)
            .args(["rev-parse", "HEAD"])
            .output()
            .await
            .context("Failed to get current commit")?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get current commit"));
        }
        
        let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(commit)
    }
    
    /// 获取指定文件的 Git 提交历史（v1.7.2）
    ///
    /// 使用 `git log --follow` 追踪文件重命名历史，返回按时间降序排列的提交列表。
    /// 文件未被 Git 追踪时返回空列表（不报错）。
    pub async fn get_file_history(
        file_rel_path: &str,
        local_path: &Path,
    ) -> Result<Vec<CommitInfo>> {
        let output = Command::new("git")
            .current_dir(local_path)
            .args([
                "-c", "core.quotePath=false",
                "log",
                "--follow",
                "--format=%H|%ae|%ai|%s",
                "--",
                file_rel_path,
            ])
            .output()
            .await
            .context("git log 执行失败")?;

        // 文件未被追踪时 git log 返回空输出（status=0），静默返回空列表
        let out = String::from_utf8_lossy(&output.stdout);
        let commits = out
            .lines()
            .filter(|l| !l.is_empty())
            .filter_map(|line| {
                // 格式：hash|author|date|subject（subject 可能包含 |，用 splitn(4)）
                let parts: Vec<&str> = line.splitn(4, '|').collect();
                if parts.len() < 4 {
                    return None;
                }
                let hash = parts[0].trim().to_string();
                let hash_short = hash.chars().take(8).collect();
                Some(CommitInfo {
                    hash_short,
                    hash,
                    author:  parts[1].trim().to_string(),
                    date:    parts[2].trim().to_string(),
                    subject: parts[3].trim().to_string(),
                })
            })
            .collect();

        Ok(commits)
    }

    /// 获取指定提交时的文件原始内容（v1.7.2）
    ///
    /// 使用 `git show {commit}:{path}` 读取历史快照内容。
    /// 返回原始 Markdown 文本，调用方负责渲染。
    pub async fn get_file_at_commit(
        file_rel_path: &str,
        commit: &str,
        local_path: &Path,
    ) -> Result<String> {
        let spec = format!("{}:{}", commit, file_rel_path);
        let output = Command::new("git")
            .current_dir(local_path)
            .args(["show", &spec])
            .output()
            .await
            .context("git show 执行失败")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "无法获取提交 {} 时的文件内容（文件可能在该提交时不存在）",
                commit
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// 获取指定提交相对于上一提交的 unified diff（v1.7.2）
    ///
    /// 使用 `git diff {commit}~1 {commit} -- {path}`。
    /// 若为首次提交（无父提交），自动回退为与空树的 diff。
    pub async fn get_file_diff(
        file_rel_path: &str,
        commit: &str,
        local_path: &Path,
    ) -> Result<String> {
        let output = Command::new("git")
            .current_dir(local_path)
            .args([
                "-c", "core.quotePath=false",
                "diff",
                &format!("{}~1", commit),
                commit,
                "--",
                file_rel_path,
            ])
            .output()
            .await
            .context("git diff 执行失败")?;

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }

        // 首次提交无父节点，改用 4b825dc...（Git 空树 hash）做比较
        let empty_tree = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";
        let output2 = Command::new("git")
            .current_dir(local_path)
            .args([
                "-c", "core.quotePath=false",
                "diff",
                empty_tree,
                commit,
                "--",
                file_rel_path,
            ])
            .output()
            .await
            .context("git diff（首次提交）执行失败")?;

        Ok(String::from_utf8_lossy(&output2.stdout).to_string())
    }

    /// 获取两个提交之间变更的文件列表
    async fn get_changed_files(
        local_path: &Path,
        old_commit: &str,
        new_commit: &str,
    ) -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
        // 获取修改和新增的文件（禁用文件名转义，使用原始 UTF-8）
        let output = Command::new("git")
            .current_dir(local_path)
            .args(["-c", "core.quotePath=false", "diff", "--name-only", "--diff-filter=AM", old_commit, new_commit])
            .output()
            .await
            .context("Failed to get changed files")?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get changed files"));
        }
        
        let changed_str = String::from_utf8_lossy(&output.stdout);
        let changed: Vec<PathBuf> = changed_str
            .lines()
            .filter(|l| !l.is_empty())
            .map(PathBuf::from)  // ✅ 不做路径转换，保持原始格式
            .collect();
        
        // 获取删除的文件（禁用文件名转义，使用原始 UTF-8）
        let output = Command::new("git")
            .current_dir(local_path)
            .args(["-c", "core.quotePath=false", "diff", "--name-only", "--diff-filter=D", old_commit, new_commit])
            .output()
            .await
            .context("Failed to get deleted files")?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to get deleted files"));
        }
        
        let deleted_str = String::from_utf8_lossy(&output.stdout);
        let deleted: Vec<PathBuf> = deleted_str
            .lines()
            .filter(|l| !l.is_empty())
            .map(PathBuf::from)  // ✅ 不做路径转换，保持原始格式
            .collect();
        
        Ok((changed, deleted))
    }
}
