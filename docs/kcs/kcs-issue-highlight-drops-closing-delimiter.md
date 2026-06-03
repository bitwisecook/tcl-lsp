# KCS: Highlight stops one character short of the closing brace

> **Audience:** Contributor
> **Type:** Issue

## Applies to

all-editors

## Symptom

A highlight over a braced, quoted, or bracketed word covers everything
*except* its closing delimiter — `{$condition}` is highlighted as
`{$condition` (or, when a second off-by-one stacks on top, `{$conditi`).
Seen when hovering a control-flow graph node or an intermediate-representation
clause in the compiler explorer, and when expanding the selection
(`textDocument/selectionRange`) over a braced word.

## Operational context

A braced word token starts on the opening `{` but its `end` sits on the last
*inner* character; the matching closer is one position past `end`, and the
token's `text` omits it. Semantic-model [`Range`](../../analyser/semantic_model.py)
ends are **inclusive** by convention, and word-token ranges follow an
"inner-end" rule: the closer is **excluded**, and a consumer that wants to
cover the whole word widens the range itself. This keeps the optimiser, SCCP,
structure-elimination, code-sinking, and minifier — which strip or re-widen
the delimiters themselves — working against a stable contract. Widening the
range at the lowering layer breaks those consumers.

An **empty** `{}` / `[]` / `""` is the exception: it has no inner character, so
`end` already sits **on** the closer, and widening it instead overshoots by one
(see the failure modes below).

## Decision rules / contracts

The closer position is **derived once from the lexer's content geometry** and
read through a single authoritative accessor — never re-computed as
`tok.end.offset + 1` at the call site.  Re-deriving is what scattered the
off-by-one across consumers: it overshoots an empty `{}`/`[]`/`""` (end already
on the closer) and silently omits quoted `"..."` words.

1. Do **not** widen word-token ranges in lowering or the segmenter's word
   tokens — many passes rely on the inner-end convention.
2. To find a delimited word's closer, call
   [`word_closer_offset`](../../shared/ranges.py) (offset) or
   [`word_end_position`](../../shared/ranges.py) (position).  Both use
   `tok.text` to detect emptiness, so they are correct for empty words and for
   quoted words whose inner text contains backslash escapes.  Do not write
   `tok.end.offset + 1` followed by a `source[...] == closer` check.
3. A whole-command range built from a token span is widened with
   `range_from_word_token`.  It is token-only (no source — required because
   nested bodies have absolute offsets but a substring source) and covers the
   closer for `STR`/`CMD` words, deriving it from the token *type*.  An empty
   `{}`/`[]` (`span_extra = (end - start) - len(text) >= 1`) already has `end`
   on the closer, so it is returned unchanged rather than advanced one more
   byte.  Quoted `"..."` words (which lex as `ESC`) are **not** widened here —
   the closer character is not derivable from the token type without source, and
   `cmd.range` consumers (W105 unbraced-body detection, segmenter tiling) rely
   on the inner-end for quoted bodies.  A consumer that needs a quoted word's
   closing `"` uses the source-aware `word_closer_offset` / `word_end_position`.
4. A consumer that renders a highlight widens via
   [`widen_range_for_closer`](../../shared/ranges.py), which guards the empty
   two-character span the same way.
5. The compiler explorer front-end slices `src.substring(startOffset,
   endOffset)` with an **exclusive** end, so serialised ranges must convert
   the inclusive semantic-model end to exclusive (`+1`), matching
   [`to_lsp_range`](../../server/_lsp_conv.py).

## File-path anchors

- `shared/ranges.py` — `word_closer_offset`, `word_end_position`, `range_from_word_token`, `widen_range_for_closer`, `widen_for_highlight`, `set_highlight_source`
- `compiler/parsing/command_segmenter.py` — `_command_range`
- `tooling/refactoring/_spans.py` — `token_end_offset`, `command_span_offsets`
- `analyser/checks/_style.py`, `analyser/irules_checks.py` — fix-range widening
- `tooling/cli/formatters.py` — `range_dict`
- `tooling/cli/serialise.py` — `serialise_result` sets the highlight source
- `compiler/codegen/wasm/_ir.py` — `_range_to_explorer_dict`
- `server/features/selection_range.py` — token-range widening

## Failure modes

- A serialiser emits the inclusive end raw, so an exclusive-slicing front-end
  drops the last character.
- A consumer renders a word-token range without widening, so the closing
  `}` / `]` / `"` is missing.
- Widening the range at lowering instead of the consumer corrupts optimiser
  passes that strip or re-widen the delimiters (string-compare detection,
  branch folding, and others).
- Widening an **empty** `{}` / `[]` / `""` word whose `end` already sits on the
  closer overshoots by one; a trailing empty argument then absorbs the
  enclosing body's `}` and a phantom stray brace fires `E102` (issue #527).

## Triage checklist

1. Slice the source with the emitted range and confirm whether the covered
   text is a complete, balanced token. The `compiler-explorer` skill's
   `slices` view prints every IR statement's range alongside the literal source
   slice it covers, so an over- or under-shoot is visible at a glance
   (`python .claude/skills/compiler-explorer/explore.py slices --source '...'`).
2. Check whether the consumer widens for the closer; if not, that is the bug.
3. For the explorer, confirm the serialised `endOffset` is the exclusive end
   (inclusive `+1`).
4. Add a regression case asserting the covered substring round-trips to the
   full word.

## Test anchors

- `tests/test_highlight_ranges.py`
- `tests/test_selection_range.py`
- `tests/test_diagnostic_ranges.py`

## Related

- [KCS index](README.md)
- [range drift across passes](kcs-issue-range-drift.md)
- [shared utility contracts](../design/contracts/shared-utility-contracts.md)
- [Glossary](../GLOSSARY.md)
