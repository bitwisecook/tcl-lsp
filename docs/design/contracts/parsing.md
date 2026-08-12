# Parsing pipeline contracts — segmentation + recovery

How source becomes commands. The parsing layer tokenises source
([lexing.md](lexing.md)), segments it into commands, and repairs common syntax
errors so downstream analysis keeps running on malformed input — which, in an
editor, is most of the time. The rules below are what hold command boundaries
and parser-facing diagnostics steady across a partial edit or a syntax error.

## Decision rules / contracts

1. Command segmentation preserves original token order and positional
   fidelity. `SegmentedCommand` is what the analyser and the lowerer consume;
   neither runs its own token-iteration loop.
2. **The red-green concrete syntax tree is authoritative for segmentation.**
   `tcl_compiler::parsing::syntax` builds a lossless CST from the existing
   lexer stream (start-to-start tiling — no second parser) and
   `syntax::segment` derives `SegmentedCommand`s from it byte-identically to
   the token-loop segmenter. A new consumer segments or descends through the
   tree rather than spinning up a private lexer loop. See
   [syntax-tree.md](../compiler/syntax-tree.md).
   - `green` is the position-independent layer: a node knows its *width* and
     its children, never an absolute offset, and trivia attaches to the
     adjacent token so a command is pure syntax while every inter-word byte
     still round-trips.
   - `red` overlays a green tree with an anchoring and resolves absolute
     positions lazily, reproducing the lexer's exact offsets, lines, and
     columns.
   - `descend` enters braced bodies and `[…]` substitutions as child CSTs
     anchored one byte past the opener.
3. `SegmentedCommand::argv` deliberately keeps only one representative token
   per word, which loses the ordered shape of a compound word such as
   `prefix-$name-[clock seconds]`. `WordFragment` is the companion record that
   retains the original fragment sequence and its reconstructed spelling, so
   the semantic IR is derived without re-lexing or guessing from a flattened
   argv string. Anything that needs word *shape* uses the fragments.
4. **Recovery repairs the segmentation, never the source.** The recovery
   helpers run between segmentation and command dispatch and mutate the
   `SegmentedCommand` in place — inserting virtual tokens — so a handler sees
   the intended argument structure even when the source has a stray `]` or a
   missing `{`.
5. A recovery heuristic and the diagnostic that reports the same defect share
   one detector. `recover_stray_close_bracket` uses
   `syntax_checks::find_first_stray_bracket` /
   `find_bracket_insertion_point` — the same functions behind E100 and its
   quick fix — so the repair fires exactly where E100 fires, at the position
   E100 would insert. Two independent copies of such a heuristic drift and
   start repairing code the diagnostic does not flag.
6. Partial and errored commands still produce deterministic structures for
   downstream consumers. A parse fault must not cascade into a stream of
   derived false positives.
7. A word's closing `}` / `]` / `"` is derived only through
   `tcl_lexer::word_closer_offset` / `word_end_position`
   ([lexing.md](lexing.md) rule 4).

## File-path anchors

- `rust/tcl-compiler/src/segmenter.rs` — `SegmentedCommand`, `WordFragment`,
  `segment_commands*`.
- `rust/tcl-compiler/src/parsing/syntax/` — the red-green CST (`green.rs`,
  `red.rs`, `build.rs`, `segment.rs`, `descend.rs`).
- `rust/tcl-compiler/src/analyser/recovery.rs` — the in-place repairs.
- `rust/tcl-compiler/src/analyser/syntax_checks.rs` — the detectors recovery
  and the diagnostics share.
- `rust/tcl-lexer/src/` — tokens, spans, source map, range accessors.

## Failure modes

- Unclosed delimiters cascading false errors after the first parse fault.
- Segment-boundary drift leading to command-name misclassification.
- Recovery producing ranges that no longer match source positions.
- A repair firing where its paired diagnostic does not, so the user sees
  analysis of code they did not write.

## Discoverability

- [Design doc index](../README.md)
- [the canonical concrete syntax tree (CST)](../compiler/syntax-tree.md)
- [lexing contracts](lexing.md)
- [shared utility contracts](shared-utility-contracts-rust.md)
- [compiler pipeline overview](../compiler/compiler-pipeline-overview.md)
