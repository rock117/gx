# gx

A command-line tool that recursively executes git commands across all git repositories in a directory tree.

## Features

- 🔍 Recursively search for git repositories in current directory and subdirectories
- 🎯 Execute git commands in all found repositories
- 🌲 Display current branch name for each repository
- 🔧 Filter repositories by branch name
- ⚙️ Configurable search depth (default: 3 levels)
- 🚀 Skip common non-project directories (node_modules, target, vendor, etc.)
- 📝 Support for complete git commands with arguments
- 🔮 Dry-run mode to preview operations without executing
- 🛡️ Continue on error by default, `--stop-on-error` to halt
- 📊 Progress indicators and execution statistics
- 📋 Repository info view (branch, status, ahead/behind)
- ⚡ Custom shortcut commands (add, remove, list)
- ⚙️ Hierarchical configuration files (project + user level)

## Usage

### Basic Syntax

```bash
gx <command> [OPTIONS]
```

### Commands

| Command | Description |
|---------|-------------|
| `gx git <cmd> [args]` | Execute git command in all repos |
| `gx info` | Show overview of all repos |
| `gx last` | Show latest commit for each repo |
| `gx log [-<N>]` | Show recent commits for each repo (default: 3, e.g. -5) |
| `gx config` | Show configuration |
| `gx shortcut add <name> "git <cmd>"` | Add a shortcut |
| `gx shortcut rm <name>` | Remove a shortcut |
| `gx shortcut list` | List all shortcuts |
| `gx shortcut clear` | Clear all shortcuts |
| `gx <shortcut> [args]` | Execute via shortcut name |

### Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--depth` | `-d` | Maximum directory depth to search | `3` |
| `--path` | `-p` | Starting directory | Current directory |
| `--branch` | | Only execute in repos matching this branch | - |
| `--dry-run` | | Show what would be done without executing | - |
| `--stop-on-error` | | Stop on first error (default: continue on error) | Continue on error |
| `--remote` | | Show commits from remote tracking branch (fetch first) | Local commits |
| `--help` | `-h` | Show help message | - |

Options can be used with any command. For example:

```bash
gx --depth 5 git pull
gx --branch main info
gx --dry-run git push
```

### Examples

**Execute git commands:**
```bash
gx git pull                        # Pull all repos
gx git push                        # Push all repos
gx git status                      # Status of all repos
gx git fetch --all                 # Fetch all remotes
gx git branch -a                   # List all branches
gx git log -1 --oneline            # Last commit in each repo
gx git diff --stat                 # Diff stats
gx git pull origin main            # Pull from specific remote/branch
```

**With options:**
```bash
gx --depth 5 git pull              # Custom search depth
gx --path /path/to/projects git pull  # Custom starting directory
gx --branch main git pull          # Only repos on main branch
gx --dry-run git push              # Preview without executing
gx --stop-on-error git push        # Stop on first error
```

**Repository info:**
```bash
gx info                            # Show all repos overview
gx info --depth 5                  # With custom depth
gx info --branch main              # Filter by branch
```

**View commits:**
```bash
gx last                            # Show latest commit for each repo
gx last --depth 5                  # With custom depth
gx last --remote                   # Show latest remote commit (fetch first)
gx last --remote --branch main     # Show latest commit on origin/main
gx log                             # Show last 3 commits for each repo
gx log -5                          # Show last 5 commits for each repo
gx log -10                         # Show last 10 commits for each repo
gx log --remote                    # Show last 3 remote commits
gx log -5 --remote                 # Show last 5 remote commits
gx log --since 2026-04-01                      # Commits since date
gx log --until 2026-04-09                       # Commits until date
gx log --author rock                            # Filter by author
gx log -5 --author rock --since 2026-04-01      # Combine filters
```

**Manage shortcuts:**
```bash
gx shortcut add pull "git pull"    # Add shortcut
gx shortcut add st "git status"
gx shortcut add co "git checkout"
gx shortcut list                   # List all shortcuts
gx shortcut rm st                  # Remove shortcut
gx shortcut clear                  # Clear all shortcuts
```

**Use shortcuts:**
```bash
gx pull                            # equivalent to: gx git pull
gx pull origin main                # equivalent to: gx git pull origin main
gx pull push                       # run pull then push sequentially
gx st pull push                    # run status, pull, push sequentially
gx st                              # equivalent to: gx git status
```

**View configuration:**
```bash
gx config
```

### gx vs git

| git | gx | Description |
|---|---|---|
| `git pull` | `gx git pull` | Pull all repos |
| `git push` | `gx git push` | Push all repos |
| `git status` | `gx git status` | Status of all repos |
| `git fetch --all` | `gx git fetch --all` | Fetch all remotes |
| `git branch -a` | `gx git branch -a` | List all branches |
| `git log -1 --oneline` | `gx git log -1 --oneline` | Last commit |
| `git diff --stat` | `gx git diff --stat` | Diff stats |
| - | `gx info` | Show all repos info |
| - | `gx last` | Show latest commit for each repo |
| - | `gx log` | Show last 3 commits for each repo |
| - | `gx log -5` | Show last 5 commits for each repo |
| - | `gx last --remote` | Show latest remote commit for each repo |
| - | `gx log --remote` | Show last 3 remote commits for each repo |
| - | `gx log --author rock` | Filter commits by author |
| - | `gx log --since 2026-04-01 --until 2026-04-09` | Filter commits by date range |
| - | `gx --branch main git pull` | Pull only `main` branch repos |
| - | `gx --dry-run git push` | Preview without executing |
| - | `gx --stop-on-error git push` | Stop on first error |
| - | `gx pull push` | Run multiple commands sequentially |
| - | `gx config` | View configuration |

### Passing Options to Git

If you need to pass options that conflict with gx options (like `-h`), use `--` as a separator:

```bash
# Show git help (not gx help)
gx -- git -h
```

| Command | Behavior |
|---|---|
| `gx -h` | Show gx help |
| `gx -- git -h` | Pass `-h` to git |

## Output

### Repository Info (`gx info`)

```bash
gx info
```

Displays a summary of all repositories:

```
  📁 project1    main       ✓ clean     ↑0 ↓0
  📁 project2    dev        ⚠ dirty     ↑2 ↓0
  📁 project3    feature/x  ✓ clean     ↑0 ↓3
  📁 project4    main       ✗ detached

  Total: 4 repos | 1 dirty | 1 ahead | 1 behind
```

**Column descriptions:**
- **Branch** - Current branch name (cyan) or `detached` (gray) for detached HEAD
- **Status** - `✓ clean` (green) if no changes, `⚠ dirty` (yellow) if there are changes, `✗ detached` (red) if detached HEAD
- **Sync** - `↑N ↓N` showing ahead/behind commits relative to upstream (hidden if up-to-date)

### Command Execution

The tool displays:
- 🔍 Search directory and depth
- 📁 Each git repository found with current branch (color-coded)
- 📊 Progress counter [X/total] for each repository
- 📋 Git command output from each repository
- 📈 Execution summary (total processed, succeeded, failed)
- ⚠️ Any errors encountered (with color-coded indicators)

Example output:
```
Searching for git repositories in: ./projects
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

`gx` supports hierarchical configuration files for flexible customization:

### Configuration Levels

Two configuration levels are supported (higher priority overrides lower):

1. **Project-level config** - `.gx/gx.json` (current directory)
   - Project-specific settings
   - Overrides user-level config
   - Not auto-created (must be created manually)

2. **User-level config** - `~/.gx/gx.json` (home directory)
   - Global defaults for all projects
   - Auto-created on first run
   - Used as fallback

### Configuration Priority

**From highest to lowest:**
1. Command-line arguments (`--depth`, `--path`)
2. Project-level config (`.gx/gx.json`)
3. User-level config (`~/.gx/gx.json`)
4. Default values (`depth: 3`, empty exclude patterns)

### Configuration Merging Rules

- **Simple values** (like `default_depth`): Higher level overrides lower level
- **Arrays** (like `exclude.names`): Merged and deduplicated
- **Shortcuts** (like `shortcuts`): Merged, project-level overrides user-level for same name

### Configuration Options

```json
{
  "default_depth": 3,
  "exclude": {
    "names": ["temp", "logs"],
    "globs": ["*-backup", "archive/*"],
    "regexes": ["^test-.*$", ".*-temp$"]
  },
  "shortcuts": {
    "pull": "git pull",
    "push": "git push",
    "st": "git status",
    "co": "git checkout"
  }
}
```

#### Fields

- **`default_depth`** (number, default: `3`)
  - Default directory search depth if not specified via command line
  - Can be overridden with `--depth` option

- **`exclude`** (object)
  - **`names`** (array of strings): Directory names or full paths to exclude
  - **`globs`** (array of strings): Glob patterns for path matching
  - **`regexes`** (array of strings): Regular expression patterns

- **`shortcuts`** (object)
  - Key-value pairs mapping shortcut name to git command
  - Can be managed via CLI: `gx shortcut add/rm/list`
  - Example: `"pull": "git pull"` allows `gx pull` instead of `gx git pull`

#### Exclusion Pattern Examples

**Exclude by directory name or full path:**
```json
{
  "exclude": {
    "names": ["build", "C:/Users/Name/temp", "projects/archive"]
  }
}
```

**Exclude by glob patterns:**
```json
{
  "exclude": {
    "globs": ["*-backup", "archive/*", "*/.backup/*"]
  }
}
```

**Exclude by regex:**
```json
{
  "exclude": {
    "regexes": ["^test-", ".*-temp$", "\\d+-backup"]
  }
}
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

## License

This project is open source and available under the MIT License.
