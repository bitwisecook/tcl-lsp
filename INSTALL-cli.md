# CLI installation

The `tcl` and `f5` CLIs are self-contained Python zipapps. Needs Python 3.10+.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install.sh | sh
```

Works on macOS, Debian/Ubuntu, RHEL/Rocky/Alma, Fedora, Arch, Alpine.
Re-run the same line to update.

To inspect first, or run unattended:

```sh
curl -fsSLo install.sh https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install.sh
less install.sh
TCL_LSP_ASSUME_YES=1 sh install.sh
```

Run `sh install.sh --help` for the full env-var list.

## Manual install

```sh
# Replace <version> with the tag from the release page.
curl -fLO https://github.com/bitwisecook/tcl-lsp/releases/latest/download/tcl-<version>.pyz
curl -fLO https://github.com/bitwisecook/tcl-lsp/releases/latest/download/f5-<version>.pyz
install -m 0755 tcl-<version>.pyz ~/.local/bin/tcl
install -m 0755 f5-<version>.pyz  ~/.local/bin/f5
```

## Verify downloads

```sh
tag="v1.9.0"
curl -fLO "https://github.com/bitwisecook/tcl-lsp/releases/download/$tag/SHA256SUMS"
sha256sum --ignore-missing -c SHA256SUMS \
    || shasum -a 256 --ignore-missing -c SHA256SUMS
```

The installer does this automatically.

## Uninstall

```sh
rm -f ~/.local/bin/{tcl,f5,tcl-lsp-mcp-server.pyz}
rm -f ~/.local/share/bash-completion/completions/{tcl,f5}
rm -f "${ZDOTDIR:-$HOME}/.zsh/completions/_"{tcl,f5}
rm -f ~/.config/fish/completions/{tcl,f5}.fish
rm -rf ~/.claude/skills/{irule,tcl,tk}-* ~/.claude/tcl-ai.pyz ~/.claude/prompts
claude mcp remove tcl-lsp 2>/dev/null || true
```

## Build from source

```sh
git clone https://github.com/bitwisecook/tcl-lsp && cd tcl-lsp
uv sync --extra dev
make zipapp-tcl zipapp-f5
```
