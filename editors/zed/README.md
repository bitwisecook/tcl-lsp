# Tcl Language Support for Zed

Tcl, iRules, and iApps language support powered by
[tcl-lsp](https://github.com/bitwisecook/tcl-lsp).

## Features

- **Syntax highlighting** — tree-sitter grammar with semantic token overlay
- **Diagnostics** — errors, warnings, security, taint tracking, style
- **Completions** — commands, subcommands, variables, switches
- **Hover** — command help, proc signatures, variable info
- **Go-to-definition**, **go-to-declaration**, **go-to-implementation**,
  **go-to-type-definition**, and **find references**
- **Document symbols**, **workspace symbols**, and **document highlight**
  (with Read/Write kinds)
- **Formatting** — configurable indent, brace style, line length; format on
  save via Zed's built-in formatter setting or the LSP `willSaveWaitUntil`
  fallback
- **Code actions** — quick fixes for diagnostics
- **Code lens** — inline reference counts on proc definitions
- **Signature help**, **rename**, **folding**, **inlay hints**
- **Linked editing range** — synchronized edits on recursive proc self-calls
- **Call hierarchy** and **type hierarchy** (TclOO classes)
- **Work-done progress** — progress indicator during the first workspace
  scan (requires Zed's workDoneProgress client capability)
- **Snippets** — 16 built-in Tcl and iRules templates
- **AI integration** — slash commands and MCP context server

### Feature availability in Zed

All features listed above are advertised by the language server and
activate automatically when Zed's LSP client requests them.  Workspace file
operations (`workspace/willRenameFiles` /
  `didRenameFiles`) are auto-wired when the client advertises
  `workspace.fileOperations` in its capabilities.  Zed forwards file
  rename events from its project panel when the client capability is
  set; if your Zed version does not yet expose this capability the
  server's hooks remain dormant and you can continue to rename files
  manually without side-effects.

## Supported languages

| Language | Extensions |
|----------|-----------|
| Tcl | `.tcl`, `.tk`, `.itcl`, `.tm`, `.tclspec` |
| iRules | `.irul`, `.irule` |
| iApps | `.iapp`, `.iappimpl`, `.impl` |
| Expect | `.exp` |

Shebang detection: files starting with `#!/usr/bin/tclsh`, `#!/usr/bin/wish`,
or `#!/usr/bin/expect` are recognised as Tcl/Expect.

## Prerequisites

- **Zed** — latest stable release
- **Network access on first use** — to download the native server binary

On first use the extension downloads the native `tcl-lsp-server` and
`tcl-mcp` binaries built for **your** platform from the matching
[GitHub release](https://github.com/bitwisecook/tcl-lsp/releases),
caches them, and runs them directly — no Python, interpreter, or runtime
dependencies are required. (A Zed extension is a single cross-platform
WebAssembly module, so it cannot embed a per-platform binary — the correct
one is fetched at runtime instead.) Dev extension builds fall back to a
`tcl-lsp-server` on your PATH.

See the [Installation Guide](../../INSTALL-editors.md) for full details.

## Installation

### From the Zed extension registry

Search for "Tcl" in the Zed extensions panel and install.

### As a dev extension

1. Open Zed.
2. Open the command palette and run **Extensions: Install Dev Extension**.
3. Point it at this `editors/zed/` directory.

## Settings

Add to your Zed `settings.json` to configure the language server:

```json
{
  "lsp": {
    "tcl-lsp": {
      "settings": {
        "tclLsp": {
          "dialect": "tcl8.6",
          "formatting": {
            "indentSize": 4,
            "indentStyle": "spaces",
            "braceStyle": "k_and_r",
            "maxLineLength": 120,
            "goalLineLength": 100,
            "spaceAfterCommentHash": true,
            "trimTrailingWhitespace": true,
            "ensureFinalNewline": true
          },
          "features": {
            "hover": true,
            "completion": true,
            "diagnostics": true,
            "semanticTokens": true,
            "codeActions": true,
            "definition": true,
            "references": true,
            "documentSymbols": true,
            "folding": true,
            "rename": true,
            "signatureHelp": true,
            "workspaceSymbols": true,
            "inlayHints": false,
            "callHierarchy": true,
            "documentLinks": true,
            "selectionRange": true,
            "documentHighlight": true,
            "codeLens": true,
            "workspaceFileOps": true,
            "willSaveWaitUntil": false,
            "implementation": true,
            "typeDefinition": true,
            "declaration": true,
            "linkedEditingRange": true
          },
          "diagnostics": {
            "W100": true,
            "W111": true
          },
          "optimiser": {
            "enabled": true
          }
        }
      }
    }
  }
}
```

### Dialect options

`tcl8.4`, `tcl8.5`, `tcl8.6`, `tcl9.0`, `tcl9.1`, `f5-irules`, `f5-iapps`,
`f5-tmsh`, `f5-bigip`, `bpf`, `expect`, `spectcl`, `cadence-eda-tcl`,
`intel-quartus-eda-tcl`, `mentor-eda-tcl`, `microchip-libero-eda-tcl`,
`synopsys-eda-tcl`, `xilinx-eda-tcl`

### Full settings reference

See the [VS Code extension
documentation](https://github.com/bitwisecook/tcl-lsp/blob/main/editors/vscode/package.json)
for the complete list of `tclLsp.*` settings — all are supported in Zed via
the `lsp.tcl-lsp.settings` path.

## Snippets

| Prefix | Description |
|--------|------------|
| `tcl-proc` | Tcl procedure |
| `tcl-namespace` | Namespace eval block |
| `tcl-package` | Package provide/require boilerplate |
| `tcl-class` | oo::class definition |
| `tcl-if` | If/else block |
| `tcl-foreach` | Foreach loop |
| `tcl-for` | For loop with braced expressions |
| `tcl-switch` | Switch with `--` option terminator |
| `tcl-catch` | Catch with result/options preservation |
| `tcl-try` | Try/trap block |
| `tcl-dict-for` | Dict iteration |
| `irule-rule-init` | RULE_INIT handler |
| `irule-http-request` | HTTP_REQUEST skeleton |
| `irule-redirect-https` | HTTP to HTTPS redirect |
| `irule-collect-release` | HTTP collect/release pair |
| `irule-class-lookup` | Data-group lookup and routing |

## AI integration

### Slash commands

Use these in Zed's AI Assistant panel:

- `/tcl-doc <command>` — look up Tcl/iRules command documentation
- `/irule-event <event>` — get iRules event reference and valid commands
- `/tcl-validate` — show validation guidance
- `/irule-test` — generate an iRule test script using the Event Orchestrator framework

### MCP context server

The extension registers a **tcl-lsp-mcp** context server that exposes
analysis tools to Zed's Agent panel:

**Analysis:** `analyze`, `validate`, `review`, `convert`, `optimize`

**LSP wrappers:** `hover`, `complete`, `goto_definition`, `find_references`,
`symbols`, `code_actions`, `format_source`, `rename`

**Domain-specific:** `event_info`, `command_info`, `event_order`

**Visualization:** `diagram`, `call_graph`, `symbol_graph`, `dataflow_graph`

**Configuration:** `set_dialect`, `xc_translate`

**iRule testing:**
- `generate_irule_test` — generate a complete test script from iRule source (CFG-informed)
- `irule_cfg_paths` — extract control flow paths to terminal actions for branch coverage analysis
- `fakecmp_which_tmm` — look up which TMM a connection 4-tuple maps to
- `fakecmp_suggest_sources` — find client addr/port combos that hit each TMM

## Troubleshooting

**Server not found (dev extension):** Release builds bundle the
`tcl-lsp-server` binary, so no setup is needed. A dev extension build falls
back to a `tcl-lsp-server` on your PATH — build it with `make rust-server`
(or `cargo build -p tcl-lsp-server`) and put `target/release/tcl-lsp-server`
somewhere on your PATH.

**No diagnostics:** Check that `tclLsp.features.diagnostics` is not set to
`false` in your settings.

## Publishing

The Zed extensions registry pins each extension as a git submodule. For a
new release of tcl-lsp:

```
make publish-zed
```

prepares a local checkout of
[`zed-industries/extensions`](https://github.com/zed-industries/extensions)
with the tcl submodule pointer advanced to the new tag and the
`extensions.toml` version field bumped, staged on a fresh branch. It then
prints the suggested commit / push / `gh pr create` commands and stops —
no push or PR is performed for you. You review the staged diff, then run
the suggested commands to raise the PR yourself.

The initial registration in `zed-industries/extensions` is a separate
one-time PR you raise by hand before this target can work.

## Configuration File

tcl-lsp reads a platform-native configuration file for editor-agnostic
defaults (diagnostics, optimiser, shimmer, features, formatting):

| Platform | Default path |
|----------|-------------|
| Linux / BSD / WSL2 | `~/.config/tcl-lsp/config.ini` |
| macOS | `~/Library/Application Support/tcl-lsp/config.ini` |
| Windows | `%APPDATA%\tcl-lsp\config.ini` |
| MSYS2 / Cygwin | `~/.config/tcl-lsp/config.ini` |

`$XDG_CONFIG_HOME` overrides the default on every platform.

Settings from the config file are applied as baseline defaults.  Zed
`lsp.tcl-lsp.settings` in `settings.json` override the config file — so
you can set shared defaults in the config file and per-project overrides
in Zed.

Use the `tcl-lsp.exportConfig` command via `workspace/executeCommand` to
write current settings to the config file.

See [docs/design/contracts/xdg-config.md](../../docs/design/contracts/xdg-config.md) for
the full reference.
