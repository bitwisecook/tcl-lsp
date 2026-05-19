# CLI installation

The `tcl` and `f5` CLIs are self-contained Python zipapps. Needs Python 3.10+.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install/install.sh | sh
```

Works on macOS, Debian/Ubuntu, RHEL/Rocky/Alma, Fedora, Arch, Alpine.
Re-run the same line to update.

To inspect first, or run unattended:

```sh
curl -fsSLo install.sh https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install/install.sh
less install.sh
TCL_LSP_ASSUME_YES=1 sh install.sh
```

Run `sh install.sh --help` for the full env-var list.

## Manual install

Asset names use the semver without the `v` prefix — tag `v1.9.0` ships
`tcl-1.9.0.pyz` and `f5-1.9.0.pyz`. Replace `1.9.0` below with the
version from the release page.

```sh
curl -fLO https://github.com/bitwisecook/tcl-lsp/releases/latest/download/tcl-1.9.0.pyz
curl -fLO https://github.com/bitwisecook/tcl-lsp/releases/latest/download/f5-1.9.0.pyz
install -m 0755 tcl-1.9.0.pyz ~/.local/bin/tcl
install -m 0755 f5-1.9.0.pyz  ~/.local/bin/f5
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
rm -f ~/.local/bin/tcl ~/.local/bin/f5 ~/.local/bin/tcl-lsp-mcp-server.pyz
rm -f ~/.local/share/bash-completion/completions/tcl
rm -f ~/.local/share/bash-completion/completions/f5
rm -f "${ZDOTDIR:-$HOME}/.zsh/completions/_tcl"
rm -f "${ZDOTDIR:-$HOME}/.zsh/completions/_f5"
rm -f ~/.config/fish/completions/tcl.fish ~/.config/fish/completions/f5.fish
rm -rf ~/.claude/skills/irule-* ~/.claude/skills/tcl-* ~/.claude/skills/tk-*
rm -rf ~/.claude/tcl-ai.pyz ~/.claude/prompts
claude mcp remove tcl-lsp 2>/dev/null || true
```

## Build from source

```sh
git clone https://github.com/bitwisecook/tcl-lsp && cd tcl-lsp
uv sync --extra dev
make zipapp-tcl zipapp-f5
```
