# KCS: Error recovery — virtual token injection

## Symptom

A contributor needs to understand how the parser handles malformed input
(unclosed brackets, quotes, braces) without aborting the pipeline, or needs
to debug why a downstream pass receives an unexpected token stream from
ill-formed source.

## Context

The parser must produce well-formed `SegmentedCommand` objects even when the
source has missing delimiters.  `recovery.py` runs a first-pass parse, detects
unterminated tokens via heuristics, injects `VirtualToken` objects, and
re-parses to produce clean commands.  Diagnostics (E201–E206) are emitted for
the user while the rest of the pipeline proceeds on the repaired parse.

Source: [`core/parsing/recovery.py`](../../../core/parsing/recovery.py)

## Content

### Virtual token injection flow

```
Source text (malformed)
    │
    ▼
segment_commands()  ← first-pass parse
    │
    ├─ unterminated CMD token detected (no ']')
    │
    ▼
Heuristic match (command break, indent, comment)
    │
    ▼
VirtualToken(offset, char="]", diagnostic=E201)
    │
    ▼
Re-parse with injected token  ← second-pass parse
    │
    ▼
Clean SegmentedCommand list + E201 diagnostic
```

### Heuristic table

| Code | Missing delimiter | Heuristic trigger |
|------|-------------------|-------------------|
| E201 | `]` | `#` comment on next line, known command on next line, or `{` inside `[` |
| E202 | `"` | Newline with known command on next non-blank line |
| E203 | `}` | De-indented line starting with a known command |
| E204 | Extra chars after `}` | Lexer warning (no virtual token needed) |
| E205 | Extra chars after `"` | Lexer warning |
| E206 | Missing `}` for `${var` | Lexer warning |

### Worked example — unclosed bracket

```tcl
set x [string length "hello"
set y 42
```

1. First parse: segmenter sees `[` at offset 6 without matching `]`.
2. `_detect_missing_bracket_at_command` fires: next non-blank line starts
   with `set` (a known command).
3. `VirtualToken(offset=29, char="]", diagnostic=E201)` is created.
4. Second parse produces two clean commands:
   - `SegmentedCommand(texts=["set", "x", '[string length "hello"]'])`
   - `SegmentedCommand(texts=["set", "y", "42"])`
5. Both downstream passes (IR lowering, CFG, SSA, codegen) proceed normally.

## Decision rule

- If the pipeline aborts with malformed input, check whether `recovery.py`
  has a heuristic for the specific delimiter pattern.
- If a new unclosed-delimiter pattern is common in the wild, add a new
  heuristic to `recovery.py` with a fresh E-code diagnostic.
- Virtual tokens are inserted at the *end* of the line where the opening
  delimiter appears — never mid-token.

## Related docs

- [Example 20 in walkthroughs](../../example-script-walkthroughs.md#example-20-error-recovery--unclosed-bracket)
- [GLOSSARY.md](../../GLOSSARY.md)
- [kcs-compiler-pipeline-overview.md](kcs-compiler-pipeline-overview.md)
