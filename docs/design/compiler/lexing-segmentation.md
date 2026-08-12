# KCS: Lexing and segmentation (Stages 1–2)

## Symptom

A contributor needs to understand how raw Tcl source is split into tokens
and grouped into commands, or is debugging why a command is being segmented
incorrectly (wrong word boundaries, missing tokens, interpolation issues).

## Context

Stage 1 (lexing) produces a flat `Vec<Token>` via `Lexer::tokenise_all()`.
Stage 2 (segmentation) groups tokens into `SegmentedCommand` values via
`segment_commands()`.  These two stages run before any compiler logic and
feed all downstream phases.

Source: `rust/tcl-lexer/src/lexer.rs` (`Lexer::tokenise_all`,
`tokenise_all_with_warnings`), `rust/tcl-lexer/src/tokens.rs` (`Token`,
`TokenType`), `rust/tcl-compiler/src/segmenter.rs` (`segment_commands`)

## Content

### Stage 1 — Lexing

`Lexer` is an `Iterator<Item = Result<Token, LexError>>`; it scans
character-by-character and produces typed tokens:

| `TokenType` | Trigger | Example |
|-------------|---------|---------|
| `Esc` | Plain word fragment (possibly escaped) | `set`, `42`, `hello` |
| `Str` | Braced string `{…}` (braces stripped) | `{hello world}` |
| `Cmd` | Command substitution `[…]` (brackets stripped) | `[expr {1+2}]` |
| `Var` | Variable substitution `$name`, `${name}`, `$arr(idx)` | `$x` |
| `Sep` | Run of intra-command whitespace | ` `, `\t` |
| `Eol` | End-of-line / semicolon | `\n`, `;` |
| `Eof` | End-of-input sentinel | |
| `Comment` | Comment from `#` to end of line | `# ...` |
| `Expand` | `{*}` argument-expansion prefix (Tcl 8.5+) | `{*}$list` |

`TokenType::name()` renders the symbolic upper-case spelling (`"ESC"`,
`"STR"`, …) used in tooling output, so those forms still appear in explorer
views and test fixtures.

**Example** — `set x 42`:
```
Token(Esc, "set")  Token(Sep, " ")  Token(Esc, "x")  Token(Sep, " ")  Token(Esc, "42")  Token(Eof, "")
```

**Example** — `set y $x`:
```
Token(Esc, "set")  Token(Sep, " ")  Token(Esc, "y")  Token(Sep, " ")  Token(Var, "x")  Token(Eof, "")
```

Note: the `$` prefix is consumed by the lexer; the token's text is the bare
variable name.

**Stray punctuation convention** — a standalone `}` or `]` that appears
outside its structural role (i.e. not closing a brace-group or command
substitution) gets `TokenType::Esc`, not a special kind. Downstream
consumers that check for stray punctuation must test
`tok.kind == TokenType::Esc` in addition to the text, to distinguish stray
characters from structural delimiters (which are part of `Str` or `Cmd`
tokens). `TokenType::group_closer()` reports the closer a group kind strips
(`Str` → `}`, `Cmd` → `]`).

**Line-tracking convention** — the lexer resolves line/column two ways that
must agree: a `\n`-only line-start index (`SourceMap` /
`LineIndex`, `rust/tcl-lexer/src/source_map.rs` and `line_index.rs`, also
consumed by the red [concrete syntax tree](syntax-tree.md) overlay), and an
incremental line/column counter advanced per character. **Only `\n` is a line
break for positions.** A lone carriage return — including a backslash-CR
*continuation* (`\<CR>`, the old-Mac line ending) — splits the word like any
continuation but does **not** advance the line, because the index never records
it; a CRLF advances the line on its `\n`. Treating a lone `\<CR>` as a line
break in the incremental counter (but not the index) made the token *after* it
report `start` one line below its own `end` — a backwards range. Any new
position-tracking path must keep the two mechanisms in lock-step.

### Stage 2 — Segmentation

`segment_commands()` does not run its own hand-rolled token loop: it builds
the canonical lossless **red-green concrete syntax tree** for the region
(`rust/tcl-compiler/src/parsing/syntax/`, see
[syntax-tree.md](syntax-tree.md)) via `build::build_document`, then *derives*
the `SegmentedCommand` list from it via `segment::segments_from_document`.
The derivation is byte-identical to the former token loop — verified
field-for-field against a frozen copy of that loop, preserved as an
independent oracle in `rust/tcl-compiler/tests/differential_segment.rs`, over
the edge-case table and the full Tcl 8.4/8.5/8.6/9.0 corpus.  The derivation
runs in local-offset space; relocation is the caller's job, via
`SegmentedCommand::shifted_by` (which `segment_commands_with_offset` applies).

The segmenter groups tokens into commands at `Eol`/`Eof` boundaries:

```rust
pub struct SegmentedCommand {
    /// Byte span covering the whole command.
    pub span: Span,
    /// Per-word representative tokens (one per argv entry).
    pub argv: Vec<Token>,
    /// Per-word reconstructed text.
    pub texts: Vec<String>,
    /// Ordered lexical fragments for every word.
    pub word_fragments: Vec<Vec<WordFragment>>,
    /// Whether each word is a single token.
    pub single_token_word: Vec<bool>,
    /// All tokens in the command (including separators).
    pub all_tokens: Vec<Token>,
    /// Whether the command is incomplete (unclosed delimiter).
    pub is_partial: bool,
    /// Which delimiter was left unclosed, when `is_partial`.
    pub partial_delimiter: Option<UnclosedDelimiter>,
    /// `{*}` expansion markers per word, if any word uses expansion.
    pub expand_word: Option<Vec<bool>>,
    /// Concatenated text of the comment line(s) immediately preceding
    /// the command; `None` when no comment precedes.
    pub preceding_comment: Option<String>,
}
```

Key fields:
- `texts[0]` = command name, `texts[1..]` = arguments (the `name()` and
  `args()` accessors wrap exactly this)
- `single_token_word[i]` = `true` when word `i` is a single atomic token —
  tells the lowerer the value is a compile-time constant
- `argv[i]` = representative (first) token of word `i`, for token-kind
  pattern matching; `arg_tokens()` is `argv[1..]`
- `expand_word` is `Some(v)` only when *some* word in the command uses
  expansion, and then `v[i]` is `true` for each `{*}`-prefixed word
  (Tcl 8.5+); it is `None` otherwise, so consumers write
  `cmd.expand_word.as_deref().unwrap_or(&[])`
- `word_fragments[i]` is the lossless companion to the
  `argv`/`texts` parallel arrays — new semantic IR consumers should use it
  when substitution order within a word matters
- Multi-token words (e.g. `"hello $name"`) are concatenated into `texts[i]`

**Variable references in texts:**
`Var` tokens are wrapped in `${…}` form: `$x` → `texts[i] == "${x}"`.

### Argument expansion `{*}` and dialect gating

`{*}` is the Tcl 8.5+ argument-expansion prefix.  When enabled, the
lexer emits a zero-width `Expand` token at word start, and the
segmenter records `expand_word[i] = true` for the following word so
that downstream passes can distinguish `{*}$list` (expanded to zero or
more runtime args) from a literal `*${list}` word.

`LexerConfig::expand_syntax` (`rust/tcl-lexer/src/lexer.rs`) controls whether
`{*}` is recognised.  Its value is dialect-derived: each `DialectProfile`
carries a `LexerGrammar` (`rust/tcl-dialect/src/grammar.rs`) that the
`LexerConfig` is built from.  The catalogue in
`rust/tcl-dialect/src/profile.rs` defines four grammars:

| Grammar | `expand_syntax` | `irules_brace_separator` | `braced_var` |
|---------|-----------------|--------------------------|--------------|
| `GRAMMAR_TCL84` | `false` | `false` | `FirstClose` |
| `GRAMMAR_TCL8X` (8.5/8.6, iApps, tmsh, Expect, EDA shells) | `true` | `false` | `FirstClose` |
| `GRAMMAR_TCL9X` (9.x, and the permissive default) | `true` | `false` | `Tcl9Nesting` |
| `GRAMMAR_IRULES` | `false` | `true` | `FirstClose` |

So expansion is **disabled** for the 8.4-based grammars (`tcl8.4`,
`f5-irules`) because `{*}` did not exist in Tcl 8.4 — the lexer must treat
`{*}$x` as a braced literal `{*}` concatenated with `$x`.

Arity checking is deliberately **conservative** about expansion; it does not
try to constant-fold the expanded word.  `count_positionals(args,
arg_expand, start)` in
`rust/tcl-compiler/src/analyser/diagnostics/validity.rs` returns
`(nargs_min, any_expand)`: expanded words are simply excluded from the count,
leaving a *lower bound* on the true runtime argument count. `arity_verdict`
then abstains where a lower bound proves nothing:

- **E002** (too few) is suppressed whenever `positional_any_expand` is set —
  the expanded word may supply the missing arguments.
- **E003** (too many) still fires when the non-expanded arguments alone
  exceed the signature maximum, since expansion can only add more. The
  surplus-argument span and its delete fix are omitted under expansion
  (`excess_positional_span` returns `None`), so the diagnostic falls back to
  the whole-command anchor.
- **E005** (wrong argument-count *shape* — `Arity::step` / `also_exact`) is
  likewise suppressed under expansion: an expanded tail's final count says
  nothing about its parity.

The same abstention runs one level up: `segment_argc` returns `None` for a
command with any expanded argument word, and `ensemble_subcommand_candidate`
declines a `{*}`-expanded subcommand word, so the cross-file arity and
ensemble checks skip those calls entirely.

### How segmented data feeds the compiler

1. **IR lowering** reads `texts[0]` to identify the command and `argv[i].kind`
   to pattern-match on token kinds (e.g. `lower_set` in
   `rust/tcl-compiler/src/lowering_hooks.rs` checks whether the value word is
   `Str`, `Esc`, `Cmd`, or `Var`).
2. **Error recovery** (`rust/tcl-compiler/src/analyser/recovery.rs`) mutates
   the `SegmentedCommand` in place after segmentation — merging tokens around
   a stray `]` into a virtual `Cmd` token, repairing a missing `{` — so
   downstream handlers see the intended argument structure.
3. **Semantic analysis** uses `span` for diagnostic positions and
   `all_tokens` for syntax highlighting / semantic tokens.

### Reuse of the tokenisation

The analysis pipeline lexes the same source bytes from several independent
paths: the segmenter (`segment_commands`), the lowerer (`lower_to_ir`),
`compiler_checks`, and `var_refs` each tokenise overlapping regions, and
nested braced bodies are re-lexed at every level of recursion.

The green token tree is what collapses most of that duplication — see
[green-token-tree.md](green-token-tree.md) and
[syntax-tree.md](syntax-tree.md). In the Rust workspace the green layer
(`rust/tcl-compiler/src/parsing/syntax/green.rs`) is genuinely
**position-independent**: a node knows only its cached *width* and its
children, never an absolute offset. The `red` layer overlays an anchoring
and resolves absolute positions lazily, and `descend` lazily re-lexes braced
bodies and `[…]` substitutions as child CSTs anchored one byte past the
opener, so a body is tokenised once per anchoring rather than once per
consumer.

Two caches sit outside the tree:

- `VarReferenceScanner` (`rust/tcl-compiler/src/var_refs.rs`) lexes at base
  offset 0 — it extracts position-independent variable names — and keeps a
  bounded LRU keyed by `(source text, scan mode)`. The mode is part of the key
  because `scan_word` and `scan_script` can legitimately disagree about the
  same text. Being text-keyed rather than offset-keyed, it shares hits across
  the SSA / GVN / interprocedural passes and across documents, which a
  per-document anchored tree cannot.
- Cross-edit reuse is the LSP database's job, not the compiler's: `tcl-lsp-db`
  wraps the pure compiler entry points in salsa `#[salsa::tracked]` queries
  with dependency-tracked invalidation, so there is no manual cache eviction
  in the compiler itself.

### Worked example — `set y $x`

Segmented:

```rust
SegmentedCommand {
    texts: vec!["set".into(), "y".into(), "${x}".into()],
    single_token_word: vec![true, true, true],
    argv: vec![/* Esc "set" */, /* Esc "y" */, /* Var "x" */],
    ..
}
```

Lowered (Stage 3) to `Statement::AssignValue { name: "y", .. }` — not
`Statement::AssignConst`, because the value contains a variable substitution.

## Decision rule

- If a command is not being lowered correctly, check `single_token_word` and
  `argv[i].kind` — these drive pattern matching in the lowering hooks.
- Multi-token words (interpolated strings) have `single_token_word[i] == false`
  and produce `Statement::AssignValue` (not `Statement::AssignConst`).
- `is_partial: true` on a `SegmentedCommand` means it was recovered from
  malformed input — downstream passes should still work but may have
  degraded precision. `partial_delimiter` says which delimiter was left
  unclosed.

## Related docs

- [syntax-tree.md](syntax-tree.md) — the canonical red-green CST the segmenter
  builds and derives `SegmentedCommand`s from
- [Examples 1–2 in walkthroughs](../../../docs/design/example-script-walkthroughs.md#example-1-set-x-42)
- [Data structure reference](../../../docs/design/example-script-walkthroughs.md#data-structure-reference)
- [kcs-error-recovery.md](../../../docs/design/compiler/error-recovery.md)
- [kcs-compiler-pipeline-overview.md](../../../docs/design/compiler/compiler-pipeline-overview.md)
