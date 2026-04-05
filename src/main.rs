mod config;
mod collect;
mod git;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

use config::{load_merged_config, show_config_info, add_shortcut, remove_shortcut, list_shortcuts};
use git::{execute_git_command, get_current_branch, get_repo_status};
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

    /// Only execute in repositories matching this branch name
    #[arg(long)]
    branch: Option<String>,

    /// Ignore errors from individual repositories, continue execution
    #[arg(long)]
    ignore_errors: bool,

    /// Show info overview of all repositories (branch, status, ahead/behind)
    #[arg(long)]
    info: bool,

    /// Git command and arguments (e.g., "git pull origin main")
    #[arg(required_unless_present_any = ["config", "info"], num_args = 1.., allow_hyphen_values = true)]
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

    // Handle --info flag
    if args.info {
        show_info(&args)?;
        return Ok(());
    }

    // Load and merge configuration files
    let (config, _loaded_files) = load_merged_config()?;

    // Handle shortcut management commands
    if !args.git_args.is_empty() && args.git_args[0] == "shortcut" {
        handle_shortcut(&args.git_args[1..])?;
        return Ok(());
    }

    // Resolve shortcut or validate git command
    let resolved_args = if !args.git_args.is_empty() && args.git_args[0] != "git" {
        if let Some(full_cmd) = config.shortcuts.get(&args.git_args[0]) {
            // Expand shortcut: "pull" + ["origin", "main"] → ["git", "pull", "origin", "main"]
            let mut expanded: Vec<String> = full_cmd.split_whitespace().map(String::from).collect();
            expanded.extend_from_slice(&args.git_args[1..]);
            expanded
        } else {
            anyhow::bail!(
                "Unknown command or shortcut '{}'. Usage: gx git <command> or gx <shortcut>",
                args.git_args[0]
            );
        }
    } else {
        args.git_args.clone()
    };

    // Validate that first argument is "git"
    if resolved_args.is_empty() || resolved_args[0] != "git" {
        anyhow::bail!("First argument must be 'git'. Usage: gx git <command> [args]");
    }

    let git_cmd = &resolved_args[1..];
    if git_cmd.is_empty() {
        anyhow::bail!("Missing git command. Usage: gx git <command> [args]");
    }

    let depth = args.depth.unwrap_or(config.default_depth);
    let start_dir = args.path.unwrap_or_else(|| PathBuf::from("."));

    println!("Searching for git repositories in: {}", start_dir.display());
    println!("Max depth: {}", depth);
    if args.branch.is_some() {
        println!("Branch filter: {}", args.branch.as_deref().unwrap());
    }
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

    // Step 1: Collect all git repositories
    let repos = collect_git_repos(&start_dir, depth, &config, &exclude_regexes)?;

    if repos.is_empty() {
        println!("No git repositories found.");
        return Ok(());
    }

    // Step 2: Filter by branch if specified
    let filtered_repos: Vec<PathBuf> = if let Some(ref branch_filter) = args.branch {
        repos
            .into_iter()
            .filter(|repo| {
                let branch = get_current_branch(repo);
                branch.as_deref() == Some(branch_filter.as_str())
            })
            .collect()
    } else {
        repos
    };

    if filtered_repos.is_empty() {
        println!("No git repositories matching branch '{}'.", args.branch.as_deref().unwrap_or(""));
        return Ok(());
    }

    println!("Found {} git repository(ies):", filtered_repos.len());
    for repo in &filtered_repos {
        let branch = get_current_branch(repo);
        match branch.as_deref() {
            Some(b) => println!("  📁 {} => \x1b[36m{}\x1b[0m", repo.display(), b),
            None => println!("  📁 {}", repo.display()),
        };
    }
    println!();

    // Step 3: Execute git commands serially
    let total = filtered_repos.len();
    let mut succeeded: usize = 0;
    let mut failed: usize = 0;

    for (index, repo) in filtered_repos.iter().enumerate() {
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
            match execute_git_command(repo, git_cmd) {
                Ok(_) => succeeded += 1,
                Err(_) => {
                    failed += 1;
                    if !args.ignore_errors {
                        println!();
                        println!("\x1b[31m✗ Stopped at {} (use --ignore-errors to continue)\x1b[0m",
                            repo.display());
                        break;
                    }
                }
            }
        }
    }

    // Show summary
    println!();
    let processed = succeeded + failed;
    if args.dry_run {
        println!("\x1b[33m[DRY RUN] Summary: {} repositories would be affected\x1b[0m", total);
    } else {
        let status = if failed == 0 { "\x1b[32m✓\x1b[0m" } else { "\x1b[33m⚠\x1b[0m" };
        println!("{} Summary: \x1b[36m{}\x1b[0m processed, \x1b[32m{}\x1b[0m succeeded, \x1b[31m{}\x1b[0m failed",
            status, processed, succeeded, failed);
        if processed < total {
            println!("\x1b[33m  {} repositories skipped (stopped early)\x1b[0m", total - processed);
        }
    }

    Ok(())
}

fn show_info(args: &Args) -> Result<()> {
    let (config, _loaded_files) = load_merged_config()?;
    let depth = args.depth.unwrap_or(config.default_depth);
    let start_dir = args.path.clone().unwrap_or_else(|| PathBuf::from("."));

    // Compile regex patterns from config
    let exclude_regexes: Result<Vec<regex::Regex>> = config
        .exclude
        .regexes
        .iter()
        .map(|pattern| regex::Regex::new(pattern).context(format!("Invalid regex pattern: {}", pattern)))
        .collect();
    let exclude_regexes = exclude_regexes.unwrap_or_default();

    let repos = collect_git_repos(&start_dir, depth, &config, &exclude_regexes)?;
    if repos.is_empty() {
        println!("No git repositories found.");
        return Ok(());
    }

    // Collect status for all repos
    let mut repo_infos: Vec<(std::path::PathBuf, git::RepoStatus)> = Vec::new();
    for repo in &repos {
        let status = get_repo_status(repo);
        repo_infos.push((repo.clone(), status));
    }

    // Filter by branch if specified
    if let Some(ref branch_filter) = args.branch {
        repo_infos.retain(|(_, status)| {
            status.branch.as_deref() == Some(branch_filter.as_str())
        });
    }

    if repo_infos.is_empty() {
        println!("No git repositories matching branch '{}'.", args.branch.as_deref().unwrap_or(""));
        return Ok(());
    }

    // Calculate column widths for alignment (based on visible chars only)
    let max_path_len = repo_infos.iter().map(|(p, _)| p.display().to_string().len()).max().unwrap_or(10);
    let path_width = max_path_len.max(6);
    let max_branch_len = repo_infos.iter().map(|(_, s)| s.branch.as_ref().map_or(8, |b| b.len())).max().unwrap_or(6);
    let branch_width = max_branch_len.max(6);

    let total = repo_infos.len();

    for (repo, status) in &repo_infos {
        let path_str = repo.display().to_string();

        // Branch (plain text for width calculation)
        let (branch_display, branch_visible_len) = match &status.branch {
            Some(b) => (format!("\x1b[36m{}\x1b[0m", b), b.len()),
            None => ("\x1b[90mdetached\x1b[0m".to_string(), 8),
        };

        // Status
        let (status_display, status_visible_len) = if status.branch.is_none() {
            ("\x1b[31m✗ detached\x1b[0m".to_string(), 10)
        } else if status.is_dirty {
            ("\x1b[33m⚠ dirty\x1b[0m".to_string(), 8)
        } else {
            ("\x1b[32m✓ clean\x1b[0m".to_string(), 7)
        };

        // Sync (ahead/behind) - always show for repos with branch
        let sync_display = if status.branch.is_some() {
            format!("↑{} ↓{}", status.ahead, status.behind)
        } else {
            String::new()
        };

        // Build line with manual padding
        let path_padded = format!("{:<width$}", path_str, width = path_width);
        let branch_padding = " ".repeat(branch_width - branch_visible_len);
        let status_padding = " ".repeat(10 - status_visible_len);

        println!("  📁 {}  {}{}  {}{}  {}",
            path_padded,
            branch_display, branch_padding,
            status_display, status_padding,
            sync_display,
        );
    }

    // Summary
    let dirty_count = repo_infos.iter().filter(|(_, s)| s.is_dirty).count();
    let ahead_count = repo_infos.iter().filter(|(_, s)| s.ahead > 0).count();
    let behind_count = repo_infos.iter().filter(|(_, s)| s.behind > 0).count();

    println!();
    print!("  Total: {} repos", total);
    if dirty_count > 0 { print!(" | \x1b[33m{} dirty\x1b[0m", dirty_count); }
    if ahead_count > 0 { print!(" | \x1b[32m{} ahead\x1b[0m", ahead_count); }
    if behind_count > 0 { print!(" | \x1b[31m{} behind\x1b[0m", behind_count); }
    println!();

    Ok(())
}

fn handle_shortcut(args: &[String]) -> Result<()> {
    if args.is_empty() {
        anyhow::bail!("Usage: gx shortcut <add|rm|list> [args]");
    }

    match args[0].as_str() {
        "add" => {
            if args.len() < 3 {
                anyhow::bail!("Usage: gx shortcut add <name> \"git <command>\"");
            }
            add_shortcut(&args[1], &args[2..].join(" "))?;
        }
        "rm" => {
            if args.len() < 2 {
                anyhow::bail!("Usage: gx shortcut rm <name>");
            }
            remove_shortcut(&args[1])?;
        }
        "list" => {
            list_shortcuts()?;
        }
        other => {
            anyhow::bail!("Unknown shortcut command '{}'. Use: add, rm, list", other);
        }
    }

    Ok(())
}
