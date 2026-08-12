# LSP feature-layer duplication/hardcoding audit

## F1: Hover's pattern/format-string detectors hardcode command names and mislocate arguments, disagreeing with semantic tokens' registry-driven marking

**Confidence:** high
**Category:** duplicated-logic

**Where it is now:** `rust/tcl-lsp-core/src/hover.rs`. A family of private
"detect the special-syntax argument under the cursor" helpers, all called
unconditionally near the top of the main `hover_impl` dispatch chain
(`hover.rs:641-649`, ahead of the generic word resolver at `hover.rs:651`):

- `glob_pattern_at_position` (`hover.rs:2164-2183`) — hardcodes
  `matches!(tokens[0], "glob") || (tokens[0]=="string" && tokens[1]=="match") || (tokens[0]=="lsearch" && tokens.contains(&"-glob"))`.
- `regex_pattern_at_position` (`hover.rs:2415-2432`) — hardcodes
  `matches!(tokens[0], "regexp" | "regsub") || (tokens[0]=="lsearch" && tokens.contains(&"-regexp"))`.
- `sprintf_format_string_at_position` (`hover.rs:1395-1405`) — hardcodes
  `tokens[0] != "format" && tokens[0] != "scan"`.
- `clock_format_string_at_position` (`hover.rs:2523-2538`) — hardcodes
  `tokens[0] != "clock" || (tokens[1] != "format" && tokens[1] != "scan")`.
- `binary_format_context_at_position` (`hover.rs:1808-1837`) — hardcodes
  `tokens[0] != "binary" || (tokens[1] != "format" && tokens[1] != "scan")`.
- `regsub_subspec_at_position` (`hover.rs:1994-2033`).

All six split the *current source line* on whitespace
(`line_text.split_whitespace()`) to find the command name, then locate the
literal under the cursor with `string_literal_at` / `string_literal_with_percent_at`
(`hover.rs:2039-2064`, `1411-1440`) — a raw brace/quote scan over `line_text.chars()`
that finds *whichever* `"…"`/`{…}` literal the cursor's column falls inside,
with **no check of that literal's argument index**. (`binary_format_context_at_position`
additionally falls back to an index-checked `word_token_at`, but only for the
unbraced-bareword case — its primary, more common path is the unchecked
`string_literal_at`.) The doc comments call this out as deliberate
("Single-line only", `hover.rs:2163`, `2414`; "This is the same single-line
context detection used by `signature_help` / `completion`", `hover.rs:2526-2529`)
but the position-blindness is not: nothing stops a hover request on a
*non-pattern* literal argument of the same command from being claimed.

**What already exists that it should use:** `rust/tcl-lsp-core/src/semantic_tokens.rs`
already solves the identical question — "which argument of this call is a
pattern/format string, and what sub-language is it" — entirely from registry
data, with no command-name matching:

- `insert_regex_overrides` (`semantic_tokens.rs:1932-1978`) gates on
  `registry.get(head).and_then(|s| s.pattern_type) == Some(PatternType::Regex)`
  and locates the argument via
  `registry.arg_indices_for_role(head, arg_texts, ArgRole::Pattern)`
  (`semantic_tokens.rs:1939-1953`), falling back to switch-skipping only when
  the spec hasn't declared a position yet.
- `insert_format_overrides` (`semantic_tokens.rs:2040-2065`) does the same for
  `FormatType::{Sprintf,Clock,Binary,Regsub}` via
  `ArgRole::FormatString`/`ArgRole::ScanFormat`.

Both operate on the segmenter's `SegmentedCommand`/`arg_texts` — the CST-derived
argument list — rather than re-tokenising line text, so they are correct for
multi-line commands and never pick up an unrelated literal.

**Disagreement evidence:** `regsub {a.*b} $str {x.y}` — registry data for
`regsub` (`rust/tcl-registry/src/commands/tcl/regsub_.rs:87,95`) assigns
`ArgRole::Pattern` only to the *first* positional argument (`exp`) and
`ArgRole::FormatString` to the third (`subSpec`, the replacement — it uses `&`/`\N`
backreference syntax, not regex). Hovering over the replacement literal `x.y`:
semantic tokens (via `insert_regex_overrides` + `arg_indices_for_role`) never
marks it as a regex fragment — it renders as ordinary text. Hover, via
`regex_pattern_at_position`, only checks that the line's first token is
`regsub` and that the cursor sits inside *some* brace/quote literal on that
line; it returns `"x.y"` and renders `regex_hover_text("x.y")`, i.e. a
**"Regex pattern"** markdown panel describing `.` as "Match any single
character" — wrong: in a `regsub` replacement, `.` is a literal character, and
the only specials are `&` and `\N`. The same shape of bug applies to
`binary_format_context_at_position` (no argument-index check on its primary
`string_literal_at` path, so a `binary format`/`binary scan` call with a
literal *value* argument gets it misparsed as the format specifier string) and
to `glob_pattern_at_position`, which additionally hardcodes exactly `"glob"`
and `"string" "match"` — the two commands whose registry specs
(`rust/tcl-registry/src/commands/tcl/glob_.rs`, the `match` `SubCommand` in
`string_.rs:1332-1349`) currently declare **no** `pattern_type`/`ArgRole::Pattern`
at all, unlike `chan names`/`parray`/`lsearch`, which already do
(`chan_.rs:616-617`, `parray.rs:92-93`). So the two commands hover.rs special-cases
by hand are precisely the two not yet wired into the generic mechanism —
this is the "migration debt" AGENTS.md describes: knowledge that should live
in the registry is duplicated, ad hoc, and only partially, in a consumer.

**Why it matters:** Hover text is directly editor-visible and is the
disagreeing half of the pair — a user hovering the replacement/value argument
of `regsub`/`binary format`/`binary scan` gets a plausible-looking but
incorrect "Regex pattern" / binary-specifier breakdown, while the semantic
token colouring on the same word correctly shows it as ordinary text. Because
these checks run unconditionally near the top of `hover_impl`, ahead of the
generic word resolver, a wrong match here also **pre-empts** whatever more
specific/correct hover (e.g. a variable or command hover) would otherwise have
fired for that literal.

**What cleanup looks like:** Extend `glob_.rs`'s and `string_.rs`'s `match`
`SubCommand` with `pattern_type: Some(PatternType::Glob)` + `arg_roles`/`arg_role_resolver`
entries for the pattern position (mirroring `chan names`/`parray`), so `glob`
and `string match` join the already-registry-driven set. Then replace
`glob_pattern_at_position` / `regex_pattern_at_position` /
`sprintf_format_string_at_position` / `clock_format_string_at_position` /
`binary_format_context_at_position` / `regsub_subspec_at_position` with a
single hover path that reuses the segmenter's `SegmentedCommand` (already
built for the enclosing call) plus `registry.arg_indices_for_role(...)` /
`CommandSpec::pattern_type` / `FormatType`, the same query
`insert_regex_overrides`/`insert_format_overrides` already perform — so hover
and semantic tokens are provably asking the registry the same question
instead of maintaining two independently-hand-coded answers.

**Scale:** Medium — six functions plus their two shared line-scanning helpers
(`string_literal_at`, `string_literal_with_percent_at`) in one file; the
registry extension is two small `CommandSpec` edits.
