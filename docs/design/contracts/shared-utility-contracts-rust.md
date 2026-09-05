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
| names / namespaces | `rust/tcl-syntax/src/naming.rs`; `rust/tcl-cmd-core/src/namespace.rs` | `qualifier_segments`; `command_resolution_candidates`; `qualifiers`; `tail`; `exists`; `exists_bytes`; `parent`; `parent_bytes`; `children`; `children_bytes`; `which_request`; `which_command`; `which_command_bytes`; `which_variable`; `variable_fqn`; `variable_fqn_bytes`; `import_pattern`; `origin`; `origin_bytes` | invariant, except `which_variable`'s alternate (global) candidate, which 9.0 drops; absolute-marker contract from #1493 | `xtask-resolution-drift` |
| lists | `rust/tcl-syntax/src/list.rs` | `find_element`; `split_list`; `list_element`; `join_list`; `append_list_element`; `junk_fragment` | invariant | none |
| dicts | `rust/tcl-syntax/src/list.rs`; `rust/tcl-syntax/src/value.rs`; `rust/tcl-cmd-core/src/dict.rs` | `find_element`; `split_list`; `canonical_dict_slots`; `ValueOps::dict_pairs`; `worded_parse_error` | invariant | none |
| glob matching | `rust/tcl-syntax/src/glob.rs` | `string_match`; `string_match_bytes`; `string_case_match` | invariant | none |
| switch body grammar | `rust/tcl-syntax/src/switch_body.rs` | `tokenise_switch_body`; `parse_braced_pairs` | invariant | none |
| numbers | `rust/tcl-syntax/src/number.rs`; `rust/tcl-dialect/src/expr_number.rs`; `rust/tcl-dialect/src/grammar.rs` | `parse`; `parse_whole_with`; `is_expr_number`; `scan_expr_number`; `scan_nan_payload`; `NumberSyntax` | `NumberSyntax` and expression-word grammar per release | `xtask-number-drift` |
| backslash escapes | `rust/tcl-lexer/src/substitution.rs`; `rust/tcl-syntax/src/backslash.rs`; `rust/tcl-dialect/src/grammar.rs` | `backslash_subst`; `backslash_subst_in`; `decode_bytes_in`; `EscapeSyntax` | `LexerGrammar::escapes` per release | none |
| boolean words | `rust/tcl-syntax/src/boolean.rs` | `parse_boolean_word`; `truthiness_with` | fixed boolean vocabulary; number axis per release | none |
| quotes / braces / word spans | `rust/tcl-lexer/src/ranges.rs` | `close_quote_offset`; `word_closer_offset`; `word_span_at`; `braced_var_name_end` | `${...}` close rule per release (`BracedVarStyle`); tmsh brace mode per dialect | none |
| array-index source scan | `rust/tcl-lexer/src/ranges.rs`; `rust/tcl-dialect/src/grammar.rs` | `scan_array_index`; `ArrayIndexSyntax` | `LexerGrammar::array_index` per release | none |
| word substitution components | `rust/tcl-lexer/src/word_parts.rs` | `decompose`; `decompose_spanned`; `scan_var_ref`; `command_subst_close`; `quoted_word_close`; `SubstFlags`; `WordPart`; `SpannedPart`; `WordBody`; `VarRef`; `RawVarRef`; `MISSING_QUOTE`; `MISSING_CLOSE_BRACKET`; `MISSING_CLOSE_BRACE`; `MISSING_PAREN`; `EXTRA_AFTER_CLOSE_BRACE` | `LexerConfig` per emulated release (`${...}` close rule, array-index source mask, escape grammar); compiled-word vs source-word `$` spelling | none |
| indices | `rust/tcl-cmd-core/src/index.rs` | `resolve_with`; `drill` | grammar-parameterised, inheriting the number axis | none |
| option words / subcommands | `rust/tcl-cmd-core/src/prefix.rs`; `rust/tcl-cmd-core/src/ensemble.rs`; `rust/tcl-registry/src/hover.rs`; `rust/tcl-registry/src/spec.rs` | `OptionTable`; `OptionSpec`; `SubCommand`; `first_positional_index`; `ensemble::EnsembleToken`; `ensemble::InvocationLayout`; `ensemble::invocation_layout`; `ensemble::UNKNOWN_DELETED_MESSAGE`; `ensemble::UNKNOWN_DELETED_ERROR_CODE`; `ensemble::CREATE_OPTIONS`; `ensemble::CONFIG_OPTIONS`; `ensemble::SUBCOMMANDS`; `ensemble::resolve_subcommand`; `ensemble::subcommand_choices`; `ensemble::unknown_subcommand_message`; `ensemble::validate_map_targets` | option surface per release/dialect; ensemble token lifecycle and invocation layout invariant | `xtask-option-registry-drift` |
| trace argument decoding | `rust/tcl-cmd-core/src/trace.rs` | `TraceKind`; `resolve_option`; `resolve_type`; `parse_ops`; `parse_legacy_variable_ops`; `legacy_ops_letters`; `callback_op_word` | option surface per release (the 8.x-only `variable`/`vdelete`/`vinfo` forms) | none |
| sort numeric parsing | `rust/tcl-cmd-core/src/sort.rs` | `parse_wide`; `parse_real` | `NumberSyntax` per release | none |
| command errors | `rust/tcl-cmd-core/src/error.rs` | `CmdError`; `wrong_args`; `bad_choice` | invariant | none |
| expression grammar / evaluation | `rust/tcl-syntax/src/expr/parser.rs`; `rust/tcl-syntax/src/expr/eval.rs`; `rust/tcl-registry/src/expr_surface.rs` | `parse_expr`; `eval`; `RuntimeExprSurface` | `RuntimeExprSurface` per release | none |
| expr math functions and the `rand` generator | `rust/tcl-syntax/src/expr/mathfunc.rs`; `rust/tcl-syntax/src/expr/rand.rs` | `NumValue`; `dispatch`; `dispatch_with_backend_int_width`; `try_dispatch_with_backend_int_width`; `IntWidth`; `MathFuncError`; `MathFuncSince`; `spec`; `all`; `added_in`; `seed_from_wide`; `next_draw`; `seed_and_draw` | `MathFuncSince` per release for the function surface and `IntWidth` for `int()`'s width; the Park-Miller generator is release-invariant | none |
| command / word segmentation | `rust/tcl-lexer/src/script.rs`; `rust/tcl-compiler/src/segmenter.rs`; `rust/tcl-compiler/src/parsing/syntax/build.rs`; `rust/tcl-compiler/src/parsing/syntax/segment.rs` | `group_commands`; `CommandSpan`; `WordSpan`; `WordKind`; `SegmentedCommand`; `segment_commands` | `LexerConfig` per document dialect | `xtask-segmentation-drift` |
| nested command-substitution words | `rust/tcl-compiler/src/word_subst.rs` | `nested_command_words`; `NestedWordsDecline`; `lifted_calls`; `lifted_exprs`; `LiftedCall` | `LexerConfig` per document dialect, inherited from the segmentation owner it runs | none |
| parse-error cut | `rust/tcl-lexer/src/parse_cut.rs` | `first_parse_cut`; `first_parse_cut_in`; `ParseCut`; `EXTRA_AFTER_CLOSE_QUOTE` | `LexerConfig` per emulated release, inherited from the segmentation and word-component owners it walks | `xtask-segmentation-drift` |
| script completeness / reparse windows | `rust/tcl-lexer/src/structural_index.rs` | `script_is_complete`; `command_boundaries`; `reparse_window`; `BracketIndex`; `BraceIndex`; `ExprParenIndex`; `ParenBalance` | dialect-blind by construction: one byte scan of stock 8.6/9.x brace, quote, `${…}`-nesting, comment and terminator structure, so an editor keystroke costs no tokenise. The two grammar axes that really do move a command boundary — the F5 `BraceLineContinuation::Continues` next-line-`{` rule and the 8.x `BracedVarStyle::FirstClose` name rule — are pinned as measured divergences by `differential_boundaries`, never silently absorbed | `xtask-segmentation-drift` |
| iRules execution boundaries and placement | `rust/tcl-syntax/src/event_handler.rs`; `rust/tcl-registry/src/events.rs`; `rust/tcl-registry/src/registry.rs`; `rust/tcl-irules/src/when_block.rs`; `rust/tcl-irules/src/executable.rs` | `event_handlers`; `event_handlers_with_head_predicate`; `script_commands`; `top_level_when_handlers_with_registry_and_head_resolver`; `IrulesDeclarationArguments`; `IrulesExecutionContext`; `IrulesCommandPlacement`; `IrulesTopLevelDeclaration`; `IrulesTopLevelEffect`; `CommandRegistry::irules_command_placement`; `CommandRegistry::irules_event_declaration`; `CommandRegistry::irules_top_level_declaration`; `CommandRegistry::irules_top_level_declaration_shape`; `CommandRegistry::irules_top_level_effect`; `when_blocks`; `irules_executable_commands` | caller-supplied `LexerConfig`; offset-keyed resolved command identity; exact single-braced declaration body; declaration-only top level; known-event roots; call-reachable procedure bodies; stateful priority (`0..=1000`, default 500) | `xtask-gen-ai-diagnostics` |
| text similarity | `rust/tcl-compiler/src/text.rs` | `edit_distance`; `rank_suggestions`; `rank_containment_suggestions` | invariant | none |
| per-command knowledge | `rust/tcl-registry/src/spec.rs`; `rust/tcl-registry/src/hooks.rs`; `rust/tcl-registry/src/registry.rs` | `CommandSpec`; `SubCommand`; `CommandRegistry` | per release/dialect | `xtask-command-backing` |
| dialect / release facts | `rust/tcl-dialect/src/profile.rs`; `rust/tcl-dialect/src/grammar.rs`; `rust/tcl-dialect/src/version.rs`; `rust/tcl-dialect/data/reference-toolchains.tsv` | `DialectProfile`; `LexerGrammar`; `TclVersion`; `TclVersion::patchlevel`; `TclVersion::reference_source_tag`; `find` | the resolved dialect/release axis plus exact pinned reference patchlevel/source tag | `xtask-editor-extensions` |
| C Tcl conformance oracles | `rust/tcl-test-support/src/lib.rs` | `reference_patchlevel`; `reference_source_tag`; `locate_tclsh`; `available_tclshs`; `run_script`; `locate_source_tree`; `Tclsh`; `TclSourceTree`; `ScriptOutcome` | exact interpreter/source agreement and provenance for the selected release line | none |
| interpreter platform bootstrap | `rust/tcl-platform/src/lib.rs` | `bootstrap::Values`; `bootstrap::Snapshot`; `bootstrap::snapshot`; `bootstrap::entries`; `bootstrap::HOST_ARRAYS`; `bootstrap::HOST_PATH_GLOBALS`; `bootstrap::safe_scrub_keys`; `bootstrap::SHARED_LIBRARY_EXTENSION` | key, selected-host snapshot, rebootstrap-clear, safe-scrub, and canonical Unix shared-library suffix invariant; runtime identity supplied per engine | none |
| shared plain types | `rust/tcl-core-types/src/diag_code.rs` | `DiagCode` | invariant | `xtask-diag-tables` |
| SslicTcl declaration model | `rust/tcl-sslictcl/src/model.rs` | `SslicModel`; `TlsFacts`; `Policy` | vocabulary version (`dsl::SUPPORTED_VOCABULARY`); no Tcl release axis — the document is never evaluated | none |
| SslicTcl document loading | `rust/tcl-sslictcl/src/dsl.rs`; `rust/tcl-sslictcl/src/vocabulary.rs` | `load_with_diagnostics`; `DslDiagnostic`; `DECLARATIONS` | vocabulary version; open/closed block rule per declaration | none |
| SslicTcl finding identity | `rust/tcl-sslictcl/src/policy.rs` | `evaluate_policy`; `PolicyFinding` | invariant `(check id, endpoint)` identity; the `grade` id is reserved | none |
| SslicTcl embedded source data | `rust/tcl-sslictcl/src/trust.rs` | `embedded_dataset` | pinned upstream revisions, recorded with hashes and licences in `data/provenance.json` | `xtask-sslictcl-data` |
| SslicTcl declaration surface | `rust/tcl-registry/src/commands/sslictcl/mod.rs`; `rust/tcl-registry/src/definer.rs` | `sslictcl_command_specs`; `SSLICTCL_GRAMMARS` | the `sslictcl` authoring surface (`SpecSurface::SSLICTCL`); Tcl 9.0 core underneath | none |
| SslicTcl editor projection | `rust/tcl-lsp-core/src/sslictcl_diagnostics.rs`; `rust/tcl-lsp-core/src/declaration_outline.rs` | `applies_to`; `diagnostics`; `SUPERSEDED_ANALYSER_CODES`; `supersede_analyser_diagnostics`; `is_declaration_document`; `declarations` | resolved authoring surface (the `sslictcl` package) per document | none |
<!-- end-owner-resolution-manifest -->

### `tcl-dialect` + `tcl-test-support` — C Tcl reference toolchains

- The `tcl-dialect/data/reference-toolchains.tsv` manifest is the
  language-neutral owner for the five pinned C Tcl patchlevels and their
  upstream Tcl/Tk source tags. `tcl-dialect` generates `TclVersion`'s release
  facts from it at build time; `tcl-test-support` reuses those APIs for oracle
  provenance, while the POSIX shell adapter under `scripts/dev` supplies the
  same rows to ensure-test-deps, the source-fetch skill, and remote-session
  bootstrap.
- Default/PATH oracle resolution requires the exact pinned patchlevel and
  records the interpreter's reported value as provenance. An explicitly
  paired source-tree interpreter may name another patchlevel on the same
  release line, but its binary and `generic/tcl.h` must agree exactly.
- The check-tcl-reference-toolchains Make target runs the hermetic
  stale-interpreter regression (including `/bin/sh` adapter execution) and the
  Rust all-axis release-fact test.

### `tcl-platform` — predefined platform surface

- `bootstrap::entries` is the one schema for the predefined
  `tcl_platform` array in both interpreters. Constants live in the schema;
  machine, user, operating-system version, engine identity, and backend facts
  enter through `bootstrap::Values`. `bootstrap::snapshot` captures those
  values together with the selected host's environment and Tcl library path,
  so neither engine may read the process host behind an embedder's selected
  `Host`.
- `bootstrap::HOST_ARRAYS` and `bootstrap::HOST_PATH_GLOBALS` define the whole
  stale surface an engine clears before installing a replacement snapshot.
  `Interp::with_host` avoids a native-host bootstrap entirely; both engines'
  `set_host` paths replace this surface, and normal children inherit their
  parent's host before their first bootstrap.
- `bootstrap::safe_scrub_keys` is derived from those same entries. It follows
  Tcl 9's `Tcl_MakeSafe` distinction: identity-bearing `os`, `osVersion`,
  `machine`, and `user` are removed; portable facts, including `threaded`,
  remain. The project-specific runtime/WASM/WASI/eBPF facts are also removed.
- `bootstrap::SHARED_LIBRARY_EXTENSION` is the one suffix exposed by both
  engines' `info sharedlibextension` implementations. It belongs beside the
  canonical Unix `tcl_platform(platform)` fact rather than in either engine's
  command adapter; real `init.tcl` package-index discovery reads it while
  rejecting Windows-only packages.

  Consumers: `tcl-vm::Vm::rebootstrap_host_globals` and
  `tcl_runtime::Interp::rebootstrap_host_globals` install the snapshot;
  each engine's `make_safe` consumes the derived scrub iterator. A fresh
  tree-walk `Interp`, its normal children, and bytecode-VM children all install
  the surface before any `init.tcl` work.

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

  Consumers: `tcl-vm` (`interp::elem_ref`, `command::looks_like_element`,
  `command::parent_namespace_of`, `exec`'s `ARRAY_MAKE_STK` guard),
  `tcl-compiler` (`codegen::values::split_array_ref` / `is_array_ref`, the
  facade the whole codegen layer calls, and
  `analyser::diagnostics::var_command`'s array-element harvest), and — inside
  the owner's own file — `split_array_name_for_style` and
  `split_array_name_braced_for_style`. The last four re-derived the split
  rather than calling it until #1606; none could diverge on any input
  (`ends_with(')') && contains('(')` cannot be satisfied by a one-byte name),
  which is exactly why the drift went unnoticed. `cargo xtask
  owner-resolution` validates the manifest, not call graphs, so it cannot
  catch a listed consumer that never calls its owner.

  Deliberately **not** a consumer: `tcl-compiler`'s
  `sccp::array_element_base` is a narrower fold-safety predicate (documented
  at the site) that excludes the zero-length array name this owner admits.
- `boolean` — `Tcl_GetBoolean` word recognition (unique prefixes of
  `true`/`yes`/`on`/`false`/`no`/`off`). Its prefix rule is *not* the
  option-table matcher below — boolean words have a fixed six-word
  vocabulary with cross-set ambiguity (`o`), so it stays here.
- `expr` — the expression AST, parser, evaluator seam, and walk
  (`ExprOps`, `mathfunc`). The math-function table is one dispatch over
  `NumValue<B>` for both engines and the const-folder, with the two release
  axes (`MathFuncSince`, `IntWidth`) and the typed refusals (`MathFuncError`)
  owned here rather than re-derived per engine.
- `rand` — the Park-Miller `rand()`/`srand()` generator both engines call
  (step, seed nudge, and C's reciprocal-multiply scaling). Only seed storage
  and the first-seed policy are per engine (#1432).
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

- `tcl_dialect::DialectProfile::find` — the one **catalogue** lookup left on
  the profile (P1-G): canonical name or registered alias to interned profile,
  `None` for everything else. It is what the environment seam and the
  documented per-crate interop twins are built on, and it resolves an
  environment id, never a user-written string. The retired name validators
  (`by_name`, `by_opt_name`, `resolve_known`, `availability_for_name`) are
  deleted; every user-written dialect name resolves through the one seam,
  `tcl_registry::model::ingress::resolve_environment` (with
  `resolve_known_environment` as the validator form), whose
  `analyser_profile` / `unit_profile` faces reproduce the old sink and
  `tk`-promotion policies exactly (pinned in the seam's tests). The
  `retired-api-gate` holds the deleted spellings at zero. See issue #1405
  and the centralisation ledger §3.

- `tcl_dialect::DialectProfile`'s `file_extensions` and `filenames` — the two axes
  of **file** recognition, and the single source every editor's registration
  list and every extension→dialect route projects. They answer different
  questions and neither substitutes for the other: `file_extensions` claims a
  trailing suffix (`xdc` → `xilinx-eda-tcl`), while `filenames` claims a whole
  basename (`bigip.conf` → `f5-bigip`) because the files it names have no
  suffix worth claiming — a bare `.conf` belongs to every unrelated config file
  on the machine. `tcl_registry::dialects::dialect_from_extension` consults the
  basename axis **first** (a name claim is the more specific one), and
  `cargo xtask gen-editor-extensions` projects both into every editor. Both are
  one-owner-per-key across the catalogue, invariant-tested in `profile.rs`.
  A consumer that restates either list rather than projecting it is the drift
  issue #1625 catalogued: six hand-maintained surfaces, three of them wrong.

- Command availability for a *document* is asked at one **point** — the
  resolved environment's authoring query
  (`tcl_registry::model::ingress::DocumentEnvironment::document_context`,
  then `ResolvedContext::authoring_query`, as
  `static_document_context_for` returns it). It is what keeps the additive
  `tk` ingress working: the `tk` environment's point names `Tk` among its
  packages even though the analyser-facing fallback's does not — the union
  the retired `availability_for_name` used to compute at the string
  boundary, now a fact of the resolved environment.

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

  Scope — the surfaces consolidated on this owner are now the
  `subst`/tokenizer surface (#1457), the compiled-word decoders
  (`segmenter` / `values` / `helpers`, #1568), the **expression** sub-lexer
  `expr_lexer::variable` (#1601 — an `expr` body is parsed out of an
  ordinary Tcl word, so `expr {${a{b}c} + 1}` must resolve the reference
  exactly as `if {${a{b}c} > 3}` does), and, from #1604, the free-text
  scanners in `tcl-syntax::naming` and across the compiler optimiser,
  dataflow, taint and analyser layers.

  A caller **passes the resolved `BracedVarStyle` down**; it does not
  re-derive the closer. `tcl_syntax::naming` exposes `split_braced_var_ref`
  plus `*_for_style` entry points for every reader that unwraps `${...}`
  (`normalise_var_name`, `var_reference`, `element_var_name`,
  `split_array_name` and their `_braced` variants) — the no-argument
  spellings take `BracedVarStyle::default()`, which is the rule a document
  with no explicit dialect is lexed under, and a caller that has resolved
  the document's dialect must use the `_for_style` form. In the compiler the
  style comes from the layer's existing dialect view:
  `optimiser::PassContext::braced_var`, `analyser::Analyser::braced_var`,
  the taint `TaintCtx` / `TaintScan` / `SinkCall`, `ScanCtx`'s `LexerConfig`,
  the `Lowerer`'s `config`. In `tcl-lsp-core` it comes from the resolved
  `DialectProfile` the rename entry points already carry.

  A **defaulting convenience overload beside a style-taking one is a trap**,
  not a courtesy: `dynamic_names::dynamic_variable_word_can_spell` had one,
  and all three production callers (`rename_safety` twice,
  `namespace_rename` once) silently took it although each held a resolved
  profile. The style is a required parameter there now, so it cannot be
  omitted by accident (PR #1645 review). Where a defaulted spelling *is* kept
  — the `naming` readers, whose callers number in the hundreds — the rule is
  that a caller holding a resolved dialect must use the `_for_style` form;
  the default is for a document that genuinely has no dialect, not a
  shorthand for one that does.

  That gate shows why the direction matters as much as the rule. The literal
  characters around a substitution bound which cells a computed name can
  spell, so the two rules move a rename decision opposite ways: 9.x reads
  `${a{b}c}` as one wildcard that can spell anything (refuse the rename),
  while 8.x ends the name at the first `}` and leaves the literal `c}`
  (provably out of reach, allow it). Reading an 8.x document with the 9.x
  default refuses a rename that is provably safe; reading a 9.x document with
  the 8.x rule lets an unsafe one through.

  Two classes of site are deliberately **not** threaded, and both are
  documented in place so they are not "fixed" back into plumbing that cannot
  change an answer:

  - a scan whose own gate rejects every name the two rules can disagree
    about. The rules differ only on names containing `{`, `}` or `\`, and
    `analyser::param_traits::extract_var_name` accepts only
    `[A-Za-z_][A-Za-z0-9_:]*` while `subst_nocommands`'
    `is_complex_var_name` accepts only alphanumerics and `_`. Threading a
    style into either could not change an answer — a mutant pinning them to
    `FirstClose` survives, which is the proof — so both carry the reasoning
    in place instead of the parameter;
  - an entry point with no document profile in scope at all
    (`auto_path_eval`, `specialise_factories`), which passes
    `BracedVarStyle::default()` explicitly rather than silently.

  Still uncovered, tracked as follow-ups: `value_shapes`'
  `scan_pure_var_ref` / `is_braced_whole_name_array_ref` /
  `whole_word_scalar_var_name` — whose combined ~45 callers make them their
  own sweep, and each of which currently *declines* on a divergent shape
  rather than answering wrongly. Do not read this entry as claiming the
  codebase has one `${...}` decoder — it has one *for the surfaces named
  above*.

- `parse_cut::first_parse_cut` — the one answer to *where a script stops
  parsing*: which command C rejects, at what offset, with which message.
  It walks `group_commands` and `word_parts` in C's own order — commands,
  then words, then components, descending into `[…]` bodies and
  `$arr(index)` — rather than filtering the lexer's flat warning stream,
  which is what made `list [sfx one] [list "oops]` answer
  `missing close-bracket` where C says `missing "`. The command index is
  the part a message alone cannot carry, and it is what a bytecode
  front-end needs to compile the prefix that runs before the raise
  (#1603).

- `word_parts::decompose` — the one splitter of a Tcl word (or a `subst`
  template) into its substitution components: C's `ParseTokens` breakdown
  into text runs with their backslash escapes folded in, `$name` /
  `${name}` / `$arr(index)` references, and `[script]` substitutions, with
  the `{*}` expansion flag and the parse errors C names carried alongside.

  It exists because that walk had **four** implementations that drifted
  (bucket R10): `runtime/rust/src/parse.rs`'s `scan_parts`, that crate's
  `subst.rs` mirror, `tcl-vm`'s `subst.rs`, and the compiler's
  `segmenter.rs` / `ir.rs` `WordExpr` builder. Only one raised C's `missing
  close-bracket`; only one found a `]` without being fooled by a brace,
  quote or comment in the substituted script; only one decoded a literal
  run's escapes. All four are consumers now: `runtime/rust`, `tcl-vm`, and
  — since #1785 — the compiler, whose `WordExpr` builder moved out of
  `ir.rs`'s fragment walk into `word_expr.rs` over `decompose_spanned`.
  The segmenter keeps what it owns, command and word boundaries; only the
  within-word breakdown is the owner's. `differential_word_expr` holds a
  frozen copy of the walk that was replaced and asserts the new production
  against it across crafted edge cases, the sample corpus and tcllib, so
  the adoption's behaviour changes are enumerated rather than assumed.

  The module sits in `tcl-lexer` because its dependencies already do —
  `braced_var_name_end`, `scan_array_index`, `command_substitution_end`,
  `close_quote_offset`, `backslash_subst_in` — and because the crate is
  below `tcl-syntax`, both runtimes and the compiler, so every consumer
  reaches it without inverting an edge.

  Three properties are contractual:

  - **Borrow-based.** `WordBody::Literal` and every `Variable` / `Command`
    part are sub-slices of the caller's source; only a text run that
    actually had an escape to decode owns its bytes. That keeps the literal
    fast path zero-copy, which is what lets the runtime's `parse_cache`
    hold parsed commands against a stable script slab
    (memory-management.md MM-B.6) with the stale-slab hazard a *compile*
    error rather than a runtime one. A future adopter must not "simplify"
    this to owned bytes.
  - **Errors are parts, not a `Result`.** A malformed construct becomes a
    `WordPart::ParseError` carrying C's exact message (`missing "`,
    `missing close-bracket`, `missing close-brace`, `missing close-brace
    for variable name`, `missing )`, `invalid character in array index`),
    and the parts scanned before it are still returned. That is C's order,
    not leniency: `subst` substitutes incrementally, so `subst {[side][b}`
    runs `side` and keeps its side effects before reporting the missing
    bracket (8.6.16 and 9.0.4 agree). A consumer wanting C's *script*
    order — `Tcl_ParseCommand` parses every word before evaluating any, so
    nothing runs — scans all its words and raises the first `ParseError`
    before resolving anything.
  - **An unterminated `[` reports what is inside it.** C recurses into
    `Tcl_ParseCommand` at the bracket rather than hunting for the matching
    `]`, so `subst $t` with `t` = `[set y ${a{b]` is `missing close-brace
    for variable name`, not `missing close-bracket`. The missing bracket is
    the fallback, not the default.

  The release axis is the whole `LexerConfig`: the `${...}` close rule
  (#1457), the array-index source mask (#1732) and the escape grammar
  (#1479). The one non-release axis is `SubstFlags::bare_var_refs`, false
  for exactly one consumer — `tcl-vm`'s compiled-word `PUSH` operands,
  where the compiler has already inlined or normalised every real
  reference, so a surviving bare `$` is data. Modelling that as a flag on
  the shared scan is what keeps the VM on this owner rather than justifying
  a private copy.

  **Not yet adopted: `tcl-compiler::segmenter`.** The fourth copy is still
  live. The API is shaped for it — `decompose` takes a word's content span
  plus a `LexerConfig` and returns parts whose byte extents are recoverable
  from the borrows, which is what `WordExpr` / `WordPart` need to keep
  their public shape, so `CommandTokens::from_segmented` maps part for
  part. The segmenter keeps owning *command* and *word* boundaries; only
  the within-word breakdown moves. Tracked in
  `docs/design/lanes/wasm-native-lowering.md` § `r10-word-parts`.

- `structural_index::command_boundaries` — the byte-scanned **reparse
  split points**, and, with `script_is_complete` and `reparse_window`, its
  own surface rather than a fourth grouper folded onto
  `script::group_commands` (issue #1786 item 1).

  The three share one scanner family. `script_is_complete` is the
  crate's `Tcl_CommandComplete` port — called on raw document text by
  `tcl-lsp-server::compute_base_analysis`, `codegen::structured`, the
  analyser's incremental gate and `tcl-vm-cli`'s REPL —
  `command_boundaries` answers "where may an incremental reparser cut this
  document into whole commands", and `reparse_window` snaps an edit range
  outward to those cuts. Every boundary satisfies
  `script_is_complete(&source[..b])`; that invariant is *why* the two
  cannot be separated.

  Folding the boundary half onto the owner was considered and rejected on
  three measurements. **One**, it would split that family: the
  completeness oracle is not a grouper and cannot move, so the invariant
  above would become unprovable and the two halves free to disagree.
  **Two**, the contracts differ — the scanner reports *terminator*
  offsets, the owner reports command spans and discards empty,
  comment-only and dangling-`{*}` commands, so `a\n\n\nb\n` is `[2,3,4,6]`
  against the owner's two commands, and `reparse_window`'s minimality
  rests on those blank-line cuts. **Three**, cost and totality: the
  scanner is a single allocation-free pass over bytes, deliberately *not*
  a `Vec<Token>` per keystroke, and it is total on malformed input where
  `tokenise_all` returns a `Result`.

  What the fold *would* have bought — dialect correctness — is instead
  paid for honestly. The scanner takes no `LexerConfig`, and two grammar
  axes really do move a command boundary: under the F5 trunk a newline
  whose next line opens with `{` does not terminate the command
  (`BraceLineContinuation::Continues`), and under the 8.x family
  `${a{b}` ends at the first `}` (`BracedVarStyle::FirstClose`), so
  `set v ${a{b}<newline>puts hi` is two commands there and one under 9.x.
  Both are pinned with their measured answers by `tcl-lexer`'s
  `differential_boundaries`, which also asserts containment, coverage and
  termination against `group_commands` over `samples/` and the Tcl 9.0.4
  library at three dialects and three nesting levels — 21.5k regions,
  39.9k commands — with tcllib behind `--ignored`. It replaced a
  seven-string non-straddling assertion in `tcl-compiler`'s segmenter that
  a `command_boundaries` returning nothing but `source.len()` would have
  passed; the coverage invariant it could not express found two live
  defects, one per tier. Both were the same shape — a nested `[…]` whose
  interior C rejects (`{a}$b`) makes the completeness scanner report
  `Terminal` with its offset collapsed to end-of-input, so every later
  boundary in the document is lost. The CI tier caught it on
  `[a [b {*}$c]]`, written in 9.0.4's `auto.tcl`; the tcllib tier caught
  it behind a quoted word, in `page/util_quote.tcl`. Fixed by taking the
  lenient closes — `ranges::command_substitution_end` and, on `Terminal`
  only, `ranges::close_quote_offset` — instead of the completeness
  oracle's error path. Coverage is asserted only where the region is a
  complete script, which is the scanner's documented precondition; those
  regions are tallied and reported.

  Agreement with the *compiler* segmenter follows transitively:
  `differential_group` proves `group_commands == segment_commands`
  command for command over the same corpora.

### `tcl-cmd-core` — portable command logic

- `namespace` — the pure `::` byte-ops `tail` / `qualifiers`
  (`last_sep_run`: colon runs are one separator) plus the
  `Namespaces`-generic cores. Runtime name resolution routes through
  these — the VM's `interp.rs` canonicalisers (`canonical_cmd_key`,
  namespace declare/find/parent/import/forget) and `command.rs`
  (rename re-homing, `proc` namespace derivation) are built on them.
  The generic cores cover `current` / `exists` / `parent` / `children`
  (including byte-valued twins and Tcl string-hash enumeration order), the
  positional `which_request`, import-source validation, `which_command` and,
  since #1442, `which_variable` (the
  `Tcl_FindNamespaceVar` probe — namespace variable tables only, never
  a call frame; its *alternate* global-rooted candidate is the one
  release axis, dropped by 9.0's `flags |= TCL_NAMESPACE_ONLY`) and
  `origin` (`NamespaceOriginCmd`). The two accessors they need are on
  the `Namespaces` role trait: `namespace_var_exists` and
  `command_origin`. `command_origin` is the *whole* import walk, not a
  single hop, because C's `TclGetOriginalCommand` is, and because a
  runtime whose import links are name-keyed needs its own
  disambiguation (the VM's hidden/visible token domains).
- `ensemble` — `tclEnsemble.c`'s tables and rules: the
  `namespace ensemble` subcommand table, the **two** option tables
  (`create` carries `-command` and no `-namespace`; `configure`
  carries `-namespace`, read-only, and no `-command`), the
  exact-then-unique-prefix subcommand scan, and the dispatch miss
  messages, plus the non-empty implementation-prefix invariant for `-map`.
  `EnsembleToken` is the shared stable command-token lifecycle: its live
  configuration and name survive imports, reconfiguration, and rename, while
  true deletion irreversibly retires that token. `InvocationLayout` and
  `invocation_layout` own the parameter/subcommand/argument positions and must
  be recomputed from the live token after an `-unknown` callback. The exact
  `UNKNOWN_DELETED` message and error code live beside that lifecycle rather
  than in either runtime.
  The scan is `prefix::scan`'s rule with one documented
  divergence: C's ensemble path is a `strncmp` over the word's length,
  so an **empty** subcommand prefixes every entry and resolves against
  a one-entry table, where `Tcl_GetIndexFromObj` forces the error
  path. `subcommand_choices` is the ensemble enumeration, which keeps
  a comma before `or` even for two entries (`bar, or baz`) — the
  wording `prefix::choice_list` must not be used for.
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
  ensemble, `tcl::prefix match`, OO option tables, and — since #1607 —
  both engines' `interp debug` option word (noun `debug option`),
  `interp limit` type word (noun `limit type`), and the `interp`
  ensemble, child-as-command, `interp create` and `interp invokehidden`
  option words. Where an engine advertises only the subcommands it
  dispatches (#1412 item 3), the miss message is composed with `scan` +
  `bad_key_message` over the shorter table — the way C reports
  `interp`'s own misses against `optionsNoSlaves[]`. `package`'s
  subcommand word and `package prefer`'s `preference` word resolve here
  too (they are `Tcl_GetIndexFromObj` tables, not an ensemble, despite
  both engines having worded their misses `unknown or ambiguous
  subcommand` before #1607), as do `update`'s option, `try`'s
  `handler type`, and `seek`'s `origin`. `after` shares only the
  `scan` — C looks its subcommands up with a NULL interp and composes
  its own `bad argument …, or an integer` sentence, so no
  `OptionTable` message may surface there. New command
  modules MUST resolve through `OptionTable` (or `scan` +
  `bad_key_message` where a byte noun or interleaved control flow
  demands composition) — never a hand-rolled scan.
- `trace` — the whole argument-decoding surface of the `trace` command,
  shared by the VM and the WASM runtime (which own only their trace
  tables and firing sites). `resolve_option` resolves the first word
  with C's `Tcl_GetIndexFromObj` rule against the option set the caller
  passes — release-gated, because the registry retires
  `variable`/`vdelete`/`vinfo` at 9.0 — and produces the matching
  `bad option` / `ambiguous option` enumeration; `resolve_type` does the
  same for the type word. `parse_ops` validates an op list and
  `parse_legacy_variable_ops` the 8.x `rwua` letter string; **both return
  the set in `TraceKind::info_order`**, the order C's `TRACE_INFO` arms
  render (`array read write unset`, `rename delete`), which is *not*
  `TraceKind::ops`' `opStrings[]` table order used by the bad-operation
  error. Storing that canonical order is what makes `trace info`
  byte-identical without per-runtime render tables.
  `legacy_ops_letters` renders a stored set back to `rwua` for
  `trace vinfo`, and `callback_op_word` supplies the single letter an
  old-style (`trace variable`-installed) trace's callback receives.
  Firing order, storage, and re-entrancy stay per-runtime — see
  [variable-trace-dispatch-and-introspection.md](variable-trace-dispatch-and-introspection.md).
- `sort::parse_wide` / `sort::parse_real` — the `-integer` / `-real`
  key parsers (`parse_wide` is the whole-string integer-only shape of
  `tcl_syntax::number`, `i128`-wide; `binary`'s wide parse narrows it
  by wrapping, matching C's `binary format`).
- `error::CmdError` — the canonical error-message catalogue
  (`wrong_args`, `bad_choice`, …). The runtimes' arity helpers are
  thin adapters: `runtime/rust`'s single `Interp::wrong_args` method
  and the VM's `interp::err_wrong_args`.

### `tcl-compiler` — nested command-substitution words

- `word_subst::nested_command_words` is the one recovery of the words
  inside a `[cmd …]`. The word snapshot keeps a substitution as a
  single opaque spelling, so the words are recovered by running the
  canonical segmenter over its recorded lexical extent — never a
  bespoke bracket scan — and anything but exactly one complete command
  declines. Consumers: the shimmer and SSA lifts
  (`word_subst::lifted_calls` / `lifted_exprs`), the native lowering's
  `nested_words`, and the WASM leaf-invoke planner's. Analysis and the
  two AOT tiers therefore cannot disagree about what a substitution
  runs, which they could while each kept its own copy.
- `LiftedCall` carries that structure to consumers in `arg_words`, so a
  braced literal is told from a word that substitutes by the
  segmenter's `WordExpr`, not by a `{`/`[` test over argument text —
  the distinction `Statement::Call::args` cannot make.

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

### `tcl-sslictcl` — the TLS declaration vocabulary

- `load_with_diagnostics` is the one reader of a `.sslictcl` document.
  It walks the canonical syntax tree and constructs no interpreter, so the
  document is never evaluated — not even a `check`'s `predicate`, which it
  retains verbatim. `DECLARATIONS` is the machine-readable statement of the
  vocabulary the loader implements; `docs/design/sslictcl-vocabulary.md` is
  its prose, and a unit test holds the two together.
- `evaluate_policy` owns **finding identity**: a policy finding is
  `(check id, endpoint)`, which is why the `grade` id is reserved and why
  declaring `check grade { … }` is `SSLIC1009`. Every consumer that
  deduplicates, suppresses, or compares findings across runs keys on that
  pair rather than on a message.
- `embedded_dataset` is the single reader of the embedded trust-store and TLS
  source bundle. Nothing else fetches it, and nothing reaches upstream at
  build, report, or editing time — see
  [`sslictcl-source-data.md`](sslictcl-source-data.md).
- The loader reuses the shared owners rather than re-deriving them: the
  command/word segmentation owner (`tcl_compiler::segmenter` over the
  canonical CST) reads the document, and `tcl_syntax::list` splits every
  braced `LIST` value, so a `forbid-ciphers {[A-Z]*RC4}` glob means exactly
  what a Tcl list means. There is no SslicTcl exception in the "Known
  deliberate exceptions" section because there is no divergence to record.
- The editor projection is a separate owner because two binaries consume it:
  `tcl-lsp-server` publishes the loader's diagnostics and `tcl-cli`'s
  `diag` / `lint` verbs report the same set, and the rule that the loader
  **supersedes** the analyser's unknown-command verdict in a never-evaluated
  document must be stated once for both. Its second half is the *outline*:
  a declaration document has no procs, classes or namespaces for the
  analyser's scope walk, so its blocks are its structure.
- Which **vocabulary** a position admits is a different question with a
  different owner, and deliberately not stated here: it is the
  definition-body grammar in force, which `tcl_lsp_core::oo_body`'s
  `definition_grammar_at` answers for every definition body of every class
  system, rooted in `CommandRegistry::document_grammar` for a dialect whose
  file is itself a declaration body. Completion and the token walk read it,
  and this owner must not grow a second answer to it.

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
6. **A per-argument fact is one authored row, projected once at load.**
   `CommandSpec`'s six parallel per-argument tables (`arg_roles`,
   `arg_types`, `arg_values`, `closed_value_args`, `arg_presentation`,
   `command_prefixes`) are index-keyed slices, and the loader's
   `ArgRows::seal` is the **one** place an authored `arg` row becomes that
   parallel form — a column added to the row and not to the projection
   vanishes silently, which is the defect the record shape exists to
   prevent.

   *Retired (redesign §11.1 O2, ruled 2026-08-27).* The per-argument
   **lifecycle** machinery this point used to describe —
   `arg_rows: &[VersionedArgRow]` retained beside the slices,
   `project_arg_rows`, `ArgTables`, `CommandSpec::arg_tables_at`,
   `CommandRegistry::arg_indices_for_role_at` and `command_prefixes_at` —
   is deleted. It was declared-and-unpopulated surface: no shipped spec and
   no pack ever gated an argument row, so every accessor took the
   `is_empty()` fast path at every call and the only consumers were their
   own tests. The retired-api gate now holds all seven spellings. Anything
   that needs a per-argument version gate later comes back **with** its
   consumer (principle P-C), and the projection point above is where it
   would attach.

   The version-gated facts that remain — a *value*'s own `Lifecycle` and
   `versioned_arg_values` — keep the request-time discipline the rest of
   this point states: the floor is a per-document fact settled by the
   document's `package require` lines, so it is an **argument** to
   `available_arg_values_at`, never registry state. Registry handles are
   cached per (profile, pack overlay) and shared across documents, so one
   that remembered a floor would answer the wrong document. Consumers
   reading during the walk keep the permissive no-floor answer: their
   verdicts are formed before the floor is knowable, which is why the arity
   axis (invariant 7) buffers and decides post-walk.
7. A version floor is a lower bound and composes by taking the greatest.
   Three things can state one — a `package require` in the document, a
   `SpecTcl` pack's `ambient_package` row, and the profile's
   `LibraryPin` — and `version_gate::FloorSource` is the single place
   that ranks them. The *version* is the max; the ordering of the
   `FloorSource` variants is the **reporting** tie-break at equal
   versions, closest-to-the-author first, and is not a claim that one
   source is more authoritative than another.

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
- `tcl-cmd-core::ensemble::subcommand_choices` — the **ensemble**
  subcommand enumeration, which C renders with a comma before `or`
  even for two items (`x1, or x2`), unlike `Tcl_GetIndexFromObj`; it
  must not be collapsed onto `tcl-cmd-core::prefix::choice_list`.
  (Both runtimes used to keep their own copy — the VM's `oxford_or`
  and the WASM runtime's `ensemble::must_be`; #1453 moved the quirk
  into the owner instead of leaving it duplicated.)
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
- `rust/tcl-cmd-core/src/ensemble.rs` — the two option tables, the
  ensemble subcommand scan's empty-word divergence, and the
  comma-before-`or` enumeration.
- `rust/tcl-vm/tests/namespace_surface_e2e.rs` and the
  `cmd_namespace.rs` test module in `runtime/rust` — the `namespace`
  command surface (`which -variable`, `origin`, `export`/`import`
  leading options, teardown, `ensemble`) pinned against tclsh 8.6.16
  and 9.0.4 on both engines.
- `rust/tcl-cmd-core/src/sort.rs` —
  `parse_wide_shares_the_canonical_integer_grammar`.
- `rust/tcl-vm/tests/namespace_colon_runs_e2e.rs` — colon-run
  resolution/creation pinned against tclsh8.6.
- `rust/tcl-vm/tests/cmd_info_prefix_e2e.rs` — `tcl::prefix` message
  texts pinned against tclsh.
- `rust/tcl-vm/tests/builtins_e2e.rs` and the `builtins.rs` /
  `cmd_alias.rs` test modules in `runtime/rust` —
  `interp_debug_option_uses_c_noun_and_abbreviates`,
  `interp_limit_type_word_resolves_like_tcl_get_index_from_obj`,
  `interp_subcommand_words_resolve_like_tcl_get_index_from_obj`, and
  `interp_create_and_invokehidden_options_resolve_like_tcl_get_index_from_obj`,
  the `interp` family's option/type nouns and abbreviation verdicts
  pinned against tclsh 8.6.16 and 9.0.4 on both engines
  (`runtime/rust/tests/rename_interp_semantics.rs` pins the runtime's
  deliberately shortened enumeration).
- `rust/tcl-vm/tests/builtins_e2e.rs` and the `cmd_package.rs` test
  module in `runtime/rust` —
  `package_option_words_resolve_like_tcl_get_index_from_obj`, with the
  release axis of `package`'s table (`prefer` from 8.5, `files` from
  9.0) pinned in
  `rust/tcl-vm/tests/cross_version_command_surface_e2e.rs`.
- `rust/tcl-vm/tests/cmd_event_e2e.rs` /
  `cmd_control_e2e.rs` / `builtins_e2e.rs` and the `cmd_event.rs`,
  `cmd_error.rs`, `cmd_chan.rs` test modules in `runtime/rust` —
  `update_and_after_words_resolve_like_tcl_get_index_from_obj`,
  `try_handler_type_resolves_like_tcl_get_index_from_obj`, and
  `seek_origin_resolves_like_tcl_get_index_from_obj`.
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
- `rust/tcl-vm/tests/dict_canonicalisation_parity.rs` — the cross-crate
  gate for `canonical_dict_slots` (#1608). The canonicalisation rule
  ("first-occurrence key position, last value wins") had three
  independent implementations plus one surface that had *missed* it, and
  `owner-resolution` cannot see that class of drift: it validates the
  manifest, not whether a surface calls the owner at all. The rule is now
  one function, and this suite feeds a duplicate-key / odd-shape corpus
  through every binding — the `ValueOps` seam, the registry `dict`
  const-folds via `run_const_fold`, and the codegen's
  `fold_dict_create_cmd` — asserting byte identity against each other and
  against real tclsh8.6/9.0. The WASM runtime's native dict rep
  (`runtime/rust/src/dict.rs`) canonicalises *incrementally* across
  mutation instead, so it binds the rule by agreement rather than by
  construction; `duplicate_keys_canonicalise_like_the_shared_owner` there
  is its leg of the same gate.
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
- `rust/tcl-registry/src/spec.rs` —
  `projection_carries_every_row_column` (rule 6's drift gate) and
  `a_row_outside_the_floor_is_filtered_from_every_slice`;
  `rust/tcl-registry/src/arity.rs` —
  `adjacent_windows_do_not_overlap_but_straddling_ones_do`;
  `rust/tcl-registry/tests/registry_sweep.rs` —
  `arity_window_gate_rejects_each_malformed_shape` (the shipped-spec
  side of the same invariant the pack loader only notices).
- `rust/tcl-compiler/src/analyser/diagnostics/version_gate.rs` —
  `the_highest_of_the_three_floors_wins` and
  `equal_floors_are_reported_in_precedence_order` (rule 7's two halves:
  the max, then the reporting tie-break), with
  `no_pack_overlay_leaves_every_floor_where_it_was` as the FN guard for
  a session that loads no packs.
- `rust/tcl-compiler/tests/differential_group.rs` — the command / word
  segmentation gate: `group_commands` against the shipping segmenter,
  command for command and word for word, over `samples/` and the Tcl
  9.0.4 library at three dialects and three nesting levels
  (`owner_matches_segmenter_over_corpora`), with tcllib behind
  `--ignored`. `rust/tcl-lexer/tests/differential_boundaries.rs` — the
  other half: `command_boundaries` against that owner
  (`scanner_agrees_with_owner_over_corpora`), plus
  `dialect_divergences_are_pinned` for the two axes the byte scanner is
  deliberately blind to. `make xtask-segmentation-drift` is the
  banned-spelling gate that keeps a *new* consumer from re-deriving
  either boundary privately — including, since #1787, by collecting C's
  parse-error messages into a private list instead of asking
  `tcl_lexer::first_parse_cut`.
- `runtime/rust/tests/parse_cut_agreement.rs` — the parse-error cut gate.
  The cut is applied twice on purpose: `first_parse_cut` answers it from
  source, for a compile front-end that needs the index of the command the
  clean prefix ends at, and `runtime/rust` answers it per command from the
  borrowed tree it has already built, for an evaluator that must not
  substitute a word of a command that does not parse. One shared driver
  was considered and rejected — it would make `runtime/rust` re-lex every
  command it evaluates, and its parse is infallible by design where the
  owner's returns an `Option`. This differential is what keeps the two
  applications one policy; it runs under `make runtime-rust-test`, and it
  pins the close-quote weld (`"a"b`) as the one shape they still answer
  differently.

## Discoverability

- [KCS index](../README.md)
- [project-layout.md](project-layout.md) — the crate boundaries these
  ownership rules sit inside.
- [family-b-routing.md](../family-b-routing.md) — the runtime seam this
  crate layering serves.
