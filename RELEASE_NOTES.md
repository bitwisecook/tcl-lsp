# v2.1.1

**2.x alpha — pre-release channel.**

The second pre-release on the **2.x** line, where the ongoing Python → Rust
rewrite of tcl-lsp ships its alphas. It is opt-in: install it from the VS Code
Marketplace **pre-release** channel, or download the pre-release VSIX / native
binaries from this GitHub release. The stable **1.x** line stays the default
for everyone who has not opted into pre-releases, and a `2.1.x` build never
becomes the "latest" GitHub release or the default Marketplace download.

## New Features

- **Native Rust MCP server (`tcl-mcp`).** A single self-contained binary that
  exposes the tcl-lsp analysis engine to Claude Code / Claude Desktop / Codex
  over MCP — no Python, no PyO3. It hosts the full 46-tool surface: analysis,
  diagnostics, LSP features (hover / completion / definition / references /
  rename / code actions / symbols), refactors, docstrings, control-flow &
  data-flow graphs, iRule/BIG-IP tools (`irule_with_context`, `explain_flow`,
  `fakecmp_*`), F5 XC translation, Tk layout, data-group suggestions, and iRule
  test generation. It fully replaces the Python `ai/mcp` server.
- **Prebuilt MCP binaries on the GitHub release.** Per-platform `tcl-mcp`
  binaries are published as release assets (checksum-verified). The installer
  fetches the native binary for your platform by default and registers it with
  Claude Code and Codex; in-repo, `.mcp.json` auto-discovers it.

## Improvements

- **Rust LSP server parity.** Closed 23 VS Code extension parity gaps
  (diagnostics, command handlers, config toggles, completion in command
  contexts, capabilities, code-lens, go-to-definition off-by-ones, …).
- **Tcl 9.1 dialect sync.** Registry + analyser updated for the 9.1 surface —
  Unicode/`timer`/`subst` options and operators, the C99 `expr` math functions
  (TIP 745), and additional 9.1 commands surfaced against the C oracle.
- **Rust-native analysis facades.** The Python AI layer now runs entirely on
  the Rust engine via the `tcl_lsp_py` facades, decoupled from the retiring
  Python compiler/analyser/tooling.
- **Installer.** Native-by-default MCP install with a graceful fallback to the
  Python zipapp on unsupported platforms; detects existing native/Python
  registrations and cleans up superseded installs when switching.

## Bug Fixes

- Analyser false-positive precision fixes and f5-cli/regex residuals.
- Relocated the F5 config/iRule input loader out of `dialects/` to honour the
  layering contract (`dialects/` must not depend on `tooling/`).
- Addressed Codex review findings across the LSP, registry, and installer.

## Using this alpha

Behaviour should match the 1.x stable line. Where it does not, that is a bug —
please file it and note that you are on the 2.x pre-release.
