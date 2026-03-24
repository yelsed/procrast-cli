# procrast-cli

A Rust CLI and TUI for browsing your [Procrast](https://github.com/yelsed/procrast) ideas. Includes a Claude Code plugin so Claude can search and read your ideas directly in conversation.

## Prerequisites

- A Procrast account

## Install the CLI

### Download prebuilt binary (recommended)

Download the latest release for your platform from [GitHub Releases](https://github.com/yelsed/procrast-cli/releases), extract it, and place the binary in your PATH:

```bash
# macOS (Apple Silicon)
curl -L https://github.com/yelsed/procrast-cli/releases/latest/download/procrast-cli-aarch64-apple-darwin.tar.gz | tar xz
sudo mv procrast-cli /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/yelsed/procrast-cli/releases/latest/download/procrast-cli-x86_64-apple-darwin.tar.gz | tar xz
sudo mv procrast-cli /usr/local/bin/

# Linux
curl -L https://github.com/yelsed/procrast-cli/releases/latest/download/procrast-cli-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv procrast-cli /usr/local/bin/
```

### Build from source

Requires [Rust](https://rustup.rs):

```bash
cargo install --git https://github.com/yelsed/procrast-cli
```

## Authenticate

```bash
procrast-cli login
```

You'll be prompted for your email and password. The auth token is stored in your OS keyring (macOS Keychain, Linux secret-service, Windows Credential Manager).

## CLI Usage

```bash
# List your ideas
procrast-cli list
procrast-cli list --limit 10 --hide-done
procrast-cli list --json

# Show a single idea (full UUID or prefix)
procrast-cli show abc123
procrast-cli show abc123 --markdown
procrast-cli show abc123 --json

# Search ideas
procrast-cli search "my query"
procrast-cli search "my query" --json

# Export an idea as a markdown file
procrast-cli export abc123
procrast-cli export abc123 --output my-idea.md

# Log out
procrast-cli logout
```

## TUI

Launch the interactive terminal UI:

```bash
procrast-cli
# or
procrast-cli tui
```

**Keybindings:**

| Key | Action |
|-----|--------|
| `j` / `Down` | Next item |
| `k` / `Up` | Previous item |
| `Enter` | View idea details |
| `/` | Search |
| `y` | Copy as markdown (in detail view) |
| `r` | Retry connection |
| `l` | Re-login (when offline) |
| `q` | Quit / Back |

The TUI works offline using a local SQLite cache.

## Claude Code Plugin

The plugin lets Claude browse, search, and read your Procrast ideas directly in conversation.

### Install

```bash
# Add the marketplace (one-time)
claude plugin marketplace add yelsed/procrast-cli

# Install the plugin
claude plugin install procrast
```

Restart Claude Code after installing. The plugin will automatically download the CLI binary on first use — no Rust or build tools required.

### What it provides

**Tools** (Claude can call these):

| Tool | Description |
|------|-------------|
| `list_ideas` | List ideas with titles, UUIDs, priorities |
| `show_idea` | Show full details of an idea by UUID |
| `search_ideas` | Full-text search across ideas |
| `export_idea` | Get an idea as formatted markdown |
| `check_auth` | Check if authenticated |

**Resources** — Ideas are also available as MCP resources at `procrast://ideas/{uuid}`, so Claude can read them like files.

### Uninstall

```bash
claude plugin uninstall procrast
claude plugin marketplace remove procrast
```

## Configuration

The CLI uses the Procrast API at `https://procrastination-station-xwi7hrqi.on-forge.com/api` by default. To override:

```bash
# Via environment variable
export PROCRAST_API_URL=https://your-server.com/api

# Or via config file at ~/.config/procrast-cli/config.toml
api_url = "https://your-server.com/api"
```

## License

MIT
