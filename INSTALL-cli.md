# CLI installation

The `tcl` and `f5` CLIs are self-contained native binaries: download one
file per tool and run it.

## Install

```sh
curl -fsSL https://github.com/bitwisecook/tcl-lsp/releases/latest/download/install.sh | sh
```

Works on macOS and glibc-based Linux (x86_64, arm64, and riscv64 on Linux).
The x86_64 and arm64 binaries require glibc 2.28 or newer; RISC-V requires
glibc 2.35 or newer. Alpine and other non-glibc systems build the native CLIs
from source. Re-run the same line to update; set `TCL_LSP_VERSION=vX.Y.Z` to
pin a release.

To inspect first, or run unattended:

```sh
curl -fsSLo install.sh https://github.com/bitwisecook/tcl-lsp/releases/latest/download/install.sh
less install.sh
TCL_LSP_ASSUME_YES=1 sh install.sh
```

The installer picks the binary for your platform, verifies it against the
release `SHA256SUMS`, installs shell completions, and offers each detected AI
harness its own `tcl-mcp` registration. Claude Code, Codex, Gemini CLI, GitHub
Copilot CLI, OpenCode, Hermes, Goose, and Bobbit are recognised. Harnesses with
project files offer project or user scope; otherwise registration is user-level.
Bobbit is project-only because it discovers the project-root `.mcp.json`.
Claude Code skills are offered separately. Run `sh install.sh --help` for the full env-var list
(`TCL_LSP_ONLY`, `TCL_LSP_NO_MCP`, `TCL_LSP_NO_SKILLS`, `TCL_LSP_NO_PATH`, …).

### Upgrading from a 1.x install

The installer removes a 1.x Python install it positively identifies — the
`tcl` / `f5` zipapps, `tcl-lsp-mcp-server.pyz`, `argcomplete` scripts, and
stale Claude Code or Codex MCP registrations — and backs up the old Claude
bundle under `~/.claude/.tcl-lsp-python-backup-*`. Shared packages (Python,
Tcl, `curl`, …) are left alone. Set `TCL_LSP_NO_LEGACY_CLEANUP=1` to keep the
old install.

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

## No prebuilt binary for your platform?

The `tcl`, `f5-query`, and `tcl-mcp` CLIs are native-only — build them from
source (see [Build from source](#build-from-source)).

The **language server** is not. Every release also carries
`tcl-lsp-server-wasi.wasm`, the same server compiled to WebAssembly (WASI). It
speaks ordinary stdio LSP, so any editor with a generic LSP client can run it
under a WebAssembly runtime:

```sh
base=https://github.com/bitwisecook/tcl-lsp/releases/latest/download
curl -fLO "$base/tcl-lsp-server-wasi.wasm"
wasmtime run --dir /path/to/project tcl-lsp-server-wasi.wasm
```

`--dir` grants the server a directory to read, and the path must be **absolute**:
editors send absolute `file:///…` URIs, and a relative preopen such as `--dir .`
matches none of them, so the server silently sees no files.
[INSTALL-editors.md](INSTALL-editors.md#no-prebuilt-binary-for-your-platform)
has the Helix and Neovim configurations.

## Verify downloads

```sh
curl -fLO https://github.com/bitwisecook/tcl-lsp/releases/latest/download/SHA256SUMS
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
rm -rf ~/.claude/skills/ai-help ~/.claude/skills/bigip-cleanup ~/.claude/skills/explain-flow
rm -rf ~/.claude/skills/f5-query ~/.claude/skills/spec-author
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
