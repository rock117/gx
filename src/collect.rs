use anyhow::{Context, Result};
use crate::config::Config;
use crate::git::is_git_repo;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Collect all git repositories up to the specified depth
pub fn collect_git_repos(
    start_dir: &Path,
    max_depth: usize,
    config: &Config,
    exclude_regexes: &[regex::Regex],
) -> Result<Vec<PathBuf>> {
    let mut repos = Vec::new();

    // WalkDir is much faster than manual recursive fs::read_dir
    let walker = WalkDir::new(start_dir)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(|entry| !should_skip_dir(entry, config, exclude_regexes));

    for entry in walker {
        let entry = entry.context("Failed to read directory entry")?;

        // Only check directories, not files
        if entry.file_type().is_dir() {
            if is_git_repo(entry.path()) {
                repos.push(entry.path().to_path_buf());
            }
        }
    }

    // Sort for consistent ordering
    repos.sort();
    Ok(repos)
}

/// Determine if a directory entry should be skipped
pub fn should_skip_dir(
    entry: &walkdir::DirEntry,
    config: &Config,
    exclude_regexes: &[regex::Regex],
) -> bool {
    let file_name = entry.file_name();
    let name = file_name.to_string_lossy();

    // Don't skip root directory "."
    if name == "." {
        return false;
    }

    // Skip hidden directories (except .git)
    if name.starts_with('.') && name != ".git" {
        return true;
    }

    // Skip built-in common non-project directories
    if matches!(
        name.as_ref(),
        "node_modules" | "target" | "vendor" | "dist" | "build"
            | ".vscode" | ".idea" | "cache" | "tmp" | "temp"
    ) {
        return true;
    }

    // Check against config file exclusion patterns

    let full_path = entry.path();

    // 1. Check directory names and full paths
    for pattern in &config.exclude.names {
        // Check if pattern matches directory name
        if pattern == &*name {
            return true;
        }

        // Check if pattern matches full path (supports both / and \ separators)
        let path_str = full_path.to_str().unwrap_or("");
        let pattern_normalized = pattern.replace('\\', "/");
        let path_normalized = path_str.replace('\\', "/");

        // Match exact path or path ending with pattern
        if path_normalized == pattern_normalized
            || path_normalized.starts_with(&format!("{}/", pattern_normalized))
            || path_normalized.ends_with(&format!("/{}", pattern_normalized))
        {
            return true;
        }
    }

    // 2. Check glob patterns
    for glob_pattern in &config.exclude.globs {
        if let Ok(matched) = glob::Pattern::new(glob_pattern) {
            if matched.matches_path(full_path) {
                return true;
            }
        }
    }

    // 3. Check regex patterns
    for regex in exclude_regexes {
        if regex.is_match(&name) || regex.is_match(full_path.to_str().unwrap_or("")) {
            return true;
        }
    }

    false
}
