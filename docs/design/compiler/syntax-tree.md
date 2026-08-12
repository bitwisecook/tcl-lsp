# The canonical concrete syntax tree (red-green CST)

> **Status:** Cycles 1–9 shipped; cycle 10 (a hot-path allocation micro-opt) is
> deferred as not worth its codegen risk, and the final shared-memo cleanup of the
> remaining value-lexers is complete. The position-independent green tree, the
> lazy red overlay, and the constructor are live in `compiler/parsing/syntax/`;
> `command_segmenter._segment_raw` derives its `SegmentedCommand`s from the tree
> byte-identically (cycle 1); lazy descent into braced bodies and `[…]`
> substitutions is in `syntax/descend.py`, byte-identical to a direct
> `green_tree` tokenisation (cycle 2); and `rust/tcl-compiler/src/compiler_checks.rs` now descends
> the shared tree and runs checks from its segments, retiring its duplicate
> mini-segmenter so nested commands are analysed identically to top-level (cycle
> 3 — a reviewed behaviour change, not byte-identical). The formatter, the whole
> minifier, switch-body lowering, the semantic-token collector (+ inlay hints /
> code actions), and the iRule object-ref scanner now all segment / lex through
> the tree and its shared `tokenise` memo — each verified byte-identical over the
> corpus (cycles 4, 6, 7, 8); `var_refs` and friends were already there (cycle 5);
> and the explorer gained `cst` / `segments` views across every surface (cycle 9).
> See [Roadmap](#roadmap) for the per-cycle detail and what remains.

This document specifies the lossless, position-independent concrete syntax tree
(CST) that is becoming the **single representation** the whole pipeline rides
on: command segmentation, AOT lowering to the bytecode VM and WASM, the
formatter, the minifier, and the per-command `tcl` / `f5 irule` tooling. Today
each of those re-lexes the bytes it cares about — the formatter and minifier
each spin up their own `TclLexer`, the segmenter runs its own token loop,
`var_refs` and `compiler_checks` re-lex bodies. The CST replaces that with one
tree everyone descends.

## Why a *red-green* tree

The model is the Roslyn / rust-analyzer split:

- **Green** (`syntax/green.py`) is **position-independent**: a node knows only
  its *width* and its children, never an absolute offset. Two structurally
  identical regions (e.g. two empty `{}` words) can be the *same* object, and a
  green subtree is reusable verbatim when an edit shifts it — the property that
  makes incremental reparse and cross-edit caching cheap.
- **Red** (`syntax/red.py`) overlays a green tree with an *anchoring*
  (`base_offset` / `base_line` / `base_col`) and resolves absolute positions
  **lazily**, reproducing exactly the `Token` offsets/lines/columns the lexer
  emits.

This is deliberately the model the earlier *green token tree* (issue #477,
[`green-token-tree.md`](green-token-tree.md)) decided **not** to build: that
structure is a context-aware tokenisation memo whose tokens carry **absolute**
positions, because ~80 consumers read `Token.start.offset` as absolute. The CST
does not change those consumers — it is built *alongside*, verified
byte-identical, and adopted incrementally. (The `green_tree.py` memo node is named
`TokenRegion`; the CST's structural node under `syntax/` is `GreenNode`, so the
two no longer collide.)

## Node model

```
GreenNode   kind: DOCUMENT | COMMAND | WORD
            children: tuple[GreenNode | GreenToken, ...]
            expand_markers: tuple[GreenToken, ...]   # {*} prefixes (WORD)
            range_end_rel:  int | None               # COMMAND: see Ranges
            preceding_comment: str | None            # COMMAND: see Comments
            trailing: tuple[GreenTrivia, ...]         # DOCUMENT: dangling trivia

GreenToken  token_type: TokenType   # ESC | STR | CMD | VAR | EXPAND
            text: str               # lexer inner text  ("abc" for {abc})
            raw:  str               # full source slice  ("{abc}")  → width
            end_rel: int            # Token.end.offset − Token.start.offset
            in_quote: bool
            leading/trailing: tuple[GreenTrivia, ...]

GreenTrivia kind: WHITESPACE | EOL | COMMENT
            text: str
```

**Trivia is attached** (leading/trailing on the adjacent token), Roslyn-style,
so a `COMMAND` / `WORD` is pure syntax while every inter-word byte is still
preserved. A token's *narrow* width is `len(raw)`; its *full* width adds the
attached trivia. A node's full width is the sum of its children's — cached at
construction.

Two text fields are kept on a token because neither is derivable from the other
without re-encoding the lexer's delimiter conventions: `text` is what
`Token.text` carries downstream (inner), `raw` is what makes the tree lossless
(full span). `end_rel` is the position-independent shape that lets the red layer
reconstruct `Token.end` — which, by the #527 convention, sits on the last inner
character for a non-empty `{…}` but *on the closer* for an empty `{}`.

## Losslessness

Concatenating every element's `full_text` in document order reproduces the
source byte-for-byte. The constructor recovers each fragment's `raw` by
**start-to-start tiling**: the lexer advances its cursor monotonically, so
`source[tok[i].start : tok[i+1].start]` is exactly the bytes fragment *i*
occupies — delimiters included — which sidesteps the inner-end / empty-delimiter
(#527) convention entirely. A trailing comment that attaches to no command
(`puts hi ;# bye`) is held as `DOCUMENT.trailing`.

## Position-equivalence (the red layer)

`SyntaxTree(green, base…, text=…)` builds a region-relative line index (from the
known `text`, or the green tree's own reconstruction) and resolves a region
offset to an absolute `SourcePosition` exactly as `TclLexer._pos_at` does:
region-relative line bisect, then shift line by `base_line` and the *first*
line's column by `base_col`. A `SyntaxToken`'s raw start is `node_start +
leading_width`; its `to_token()` reproduces the lexer `Token` (type, inner text,
start, `end` via `end_rel`, `in_quote`). The red views are created lazily on
walk, so nothing is materialised that a consumer does not visit.

## Deriving `SegmentedCommand` (the first consumer)

`segment.segments_from_tree` walks the red tree and produces the segmenter's
public `SegmentedCommand` shape, byte-identical to the former hand-rolled loop:

- **`argv`** — one merged token per word (type/text/start from the first
  fragment, end from the last: the compound-word merge of `{a}b`).
- **`texts`** — `_word_piece` per fragment, reused from `command_segmenter`.
- **`all_tokens`** — every fragment *and* `{*}` marker, in document order.
- **`range`** — see below.
- **`preceding_comment`** / **`expand_word`** — see below.

Word-less commands (a line of only dangling `{*}` markers) are skipped, matching
the segmenter, which discards an `argv`-empty command.

### Ranges — the `word_boundary` rule

A command's faithful end is **not** geometric (last fragment's closer): the
segmenter derives it from a `word_boundary` tracker with a genuine quirk — `{*}`
advances the parser past a word without updating the boundary, so a trailing
dangling `{*}` (and even `a b{*}\`) yields a *stale* boundary the range falls
back from. The constructor replicates that tracker exactly and stores the result
as `range_end_rel`, a width **relative to the command's first token** (hence
position-independent). The red layer resolves it against the first token.

### Comments — `preceding_comment`

The segmenter's `last_comment` accumulates **across** dangling-`{*}` commands, so
a comment before a marker still attaches forward to the next real command. That
is non-local — the comment physically precedes the marker and must live in the
marker's leading trivia for losslessness — so the constructor computes
`preceding_comment` during the build (replaying the exact accumulate/blank-line-
reset rule) and stores it on the command. This doubles as the natural
"leading doc-comment" an AST wants.

## Descent into bodies (`syntax/descend.py`)

At its own level a `{…}` body or `[…]` command substitution is a single `STR` /
`CMD` token whose `raw` owns the full span — delimiters included. *Descending*
re-lexes its inner text (`token.text`) as a child tree anchored **one byte past
the opener** (`start.offset + 1`, `character + 1`), so the child owns the
delimiter-excluding interior with absolute positions matching where it sits in
the document — the "node owns its span, children exclude delimiters" shape,
realised for nested bodies. Descent is lazy and shares the lex memo (the child's
`build_document` tokenises through the same `green_tree.tokenise`).

- `descend_token(token, source)` descends an absolutely-positioned `STR`/`CMD`
  token; an unterminated region (closer absent — decided by `_terminated`, from
  the inner *length* so empty `{}`/`[]` classify correctly) yields a **recovered**
  child that still carries its inner tokens, the lossless representation of
  malformed input.
- `descend_command(cmd_name, args, arg_tokens, source)` descends each
  registry-resolved `ArgRole.BODY` argument (via `iter_body_arguments`); `EXPR`
  arguments stay with `expr_lexer` and data words are left opaque.

Because this descent's `build_document` and a direct `green_tree.tokenise` of the
same region share the one tokenisation memo (same inner text, same anchor), the
descended token stream is **identical by construction** — the
parity is asserted directly (same child fragments, same terminated/recovered
classification) over the corpus, 8 000 randomised nested cases, and multi-level
descent (a substitution inside a descended body).

## Verification

The bar is byte-identity with the prior segmenter, established before wiring and
locked by the suite afterwards:

- **Losslessness** — `full_text == source` over the real-world corpus (157 Tcl
  8.6/9.0 library files + fixtures), 120k randomised sources, and the edge-case
  table in `tests/test_syntax_tree.py`.
- **Position-equivalence** — red fragment tokens equal the lexer's non-trivia
  token stream (offsets, lines, UTF-16 columns, `in_quote`) over the same
  corpus + fuzz, including multi-line and unicode bodies. The randomised
  generator in `tests/test_syntax_tree.py` now also covers carriage returns
  (lone `\<CR>` continuations), astral / combining unicode, and a quoted word
  whose entire content is a backslash-newline — the seams a `\n`-only, ASCII
  generator never reached. The last is a real fragment, not trivia: the lexer
  emits a separator backslash-newline as `SEP`, but a quoted-word content
  backslash-newline as an `ESC` token the tree must keep (a fold there dropped
  the only fragment of the word).
- **Segment byte-identity** — `segments_from_*` matched the former
  `_segment_raw` field-for-field (`range`, `argv`, `texts`, `single_token_word`,
  `all_tokens`, `preceding_comment`, `expand_word`) over the corpus, 120k
  randomised differential cases, and nested-body (non-zero base) anchoring,
  before the loop was replaced.
- **Full `make test-rust`** green with the segmenter on the tree — the
  end-to-end proof that diagnostics, analysis, and AOT codegen are unchanged.

## Performance

Build + derive is ~1.55× the former bespoke loop on a library-file corpus
(build ~1.13×, derive ~0.42×). The dominant residual is the line index being
built twice — once in the lexer, once in the red layer; sharing the lexer's
index is the obvious follow-up. The overhead is amortised — and then erased — as
other consumers drop their own re-lexing onto the one tree. The lexing itself
(the real cost) is shared through the `green_tree` memo.

## Roadmap

Cycles 1 (foundation + segmenter), 2 (descent), 3 (`compiler_checks`), and 9
(the compiler explorer's `cst` / `segments` views, pulled forward as low-risk
read-only tooling) are shipped. The remaining follow-ons, each its own verified
cycle:

1. ~~**Descent**~~ — *shipped (cycle 2).* `syntax/descend.py` re-lexes braced
   bodies and `[…]` substitutions as child CSTs, so a delimited region is a node
   owning its full span with delimiter-excluding children — what the formatter
   and lowering need to recurse.
2. ~~**`compiler_checks`**~~ — *shipped (cycle 3).* Its `_process_node`
   mini-segmenter (a duplicate that fed checks raw `tok.text` and mishandled
   `{*}`) is retired: `_process_text` and the recursion descend the shared tree
   and run checks from `segments_from_tree`, so nested commands are analysed
   **identically to top-level**. This was deliberately *not* byte-identical —
   nested commands now catch warnings they were missing (W306/W304/W103/W101/W100…),
   one false positive (W104) is gone, and W105/W106 correctly escalate a var-subst
   unbraced body to ERROR. Every change was reviewed against a 2000-file corpus
   (tcllib, tklib, the Tcl trees, SpiceGenTcl). The fix also surfaced a latent
   W212 message bug (`$${v}`) the raw-text path had hidden.
The sequence below is driven by a full audit of every remaining consumer of the
old parsing methods (direct `TclLexer`, hand-rolled `prev_type` mini-segmenters,
and the dead-to-callers `green_tree.descend_*`). What stays: `green_tree.tokenise`
and `green_tree_scope` are the shared lex memo `build_document` itself rides — not
parsing methods to retire; `expr_lexer` and the `f5/query` lexer are separate
tokenisers; line/prefix lexers (hover, completion) are cursor-local.

**Audit findings (re-verified after cycle 9).** The repo is further along than a
raw `TclLexer` grep suggests: the public `segment_commands` API (≈30 call sites —
core analyses, optimiser, taint, PGO, refactoring spans, server `declaration` /
`folding` / `document_links` / `code_actions`, AI) has been **CST-backed since
cycle 1**, so those consumers already ride the tree. `var_refs`,
`proc_fingerprint`, and `place_bridge` already lex through the kept
`green_tree.tokenise` memo at base 0 and emit only name-sets / `Place`s (no
offsets). And no production code calls the old `green_tree.descend_*` any more
(only `tests/test_syntax_descend.py`'s NEW-vs-OLD equivalence harness and
`test_green_tree.py`), so the cleanup is gated only on keeping that safety net
until the last consumer lands. The **genuine** remaining work is the small set of
hand-rolled mini-segmenters that re-implement word-grouping + recovery on a raw
`TclLexer`: the formatter (`engine.parse_commands`, `_format_switch_body`), the
minifier (`_minify_body` + three work-stack descent scanners), semantic tokens
(`_semantic_tokens/_collect.py`, `_format_args.py`), `inlay_hints`, `hover`, and
`irules_refs` — i.e. cycles 4, 7, 8. Cycles 5–6 are mostly already-on-CST
(`segment_commands` / `tokenise`) with only the genuine re-lex sites in `lowering`
and the refactoring body splitters left to fold onto `descend_*`.

3. **Formatter + minifier** (cycle 4) — *mostly shipped.* The formatter
   (`engine.parse_commands` + `_format_switch_body`) and the minifier's body
   pass (`_minify_body`) now group the tree's `COMMAND`→`WORD` structure instead
   of a private `TclLexer` loop; `is_braced` / `is_quoted` come from each word's
   first token's lossless `raw`, comments/blank-lines from attached trivia.
   `_reconstruct_raw` is kept (it operates on the same lexer `Token`s, preserving
   the CMD backslash-newline normalisation), so output is unchanged — verified
   **byte-identical over 1126 corpus files** (formatter + default-path minifier)
   and **277 compact-path files** (minifier, old vs new), plus the formatter /
   minifier / semantics suites; `TclLexer` is retired from the formatter.
   *Remaining:* the minifier's three **compact-mode** descent scanners
   (`_scan_array_tokens` / `_scan_argument_tokens` / `_collect_string_literals`)
   are offset-precise source-edit machinery still on `TclLexer`; folding them onto
   `descend_*` is a follow-up with a compact-names byte-identical bar.
   Prereq (shipped): per-word `braced_word` / `quoted_word` on `SegmentedCommand`
   — and the incremental reparser's `_shift_command` now carries them too (a
   latent `de53e21` divergence test-slow surfaced once the stamp caught up).
4. ~~**`var_refs`** (+ `proc_fingerprint`, `place_bridge`)~~ — *shipped (cycle 5).*
   These collect name-sets / `Place`s (no offsets) through the shared
   `green_tree.tokenise` memo at base 0, preserving the cross-document name-set
   LRU. The `VAR_READ`-role scan additionally rebuilt its words from bare
   `tok.text`, so a substitution-named read target was miscounted as a literal
   read (`info exists [foo]` / `array get $name` reported `foo` / `name`); it now
   reads words from the canonical `segment_commands` source and skips VAR/CMD-led
   read targets, consistent with the CFG def-scan and the segmenter's spelling.
5. ~~**Refactoring + lowering body re-lexes**~~ — *shipped (cycle 6).*
   `lowering._switch_body_elements` (the only raw `TclLexer` left in lowering)
   now lexes through the shared `tokenise` memo — verified byte-identical by
   comparing the new and old `(elements, element_tokens)` over 4362 corpus bodies.
   The refactoring body splitters (`_switch_to_dict`, `_extract_datagroup`,
   `_spans`) and `_barrier_gate` already ride the CST-backed `segment_commands`.
6. ~~**Semantic tokens + server features**~~ — *shipped (cycle 7).*
   `_semantic_tokens/_collect.py` (the full segmenter + `virtual_insertions`
   recovery + recursion clone) and `_format_args._split_words` now lex through
   the shared `tokenise` memo (the collector's shared line index via
   `build_line_starts`, byte-identical to the lexer's); `inlay_hints` and
   `code_actions`' profile-directive scan likewise. Verified byte-identical:
   `semantic_tokens_full` output over 174 corpus files (separate-process compare)
   plus the semantic-token / inlay / code-action suites.
7. ~~**iRule object refs**~~ — *shipped (cycle 8).* `irules_refs` already
   segmented through CST-backed `segment_commands` and recursed on
   segment-provided tokens; its one remaining private `TclLexer` (the EXPR
   command-substitution scan) now lexes through the shared `tokenise` memo.
8. ~~**Compiler explorer**~~ — *shipped (cycle 9).* A structural `cst` view (each
   node's range vs its raw source slice, `text` vs `raw`, the inner-end convention,
   `{*}` markers, per-word `single`/`braced`/`quoted`/`expand` shape, and descent
   with the `terminated`/`recovered` flag) and a `segments` view (the public
   `SegmentedCommand` contract — range, word pieces, flags, preceding comment)
   are live in `rust/tcl-explorer/src/views.rs`, registered in `_VIEW_ORDER` /
   `ALL_VIEWS`. `greentree` stays the oracle. Mirrored into every surface: the
   TUI renders both as captured-ANSI text views; `rust/tcl-explorer/src/serialise.rs` gains
   `_serialise_cst` / `_serialise_segments` (+ `_VIEW_META`) so the web GUI
   (`static/index.html`, `explorer-core.js`) and the in-browser pyodide worker
   show structural, expandable, source-linked trees. **The Rust explorer
   (`EXP*`) drops the `greentree` view entirely:** the Rust parser produces a
   single red-green CST (`tcl-compiler::parsing::syntax`), with no separate
   legacy green-token tree, so `cst` is the sole parse-tree tab. The shared
   `static/` frontend (now driven by the Rust→WASM module) drops the tab to
   match; Python keeps emitting `greentree` while it remains the legacy oracle.
9. **Direct AOT lowering** (cycle 10) — *deferred (perf-only).* Lowering already
   rides the CST: it consumes `segment_commands` (CST-backed since cycle 1) and
   copies the fields into `_Command`. Making `_Command` a *view* over a
   `SyntaxNode` COMMAND would only retire the intermediate `SegmentedCommand`
   allocation — a hot-path micro-optimisation that feeds codegen, so its
   byte-identical IR + bytecode + `test-py` bar is not worth the risk for one
   fewer allocation. The CST-adoption goal is already met here.
10. **Shared-memo cleanup** (done) — the private `TclLexer` sites that lexed a
    *value* to analyse it (the var_refs-style uses in `core_analyses`, `cfg`,
    `gvn`, `taint`, the optimiser, `interprocedural`, `irules_flow`,
    `proc_arg_traits`, `execution_intent`, the analyser checks, `document_state`,
    …) now lex through the shared `tokenise` memo — token-for-token-identical
    value lexes that gain memo sharing within the analysis scope, not duplicate
    segmenters. The cursor-local prefix lexers (`hover`, `symbol_resolution`),
    the tokeniser foundation (`green_tree`, `lexer`), and the VM's `substitution`
    / `compiler` (which lex under lexer-affecting thread-locals outside any
    tokenise scope) keep their own `TclLexer`. The old flat-list `green_tree`
    descent is gone; `node_for` survives
    for the explorer debug dumps.

## Pointers

- Green layer: `compiler/parsing/syntax/green.py`
- Red layer: `compiler/parsing/syntax/red.py`
- Descent: `compiler/parsing/syntax/descend.py`
- Constructor: `compiler/parsing/syntax/build.py`
- Segment derivation: `compiler/parsing/syntax/segment.py`
- Segmenter (consumer): `compiler/parsing/command_segmenter.py`
- Tests: `tests/test_syntax_tree.py`

## Related docs

- [green-token-tree.md](green-token-tree.md) — the context-aware tokenisation
  memo (a different structure — its node is `TokenRegion`) and incremental
  reparse.
- [lexing-segmentation.md](lexing-segmentation.md) — lexer/segmenter contract.
