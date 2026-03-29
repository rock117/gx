use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;

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

    // Canonicalize the path to get absolute path
    let start_dir = fs::canonicalize(&start_dir)
        .context("Failed to resolve start directory")?;

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

    process_directory(&start_dir, 0, args.depth, git_cmd)?;

    Ok(())
}

fn process_directory(dir: &Path, current_depth: usize, max_depth: usize, git_cmd: &[String]) -> Result<()> {
    // Check if current directory is a git repository
    if is_git_repo(dir) {
        println!("📁 Found git repo: {}", dir.display());
        execute_git_command(dir, git_cmd)?;
    }

    // Don't go deeper than max_depth
    if current_depth >= max_depth {
        return Ok(());
    }

    // Recursively process subdirectories
    let entries = fs::read_dir(dir)
        .context(format!("Failed to read directory: {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden directories (except .git which we already checked)
        if path.is_dir() {
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            // Skip hidden directories and common non-project directories
            if file_name.starts_with('.') && file_name != ".git" {
                continue;
            }

            // Skip common directories that don't contain projects
            matches!(file_name,
                "node_modules" | "target" | "vendor" | "dist" | "build" |
                ".vscode" | ".idea" | "cache" | "tmp" | "temp"
            );

            process_directory(&path, current_depth + 1, max_depth, git_cmd)?;
        }
    }

    Ok(())
}

fn is_git_repo(dir: &Path) -> bool {
    let git_dir = dir.join(".git");
    git_dir.exists()
}

fn execute_git_command(repo_dir: &Path, git_cmd: &[String]) -> Result<()> {
    let output = Command::new("git")
        .args(git_cmd)
        .current_dir(repo_dir)
        .output()
        .context(format!("Failed to execute git command in: {}", repo_dir.display()))?;

    // Display stdout if present
    if !output.stdout.is_empty() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        print!("{}", stdout);
    }

    // Display stderr if present (git often outputs info to stderr)
    if !output.stderr.is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // If command failed, show error indicator
        if !output.status.success() {
            eprint!("  ⚠ Error: ");
        }
        eprint!("{}", stderr);
    }

    // Add a blank line between repositories for better readability
    if !output.stdout.is_empty() || !output.stderr.is_empty() {
        println!();
    }

    Ok(())
}
