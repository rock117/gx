mod config;
mod collect;
mod git;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

use config::{load_or_create_config, show_config_info};
use git::{execute_git_command, get_current_branch};
use collect::collect_git_repos;

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
    let start_dir = args.path.unwrap_or_else(|| std::path::PathBuf::from("."));

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
