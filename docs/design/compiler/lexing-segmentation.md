# Lexing and segmentation (Stages 1–2)

How raw Tcl source is split into tokens and grouped into commands. Read this
when debugging word boundaries, missing tokens, or interpolation handling at
the front of the pipeline.

Stage 1 (lexing) produces a flat `Vec<Token>` via `Lexer::tokenise_all`
(`Lexer` is an `Iterator`, so `tokenise_all` is a `collect`;
`tokenise_all_with_warnings` returns the non-fatal `LexWarning`s alongside).
Stage 2 (segmentation) groups tokens into `SegmentedCommand` values via
`segment_commands`.  These two stages run before any compiler logic and
feed all downstream phases.

Source: `rust/tcl-lexer/src/lexer.rs` (`Lexer`, `LexerConfig`, `tokenise_all`),
`rust/tcl-lexer/src/tokens.rs`,
`rust/tcl-compiler/src/segmenter.rs` (`SegmentedCommand`, `segment_commands`)

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

**Stray punctuation convention** — a standalone `}` or `]` that appears
outside its structural role (i.e. not closing a brace-group or command
substitution) receives `TokenType.ESC`, not a special type. Downstream
consumers that check for stray punctuation must test
`tok.kind == TokenType::ESC` in addition to `tok.text` to distinguish stray
characters from structural delimiters (which are part of `STR` or `CMD`
tokens).

**Line-tracking convention** — the lexer resolves line/column two ways that
must agree: a `\n`-only line-start index (`LineIndex`'s `line_starts`,
consumed by `position_at` and the red [concrete syntax tree](syntax-tree.md)
overlay), and an
incremental `line`/`col` counter advanced per character. **Only `\n` is a line
break for positions.** A lone carriage return — including a backslash-CR
*continuation* (`\<CR>`, the old-Mac line ending) — splits the word like any
continuation but does **not** advance the line, because the index never records
it; a CRLF advances the line on its `\n`. Treating a lone `\<CR>` as a line
break in the incremental counter (but not the index) made the token *after* it
report `start` one line below its own `end` — a backwards range. Any new
position-tracking path must keep the two mechanisms in lock-step.

### Stage 2 — Segmentation

`segment_commands` does not run its own token loop: it builds
the canonical lossless **red-green concrete syntax tree** for the region
(`rust/tcl-compiler/src/parsing/syntax/`, see
[syntax-tree.md](syntax-tree.md)) and *derives* the `SegmentedCommand` list from
it.  The derivation matches the token loop field-for-field — `span`, `argv`,
`texts`, `single_token_word`, `all_tokens`, `preceding_comment`, and
`expand_word` (verified over the real-world corpus, 120k randomised
differential cases, and nested-body anchoring) — so everything below
describes the output shape.

The segmenter groups tokens into commands at `EOL`/`EOF` boundaries:

```rust
pub struct SegmentedCommand {
    pub span: Span,
    pub argv: Vec<Token>,              // first token of each word
    pub texts: Vec<String>,            // concatenated text per word
    pub word_fragments: Vec<Vec<WordFragment>>,
    pub single_token_word: Vec<bool>,  // true when the word is one token
    pub all_tokens: Vec<Token>,        // every token in the command
    pub is_partial: bool,
    pub partial_delimiter: Option<UnclosedDelimiter>,
    pub expand_word: Option<Vec<bool>>,
    pub preceding_comment: Option<String>,
}
```

Key fields:
- `texts[0]` = command name, `texts[1..]` = arguments
- `single_token_word[i]` = `true` when word `i` is a single atomic token —
  tells the lowerer the value is a compile-time constant
- `argv[i]` = first token of word `i` (for token-type pattern matching)
- `expand_word` = `Some(flags)` where `flags[i]` is `true` when word `i` is
  preceded by the `{*}` argument-expansion prefix (Tcl 8.5+).  `None` when
  no word in the command uses expansion.
- Multi-token words (e.g. `"hello $name"`) are concatenated into `texts[i]`

**Variable references in texts:**
VAR tokens are wrapped in `${…}` form: `$x` → `texts[i] = "${x}"`.

### Argument expansion `{*}` and dialect gating

`{*}` is the Tcl 8.5+ argument-expansion prefix.  When enabled, the
lexer emits a zero-width `EXPAND` token at word start, and the
segmenter records `expand_word` flag `i` as `true` for the following word so
that downstream passes can distinguish `{*}$list` (expanded to zero or
more runtime args) from a literal `*${list}` word.

The `expand_syntax` field of `LexerConfig`
(`rust/tcl-lexer/src/lexer.rs`) controls whether `{*}` is recognised.  It
is populated from the active dialect's `LexerGrammar`
(`rust/tcl-dialect/src/profile.rs`):

- **Enabled** by `GRAMMAR_TCL8X` and `GRAMMAR_TCL9X` — all Tcl
  8.5 / 8.6 / 9.x profiles and every dialect whose base Tcl version is
  at least 8.5 (f5-iapps, f5-tmsh, EDA vendors, Expect).
- **Disabled** by `GRAMMAR_TCL84` and `GRAMMAR_IRULES` (`tcl8.4`,
  `f5-irules`) because `{*}` did not exist in Tcl 8.4 — the lexer must
  treat `{*}$x` as a braced literal `{*}` concatenated with `$x`.

Arity checks at both the analyser (user-proc call sites) and the IR layer
(`check_simple_arity` in
`rust/tcl-compiler/src/analyser/diagnostics/validity.rs`, which takes the
command's `arg_expand` flags alongside its argument words) treat each
expanded word as an
unknown number of runtime arguments and try to refine the bound by
constant-folding the expanded word.  Refinement requires the word to
be **single-token** (so concatenations like `{*}$x$y` or
`{*}{a b}$suffix` stay unrefined) and depends on the layer:

- **Analyser layer (user proc calls)** can refine
  - braced literal lists (`{*}{a b c}` → 3, `{*}{}` → 0),
  - pure variable references with a known constant string value
    (`set rgb {255 255 255}; foo {*}$rgb` → 3) via the analyser's
    `const_strings` map (`rust/tcl-compiler/src/analyser/state.rs`).
- **IR layer (built-in commands)** can refine
  - braced literal lists (the segmenter strips the braces, so the
    refinement uses the original `STR` token type to disambiguate
    the resulting text from a variable substitution),
  - literal `[list ...]` command substitutions via
    `extract_foreach_elements` (`rust/tcl-compiler/src/sccp.rs`).
  IR-layer refinement does *not* yet resolve `$var` substitutions
  back to their constant values — pure-var expansions in built-in
  calls fall back to the `0..∞` range below.

When refinement succeeds the leading-options scan and the positional
count both see the inlined elements, so E002/E003 still fire when the
count is statically wrong (and `puts {*}{-nonewline} chan msg` is
correctly accepted because the literal list contributes a leading
option).  Otherwise the expanded word contributes `0..∞` arguments,
E002 is suppressed, and E003 only fires when the non-expanded
arguments alone exceed the signature maximum.

### How segmented data feeds the compiler

1. **IR lowering** reads `texts[0]` to identify the command, `argv[i].kind`
   to pattern-match on token types (e.g. `lower_set()` checks if the value
   is `STR`, `ESC`, `CMD`, or `VAR`).
2. **Error recovery** re-parses with virtual tokens injected, producing
   clean `SegmentedCommand` objects.
3. **Semantic analysis** uses `span` for diagnostic positions and
   `all_tokens` for syntax highlighting/semantic tokens.

### Shared tokenisation memo (now the green token tree)

The analysis pipeline lexes the same source bytes from several independent
paths: the segmenter (`segment_commands`), the lowerer (`lower_to_ir`),
`compiler_checks`, and `var_refs` each tokenise overlapping regions, and
nested braced bodies are re-lexed at every level of recursion.

The per-analysis memo is the **green token tree**'s analysis-scoped intern
index — see [green-token-tree.md](green-token-tree.md). Its correctness
rules:

- Keyed by `(base_offset, base_line, base_col, mode, text)` → a `TokenRegion`
  carrying `(tokens, warnings)`. The `text` is part of the key so two distinct
  substrings lexed at the same base offset (e.g. two bodies both lexed at
  base 0) never collide.
- Tokens are immutable, so the cached stream is shared read-only — consumers
  build their own derived structures and never mutate it.
- Regions lexed with error-recovery virtual insertions are never interned
  (the insertions are request-specific).
- The index is scoped to one analysis and discarded when that analysis
  ends, so memory is bounded and the lexer-affecting context (dialect,
  strict-quoting) is stable for its lifetime.

`var_refs` (`rust/tcl-compiler/src/var_refs.rs`) lexes at base offset 0 (it
extracts position-independent variable names) and keeps its own bounded LRU
keyed by the scanned text and scan mode, which shares across the SSA / GVN /
interprocedural scanners (and across documents) in a way the
absolute-offset, per-document tree cannot. It consults the tree's leaf
tokenisation but keeps that result cache.

### Worked example — `set y $x`

```rust
// Segmented:
SegmentedCommand {
    texts: vec!["set", "y", "${x}"],
    single_token_word: vec![true, true, true],
    argv: vec![Token(ESC, "set"), Token(ESC, "y"), Token(VAR, "x")],
    ..
}

// Lowered (Stage 3):
Statement::AssignValue { name: "y", value: "${x}", .. }
// (not Statement::AssignConst — the value contains a variable substitution)
```

## Decision rule

- If a command is not being lowered correctly, check `single_token_word` and
  `argv[i].kind` — these drive pattern matching in lowering hooks.
- Multi-token words (interpolated strings) have `single_token_word[i] == false`
  and produce `Statement::AssignValue` (not `Statement::AssignConst`).
- `is_partial == true` on a `SegmentedCommand` means it was recovered from
  malformed input — downstream passes should still work but may have
  degraded precision.

## Related docs

- [syntax-tree.md](syntax-tree.md) — the canonical red-green CST the segmenter
  builds and derives `SegmentedCommand`s from
- [Examples 1–2 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-1-set-x-42)
- [Data structure reference](../../../docs/design/example-script-walkthroughs.md#data-structure-reference)
- [error-recovery.md](error-recovery.md)
- [compiler-pipeline-overview.md](compiler-pipeline-overview.md)
