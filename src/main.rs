use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// Configuration file structure
#[derive(Debug, Deserialize, Serialize, Clone)]
struct Config {
    /// Default maximum directory depth to search
    #[serde(default = "default_depth")]
    default_depth: usize,

    /// Directories/patterns to exclude from search
    #[serde(default)]
    exclude: ExcludePatterns,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
struct ExcludePatterns {
    /// Directory names to exclude
    #[serde(default)]
    names: Vec<String>,

    /// Glob patterns to exclude
    #[serde(default)]
    globs: Vec<String>,

    /// Regex patterns to exclude
    #[serde(default)]
    regexes: Vec<String>,
}

fn default_depth() -> usize {
    3
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_depth: 3,
            exclude: ExcludePatterns::default(),
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "gx")]
#[command(about = "Execute git commands recursively in all git repositories", long_about = None)]
struct Args {
    /// Maximum directory depth to search (overrides config file)
    #[arg(short, long)]
    depth: Option<usize>,

    /// Starting directory (default: current directory)
    #[arg(short, long)]
    path: Option<PathBuf>,

    /// Show configuration file location and contents
    #[arg(long)]
    config: bool,

    /// Show what would be done without actually executing
    #[arg(long)]
    dry_run: bool,

    /// Git command and arguments (e.g., "git pull origin main")
    #[arg(required_unless_present = "config", num_args = 1..)]
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

    // Handle --config flag
    if args.config {
        show_config_info()?;
        return Ok(());
    }

    // Load or create configuration file
    let config = load_or_create_config()?;

    // Use command line depth if provided, otherwise use config default
    let depth = args.depth.unwrap_or(config.default_depth);
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
    println!("Max depth: {}", depth);
    println!("Command: git {}\n", git_cmd.join(" "));

    if args.dry_run {
        println!("\x1b[33m[DRY RUN] Showing what would be done without executing\x1b[0m\n");
    }

    // Compile regex patterns from config
    let exclude_regexes: Result<Vec<regex::Regex>> = config
        .exclude
        .regexes
        .iter()
        .map(|pattern| regex::Regex::new(pattern).context(format!("Invalid regex pattern: {}", pattern)))
        .collect();
    let exclude_regexes = exclude_regexes.unwrap_or_default();

    // Step 1: Efficiently collect all git repositories using WalkDir
    let repos = collect_git_repos(&start_dir, depth, &config, &exclude_regexes)?;

    if repos.is_empty() {
        println!("No git repositories found.");
        return Ok(());
    }

    println!("Found {} git repository(ies):", repos.len());
    for repo in &repos {
        let branch = get_current_branch(repo);
        match branch.as_deref() {
            Some(b) => println!("  📁 {} => \x1b[36m{}\x1b[0m", repo.display(), b),
            None => println!("  📁 {}", repo.display()),
        };
    }
    println!();

    // Step 2: Execute git commands serially (preserves output order)
    let total = repos.len();
    let mut succeeded = 0;
    let mut failed = 0;

    for (index, repo) in repos.iter().enumerate() {
        let branch = get_current_branch(repo);
        let progress = format!("[{}/{}]", index + 1, total);

        match branch.as_deref() {
            Some(b) => println!("{} 📁 {} => \x1b[36m{}\x1b[0m", progress, repo.display(), b),
            None => println!("{} 📁 {}", progress, repo.display()),
        };

        if args.dry_run {
            println!("  [DRY RUN] Would execute: git {} in {}",
                git_cmd.join(" "),
                repo.display());
        } else {
            match execute_git_command(&repo, git_cmd) {
                Ok(_) => succeeded += 1,
                Err(_) => failed += 1,
            }
        }
    }

    // Show summary
    println!();
    if args.dry_run {
        println!("\x1b[33m[DRY RUN] Summary: {} repositories would be affected\x1b[0m", total);
    } else {
        let status = if failed == 0 {
            "\x1b[32m✓\x1b[0m"
        } else {
            "\x1b[33m⚠\x1b[0m"
        };
        println!("{} Summary: \x1b[36m{}\x1b[0m processed, \x1b[32m{}\x1b[0m succeeded, \x1b[31m{}\x1b[0m failed",
            status, total, succeeded, failed);
    }

    Ok(())
}

/// Get the configuration file path (cross-platform)
fn get_config_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().context("Failed to determine home directory")?;
    let config_dir = home_dir.join(".gx");

    // Create .gx directory if it doesn't exist
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .context(format!("Failed to create config directory: {}", config_dir.display()))?;
    }

    Ok(config_dir.join("gx.json"))
}

/// Show configuration file location and contents
fn show_config_info() -> Result<()> {
    let config_path = get_config_path()?;

    println!("📁 Configuration File Location:");
    println!("{}\n", config_path.display());

    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .context("Failed to read config file")?;

        println!("📄 Current Configuration:");
        println!("{}", content);
    } else {
        println!("⚠ Configuration file does not exist yet.");
        println!("It will be created automatically on first run with default values.");
    }

    Ok(())
}

/// Load existing config or create default config file
fn load_or_create_config() -> Result<Config> {
    let config_path = get_config_path()?;

    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .context(format!("Failed to read config file: {}", config_path.display()))?;

        let config: Config = serde_json::from_str(&content)
            .context(format!("Failed to parse config file: {}", config_path.display()))?;

        Ok(config)
    } else {
        // Create default config file
        let default_config = Config::default();
        let content = serde_json::to_string_pretty(&default_config)
            .context("Failed to serialize default config")?;

        fs::write(&config_path, content)
            .context(format!("Failed to write config file: {}", config_path.display()))?;

        eprintln!("Created default config file at: {}", config_path.display());
        eprintln!("You can customize it to set default depth and exclude patterns.\n");

        Ok(default_config)
    }
}

/// Collect all git repositories up to the specified depth
fn collect_git_repos(
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
fn should_skip_dir(
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
fn get_current_branch(repo_dir: &Path) -> Option<String> {
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
