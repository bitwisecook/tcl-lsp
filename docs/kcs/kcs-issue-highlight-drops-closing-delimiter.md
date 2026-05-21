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

## Decision rules / contracts

1. Do **not** widen word-token ranges in lowering or the segmenter's word
   tokens — many passes rely on the inner-end convention.
2. A consumer that renders a highlight widens the range itself, the same way
   the diagnostics pipeline does, via
   [`widen_range_for_closer`](../../shared/ranges.py).
3. A whole-command range built from a token span is widened with
   `range_from_word_token` (closer derived from the token *type*, no source
   needed — required because nested bodies have absolute offsets but a
   substring source).
4. The compiler explorer front-end slices `src.substring(startOffset,
   endOffset)` with an **exclusive** end, so serialised ranges must convert
   the inclusive semantic-model end to exclusive (`+1`), matching
   [`to_lsp_range`](../../server/_lsp_conv.py).

## File-path anchors

- `shared/ranges.py` — `widen_range_for_closer`, `range_from_word_token`, `widen_for_highlight`, `set_highlight_source`
- `compiler/parsing/command_segmenter.py` — `_command_range`
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

## Triage checklist

1. Slice the source with the emitted range and confirm whether the covered
   text is a complete, balanced token.
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
