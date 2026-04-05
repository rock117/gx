use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Execute git command in the specified repository
pub fn execute_git_command(repo_dir: &Path, git_cmd: &[String]) -> Result<()> {
    let output = Command::new("git")
        .args(git_cmd)
        .current_dir(repo_dir)
        .output()
        .context(format!("Failed to execute git command in: {}", repo_dir.display()))?;

    // Display stdout
    if !output.stdout.is_empty() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        print!("{}", stdout);
    }

    // Display stderr
    if !output.stderr.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            eprint!("  \x1b[31m✗ Error:\x1b[0m ");
        }
        eprint!("{}", stderr);
    }

    // Add spacing between repositories
    if !output.stdout.is_empty() || !output.stderr.is_empty() {
        println!();
    }

    // Return error if git command failed
    if !output.status.success() {
        anyhow::bail!("Git command failed with exit code: {:?}", output.status.code());
    }

    Ok(())
}

/// Get the current branch name of a git repository
pub fn get_current_branch(repo_dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_dir)
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() || branch == "HEAD" {
            None
        } else {
            Some(branch)
        }
    } else {
        None
    }
}

/// Repository status info for overview display
pub struct RepoStatus {
    pub branch: Option<String>,
    pub is_dirty: bool,
    pub ahead: usize,
    pub behind: usize,
}

/// Get detailed status of a git repository
pub fn get_repo_status(repo_dir: &Path) -> RepoStatus {
    let branch = get_current_branch(repo_dir);

    // Get ahead/behind count
    let (ahead, behind) = get_ahead_behind(repo_dir);

    // Get porcelain status for dirty check
    let is_dirty = is_dirty_check(repo_dir);

    RepoStatus {
        branch,
        is_dirty,
        ahead,
        behind,
    }
}

/// Get ahead/behind counts relative to upstream
fn get_ahead_behind(repo_dir: &Path) -> (usize, usize) {
    let output = Command::new("git")
        .args(["rev-list", "--count", "--left-right", "@{upstream}...HEAD"])
        .current_dir(repo_dir)
        .output()
        .ok();

    if let Some(output) = output {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let parts: Vec<&str> = s.split_whitespace().collect();
            if parts.len() == 2 {
                let behind = parts[0].parse().unwrap_or(0);
                let ahead = parts[1].parse().unwrap_or(0);
                return (ahead, behind);
            }
            // If only one number, it could be behind only
            if parts.len() == 1 {
                if let Ok(n) = parts[0].parse() {
                    return (0, n);
                }
            }
        }
    }
    (0, 0)
}

/// Check if repo has any changes (dirty)
fn is_dirty_check(repo_dir: &Path) -> bool {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1"])
        .current_dir(repo_dir)
        .output()
        .ok();

    if let Some(output) = output {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            return !s.trim().is_empty();
        }
    }

    false
}

/// Check if directory is a git repository
pub fn is_git_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

/// Latest commit info
pub struct LatestCommit {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

/// Get the latest commit of a git repository
pub fn get_latest_commit(repo_dir: &Path) -> Option<LatestCommit> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%h|%an|%ar|%s"])
        .current_dir(repo_dir)
        .output()
        .ok()?;

    if output.status.success() {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let parts: Vec<&str> = s.splitn(4, '|').collect();
        if parts.len() == 4 {
            return Some(LatestCommit {
                hash: parts[0].to_string(),
                author: parts[1].to_string(),
                date: parts[2].to_string(),
                message: parts[3].to_string(),
            });
        }
    }
    None
}
