# gx

A command-line tool that recursively executes git commands across all git repositories in a directory tree.

## Features

- 🔍 Recursively search for git repositories in current directory and subdirectories
- 🎯 Execute git commands in all found repositories
- 🌲 Display current branch name for each repository
- ⚙️ Configurable search depth (default: 3 levels)
- 🚀 Skip common non-project directories (node_modules, target, vendor, etc.)
- 📝 Support for complete git commands with arguments
- 🔮 Dry-run mode to preview operations without executing
- 📊 Progress indicators and execution statistics

## Installation

### Build from source

```bash
# Clone or navigate to the project directory
cd gx

# Build the project
cargo build --release

# The binary will be at: ./target/release/gx.exe (Windows) or ./target/release/gx (Linux/Mac)
```

### Add to PATH

**Windows:**
```powershell
# Copy to a directory in your PATH, e.g.:
copy target\release\gx.exe C:\Users\YourName\.cargo\bin\
```

**Linux/Mac:**
```bash
# Copy to a directory in your PATH, e.g.:
sudo cp target/release/gx /usr/local/bin/
```

## Usage

### Basic Syntax

```bash
gx [OPTIONS] git <command> [args...]
```

### Examples

**Pull all repositories (current directory, depth 3):**
```bash
gx git pull
```

**Pull with specified depth (5 levels):**
```bash
gx --depth 5 git pull
```

**Pull from specific remote and branch:**
```bash
gx git pull origin main
```

**Check status of all repositories:**
```bash
gx git status
```

**Fetch from all remotes:**
```bash
gx git fetch --all
```

**Specify starting directory:**
```bash
gx --path /path/to/projects git pull
```

**Show last commit in each repository:**
```bash
gx git log -1 --oneline
```

**Preview what would be done without executing (dry-run):**
```bash
gx --dry-run git push
```

### Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--depth` | `-d` | Maximum directory depth to search | `3` |
| `--path` | `-p` | Starting directory (absolute or relative path) | Current directory |
| `--config` | - | Show configuration file location and contents | - |
| `--dry-run` | - | Show what would be done without executing | - |
| `--help` | `-h` | Show help message | - |

### Show Configuration

To view your configuration file location and current settings:

```bash
gx --config
```

Output example:
```
📁 Configuration File Location:
C:\Users\YourUsername\.gx\gx.json

📄 Current Configuration:
{
  "default_depth": 5,
  "exclude": {
    "names": ["temp", "logs"],
    "globs": ["*-backup"],
    "regexes": ["^test-.*$"]
  }
}
```

### Dry-run Mode

Preview what would be done without actually executing commands:

```bash
gx --dry-run git push
```

Output example:
```
[DRY RUN] Showing what would be done without executing

Found 2 git repository(ies):
  📁 ./repo1 => main
  📁 ./repo2 => dev

[1/2] 📁 ./repo1 => main
  [DRY RUN] Would execute: git push in ./repo1
[2/2] 📁 ./repo2 => dev
  [DRY RUN] Would execute: git push in ./repo2

[DRY RUN] Summary: 2 repositories would be affected
```

**Use cases for dry-run:**
- 🔍 **Preview impact** - See which repositories will be affected
- ⚠️ **Safety check** - Avoid accidental destructive operations
- 🔧 **Debug configuration** - Verify exclusion patterns are working
- 📊 **Performance estimate** - Know how many repos will be processed

```bash
gx --config
```

Output example:
```
📁 Configuration File Location:
C:\Users\YourUsername\.gx\gx.json

📄 Current Configuration:
{
  "default_depth": 5,
  "exclude": {
    "names": ["temp", "logs"],
    "globs": ["*-backup"],
    "regexes": ["^test-.*$"]
  }
}
```

### Output

The tool displays:
- 🔍 Search directory and depth
- 📁 Each git repository found with current branch (color-coded)
- 📊 Progress counter [X/total] for each repository
- 📋 Git command output from each repository
- 📈 Execution summary (total processed, succeeded, failed)
- ⚠️ Any errors encountered (with color-coded indicators)

Example output:
```
Searching for git repositories in: C:\Users\projects
Max depth: 3
Command: git pull

Found 2 git repository(ies):
  📁 ./project1 => main
  📁 ./project2 => dev

[1/2] 📁 ./project1 => main
Already up to date.

[2/2] 📁 ./project2 => dev
Updating abc1234..def5678
Fast-forward
 file.txt | 2 +-
 1 file changed, 1 insertion(+), 1 deletion(-)

✓ Summary: 2 processed, 2 succeeded, 0 failed
```

**Color coding:**
- 🟦 Cyan - Branch names
- 🟢 Green - Success indicators (✓)
- 🟡 Yellow - Warnings and dry-run mode
- 🔴 Red - Error indicators (✗)

## Configuration File

`gx` supports a configuration file for customizing default behavior. The config file is automatically created on first run at:

**Windows:** `C:\Users\YourUsername\.gx\gx.json`
**Linux/Mac:** `~/.gx/gx.json`

### Configuration Options

```json
{
  "default_depth": 3,
  "exclude": {
    "names": ["temp", "logs"],
    "globs": ["*-backup", "archive/*"],
    "regexes": ["^test-.*$", ".*-temp$"]
  }
}
```

#### Fields

- **`default_depth`** (number, default: `3`)
  - Default directory search depth if not specified via command line
  - Can be overridden with `--depth` option

- **`exclude`** (object)
  - **`names`** (array of strings): Directory names to exclude
  - **`globs`** (array of strings): Glob patterns for path matching
  - **`regexes`** (array of strings): Regular expression patterns

#### Exclusion Pattern Examples

**Exclude by directory name:**
```json
{
  "exclude": {
    "names": ["build", "dist", "coverage"]
  }
}
```

**Exclude by full path:**
```json
{
  "exclude": {
    "names": [
      "C:/Users/Name/temp",
      "/home/user/projects/old-project",
      "projects/archive"
    ]
  }
}
```

**Note:** The `names` field supports both directory names and full/relative paths. Path matching works with both `/` and `\` separators on all platforms.

**Exclude by glob patterns:**
```json
{
  "exclude": {
    "globs": [
      "*-backup",
      "archive/*",
      "*/.backup/*"
    ]
  }
}
```

**Exclude by regex:**
```json
{
  "exclude": {
    "regexes": [
      "^test-",
      ".*-temp$",
      "\\d+-backup"
    ]
  }
}
```

**Combined example:**
```json
{
  "default_depth": 5,
  "exclude": {
    "names": ["node_modules", "target", ".venv"],
    "globs": ["*-old", "deprecated/*"],
    "regexes": ["^backup-.*$", ".*-test$"]
  }
}
```

### Priority

Command line options take precedence over config file settings:
```bash
# Uses depth from config file
gx git pull

# Overrides config file depth
gx --depth 10 git pull
```

## Built-in Exclusions

The tool automatically skips these directories (in addition to your config):
- Hidden directories (starting with `.`)
- `node_modules`
- `target`
- `vendor`
- `dist`
- `build`
- `.vscode`
- `.idea`
- `cache`
- `tmp`
- `temp`

## Requirements

- Rust 1.93.1 or later (for building)
- Git (installed and available in PATH)

## Building

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test
```

## License

This project is open source and available under the MIT License.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
