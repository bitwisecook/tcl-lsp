# Shared-utility contracts (Rust workspace)

The low-level logic every crate must share rather than reimplement, and why.
Behaviour drifts between the Rust crates because equivalent low-level
logic (namespace-name splitting, number parsing, unique-prefix option
matching, canonical error texts, backslash decoding, list codec) is
reimplemented locally. The drifts are subtle and user-visible: a naive
`rsplit("::")` mishandles colon *runs* (`namespace tail foo:::` must be
`""`, and `foo:::bar` must dispatch `foo::bar` — tclsh-verified), a
hand-rolled integer parser accepts `--5` or misses the 9.0 `0d`/`_`
forms, and a local prefix matcher resolves `""` against a one-entry
table where `Tcl_GetIndexFromObj` errors.

## Operational context

The Rust workspace splits shared logic by dependency altitude: the
grammar crates (`tcl-lexer`, `tcl-syntax`) at the bottom, the portable
command cores (`tcl-cmd-core`) above them, and the two runtimes
(`rust/tcl-vm`, `runtime/rust`), the compiler, and the LSP server as
consumers. Each utility below has exactly **one** owner; every other
crate calls it, wraps it, or (for `&str`/byte-slice duality) adapts it —
never re-derives it.

## Owners

The machine-checked manifest below is the canonical source-to-owner map. An
owner row names the source files that implement the owner, the public entry
points consumers use, the semantic axis that must be threaded, and the drift
gate (if one exists). `cargo xtask owner-resolution` fails when a listed file,
entry point, or gate moves without this contract being updated.

<!-- owner-resolution-manifest -->
| Surface | Owner source paths | Public entry points | Dialect/release axis | Drift gate |
| --- | --- | --- | --- | --- |
| names / namespaces | `rust/tcl-syntax/src/naming.rs`; `rust/tcl-cmd-core/src/namespace.rs` | `qualifier_segments`; `command_resolution_candidates`; `qualifiers`; `tail` | invariant; absolute-marker contract from #1493 | `xtask-resolution-drift` |
| lists | `rust/tcl-syntax/src/list.rs` | `find_element`; `split_list`; `list_element`; `join_list`; `append_list_element`; `junk_fragment` | invariant | none |
| dicts | `rust/tcl-syntax/src/list.rs`; `rust/tcl-syntax/src/value.rs` | `find_element`; `split_list`; `ValueOps::dict_pairs` | invariant | none |
| glob matching | `rust/tcl-syntax/src/glob.rs` | `string_match`; `string_case_match` | invariant | none |
| switch body grammar | `rust/tcl-syntax/src/switch_body.rs` | `tokenise_switch_body`; `parse_braced_pairs` | invariant | none |
| numbers | `rust/tcl-syntax/src/number.rs`; `rust/tcl-dialect/src/expr_number.rs`; `rust/tcl-dialect/src/grammar.rs` | `parse`; `parse_whole_with`; `is_expr_number`; `scan_expr_number`; `scan_nan_payload`; `NumberSyntax` | `NumberSyntax` and expression-word grammar per release | `xtask-number-drift` |
| backslash escapes | `rust/tcl-lexer/src/substitution.rs`; `rust/tcl-syntax/src/backslash.rs`; `rust/tcl-dialect/src/grammar.rs` | `backslash_subst`; `backslash_subst_in`; `decode_bytes_in`; `EscapeSyntax` | `LexerGrammar::escapes` per release | none |
| boolean words | `rust/tcl-syntax/src/boolean.rs` | `parse_boolean_word`; `truthiness_with` | fixed boolean vocabulary; number axis per release | none |
| quotes / braces / word spans | `rust/tcl-lexer/src/ranges.rs` | `close_quote_offset`; `word_closer_offset`; `word_span_at`; `braced_var_name_end` | `${...}` close rule per release (`BracedVarStyle`); tmsh brace mode per dialect | none |
| indices | `rust/tcl-cmd-core/src/index.rs` | `resolve_with`; `drill` | grammar-parameterised, inheriting the number axis | none |
| option words / subcommands | `rust/tcl-cmd-core/src/prefix.rs`; `rust/tcl-registry/src/hover.rs`; `rust/tcl-registry/src/spec.rs` | `OptionTable`; `OptionSpec`; `SubCommand`; `first_positional_index` | option surface per release/dialect | `xtask-option-registry-drift` |
| sort numeric parsing | `rust/tcl-cmd-core/src/sort.rs` | `parse_wide`; `parse_real` | `NumberSyntax` per release | none |
| command errors | `rust/tcl-cmd-core/src/error.rs` | `CmdError`; `wrong_args`; `bad_choice` | invariant | none |
| expression grammar / evaluation | `rust/tcl-syntax/src/expr/parser.rs`; `rust/tcl-syntax/src/expr/eval.rs`; `rust/tcl-registry/src/expr_surface.rs` | `parse_expr`; `eval`; `RuntimeExprSurface` | `RuntimeExprSurface` per release | none |
| command / word segmentation | `rust/tcl-compiler/src/segmenter.rs` | `SegmentedCommand`; `segment_commands` | `LexerConfig` per document dialect | none |
| iRules execution boundaries and placement | `rust/tcl-syntax/src/event_handler.rs`; `rust/tcl-registry/src/events.rs`; `rust/tcl-registry/src/registry.rs`; `rust/tcl-irules/src/when_block.rs`; `rust/tcl-irules/src/executable.rs` | `event_handlers`; `event_handlers_with_head_predicate`; `script_commands`; `top_level_when_handlers_with_registry_and_head_resolver`; `IrulesDeclarationArguments`; `IrulesExecutionContext`; `IrulesCommandPlacement`; `IrulesTopLevelDeclaration`; `IrulesTopLevelEffect`; `CommandRegistry::irules_command_placement`; `CommandRegistry::irules_event_declaration`; `CommandRegistry::irules_top_level_declaration`; `CommandRegistry::irules_top_level_declaration_shape`; `CommandRegistry::irules_top_level_effect`; `when_blocks`; `irules_executable_commands` | caller-supplied `LexerConfig`; offset-keyed resolved command identity; exact single-braced declaration body; declaration-only top level; known-event roots; call-reachable procedure bodies; stateful priority (`0..=1000`, default 500) | `xtask-gen-ai-diagnostics` |
| text similarity | `rust/tcl-compiler/src/text.rs` | `edit_distance`; `rank_suggestions`; `rank_containment_suggestions` | invariant | none |
| per-command knowledge | `rust/tcl-registry/src/spec.rs`; `rust/tcl-registry/src/hooks.rs`; `rust/tcl-registry/src/registry.rs` | `CommandSpec`; `SubCommand`; `CommandRegistry` | per release/dialect | `xtask-command-backing` |
| dialect / release facts | `rust/tcl-dialect/src/profile.rs`; `rust/tcl-dialect/src/grammar.rs` | `DialectProfile`; `LexerGrammar`; `by_name` | the resolved dialect/release axis | none |
| shared plain types | `rust/tcl-core-types/src/diag_code.rs` | `DiagCode` | invariant | `xtask-diag-tables` |
<!-- end-owner-resolution-manifest -->

### `tcl-syntax` — the parse grammars and value seam

- `list` — the Tcl list codec. `find_element` is the single grammar
  primitive every splitter layers over (`split_list`,
  `split_list_raw`, and the lenient pair), and the **dict** string-rep
  scan in the WASM runtime walks it too — `SetDictFromAny` uses the
  same `FindElement` grammar and differs only in the noun it prints,
  which it composes from `junk_fragment`. On the join side,
  `append_list_element` is the byte entry point
  (`TclScanElement` + `TclConvertElement`); `list_element` and
  `join_list` are its `&str` facades, and the WASM runtime's list and
  dict string-rep generation binds it directly rather than carrying a
  second port (#1439).
- `number` — the `TclParseNumber` port (9.0-first: `0d` radix prefix,
  `_` digit separators, bare leading `0` is decimal), with
  `parse`/`parse_whole`/`parse_whole_with` (`ParseFlags` mirrors
  `TCL_PARSE_INTEGER_ONLY` etc.), `is_expr_number` (which delegates its
  boundary question below), and `format_double` (`Tcl_PrintDouble`).
- `glob` — `string match` globbing (`string_match`,
  `string_case_match`).
- `switch_body` — the one `switch` pattern/body-pair tokeniser (brace
  levels, comment rule, `-`-fallthrough), shared by the analyser,
  formatter, minifier, and semantic tokens.
- `naming` — `::`-qualified-name parsing and command resolution:
  `qualifier_segments` / `qualifier_segments_owned` (a colon **run** is
  one separator, mirroring `TclGetNamespaceForQualName`),
  `ends_with_separator`, `is_qualified`, `normalise_qualified_name`,
  `qualify`, `command_resolution_candidates` / `resolve_command_with`
  (the `Tcl_FindCommand` order, conformance-pinned), and the variable
  helpers (`normalise_var_name`, `split_array_name`,
  `split_element_ref` / `split_element_ref_bytes`, …).
  `split_element_ref` is `TclObjLookupVarEx`'s rule over an already
  **resolved** name (no `$` / `${...}` sigil stripping): array element
  iff longer than one byte, ends in `)`, and contains a `(`. The `(`
  may sit at offset 0, so a zero-length array name is legal — `set (x) 5`
  writes element `x` of the array named `""`. Both runtimes' element
  splitters bind it (#1458).
- `boolean` — `Tcl_GetBoolean` word recognition (unique prefixes of
  `true`/`yes`/`on`/`false`/`no`/`off`). Its prefix rule is *not* the
  option-table matcher below — boolean words have a fixed six-word
  vocabulary with cross-set ambiguity (`o`), so it stays here.
- `expr` — the expression AST, parser, evaluator seam, and walk
  (`ExprOps`, `mathfunc`).
- `backslash` — the byte-slice convenience over the lexer's decoder
  (see next); deliberately no second decode implementation.

### `tcl-dialect` — foundational expression grammar

- `tcl_dialect::scan_expr_number` — the lower `ParseLexeme` numeric-boundary
  owner. The lexer consumes its `ExprNumberLexeme`; `tcl-syntax` uses the same
  boundary before it classifies a token's value. It lives below both crates so
  neither can reintroduce a second `1_eq` / fractional-`_` scanner.
- `tcl_dialect::scan_nan_payload` — the 8.5+ `TclParseNumber` NaN payload
  state machine: ASCII whitespace is allowed around/between one through thirteen
  hexadecimal digits; a fourteenth digit invalidates the parenthesised form.
  The scanner and value parser share it.

### `tcl-lexer` — source-text decoding

- `backslash_subst` (re-exported as `tcl_syntax::backslash::decode`) —
  the one byte-exact `TclParseBackslash` port, shared by the
  LSP/compiler token pipeline and both runtimes.
- `ranges::braced_var_name_end` — the one release-aware `${...}` close
  rule (`Tcl_ParseVarName`), selected by `BracedVarStyle`: 9.x counts
  nested `{...}` and consumes `\X` as an inert pair, the 8.x family ends
  the name at the first literal `}`. The lexer's own `parse_var` /
  array-index scans and **both** `subst` engines resolve the form here;
  an engine that hard-codes one release's rule answers
  `subst {${a{b}c}}` wrongly on the other (#1457).

  Returns `BracedVarEnd` — `Closed(offset)` or `Unterminated` — **not** an
  `Option`. "No closer" is an error C names
  (`MISSING_CLOSE_BRACE_FOR_VAR`, owned here too), not a benign miss, and
  an `Option` let each consumer invent its own recovery: the VM emitted the
  whole `${...}` literally and the WASM runtime swallowed the rest of the
  template. Evaluating engines must raise; only a tokenizer may recover
  (the lexer runs the name to end-of-input so it can keep tokenizing
  half-typed source). The 9.x rule also *widens* what is unterminated —
  `${a\}` and `${a{b}` close under 8.x but not under 9.x.

  Scope — this owns the `subst`/tokenizer surface **only**, and the
  uncovered remainder is much larger than one path. A centralisation audit
  found, besides the compiled-word decoders (`segmenter` / `values` /
  `helpers`, #1568) and the runtime's script-word surface
  (`runtime/rust/src/parse.rs::build_word`), roughly **20 free-text
  "first `}` wins" scanners and 15 "strip the trailing `}`" helpers** spread
  across the compiler optimiser and analyser and `tcl-lsp-core`. Every one
  of them hard-codes a release rule this owner exists to decide. They are
  tracked as follow-ups rather than fixed here; do not read this entry as
  claiming the codebase has one `${...}` decoder — it has one *for the
  surfaces named above*.

### `tcl-cmd-core` — portable command logic

- `namespace` — the pure `::` byte-ops `tail` / `qualifiers`
  (`last_sep_run`: colon runs are one separator) plus the
  `Namespaces`-generic cores. Runtime name resolution routes through
  these — the VM's `interp.rs` canonicalisers (`canonical_cmd_key`,
  namespace declare/find/parent/import/forget) and `command.rs`
  (rename re-homing, `proc` namespace derivation) are built on them.
- `index` — Tcl index parsing (`Tcl_GetIntForIndex`: `end`, `end-2`,
  `1+1`) and nested-index drilling.
- `prefix` — the `Tcl_GetIndexFromObjStruct` port, with
  `prefix::OptionTable` as the one API: a const-constructible value
  generic over `AsRef<[u8]>` entries carrying a command's names in C
  table order, its error noun, and its abbreviation mode
  (`abbreviating` = C flags `0`; `exact_only` = `TCL_EXACT`).
  `resolve` applies C's rule (exact-match wins; unique non-empty
  prefix; ambiguous-vs-bad distinguished exactly as C words it,
  including the empty-key rule) and `index_of`/`index_of_str` attach
  the canonical miss message. The composing escape hatches stay
  public for sites that build their own sentence: `scan` (the
  noun-free rule), `choice_list_bytes` / `choice_list` (the `a`,
  `a or b`, `a, b, or c` enumeration with C's empty-entry quirks),
  and `bad_key_message` (byte nouns — the runtime's `tcl::prefix
  match` `-message`). Consumers: `switch`/`lsort`/`lsearch`/`regexp`/
  `regsub`/`trace`/`string is` option words (this crate), the VM's
  `tcl::prefix match` and `string is`, the WASM runtime's `string`
  ensemble, `tcl::prefix match`, and OO option tables. New command
  modules MUST resolve through `OptionTable` (or `scan` +
  `bad_key_message` where a byte noun or interleaved control flow
  demands composition) — never a hand-rolled scan.
- `sort::parse_wide` / `sort::parse_real` — the `-integer` / `-real`
  key parsers (`parse_wide` is the whole-string integer-only shape of
  `tcl_syntax::number`, `i128`-wide; `binary`'s wide parse narrows it
  by wrapping, matching C's `binary format`).
- `error::CmdError` — the canonical error-message catalogue
  (`wrong_args`, `bad_choice`, …). The runtimes' arity helpers are
  thin adapters: `runtime/rust`'s single `Interp::wrong_args` method
  and the VM's `interp::err_wrong_args`.

### `tcl-compiler` — text similarity

- `text` — `edit_distance` (optimal string alignment over chars),
  `suggest_similar` and the ranking cores `rank_suggestions`
  (ascending `(score, name)`, capped) and
  `rank_containment_suggestions` (exact > prefix > substring).
  Consumers: every did-you-mean suffix (W001/W123/W210/W212/W215,
  E001), completion's fuzzy fallback, and the package-suggestion
  ranking in code actions. Re-homing into `tcl-syntax` was assessed
  July 2026 and declined — no compiler-independent consumer exists;
  revisit only if one appears.

### `tcl-syntax` — event-handler boundaries

- `event_handlers` is the dependency-low extractor for live
  `when EVENT { … }` handlers in one supplied script region;
  `event_handlers_with_head_predicate` lets a higher layer supply the resolved
  command identity without making `tcl-syntax` depend on compiler facts.
  `script_commands` exposes the same lexer-owned word boundaries without
  guessing that arbitrary braced values are executable.
  `tcl_registry::events::top_level_when_handlers_with_registry_and_head_resolver`
  resolves each top-level head at its absolute document offset before accepting
  an event handler, using the resolved profile's lexer grammar. Nested script
  surfaces remain handler data: F5 iRules permits `when` only at the top level.
  Rooted colon runs, event case, comments, quoting, and iRules' `}{` separator
  therefore have one lexer/naming contract. `tcl-irules::when_blocks` is the
  iRules-configured wrapper. Registry, CLI, explorer, LSP, and MCP consumers
  use these APIs and spans rather than scanning text.
- `IrulesDeclarationArguments` carries the decoded values beside each
  argument's lexer token and single-word fact. The registry uses it to accept
  an iRules `when` or `proc` declaration only when its body is one braced
  source word; bare, quoted, and compound bodies cannot create lowering,
  symbol, diagnostic, or executable-inventory regions. The shape query keeps
  an otherwise valid unknown event available to IRULE1002, while the
  known-event query excludes it from executable roots.
- The registry wrapper also owns iRules priority state. `priority N` changes
  the inherited priority of subsequent event declarations, an inline
  `when EVENT priority N` overrides only that handler, and an omitted priority
  inherits 500 until changed. Valid priorities are `0..=1000`; lower values
  run first. Repeated handlers for one event remain distinct, and equal-priority
  handlers preserve source insertion order. Cross-file ties preserve the
  virtual server's iRule attachment order at the host boundary.
- `IrulesExecutionContext` and `IrulesCommandPlacement` own the other half of
  that boundary: the iRules top level is declaration-only (`when`, `proc`,
  `timing`, and `priority`), while executable commands belong in event or
  procedure bodies. The analyser supplies lexical context and consumes the
  registry decision for IRULE5005, IRULE5006, and IRULE5007; it does not keep
  a second command-name allow-list.
- `tcl_irules::irules_executable_commands` is the inventory view of that same
  contract. It follows registry-declared bodies, clause lists, and command
  substitutions inside valid top-level event and procedure declarations while
  treating comments, ordinary Tcl data, invalid top-level execution, and
  nested declarations as inert.

### `tcl-core-types` — shared vocabulary

- The crate for cross-runtime plain types (diagnostic codes today).
  Anything two crates must *name* identically without depending on
  each other's machinery lands here.

## Decision rules / contracts

1. Consumers must not re-derive these utilities. A new `split("::")`,
   integer scanner, option-prefix loop, or `wrong # args:` /
   `bad option` format string in a consumer crate is a review defect —
   call the owner (or extend it) instead.
2. Byte/`&str` duality is handled by the owner: the canonical
   implementation is byte-based where both runtimes need it
   (`tcl-cmd-core::namespace`, `::prefix`; `tcl_syntax::naming::
   qualifier_segments`), with `&str` conveniences layered on top —
   never a parallel string-side re-implementation.
3. Namespace-name splitting must be separator-**run** aware everywhere
   (a run of 2+ colons is one separator; a lone `:` is an ordinary name
   character). Command names keep a trailing run as the `{}`-named
   entity (`proc quux::: …` defines `::quux::`); namespace names drop
   it (`namespace eval c::: {}` creates `::c`). Both tclsh-verified.
4. Message texts come from the owner so they stay byte-identical to C
   Tcl across runtimes (`tcl-cmd-core::prefix` for bad/ambiguous
   option, `CmdError` for arity). Adapters may *prefix* (see the OO
   exception below) but never re-spell the core text.
5. Grammar direction is 9.0-first by design: the shared number grammar
   accepts `0d5` and `1_000` even though 8.6 rejects them (dialect
   gating is a lexer/analyser concern, not a per-consumer parser fork).

## Known deliberate exceptions

- `string match` / `string map` `-nocase`: C hand-rolls a
  `length > 1` prefix test (`strncmp`) instead of
  `Tcl_GetIndexFromObj`, which differs from the table rule on a lone
  `-` (`string match - a b` is `bad option`, where the table rule
  would call it ambiguous) — kept hand-rolled, with the probe cited at
  the site (`tcl-cmd-core::string`).

Each of these is a *documented* divergence — keep the comment at the
site pointing back here, and do not "fix" them onto the canonical
helper without reading the rationale:

- `rust/tcl-registry/src/const_fold.rs::split_list` — a conservative
  *fold-safety* splitter that bails on any backslash or bare
  `{`/`}`/`"`, so the optimiser only folds provably-simple lists. Using
  the canonical `tcl_syntax::list::split_list` would fold **more**
  (changing optimiser output); the policy is local on purpose.
- `rust/tcl-compiler/src/codegen/values.rs::is_bare_var_name` — the
  looser codegen-side contract: the run of characters a `$name`
  reference consumes (alphanumerics, `_`, **any** `:`), used to decide
  substitution shape — not the stricter `::`-segmented
  `tcl_syntax::naming::is_bare_var_name` that quick fixes use to keep
  `${x}` ↔ `$x` rewrites meaning-preserving. It is `pub(crate)`, and
  `codegen::values::whole_var_reference` is the **single owner** of the
  `$name` / `${name}` whole-word extractor built on it: given a word, the
  exact variable name it refers to, or `None` when the word is not one
  simple reference. `codegen::wasm::backend` (`emit_word_value`, the
  `ChannelWrite` argument guard) and `codegen::wasm::leaf_invoke`
  (`plan_variable`) consume it; each carried a byte-identical private copy
  until issue #1459.
  The braced and bare spellings are validated **differently** and must
  stay that way: `${…}` accepts any non-empty name verbatim (braces are
  Tcl's own escape for a name the bare charset cannot express), while a
  bare `$name` must be a whole `is_bare_var_name` run. Routing the braced
  arm through the charset check as well changes behaviour, not just
  duplication — pinned by
  `whole_var_reference_accepts_any_non_empty_braced_name`.
  The release-aware `${…}` *close* rule is a separate question, owned by
  `tcl_lexer::ranges::braced_var_name_end` and consumed by the decoders
  (`parse_simple_var_ref`, `parse_subst_template`), not here.
- `runtime/rust/src/cmd_oo.rs::wrong_args` — wraps the shared
  `Interp::wrong_args` but prepends the active `oo::define`
  ensemble-rewrite prefix, so single-command definition forms report
  the whole original command (`oo::define Foo method …`) as C's
  `Tcl_WrongNumArgs` rewrite path does.
- `rust/tcl-vm/src/interp.rs::oxford_or` — the **ensemble** subcommand
  enumeration, which C renders with a comma before `or` even for two
  items (`x1, or x2`), unlike `Tcl_GetIndexFromObj`; it must not be
  collapsed onto `tcl-cmd-core::prefix::choice_list`.
- The LSP-side matchers (semantic-tokens / minify candidate ranking)
  and `tcl_syntax::boolean` keep their own prefix rules — different
  contracts (ranking, fixed vocabulary with cross-set ambiguity), not
  option-table lookup.

## Failure modes

- Colon-run names resolve differently between the compiler, the VM,
  and the WASM runtime (`foo:::bar` dispatches in one and errors in
  another).
- `lsort -integer` and `binary format` disagree on which strings are
  integers.
- `bad option` / `ambiguous option` texts drift from tclsh by a comma
  or an empty-table wording (`no valid options` vs a pluralised noun).
- An abbreviation resolves in one runtime and is ambiguous in the
  other.
- The optimiser folds a list the runtime would split differently.

## Test anchors

- `rust/tcl-syntax/src/naming.rs` — `qualifier_segments_cases`,
  doctests; `rust/tcl-syntax/tests/command_resolution_conformance.rs`
  (tclsh-pinned; `tcl-compiler` and `tcl-vm` each carry a same-named suite
  for their own layer).
- `rust/tcl-cmd-core/src/namespace.rs` — `qualifiers_and_tail_match_c`.
- `rust/tcl-cmd-core/src/prefix.rs` — C-parity unit tests (empty-key,
  empty-entry, exact-mode wording).
- `rust/tcl-cmd-core/src/sort.rs` —
  `parse_wide_shares_the_canonical_integer_grammar`.
- `rust/tcl-vm/tests/namespace_colon_runs_e2e.rs` — colon-run
  resolution/creation pinned against tclsh8.6.
- `rust/tcl-vm/tests/cmd_info_prefix_e2e.rs` — `tcl::prefix` message
  texts pinned against tclsh.
- `rust/tcl-compiler/src/interprocedural.rs` —
  `namespace_parts_from_proc_extracts_segments` (colon-run rows).
- `rust/tcl-syntax/src/list.rs` — `list_element_matches_tcl9` (the single
  join parity table, merged from both former ports) and
  `append_list_element_is_byte_exact_and_agrees_with_the_str_api`.
- `runtime/rust/src/dict.rs` —
  `backslash_newline_absorbs_the_following_space_run` and
  `delimiter_errors_keep_their_dict_wording_and_fragment` (the dict scan
  riding the shared list codec).
- `rust/tcl-vm/tests/cmd_collections_e2e.rs` —
  `duplicate_dict_keys_canonicalise_last_value_wins` (every VM dict path
  through `ValueOps::dict_pairs`, including the compile-time
  `dict create` fold) and
  `dict_parse_errors_use_the_dict_noun_and_error_code` (#1573);
  `rust/tcl-vm/tests/cross_version_command_surface_e2e.rs` — the
  *foldable* `dict create` availability vector.
- `rust/tcl-lexer/src/ranges.rs` —
  `braced_var_name_end_follows_the_release_rule`;
  `rust/tcl-vm/tests/cross_version_vars_e2e.rs` —
  `subst_braced_var_close_rule_follows_the_emulated_release`,
  `unterminated_braced_var_raises_on_both_releases` and their
  tclsh-pinned sibling; `runtime/rust/src/subst.rs` —
  `braced_var_close_rule_follows_the_emulated_release`;
  `runtime/rust/src/builtins.rs` —
  `unterminated_braced_var_raises_missing_close_brace`.
- `rust/tcl-vm/tests/language_e2e.rs` —
  `zero_length_array_name_is_an_array_element` and
  `link_commands_reject_element_looking_names` (`split_element_ref`).

## Discoverability

- [KCS index](../README.md)
- [project-layout.md](project-layout.md) — the crate boundaries these
  ownership rules sit inside.
- [family-b-routing.md](../family-b-routing.md) — the runtime seam this
  crate layering serves.
