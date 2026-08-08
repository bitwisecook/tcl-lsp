# CLI installation

The `tcl` and `f5` CLIs are self-contained native binaries. No Python,
no runtime, no interpreter — download one file per tool and run it.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install/install.sh | sh
```

Works on macOS and Linux (x86_64, arm64, and riscv64 on Linux).
Re-run the same line to update.

To inspect first, or run unattended:

```sh
curl -fsSLo install.sh https://raw.githubusercontent.com/bitwisecook/tcl-lsp/main/scripts/install/install.sh
less install.sh
TCL_LSP_ASSUME_YES=1 sh install.sh
```

The installer picks the binary for your platform, verifies it against the
release `SHA256SUMS`, installs shell completions, and offers to set up the
`tcl-mcp` MCP server and the Claude Code skills if it finds `claude` or
`codex`. Run `sh install.sh --help` for the full env-var list
(`TCL_LSP_ONLY`, `TCL_LSP_NO_MCP`, `TCL_LSP_NO_SKILLS`, `TCL_LSP_NO_PATH`, …).

## Manual install

Release assets are named `<tool>-<target-triple>`, with no version in the
filename. The `f5` CLI ships under its binary name, `f5-query`.

| Platform | Target triple |
|---|---|
| macOS arm64 | `aarch64-apple-darwin` |
| macOS x86_64 | `x86_64-apple-darwin` |
| Linux x86_64 | `x86_64-unknown-linux-gnu` |
| Linux arm64 | `aarch64-unknown-linux-gnu` |
| Linux riscv64 | `riscv64gc-unknown-linux-gnu` |
| Windows x86_64 | `x86_64-pc-windows-msvc` |
| Windows arm64 | `aarch64-pc-windows-msvc` |

For macOS on Apple silicon:

```sh
base=https://github.com/bitwisecook/tcl-lsp/releases/latest/download
curl -fLO "$base/tcl-aarch64-apple-darwin"
curl -fLO "$base/f5-query-aarch64-apple-darwin"
install -m 0755 tcl-aarch64-apple-darwin       ~/.local/bin/tcl
install -m 0755 f5-query-aarch64-apple-darwin  ~/.local/bin/f5
```

The MCP server is published the same way, as `tcl-mcp-<triple>`.

## Verify downloads

```sh
tag="v2.1.16"
curl -fLO "https://github.com/bitwisecook/tcl-lsp/releases/download/$tag/SHA256SUMS"
sha256sum --ignore-missing -c SHA256SUMS \
    || shasum -a 256 --ignore-missing -c SHA256SUMS
```

The installer does this automatically and refuses to install on a mismatch.

## Uninstall

```sh
rm -f ~/.local/bin/tcl ~/.local/bin/f5 ~/.local/bin/tcl-mcp
rm -f ~/.local/share/bash-completion/completions/tcl
rm -f ~/.local/share/bash-completion/completions/f5
rm -f "${ZDOTDIR:-$HOME}/.zsh/completions/_tcl"
rm -f "${ZDOTDIR:-$HOME}/.zsh/completions/_f5"
rm -f ~/.config/fish/completions/tcl.fish ~/.config/fish/completions/f5.fish
rm -rf ~/.claude/skills/irule-* ~/.claude/skills/tcl-* ~/.claude/skills/tk-*
claude mcp remove tcl-lsp 2>/dev/null || true
```

The installer adds a `# Added by tcl-lsp installer` line to your shell rc
file when `~/.local/bin` was not already on `PATH`; remove it by hand if you
no longer want it.

## Build from source

Needs a current Rust stable toolchain (via rustup):

```sh
git clone https://github.com/bitwisecook/tcl-lsp && cd tcl-lsp
make rust-clis          # builds target/release/tcl and target/release/f5-query
```

Build the other binaries the same way: `make rust-server` for
`tcl-lsp-server`, `make rust-mcp` for `tcl-mcp`. Pass `PROFILE=debug` for a
faster, slower-running build.
