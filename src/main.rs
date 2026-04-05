mod config;
mod collect;
mod git;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

use config::{load_merged_config, show_config_info};
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

    /// Only execute in repositories matching this branch name
    #[arg(long)]
    branch: Option<String>,

    /// Ignore errors from individual repositories, continue execution
    #[arg(long)]
    ignore_errors: bool,

    /// Generate shell completion script (bash, zsh, fish, powershell, elvish)
    #[arg(long, value_name = "SHELL")]
    completions: Option<String>,

    /// Git command and arguments (e.g., "git pull origin main")
    #[arg(required_unless_present_any = ["config", "completions"], num_args = 1.., allow_hyphen_values = true)]
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

    // Handle --completions
    if let Some(shell) = args.completions {
        print_completions(&shell)?;
        return Ok(());
    }

    // Handle --config flag
    if args.config {
        show_config_info()?;
        return Ok(());
    }

    // Load and merge configuration files
    let (config, _loaded_files) = load_merged_config()?;

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

fn print_completions(shell: &str) -> Result<()> {
    let shell_type = match shell {
        "bash" => clap_complete::Shell::Bash,
        "zsh" => clap_complete::Shell::Zsh,
        "fish" => clap_complete::Shell::Fish,
        "powershell" | "powershell.exe" | "ps1" => clap_complete::Shell::PowerShell,
        "elvish" => clap_complete::Shell::Elvish,
        _ => anyhow::bail!(
            "Unknown shell '{}'. Supported: bash, zsh, fish, powershell, elvish",
            shell
        ),
    };
    clap_complete::generate(
        shell_type,
        &mut <Args as clap::CommandFactory>::command(),
        "gx",
        &mut std::io::stdout(),
    );
    Ok(())
}
