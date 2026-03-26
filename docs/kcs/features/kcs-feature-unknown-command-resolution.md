# KCS: feature — Unknown Command Resolution (W123)

## Summary

Detects unresolved commands and offers "did you mean?" suggestions. Static
analysis of user-defined `unknown` procs extracts dispatch targets to reduce
false positives.

## Surface

lsp, all-editors

## How to use

- **Enable**: W123 is opt-in. Set `tclLsp.diagnostics.W123 = true` in your editor settings.
- **Diagnostics**: Unresolved commands appear as HINT-level underlines. Hover shows the message and any "did you mean?" suggestion.
- **Code actions**: When a suggestion is available, a quick-fix replaces the command name.
- **Suppress per-line**: Append `# noqa: W123` to suppress on a specific line.

### Recognised `unknown` proc patterns

When the analyser encounters `proc unknown {cmd args} { ... }`, it inspects
the body and adjusts W123 behaviour accordingly:

| Pattern | Example | Effect |
|---------|---------|--------|
| **switch dispatch** | `switch $cmd { foo {...} bar {...} }` | `foo`, `bar` are treated as known commands |
| **Empty stub** | `proc unknown {args} {}` | Nothing resolves; W123 fires for all unknown commands |
| **Chain to original** | `_original_unknown $cmd {*}$args` | Conservative: W123 suppressed entirely |
| **auto\_load** | `auto_load $cmd` | Conservative: W123 suppressed entirely |
| **exec** | `exec $cmd {*}$args` | Conservative: W123 suppressed entirely |
| **Case-insensitive** | `switch [string tolower $cmd] {...}` | All known commands match; W123 suppressed |

### Other suppression mechanisms

- **Dynamic providers**: If `load` or `auto_path` manipulation is detected, W123 is suppressed for the entire file.
- **Dialect stubs**: Commands declared via `# tcl-lsp: stub` are treated as known. See [Dialect Stubs](../kcs-dialect-stubs.md).
- **User-defined procs**: Any `proc` defined in the file (or sourced via packages) is a known command.
- **Namespace-qualified names**: Commands containing `::` are skipped (may come from `namespace import`).

## Operational context

W123 runs as a **post-analysis pass** after all proc definitions have been
collected.  This means forward-defined procs and `unknown` handlers defined
later in the file are still captured.

The "did you mean?" engine uses Levenshtein edit distance (max distance 2)
against the union of: registry commands, user-defined procs, stub commands,
and `unknown` dispatch targets.

## File-path anchors

- `core/analysis/analyser.py` — `_emit_unresolved_command_diagnostics`, `_extract_unknown_proc_info`
- `core/analysis/semantic_model.py` — `UnknownProcInfo`
- `core/common/text.py` — `edit_distance`, `suggest_similar`
- `lsp/features/diagnostics.py` — `_to_lsp_diagnostic` (code description link)

## Failure modes

- False positives in codebases using `interp alias`, `namespace import *`, or other dynamic command creation not detected by the gating logic.
- Forward-defined `unknown` proc in a sourced file (cross-file) is not detected — analysis is single-file.

## Test anchors

- `tests/test_analyser.py` — W123 test cases
- `tests/test_text_utils.py` — edit distance and suggestion tests

## Discoverability

- [KCS feature index](README.md)
- [Diagnostics calculation](../compiler/kcs-diagnostics-calculation.md)
- [Dialect stubs](../kcs-dialect-stubs.md)
