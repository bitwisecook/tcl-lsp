---
name: lsp-client
description: >
  Use when verifying Tcl LSP server behavior: semantic tokens, diagnostics,
  formatting, hover, completion, definition, references, code lenses, code
  actions, optimizations, document symbols, diagram extraction, event/command
  registry lookups, or benchmarking server performance and collecting timing
  logs. Tests the server directly over JSON-RPC without VS Code.
allowed-tools: Bash, Read
---

# Tcl LSP Client

Runs a standalone LSP client that starts the Tcl language server, exercises
an LSP feature against a Tcl file, and prints human-readable results.
Use this to verify server behavior after making changes.

## Usage

Run from the project root (the git worktree directory):

```bash
python3 .claude/skills/lsp-client/lsp_client.py <subcommand> <args...>
```

The script auto-detects the `tcl-lsp/` server directory.  Override with
`--server-dir /path/to/tcl-lsp` if needed.

## Subcommands

| Subcommand | Arguments | What it does |
|---|---|---|
| `semantic-tokens` | `<file.tcl>` | Decode and display all semantic tokens with types |
| `diagnostics` | `<file.tcl>` | Show warnings, errors, hints from the analyzer and optimizer |
| `format` | `<file.tcl>` | Show formatting edits the server would apply |
| `hover` | `<file.tcl> <line> <col>` | Show hover info at a 0-based position |
| `completion` | `<file.tcl> <line> <col>` | Show completions at a 0-based position |
| `definition` | `<file.tcl> <line> <col>` | Show go-to-definition locations |
| `references` | `<file.tcl> <line> <col>` | Show all reference locations |
| `code-lens` | `<file.tcl>` | Show code lenses (reference-count lenses), auto-resolving each via `codeLens/resolve` |
| `code-actions` | `<file.tcl> <l> <c> <el> <ec>` | Show code actions in a 0-based range |
| `optimize` | `<file.tcl>` | Show optimization suggestions and rewritten source |
| `symbols` | `<file.tcl>` | Show document symbol hierarchy (procs, events, namespaces, variables) |
| `diagram` | `<file.tcl>` | Extract control flow diagram data from compiler IR |
| `event-info` | `<EVENT_NAME>` | Show iRules event registry metadata (no file needed) |
| `command-info` | `<COMMAND_NAME>` | Show iRules command registry metadata (no file needed) |
| `context` | `<file.tcl>` | Build context pack: diagnostics + symbols + event metadata |
| `all` | `<file.tcl>` | Run semantic-tokens + diagnostics + symbols + format + optimize together |
| `bench` | `<file.tcl> [--iterations N]` | Benchmark time-to-semantic-tokens with server timing breakdown |
| `logs` | `<file.tcl> [--timing-only]` | Collect and display server logs and timing information |

All line/col arguments are **0-based**, matching the LSP protocol.

### Cross-file assertions

`definition`, `references`, `diagnostics`, `code-actions`, `context`,
`all`, `completion`, and `code-lens` wait for the server's background
workspace scan to finish before
proceeding (bounded by `--scan-timeout`, default 30s — see below) —
otherwise cross-file results (workspace variables, package tiers,
cross-file definition/references, sibling-file completions,
workspace-wide lens counts) are racy depending on scan timing (issue #1094).
Other subcommands are single-file and unaffected. If you add a new
cross-file check, call `client.wait_for_workspace_scan()` yourself before
opening the document(s) it depends on — see the method's docstring in
`lsp_client.py` for the exact signal it waits on and why waiting *before*
`didOpen` matters for diagnostics specifically.

**Multi-file helper (`--also-open`):** pass `--also-open FILE` (repeatable,
any subcommand) to open one or more companion files before the main
`<file>` argument — the first-class "open two files and assert" helper
(issue #1111). Companion files open *after* the workspace-scan wait above
and *before* `<file>`, so whichever subcommand you run sees them:

```bash
python3 .claude/skills/lsp-client/lsp_client.py --also-open lib.tcl \
    definition consumer.tcl 3 10
```

(`--also-open`, like `--scan-timeout` and `--server-bin`, is a top-level
option — it must come *before* the subcommand name, not after.)

This opens `lib.tcl`, waits the usual workspace-scan barrier (since
`definition` is a cross-file command), then opens `consumer.tcl` and asks
for the definition at `3:10` — the pattern the docstring above used to ask
callers to hand-roll with `wait_for_workspace_scan()` + two `open_document()`
calls.

## Interpreting Output

### Semantic Tokens
Each token shows `line:col type "text"`:
```
  0:0  keyword      "set"
  0:4  string       "name"
  1:0  keyword      "puts"
  1:5  variable     "$name"
```
Token types: keyword, function, variable, string, comment, number,
operator, parameter, namespace, regexp.

### Diagnostics
```
  WARNING  W100  2:4-2:18  Unbraced expr in 'if' condition
  HINT     W302  5:0-5:12  catch without result variable
  INFO     O101  3:10-3:18  Fold constant expression
```
Optimizer suggestions (O100, O101, O102) appear as INFO-level diagnostics
with QuickFix code actions in VS Code.

### Optimizations
```
=== Optimizations (2 items) ===
  O101   2:10-2:18  Fold constant expression  →  "{3}"
  O102   3:8-3:27   Fold constant expr command substitution  →  "3"

=== Optimized Source ===
    set a 1
    set b 3
    ...
```
Shows each optimization and the fully rewritten source.

### Symbols
```
=== Symbol Definitions (4 symbols) ===
  Event HTTP_REQUEST (line 1)
  Event HTTP_RESPONSE (line 15)
  Function my_proc (a b) (line 25)
    Variable result (line 26)
```
Hierarchical display of events, procs, namespaces, and variables.

### Diagram Data
Shows events in canonical firing order with multiplicity and priority,
procedures with parameters, and the full structured JSON for diagram
generation.

### Event Info
```
=== Event Info ===
  Event: HTTP_REQUEST
  Known: yes
  Deprecated: no
  Valid commands: 87
  Sample commands: HTTP::host, HTTP::path, HTTP::method, ...
```

### Command Info
```
=== Command Info ===
  Command: HTTP::uri
  Summary: Returns or sets the HTTP URI
  Synopsis: HTTP::uri [<uri>]
  Valid in: HTTP_REQUEST, HTTP_RESPONSE, ...
```

### Context Pack
Combined output for AI skill consumption:
```
=== Context Pack ===
  Dialect: f5-irules
  File: redirect.tcl
  Lines: 42

=== Diagnostics (2) ===
  WARNING W100 line 15: unbraced expression in expr
  WARNING W304 line 22: missing -- option terminator

=== Symbol Definitions (3) ===
  Event HTTP_REQUEST (line 1)
  Event HTTP_RESPONSE (line 15)
  Function my_proc (a b) (line 25)

=== Event Metadata (2 events, in source order) ===
  HTTP_REQUEST: known=yes, deprecated=no, validCommands=87
    sample: HTTP::host, HTTP::path, HTTP::method, HTTP::uri
  HTTP_RESPONSE: known=yes, deprecated=no, validCommands=45
    sample: HTTP::header, HTTP::status
```

### Hover
Shows the markdown content the editor would display on hover.

### Completions
Shows label, kind (Keyword, Function, Variable), and detail.

### Code Lenses
Shows every reference-count lens, resolved.  Proc / class / method /
classmethod lenses are all returned lazily (range + `data`, no `command`)
so the server can recompute the count at resolve time — `code-lens` calls
`codeLens/resolve` on each one automatically, the way a real editor does,
rather than showing the raw unresolved list:
```
=== Code Lenses (2 lenses) ===
  0:17-0:20  '1 reference'  [tcl-lsp.showReferences, 3 args]
  6:11-6:14  '1 reference'  [tcl-lsp.showReferences, 3 args]
```
A lens whose `command` carries an empty command id renders as
`[inert — empty command id]` — a lens in that shape is clickable-broken
(the "reference is not active" defect, #724 / #956): it shows a count but
does nothing when clicked.

### Bench
Measures wall-clock time from request to semantic token response, plus
server-side timing breakdown from `[timing]` log entries:
```
=== Benchmark ===
  File: 09_long_code.tcl
  Lines: 540, Size: 15208 bytes
  Iterations: 3

  Iteration 1:
    Wall clock (request → response): 445.1ms
    Tokens: 1557
    Server timings:
      _build_full_chunk_caches: 1ms
      semantic_tokens_full: 200ms
      semanticTokens/full: 200ms
      workspace_state.update: 459ms

  After mid-file edit:
    Wall clock: 180.5ms
    Server timings:
      workspace_state.update: 150ms
```
Use `--iterations N` to run multiple open/close cycles for reliable medians.
The edit benchmark simulates a mid-file change to measure incremental
update performance.

### Logs
Shows server stderr and `window/logMessage` notifications. Use `--timing-only`
to filter to just `[timing]` entries for performance analysis.

## When to Use

- After changing semantic token logic, verify tokens are correct
- After changing diagnostic checks, verify warnings appear
- After changing the formatter, verify edit output
- After changing the optimizer, verify suggestions and rewrites
- Quick smoke test with `all` on any example file
- Verify position-dependent features (hover, completion, definition)
  at specific cursor locations
- Use `context` to build rich LSP context for AI skill consumption
- Use `diagram` to extract structured flow data for Mermaid generation
- Use `event-info` / `command-info` to look up iRules registry metadata
- After performance changes, use `bench` to measure time-to-semantic-tokens
- Use `logs --timing-only` to see the server-side timing breakdown

## Example Invocations

```bash
python3 .claude/skills/lsp-client/lsp_client.py semantic-tokens tcl-lsp/samples/for_screenshots/03-completions.tcl
python3 .claude/skills/lsp-client/lsp_client.py diagnostics tcl-lsp/editors/vscode/testFixture/diagnostics.tcl
python3 .claude/skills/lsp-client/lsp_client.py hover tcl-lsp/editors/vscode/testFixture/procs.tcl 1 6
python3 .claude/skills/lsp-client/lsp_client.py code-lens tcl-lsp/editors/vscode/testFixture/objMethodDispatch.tcl
python3 .claude/skills/lsp-client/lsp_client.py optimize tcl-lsp/samples/for_screenshots/22-optimiser-before.tcl
python3 .claude/skills/lsp-client/lsp_client.py symbols tcl-lsp/samples/for_screenshots/ai-scene.irul
python3 .claude/skills/lsp-client/lsp_client.py diagram tcl-lsp/samples/for_screenshots/ai-scene.irul
python3 .claude/skills/lsp-client/lsp_client.py event-info HTTP_REQUEST
python3 .claude/skills/lsp-client/lsp_client.py command-info HTTP::uri
python3 .claude/skills/lsp-client/lsp_client.py context tcl-lsp/samples/for_screenshots/ai-scene.irul
python3 .claude/skills/lsp-client/lsp_client.py all tcl-lsp/samples/for_screenshots/03-completions.tcl
python3 .claude/skills/lsp-client/lsp_client.py bench tcl-lsp/samples/tcl/09_long_code.tcl --iterations 3
python3 .claude/skills/lsp-client/lsp_client.py logs tcl-lsp/samples/tcl/09_long_code.tcl --timing-only
python3 .claude/skills/lsp-client/lsp_client.py --also-open tcl-lsp/samples/tcl/lib.tcl definition tcl-lsp/samples/tcl/consumer.tcl 3 10
```

$ARGUMENTS
