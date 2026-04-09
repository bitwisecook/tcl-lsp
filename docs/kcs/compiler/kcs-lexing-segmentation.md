# KCS: Lexing and segmentation (Stages 1–2)

## Symptom

A contributor needs to understand how raw Tcl source is split into tokens
and grouped into commands, or is debugging why a command is being segmented
incorrectly (wrong word boundaries, missing tokens, interpolation issues).

## Context

Stage 1 (lexing) produces a flat `list[Token]` via `TclLexer.tokenise_all()`.
Stage 2 (segmentation) groups tokens into `SegmentedCommand` objects via
`segment_commands()`.  These two stages run before any compiler logic and
feed all downstream phases.

Source: [`core/parsing/lexer.py`](../../../core/parsing/lexer.py) (`tokenise_all` at line 494),
[`core/parsing/tokens.py`](../../../core/parsing/tokens.py),
[`core/parsing/command_segmenter.py`](../../../core/parsing/command_segmenter.py) (`segment_commands` at line 390)

## Content

### Stage 1 — Lexing

The lexer scans character-by-character and produces typed tokens:

| TokenType | Trigger | Example |
|-----------|---------|---------|
| `ESC` | Plain word fragment (possibly escaped) | `set`, `42`, `hello` |
| `STR` | Braced string `{…}` | `{hello world}` |
| `CMD` | Command substitution `[…]` | `[expr {1+2}]` |
| `VAR` | Variable substitution `$name` | `$x`, `${arr(idx)}` |
| `SEP` | Whitespace separator | ` `, `\t` |
| `EOL` | End-of-line / semicolon | `\n`, `;` |
| `EOF` | End of input | |
| `COMMENT` | Comment to end of line | `# ...` |
| `EXPAND` | `{*}` expansion prefix | `{*}$list` |

**Example** — `set x 42`:
```
Token(ESC, "set")  Token(SEP, " ")  Token(ESC, "x")  Token(SEP, " ")  Token(ESC, "42")  Token(EOF, "")
```

**Example** — `set y $x`:
```
Token(ESC, "set")  Token(SEP, " ")  Token(ESC, "y")  Token(SEP, " ")  Token(VAR, "x")  Token(EOF, "")
```

Note: the `$` prefix is consumed by the lexer; `Token.text` contains the bare
variable name.

### Stage 2 — Segmentation

The segmenter groups tokens into commands at `EOL`/`EOF` boundaries:

```python
SegmentedCommand(
    range=Range(start, end),
    argv=[first_token_per_word, ...],
    texts=["set", "x", "42"],           # concatenated text per word
    single_token_word=[True, True, True], # True when word is one token
    all_tokens=[...],                     # every token in the command
)
```

Key fields:
- `texts[0]` = command name, `texts[1:]` = arguments
- `single_token_word[i]` = `True` when word `i` is a single atomic token —
  tells the lowerer the value is a compile-time constant
- `argv[i]` = first token of word `i` (for token-type pattern matching)
- `expand_word[i]` = `True` when word `i` is preceded by the `{*}`
  argument-expansion prefix (Tcl 8.5+).  `None` when no word in the
  command uses expansion.
- Multi-token words (e.g. `"hello $name"`) are concatenated into `texts[i]`

**Variable references in texts:**
VAR tokens are wrapped in `${…}` form: `$x` → `texts[i] = "${x}"`.

### Argument expansion `{*}` and dialect gating

`{*}` is the Tcl 8.5+ argument-expansion prefix.  When enabled, the
lexer emits a zero-width `EXPAND` token at word start, and the
segmenter records `expand_word[i] = True` for the following word so
that downstream passes can distinguish `{*}$list` (expanded to zero or
more runtime args) from a literal `*${list}` word.

The `TclLexer.expand_syntax` flag controls whether `{*}` is recognised.
`configure_signatures()` in
[`core/commands/registry/runtime.py`](../../../core/commands/registry/runtime.py)
sets the flag based on the active dialect:

- **Enabled** for dialects in `dialects_since("tcl8.5")` — all Tcl
  8.5 / 8.6 / 9.0 profiles and every dialect whose base Tcl version is
  at least 8.5 (f5-iapps, f5-tmsh, EDA vendors, Expect).
- **Disabled** for 8.4-based dialects (`tcl8.4`, `f5-irules`) because
  `{*}` did not exist in Tcl 8.4 — the lexer must treat `{*}$x` as a
  braced literal `{*}` concatenated with `$x`.

Arity checks at both the analyser (`_check_proc_call_arity` in
`core/analysis/analyser.py`) and the IR layer (`_check_simple_arity` in
`core/compiler/compiler_checks.py`) treat each expanded word as an
unknown number of runtime arguments and try to refine the bound via
constant folding: braced literal lists, `[list ...]` substitutions,
and variables with known constant values resolve to an exact element
count, so E002/E003 still fire when the count is statically wrong.
Otherwise the expanded word contributes `0..∞` arguments, E002 is
suppressed, and E003 only fires when the non-expanded arguments alone
exceed the signature maximum.

### How segmented data feeds the compiler

1. **IR lowering** reads `texts[0]` to identify the command, `argv[i].type`
   to pattern-match on token types (e.g. `lower_set()` checks if the value
   is `STR`, `ESC`, `CMD`, or `VAR`).
2. **Error recovery** re-parses with virtual tokens injected, producing
   clean `SegmentedCommand` objects.
3. **Semantic analysis** uses `range` for diagnostic positions and
   `all_tokens` for syntax highlighting/semantic tokens.

### Worked example — `set y $x`

```python
# Segmented:
SegmentedCommand(
    texts=["set", "y", "${x}"],
    single_token_word=[True, True, True],
    argv=[Token(ESC,"set"), Token(ESC,"y"), Token(VAR,"x")],
)

# Lowered (Stage 3):
IRAssignValue(name="y", value="${x}")
# (not IRAssignConst — the value contains a variable substitution)
```

## Decision rule

- If a command is not being lowered correctly, check `single_token_word` and
  `argv[i].type` — these drive pattern matching in lowering hooks.
- Multi-token words (interpolated strings) have `single_token_word[i]=False`
  and produce `IRAssignValue` (not `IRAssignConst`).
- `is_partial=True` on a `SegmentedCommand` means it was recovered from
  malformed input — downstream passes should still work but may have
  degraded precision.

## Related docs

- [Examples 1–2 in walkthroughs](../../example-script-walkthroughs.md#example-1-set-x-42)
- [Data structure reference](../../example-script-walkthroughs.md#data-structure-reference)
- [kcs-error-recovery.md](kcs-error-recovery.md)
- [kcs-compiler-pipeline-overview.md](kcs-compiler-pipeline-overview.md)
