use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration file structure
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    /// Default maximum directory depth to search
    #[serde(default = "default_depth")]
    pub default_depth: usize,

    /// Directories/patterns to exclude from search
    #[serde(default)]
    pub exclude: ExcludePatterns,
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
        }
    }
}

/// Get the user-level configuration file path (cross-platform)
pub fn get_config_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().context("Failed to determine home directory")?;
    let config_dir = home_dir.join(".gx");

    // Create .gx directory if it doesn't exist
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
fn merge_arrays(base: Vec<String>, override_: Vec<String>) -> Vec<String> {
    let mut result = base.clone();
    for item in override_ {
        if !result.contains(&item) {
            result.push(item);
        }
    }
    result
}

/// Merge two configs (override values take precedence)
fn merge_configs(base: Config, override_: Config) -> Config {
    Config {
        default_depth: override_.default_depth,
        exclude: ExcludePatterns {
            names: merge_arrays(base.exclude.names, override_.exclude.names),
            globs: merge_arrays(base.exclude.globs, override_.exclude.globs),
            regexes: merge_arrays(base.exclude.regexes, override_.exclude.regexes),
        },
    }
}

/// Load and merge configurations from multiple sources
/// Returns (merged_config, list_of_loaded_files)
pub fn load_merged_config() -> Result<(Config, Vec<PathBuf>)> {
    let mut final_config = Config::default();
    let mut loaded_files = Vec::new();

    // 1. Load user-level config (base configuration)
    let user_config_path = get_config_path()?;
    if user_config_path.exists() {
        let config = load_config_from_path(&user_config_path)?;
        final_config = config;
        loaded_files.push(user_config_path);
    } else {
        // Create default user config
        let default_config = Config::default();
        let content = serde_json::to_string_pretty(&default_config)
            .context("Failed to serialize default config")?;

        fs::write(&user_config_path, content)
            .context(format!("Failed to write config file: {}", user_config_path.display()))?;

        eprintln!("Created default config file at: {}", user_config_path.display());
        eprintln!("You can customize it to set default depth and exclude patterns.\n");

        loaded_files.push(user_config_path);
    }

    // 2. Load project-level config (overrides user config)
    let project_config_path = PathBuf::from(".gx/gx.json");
    if project_config_path.exists() {
        let config = load_config_from_path(&project_config_path)?;
        final_config = merge_configs(final_config, config);
        loaded_files.push(project_config_path);
    }

    Ok((final_config, loaded_files))
}

/// Show configuration file location and contents
pub fn show_config_info() -> Result<()> {
    let (config, loaded_files) = load_merged_config()?;

    println!("📁 Active Configuration Files:");
    if loaded_files.is_empty() {
        println!("  ⚠ No configuration files found");
    } else {
        for (i, path) in loaded_files.iter().enumerate() {
            // Check if it's a relative path starting with "."
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

/// Load existing config or create default config file (legacy function for backward compatibility)
#[deprecated(note = "Use load_merged_config() instead")]
pub fn load_or_create_config() -> Result<Config> {
    let (config, _) = load_merged_config()?;
    Ok(config)
}
