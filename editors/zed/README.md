# Tcl Language Support for Zed

Tcl, iRules, and iApps language support powered by
[tcl-lsp](https://github.com/bitwisecook/tcl-lsp).

## Features

- **Syntax highlighting** — tree-sitter grammar with semantic token overlay
- **Diagnostics** — errors, warnings, security, taint tracking, style
- **Completions** — commands, subcommands, variables, switches
- **Hover** — command help, proc signatures, variable info
- **Go-to-definition** and **find references**
- **Document symbols** and **workspace symbols**
- **Formatting** — configurable indent, brace style, line length
- **Code actions** — quick fixes for diagnostics
- **Signature help**, **rename**, **folding**, **inlay hints**
- **Snippets** — 16 built-in Tcl and iRules templates
- **AI integration** — slash commands and MCP context server

## Supported languages

| Language | Extensions |
|----------|-----------|
| Tcl | `.tcl`, `.tk`, `.itcl`, `.tm` |
| iRules | `.irul`, `.irule` |
| iApps | `.iapp`, `.iappimpl`, `.impl` |

Shebang detection: files starting with `#!/usr/bin/tclsh` or `#!/usr/bin/wish`
are recognised as Tcl.

## Prerequisites

- **Python 3.10+** — the extension auto-discovers `python3.10`–`python3.15`
  on your PATH
- **Zed** — latest stable release

The `tcl-lsp-server.pyz` zipapp is downloaded automatically from GitHub
releases on first use. You can also place it in your workspace root or PATH
for offline use.

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
            "formatting": true,
            "semanticTokens": true,
            "codeActions": true,
            "definition": true,
            "references": true,
            "documentSymbols": true,
            "folding": true,
            "rename": true,
            "signatureHelp": true,
            "workspaceSymbols": true,
            "inlayHints": true,
            "callHierarchy": true,
            "documentLinks": true,
            "selectionRange": true
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

`tcl8.4`, `tcl8.5`, `tcl8.6`, `tcl9.0`, `f5-irules`, `f5-iapps`,
`f5-bigip`, `eda-tools`

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

**Python not found:** Ensure `python3` (3.10+) is on your PATH. The extension
tries versioned binaries (`python3.13`, `python3.12`, etc.) before falling
back to `python3`.

**Server download fails:** Check your network connection. The extension
downloads from GitHub releases. You can manually download `tcl-lsp-server.pyz`
and place it on your PATH.

**No diagnostics:** Check that `tclLsp.features.diagnostics` is not set to
`false` in your settings.

## Publishing

To publish to the Zed extension registry:

1. Push this directory to a public GitHub repo.
2. Fork [zed-industries/extensions](https://github.com/zed-industries/extensions).
3. Add the repo as a submodule and update `extensions.toml`.
4. Submit a PR.
