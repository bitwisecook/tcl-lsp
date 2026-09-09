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

| `TokenType` | Trigger | Example |
|-----------|---------|---------|
| `Esc` | Plain word fragment (possibly escaped) | `set`, `42`, `hello` |
| `Str` | Braced string `{…}` | `{hello world}` |
| `Cmd` | Command substitution `[…]` | `[expr {1+2}]` |
| `Var` | Variable substitution `$name` | `$x`, `${arr(idx)}` |
| `Sep` | Whitespace separator | ` `, `\t` |
| `Eol` | End-of-line / semicolon | `\n`, `;` |
| `Eof` | End of input | |
| `Comment` | Comment to end of line | `# ...` |
| `Expand` | `{*}` expansion prefix | `{*}$list` |
| `ExprSugar` | JimTcl `$(…)` expression substitution (only under `VarSyntax::Jim`) | `$($a * 2)` |

**Example** — `set x 42`:
```
Token(Esc, "set")  Token(Sep, " ")  Token(Esc, "x")  Token(Sep, " ")  Token(Esc, "42")  Token(Eof, "")
```

**Example** — `set y $x`:
```
Token(Esc, "set")  Token(Sep, " ")  Token(Esc, "y")  Token(Sep, " ")  Token(Var, "x")  Token(Eof, "")
```

Note: the `$` prefix is consumed by the lexer; `Token.text` contains the bare
variable name.

**Stray punctuation convention** — a standalone `}` or `]` that appears
outside its structural role (i.e. not closing a brace-group or command
substitution) receives `TokenType::Esc`, not a special type. Downstream
consumers that check for stray punctuation must test
`tok.kind == TokenType::Esc` in addition to `tok.text` to distinguish stray
characters from structural delimiters (which are part of `Str` or `Cmd`
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
it.  `rust/tcl-compiler/tests/differential_segment.rs` holds the derivation
byte-identical to a frozen copy of the earlier token loop, so everything below
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
`Var` tokens are wrapped in `${…}` form: `$x` → `texts[i] = "${x}"`.

### Argument expansion `{*}` and dialect gating

`{*}` is the Tcl 8.5+ argument-expansion prefix.  When enabled, the
lexer emits a zero-width `Expand` token at word start, and the
segmenter records `expand_word` flag `i` as `true` for the following word so
that downstream passes can distinguish `{*}$list` (expanded to zero or
more runtime args) from a literal `*${list}` word.

The `expand_syntax` field of `LexerConfig`
(`rust/tcl-lexer/src/lexer.rs`) controls whether `{*}` is recognised.  It
is populated from the active dialect's `LexerGrammar`
(`rust/tcl-dialect/src/profile.rs`):

- **Enabled** by `GRAMMAR_TCL85`, `GRAMMAR_TCL86`, and `GRAMMAR_TCL9X` — the
  Tcl 8.5 / 8.6 / 9.x profiles, the EDA vendors, and Expect.
- **Disabled** by `GRAMMAR_TCL84` (`tcl8.4`) and `GRAMMAR_F5_TCL`
  (`f5-irules`, `f5-iapps`, `f5-tmsh`) because `{*}` did not exist in the
  Tcl 8.4 core those dialects are built on — the lexer must treat `{*}$x` as
  a braced literal `{*}` concatenated with `$x`.  `GRAMMAR_F5_TCL` is also
  the grammar that treats `}{` as a word separator.

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
    refinement uses the original `Str` token type to disambiguate
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
   is `Str`, `Esc`, `Cmd`, or `Var`).
2. **Error recovery** re-parses with virtual tokens injected, producing
   clean `SegmentedCommand` objects.
3. **Semantic analysis** uses `span` for diagnostic positions and
   `all_tokens` for syntax highlighting/semantic tokens.

### Who lexes

The segmenter builds the CST once per region and derives from it; the
lowerer re-segments each nested braced body it descends into
(`segment_commands_with_offset_and_config`), which builds a CST for that
region.  One scanner lexes for itself: `var_refs`
(`rust/tcl-compiler/src/var_refs.rs`) lexes at base offset 0 — it extracts
position-independent variable names — behind a bounded LRU keyed by scanned
text and scan mode, shared across the SSA / GVN / interprocedural scanners and
across documents.  There is no shared tokenisation memo.

### Worked example — `set y $x`

```rust
// Segmented:
SegmentedCommand {
    texts: vec!["set", "y", "${x}"],
    single_token_word: vec![true, true, true],
    argv: vec![Token(Esc, "set"), Token(Esc, "y"), Token(Var, "x")],
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
- [Examples 1–2 in walkthroughs](../../../docs/design/compiler/example-walkthroughs.md#example-1-set-x-42)
- [Data structure reference](../../../docs/design/compiler/example-walkthroughs.md#data-structure-reference)
- [error-recovery.md](error-recovery.md)
- [compiler-pipeline-overview.md](compiler-pipeline-overview.md)
