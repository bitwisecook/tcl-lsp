---
name: lsp-client
description: >
  Use when verifying Tcl LSP server behavior: semantic tokens, diagnostics,
  formatting, hover, completion, definition, references, code actions,
  optimizations, document symbols, diagram extraction, or event/command registry lookups.
  Tests the server directly over JSON-RPC without VS Code.
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
| `code-actions` | `<file.tcl> <l> <c> <el> <ec>` | Show code actions in a 0-based range |
| `optimize` | `<file.tcl>` | Show optimization suggestions and rewritten source |
| `symbols` | `<file.tcl>` | Show document symbol hierarchy (procs, events, namespaces, variables) |
| `diagram` | `<file.tcl>` | Extract control flow diagram data from compiler IR |
| `event-info` | `<EVENT_NAME>` | Show iRules event registry metadata (no file needed) |
| `command-info` | `<COMMAND_NAME>` | Show iRules command registry metadata (no file needed) |
| `context` | `<file.tcl>` | Build context pack: diagnostics + symbols + event metadata |
| `all` | `<file.tcl>` | Run semantic-tokens + diagnostics + symbols + format + optimize together |

All line/col arguments are **0-based**, matching the LSP protocol.

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

## Example Invocations

```bash
python3 .claude/skills/lsp-client/lsp_client.py semantic-tokens tcl-lsp/samples/for_screenshots/03-completions.tcl
python3 .claude/skills/lsp-client/lsp_client.py diagnostics tcl-lsp/editors/vscode/testFixture/diagnostics.tcl
python3 .claude/skills/lsp-client/lsp_client.py hover tcl-lsp/editors/vscode/testFixture/procs.tcl 1 6
python3 .claude/skills/lsp-client/lsp_client.py optimize tcl-lsp/samples/for_screenshots/22-optimiser-before.tcl
python3 .claude/skills/lsp-client/lsp_client.py symbols tcl-lsp/samples/for_screenshots/ai-scene.irul
python3 .claude/skills/lsp-client/lsp_client.py diagram tcl-lsp/samples/for_screenshots/ai-scene.irul
python3 .claude/skills/lsp-client/lsp_client.py event-info HTTP_REQUEST
python3 .claude/skills/lsp-client/lsp_client.py command-info HTTP::uri
python3 .claude/skills/lsp-client/lsp_client.py context tcl-lsp/samples/for_screenshots/ai-scene.irul
python3 .claude/skills/lsp-client/lsp_client.py all tcl-lsp/samples/for_screenshots/03-completions.tcl
```

$ARGUMENTS
