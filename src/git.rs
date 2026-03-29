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

/// Check if directory is a git repository
pub fn is_git_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}
