# gitp

K9s-style TUI for managing git operations across multiple repositories.

![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- Browse all git repos in a directory at a glance
- Repo status: clean, dirty, ahead/behind, diverged
- Pull, fetch, diff, checkout across one or all repos
- View commits, GitHub PRs, and Actions runs
- Open repos in browser with one key

## Install

### Homebrew

```bash
brew tap MerzoukeMansouri/gitp
brew install gitp
```

### From source

```bash
cargo build --release
ln -sf $(pwd)/target/release/gitp ~/.local/bin/gitp
```

Add to `PATH` if needed:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## Usage

Run from any directory containing git repositories:

```bash
gitp
gitp ~/Projects       # specify a root directory
```

## Keybindings

Lowercase = current repo. Uppercase = all repos.

| Key | Action |
|-----|--------|
| `↑/↓` or `j/k` | Navigate repos |
| `u` / `U` | Pull (rebase) current / all |
| `f` / `F` | Fetch current / all |
| `l` / `L` | Last 10 commits current / all |
| `d` / `D` | Diff current / all |
| `s` / `S` | Status current / all |
| `c` / `C` | Checkout current / all |
| `a` / `A` | GitHub Actions runs current / all |
| `p` / `P` | GitHub PRs current / all |
| `n` | New PR (`gh pr create`) |
| `o` | Open repo in browser |
| `r` | Refresh |
| `h` | Toggle help |
| `q` | Quit |

> **Note:** GitHub features (`a`, `p`, `n`, `o`) require the [GitHub CLI](https://cli.github.com/) (`gh`) to be installed and authenticated.

## Requirements

- Rust 1.70+
- Git
- [gh](https://cli.github.com/) — for GitHub features (optional)

## Contributing

Issues and PRs welcome.
