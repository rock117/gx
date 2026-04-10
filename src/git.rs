use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

use crate::color::{c, Color};

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
            eprint!("  {} ", c(Color::Red, "✗ Error:"));
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
    pub modified: usize,
    pub deleted: usize,
    pub added: usize,
}

/// Get detailed status of a git repository
pub fn get_repo_status(repo_dir: &Path) -> RepoStatus {
    let branch = get_current_branch(repo_dir);

    // Get ahead/behind count
    let (ahead, behind) = get_ahead_behind(repo_dir);

    // Get porcelain status for dirty check and file change counts
    let (is_dirty, modified, deleted, added) = parse_porcelain_status(repo_dir);

    RepoStatus {
        branch,
        is_dirty,
        ahead,
        behind,
        modified,
        deleted,
        added,
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

/// Parse porcelain status to get dirty flag and file change counts
fn parse_porcelain_status(repo_dir: &Path) -> (bool, usize, usize, usize) {
    let output = Command::new("git")
        .args(["status", "--porcelain=v1"])
        .current_dir(repo_dir)
        .output()
        .ok();

    if let Some(output) = output {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            let lines = s.lines().filter(|l| !l.trim().is_empty());
            let mut modified = 0;
            let mut deleted = 0;
            let mut added = 0;

            for line in lines {
                // Porcelain format: XY filename
                // X = index status, Y = worktree status
                let xy = line.as_bytes();
                if xy.len() < 2 { continue; }
                let x = xy[0] as char;
                let y = xy[1] as char;

                // Count based on combined X and Y status
                let is_modified = x == 'M' || y == 'M'
                    || x == 'R' || y == 'R'   // renamed counts as modified
                    || x == 'U' || y == 'U';  // updated but unmerged
                let is_deleted = x == 'D' || y == 'D';
                let is_added = x == 'A' || y == 'A'
                    || x == '?' || y == '?';  // untracked counts as added

                if is_added { added += 1; }
                else if is_deleted { deleted += 1; }
                else if is_modified { modified += 1; }
            }

            let is_dirty = modified > 0 || deleted > 0 || added > 0;
            return (is_dirty, modified, deleted, added);
        }
    }

    (false, 0, 0, 0)
}

/// Check if directory is a git repository
pub fn is_git_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

/// Latest commit info
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

/// Get the latest commit of a git repository
pub fn get_latest_commit(repo_dir: &Path) -> Option<CommitInfo> {
    get_commits(repo_dir, 1, &[]).into_iter().next()
}

/// Get the N latest commits of a git repository
pub fn get_commits(repo_dir: &Path, n: usize, extra_args: &[String]) -> Vec<CommitInfo> {
    get_commits_from_ref(repo_dir, n, "HEAD", extra_args)
}

/// Get the N latest commits from a specific ref (e.g. "origin/main")
pub fn get_commits_from_ref(repo_dir: &Path, n: usize, git_ref: &str, extra_args: &[String]) -> Vec<CommitInfo> {
    let mut args = vec![
        "log".to_string(),
        format!("-{}", n),
        "--format=%h|%an|%ad|%s".to_string(),
        "--date=format:%Y-%m-%d %H:%M".to_string(),
    ];
    args.extend_from_slice(extra_args);
    args.push(git_ref.to_string());

    let output = std::process::Command::new("git")
        .args(&args)
        .current_dir(repo_dir)
        .output()
        .ok();

    if let Some(output) = output {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            let mut commits = Vec::new();
            for line in s.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let parts: Vec<&str> = line.splitn(4, '|').collect();
                if parts.len() == 4 {
                    commits.push(CommitInfo {
                        hash: parts[0].to_string(),
                        author: parts[1].to_string(),
                        date: parts[2].to_string(),
                        message: parts[3].to_string(),
                    });
                }
            }
            return commits;
        }
    }
    Vec::new()
}

/// Fetch remote updates quietly
pub fn fetch_remote(repo_dir: &Path) -> bool {
    Command::new("git")
        .args(["fetch", "--quiet"])
        .current_dir(repo_dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the upstream tracking branch name (e.g. "origin/main")
pub fn get_upstream_branch(repo_dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "@{upstream}"])
        .current_dir(repo_dir)
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() {
            None
        } else {
            Some(branch)
        }
    } else {
        None
    }
}
