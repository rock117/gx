use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "gx")]
#[command(about = "Execute git commands recursively in all git repositories", long_about = None)]
struct Args {
    /// Maximum directory depth to search (default: 3)
    #[arg(short, long, default_value_t = 3)]
    depth: usize,

    /// Starting directory (default: current directory)
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// Git command and arguments (e.g., "git pull origin main")
    #[arg(required = true, num_args = 1..)]
    git_args: Vec<String>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = Args::parse();
    let start_dir = args.path.unwrap_or_else(|| PathBuf::from("."));

    // Validate that first argument is "git"
    if args.git_args.is_empty() || args.git_args[0] != "git" {
        anyhow::bail!("First argument must be 'git'. Usage: gx git <command> [args]");
    }

    let git_cmd = &args.git_args[1..];
    if git_cmd.is_empty() {
        anyhow::bail!("Missing git command. Usage: gx git <command> [args]");
    }

    println!("Searching for git repositories in: {}", start_dir.display());
    println!("Max depth: {}", args.depth);
    println!("Command: git {}\n", git_cmd.join(" "));

    // Step 1: Efficiently collect all git repositories using WalkDir
    let repos = collect_git_repos(&start_dir, args.depth)?;

    if repos.is_empty() {
        println!("No git repositories found.");
        return Ok(());
    }

    println!("Found {} git repository(ies):", repos.len());
    for repo in &repos {
        println!("  📁 {}", repo.display());
    }
    println!();

    // Step 2: Execute git commands serially (preserves output order)
    for repo in repos {
        println!("📁 Executing in: {}", repo.display());
        execute_git_command(&repo, git_cmd)?;
    }

    Ok(())
}

/// Collect all git repositories up to the specified depth
fn collect_git_repos(start_dir: &Path, max_depth: usize) -> Result<Vec<PathBuf>> {
    let mut repos = Vec::new();

    // WalkDir is much faster than manual recursive fs::read_dir
    let walker = WalkDir::new(start_dir)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(|entry| !should_skip_dir(entry));

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
fn should_skip_dir(entry: &walkdir::DirEntry) -> bool {
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

    // Skip common non-project directories
    matches!(
        name.as_ref(),
        "node_modules" | "target" | "vendor" | "dist" | "build"
            | ".vscode" | ".idea" | "cache" | "tmp" | "temp"
    )
}

/// Check if directory is a git repository
fn is_git_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

/// Execute git command in the specified repository
fn execute_git_command(repo_dir: &Path, git_cmd: &[String]) -> Result<()> {
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
            eprint!("  ⚠ ");
        }
        eprint!("{}", stderr);
    }

    // Add spacing between repositories
    if !output.stdout.is_empty() || !output.stderr.is_empty() {
        println!();
    }

    Ok(())
}
