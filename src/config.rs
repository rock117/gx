use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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

/// Get the configuration file path (cross-platform)
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

/// Show configuration file location and contents
pub fn show_config_info() -> Result<()> {
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
pub fn load_or_create_config() -> Result<Config> {
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
