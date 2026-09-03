---
name: lsp-client
description: >
  Use when verifying Tcl LSP server behaviour: semantic tokens, diagnostics,
  formatting, hover, completion, definition, references, code lenses, code
  actions, optimisations, document symbols, diagram extraction, event/command
  registry lookups, or benchmarking server performance and collecting timing
  logs. Drives the native server directly over JSON-RPC without an editor.
allowed-tools: Bash, Read
---

# Tcl LSP Client

Starts the native `tcl-lsp-server`, exercises one feature against a file, and
prints readable results. Run from the repo root:

```bash
python3 .claude/skills/lsp-client/lsp_client.py [--server-bin PATH] [--scan-timeout S] [--also-open FILE]... <subcommand> <args...>
```

The server is found under `target/{release,debug}/` — build with
`make rust-server` or pass `--server-bin`. Top-level options go *before* the
subcommand. All line/col arguments are **0-based**.

## Subcommands

| Subcommand | Arguments | Shows |
|---|---|---|
| `semantic-tokens` | `<file>` | every token as `line:col type "text"` |
| `diagnostics` | `<file>` | `SEVERITY CODE l:c-l:c message`; optimiser O-codes are INFO with quick-fix actions |
| `format` | `<file>` | the edits the formatter would apply |
| `hover` / `completion` / `definition` / `references` | `<file> <line> <col>` | the feature at a position |
| `code-lens` | `<file>` | reference-count lenses, each resolved via `codeLens/resolve` as an editor would; `[inert — empty command id]` marks the clickable-broken shape (#724 / #956) |
| `code-actions` | `<file> <l> <c> <el> <ec>` | code actions in a range |
| `optimize` | `<file>` | each rewrite and the full optimised source |
| `symbols` | `<file>` | the document symbol hierarchy (events, procs, namespaces, variables) |
| `diagram` | `<file>` | control-flow diagram data from the IR |
| `event-info` / `command-info` | `<NAME>` | iRules registry metadata; no file needed |
| `context` | `<file>` | dialect + diagnostics + symbols + event metadata, the pack an AI skill consumes |
| `all` | `<file>` | tokens + diagnostics + symbols + format + optimize |
| `bench` | `<file> [--iterations N]` | wall clock request→response, token count, server `[timing]` breakdown, then a mid-file edit for incremental cost |
| `logs` | `<file> [--timing-only]` | server stderr and `window/logMessage`; `--timing-only` keeps `[timing]` lines |

## Cross-file assertions

`definition`, `references`, `diagnostics`, `code-actions`, `context`, `all`,
`completion`, and `code-lens` wait for the background workspace scan
(`--scan-timeout`, default 30 s) before proceeding; otherwise cross-file
results race the scan (#1094). A new cross-file check must call
`client.wait_for_workspace_scan()` *before* `didOpen` — its docstring says
why. `--also-open FILE` (repeatable) opens companion files after that wait
and before `<file>` (#1111):

```bash
python3 .claude/skills/lsp-client/lsp_client.py --also-open lib.tcl definition consumer.tcl 3 10
```

## When to use

After changing tokens, diagnostics, the formatter, or the optimiser; to check
a position-dependent feature at a cursor; `all` as a smoke test; `bench` and
`logs --timing-only` after a performance change.

```bash
python3 .claude/skills/lsp-client/lsp_client.py semantic-tokens samples/for_screenshots/03-completions.tcl
python3 .claude/skills/lsp-client/lsp_client.py diagnostics editors/vscode/testFixture/diagnostics.tcl
python3 .claude/skills/lsp-client/lsp_client.py hover editors/vscode/testFixture/procs.tcl 1 6
python3 .claude/skills/lsp-client/lsp_client.py code-lens editors/vscode/testFixture/objMethodDispatch.tcl
python3 .claude/skills/lsp-client/lsp_client.py diagram samples/for_screenshots/ai-scene.irul
python3 .claude/skills/lsp-client/lsp_client.py event-info HTTP_REQUEST
python3 .claude/skills/lsp-client/lsp_client.py bench samples/tcl/09_long_code.tcl --iterations 3
```

$ARGUMENTS
