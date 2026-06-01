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
| `p` / `P` | Pull current / all |
| `f` / `F` | Fetch current / all |
| `l` | Last 10 commits |
| `d` | Diff |
| `s` | Detailed status |
| `r` | Refresh |
| `q` | Quit |
