# gitp

K9s-style TUI for managing git operations across multiple repositories.

## Install

```bash
cargo build --release
ln -sf $(pwd)/target/release/gitp ~/.local/bin/gitp
```

Ensure `~/.local/bin` is in `PATH`:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## Usage

Run from any directory containing git repositories:

```bash
gitp
```

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate repos |
| `u` / `U` | Pull (rebase) current / all |
| `f` / `F` | Fetch current / all |
| `l` / `L` | Last 10 commits current / all |
| `d` / `D` | Diff current / all |
| `s` | Detailed status |
| `S` | Status all |
| `c` / `C` | Checkout current / all |
| `a` / `A` | GitHub Actions runs current / all |
| `p` / `P` | GitHub PRs current / all |
| `n` | New PR (gh pr create) |
| `o` | Open repo in browser |
| `r` | Refresh |
| `h` | Toggle help |
| `q` | Quit |
