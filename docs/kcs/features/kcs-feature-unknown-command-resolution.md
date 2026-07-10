# KCS: feature — Unknown Command Resolution (W123)

> **Audience:** User
> **Type:** Functionality

## Summary

Detects unresolved commands and offers "did you mean?" suggestions. Static
analysis of user-defined `unknown` procs extracts dispatch targets to reduce
false positives.

## Applies to

all-editors, jetbrains (other editors via XDG config), warning

## How to use

- **Enable**: W123 is on by default. Set `tclLsp.diagnostics.W123 = false` in your editor settings to disable it.
- **Diagnostics**: Unresolved commands appear as HINT-level underlines. Hover shows the message and any "did you mean?" suggestion.
- **Code actions**: When a suggestion is available, a quick-fix replaces the command name.
- **Suppress per-line**: Append `# noqa: W123` to suppress on a specific line.

### Recognised `unknown` proc patterns

When the analyser encounters `proc unknown {cmd args} { ... }` (or the
qualified form `proc ::tcl::unknown ...`), it inspects the body and adjusts
W123 behaviour accordingly:

| Pattern | Example | Effect |
|---------|---------|--------|
| **switch -exact dispatch** | `switch $cmd { foo {...} bar {...} }` | `foo`, `bar` are treated as known commands |
| **switch -glob/-regexp** | `switch -glob $cmd { fo* {...} }` | Conservative: W123 suppressed entirely |
| **Empty stub** | `proc unknown {args} {}` | Nothing resolves; W123 fires for all unknown commands |
| **Chain to original** | `_original_unknown $cmd {*}$args` | Conservative: W123 suppressed entirely |
| **auto\_load** | `auto_load $cmd` | Conservative: W123 suppressed entirely |
| **exec** | `exec $cmd {*}$args` | Conservative: W123 suppressed entirely |
| **Case-insensitive** | `switch [string tolower $cmd] {...}` | All known commands match; W123 suppressed |

### Other suppression mechanisms

- **Dynamic providers**: If `load`, `set auto_path`, `lappend auto_path`, `rename`, or `namespace import` is detected, W123 is suppressed for the entire file.
- **Package require**: If any `package require` is present, W123 is suppressed (external packages may define commands).
- **Dialect stubs**: Commands declared via `# tcl-lsp: stub` are treated as known. See [Dialect Stubs](../../../docs/design/contracts/dialect-stubs.md).
- **User-defined procs**: Any `proc` defined in the file (or sourced via packages) is a known command.
- **Command aliases**: Commands defined via `interp alias` are treated as known. See [Command Alias Resolution](../../../docs/design/contracts/command-alias-resolution.md).
- **Namespace-qualified names**: Commands containing `::` are skipped (may come from `namespace import`).

## Operational context

W123 runs as a **post-analysis pass** after all proc definitions have been
collected.  This means forward-defined procs and `unknown` handlers defined
later in the file are still captured.

The "did you mean?" engine uses Levenshtein edit distance (max distance 2)
against the union of: registry commands, user-defined procs, stub commands,
`unknown` dispatch targets, and command alias names.

## File-path anchors

- `analyser/_analyser/_diag_commands.py` — `_emit_unresolved_command_diagnostics`
- `analyser/_analyser/_oo.py` — `_extract_unknown_proc_info`
- `analyser/semantic_model.py` — `UnknownProcInfo`
- `shared/text.py` — `edit_distance`, `suggest_similar`
- `server/features/diagnostics.py` — `_to_lsp_diagnostic` (code description link)

## Failure modes

- False positives in codebases using dynamic command creation (e.g. `apply`, `coroutine`) not detected by the gating logic.
- Forward-defined `unknown` proc in a sourced file (cross-file) is not detected — analysis is single-file.

## Test anchors

- `tests/test_analyser.py` — W123 test cases
- `tests/test_text_utils.py` — edit distance and suggestion tests

## Example

With `tclLsp.diagnostics.W123` at its default (enabled):

```tcl
proc greet {name} {
    puts "Hello, $name"
}

gret "Alice"
```

Line 5 shows a hint-level squiggle under `gret` with the message
`W123 unresolved command 'gret' — did you mean 'greet'?`. A
lightbulb code action offers **Replace with `greet`** which
rewrites the call in one click.

## Discoverability

- [KCS feature index](README.md)
- [Diagnostics calculation](../../../docs/design/compiler/diagnostics-calculation.md)
- [Dialect stubs](../../../docs/design/contracts/dialect-stubs.md)
