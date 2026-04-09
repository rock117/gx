use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::color::{c, Color};

/// Configuration file structure
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    /// Default maximum directory depth to search
    #[serde(default = "default_depth")]
    pub default_depth: usize,

    /// Directories/patterns to exclude from search
    #[serde(default)]
    pub exclude: ExcludePatterns,

    /// Custom shortcut commands
    #[serde(default)]
    pub shortcuts: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ExcludePatterns {
    /// Directory names to exclude
    #[serde(default)]
    pub names: Vec<String>,

    /// Glob patterns to exclude
    #[serde(default)]
    pub globs: Vec<String>,

    /// Regex patterns to exclude
    #[serde(default)]
    pub regexes: Vec<String>,
}

fn default_depth() -> usize {
    3
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_depth: 3,
            exclude: ExcludePatterns::default(),
            shortcuts: BTreeMap::new(),
        }
    }
}

/// Get the user-level configuration file path (cross-platform)
pub fn get_config_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().context("Failed to determine home directory")?;
    let config_dir = home_dir.join(".gx");

    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .context(format!("Failed to create config directory: {}", config_dir.display()))?;
    }

    Ok(config_dir.join("gx.json"))
}

/// Load configuration from a specific file path
fn load_config_from_path(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .context(format!("Failed to read config file: {}", path.display()))?;
    let config: Config = serde_json::from_str(&content)
        .context(format!("Failed to parse config file: {}", path.display()))?;
    Ok(config)
}

/// Merge arrays with deduplication
fn merge_arrays(base: &[String], override_: &[String]) -> Vec<String> {
    let mut result = base.to_vec();
    for item in override_ {
        if !result.contains(item) {
            result.push(item.clone());
        }
    }
    result
}

/// Merge two configs (override takes precedence)
fn merge_configs(base: &Config, override_: &Config) -> Config {
    let mut shortcuts = base.shortcuts.clone();
    for (k, v) in &override_.shortcuts {
        shortcuts.insert(k.clone(), v.clone());
    }

    Config {
        default_depth: override_.default_depth,
        exclude: ExcludePatterns {
            names: merge_arrays(&base.exclude.names, &override_.exclude.names),
            globs: merge_arrays(&base.exclude.globs, &override_.exclude.globs),
            regexes: merge_arrays(&base.exclude.regexes, &override_.exclude.regexes),
        },
        shortcuts,
    }
}

/// Load and merge configurations from multiple sources.
/// Returns (merged_config, list_of_loaded_files)
pub fn load_merged_config() -> Result<(Config, Vec<PathBuf>)> {
    let mut final_config = Config::default();
    let mut loaded_files = Vec::new();

    // 1. User-level config
    let user_config_path = get_config_path()?;
    if user_config_path.exists() {
        let config = load_config_from_path(&user_config_path)?;
        final_config = config;
        loaded_files.push(user_config_path);
    } else {
        let content = serde_json::to_string_pretty(&final_config)
            .context("Failed to serialize default config")?;
        fs::write(&user_config_path, content)
            .context(format!("Failed to write config file: {}", user_config_path.display()))?;
        eprintln!("Created default config file at: {}", user_config_path.display());
        eprintln!("You can customize it to set default depth and exclude patterns.\n");
        loaded_files.push(user_config_path);
    }

    // 2. Project-level config (overrides user config)
    let project_config_path = PathBuf::from(".gx/gx.json");
    if project_config_path.exists() {
        let config = load_config_from_path(&project_config_path)?;
        final_config = merge_configs(&final_config, &config);
        loaded_files.push(project_config_path);
    }

    Ok((final_config, loaded_files))
}

/// Show all active configuration files and merged result
pub fn show_config_info() -> Result<()> {
    let (config, loaded_files) = load_merged_config()?;

    println!("📁 Active Configuration Files:");
    if loaded_files.is_empty() {
        println!("  ⚠ No configuration files found");
    } else {
        for (i, path) in loaded_files.iter().enumerate() {
            let is_project = path.is_relative() && path.starts_with(".gx");
            let level = if is_project { "Project" } else { "User" };
            println!("  {}. [{}] {}", i + 1, level, path.display());
        }
    }
    println!();

    println!("📄 Merged Configuration:");
    let content = serde_json::to_string_pretty(&config)?;
    println!("{}", content);

    Ok(())
}

/// Save config to user-level config file
fn save_user_config(config: &Config) -> Result<()> {
    let config_path = get_config_path()?;
    let content = serde_json::to_string_pretty(config)
        .context("Failed to serialize config")?;
    fs::write(&config_path, content)
        .context(format!("Failed to write config file: {}", config_path.display()))?;
    Ok(())
}

/// Add a shortcut command
pub fn add_shortcut(name: &str, command: &str) -> Result<()> {
    let (mut config, _) = load_merged_config()?;

    // Validate command starts with "git"
    if !command.starts_with("git ") {
        anyhow::bail!("Shortcut command must start with 'git'. Example: 'git pull'");
    }

    let existed = config.shortcuts.contains_key(name);
    config.shortcuts.insert(name.to_string(), command.to_string());
    save_user_config(&config)?;

    if existed {
        println!("{} shortcut: {} → {}", c(Color::Yellow, "Updated"), c(Color::Cyan, name), command);
    } else {
        println!("{} shortcut: {} → {}", c(Color::Green, "Added"), c(Color::Cyan, name), command);
    }
    Ok(())
}

/// Remove a shortcut command
pub fn remove_shortcut(name: &str) -> Result<()> {
    let (mut config, _) = load_merged_config()?;

    if config.shortcuts.remove(name).is_some() {
        save_user_config(&config)?;
        println!("{} shortcut: {}", c(Color::Green, "Removed"), c(Color::Cyan, name));
    } else {
        anyhow::bail!("Shortcut '{}' not found", name);
    }
    Ok(())
}

/// List all shortcut commands
pub fn list_shortcuts() -> Result<()> {
    let (config, _) = load_merged_config()?;

    if config.shortcuts.is_empty() {
        println!("No shortcuts defined.");
        println!("Add one with: gx shortcut add <name> \"git <command>\"");
        return Ok(());
    }

    println!("{}", c(Color::Bold, "Shortcuts:"));
    let max_name_len = config.shortcuts.keys().map(|k| k.len()).max().unwrap_or(10);
    for (name, command) in &config.shortcuts {
        println!("  {:<width$}  {}", c(Color::Cyan, name), command, width = max_name_len);
    }
    Ok(())
}

/// Clear all shortcut commands
pub fn clear_shortcuts() -> Result<()> {
    let (mut config, _) = load_merged_config()?;

    if config.shortcuts.is_empty() {
        println!("No shortcuts to clear.");
        return Ok(());
    }

    let count = config.shortcuts.len();
    config.shortcuts.clear();
    save_user_config(&config)?;
    println!("{} {} shortcut(s)", c(Color::Green, "Cleared"), count);
    Ok(())
}
