# KCS: Why don't W112 and W118 offer a quick fix?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

all-editors, diagnostic

## Question

Why do W112 (Trailing whitespace.) and W118 (Inconsistent line endings.)
stay hint-only, with no quick-fix code action to remove the whitespace
or rewrite the line endings?

## Answer

Both are deliberate decisions, reviewed in July 2026: the document
formatter is the fix, and a per-diagnostic quick fix would be either
unsafe or redundant.

**W112** is a plain line scan over the source text
(`rust/tcl-lsp-core/src/source_style.rs`) with no syntax knowledge. That
is fine for a hint, but not for an automatic edit: a trailing space at
the end of a line *inside* a multi-line braced or quoted word is string
data, and deleting it changes the program's values. The formatter's
trailing-whitespace pass (`trim_trailing_ws_preserving_literals`, on by
default via `trim_trailing_whitespace`) carries a cross-line brace,
quote, and backslash scan precisely so it never trims those lines
(`RUST_ISSUE_037`). A safe per-diagnostic fix would have to duplicate
that scan for every flagged line — all of the formatter's cost with none
of its coverage. The naive remove-whitespace edit the checker computes
internally (`StyleDiagnostic::fix`) therefore stays unsurfaced.

**W118** is a single file-level hint, and its only meaningful fix is to
rewrite every structural line ending in the document — which is a
formatting run, not a spot fix. The formatter already does it: output is
emitted with the configured `line_ending` (default LF), while endings
inside multi-line string literals are data and stay untouched. A quick
fix would add nothing over running the formatter, and would need the
same expected-ending configuration the formatter already has.

What to do instead: run your editor's **Format Document** action (or
format on save). To silence the hints, set
`tclLsp.diagnostics.W112 = false` / `tclLsp.diagnostics.W118 = false`;
W112 also honours a per-line `# noqa: W112`, while W118 is file-level
and is not line-suppressible.

## Related

- [KCS index](README.md)
- [W112 — trailing whitespace](codes/kcs-diagnostic-w112-trailing-whitespace.md)
- [W118 — inconsistent line endings](codes/kcs-diagnostic-w118-inconsistent-line-endings.md)
- [How to suppress diagnostics](kcs-howto-suppress-diagnostics.md)
- [Formatter engine design](../design/contracts/formatter-engine.md)
