# pgit

K9s-style TUI for managing git operations across multiple repositories.

![Release](https://img.shields.io/github/v/release/MerzoukeMansouri/pgit)
![Rust](https://img.shields.io/badge/rust-stable-orange?logo=rust&logoColor=white)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- Browse git repos across multiple directories at a glance
- Repo status: clean, dirty, ahead/behind, diverged
- Pull, fetch, discard changes, or run any command across one or all repos
- View commits, GitHub PRs, Actions runs, and security alerts
- Open repos in browser with one key

## Install

### Homebrew

```bash
brew tap MerzoukeMansouri/homebrew
brew install MerzoukeMansouri/homebrew/pgit
```

### Update

```bash
brew update
brew upgrade MerzoukeMansouri/homebrew/pgit
```

### From source

```bash
cargo build --release
ln -sf $(pwd)/target/release/pgit ~/.local/bin/pgit
```

Add to `PATH` if needed:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## Usage

Run from any directory containing git repositories:

```bash
pgit
pgit ~/Projects                    # specify a root directory
pgit ~/Projects ~/work/other-repo  # scan multiple directories / repos at once
```

Each argument can be a directory holding several repos (scanned one level deep) or a single git repo itself — mix and match freely. Duplicates are collapsed.

Paths passed on the command line are saved to `~/.config/pgit/paths.json` (or `$XDG_CONFIG_HOME/pgit/paths.json`). Run `pgit` with no arguments afterwards and it reuses the saved list — no need to retype paths every launch. Pass new paths again to overwrite the saved config.

## Keybindings

Lowercase = current repo. Uppercase = all repos. Click a pane to focus it, click again to open the command input for it.

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate repos |
| `Enter` / `s` | Show repo details (branch, sync, changes, status) |
| `u` / `U` | Pull (rebase) current / all |
| `f` / `F` | Fetch current / all |
| `l` / `L` | Last 10 commits current / all |
| `d` / `D` | Discard changes current / all (asks to confirm) |
| `S` | Status (`-sb`) on all repos |
| `c` / `C` | Run a command current / all (`git ...` by default, prefix with `!` for any binary) |
| `a` / `A` | GitHub Actions runs current / all |
| `p` / `P` | GitHub PRs current / all |
| `x` / `X` | GitHub security alerts current / all |
| `n` | New PR (`gh pr create`) |
| `o` | Open repo in browser |
| `r` | Refresh |
| `h` | Toggle help |
| `q` | Quit |
| `Esc` | Unfocus pane / cancel current mode |

Inside the Actions, PRs, or security alerts views: `↑/↓` to navigate, `Enter`/`o` to open in browser, `Esc`/`q` to close. Actions view also supports `l` to view logs and `R` to rerun a workflow. PR view also supports `/` to filter and `c` to checkout the selected PR's branch.

> **Note:** GitHub features (`a`, `p`, `x`, `n`, `o`) require the [GitHub CLI](https://cli.github.com/) (`gh`) to be installed and authenticated.

## Requirements

- Rust 1.70+
- Git
- [gh](https://cli.github.com/) — for GitHub features (optional)

## Contributing

Issues and PRs welcome.
