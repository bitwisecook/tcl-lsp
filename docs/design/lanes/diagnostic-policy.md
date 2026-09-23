# Lane: diagnostic policy — slices 1 to 7

Tracking document for slices 1 to 7 of
[`docs/design/compiler/diagnostic-policy.md`](../compiler/diagnostic-policy.md)
§ *Slices* (issue #2089), and the plan for slices 8 to 10. Slices 1–3
landed in `5fa79406`; slices 4–7 landed item by item from the hand-off
checkpoint `5bc40e95` (made green by DP4.0) to the landing commit that
§ *Slices 4–7 landed* records. Slices 8–10 are § *Plan*'s remaining items.
Protocol: [README.md](README.md) — the tree compiles before every commit,
files are staged by explicit path, every checkpoint is
`wip(diagnostic-policy): …`, the orchestrator pushes.

Branch: `claude/spectcl-optimization-discussion-5qhf42`. The lane owns
`rust/tcl-lsp-core/src/diagnostic_policy.rs`, `rust/tcl-lsp-core/src/config_ini*`,
`rust/f5-xc/src/diagnostics.rs`, the diagnostic-code table, the xtask
catalogue generators, and the caller edits slice 3 needs in
`rust/tcl-lsp-server/src/lib.rs`, `rust/tcl-cli/src/commands/diag.rs` and
`rust/xtask/src/fp_sweep.rs`. It never touches `rust/tcl-registry`,
`rust/tcl-compiler`, `rust/tcl-cmd-core`, `rust/tcl-spectcl`,
`rust/tcl-spec-hooks` or `rust/xtask/src/value_transfers*` (the
`value-transfers` lane's).

## Goal

The page's slices 1–3, and nothing beyond them:

1. `Finding`, `Outcome`, `Reason`, `Report` in `tcl-lsp-core`, with a
   conversion from every producer type; `XC100`–`XC301` in the code table.
2. `config_ini` moved from the server into `tcl-lsp-core`, with
   `DEFAULT_OFF_CODES`, `default_disabled_set`,
   `settings_disabled_diagnostics`, `settings_severity_overrides` and
   `parse_severity_value`; the `Policy` builder.
3. `apply`, in the page's step order; `source_style::style_diagnostics`,
   `sslictcl_diagnostics::diagnostics` and the callers of
   `source_decode::encoding_integrity_diagnostics` stop applying policy.

Slices 4–6 (the server, CLI and MCP adapters) are the next lane, so every
commit here keeps each surface's observable behaviour exactly as it is
today: where a producer stops filtering, its callers obtain the same
outcome in the same commit through the smallest change that keeps parity.
The existing end-to-end suites on the three surfaces are the parity gate.

## Decisions taken

- **`StyleDiagnostic::code` and `XcDiagnostic::code` are `DiagCode`s.**
  Both were `&str` / `String`; typing them makes `Finding::from_style` and
  `From<XcDiagnostic>` total conversions instead of fallible ones, and puts
  the two producers on the one code space the page argues for. Every
  consumer that read the field as a string (`lift_style_diagnostics`,
  `lift_f5_source_integrity_diagnostics`, `lift_xc_diagnostics`, the CLI's
  `style_row` / `abstained_rows`, the xtask false-positive sweep) reads
  `as_str()` / `to_string()` instead; no rendered output changes.
  `ConfigDiagnostic::code` stays a `String` — `tcl-bigip` has many
  constructors and non-LSP consumers — so its conversion is `TryFrom` and an
  uncatalogued spelling is a conversion failure, as the page requires.
- **`XcDiagnostic` gains `span`**, the byte span its `range` was resolved
  from, so the conversion needs no line index and loses nothing.
- **The XC family is thirteen codes, not a range**: `XC100`, `XC101`,
  `XC102`, `XC103`, `XC105`, `XC106`, `XC107`, `XC200`, `XC201`, `XC203`,
  `XC250`, `XC300`, `XC301` — exactly what `rust/f5-xc/src/translator.rs`
  emits. They are `diag(Xc, true, …)`: user-configurable (the server's
  `tclLsp.diagnostics.XC100 = false` already works), default-on within the
  `xcDiagnostics` family switch (the shimmer shape), untagged. Being in the
  table puts them in every generated catalogue, so the editor settings
  gain a "Diagnostics — XC Translation" section (see *Behavioural deltas*).
- **`diag-emission-check` scans `rust/f5-xc/src`**: the translator writes
  the wire spelling onto each `TranslationItem`, which is the bare-string
  construction shape the gate already accepts. The forward direction (every
  spelling the translator writes is catalogued) is pinned by
  `emitted_codes_are_catalogued` in `f5-xc`.
- **`Reason::Overlap` carries an `OverlapOwner`**, not a `DiagCode` as the
  page's sketch has it: the SslicTcl entry's owner is a producer, and the
  page's own `OverlapOwner` enum already says so.
- **`Report` exposes `outcome_for` beside `reason_for`**, so "absent" and
  "shown" are distinguishable; the page names only `reason_for`.
- **`Policy::document` groups `reporting`, `excluded` and `abstain`** as a
  `DocumentGates` value. The page's sketch has them as three direct
  fields; with `shimmer` that is four `bool`s on one struct, which
  `clippy::pedantic` refuses and the repository allows no new `#[allow]`
  for. The step order is unchanged.
- **A compiler-check `replacement` becomes a same-span `Fix`** titled with
  the message and classified `FixSafety::RequiresReview`, because the check
  proved nothing more; `category` is dropped as the page says.
- **`PolicyBuilder` takes the layers one at a time, lowest first**, not the
  merged JSON the page's § Configuration mentions: only the unmerged layers
  can name the layer that decided a code, which `Reason::Disabled` and the
  per-code tri-state need — the page's own failure mode ("a flat disabled
  set reintroduced in the builder"). Within a section the later layer wins
  per key, as `merge_settings` merges; a key a higher layer sets to an
  unusable value (a non-boolean toggle, an unknown severity) resets the
  lower decision, which is exactly what merge-then-parse did. The builder
  reads the `tclLsp` content shape, a `{"tclLsp": …}` wrapper, and
  flat-dotted keys, the three shapes the server's readers accept.
- **`Policy::from_disabled_set` exists for the transition**: the server
  and the CLI fold their layers into one flat set before the policy step
  today, and the slice-3 parity shims hand that set to `apply` at the layer
  the caller names. It cannot express "the project enables what the global
  file disabled" and says so; slices 4–6 replace it with the builder.
- **`config_ini` moved by content, not by `git mv`**: the index is shared
  with two other lanes, so nothing is staged until the commit itself; git
  detects the rename from the content. The server re-exports the module
  (`pub use tcl_lsp_core::config_ini`), so every `config_ini::…` path in
  `lib.rs` and its tests is unchanged, and keeps a private
  `settings_severity_overrides` that maps the shared parse onto the wire
  `DiagnosticSeverity` its state stores (through the new `lsp_severity`).
- **`DEFAULT_OFF_CODES` is `&[DiagCode]`** and
  `default_off_codes_match_the_catalogue` pins it to the table's
  `default_on` column, so the seed and the catalogue cannot drift.
- **The parity shims render the producer's own record, not the finding.**
  `apply` is order-stable and keeps every finding, so
  `Report::shown_items` pairs the report with the style pass's records by
  position and the server / CLI keep publishing the exact LSP ranges the
  pass computed. Lifting the converted span instead would be identical for
  every code but W107 in a lone-`\r` file, where the pass's own position
  ignores the lone `\r` and the LSP line model does not; that fix belongs
  to the adapter slices, not to a parity commit. The SslicTcl findings are
  lifted from the finding (`lift_shown_findings`), which reproduces
  `lift_analyser_diagnostics` field for field.
- **`WHOLE_FILE_CODES`** (W107, W109, W118) is the policy step's statement
  of the rule the style pass used to encode in control flow: the inline
  bucket is skipped for those codes, the file bucket is not.
- **`supersede_analyser_diagnostics` stays** until the server and CLI
  adapters read the report; `dialect_overlaps` reads
  `SUPERSEDED_ANALYSER_CODES`, so the rule has one list.
- **The server's F5 integrity lift and the CLI's abstention rows** hand the
  top-of-file directive to the policy step as a file bucket
  (`Directives::new`) instead of folding it into a flat set — the same
  outcome, since W107 / W109 are whole-file codes and the bucket carries
  `*`.

## Site inventory

| Site | Slice | Status |
|---|---|---|
| `DiagSection::Xc` + 13 rows, `rust/tcl-core-types/src/diag_code.rs` | 1 | done |
| `rust/tcl-lsp-core/src/diagnostic_policy.rs` — `Finding`, `Producer`, `Fix`, `FindingData`, `Outcome`, `Reason`, `PolicyLayer`, `OverlapOwner`, `Report`, `Shown` | 1 | done |
| Conversions: analyser, compiler checks, optimiser (`From`); style / decode (`Finding::from_style`); BIG-IP (`TryFrom<&ConfigDiagnostic>`) | 1 | done |
| `From<XcDiagnostic> for Finding`, `rust/f5-xc/src/diagnostics.rs` | 1 | done |
| `StyleDiagnostic::code: DiagCode`, `XcDiagnostic::code: DiagCode` + `span` | 1 | done |
| xtask: `diag_emission` root, `xc` in the three `SECTIONS` tables, `gen_ai` category; regenerated catalogues | 1 | done |
| `rust/tcl-lsp-core/src/config_ini.rs` + `config_ini/tests.rs` (moved from the server) | 2 | done |
| `DEFAULT_OFF_CODES`, `default_disabled_set`, `settings_disabled_diagnostics`, `settings_severity_overrides`, `parse_severity_value` into `config_ini` | 2 | done |
| `Policy`, `CodeDecision`, `OptimiserPolicy`, `Overlap`, `OverlapScope`, `Directives`, `PolicyBuilder`, `dialect_overlaps`, `Policy::from_disabled_set` | 2 | done |
| `apply` and its unit truth table (`apply_tests`) | 3 | done |
| `style_diagnostics` loses `disabled` / `suppressed`; `sslictcl_diagnostics::diagnostics` returns every finding | 3 | done |
| Parity shims in `lift_source_style_diagnostics`, `extend_with_sslictcl_diagnostics`, `lift_f5_source_integrity_diagnostics`, the CLI's `style_rows` / `push_sslictcl_rows` / `abstained_rows`, xtask `fp_sweep` | 3 | done |
| KCS note for the module (`kcs-qa-where-is-diagnostic-policy-applied.md`); "Today" sentences on the page and in `diagnostics-calculation.md` § Suppression | 3 | done |

## Behavioural deltas accepted

- **Editor settings catalogues grow an XC section.** VS Code's
  `package.json`, the generated `diagnosticCatalog.ts`, the JetBrains
  `DiagnosticCatalog.kt` and the AI diagnostics catalogue now list
  `tclLsp.diagnostics.XC100` … `XC301` (thirteen toggles under
  "Diagnostics — XC Translation"; AI category `irules`). The toggles
  already worked on the server; they were simply not offered. No diagnostic
  output changes.
- An XC `TranslationItem` whose `diagnostic_code` is not catalogued no
  longer becomes an `XcDiagnostic`. Every spelling the translator emits is
  catalogued and the pair of gates keeps it so, so no output changes.
- **A literal `*` in a configuration or flag disabled set no longer reaches
  the style and integrity codes.** `tclLsp.diagnostics.* = false` (or
  `--disable '*'`) was never documented; on the server it silenced only the
  style pass (the analyser, the compiler-check lift and the XC lift compare
  codes exactly), and on the CLI only the style and abstention rows. The
  policy step keys decisions by `DiagCode`, so the spelling is ignored
  everywhere, which makes the surfaces consistent with each other. The
  `*` of `# noqa` and `# tcl-lsp: disable=*` is unaffected: those are
  directives, and the directive buckets carry it.

## Open uncertainties

- Whether the owner wants the thirteen XC toggles offered in the editor
  settings UI, or the family kept out of the catalogues (`diag_internal`
  would do that, at the cost of `is_internal`'s "always active" meaning
  being false for a family behind a switch). The tree today is the first
  reading.

## Verification, and why the three slices are one commit

The plan was one checkpoint per slice. Slice 1 was verified in full
against a green workspace: `cargo check --workspace`; `cargo test` for
`tcl-core-types`, `f5-xc`, `tcl-lsp-core` (lib and
`lsp_edit_workspace`), `xtask`, and the server's `lift_*`, `settings_*`
and `xc_diagnostics` unit tests; `cargo clippy --all-targets -D warnings`
on every touched crate; `diag-tables --check`, `diag-emission-check`,
`gen-ai-diagnostics --check`, `gen-editor-settings --check`,
`gen-vscode-package --check`, `gen-jetbrains-catalog --check`,
`gen-editor-catalogs --check` and `kcs-index-links`. It could not be
committed at that point: the `value-transfers` lane's uncommitted edits
to `tcl-registry` and then to `tcl-compiler` left the workspace
uncompilable for the better part of an hour, and a lane commits only on
a green tree. Slices 2 and 3 were written in the meantime; when the
compiler came back, `cargo check -p tcl-lsp-core -p tcl-lsp-server -p
xtask` passed on the three-slice state, the `tcl-lsp-core` integration
test, `tcl-core-types`, `f5-xc` and `xtask` suites passed, and a
background `cargo check --workspace` poll recorded one full pass on that
state. Then the shared disk filled (`ENOSPC` inside `target/`) while the
`tcl-lsp-core` lib suite and the server suites were building, and the
lane stopped building as instructed.

So the commit that lands the three slices (`5fa79406`) carries them
verified as above and no further; the disk was cleared afterwards and the
rest ran on that state plus the two follow-up commits: `cargo check
--workspace` green; `cargo test -p tcl-lsp-core --lib` (2302, including
`config_ini`, `policy_tests` and `apply_tests`) and `--test
lsp_edit_workspace` (37); the server's `lift_*`, `settings_*`,
`default_off_w242`, `xc_diagnostics`, `config_precedence`,
`apply_global_config` and `sslictcl` unit tests; the server e2e subsets
`noqa` (6), `severity` (5), `sslictcl` (15), `optimiser_disable` (1),
`xc_` (5) and `style` (5); the whole CLI suite (25, with
`diag_honours_noqa_directives_the_way_the_editor_does`,
`diag_honours_a_file_directive_on_an_abstaining_document`, the style-row,
W118 and `SSLIC` tests); the MCP suite (81); `f5-xc` and `tcl-core-types`;
`xtask` except `value_transfers::tests::the_enforced_tier_is_clean_or_waived`,
which is the `value-transfers` lane's own uncommitted test; `cargo clippy
--all-targets --no-deps -- -D warnings` on `tcl-lsp-core`,
`tcl-lsp-server`, `tcl-cli`, `xtask`, `f5-xc` and `tcl-core-types`
(`--no-deps` because the other lane's in-flight `tcl-compiler` fails
pedantic on its own); and every catalogue gate plus `kcs-index-links`.

## Status at hand-off (slices 4–7)

Implementation moves to other agents from here. Everything below is in the
checkpoint commit `wip(diagnostic-policy): slices 4–7 checkpoint — hand-off`
(`5bc40e95`) on `claude/spectcl-optimization-discussion-5qhf42`. Item DP4.0
then ran every suite of the five surfaces' crates against it, fixed what
failed at its cause, landed the parity tests and made the three deletions;
§ *DP4.0 — the checkpoint made green* at the end of this section is the
record, and the subsections in between are the hand-off as written, with
their "not yet" statements answered there.

### What landed

**Core (`rust/tcl-lsp-core`).**

- `src/diagnostic_policy.rs`: `Report` is a struct (`Report::new`,
  `outcomes()`, `extend`, `skipped()`, `declare_skipped(codes, &policy)`);
  `reason_for` falls back to the declared skip when no finding of the code
  exists. `Policy::code_reason(code)` (steps 4–5 of `apply` for a code
  alone), `Policy::production_skip()` (every catalogued code the policy
  hides: the analyser's `with_disabled_diagnostics` set on every surface),
  `Policy::unrestricted()` (everything shows; for a host that renders a raw
  set and says so). Tests: `code_reason_is_the_per_code_decision_and_the_family_gates`,
  `production_skip_is_every_code_the_policy_hides_without_a_finding`,
  `a_declared_skip_explains_a_code_no_finding_carries` in `apply_tests`.
- `src/diagnostic_report.rs` (new): **the one "findings for document under
  policy" function** — `document_report(&DocumentSource, produced: Vec<Finding>,
  &Policy) -> Report`. The caller passes the findings of the producers it
  runs itself (analyser, compiler checks, optimiser, XC, BIG-IP model),
  already converted; the function runs the producers this crate owns — the
  style pass with the byte-integrity checks (`SourcePass::Tcl { line_length }`)
  or, for a non-Tcl F5 model document, the integrity codes plus W305
  (`SourcePass::IntegrityOnly`) — and the `SslicTcl` projection when the
  dialect applies, then `apply`. `with_brace_expr_hints(report, &policy)`
  is the transitional O111 (paired with every *shown* W100, gated by the
  optimiser switch and per-code set only — exactly the server's old
  `append_brace_expr_perf_hints`; slice 8 replaces it). `optimise_under_policy(source,
  registry, dialect, max_iterations, &policy) -> OptimisedSource { text,
  applied, iterations }` is the shared rewrite loop: each pass rescans the
  directives from the current text and applies only the rewrites the policy
  shows. Unit tests in the module (six).
- `src/config_ini.rs`: `global_layer()` (the user's `config.ini`, empty
  when absent), `project_root_for(path)` (walks up from the file's own
  directory, bounded by 20 like `tcl pkg`'s manifest walk), `project_layer_at(root)`,
  `project_layer_for(path)`.
- `src/code_actions.rs` (slice 7): `code_actions(source, range, analysis,
  report: &Report)` and `code_actions_in_program(…, report: &Report, …)`
  lift every fix from the report's **shown** findings — the analyser's and
  the compiler checks' `Fix`es, the shimmer `# noqa` suppress action, the
  W100 brace refactor, and (new) an optimiser rewrite carried as
  `FindingData::Rewrite` with `hint_only: false` and a non-empty replacement,
  as a `QuickFix` titled with the finding's message. `check_diagnostic_actions`
  is gone. Every in-crate caller and test (`code_actions_depth.rs`,
  `docstring.rs`, `force_import_shadow_consumers.rs`, `lsp_lens_links_symbols.rs`,
  `lsp_providers.rs`, the unit tests) builds its report with a `report_of`
  helper over `Policy::unrestricted()`, and the old check tests go through a
  `check_actions` helper that builds an editor layer from the disabled set.

**Server (`rust/tcl-lsp-server/src/lib.rs`, slice 4).**

- `PolicyLayers { global, editor, project }` with `builder()` and
  `production_skip()`; stored on `Backend::policy_layers` (the session's
  three layers) and `FolderConfig::policy_layers` (a configured folder's
  own three, whose `disabled_diagnostics` is now that policy's production
  skip so the analyser's skip and the policy agree). Set in
  `pull_and_apply_config_values` (global and per folder), `apply_global_config`
  (tests), and merged into the editor layer by the `initializationOptions`
  and `did_change_configuration` inline-settings paths.
  `resolved_policy_layers(uri)` picks the folder's or the session's.
- `document_policy(layers, decode, dialect, directives) -> Policy`;
  `lift_report(text, &Report) -> Vec<Diagnostic>` is **the LSP adapter**
  (span through `lift_span`, severity through `lsp_severity`, the tag from
  `Shown::tag`, `data` = `{replacement, startOffset, endOffset}` for a
  non-`hint_only` rewrite with a non-empty replacement, else `None`);
  `analyser_findings`, `compiler_findings`, `xc_findings`, `model_findings`,
  `bigip_config_findings`, `apl_presentation_findings` are conversions;
  `f5_model_report` is the F5 report (directives scanned from the text since
  those families never run the analyser); `skipped_codes` turns the flat skip
  set into `DiagCode`s for `declare_skipped`.
- `publish_fast_tier`, `refine_and_lift_diagnostics`, `analysed_diagnostics_for`,
  `run_diagnostics_f5_dialect`, `f5_pull_report` are one `document_report`
  (+ `with_brace_expr_hints` on the two deep paths) + `lift_report` each.
  `optimise_document_command` runs `optimise_under_policy` under the folder's
  layers plus an `Invocation` layer `{optimiser: {profile: <arg>}}`; its own
  iteration rule is kept (`"full"` → 5 passes, anything else → 1).
  The code-action handler builds one report — the published analyser set,
  the uncached compiler checks and the optimiser's rewrites under the
  document's policy — and passes it to `code_actions_in_program`;
  `retain_unsuppressed_diagnostics`, `check_actions`, `DialectActionInputs::disabled`
  and the `supersede_analyser_diagnostics` calls are gone (the SslicTcl
  overlap entry does that in the policy step).
- Removed: `lift_analyser_diagnostics`, `lift_compiler_diagnostics`,
  `lift_source_style_diagnostics`, `lift_style_diagnostics`,
  `lift_f5_source_integrity_diagnostics`, `lift_xc_diagnostics`,
  `lift_config_diagnostic`, `extend_with_sslictcl_diagnostics`,
  `append_brace_expr_perf_hints`, `finalise_diagnostics`,
  `apply_encoding_abstention`, `apply_diagnostic_tags`,
  `apply_severity_overrides`, `suppress_duplicate_o120`,
  `retain_unsuppressed_diagnostics`, `transitional_policy`,
  `lift_shown_findings`, `shown_style_records`, `parse_folder_optimiser`,
  the server-local `settings_severity_overrides`, `resolved_severity_overrides`,
  and the `severity_overrides` / `optimiser_code_overrides` state on
  `Backend` and `FolderConfig` (`optimiser_enabled`, `optimiser_profile`,
  `shimmer_enabled` stay: `getEffectiveConfig` / `render_config_ini` and the
  shimmer fold in `resolved_analysis_settings` still read them).
  `resolved_analysis_settings` returns `(disabled, non_ascii_mode)` only;
  `DiagInputs` / `LiftInputs` carry `policy_layers` instead of
  `severity_overrides` / `opt_disabled` / `optimiser_enabled`.
- Unit tests rewritten over the report: `the_report_surfaces_compiler_check_codes`,
  `the_report_honours_the_optimiser_master_switch_and_per_code_set`,
  `the_report_surfaces_irules_taint_flow`, `the_report_honours_a_per_check_disable`,
  `the_report_honours_an_inline_noqa_on_a_compiler_check`,
  `the_report_keeps_one_squiggle_where_w110_and_o120_coincide`,
  `the_report_relabels_a_code_the_editor_layer_overrides`,
  `the_report_surfaces_style_codes`, `the_report_honours_a_file_directive_on_the_style_pass`,
  `bigip_config_findings_surface_codes`, `the_f5_report_honours_a_disabled_model_code`,
  `apl_presentation_findings_surface_codes`, `a_model_finding_lifts_with_an_exclusive_end`,
  `report_driven_abstention_survives_a_disabled_w109` (through `f5_model_report`),
  `xc_findings_surface_and_the_report_filters_codes`,
  `resolved_analysis_settings_falls_back_to_global_defaults`,
  `pull_diagnostics_include_compiler_and_optimiser_codes`, the two
  snapshot-epoch tests and the folder generic-patterns test (they flip
  `policy_layers.editor` instead of the removed switch). **None of them has
  been run yet.**

**CLI (`rust/tcl-cli`, slice 5).**

- `src/commands/policy.rs` (new): `ConfigLayers::new(invocation)` reads the
  global layer once and project layers once per root; `builder_for(path)`
  (global, invocation, then the file's project layer; a pathless document
  has none); `share_one_project(paths)`; `invocation_layer(disable, enable,
  section)` (`diagnostics` for the diagnostic verbs, `optimiser` for `opt`).
  Two unit tests.
- `src/commands/diag.rs`: `collect_rows(document, dialect, &ConfigLayers,
  evidence)` is the row adapter: the analyser runs with the policy's
  `production_skip()` (so W242 is seeded off, as in the editor), the
  compiler checks (**including** the O-codes) join the analyser's findings,
  `document_report` adds the style pass and the loader, and `rows_of`
  renders `shown()`. `diag_policy` turns the optimiser off (O-codes become
  `OptimiserOff` suppressions — this verb never shows rewrites). An
  abstaining document goes through `SourcePass::IntegrityOnly` under
  `Directives::scan`. `resolve_disabled`, `push_sslictcl_rows`,
  `transitional_policy`, `shown_style_records`, `abstained_rows`,
  `style_rows`, `style_row`, `line_of` are gone.
- `src/commands/transform.rs`: `run_opt` builds the invocation layer
  (`optimiser.<CODE>` overrides plus `optimiser.profile`), folds the inputs
  into one text only when `share_one_project` holds **and** no input carries
  any directive, else optimises each document under its own policy and joins
  the outputs with the new `tcl_cli_support::combine_texts` (issue #2062);
  the summary block lists what was applied.
- `src/cli.rs` help text for `--disable` / `--enable` on `diag` and `opt`;
  `docs/kcs/features/kcs-feature-tcl-verb-cli.md` § Verb contracts and
  `docs/kcs/kcs-howto-suppress-diagnostics.md` § Precedence describe the
  layers on the CLI and MCP.

**MCP (`rust/tcl-mcp/src/tools.rs`, slice 6).**

- `PolicyInputs { global, invocation }` (`for_call(args, section)`,
  `builder()`), `invocation_layer(args, section)` (the `disable` / `enable`
  arguments, comma-separated; `profile` under `optimiser`), `Analysed { analysis,
  policy, skipped }` with `report()` / `report_with(more)`, and
  `analyse_under(source, dialect, &inputs)` (the analyser with the policy's
  production skip; the plain `analyse` stays for the non-diagnostic tools).
- `analyze`, `validate`, `review`, `find-legacy` render `report().shown()`
  through `diag_to_json(&Shown, sm)` (severity as resolved); `optimize` runs
  `optimise_under_policy`; `code_actions` builds one report over the
  analyser, `run_all_checks` over a unit built like `tcl diag`'s, and
  `optimise_with_dialect`. Each has a `*_with(args, &PolicyInputs)` form so
  a test can pass layers of its own instead of the machine's `config.ini`.
  The five tools' definitions carry `DISABLE` / `ENABLE` (`optimize`:
  `OPT_DISABLE` / `OPT_ENABLE`) and updated descriptions.

### The `data`-payload question, answered

The old analyser lift (`lift_analyser_diagnostics`) attached `data: None`
to every diagnostic; only `lift_compiler_diagnostics`' optimiser loop
attached `{"replacement", "startOffset", "endOffset"}`, and only for a
rewrite with `!hint_only && !replacement.is_empty()`. `lift_report`
reproduces exactly that, from `FindingData::Rewrite`. Every other wire
field is reproduced too: `code_description: None`, `source: "tcl-lsp"`,
`related_information: None`, and `tags` from `Shown::tag` (the old
`apply_diagnostic_tags`).

### Half done, and where it stops

As handed off; DP4.0's answer to each bullet is in brackets, and
§ *DP4.0 — the checkpoint made green* has the detail.

- **No end-to-end suite has been run on this state.** [Every suite ran and
  passes.] The five crates
  (`tcl-lsp-core`, `tcl-lsp-server`, `tcl-cli`, `tcl-cli-support`,
  `tcl-mcp`) compile with `--all-targets` and no warnings; that is all.
- **CLI parity tests are drafted, not in the tree.** The three tests below
  (§ *Drafted CLI tests*) were written for `rust/tcl-cli/tests/cli.rs` and
  never compiled: `diag_seeds_the_default_off_codes_like_the_editor`,
  `diag_resolves_the_project_and_global_layers_per_input_file` (#2063),
  `opt_applies_only_the_rewrites_the_policy_shows` (#2062). They isolate the
  global layer with `XDG_CONFIG_HOME`. [Landed, adjusted to the INI grammar
  and to the fold's real code.]
- **MCP parity tests are not written.** Planned, against the `*_with`
  forms with `PolicyInputs { global: json!({}), invocation }`:
  `analyze` honours an inline `# noqa` (W210 before/after), honours
  `disable`, seeds W242 off and `enable: "W242"` brings it back; `optimize`
  skips a `# noqa: O102` fold and a `# tcl-lsp: disable=*` document;
  `code_actions` offers no "Brace expr" for a `# noqa: W100` line and
  offers the O102 fold as a `quickfix` on a shown rewrite. [Written — nine
  tests; the fold is O101.]
- **Not started:** the `tcl-lsp-db` `file_analysis` commit (the coordinator's
  own instruction: keep the skip, document it as the declared production
  skip; stage only those hunks — the value-transfers lane edits
  `FnLatticeKey` in the same file), removal of `Policy::from_disabled_set`
  (no caller left after slice 5 — delete it and its doc), removal of
  `sslictcl_diagnostics::supersede_analyser_diagnostics` (no caller left;
  keep `SUPERSEDED_ANALYSER_CODES`, `dialect_overlaps` reads it),
  `InputDocument::encoding_diagnostics` (no caller left in the workspace).
  [The three deletions are done; the `tcl-lsp-db` commit is a later item.]
- **The style-finding lift now goes through the finding's byte span** (the
  slice-3 tracking note's deferred W107-in-a-lone-`\r`-file position
  change): expected identical for every other code; unverified. [No test
  pins the lone-`\r` W107 position; no suite regressed.]

### Remaining steps, in order

Steps 1 to 7 are done (DP4.0), except the `tcl-lsp-db` commit in step 6
and `make rust-check` in step 7; step 8 is one commit.

1. `cargo test -p tcl-lsp-core --lib` and `cargo test -p tcl-lsp-core --test
   code_actions_depth --test docstring --test force_import_shadow_consumers
   --test lsp_lens_links_symbols --test lsp_providers --test lsp_edit_workspace`.
   Expected risk: `check_actions_empty_without_fixes`-style assertions that
   the helper's QuickFix filter does not satisfy; the `rewrite_action` title
   (the finding message) if a test pins titles.
2. `cargo test -p tcl-lsp-server --lib` (the rewritten unit tests above,
   plus everything that touched `resolved_analysis_settings` /
   `apply_global_config`). Then the e2e subsets, in this order: `noqa`,
   `severity`, `sslictcl`, `optimiser_disable`, `xc_`, `style`,
   `code_actions`, `commands` (`optimiseDocument`), `config`, `diagnostics`,
   `diagnostic_matrix`, `bigip`, `irules`, `issue1326_encoding`,
   `issue1333_diagnostic_tags`, `issue1556_diagnostics_exclude`; then the
   whole `--test e2e`. A failure in a code-action test that counts actions
   is probably the new rewrite quick-fix (page-mandated: update the test);
   a failure in a folder-config test is probably the layer semantics
   (§ *Decisions* below).
3. Lift the drafted CLI tests into `rust/tcl-cli/tests/cli.rs`
   (`Scratch`, `run_tcl_env`, `diag_codes_by_file` helpers included) and run
   `cargo test -p tcl-cli --test cli`.
4. Write the MCP tests and run `cargo test -p tcl-mcp`.
5. `cargo clippy -p tcl-lsp-core -p tcl-lsp-server -p tcl-cli
   -p tcl-cli-support -p tcl-mcp --all-targets -- -D warnings` (pedantic;
   expect `too_many_lines` / `too_many_arguments` on the rewritten
   `collect_rows` and `code_actions_with`, fix by splitting, never `#[allow]`).
6. The three deletions and the `tcl-lsp-db` commit listed under *Not started*.
7. `cargo xtask diag-tables --check`, `diag-emission-check`,
   `gen-ai-diagnostics --check`, `gen-editor-settings --check`,
   `gen-vscode-package --check`, `gen-jetbrains-catalog --check`,
   `gen-editor-catalogs --check`, `kcs-index-links`; `make rust-check`.
8. Commit per slice if the history is to be split (the checkpoint holds all
   four); otherwise one commit with the deltas below in its message.

### Decisions the page does not state

- **Commit order 7 → 4 → 5 → 6.** The core `code_actions` change (7) is
  what the server's code-action handler calls, so it was written first; the
  MCP call site was moved to the new signature in a parity form
  (`Policy::unrestricted()`) and then replaced by slice 6.
- **Folder policy layers are the folder's own three**, `[global_ini,
  folder editor cfg, folder project]`, not a per-field inheritance from the
  session's. With a real client the scoped configuration is a superset of
  the unscoped one, so this is the same as today except in one multi-root
  corner: a secondary root with no policy section of its own no longer
  inherits the primary root's `.tcl-lsp.ini` sections. The analyser's skip
  for such a folder is derived from the same layers, so skip and policy
  cannot disagree.
- **`Reason::Disabled` on the server names `Global` / `Editor` / `Project`
  truthfully**; the `initializationOptions` and `did_change_configuration`
  inline payloads merge into the editor layer (`merge_settings`) until the
  next pull replaces it.
- **`tcl diag` keeps the optimiser off by policy** (`diag_policy`): O-codes
  from `run_all_checks` are `OptimiserOff` suppressions rather than dropped
  at production.
- **`tcl opt` and MCP `optimize` map `--disable` / `--enable` to
  `optimiser.<CODE>`**, so they override the profile exactly as the editor's
  `tclLsp.optimiser.<CODE>` does; `tcl diag` maps them to `diagnostics.<CODE>`.
- **The MCP tools gain `enable` beside `disable`** (the page names only
  `disable`): without it a default-off code could never be turned on from an
  MCP call, since a `source` string has no project layer.
- **`optimiseDocument` keeps its own iteration rule** (`"full"` → 5 passes)
  while joining the shared loop; the page says "the same path", which is
  the loop, not the pass count.
- **The rewrite quick-fix is `QuickFix`, titled with the finding's message.**
- **`SourcePass::IntegrityOnly` also serves an abstaining CLI document**
  (integrity codes + W305, the editor's abstention survivors), which adds
  W305 to `tcl diag` on a mis-decoded file — parity with the editor.
- **The fast tier now also runs the `SslicTcl` projection** (it is a
  workspace-independent producer; the deep tier is still a superset).
- **O111 is appended after every other finding** (it was inserted after the
  analyser's set); LSP clients sort by range, and no test is known to pin
  the order.

### Behavioural deltas expected (page-mandated unless noted)

- `tcl diag` / `lint` / `validate`: W242 (default-off) no longer reported
  unless a layer enables it; the global `config.ini` and each file's project
  `.tcl-lsp.ini` apply; W305 on an abstaining document (parity).
- `tcl opt`: suppressed rewrites are not applied; inputs with differing
  policies are optimised separately; the global and project layers apply.
- MCP `analyze` / `validate` / `review` / `find-legacy`: inline `# noqa`
  honoured; W242 seeded off; the global `config.ini` and `disable` /
  `enable` apply; the reported severity is the resolved one.
- MCP `optimize`: directives, the global file and the per-code overrides
  reach a rewrite. MCP `code_actions`: compiler-check fixes and optimiser
  rewrites offered; nothing offered for a silenced finding.
- Server: `tcl-lsp.optimiseDocument` honours the optimiser switch, the
  profile's set, per-code overrides and directives (the field's doc always
  promised the switch); code actions include an optimiser rewrite quick-fix;
  the multi-root corner above.

### Gate results at hand-off

- `cargo check -p tcl-lsp-core -p tcl-lsp-server -p tcl-cli -p tcl-cli-support
  -p tcl-mcp --all-targets`: **green, no warnings**, on the checkpoint.
- `cargo check --workspace`: see the hand-off report (the value-transfers
  lane's `tcl-compiler` / `tcl-registry` edits were red for most of the
  session; the lane commits on `-p` green as instructed).
- No test, clippy, xtask or `make rust-check` run on this state (DP4.0
  ran them; below).

### DP4.0 — the checkpoint made green

Item DP4.0 took the hand-off's remaining steps 1 to 7 except the
`tcl-lsp-db` `file_analysis` commit (DP4.1's) and `make rust-check`
(checkpoint C4's).
Every build shared `target/` with a concurrent implementer whose
uncommitted `tcl-compiler` and `tcl-registry` edits were in the tree while
these suites ran; the suites pass with those edits present.

**Suites** (every one green; the last run of each is on the committed
state, except the 31 `tcl-lsp-core` integration binaries no later edit
touched, which ran once, before the deletions):

| Suite | Tests |
|---|---|
| `tcl-lsp-core` lib | 2309 (2311 before the two deleted functions' own tests went) |
| `tcl-lsp-core`, the 33 integration binaries | 1218 (`lsp_edit_workspace` 37 and `code_actions_depth` 46 re-ran after the deletions) |
| `tcl-lsp-server` lib | 570 |
| `tcl-lsp-server` `e2e` | 1595, 5 ignored |
| `tcl-lsp-server` `smoke` / `stdio_deadlock` / `preview_tickets_e2e` | 14 / 6 / 22 |
| `tcl-cli` lib / `cli` / `compile_verbs` / `explorer_gui` / `pkg_verbs` / `spec_verbs` | 26 / 28 / 11 / 2 / 13 / 18 |
| `tcl-cli-support` | 19 |
| `tcl-mcp` | 90 |
| `f5-xc` lib / `differential` / `model` | 22 / 1 / 23 |
| `tcl-core-types` | 40 |

The `e2e` subsets the hand-off named (`noqa`, `severity`, `sslictcl`,
`optimiser_disable`, `xc_`, `style`, `code_actions`, `commands`, `config`,
`diagnostics`, `diagnostic_matrix`, `bigip`, `irules`,
`issue1326_encoding`, `issue1333_diagnostic_tags`,
`issue1556_diagnostics_exclude`) ran inside the whole binary. None failed,
so no code-action count or folder-config expectation needed changing.

**Fixes, each at its cause.**

1. `diagnostic_report::tests::optimise_under_policy_skips_a_rewrite_a_directive_silences`
   asserted that the fold of `set x [expr {1 + 2}]` is O102, and its
   `# noqa: O102` silenced nothing. The optimiser reports that fold as O101
   ("Fold constant expression"); O102 forwards a variable's reaching
   literal. The test, the drafted CLI test and the planned MCP tests use
   O101.
2. `spec_packs::a_workspace_packs_argument_roles_drive_semantic_tokens` and
   `spec_packs::the_bundled_eda_loadables_make_their_vendor_commands_known`
   failed on every run: the analysis never saw a workspace or bundled pack.
   Cause: the hand-off sets every configured folder's
   `disabled_diagnostics` to its policy's `production_skip()`, so every
   configured folder now has its own salsa `AnalyserConfig` handle, where
   before a folder without an analyser override read the global one. A
   folder handle takes the pack key when `apply_folder_configs` creates it,
   during the `initialized` configuration pull and before the startup pack
   reload, and the reload's `sync_db_config` moved only the global handle.
   Fix, in `sync_db_config`: the pack key is set on every live folder handle
   too — packs are a workspace fact, which `apply_folder_configs` already
   states. The same gap existed before for any folder with a handle of its
   own, which includes every VS Code folder (its scoped settings always
   carry a `diagnostics` section).
3. The hand-off commit dropped three server unit tests whose subjects it
   never touched — `parse_non_ascii_mode_maps_settings`,
   `settings_non_ascii_mode_nested_and_flat`,
   `semantic_tokens_capability_advertises_delta_and_range` — as collateral
   of a block replacement; they are restored as they were. The same edit
   had turned the `"http::foo\n"` literal in a `code_actions` test into a
   real line break; restored.

**Tests landed.**

- CLI, in `rust/tcl-cli/tests/cli.rs` with the `Scratch`, `run_tcl_env`
  and `diag_codes_by_file` helpers:
  `diag_seeds_the_default_off_codes_like_the_editor`,
  `diag_resolves_the_project_and_global_layers_per_input_file` (#2063),
  `opt_applies_only_the_rewrites_the_policy_shows` (#2062). Two adjustments
  to the draft, both to the real behaviour. An INI file only turns codes
  off: `insert_diagnostics` reads `[diagnostics] disabled = …` and no
  per-code `W112 = false` / `true` key, so the layers test writes
  `disabled = W112`, and the draft's "a project file turns the code back
  on" — which the INI grammar cannot say — becomes the two precedence pairs
  it can: `--enable` overrules the global file, and a project `disabled =`
  overrules `--enable`. And the fold is O101 over `set x [expr {1 + 2}]`
  alone: with the draft's following `puts $x` the optimiser inlines the
  value and deletes the store (O100 and O109), so no `set x 3` or
  `[expr …]` is left to look for.
- MCP, in `rust/tcl-mcp/src/tools.rs` `mod policy_tests`, through the
  `*_with` forms under a global layer parsed from INI text (never the
  machine's `config.ini`): `analyze_honours_an_inline_noqa`,
  `analyze_honours_disable_enable_and_the_global_file`,
  `analyze_seeds_the_default_off_codes_and_enable_reaches_them`,
  `analyze_reports_the_resolved_severity`,
  `the_grouping_tools_read_the_shown_set` (`find-legacy`, `validate`),
  `optimize_applies_only_the_rewrites_the_directives_leave_shown`,
  `optimize_honours_the_profile_the_overrides_and_the_global_file`,
  `code_actions_offer_nothing_for_a_silenced_finding`,
  `code_actions_offer_a_shown_rewrite_as_a_quickfix`. The O101 rewrite
  spans the whole statement, so its quick-fix edit is `set x 3`, and the
  default `readability` profile keeps it off unless a layer selects `full`.
- No new integration-test file, so the nextest shard manifest is unchanged.

**Deletions.** No production code referenced any of the three.
`Policy::from_disabled_set` went with its own test; the three tests that
used it as shorthand build an editor layer through `PolicyBuilder`.
`sslictcl_diagnostics::supersede_analyser_diagnostics` went with its test,
which `a_producer_owns_the_document_without_a_finding_of_its_own` already
covers through the overlap entry; the owner manifest in
`docs/design/contracts/shared-utility-contracts-rust.md` drops the entry
point. `InputDocument::encoding_diagnostics` had no reference at all; two
comments name the byte-integrity pass instead. § Today and § Anchors on the
design page still name them, as they name the removed lifts: slice 10
rewrites those.

**Behavioural deltas confirmed.** From the hand-off's list: W242 is off on
`tcl diag` unless a layer enables it, and the global file and each file's
project `.tcl-lsp.ini` apply per input (CLI tests); `tcl opt` leaves a
suppressed rewrite alone and optimises inputs whose directives differ
separately (CLI test); the MCP diagnostics tools honour `# noqa`, the W242
seed, the global file, `disable` / `enable` and report the resolved
severity; `optimize` honours directives, the profile, per-code overrides,
the global file and the master switch; `code_actions` offers a shown
rewrite and nothing for a silenced finding (MCP tests). Not pinned by a new
test: W305 on an abstaining CLI document, `optimiseDocument` under policy
(the core `optimise_under_policy` test pins the shared loop), the server's
rewrite quick-fix (no `e2e` test counts it) and the multi-root corner.
Added by fix 2: every configured folder now has its own `AnalyserConfig`
handle — see *Open uncertainties (DP4.0)*.

**Open uncertainties (DP4.0).**

- Every configured folder's own `AnalyserConfig` handle (fix 2's cause)
  is correct now, but `db_document_symbols` reads the global handle, so a
  document under a configured folder can be analysed twice per revision —
  once for diagnostics under the folder handle, once for symbols under the
  global one. A VS Code folder already had its own handle before the
  hand-off (its scoped settings always carry a `diagnostics` section), so
  this is not new for that client. Not measured; giving a folder the global
  handle whenever its production skip equals the global one would restore
  the sharing, and is a decision for the lane, not a parity fix.

**Gates.** `cargo clippy -p tcl-lsp-core -p tcl-lsp-server -p tcl-cli
-p tcl-cli-support -p tcl-mcp -p f5-xc --all-targets -- -D warnings`
(pedantic) is clean after three fixes, none an `#[allow]`: a test helper's
lifetime elided in `diagnostic_report`; `share_one_project` a free function
in `commands/policy.rs`, since the method never read `self`; and the
server's `code_action` handler (107 lines) brought under the limit by
extracting `code_action_report`, the report the lightbulb reads.
`cargo fmt` over the five crates. `cargo xtask diag-tables --check`,
`diag-emission-check`, `gen-ai-diagnostics --check`,
`gen-editor-settings --check`, `gen-vscode-package --check`,
`gen-jetbrains-catalog --check`, `gen-editor-catalogs --check`,
`kcs-index-links` and `owner-resolution` all pass, so nothing needed
regenerating: the new CLI help text and MCP arguments reach no generated
file. `cargo check --workspace` is green. Not run: `make rust-check`
(workspace-wide, over the concurrent lanes' uncommitted crates too; the
plan leaves it to checkpoint C4) and the `tcl-lsp-core` doc-tests.

### DP4.1 — the skip is the policy's, and declared

Built as § Work items › DP4.1 says, in two commits: the code, and the
`tcl-lsp-db` documentation `d941667f`, staged as that file's own hunks
after the value-transfers lane had committed its `ValueTransferContext`
edits to the same file (`6d0de164`). Beyond the item's list: D37 (the
server's shimmer-switch state, which only the deleted fold read) and D38
(`F5PullInputs::disabled`, which only the deleted parameter read). Two
server unit tests follow the rule they pinned:
`resolved_analysis_settings_falls_back_to_global_defaults` now sets W211
through the configuration, because an `apply_global_config` rewrites the
session skip from the layers and a set inserted by hand beforehand no
longer survives it; `shimmer_disabled_folds_shimmer_family_into_disabled`,
which pinned the fold, is deleted — `the_session_skip_is_the_layers_production_skip`
pins the reverse and that the switch reaches the policy (`ShimmerOff`).
`sync_db_config` still sets the salsa input on every sync, as before; the
skip it writes is sorted and derived deterministically from the layers, so
the change adds no churn.

Suites: core `--lib` 2318; `code_actions_depth` 46, `docstring` 37,
`force_import_shadow_consumers` 10, `lsp_edit_workspace` 37,
`lsp_lens_links_symbols` 45, `lsp_providers` 51; server `--lib` 580; the
whole `e2e` 1595 (5 ignored); `tcl-cli` lib 26, `cli` 38, `compile_verbs`
11, `explorer_gui` 2, `pkg_verbs` 13, `spec_verbs` 18; `tcl-cli-support`
19; `tcl-mcp` 91. The crate clippy is clean (one `assert!` over an
equality became `assert_eq!`), `cargo fmt --check` is clean on the lane's
files, and `cargo check --workspace` is green. No suite changed outcome:
`getEffectiveConfig`'s `disabled_diagnostics` lists catalogued codes only
by construction, and no test pinned an uncatalogued spelling there.

### DP4.3 — the hand-off decisions pinned

Four tests, no production change, as the item lists them: server
`a_configured_folder_resolves_its_own_three_layers`,
`a_secondary_root_does_not_inherit_the_primary_project_file` (the
multi-root corner; the owner's answer to § Open questions 7 flips its first
assertion) and `apply_global_config_populates_the_editor_layer`; CLI
`diag_keeps_a_bidi_control_on_an_abstaining_document` (W109 and W305 on a
UTF-16 byte-order mark ahead of UTF-8 text carrying U+202E, W109 alone
without the control — D10 end to end). `Scratch` gains `write_bytes` for the
byte-order mark. The `sonnet` items of this plan were carried out by the
lane's implementer directly: the session that ran them had no `Agent` tool
to delegate with.

### DP5.1 — an INI layer can turn a code back on

`insert_diagnostics` and `insert_optimiser` read every key of their section
whose trimmed, upper-cased spelling is a catalogued code and whose value
`parse_bool` accepts, after the `disabled` list and in file order
(`insert_code_toggles`), so `W242 = true` enables, `W111 = false` disables,
and a per-code key wins over `disabled` in the same file. Tests:
`a_per_code_key_turns_a_code_on_or_off` (which also resolves a project
`W112 = true` over a global `disabled = W112` through `PolicyBuilder`),
`an_optimiser_per_code_key_keeps_the_switch_keys`,
`an_unparseable_per_code_value_is_ignored`, and CLI
`diag_a_project_file_turns_a_code_back_on`. Documents: the `[diagnostics]`
and `[optimiser]` tables of `xdg-config.md` gain the `<CODE>` row, and the
suppression how-to's § 3 example gains `W242 = true` (DP10.3 writes the
prose). One correction to the item's text: the server's `render_config_ini`
does not write `disabled = …` — it writes one `CODE = false` line per
disabled code under `[diagnostics]`, which no parser read until now; an
exported file therefore reads back as it was written.

### DP5.3 — the batch verbs read no LSP document gate

`ConfigLayers::builder_for` and the MCP `PolicyInputs::builder` call
`.reporting(true)` and never set `excluded`, each with the item's doc
sentence; tests `a_features_toggle_does_not_silence_the_cli` and
`the_features_toggle_is_an_editor_setting`. The same gate reached a third
rewrite surface the item does not name: `optimiseDocument` built its policy
from the folder's layers with no `.reporting(true)`, so a
`features.diagnostics = false` made it a no-op — a regression against
`rust`'s #2119 command, fixed with the same call (D39) and pinned by
`optimise_document_command_reads_no_diagnostics_feature_toggle`, which
failed before the fix. `tcl opt` and MCP `optimize` share the two builders,
so a global `[features] diagnostics = false` no longer stops them rewriting
either.

### DP5.4 — the CLI tests never read the machine's `config.ini`

`tests/cli.rs` gains `tcl()` — the built binary with `XDG_CONFIG_HOME` set
to `empty_config_home()`, one empty directory under the temporary directory
created once through a `OnceLock` and never removed — and every spawn in
the file starts there: `run_tcl`, `run_tcl_in`, `run_tcl_allow_failure`,
`run_tcl_env` (whose own `XDG_CONFIG_HOME` still overrides it), the inline
spawns of `command_info_discovers_the_current_projects_spec_pack`,
`diag_analysis_changes_when_the_current_projects_spec_pack_is_present`,
`minimize_missing_code_errors`, `minimize_reduced_output_still_fires` and
`compwasm_compiles_a_cr_terminated_document_the_way_the_editor_does`, and
through those every other helper. `Scratch` replaces the hand-rolled
directories of `multi_file_diag_text`, `tcl_diag_rows`,
`sslictcl_diag_rows` and `minify_symbol_map_written_for_plain_minify`. The
suite passes as before, and passes again when the test process itself
runs under a global `config.ini` that disables W112, W210, W100, E002,
W120 and SSLIC1101 and switches the optimiser off.

### DP6.1 — one standalone producer run

`diagnostic_report::standalone_findings(&StandaloneDocument, &skip) ->
StandaloneFindings` is `collect_rows`' analysed path moved down unchanged:
the document's declared surface, one `CompilationUnit` under the
document's own environment grammar with the caller's evidence, the
analyser over the skip with `set_cu_override` on that unit, and
`run_all_checks` over it; it reads no policy. `tcl diag`'s `collect_rows`
calls it (rows unchanged: the CLI suite passes as before), and so do the
MCP diagnostics tools through `analyse_under`, which now hands the
analysis form of the source (lone `\r` rewritten) to the producers and
calls `registry(dialect)` before them. `Analysed` carries the source, the
dialect, the analysis, the produced findings and the policy;
`report_with(more, optimiser)` builds the report through `document_report`
(so the style pass and, for `sslictcl`, the loader run) and declares the
analyser skip — the four diagnostics tools with the optimiser off, as
`tcl diag` has it (D5), `code_actions` with the layers' switch (D40).
`code_actions_with` loses its private unit build and `run_all_checks`
call. The four tools' descriptions name the compiler-check families and
the style pass. The MCP suite, including `find_legacy_tests`,
`source_integrity_tests` (one W305, not two) and DP4.0's `policy_tests`,
passes unchanged.

### DP6.2 — #2061's cases

In `policy_tests`, through the `*_with` forms under an empty global layer:
`review_reports_the_compiler_check_families` (T100 in `taint`, IRULE3001
in `security`, IRULE4002 in `thread_safety`, and an empty `taint` for an
untainted `eval`), `analyze_reports_the_source_style_pass` (W112, gone
under `disable: "W112"`), `analyze_reports_the_sslictcl_loader` (SSLIC1101
and no W123), and `a_check_emitted_rewrite_is_suppressed_not_missing`,
which also reads the report in-crate: the O100 a check emits for `if {1}`
stands as an `OptimiserOff` suppression (DP9.3 adds the payload's half).
One test beyond the item: `analyze_honours_the_noqa_fixture_as_tcl_diag_does`
runs `analyze` over #2020's own fixture — #2061's lead reproduction — and
asserts what `diag_honours_noqa_directives_the_way_the_editor_does` asserts
for `tcl diag`: each `# noqa` silences its command's analyser and
compiler-check codes, the unmarked W210s and S100 stand.

### DP7.3 — code actions end to end

`tests/e2e/code_actions.rs` gains `an_optimiser_rewrite_is_offered_as_a_quick_fix`
(under `optimiser.profile = full` the O101 fold of `set x [expr {1 + 2}]`
is a `quickfix` whose edit is `set x 3`; under the default `readability`
nothing offers it) and `no_quick_fix_for_a_finding_a_noqa_silences` (a
`# noqa: W100` line offers no "Brace expr for safety and performance", the
unmarked control does). The three test-section comments that still named
`check_diagnostic_actions` read "compiler-check fixes"; `grep -rn
check_diagnostic_actions rust` is empty. This pins the server's rewrite
quick-fix, which DP4.0's record listed as unpinned.

### Slices 4–7 landed

Every item of slices 4–7 is in: DP4.0 (`23eec80f`); DP4.1 (`d941667f`,
the `tcl-lsp-db` documentation, and `4630323d`); DP4.2, DP5.2, DP7.1 and
DP7.2 with the merge of `rust` (`8b5a8c88`, § *`rust` has moved under the
branch*) and R1 (`cb395386`); DP4.3 (`6f1789a7`); DP5.1 (`8932e597`);
DP5.3 (`02ebc7f7`); DP5.4 (`6d5262b6`); DP6.1 (`3027ac95`); DP6.2
(`bb9bf0ea`); DP7.3 (`ac1cf501`). Checkpoints C4 (`6496e16f`), C5
(`a4374569`) and C6 (`7f0957cb`); C7's gates are the landing gates below.
Every transitional piece § *Transitional pieces to retire* assigns to
DP4.0–DP7.3 is gone from `rust/`: `Policy::from_disabled_set`,
`supersede_analyser_diagnostics`, `InputDocument::encoding_diagnostics`,
`default_disabled_set`, `settings_disabled_diagnostics`,
`settings_severity_overrides`, the server's `skipped_codes` and shimmer
fold, `share_one_project`, `rewrite_action`, and the three
`check_diagnostic_actions` comments. The design page still names the three
`config_ini` readers in its § *Today*, § *Configuration* history and
§ *Anchors*, as it names DP4.0's retirements; DP10.1 removes them from
§ *Anchors* by name.

**Exit evidence.** Each slice's column in § *Goal and exit per slice* is
met except the entries the plan orders after slice 7: slice 4's
`lifted_report` (DP8.2) and LSP truth-table pass (DP9.5), slice 5's CLI
passes (DP9.6), slice 6's MCP passes (DP9.7), and slice 7's lightbulb on
the published report (DP8.3) and code-action passes (DP9.5, DP9.7). Those
add a producer and a parity gate over the adapters landed here and change
none of them.

**Issues these slices' witnesses close.**

- #2061 — the MCP diagnostics tools honour the inline `# noqa`, run the
  compiler checks and report the editor's set. On #2020's own fixture the
  built `tcl-mcp`'s `analyze` and the built `tcl diag` report the same
  three findings (S100 at line 22, W210 at lines 12 and 15), pinned by
  `analyze_honours_the_noqa_fixture_as_tcl_diag_does` and
  `analyze_honours_an_inline_noqa`; `review` fills `taint` (T100),
  `security` (IRULE3001) and `thread_safety` (IRULE4002) on the issue's
  three programs (`review_reports_the_compiler_check_families`);
  `code_actions` offers nothing for a silenced finding
  (`code_actions_offer_nothing_for_a_silenced_finding`); the loader owns
  W123 in a `sslictcl` source (`analyze_reports_the_sslictcl_loader`); W242
  is seeded off (`analyze_seeds_the_default_off_codes_and_enable_reaches_them`).
  One departure from the issue's "the set the editor publishes", kept by D5
  for the owner to accept or reverse: the MCP diagnostics tools follow
  `tcl diag` in hiding O-codes as `OptimiserOff`; `optimize` and
  `code_actions` carry them; DP9.3 makes them visible. The issue's W110
  over O120 overlap therefore only matters on the surfaces where O120 can
  show.
- #2062 — `tcl opt` and MCP `optimize` apply no rewrite a directive
  silences: `opt_applies_only_the_rewrites_the_policy_shows`,
  `optimize_applies_only_the_rewrites_the_directives_leave_shown`, and the
  analyser's attribution of a `# noqa` to the whole command it precedes
  (`a_noqa_reaches_every_line_of_the_command_it_precedes`). The issue's own
  program — `# noqa: O109` over `set x 1` in `proc f` — keeps the store
  through the built `tcl opt --profile full` and the built `tcl-mcp`'s
  `optimize` (`total` 0), while the unmarked control is rewritten (O109)
  on both.
- #2063 — `tcl diag` / `lint` / `validate` resolve the global
  `config.ini`, the flags in the editor slot and each input's own
  `.tcl-lsp.ini` (`diag_resolves_the_project_and_global_layers_per_input_file`,
  `diag_a_project_file_turns_a_code_back_on`), with W242 seeded off
  (`diag_seeds_the_default_off_codes_like_the_editor`); the MCP tools read
  the global file and `disable` / `enable`
  (`analyze_honours_disable_enable_and_the_global_file`). The issue's own
  reproduction — a project `disabled = W100` over `if [expr $a + 1]` —
  loses both W100s under the project and keeps them beside it through the
  built `tcl diag`. The issue asked for the flags "on top"; the page's
  rule 5 puts them in the editor slot, under the project file, and D36
  keeps that order for every per-code decision.

The truth-table passes (DP9.6, DP9.7) are parity gates over these
surfaces, not a condition of any of the three issues' asks, so the
landing commit names all three as closed (D41).

**Behavioural deltas.** Every entry of § *Behavioural deltas expected per
slice* for slices 4–7 is in the tree, the two flagged `[features]` deltas
reverted by DP5.3. A test pins each except the fast tier publishing the
`SslicTcl` loader's codes a publish earlier (D11); W107's position in a
lone-`\r` file and `getEffectiveConfig` listing catalogued codes only were
unpinned at the landing and are pinned by the review fixes, which also
corrected the first (§ *Review fixes for slices 4–7*). Beyond the list,
one regression the checkpoint had introduced is fixed: a
`tclLsp.features.diagnostics = false` made `optimiseDocument` a no-op,
which `rust`'s #2119 command never was (D39). No other suite outcome
changed. The review added one delta the list lacked: DP5.1 makes the
server's exported `CODE = false` lines readable (slice 5 below).

**Gates at landing**, on the tree at `ac1cf501` with the value-transfers
lane's uncommitted edits present: `cargo test -p tcl-lsp-core -p
tcl-lsp-server -p tcl-cli -p tcl-cli-support -p tcl-mcp -p tcl-lsp-db`
exits 0 — core `--lib` 2321 and its 33 integration binaries 1218; server
`--lib` 584, `e2e` 1597 (5 ignored), `smoke` 14, `stdio_deadlock` 6,
`preview_tickets_e2e` 22; `tcl-cli` lib 27, `cli` 40, `compile_verbs` 11,
`explorer_gui` 2, `pkg_verbs` 13, `spec_verbs` 18; `tcl-cli-support` 19;
`tcl-mcp` 97; `tcl-lsp-db` lib 96 and its ten integration binaries 26
(5 ignored). Pedantic clippy with `--all-targets --no-deps -D warnings`
on `tcl-lsp-core`, `tcl-lsp-server`, `tcl-cli`, `tcl-cli-support`,
`tcl-mcp`, `f5-xc` (`--all-features`) and `tcl-lsp-db`: clean, with no new
`#[allow]`. `cargo fmt --check` on the lane's crates: clean. The ten
catalogue gates (`diag-tables --check`, `diag-emission-check`,
`gen-ai-diagnostics --check`, `gen-editor-settings --check`,
`gen-vscode-package --check`, `gen-jetbrains-catalog --check`,
`gen-editor-catalogs --check`, `kcs-index-links`, `owner-resolution`,
`retired-api-gate`): pass, so nothing regenerates — no new CLI flag or MCP
argument reaches a generated file. `make rust-check`: exits 0 (workspace
fmt and clippy, the runtime crate, every `xtask-check` gate). No new
integration-test binary, so the shard manifest is unchanged.

### Review fixes for slices 4–7

The review of the landing (`14533506`) returned "land after fixes"; one
commit fixes its findings.

1. **W107 in a lone-`\r` file.** `source_decode::position_of` counted `\n`
   only, so for `set a 1\rset b 2\rputs "\xff bad"\n` the pass said
   `(0, 22)` and `Finding::from_style`, reading it back through
   `LineIndex::new_lsp`, clamped it to the end of line 0 — `tcl diag` put
   W107 at `1:8` where the `\n` twin says `3:7`. It now positions through
   `LineIndex::new_lsp`, the line-model owner. Pinned by core
   `w107_sits_at_the_replacement_character_in_a_lone_cr_file` (the span is
   the U+FFFD's) and server `the_report_places_w107_on_the_lsp_line_model`
   (`(2, 6)`–`(2, 7)` on the wire).
2. **#2061 and D5.** The landing record's #2061 entry now says that the MCP
   diagnostics tools hide O-codes as `tcl diag` does, and why.
3. **The MCP rewrite tools' raw text.** `Analysed` keeps the analysis form
   once (`analysis_text`, so `normalise_lone_cr` runs once per call);
   `code_actions` hands it to the optimiser and the code-action provider,
   and `optimize` hands it to the rewrite loop (D43). Pinned by
   `a_lone_cr_source_optimises_and_acts_like_its_lf_twin`: the binary before
   the fix folded nothing on the `\r` form of a program whose `\n` form it
   folded twice.
4. **The KCS note** `kcs-qa-where-is-diagnostic-policy-applied.md` no longer
   says the surfaces assemble their own checks: every surface renders one
   report.
5. **Configuration read errors** (D42): `read_layer` treats only `NotFound`
   as absence and reports the rest on stderr; the project walk stops at an
   unreadable `.tcl-lsp.ini`. Tests `only_a_missing_file_is_an_absent_layer`
   and `an_unreadable_project_file_ends_the_walk`; `xdg-config.md` states
   the rule.
6. **The effective skip** is pinned: `the_effective_skip_lists_catalogued_codes_only`
   — after `{"diagnostics": {"W9999": false, "W210": false}}` the INI export
   lists `W210 = false` and `W242 = false` only, and `getEffectiveConfig`'s
   `disabled_diagnostics` is `["W210", "W242"]`.
7. **DP5.1's second delta** is in § *Behavioural deltas expected per slice*.
8. **Nits.** `Analysed::report_with(more, optimiser)` is two named methods,
   `diagnostics_report` (optimiser off, D5) and `actions_report` (the
   layers' switch, D40); `StandaloneFindings::unit` had no reader and is
   gone; the `tests/e2e/diagnostics.rs` comment that named
   `lift_analyser_diagnostics` names the policy step; `tests/cli.rs` gives
   every spawn its own `XDG_CONFIG_HOME` that names no directory
   (`absent_config_home`), so nothing is left under the temporary
   directory; DP9.2's text records that a gap row renders only for a code
   no finding carries.

With them, #2062's own program is a test on both rewrite surfaces:
`opt_keeps_a_store_a_noqa_o109_marks` and
`optimize_keeps_a_store_a_noqa_o109_marks`.

## Slices 8–10 as built

One record per item, in the order the items land; § *Progress* has the
commits.

### DP8.1 — a fact code is computed, then decided

`FACT_CODES` (W100) and `Policy::disabled_codes` in `diagnostic_policy.rs`:
`production_skip` is `disabled_codes` less the fact codes, and
`analyser_skip` builds on it, so a W100 a layer turns off is computed by
the analyser and suppressed `Disabled(layer)` by the policy step. A
`# tcl-lsp: disable=W100` still reaches the analyser's own skip, which folds
the file directive inside `tcl-compiler` (§ Open questions 5). No surface's
shown set changes. Test `production_skip_never_skips_a_fact_code`.

The item names `diagnostic_policy.rs` alone; two server readers of the
skip needed the change too (D44). The INI export writes
`PolicyLayers::disabled_codes`; `getEffectiveConfig` writes
`PolicyLayers::reported_disabled`, the skip in force plus the disabled
fact codes. `tcl-lsp.fixAllSafeIssues` builds the document's policy on each
pass, applies the shown findings' fixes (`bulk_applicable_fixes` reads a
`Report`), and analyses under `PolicyLayers::production_skip` of the
document's own layers. `Backend::analyser_config` lost its last caller and
is deleted. Tests: server
`a_disabled_fact_code_is_reported_disabled_but_computed` (the session skip
lacks W100; `disabled_diagnostics` is `["W100", "W242"]`; the export has
`W100 = false`), `fix_all_safe_issues_applies_only_shown_fixes` (the
control is braced; under `# noqa: W100`, with W100 off at a layer, and on
a document that abstains — whose decoded text alone is braced — nothing is
applied) and
`fix_all_safe_issues_analyses_under_the_documents_layers` (IRULE2002 off
for the session and on for a folder: `http_host` becomes `HTTP::host`
under the folder and stays elsewhere; red under the session's skip). The
`e2e` test `fix_all_safe_issues_respects_a_disabled_diagnostic` passes
unchanged, and its comment says why.

Suites: core `--lib` 2325; server `--lib` 589; the whole `e2e` 1597 (5
ignored); `tcl-mcp` 99; `tcl-cli --test cli` 41. The crate clippy on
`tcl-lsp-core` and `tcl-lsp-server` (`--all-targets --all-features
--no-deps -D warnings`) is clean, `cargo fmt --check` is clean on both, and
`cargo check --workspace` is green. Nothing regenerates: no diagnostic
code, flag or setting changed.

### DP8.2 — O111 is a producer; every publish path is one call

Core: `brace_expr_hints(produced)` replaces `with_brace_expr_hints`. It
emits one O111 — `Info`, `Producer::Optimiser`, the old message — at the
span of every W100 the analyser emitted and reads no policy.
`standalone_findings` adds the hints right after the analyser's findings,
so `tcl diag`, `lint` and `validate` and the MCP tools carry O111: an
`OptimiserOff` suppression in the diagnostics verbs and tools, whose
optimiser is off (D5), and a shown finding in the MCP `code_actions`
report, where it offers nothing because it carries no fix. `Report::extend`
lost its last caller and is deleted; the module doc describes the
producer.

Server: `lifted_report(doc, produced, layers, directives, analysed)` —
`document_policy`, `document_report`, the analyser skip's declaration when
`analysed`, `lift_report` — is the one call `publish_fast_tier`,
`refine_and_lift_diagnostics`, `analysed_diagnostics_for` and
`f5_model_report` make (the last with `Directives::scan` and `analysed`
false). The deep push and the pull add `brace_expr_hints` of the analyser's
findings right after them; the fast tier adds none.

Tests: core `the_brace_expr_hint_follows_every_w100_the_analyser_finds`,
which replaces `the_brace_expr_hint_follows_every_shown_w100`: the hints sit
at W100's spans with O111's severity, message and producer; with W100 off
at the editor layer W100 is `Disabled(Editor)` and O111 shows; under
`# noqa: W100` W100 is `InlineDirective` and O111 shows; with the optimiser
switched off O111 is `OptimiserOff` and W100 shows. Server:
`o111_brace_expr_hint_pairs_with_w100` passes unchanged, and
`o111_survives_a_disabled_w100` pulls O111 without W100 under a layer's
`W100 = false` and under a `# noqa: W100`.

Suites: core `--lib` 2325; server `--lib` 590; the whole `e2e` 1597 (5
ignored), `large_file_publishes_fast_tier_before_deep_tier` and the
`diagnostics`, `sslictcl` and `bigip` subsets among them; `tcl-mcp` 99;
`tcl-cli` lib 27, `cli` 41, `compile_verbs` 11, `explorer_gui` 2,
`pkg_verbs` 13, `spec_verbs` 18. `cargo xtask diag-emission-check` passes:
O111's construction site is `brace_expr_hints`, under
`rust/tcl-lsp-core/src`. The crate clippy on `tcl-lsp-core`,
`tcl-lsp-server`, `tcl-cli` and `tcl-mcp` is clean (a test closure renamed
for `similar_names`), `cargo fmt --check` is clean on core and server, and
`cargo check --workspace` is green. The code table is unchanged, so nothing
regenerates.

### DP8.3 — The lightbulb reads the published report; W115's conversion follows its finding

Server: `lifted_report` splits into `published_report(doc, produced, layers,
directives, analysed) -> core_policy::Report` and `lift_report`, and stays
their composition. `published_findings(analyser_diags, compiler_diags,
xc_for_irules, xc_source)` — the analyser's set, `brace_expr_hints`,
`compiler_findings`, and (opt-in) `xc_findings` — is the pull path's own
assembly moved out, and `code_action` calls it too. `code_action` builds its
`DocumentSource` the pull path's way: `text` is `doc.raw()` (the client's
exact buffer) and `analysis_text` is `doc.text` — `DocumentState::text` is
already the snapshot's analysis form by the time a request handler reads it
(`DocumentState::normalised_for_analysis`), so no second `normalise_lone_cr`
runs. A new `CodeActionReportInputs` struct (`published`, `registry`,
`generic_patterns`, `evidence`, `layers`, `style_line_length`,
`xc_for_irules`) carries `published_findings`'s and `published_report`'s
inputs across the `spawn_blocking` boundary, and a same-named
`code_action_report(doc, analysis, inputs)` composes them into the report —
the old hand-rolled `code_action_report` is deleted as the item says, but
the name is reintroduced for this one call, because inlining its four
statements into `code_action`'s closure put the handler at 116 lines against
clippy's 100-line limit; the reintroduced function stays under
`too_many_arguments` by taking the struct instead of nine parameters. The
lightbulb's report now carries the style pass, the `SslicTcl` projection,
O111, the XC findings and the declared skip, none of which carries a `fixes`
entry, so no action appears or disappears on that account.

Core, `code_actions.rs`: `continuation_comment_actions` takes `report: &Report`
and `line_index: &LineIndex` and returns no action unless a shown W115
overlaps `range` (`ranges_overlap` against the finding's own span, the same
pattern `push_brace_expr_refactors` and the general fix loop use).
`code_actions_in_program` passes its own `report` and `line_index` through.
`code_actions_depth.rs`: `report_of` gains a `source: &str` parameter and
builds through `diagnostic_report::document_report` over a `SourcePass::Tcl`
`DocumentSource` (`Policy::unrestricted()`) instead of `apply` over the bare
analyser set, so its continuation tests carry the style pass's W115; every
call site updated. No other test's actions changed, matching the item's
prediction: a style finding carries no `fixes`.

Tests: core `code_actions::tests::a_conversion_follows_a_shown_w115` (the
three cases the item names, using a `w115_test_doc` helper — a plain
closure could not express `DocumentSource<'_>`'s borrow across two call
sites, so it is a named function); server
`tests::the_lightbulb_reads_the_published_report` (the shown codes of
`published_report(published_findings(…))`, built the way `code_action`
builds it, equal `full_diagnostics_for`'s for `set x 1   \nputs $x\n`, W112
among them); server e2e `code_actions::no_conversion_for_a_disabled_w115`
(`apply_configuration_settle` rather than `Lsp::with_config`: the harness's
`config_reflected` barrier has no settle mapping for a bare `"diagnostics"`
key, so the settled form polls `getEffectiveConfig`'s `disabled_diagnostics`
directly); `test_simple_continuation_fix` passes unchanged as the positive.

Suites: core `--lib` 2328 (`--lib -- code_actions` 97 on that count,
including `diagnostic_policy::apply_tests`/`policy_tests`/`tests`), the 33
integration binaries with `code_actions_depth` 46; server `--lib` 591
(`-- lightbulb` 1), the whole `e2e` 1598 (5 ignored), the `code_actions`
subset 93. The crate clippy on `tcl-lsp-core` and `tcl-lsp-server` is clean
after the two extractions above (`too_many_lines`, `too_many_arguments`,
neither an `#[allow]`); `cargo fmt` touched both crates' files, no test
regressed after; `cargo check --workspace` is green. The code table is
unchanged, so nothing regenerates.

### DP9.1 — One spelling for every reason, and the report's gaps

`rust/tcl-lsp-core/src/diagnostic_policy.rs`: `impl core::fmt::Display for
Reason` renders exactly the item's table (`InlineDirective { .. }` carries
no line — the finding's own row does); `PolicyLayer::as_str` (`global` /
`editor` / `invocation` / `project`) and `Producer::as_str` (`analyser` /
`compiler-check` / `optimiser` / `source-style` / `source-decode` /
`sslictcl` / `xc` / `bigip-model`) back the `disabled:<layer>` and
`overlap:<producer>` spellings. `Report::gaps(&self) -> impl Iterator<Item =
(DiagCode, Reason)> + '_` is `self.skipped()` filtered to codes no finding
in `self.outcomes` carries — the DP9.2-review addition
(`Policy::production_skip` declares every catalogued code the decision
turns off, whichever producer emits it, so a checks-emitted code can carry
both a `Disabled` finding and a declared skip) falls out of this filter with
no special case, which the new `apply_tests` test below pins directly.

Tests: `tests::every_reason_has_one_stable_spelling` (every row of the
table, plus every `PolicyLayer` and `Producer` spelling in full, since the
table renders only one example of `Disabled`/`Overlap` each);
`apply_tests::a_gap_is_a_declared_skip_no_finding_explains` (W210 and T100
both `Disabled(Project)`, one T100 finding in the report — `declare_skipped`
on both — `gaps()` yields W210 alone).

Suites: core `--lib` 2328 (see DP8.3's row — the two items' tests are
counted together at that state); `--lib -- diagnostic_policy` 97. The crate
clippy on `tcl-lsp-core` is clean; `cargo fmt` touched the file; `cargo
check --workspace` is green. No new identifier reaches a generated table
(`Display` and `as_str` are Rust-side only), so nothing regenerates.

### DP9.2 — `--show-suppressed` on `tcl diag` / `lint`

`cli.rs`: `ReportArgs { show_suppressed: bool }` (`--show-suppressed`),
flattened into `Diag` and `Lint` only, as the item says (`Validate` lists
errors and takes no such flag — § Open questions 8). `lib.rs` dispatches
both to `run_diag(input, diag, report)`.

`diag.rs`: `collect_rows` returns `DocumentRows { shown: Vec<Row>, hidden:
Vec<HiddenRow> }` — `HiddenRow` is built by `hidden_rows_of` from
`report.suppressed()` (positioned, the finding's own severity and message)
then `report.gaps()` without `Reason::DefaultOff` (no position, severity or
message), positioned rows sorted by `(line, column, code)` and gaps
following, sorted by code; `document_rows_of` pairs it with the unchanged
`rows_of`. `hidden` is always collected — `collect_rows` decides nothing —
and only `run_diag` reads it, gated on the flag; `run_validate` reads
`.shown` only. JSON: `FileReport` gains `#[serde(skip_serializing_if =
"Option::is_none")] suppressed: Option<Vec<SuppressedItem>>`
(`{line, column, severity, code, message, reason}`, a gap's four position
and content fields `null`). Text: `format_hidden_line` (`format_line`'s
shape with `"hidden"` for the severity slot and `[reason]` appended) and
`format_gap_line` (`{file}: hidden<7> code<8> [reason]`, no position at
all); `file_text_lines` merges `shown` with `hidden`'s positioned prefix by
`(line, column, code)` (a manual two-pointer merge, since `Option<u32>`'s
derived `Ord` sorts a gap's `None` first, not last, so a single combined
sort key would misorder them) and appends the gap suffix. `diagnostics=`
gains ` suppressed={m}` on stderr only with the flag; `problem_count` and
`diagnostic_count` are unchanged, so the exit status and the plain output
are preserved byte for byte (confirmed: the 41 pre-existing `cli` tests
pass unchanged). "no diagnostics" prints only when both counts are zero.

Tests, verified against the real binary before writing them:
`diag_show_suppressed_lists_every_hidden_finding_with_its_reason` (the
`noqaSuppression.tcl` fixture: W210 on `suppressedByCode` and S100 on
`dictValue`, both `inline-directive`; the plain JSON carries no `suppressed`
key at all); `diag_show_suppressed_lists_a_disabled_analyser_code_as_a_gap`
(`--disable W210 --source 'puts $y'` → one gap, `disabled:invocation`, no
W242); `diag_show_suppressed_lists_o111_as_optimiser_off` (`set a 1\nset b
[expr $a + 1]\n` → W100 shown, O111 suppressed `optimiser-off`);
`diag_show_suppressed_text_rows_keep_the_exit_status` (a `# noqa`-only
document's text rows carry `[inline-directive]` and the process exits 0).
`diag_suppressed_rows` is the JSON-row helper alongside `diag_codes_by_file`.

Docs: `kcs-feature-tcl-verb-cli.md`'s `diag` bullet gains the flag (DP10.3
folds the how-to, unchanged here); `cargo xtask kcs-index-links` passes.

Suites: `tcl-cli` lib 27, `cli` 45 (41 + 4), `compile_verbs` 11,
`explorer_gui` 2, `pkg_verbs` 13, `spec_verbs` 18. The crate clippy on
`tcl-cli` is clean; `cargo fmt` touched `cli.rs`, `commands/diag.rs`,
`lib.rs` and `tests/cli.rs`; `cargo check --workspace` is green. No new
diagnostic code, so no catalogue regenerates.

### DP9.3 — The MCP `suppressed` array

`tools.rs`: `suppressed_to_json(finding, reason, sm)` →
`{code, range: byte_range(sm, finding.span), reason: reason.to_string(),
message: finding.message}`; `suppressed_json(report, sm, keep: impl
Fn(&str) -> bool)` renders `report.suppressed()` through it, filtered by
`keep`, then `report.gaps()` without `Reason::DefaultOff` (`range` and
`message` both `Value::Null`), filtered the same way — one function so a
gap's shape is written once. `analyze` and `validate` keep every code
(`|_| true`); `review` keeps the union of `meta.security_codes`,
`taint_codes` and `thread_codes`; `find-legacy` keeps
`tcl_cli::CONVERTIBLE_CODES`. Each of the four payloads gains `"suppressed":
suppressed_json(&report, &sm, keep)`; every existing key is untouched. The
four `ToolDef::description`s gain the item's sentence.

Tests: `policy_tests::the_diagnostics_tools_list_what_the_policy_hides`
(`analyze` over `# noqa: W210\nputs $y\n` → suppressed W210,
`inline-directive`, range starting line 1; over `puts $y\n` with `disable:
"W210"` → `{range: null, reason: "disabled:invocation"}`; `review` over
`set x hello\n# noqa: S100\nincr x\n` lists no S100 — S100 is not a
security/taint/thread code, whether or not it fires here); a new
`analyze_suppressed` test helper (mirrors `analyzed`, reading `["suppressed"]`
instead of `["diagnostics"]`) gives
`policy_tests::a_check_emitted_rewrite_is_suppressed_not_missing` its other
half — O100 in `analyze`'s own JSON `suppressed` array with `optimiser-off`,
beside its existing check against the raw `Report`.

Suites: `tcl-mcp` 100 (99 + 1; `a_check_emitted_rewrite_is_suppressed_not_missing`
gained an assertion rather than becoming a new test). The crate clippy is
clean; `cargo fmt` touched `tools.rs`; `cargo check --workspace` is green.
No new diagnostic code or catalogue entry, so nothing regenerates.

### DP9.4 — the truth table and its core pass

`rust/tcl-lsp-core/src/diagnostic_policy/truth_table.rs`, declared
`#[cfg(any(test, feature = "truth-table"))] pub mod truth_table;` in
`diagnostic_policy.rs`; `tcl-lsp-core` gains the feature `truth-table =
[]`. The item's shapes, with two additions:

- `Want` gains `Offered(bool)` and `Applied(bool)`. The item derives
  "`Offered(want is Shown or ShownAt)`" and "`Applied(want is Shown)`" for
  the action and rewrite surfaces, but its `Want` had no variant to hold
  them, and `expected` returns `Expect`s. A row never writes either.
- `Row` gains `defects: &'static [Defect]` (D45), and a surface appears in
  one defect at most. Rows 36 and 37 need it: their wanted reasons do not
  hold on some surfaces with any program (§ Open questions 10 and 11).

`ROWS` holds the item's 41 rows in its order, with its programs and lines
unchanged. `runs_on` computes the item's surfaces column for every row, and
`the_surfaces_each_row_runs_on` pins that. `expected` states the item's
rules; `check` compares the observations as a multiset of `(line, state)`
for each named code, and a specific expectation claims its observation
before a `Shown` wildcard does. `core_report` builds the report the item's
way: `standalone_findings` under the production skip, the optimiser's
rewrites, and `document_report` under the three layers, with the analyser
skip declared. `Row::ini` writes each layer as its INI file, and
`settings_from_ini` reads every row's layers back equal
(`a_layer_round_trips_through_its_ini_file`). `offered`'s O101 rule, "new
text is `set x 3`", holds: the fold's rewrite spans the whole command.

Tests: `every_row_holds_in_the_core_report` (it also compares the typed
reason, so an inline directive's line is checked where the rendered
spelling leaves it out), `every_reason_is_covered`,
`the_surfaces_each_row_runs_on`, `the_surface_rules_restate_the_page`,
`a_layer_round_trips_through_its_ini_file`, `the_slot_becomes_flags`,
`check_names_the_row_and_the_mismatch`, `a_fixed_defect_fails_its_row`,
`a_defect_names_each_surface_once`, `offered_reads_each_subjects_action`,
`row_names_are_unique_and_snake_case`;
and in `policy_tests`, `directives_agree_with_line_suppressed` (seven maps:
an inline and the file bucket, each holding `*`, W210 or W112, and every
entry at once; W210, W112 and W118 at lines 0–3).

On the first run 40 rows held as written. Row 36, `a_same_span_overlap`,
did not: the analyser anchors W110 on the `==` operator (`W110Anchor`),
and the optimiser's O120 spans the whole condition, because
`branch_folding` rewrites the condition's text. `SameSpan` therefore never
fires, and the editor shows both. `suppress_duplicate_o120` compared equal
ranges too, so this predates the lane, and every O120 test builds its
findings by hand at one span. Row 36 keeps its wanted reason. Its defect
pins today's rendering on `Core` and `Lsp` (W110 and O120 both shown). On
`Cli` and `Mcp` the overlap cannot show at all, because the optimiser is
off there. § Open questions 10 asks the owner for the fix.

A second defect is on `Cli` and `Mcp`, and a follow-up commit records it.
The item's rule renders O120 there as an `OptimiserOff` suppression.
`tcl diag` and the MCP diagnostics tools never run the optimiser, though:
`standalone_findings` runs the analyser, the O111 producer and the compiler
checks, and O120 comes only from `optimise_with_dialect`. So O120 has no
finding on those surfaces and no declared gap, and `--show-suppressed` and
`suppressed` cannot explain its absence. On the built `tcl`, `diag --json
--show-suppressed` over row 36's program shows W110 at line 2, and the
`suppressed` array is empty. With `--disable W110` it holds only W110's
`disabled:invocation` gap. A code a compiler check emits is suppressed as
the rule says: O100 (`a_check_emitted_rewrite_is_suppressed_not_missing`),
and O111 on rows 39–41. Rows 36 and 37 record the defect on `Cli` and `Mcp`
(`REWRITE_NOT_RUN`: today W110 as the rules derive it, and nothing for
O120). DP9.6 and DP9.7 therefore meet it as a recorded defect, not as a
failing row. § Open questions 11 asks the owner.

Suites: core `--lib` 2339 and `--lib --features truth-table` 2339 at the
item's commit, 2340 each with the follow-up's test. Pedantic clippy on
`tcl-lsp-core` with `--all-targets --all-features --no-deps -D warnings`
is clean, with the helpers only the tests read kept in the tests module. `cargo fmt --check` is clean and `cargo check --workspace` is
green. No lockfile change, and nothing regenerates.

### DP9.5 — the LSP and code-action passes on the server

`lib.rs`: `apply_session_layers(&PolicyLayers)` is the session half of
`pull_and_apply_config_values`, moved out unchanged. It sets
`Backend::policy_layers`, merges global under editor (`global_editor`)
and then the project file (`merged`), and calls
`apply_global_config_with_signature_fallback(&merged, &global_editor)`.
The pull collapses the inlay alias per layer, calls it, and goes on to the
folders as before. `rust/tcl-lsp-server/Cargo.toml` enables `truth-table`
from `[dev-dependencies]`, and the lockfile does not change.
`src/policy_truth_table.rs` is declared `#[cfg(test)] mod
policy_truth_table;`. It builds its backends with `tests::test_backend`,
now `pub(super)` so that a sibling test module can reach it. Each row gets
a fresh backend configured through `apply_session_layers` (the slot as the
editor layer), with its document at `file:///truth/<name>.tcl` and, for a
`bytes` row, the decode report on the `DocumentState`.

- `every_row_publishes_its_shown_set`: all 41 rows through
  `full_diagnostics_for`, the pull path, which shares `lifted_report` with
  both pushes. Each published diagnostic is observed at its 1-based line,
  with its severity mapped back. Row 28 also requires W211's `tags` to be
  `[UNNECESSARY]`.
- `every_row_offers_fixes_for_shown_findings_only`: the 12 rows with an
  actionable subject (11, 12, 29–35, 39–41), through `code_action` over the
  whole document with an empty context. Each `CodeAction` becomes an
  `ActionView` of its title, kind and `changes` edits, and `offered`
  judges each subject.
- `every_rewrite_row_applies_through_optimise_document`: rows 30–35. The
  slot's `profile` is the command's argument, and its per-code keys stay in
  the editor layer. The result is `Applied(source contains "set x 3")`.

All three passes held on their first run, row 36's recorded defect included:
both W110 and O120 publish on `Lsp` (D45). The four unit tests the page
names are deleted: `the_report_honours_an_inline_noqa_on_a_compiler_check`
(rows 11–12), `the_report_honours_the_optimiser_master_switch_and_per_code_set`
(rows 32–33), `the_report_honours_a_file_directive_on_the_style_pass` (row 8)
and `the_report_relabels_a_code_the_editor_layer_overrides` (row 27).
`open_policy`, `lifted_compiler_set` and `lifted_style_set` stay, because
other tests still read them.

Suites: server `--lib` 590 (591 + 3 − 4); the `e2e` subset `config` 40;
the whole `e2e` 1598 (5 ignored). The change to the configuration pull only
moves code, so the whole `e2e` ran as well as the item's `config` subset.
Pedantic clippy on `tcl-lsp-server` with `--all-targets --all-features
--no-deps -D warnings` is clean, `cargo fmt --check` is clean, and `cargo
check --workspace` is green. No lockfile change, and nothing regenerates.

### DP9.6 — the CLI passes

`rust/tcl-cli/Cargo.toml` gains `tcl-lsp-core` from `[dev-dependencies]`
with the `truth-table` feature (the DP9.5 precedent on `tcl-lsp-server`);
`rust/tcl-cli/tests/cli.rs` gains `truth_table_scratch` (a row's
`xdg/tcl-lsp/config.ini`, `proj/.tcl-lsp.ini` and `proj/<name>.tcl` — the
raw `bytes` for an abstaining row, never the decoded `program`, so the
CLI's own encoding detection decides the abstention as it would for a real
file) and `push_slot_flags` (the slot's codes as repeated `--disable` /
`--enable` pairs), shared by two tests.

`truth_table_rows_render_through_tcl_diag` spawns `tcl diag --json
--show-suppressed --dialect <dialect>` plus the slot's flags, once per row
that `runs_on(Surface::Cli)`, under the row's own `XDG_CONFIG_HOME`. The
report's `diagnostics` become `Shown` observations (the CLI's `severity`
label read back to a `Severity`) and `suppressed` become `Suppressed`
observations, both already on the CLI's 1-based line (a `null` `line` a
gap); `check(row, Surface::Cli, …)` closes every row.
`truth_table_rewrite_rows_render_through_tcl_opt` spawns `tcl opt` with the
slot's `--profile` and its flags, once per row that
`runs_on(Surface::CliRewrite)`; `Applied(stdout contains "set x 3")`.

Rows 36 and 37's `REWRITE_NOT_RUN` defect (DP9.4's follow-up) meets `check`
as the recorded defect on `Cli`, not a failing row — § Open questions 11.
Both tests held on their first run. One spawn per row; the pass is not in
the smoke tier.

Suites: `tcl-cli` lib 27, `cli` 47 (45 + 2, `-- truth_table` 2, 79 s);
`compile_verbs` 11, `explorer_gui` 2, `pkg_verbs` 13, `spec_verbs` 18;
`tcl-cli-support` 19; core `--lib --features truth-table` 2340. Clippy on
`tcl-cli` with `--all-targets --no-deps -D warnings` is clean; `cargo fmt`
is clean; `cargo check --workspace` is green. No lockfile change, and
nothing regenerates.

### DP9.7 — the MCP passes

`rust/tcl-mcp/Cargo.toml` gains `tcl-lsp-core` from `[dev-dependencies]`
with the `truth-table` feature; `rust/tcl-mcp/src/tools.rs`'s
`policy_tests` gains `observed_from_analyze` (an `analyze` payload's
`diagnostics` and `suppressed` arrays as observations — `range.start.line`
is 0-based, so a line is that plus one, and a gap's `range` is `null`) and
three tests.

`every_row_renders_through_analyze` calls `analyze_with` for each row that
`runs_on(Surface::Mcp)`, with the slot's codes as the call's
comma-separated `disable` / `enable` strings and the global layer from the
existing `inputs` helper (`settings_from_ini(&Row::ini(row.global),
Layer::Global)`). `every_row_offers_fixes_for_shown_findings_only` calls
`code_actions_with` over the whole document for each row that
`runs_on(Surface::McpActions)`, turns each action's JSON into an
`ActionView` and judges it with `truth_table::offered`.
`every_rewrite_row_renders_through_optimize` calls the existing `optimized`
helper for each row that `runs_on(Surface::McpRewrite)`, with the slot's
`profile` and codes; `Applied(optimized_source contains "set x 3")`.

Rows 36 and 37's `REWRITE_NOT_RUN` defect meets `check` as the recorded
defect on `Mcp` in the first test, same as DP9.6's `Cli`. All three tests
held on their first run.

Suites: `tcl-mcp` 103 (100 + 3); core `--lib --features truth-table` 2340.
Clippy on `tcl-mcp` with `--all-targets --no-deps -D warnings` is clean;
`cargo fmt` is clean; `cargo check --workspace` is green. No lockfile
change, and nothing regenerates.

### The owner's ruling on § Open questions 10: W110 owns the O120 it sits in

The ruling: the overlap is real, and both anchors are right — W110 on the
`==` operator, O120 on the whole condition it rewrites — so the relation is
containment, not equality (D46). `OverlapScope::WithinSpan` wins where the
owner's span lies inside the superseded finding's span. The base overlap
entry, W110 over O120, uses it, and `SameSpan` stays for an entry that
needs equal spans. `Overlap::claimed_by` states the three scopes in one
`match`.

Tests: `w110_owns_an_o120_whose_span_holds_it` (was
`a_same_span_overlap_needs_a_standing_owner`) holds an O120 over the whole
condition and one on W110's own span, and releases one that starts after
W110 and one elsewhere; its second half keeps the "a standing owner" case.
`a_same_span_overlap_claims_only_the_span_it_shares` pins `SameSpan`
through an explicit entry. `the_overlap_table_follows_the_dialect` reads
`WithinSpan`. Truth-table row 36 now holds as wanted on `Core` and `Lsp`
(W110 shown, O120 `overlap:W110`), so its defect on those surfaces is gone;
`check` would fail with the marker kept. `a_fixed_defect_fails_its_row`
runs over a row of its own. The `e2e` test
`test_optimiser_toggle_suppresses_o_codes` needed an O-code the default
profile shows, and `if {$x == "foo"}`'s O120 no longer shows beside its
W110. It was red on the old program, and it reads O111 over `puts [expr
$a + 1]` now. `kcs-qa-where-is-diagnostic-policy-applied.md` says
"W110 owns an O120 whose span holds its own".

Suites: core `--lib` 2341; server `--lib` 590; the whole `e2e` 1598 (5
ignored); `tcl-mcp` 103; `tcl-cli --test cli -- truth_table` 2. Pedantic
clippy on `tcl-lsp-core` and `tcl-lsp-server` is clean; `cargo fmt
--check` is clean; `cargo check --workspace` is green.

## Plan for finishing slices 4–7 and for slices 8–10

The execution plan from the checkpoint `5bc40e95` to the end of the page's
§ Slices. It is written for an implementer with no other context: every item
names its files, its Rust items with signatures, what it preserves byte for
byte, what it changes and the page sentence or issue that mandates the
change, the tests that pin it, the gates it touches, and the model class that
executes it — `opus` for semantics, adapters and the truth-table design;
`sonnet` for deletions, test scaffolding from a given shape, rendering to a
given format, and document edits. A reviewer checks each landed item against
its entry here and against § Review checklist. Items run in the order listed;
an item's *after* line is its only ordering constraint beyond that. Sizes: S
is under an hour, M is a session, L is more than one.

Vocabulary used below, exactly as the tree spells it:

- *the lane's crates*: `tcl-lsp-core`, `tcl-lsp-server`, `tcl-cli`,
  `tcl-cli-support`, `tcl-mcp`, `f5-xc` (the XC conversion), plus
  `tcl-lsp-db` for DP4.1's documentation commit alone.
- *the crate check*: `cargo check -p tcl-lsp-core -p tcl-lsp-server -p
  tcl-cli -p tcl-cli-support -p tcl-mcp -p f5-xc --all-targets
  --all-features`.
- *the crate clippy*: `cargo clippy -p tcl-lsp-core -p tcl-lsp-server -p
  tcl-cli -p tcl-cli-support -p tcl-mcp -p f5-xc --all-targets
  --all-features --no-deps -- -D warnings` (pedantic is the workspace lint
  level; `--no-deps` because the value-transfers lane's in-flight
  `tcl-compiler` is not always clean; `--all-features` so the `truth-table`
  feature DP9.4 adds is linted), and `cargo fmt --all --check`.
- *the catalogue gates*: `cargo xtask diag-tables --check`, `cargo xtask
  diag-emission-check`, `cargo xtask gen-ai-diagnostics --check`, `cargo xtask
  gen-editor-settings --check`, `cargo xtask gen-vscode-package --check`,
  `cargo xtask gen-jetbrains-catalog --check`, `cargo xtask
  gen-editor-catalogs --check`, `cargo xtask kcs-index-links`, `cargo xtask
  owner-resolution`, `cargo xtask retired-api-gate`.
- *the suites*: `cargo test -p tcl-lsp-core --lib`; `cargo test -p
  tcl-lsp-core --test code_actions_depth --test docstring --test
  force_import_shadow_consumers --test lsp_lens_links_symbols --test
  lsp_providers --test lsp_edit_workspace`; `cargo test -p tcl-lsp-server
  --lib`; `cargo test -p tcl-lsp-server --test e2e`; `cargo test -p tcl-cli`;
  `cargo test -p tcl-cli-support`; `cargo test -p tcl-mcp`; `cargo test -p
  f5-xc`. DP4.0 runs every `tcl-lsp-core` integration binary; each later
  item runs these six.
- *the e2e subsets*, in this order: `noqa`, `severity`, `sslictcl`,
  `optimiser_disable`, `xc_`, `style`, `code_actions`, `commands`, `config`,
  `diagnostics`, `diagnostic_matrix`, `bigip`, `irules`,
  `issue1326_encoding`, `issue1333_diagnostic_tags`,
  `issue1556_diagnostics_exclude`, `vscode_parity`, `spec_packs` — each as
  `cargo test -p tcl-lsp-server --test e2e -- <subset>`.
- *green*: the suites of every crate the item touches, the crate clippy and
  the catalogue gates all pass, and `make rust-check` passes whenever `cargo
  check --workspace` compiles. The other lanes can leave the workspace red; a
  lane commit then needs the crate check and the crate clippy on its own
  crates, which is the rule the hand-off followed.

Capture every gate with `tee` to `/tmp/<gate>-diagnostic-policy.log` and
`grep` it; a `tail` loses a mid-run failure. A cross-crate name in a doc
comment is written in backticks, never as an intra-doc link, so `rustdoc`
has nothing to break.

### The tree at 5bc40e95 against the page's names

Verified by reading the tree at `3a83a9f8` (the lane's files are unchanged
since `5bc40e95`). Where the page's name and the tree's differ, the plan uses
the tree's, and DP10.1 corrects the page.

- Gone from `rust/tcl-lsp-server/src/lib.rs` (the page still names them in
  § Where each step lives and § Anchors): `lift_analyser_diagnostics`,
  `lift_compiler_diagnostics`, `lift_source_style_diagnostics`,
  `lift_style_diagnostics`, `lift_f5_source_integrity_diagnostics`,
  `lift_xc_diagnostics`, `extend_with_sslictcl_diagnostics`,
  `append_brace_expr_perf_hints`, `suppress_duplicate_o120`,
  `finalise_diagnostics`, `apply_encoding_abstention`,
  `apply_diagnostic_tags`, `apply_severity_overrides`,
  `retain_unsuppressed_diagnostics`, `check_actions`, and the server's
  `DEFAULT_OFF_CODES` (now `tcl_lsp_core::config_ini`'s). Present, by line:
  `DiagInputs` (3319), `run_diagnostics_f5_dialect` (3962), `is_fast_tier`
  (6029, `!code.refined_by_workspace()`), `publish_fast_tier` (6041),
  `PolicyLayers` (6099), `document_policy` (6131), `lift_report` (6156),
  `analyser_findings` (6200), `compiler_findings` (6212), `xc_findings`
  (6230), `LiftInputs` (6239), `f5_model_report` (6303), `skipped_codes`
  (6329), `refine_and_lift_diagnostics` (6352), `FolderConfig` (8445),
  `apply_initialization_options` (11179), `optimise_document_command`
  (16841), `pull_and_apply_config_values` (18477), `apply_global_config`
  (18737, `#[cfg(test)]`: it fills the **editor** slot of
  `Backend::policy_layers`), `apply_global_analyser_knobs` (18976),
  `resolved_analysis_settings` (19482), `resolved_policy_layers` (19518),
  `f5_pull_report` (20268), `full_diagnostics_for` (20312),
  `analysed_diagnostics_for` (20325), `published_analyser_diagnostics`
  (20540), `did_change_configuration` (23436), `code_action` (25620),
  `read_ini_layer` (27245), `lsp_severity` (27394), `parse_folder_config`
  (27512), `lift_span` (27894), `model_findings`, `bigip_config_findings`,
  `apl_presentation_findings`.
- The server's analyser skip has two sources. A configured folder's is
  `PolicyLayers::production_skip()` (`pull_and_apply_config_values`); the
  session's is still `settings_disabled_diagnostics` of a payload, written in
  `apply_global_analyser_knobs` (from the merged configuration),
  `apply_initialization_options` and `did_change_configuration` (each from
  the raw payload alone). `resolved_analysis_settings` adds S100–S103 and
  S110 to it when shimmer is off, although the analyser emits no shimmer
  code; the three Tcl publish paths declare the result through
  `skipped_codes`, and `f5_model_report` declares it too although no
  analyser ran. `Policy::production_skip` holds every optimisation code the
  profile disables.
- `optimise_document_command` adds its `Invocation` layer after the project
  layer and turns an absent argument into `full`.
- The code-action handler builds its report with `core_policy::apply` over
  the published analyser set, the uncached checks and the rewrites — no
  style pass and no loader, which is enough: no style finding carries an
  action, and the `SslicTcl` overlap is unconditional.
- `rust/tcl-cli/src/commands/diag.rs`: of the page's anchors only
  `collect_rows` remains; `resolve_disabled`, `abstained_rows`,
  `style_rows`, `push_sslictcl_rows` are gone; `diag_policy` and `rows_of`
  are new. `rust/tcl-cli/src/commands/policy.rs` (`ConfigLayers`,
  `invocation_layer`) is not on the page. Neither `ConfigLayers::builder_for`
  nor the MCP's `PolicyInputs::builder` calls `PolicyBuilder::reporting`, so
  a configuration file's `[features] diagnostics = false` reaches `tcl diag`
  and the MCP tools. `run_opt` (`transform.rs`) folds every input into one
  `combine_sources` text when `share_one_project` holds and no input carries
  a directive, optimises `document.source` rather than the analysis form,
  resolves one combined dialect, and takes its pass count from the
  `--profile` flag.
- `rust/tcl-cli-support/src/input.rs`: `InputDocument::encoding_diagnostics`
  has no caller; `combine_texts` is new beside `combine_sources`.
- `rust/tcl-lsp-core/src/code_actions.rs`: `check_diagnostic_actions` is
  gone, surviving only as three test-section comments (`code_actions.rs`
  lines 3217 and 3327, `tests/code_actions_depth.rs` line 773);
  `code_actions(source, range, analysis, report: &Report)` and
  `code_actions_in_program(source, range, analysis, report, program,
  docstring_style)`; `rewrite_action` offers every shown non-`hint_only`
  rewrite with a non-empty replacement, whether or not it belongs to a group.
- `rust/tcl-lsp-core/src/diagnostic_report.rs` is not on the page:
  `SourcePass`, `DocumentSource`, `document_report`, `with_brace_expr_hints`,
  `OptimisedSource`, `optimise_under_policy`. `optimise_under_policy`
  rescans each pass's directives with `Directives::scan`, which attributes a
  `# noqa` to the next line only, where the analyser attributes it to every
  line of the command it precedes (`apply_preceding_noqa`).
- `rust/tcl-lsp-core/src/diagnostic_policy.rs`: `Report` is a struct (`new`,
  `declare_skipped`, `skipped`, `shown`, `suppressed`, `outcome_for`,
  `reason_for`, `shown_items`, `iter`, `outcomes`, `extend`, `len`,
  `is_empty`), not the page's tuple struct; `Reason::Overlap { owner:
  OverlapOwner }`; `Policy::document: DocumentGates`; `Policy::unrestricted`,
  `code_reason`, `production_skip`, `from_disabled_set`; `Directives::{new,
  from_analysis, scan, none, lines, reason_for}`; `WHOLE_FILE_CODES`;
  `dialect_overlaps`. `Directives::hit` restates `line_suppressed`'s bucket
  rule rather than calling it.
- `rust/tcl-lsp-core/src/sslictcl_diagnostics.rs`:
  `supersede_analyser_diagnostics` has no caller outside its own test.
- `rust/tcl-lsp-core/src/config_ini.rs`: `global_layer`,
  `project_layer_for`, `project_layer_at`, `project_root_for`
  (`PROJECT_WALK_LIMIT` = 20) are new. `insert_diagnostics` reads
  `disabled`, `exclude` and `generic_variable_patterns`, and
  `insert_optimiser` reads `enabled`, `profile` and `disabled`: an INI layer
  cannot turn a code back on, which § The five scopes and config precedence
  promise ("project, editor and global can each turn a code **back on**").
  `settings_severity_overrides` has no caller outside `config_ini/tests.rs`
  since slice 4.
- `rust/tcl-mcp/src/tools.rs`: `PolicyInputs`, `invocation_layer`,
  `Analysed`, `analyse_under`, the `*_with` forms, `DISABLE`, `ENABLE`,
  `OPT_DISABLE`, `OPT_ENABLE`. The diagnostics tools run the analyser alone
  and call `apply` directly: `run_all_checks`, the style pass and the
  `SslicTcl` projection never run for `analyze` / `validate` / `review` /
  `find-legacy`, which is the first half of #2061's title. The crate is
  bin-only; its suite is in-crate (`#[cfg(test)]` modules), run as the
  `tcl-mcp::bin/tcl-mcp` shard.
- `rust/tcl-lsp-db/src/lib.rs`: `file_analysis` (523),
  `apply_cross_file_resolution` (1234), `apply_project_callback_arity`
  (1285), `project_diagnostics` (1373), `file_analysis_incremental` (3288),
  `CompilerDiagnostics` (3363), `compiler_check_diagnostics` (3401),
  `compiler_check_diagnostics_uncached` (3446). Untouched by the checkpoint.
  The value-transfers lane's 68 `FnLatticeKey` lines are committed
  (`f3f9390f`); its later edits to this file (slice 4's `spec_pack_key` →
  `compilation_unit`) are still to come. Three doc comments describe the
  gone lifts: `CompilerDiagnostics` ("for the server to filter"),
  `compiler_check_diagnostics` ("Byte-identical to the direct
  `lift_compiler_diagnostics` build") and `apply_cross_file_resolution`
  ("the LSP lift does not re-filter").
- The analyser (`rust/tcl-compiler/src/analyser/state.rs`, lines 2041, 2427
  and 2524) folds `parse_file_suppression` into its own
  `disabled_diagnostics` and filters by exact code
  (`apply_disabled_diagnostics`, `rust/tcl-compiler/src/analyser/diagnostics.rs`
  line 1075 — the page's § Anchors puts it in `state.rs`). So `# tcl-lsp:
  disable=W210` removes W210 at production, with no reason in the report,
  while `# tcl-lsp: disable=*` does not (the literal `*` matches no code;
  the policy step's `FileDirective` hides those findings). The W305 producer
  (`bidi_control_diagnostics_with_suppressions`, `state.rs` ~2990) filters
  by the disabled set and the directive map itself; it is `line_suppressed`'s
  only production caller (`analyser/source_integrity.rs` line 72).
- The xtask names the task uses: `gen_ai_diagnostics` is
  `rust/xtask/src/gen_ai.rs` (command `gen-ai-diagnostics`, writing
  `ai/shared/diagnostics.json`, `rust/tcl-mcp/diagnostics.json` and the
  prompt files from the code table); `gen_editor_settings` is
  `gen_editor_settings.rs`; `diag_tables` is `diag_tables.rs`;
  `diag_emission` is `diag_emission.rs` (command `diag-emission-check`; its
  `SEARCH_ROOTS` include `rust/tcl-lsp-core/src`, so a producer there is a
  recognised emission site). No generator reads the CLI's clap definitions
  or the MCP `TOOLS` table, and no golden pins `tcl --help`, so a new flag or
  MCP argument regenerates nothing.
- `scripts/dev/rust-test-binary-shards.tsv`: `tcl-lsp-server` is
  `@exclude`d (its own e2e job), `tcl-cli::cli` is shard 4, `tcl-lsp-core`
  (lib) shard 2, `tcl-mcp::bin/tcl-mcp` shard 5. Nothing in this plan adds an
  integration-test binary, so the manifest is untouched; an implementer who
  adds one adds its row in the same commit.
- `docs/design/contracts/shared-utility-contracts-rust.md` carries the
  `owner-resolution` manifest. Its "SslicTcl editor projection" row names
  `supersede_analyser_diagnostics` (deleting the function means deleting the
  name, or `cargo xtask owner-resolution` fails), and no row or owner heading
  names the policy step.
- The six documents slice 10 names exist at the paths the page gives.
  `docs/design/README.md` (line 95) still calls the page a **proposal**.
- Issues (read on GitHub): #2061 (MCP tools ignore `# noqa` and never run
  the compiler checks; `review` can never report a taint finding; its
  correction comment: the top-of-file directive does reach analyser codes
  there, through the analyser's fold), #2062 (`tcl opt` and MCP `optimize`
  apply a rewrite an inline `# noqa` silences), #2063 (`tcl diag` / `lint` /
  `validate` ignore the project `.tcl-lsp.ini` and the global `config.ini`;
  W242 seeding), and #2089's own correction comment.
- The working tree at plan time carries DP4.0 in flight (the three
  retirements, the drafted CLI tests written with `disabled = W112` and
  O101, the MCP `policy_tests` module, the O101 correction in
  `optimise_under_policy_skips_a_rewrite_a_directive_silences`, the
  manifest row, a `sync_db_config` change for folder handles, the three
  server unit tests the checkpoint dropped, and its record in the lane
  document), and the value-transfers lane's uncommitted edits under
  `rust/tcl-compiler/src/` and `rust/tcl-registry/src/`.

### `rust` has moved under the branch

The branch's merge base with `rust` is `3b5eba8a`. Since then `rust` has
landed, in this lane's files:

- #2121 (`d866e41`, PR #2148): `DiagSection::Xc` and the thirteen XC rows —
  the same set slice 1 added — with `TranslationItem::diagnostic_code` typed
  as `DiagCode` inside `f5-xc`, the xtask `xc` arms, regenerated catalogues,
  and `published_xc_codes_are_known_diag_codes_issue_2121`.
- #2120 (`3d759e66`, `5a9d8f78`, PR #2148): `tcl opt` optimises each input
  as its own program (its own `effective_dialect`, registry and
  `analysis_source`), joins the outputs as `combine_sources` joined the
  inputs, keeps a single input's bytes exactly, and lists each file's
  rewrites under a `# file: <label>` line inside the trailing summary; tests
  `opt_does_not_fold_a_store_across_a_file_boundary` and
  `opt_over_several_inputs_keeps_the_first_shebang_at_byte_zero`.
- #2119 (`69fd665`, `8957d6d`, PR #2150): `optimiseDocument` honours the
  switch, the profile's set, the per-code overrides and the directives,
  resolved per pass from `Analyser::analyse(..).suppressed_lines`; a named
  `profile` argument selects the categories, and an absent or unrecognised
  one falls back to the configured profile; `optimise_source_multipass_admitting`
  joins `rust/tcl-compiler/src/optimiser/manager.rs`.
- #2122 (`b0d38a4`): the server's abstention sites call `should_abstain`.
- #2123 and #2149 (`6985a9b`, `c696ebc`): every member of an intact
  optimisation group carries `{group, edits}` in its diagnostic's `data` and
  never the flat `replacement` / `startOffset` / `endOffset` triple; a group
  that lost a member carries no payload; `optimiseDocument` drops a group
  that lost a member.

The checkpoint contradicts three of these: `run_opt` folds inputs (#2120);
`lift_report`, `rewrite_action` and `optimise_under_policy` offer and apply
half a group (#2149); `optimiseDocument` answers an absent argument with
`full` (#2119). DP5.2, DP7.1 and DP4.2 adopt `rust`'s semantics on the
branch, so the orchestrator's merge of `rust` resolves each conflict to the
lane's side (§ Boundaries, `rust`), and DP10.4 finishes the reconciliation.

**As merged.** The orchestrator merged `rust` (at `08bceb36`) before DP4.2,
DP5.2 and DP7.1 had landed, so the merge resolved each conflict to those
items' shapes by writing them, as specified here: DP4.2's
`builder_with_invocation` and `optimise_document_command` with its four
tests (`rust`'s three ported onto `apply_global_config`, and
`a_project_profile_overrules_the_command_argument`); DP5.2's per-input
`run_opt` (`share_one_project` deleted, the KCS bullet rewritten); DP7.1's
`applicable_rewrites` / `applicable_items`, `rewrite_actions` and the
`lift_report` group payloads with their core and server tests
(`a_grouped_optimisation_is_never_independently_applicable_issue_2149`
ported onto `lift_report`, now also pinning that a group that lost a
member carries no payload); and DP7.2's analyser directive map in the
rewrite loop, because `rust`'s #2119 already read that map per pass. The
XC test runs over `xc_findings`. What DP10.4 still owes is the thin
wrapper over `optimise_source_multipass_admitting` and the review.

*Superseded (R1, the owner's ruling of 2026-09-22; D36).* The DP4.2 half
of this note — `builder_with_invocation` putting the call's named profile
in the editor slot, so a project profile beats the argument, with the pass
rule kept — no longer describes the tree: `builder_with_invocation` is
gone, the named profile is `PolicyBuilder::requested_profile` and wins over
every layer, the pass count is the profile in force's
(`OptimisationProfile::max_iterations`), and
`a_project_profile_overrules_the_command_argument` is
`an_invocation_profile_overrules_the_project_file`. The DP5.2 half's "the
profile in force sets the passes" stands, with the profile in force now
the named one when `--profile` is given.

### Goal and exit per slice

| Slice | Deliverable, in the page's words | Exit evidence |
|---|---|---|
| 4 — Server: the LSP adapter | "`publish_fast_tier`, `refine_and_lift_diagnostics` and `analysed_diagnostics_for` … each call one function; the lifts, `finalise_diagnostics`, `suppress_duplicate_o120` and `retain_unsuppressed_diagnostics` become the adapter or disappear. `tcl-lsp.optimiseDocument` joins the same path." | DP4.0 green on every suite and on the whole `e2e`; DP4.1's `tcl-lsp-db` documentation commit; DP4.1's declared-skip tests; DP4.2's four `optimise_document_command` tests; DP4.3's pins of the folder layers, the multi-root corner and W305 on an abstaining document; DP8.2's `lifted_report` as the one call every publish path makes; the LSP pass of the truth table (DP9.5). |
| 5 — CLI | "`collect_rows` … becomes the row adapter; `resolve_disabled` becomes the invocation layer over the seeded set; the project and global layers resolve per input file (closes #2063). `run_opt` … applies only shown rewrites, and stops folding several inputs into one `combine_sources` text when their policies differ (closes #2062)." | DP4.0's three CLI tests; DP5.1's INI tests and `diag_a_project_file_turns_a_code_back_on`; DP5.2's per-input tests (`rust`'s #2120 pair); `samples_optimiser_profiles_are_regenerated`; DP5.4's isolation; the CLI passes of the truth table (DP9.6). |
| 6 — MCP | "`analyze`, `validate`, `review`, `find-legacy`, `optimize` and `code_actions` … read the report; the diagnostics tools gain a `disable` argument (closes #2061)." | DP4.0's `policy_tests`; DP6.1's shared producer run; DP6.2's #2061 cases (the three `review` programs, the style pass); the MCP passes of the truth table (DP9.7). |
| 7 — Code actions | "`check_diagnostic_actions` and `code_actions` … lift fixes from shown findings only and lose their filtering parameters; a shown finding carrying `FindingData::Rewrite` becomes an offerable action instead of only a diagnostic payload." | The core integration suites green (DP4.0); DP7.1's group tests in core, server and code actions; DP7.2's multi-line directive test; DP7.3's end-to-end tests; DP8.3's lightbulb on the published report and its W115 tests; the code-action passes of the truth table (DP9.5, DP9.7). |
| 8 — O111 as a producer | "O111 becomes a producer over that fact, emitting a `Finding` at the same span for every unbraced expression, and policy decides both independently: disabling W100 does not silence O111, and the optimiser gate reaches O111 in the one place it reaches every other O-code." | `with_brace_expr_hints` and `Report::extend` deleted; `brace_expr_hints` tested in core and on the server; `large_file_publishes_fast_tier_before_deep_tier` green; rows 39–41 of the truth table green on every surface. |
| 9 — The truth table, `--show-suppressed`, the `suppressed` array | "One table from `(program, policy)` to `Report`, in `tcl-lsp-core` beside `apply`, is the parity gate … The table lives once and every adapter runs it." "`--show-suppressed` renders `suppressed()` too, one row per finding with its reason." "Each payload gains a `suppressed` array of `{code, range, reason}`." | `rust/tcl-lsp-core/src/diagnostic_policy/truth_table.rs` green in core; every adapter pass green (server `--lib`, `tcl-cli --test cli`, `tcl-mcp`); DP9.2 and DP9.3 tests green; the four server unit tests the page names deleted. |
| 10 — Owner docs | The six documents "point at the policy step instead of describing the server's lifts", the `tcl-lsp-db` row "loses 'suppression policy'", the consumer list "becomes one consumer", config-precedence "records that the layers resolve in `tcl-lsp-core` and that a surface's flags are the editor layer", and the KCS how-to "can finally promise the five scopes on every surface, and document `--show-suppressed`". | `cargo xtask kcs-index-links` and `cargo xtask owner-resolution` green with the policy step's manifest row; no retired name (§ Transitional pieces) in `docs/` outside the lane documents; the page's status reads *built*; DP10.4's reconciliation with `rust` green; the lane document removed (DP10.5). |

### Work items

#### DP4.0 — Green the checkpoint: the hand-off's remaining steps

`opus`, L, after nothing. In flight concurrently with this plan. Its scope is
§ Status at hand-off › Remaining steps 1 to 7, restated here in the same
terms so the implementer and the reviewer read one list, less step 6's
`tcl-lsp-db` commit (DP4.1 makes it, beside the declared skip it
documents) and step 7's `make rust-check` (C4 runs it once the workspace
compiles); step 8 is one commit. The implementer's record is the lane
document's § DP4.0 — the checkpoint made green.

1. Core: `cargo test -p tcl-lsp-core --lib`, then the five integration
   suites and `lsp_edit_workspace` (*the suites*).
2. Server: `cargo test -p tcl-lsp-server --lib`; the e2e subsets in order;
   the whole `--test e2e`.
3. The three drafted CLI tests land in `rust/tcl-cli/tests/cli.rs` with
   their helpers `Scratch`, `run_tcl_env`, `diag_codes_by_file` and the
   constant `UNPROVABLE_LOOP`:
   `diag_seeds_the_default_off_codes_like_the_editor`,
   `diag_resolves_the_project_and_global_layers_per_input_file` (#2063),
   `opt_applies_only_the_rewrites_the_policy_shows` (#2062). Gate: `cargo
   test -p tcl-cli --test cli`.
4. The MCP parity tests, `#[cfg(test)] mod policy_tests` in
   `rust/tcl-mcp/src/tools.rs`, against the `*_with` forms with a global
   layer parsed from INI text by `config_ini::settings_from_ini` (never the
   machine's `config.ini`): `analyze_honours_an_inline_noqa`,
   `analyze_honours_disable_enable_and_the_global_file`,
   `analyze_seeds_the_default_off_codes_and_enable_reaches_them`,
   `analyze_reports_the_resolved_severity`,
   `the_grouping_tools_read_the_shown_set`,
   `optimize_applies_only_the_rewrites_the_directives_leave_shown`,
   `optimize_honours_the_profile_the_overrides_and_the_global_file`,
   `code_actions_offer_nothing_for_a_silenced_finding`,
   `code_actions_offer_a_shown_rewrite_as_a_quickfix`. Gate: `cargo test -p
   tcl-mcp`.
5. The crate clippy.
6. The three retirements:
   - `Policy::from_disabled_set` is deleted with its doc, the `policy_tests`
     test `a_flat_disabled_set_records_the_callers_layer_and_opens_every_other_gate`
     and the `BuildHasher` import. Its other three callers —
     `apply_tests::the_style_pass_through_apply_keeps_the_orchestrator_rules`,
     `sslictcl_diagnostics::tests::every_finding_carries_the_loader_span_and_the_sslictcl_producer`,
     `lsp_edit_workspace::style_orchestrator_merges_checks_and_policy_hides_a_disabled_code`
     — build `PolicyBuilder::new().layer(PolicyLayer::Editor,
     &json!({"diagnostics": {"<CODE>": false}})).build()`, and the expected
     reason stays `Reason::Disabled(PolicyLayer::Editor)`.
   - `sslictcl_diagnostics::supersede_analyser_diagnostics` is deleted with
     `the_loader_supersedes_the_unknown_command_verdict` and the `Diagnostic`
     import, and its name leaves the "SslicTcl editor projection" row of
     `docs/design/contracts/shared-utility-contracts-rust.md`. The rule's
     coverage stands in `apply_tests::a_producer_owns_the_document_without_a_finding_of_its_own`
     and `policy_tests::the_overlap_table_follows_the_dialect`;
     `SUPERSEDED_ANALYSER_CODES` stays (`dialect_overlaps` reads it).
   - `InputDocument::encoding_diagnostics` is deleted with the
     `StyleDiagnostic` / `encoding_integrity_diagnostics` imports; the
     `decode` field's doc and the `read_input_documents` comment name the
     byte-integrity pass (`tcl_lsp_core::source_decode::encoding_integrity_diagnostics`)
     instead.
7. The catalogue gates.
8. One commit, `wip(diagnostic-policy): …`, every file staged by path; the
   lane document gains the record and § Progress's DP4.0 row.

What DP4.0 finds, and the fix each takes (the record has the detail):

- **The constant fold is O101, not O102.** `set x [expr {1 + 2}]` folds
  under O101 ("Fold constant expression"); O102 forwards a variable's
  reaching literal. The drafted `opt_applies_only_the_rewrites_the_policy_shows`,
  the MCP tests and `diagnostic_report::tests::optimise_under_policy_skips_a_rewrite_a_directive_silences`
  name O101, and the fold's quick-fix replaces the whole statement
  (`new_text` `set x 3`). The editor's and MCP `code_actions`' default
  `readability` profile keeps O101 off unless a layer selects `full`;
  `tcl opt` and MCP `optimize` default to `full`.
- **The fold's program stands alone.** `set x [expr {1 + 2}]\n` with
  nothing after it: followed by the draft's `puts $x`, `full` inlines the
  value and deletes the store (O100, O109), and neither `set x 3` nor
  `[expr …]` is left to look for. Every later test in this plan that looks
  for `set x 3` uses the same program (`FOLD`, DP9.4).
- **The drafted INI spelling is not read.** The per-file test wrote
  `[diagnostics]\nW112 = false` / `W112 = true`, which `insert_diagnostics`
  ignores. DP4.0 writes the documented `disabled = W112`, and the draft's
  "a project file turns the code back on" becomes the two precedence pairs
  the grammar can say: `--enable W112` overrules a global disable, and a
  project `disabled = W112` overrules `--enable`. A *file* layer turning a
  code back on is DP5.1's.
- **`spec_packs` e2e.** Every configured folder now carries its own
  analyser skip, so every folder gets its own salsa `AnalyserConfig`
  handle, and `sync_db_config` moved only the session handle's pack key:
  `a_workspace_packs_argument_roles_drive_semantic_tokens` and
  `the_bundled_eda_loadables_make_their_vendor_commands_known` fail. The
  fix sets the pack key on every live folder handle in `sync_db_config` —
  packs are a workspace fact.
- **Three server unit tests the checkpoint dropped.** Its block
  replacement in `rust/tcl-lsp-server/src/lib.rs` took
  `parse_non_ascii_mode_maps_settings`,
  `settings_non_ascii_mode_nested_and_flat` and
  `semantic_tokens_capability_advertises_delta_and_range` with it, and
  turned the `"http::foo\n"` literal in a `code_actions` unit test into a
  real line break; all are restored as they were.
- **The hand-off's `e2e` subsets pass as they stand**: no code-action
  count or folder-config expectation changes.

Rules DP4.0 keeps:

- **`too_many_lines`** on `collect_rows` or `code_actions_with`: split, never
  `#[allow]`.
- **`samples_optimiser_profiles_are_regenerated`** stays green: for a
  directive-free input under an empty global layer `optimise_under_policy`
  applies what `optimise_source_multipass_filtered` applied. A red one is a
  regression, not a regeneration.
- **A red `gen-*` gate** is another lane's stale generated file: report it,
  never regenerate another lane's output.

Exit: steps 1 to 7 green; the record and § Progress's DP4.0 row in the
lane document.

#### DP4.1 — The analyser's skip is the policy's on every path, and the report declares it

`opus`, M, after DP4.0.

Files: `rust/tcl-lsp-core/src/diagnostic_policy.rs`,
`rust/tcl-lsp-core/src/config_ini.rs`, `rust/tcl-lsp-core/src/config_ini/tests.rs`,
`rust/tcl-lsp-server/src/lib.rs`, `rust/tcl-cli/src/commands/diag.rs`,
`rust/tcl-mcp/src/tools.rs`; `rust/tcl-lsp-db/src/lib.rs` in a commit of
its own (below).

Core, `diagnostic_policy.rs`:

- `Policy::production_skip(&self) -> BTreeSet<DiagCode>` narrows to step 4
  of `apply`: the catalogued codes whose per-code decision is off, and the
  `default_off` codes no layer turned on. The family gates are not in it:
  no producer that honours a skip emits an optimisation or shimmer code —
  the analyser emits neither, and the compiler checks and the optimiser
  always run — so a family-gated code is never skipped and must not be
  declared as if it were. The doc says so.
- `pub fn gap_reason(&self, code: DiagCode) -> Option<Reason>` — why a code
  with no finding is absent, in `apply`'s order for the steps that need no
  span: `ReportingOff`, `Excluded`, `EncodingAbstention` (unless the code is
  in `ABSTENTION_SURVIVORS`), `FileDirective` (the file bucket names the
  code or `*`), then `self.code_reason(code)`.
- `pub fn analyser_skip(&self) -> BTreeSet<DiagCode>` —
  `self.production_skip()` plus every code the directives' file bucket
  names that parses as a `DiagCode`: `Analyser::analyse` folds
  `parse_file_suppression` into its own `disabled_diagnostics`, so those
  codes go uncomputed too. `*` is not a code and is not in it.
- `Report::declare_skipped` records `policy.gap_reason(code)` (was
  `code_reason`).
- `pub fn declare_analyser_skip(&mut self, policy: &Policy)` on `Report` —
  `self.declare_skipped(policy.analyser_skip(), policy)`.

Server, `lib.rs`:

- Every write of `Backend::disabled_diagnostics` becomes the session layers'
  `PolicyLayers::production_skip()`, computed after the `policy_layers`
  write it accompanies: `apply_global_analyser_knobs` reads
  `self.policy_layers` (both its callers set the layers first —
  `pull_and_apply_config_values` and the `#[cfg(test)]`
  `apply_global_config`); `apply_initialization_options` and
  `did_change_configuration` compute it after merging the payload into
  `layers.editor`. `Backend::with_store` and the test backend initialise
  it, and the salsa `AnalyserConfig`'s `disabled_diagnostics`, from
  `PolicyLayers::default().production_skip()`. `parse_folder_config` sets
  `fc.disabled_diagnostics = Some(PolicyLayers { editor: cfg.clone(),
  ..PolicyLayers::default() }.production_skip())` for its merged-payload
  test seam; the pull overwrites it with the folder's own layers, as today.
- `apply_folder_configs` gives a folder its own `AnalyserConfig` handle
  only when one of its resolved inputs differs from the session's: the
  sorted skip, the non-ASCII mode, the extra commands, the generic
  variable patterns, the package provides, the BIG-IP version or the
  targets. Since the checkpoint every configured folder carries
  `Some(skip)`, so every one got a handle, and a document under it is
  analysed twice per revision — for diagnostics under the folder's handle,
  for `db_document_symbols` under the session's (DP4.0's open
  uncertainty). A folder whose inputs equal the session's shares the
  session's handle and its memo; the published set is identical.
- `resolved_analysis_settings` loses the shimmer fold and its stale "so the
  compiler-check lift drops them" comment.
- `publish_fast_tier`, `refine_and_lift_diagnostics` and
  `analysed_diagnostics_for` replace `report.declare_skipped(skipped_codes(&disabled),
  &policy)` with `report.declare_analyser_skip(&policy)`. `f5_model_report`
  declares nothing (no analyser ran) and loses its `disabled` parameter.
  `LiftInputs::disabled`, `F5PullInputs::disabled` and
  `PullRefinementInputs::disabled` stay: the cross-file arity predicates
  read them. `skipped_codes` is deleted, and so is the
  `use tcl_lsp_core::config_ini::{default_disabled_set, settings_disabled_diagnostics}`
  import.

CLI, `collect_rows`: `report.declare_skipped(skip…)` becomes
`report.declare_analyser_skip(&policy)`; `skip` stays as the analyser's
`with_disabled_diagnostics` input. MCP, `Analysed::report_with`: likewise,
and the `skipped` field goes.

Core, `config_ini.rs`: `default_disabled_set`,
`settings_disabled_diagnostics` and `settings_severity_overrides` are
deleted with their tests in `config_ini/tests.rs` and the module doc's
sentence naming "the four readers"; `parse_severity_value` stays (the
builder reads it).

Preserved: every published diagnostic, every CLI row and every MCP payload.
The analyser's skip loses only codes it never emits (optimisation and
shimmer codes) and spellings the catalogue lacks.

Changes: `tcl-lsp.getEffectiveConfig`'s `disabled_diagnostics` lists
catalogued codes only — a mistyped spelling or a `*` no longer appears
(§ The conversions: "an unparseable code is a conversion failure rather
than a value that silently skips every table"). `Report::reason_for`
explains an absent analyser code the file directive folded away
(`FileDirective`), where it answered `None` — § Producers that change: "the
policy step is told which codes the producer skipped … so a report never
shows a gap it cannot explain".

Tests:

- `apply_tests::production_skip_is_every_code_the_policy_hides_without_a_finding`
  becomes `production_skip_is_the_per_code_decision_and_the_seed`: a Global
  W210 disable and the W242 seed are in it; O107 under the default
  readability profile and S100 under `shimmer: false` are not.
- `apply_tests::a_gap_reason_follows_the_step_order`: under abstention W210
  answers `EncodingAbstention` and W109 `None`; a file bucket naming W210
  answers `FileDirective` ahead of a Global disable of W210; reporting off
  answers `ReportingOff` before everything.
- `apply_tests::the_analyser_skip_adds_the_codes_the_file_directive_names`:
  over `# tcl-lsp: disable=W210, *\n` the analyser skip holds W210 and the
  production skip does not.
- `diagnostic_report::tests::a_file_directive_gap_is_declared`: analyse
  `# tcl-lsp: disable=W210\nputs $y\n` as the server does
  (`Analyser::with_disabled_diagnostics` over the production skip), build
  the report with `declare_analyser_skip`; no W210 finding exists and
  `report.reason_for(DiagCode::W210, Span::new(0, 0))` is
  `Some(Reason::FileDirective)`.
- Server `a_folder_matching_the_session_shares_its_analyser_handle`:
  `apply_folder_configs` with a folder whose `disabled_diagnostics` equals
  the session skip leaves `folder_db_configs` empty; with `W100` added to
  it, the folder has one handle. `folder_db_config_handle_is_retired_and_revived`
  stays green unchanged.
- Server `the_session_skip_is_the_layers_production_skip`: after
  `apply_global_config(&json!({"diagnostics": {"W210": false, "W242": true}}))`
  the session skip is exactly `{"W210"}`; under `{"shimmer": {"enabled":
  false}}` `resolved_analysis_settings` returns no S-code.

The `tcl-lsp-db` commit (hand-off step 6's, moved here from DP4.0), alone,
documentation only, in `rust/tcl-lsp-db/src/lib.rs`:

- `file_analysis` and `file_analysis_incremental` gain the paragraph:
  "`config.disabled_diagnostics` is the analyser's production-time
  skip — rule 2's permitted saving in
  `docs/design/compiler/diagnostic-policy.md` § Producers that change:
  the codes the document's policy turns off, which the analyser need
  not compute. It is never a presentation filter. The surface that
  reads this analysis declares the same set to its report, so a code
  left uncomputed is explained rather than read as clean; what the
  document shows is the policy step's decision."
- `apply_cross_file_resolution`: the sentence "it is produced *after*
  the analyser applied its own `apply_disabled_diagnostics` filter (and
  the LSP lift does not re-filter), so the filter must be replicated
  here" becomes "the synthesised arity codes honour the same
  production skip as the analyser's own; whether they show is the
  policy step's decision". `apply_project_callback_arity` and
  `project_diagnostics` gain the same clause where they read the set.
- `CompilerDiagnostics`: "Returned by `compiler_check_diagnostics` for
  the server to filter (optimiser master switch / per-code disables)
  and lift into LSP diagnostics" becomes "Unfiltered: every surface
  converts these to findings, and the policy step decides what
  shows".
- `compiler_check_diagnostics`: "Byte-identical to the direct
  `lift_compiler_diagnostics` build" becomes "Byte-identical to
  `compiler_check_diagnostics_uncached`".

Staging never uses an interactive `git add`. When `git diff
rust/tcl-lsp-db/src/lib.rs` shows only these hunks, `git add
rust/tcl-lsp-db/src/lib.rs`; otherwise write the diff to
`$SCRATCH/db.patch`, delete every hunk that is not one of these,
`git apply --cached --check $SCRATCH/db.patch && git apply --cached
$SCRATCH/db.patch`. Before committing, `git diff --cached --stat`
lists this one file and `git diff --cached` shows doc comments only.
Message: `wip(diagnostic-policy): file_analysis — the disabled set is
the declared production skip`. Gates: `cargo check -p tcl-lsp-db`,
`cargo clippy -p tcl-lsp-db --no-deps -- -D warnings` (`doc_markdown`
reads doc comments).

Gates: *the suites* for the five crates, the e2e subsets `noqa`, `severity`,
`config`, `diagnostics`, `sslictcl`; the crate clippy.

#### DP4.2 — `optimiseDocument`'s argument follows #2119, in the editor slot

*Superseded in part by R1 (D36).* The argument is no longer a layer in
the editor slot: a named profile wins over the project file, the pass
count follows the profile in force, and the fourth test below is
`an_invocation_profile_overrules_the_project_file`, joined by
`a_named_profile_leaves_the_switch_and_the_codes_to_the_layers` and
`the_pass_count_follows_the_profile_in_force`. The rest stands as written.

`opus`, S, after DP4.0.

Files: `rust/tcl-lsp-server/src/lib.rs`.

- `PolicyLayers::builder_with_invocation(&self, invocation: Option<&serde_json::Value>)
  -> core_policy::PolicyBuilder` — the global layer, the editor layer, the
  invocation layer when `Some`, then the project layer. This is the page's
  slot ("A surface's own flags occupy the editor layer's slot in the
  precedence order, under the project file and over the global file"),
  with the call's own argument over the editor's setting inside it.
  `builder()` becomes `self.builder_with_invocation(None)`.
- `optimise_document_command`: `named` is `args.get(1)` as a string that
  equals some `OptimisationProfile::ALL[i].name()`; the invocation layer
  `{"optimiser": {"profile": named}}` exists only when `named` is `Some`.
  The pass count stays `if args.get(1).and_then(Value::as_str).unwrap_or("full")
  == "full" { 5 } else { 1 }`.

Preserved: the `{source, optimisations}` payload and its item fields; the
pass rule.

Changes, from #2119 as `rust` merged it (#2150): an absent argument
optimises under the configured profile's categories (was `full`'s — the
VS Code client sends none); an unrecognised name falls back to the
configured profile (was `OptimisationProfile::parse`'s `readability`
fallback). A project `[optimiser] profile` overrules a named argument — the
page's slot rule; `rust`'s `optimiser_policy_for_command` lets the argument
win over the project too (§ Open questions 2).

Tests (server `--lib`, named as `rust`'s so the merge keeps one copy, and
configured through `apply_global_config` because the branch's policy reads
`policy_layers`, not `optimiser_profile` / `optimiser_enabled`):

- `optimise_document_command_honours_the_optimiser_policy_issue_2119`: on a
  default backend `set x [expr {1 + 2}]\nputs $x\n` yields no rewrite;
  under `{"optimiser": {"profile": "aggressive"}}` it yields some; adding
  `"enabled": false` leaves the source byte-identical; turning every
  baseline code off per code applies none.
- `optimise_document_command_profile_argument_selects_the_categories_issue_2119`:
  `[uri, "full"]` folds `puts [llength [list a b c]]\n` to `puts 3` on a
  default backend; `[uri, "not-a-profile"]` does not.
- `optimise_document_command_honours_noqa_issue_2119`: the O100
  differential — `set x [expr {1 + 2}]\n# noqa\nputs $x\n` under
  `aggressive` applies no O100 where the unmarked control does.
- `a_project_profile_overrules_the_command_argument`: `Backend::policy_layers`
  set directly to a project layer `{"optimiser": {"profile":
  "readability"}}`; `[uri, "full"]` over `set x [expr {1 + 2}]\n` returns
  the source byte-identical (O101 is outside `readability`); the control
  with no project layer returns `set x 3\n`.

Gates: `cargo test -p tcl-lsp-server --lib -- optimise_document`; the e2e
subsets `commands`, `config`, `diagnostic_matrix` (every e2e call passes
`"full"` explicitly); the crate clippy.

#### DP4.3 — Pin the hand-off decisions no test pins yet

`sonnet`, S, after DP4.1.

Files: `rust/tcl-lsp-server/src/lib.rs` (`mod tests`),
`rust/tcl-cli/tests/cli.rs`. Four tests, no production change. DP4.0's
record lists what no test pins; DP4.2 takes `optimiseDocument`, DP7.3 the
server's rewrite quick-fix, and these the rest:

- `a_configured_folder_resolves_its_own_three_layers`: `folder_configs`
  holds two folders; the first carries `policy_layers: Some(PolicyLayers {
  project: json!({"diagnostics": {"W112": false}}), ..Default::default() })`,
  the second `policy_layers: None`. For a URI under the first,
  `resolved_policy_layers(&uri).await.builder().build().code_reason(DiagCode::W112)`
  is `Some(Reason::Disabled(PolicyLayer::Project))`; for a URI under the
  second it is `None` (the session's layers). This pins the hand-off
  decision "Folder policy layers are the folder's own three" (§ Decisions
  taken, D2).
- `a_secondary_root_does_not_inherit_the_primary_project_file`: the
  session's `policy_layers.project` is `{"diagnostics": {"W112": false}}`
  (the primary root's `.tcl-lsp.ini`); a second folder carries
  `line_length: Some(100)` and `policy_layers: Some(PolicyLayers::default())`
  (its own three, none with a policy section). Under that folder
  `code_reason(DiagCode::W112)` is `None`; outside every folder it is
  `Some(Reason::Disabled(PolicyLayer::Project))`. This pins the multi-root
  corner (§ Open questions 7) so that the owner's answer flips one
  assertion.
- `apply_global_config_populates_the_editor_layer`: after
  `apply_global_config(&json!({"diagnostics": {"W112": false}}))`,
  `resolved_policy_layers` for any URI yields `code_reason(W112) ==
  Some(Reason::Disabled(PolicyLayer::Editor))`, and the session skip holds
  `W112` (DP4.1).

- CLI `diag_keeps_a_bidi_control_on_an_abstaining_document`: a scratch file
  holding the bytes `FF FE` followed by the UTF-8 text `# \u{202E}hidden\nputs
  hi\n`, under an empty `XDG_CONFIG_HOME`; `tcl diag --json` reports exactly
  W109 and W305 (`decode_source` decodes the tail losslessly, and W305 is
  an abstention survivor). The control without the bidi character reports
  W109 alone. This pins D10 end to end; the core test
  `an_integrity_only_document_carries_the_bidi_finding` pins the pass.

Gates: `cargo test -p tcl-lsp-server --lib -- resolves_its_own_three_layers
inherit_the_primary_project_file populates_the_editor_layer`; `cargo test -p
tcl-cli --test cli -- abstaining_document`.

#### DP5.1 — An INI layer can turn a code back on

`opus`, S, after DP4.0.

Files: `rust/tcl-lsp-core/src/config_ini.rs` (`insert_diagnostics`,
`insert_optimiser`), `rust/tcl-lsp-core/src/config_ini/tests.rs`,
`rust/tcl-cli/tests/cli.rs`.

Change: in `[diagnostics]`, a key whose trimmed, upper-cased spelling
parses as a `DiagCode` and whose value `parse_bool` accepts becomes
`{CODE: bool}` — `W242 = true` enables, `W111 = false` disables. `disabled =
…` keeps producing `{CODE: false}`; the per-code keys are inserted after the
`disabled` list in file order, so within one file a per-code key wins over
`disabled` for its code. In `[optimiser]`, the same per-code keys;
`enabled`, `profile` and `disabled` keep their meanings (no code is spelled
like them). A per-code key whose value `parse_bool` rejects contributes
nothing. The server's `render_config_ini` is unchanged: it writes
`disabled = …`, which stays readable.

Why: § The five scopes and their order — "project, editor and global can
each turn a code **back on**, so a layer's contribution is a per-code
tri-state and not a set union" — and § The truth table's "a default-off
code turned back on at each layer". The parser reads only `disabled =`, so
no file layer could enable W242 or overrule a lower layer's disable; the
builder's tri-state had no INI spelling, and the KCS how-to's promise
("a project config that enables a code overrules an editor or global
config that disables it") held only for editor settings.

Preserved: every existing INI file reads identically — no documented or
shipped file carries a code-named key in these sections.

Tests: `config_ini/tests.rs` —
`a_per_code_key_turns_a_code_on_or_off` (`[diagnostics]\ndisabled =
W111\nW242 = true\nw111 = true\n` → `{"W111": true, "W242": true}`);
`an_optimiser_per_code_key_keeps_the_switch_keys` (`[optimiser]\nenabled =
false\nO106 = true\n` → `{"enabled": false, "O106": true}`);
`an_unparseable_per_code_value_is_ignored` (`W242 = maybe` contributes
nothing; `W999 = true` contributes nothing). `tests/cli.rs` —
`diag_a_project_file_turns_a_code_back_on`: a global `config.ini` with
`[diagnostics]\ndisabled = W112` and a project `.tcl-lsp.ini` with
`[diagnostics]\nW112 = true` → `tcl diag --json` on a file under the
project reports W112, and a sibling outside it does not.

Docs: `docs/design/contracts/xdg-config.md` (the `[diagnostics]` and
`[optimiser]` key tables gain a "`<CODE>` — bool — turns one code on or
off; wins over `disabled` in the same file" row); the KCS how-to's § 3
example gains `W242 = true` (DP10.3 folds the prose). Gates: `cargo test -p
tcl-lsp-core --lib -- config_ini`, `cargo test -p tcl-cli --test cli --
turns_a_code_back_on`, `cargo xtask kcs-index-links`.

#### DP5.2 — `tcl opt` optimises every input on its own

`opus`, S, after DP4.0.

Files: `rust/tcl-cli/src/commands/transform.rs` (`run_opt`),
`rust/tcl-cli/src/commands/policy.rs`, `rust/tcl-cli/tests/cli.rs`,
`docs/kcs/features/kcs-feature-tcl-verb-cli.md`.

`run_opt` loops over the input documents. For each: `dialect =
document.effective_dialect(explicit)`, `registry =
registry_for_dialect(dialect.name)`, `source = document.analysis_source()`,
`policy = layers.builder_for(document.path.as_deref()).dialect(dialect).build()`,
and `optimise_under_policy(&source, &registry, Some(dialect),
policy.optimiser.profile.max_iterations(), &policy)`. The output is the one
section itself for a single input (no trim, no join) and `combine_texts`
over the sections for several. The stdout summary is `# optimised: N
rewrite(s)` followed, for several inputs, by a `# file: <label>` line ahead
of each file's entries (a file with none is skipped) and, for one input, by
the entries alone. `combined_effective_dialect` survives only to pick the
highlighting dialect. The fold condition (`share_one_project` and the
`Directives::scan` emptiness test) goes, and `policy::share_one_project`
(a free function since DP4.0) is deleted with it and with its assertion in
`a_pathless_document_has_no_project_layer`.

Why: #2120, closed on `rust` by PR #2148 — "Each input is optimised as its
own program, with its own dialect, its own directives and … its own
project configuration" — and the page's slice 5, whose "when their
policies differ" #2120 widens to always. This is `rust`'s `run_opt` shape
(`3d759e66`, `5a9d8f78`) with the policy step inside it, so the merge
resolves to this version.

Preserved: the rewrites a directive-free input receives; the summary
block's format for a single input; `--profile`, `--disable`, `--enable` as
the invocation layer; `samples/optimiser/*`.

Changes: several inputs never fold across a file boundary (#2120); a
single input renders as its own section, untrimmed, exactly as `rust`'s
#2120 renders it; a lone-`\r` input is optimised in its analysis form, so
the output carries `\n` endings (#2120 as merged: `analysis_source`); the
pass count follows the profile in force, so a project `[optimiser] profile`
decides both the category set and the passes (§ Configuration's slot;
§ Decisions taken, D17). *Superseded in part by R1 (D36):* `--profile` is
no longer in the invocation layer; named, it is the profile in force and
decides both, and the project file's profile decides them only when it is
omitted.

Tests (`tests/cli.rs`, added verbatim from `rust` so the merge sees
identical additions): `opt_does_not_fold_a_store_across_a_file_boundary`,
`opt_over_several_inputs_keeps_the_first_shebang_at_byte_zero`. DP4.0's
`opt_applies_only_the_rewrites_the_policy_shows` keeps passing (its two
files are now always separate). `samples_optimiser_profiles_are_regenerated`
stays green. Docs: the `opt` bullet of `kcs-feature-tcl-verb-cli.md` §
Verb contracts replaces "Inputs are folded into one text only when their
policies agree (one project, no directives); otherwise each is optimised on
its own and the outputs joined" with "Each input is optimised as its own
program under its own policy and the outputs joined; the summary names each
file's rewrites". Gates: `cargo test -p tcl-cli`, the crate clippy.

#### DP5.3 — The batch verbs read no LSP document gate

`sonnet`, S, after DP4.0.

Files: `rust/tcl-cli/src/commands/policy.rs` (`ConfigLayers::builder_for`),
`rust/tcl-mcp/src/tools.rs` (`PolicyInputs::builder`).

Both builders call `.reporting(true)`. `excluded` is never set on either
surface. Doc sentence on each: "`[features]` configures the language
server's features (`docs/design/contracts/xdg-config.md` § `[features]`); a
batch verb reports whatever it is asked to, so neither whole-document gate
reaches it."

Why: the checkpoint lets a configuration file's `[features] diagnostics =
false` silence `tcl diag` and the MCP tools, which nothing mandates — the
page's rule 1 lists the steps every surface applies and neither
whole-document gate is among them, and § Today records them as
editor-only. This restores today's CLI behaviour (§ Open questions 4).

Tests: `commands/policy.rs` —
`a_features_toggle_does_not_silence_the_cli`: a `ConfigLayers` whose global
layer is `{"features": {"diagnostics": false}}` builds a policy with
`document.reporting == true`. `tools.rs` `policy_tests` —
`the_features_toggle_is_an_editor_setting`: `analyze_with` over `puts $y\n`
under a global layer from `[features]\ndiagnostics = false\n` still reports
W210. Gates: `cargo test -p tcl-cli --lib`, `cargo test -p tcl-mcp`, the
crate clippy.

#### DP5.4 — The CLI tests never read the machine's `config.ini`

`sonnet`, S, after DP4.0.

Files: `rust/tcl-cli/tests/cli.rs`.

- `fn tcl() -> Command` — `Command::new(env!("CARGO_BIN_EXE_tcl"))` with
  `XDG_CONFIG_HOME` set to `empty_config_home()`, a process-wide empty
  directory under `std::env::temp_dir()` created once through a
  `std::sync::OnceLock<PathBuf>` and never removed. `config_path_for`
  consults `XDG_CONFIG_HOME` first on every platform, so this isolates the
  global layer on Linux, macOS and Windows alike.
- Every spawn in the file builds on it: `run_tcl`, `run_tcl_in`,
  `run_tcl_allow_failure`, `run_tcl_env` (whose explicit `env` pairs then
  override the default), `diag_messages`, `multi_file_diag_text`,
  `tcl_diag_rows`, `sslictcl_diag_rows`, and the inline spawns in
  `command_info_discovers_the_current_projects_spec_pack`,
  `diag_analysis_changes_when_the_current_projects_spec_pack_is_present`,
  `minimize_missing_code_errors` and `minimize_reduced_output_still_fires`.
- `Scratch` replaces the hand-rolled nanosecond directories in
  `multi_file_diag_text`, `tcl_diag_rows`, `sslictcl_diag_rows` and
  `minify_symbol_map_written_for_plain_minify` (behaviour-identical).

Why: since slice 5 `diag`, `lint`, `validate` and `opt` read
`config_ini::global_layer()`, so a developer whose `config.ini` disables a
code sees tests fail that CI passes. Behaviour of the binary is unchanged.
Gates: `cargo test -p tcl-cli --test cli`, the crate clippy.

#### DP6.1 — One standalone producer run; the MCP diagnostics tools report the editor's set

`opus`, M, after DP4.1 and DP5.3.

Files: `rust/tcl-lsp-core/src/diagnostic_report.rs`,
`rust/tcl-cli/src/commands/diag.rs`, `rust/tcl-mcp/src/tools.rs`.

Core, `diagnostic_report.rs` — the producers a surface without the salsa
database runs itself, once, so `tcl diag`, the MCP tools and the truth
table's core pass run the same ones:

```rust
/// One document as a surface without the salsa database analyses it.
#[derive(Debug, Clone, Copy)]
pub struct StandaloneDocument<'a> {
    /// The analysis form of the text (lone `\r` rewritten).
    pub source: &'a str,
    /// The document's path, for the analyser's file-scoped facts.
    pub file_path: Option<&'a str>,
    /// The document's dialect.
    pub dialect: &'static DialectProfile,
    /// The registry the surface resolved for the dialect.
    pub registry: &'a CommandRegistry,
    /// `Analyser::with_pack_overlay`'s key.
    pub pack_overlay: u64,
    /// Cross-file call-site evidence, when the surface gathered any.
    pub external_call_sites: Option<&'a CallSiteEvidence>,
}

/// What [`standalone_findings`] produced.
#[derive(Debug)]
pub struct StandaloneFindings {
    /// The analysis, whose directive map feeds the policy.
    pub analysis: AnalysisResult,
    /// The analyser's findings, then the compiler checks', converted.
    pub produced: Vec<Finding>,
    /// The unit the analyser and the checks shared.
    pub unit: Arc<CompilationUnit>,
}

/// The analyser under `skip` — the policy's production skip — and the
/// compiler checks, over one compilation unit.
#[must_use]
pub fn standalone_findings(
    doc: &StandaloneDocument<'_>,
    skip: &BTreeSet<DiagCode>,
) -> StandaloneFindings;
```

Its body is `collect_rows`' analysed path moved down unchanged:
`document_declared_surface(source, file_path, dialect.name)`, one
`CompilationUnit::build_with_options` with `LexerConfig::for_profile(Some(dialect))`,
`external_call_sites` and the declared surface; `Analyser::with_disabled_diagnostics`
over `skip`, `with_file_path`, `with_pack_overlay`, `set_cu_override` on the
shared unit; `run_all_checks(&unit, registry, Some(dialect))`. It reads no
policy — rule 2 — and takes the skip as a producer input.

CLI, `collect_rows`: the analysed path becomes `standalone_findings` with
`pack_overlay: tcl_cli_support::spec_pack_key(dialect.name)`,
`registry_for_dialect(dialect.name)`, the document's path and the evidence
slice, then `document_report` and `declare_analyser_skip` as now.

MCP, `tools.rs`:

- `struct Analysed { source: String, dialect: &'static DialectProfile,
  analysis: AnalysisResult, produced: Vec<Finding>, policy: Policy }`.
- `analyse_under(source, dialect, inputs)` calls `registry(dialect)` first
  (it installs the bundled loadables the overlay key names, as `analyse`
  does), then `standalone_findings` with `source:
  &normalise_lone_cr(source)`, `file_path: None`, `dialect:
  crate::environment::profile_for_dialect(dialect)`, `registry:
  &registry(dialect)`, `pack_overlay: tcl_spectcl::bundled::packs().key`,
  `external_call_sites: None`, and the skip from
  `inputs.builder().dialect(profile).build().production_skip()`.
- `Analysed::report_with(&self, more: Vec<Finding>, optimiser: bool) ->
  Report`: `produced` then `more`; the policy is `self.policy` with
  `optimiser.enabled = optimiser`; the report is `document_report(&DocumentSource
  { text: &self.source, analysis_text: &normalise_lone_cr(&self.source),
  decode: None, dialect: self.dialect, pass: SourcePass::Tcl { line_length:
  DEFAULT_LINE_LENGTH } }, produced, &policy)` with `declare_analyser_skip`.
  `report()` is `report_with(Vec::new(), false)`: the four diagnostics tools
  run with the optimiser off, as `tcl diag`'s `diag_policy` does, so an
  O-code the checks emit is an `OptimiserOff` suppression, never a silently
  missing finding (§ Decisions taken, D5).
- `code_actions_with` drops its private unit build and `run_all_checks` call
  and calls `report_with(rewrites, true)`, `rewrites` being
  `optimise_with_dialect(source, &registry, Some(profile))` converted.
- The tool descriptions of `analyze`, `validate`, `review` and
  `find-legacy` add "including the compiler checks (S1xx, T1xx,
  IRULE1xxx–5xxx) and the source-style pass (W111, W112, W115, W118)".

Preserved: `diag_to_json`'s wire shape; `symbols`, `events`, `event_order`;
`optimize`; `code_actions`' action JSON; the plain `analyse` for the
non-diagnostic tools; `tcl diag`'s rows byte for byte.

Changes — #2061: "The MCP tools should report the set the editor publishes
for the same text and dialect: analyser findings plus `run_all_checks`,
minus directives, minus disabled codes, with the same overlap rules … and
the same default-off seeding." The four tools gain the compiler-check
families, the style codes and, for a `sslictcl` source, the loader's
findings; `review.taint` and the IRULE3xxx / IRULE4002 entries of
`security` / `thread_safety` fill; `validate` gains the `style` and
`performance` groups when present; `find-legacy` can report IRULE5001.
W305 is not doubled: the Tcl style pass carries W107 / W109, not W305.

Tests: the existing `find_legacy_tests`, `source_integrity_tests` and
DP4.0's `policy_tests` stay green. Gates: `cargo test -p tcl-mcp`, `cargo
test -p tcl-cli --test cli`, `cargo test -p tcl-lsp-core --lib --
diagnostic_report`, the crate clippy. Nothing regenerates:
`rust/tcl-mcp/diagnostics.json` is the code table's projection.

#### DP6.2 — #2061's cases on the MCP tools

`sonnet`, S, after DP6.1.

Files: `rust/tcl-mcp/src/tools.rs`, `mod policy_tests`. Every test uses the
`*_with` forms and DP4.0's `inputs(args, section, global_ini)` helper with
an empty global layer.

- `review_reports_the_compiler_check_families` — #2061's three programs:
  `set cmd [gets stdin]\neval $cmd\n` (`tcl9.0`) → `taint` holds T100;
  `when HTTP_REQUEST {\n  set host [HTTP::host]\n  HTTP::respond 200
  content "<h1>$host</h1>"\n}\n` (`f5-irules`) → `security` holds
  IRULE3001; `when RULE_INIT { set static::debug 0 }\n` (`f5-irules`) →
  `thread_safety` holds IRULE4002. Negative: `set cmd safe\neval $cmd\n`
  → `taint` is empty.
- `analyze_reports_the_source_style_pass`: `set x 1   \n` → W112; the same
  source under `disable: "W112"` → none.
- `analyze_reports_the_sslictcl_loader`: `sslictcl 1\nunknown-declaration
  {a b}\n` under `sslictcl` → SSLIC1101 and no W123.
- `a_check_emitted_rewrite_is_suppressed_not_missing`: `if {1} { set x 1 }
  else { set y 2 }\n` → no O100 in `diagnostics` (the diagnostics tools run
  with the optimiser off); DP9.3 adds the `suppressed` half.

Gates: `cargo test -p tcl-mcp`.

#### DP7.1 — A rewrite group is shown, offered and applied whole or not at all

`opus`, M, after DP4.0.

Files: `rust/tcl-lsp-core/src/diagnostic_policy.rs`,
`rust/tcl-lsp-core/src/diagnostic_report.rs`,
`rust/tcl-lsp-core/src/code_actions.rs`, `rust/tcl-lsp-server/src/lib.rs`
(`lift_report`).

Core, `diagnostic_policy.rs`:

```rust
/// A rewrite a surface may offer: one shown ungrouped rewrite, or every
/// member of an optimisation group all of whose members show.
#[derive(Debug, Clone)]
pub struct ApplicableRewrite<'a> {
    /// The group, for a grouped rewrite.
    pub group: Option<u32>,
    /// The members in the producers' order — one for an ungrouped rewrite.
    pub members: Vec<&'a Finding>,
}

impl Report {
    /// The rewrites a surface may offer or publish as an edit. Ungrouped: a
    /// shown `FindingData::Rewrite` that is not `hint_only` and has a
    /// non-empty replacement. Grouped: a group every member of which shows
    /// and none of which is `hint_only` (a member's replacement may be
    /// empty — a deletion). A group that lost a member to the policy — a
    /// directive on one member's line, a per-code toggle — is not
    /// applicable at all: its edits apply all-or-nothing.
    #[must_use]
    pub fn applicable_rewrites(&self) -> Vec<ApplicableRewrite<'_>>;

    /// `items`, one per finding in the producers' order, kept where the
    /// finding shows and, for a grouped rewrite, where its whole group
    /// shows — the rewrite loop's filter. Like [`Self::shown_items`], sound
    /// because `apply` is order-stable and keeps every finding.
    #[must_use]
    pub fn applicable_items<T>(&self, items: Vec<T>) -> Vec<T>;
}
```

A group's members are the report's findings carrying
`FindingData::Rewrite { group: Some(g), .. }`; because the report keeps
every finding, it knows each group's full size without a second list.

Consumers:

- `lift_report` (server): a shown rewrite's `data` is
  `{"replacement", "startOffset", "endOffset"}` when it is an applicable
  ungrouped rewrite; `{"group": g, "edits": [{"replacement", "startOffset",
  "endOffset"}, …]}` — every member's edit, on every member's diagnostic —
  when its group is applicable; and absent otherwise. This is `rust`'s
  #2150 payload (`grouped_quick_fix_payloads`) computed from the report.
- `code_actions.rs`: `rewrite_action(finding, …)` becomes
  `rewrite_actions(report: &Report, source: &str, range: LspRange,
  line_index: &LineIndex) -> Vec<CodeAction>` — one `QuickFix` per
  applicable rewrite any member of which overlaps `range`, titled with the
  first member's message, carrying every member's edit. It is called once
  from `code_actions_in_program`, outside the per-finding loop.
- `optimise_under_policy`: `report.shown_items(opts)` becomes
  `report.applicable_items(opts)`.

Preserved: every ungrouped rewrite's payload, action and application;
`hint_only` never publishes or offers; `apply_optimisations` still skips a
`hint_only` record, and a shown ungrouped `hint_only` record still reaches
`OptimisedSource::applied` exactly as today, so the `tcl opt` summary and
`samples/optimiser/*` are unchanged.

Changes — #2149 and #2123, closed on `rust` by PR #2150 ("a grouped member
must not be independently applicable"): an O127 pair publishes `{group,
edits}` on both members and never the flat triple (was: the inline member
alone carried the triple, offering the assignment's double evaluation); a
pair that lost a member publishes no payload, offers no action and is not
applied by `tcl opt`, MCP `optimize` or `optimiseDocument` (was: the other
member was applied — a store deleted while its read remains). § Adapters,
code actions: "A fix is offered for a shown finding and for no other."

Tests:

- core `apply_tests::a_group_that_lost_a_member_is_neither_offered_nor_applied`:
  two `Optimisation`s in group 7 (an inline with a replacement, a delete
  with an empty one) on lines 1 and 3 of a text, with `Directives::new`
  carrying an inline bucket for line 3;
  `applicable_rewrites()` is empty and `applicable_items(vec![0, 1])` is
  empty; without the directive, one applicable rewrite with two members.
- core `diagnostic_report::tests::optimise_under_policy_never_applies_half_a_group`:
  `proc p {y} {\n    set x [llength $y]\n    # noqa\n    puts $x\n}\n` under
  `tcl8.6` and `OptimiserPolicy::all_on()` — the output still contains `set
  x [llength $y]` and `applied` holds no O127.
- core `code_actions` test `a_grouped_rewrite_is_one_action_with_every_edit`:
  the same program without the directive, full range → exactly one action
  whose edits are the pair's.
- server `a_grouped_optimisation_is_never_independently_applicable_issue_2149`
  (`rust`'s name): `lift_report` over the report of `proc p {y} {\n    set x
  [llength $y]\n    puts $x\n}\n` (`tcl8.6`, O127 needs a resolved profile)
  gives both O127 diagnostics `data` with `group` and a two-element `edits`
  and no `replacement` key.
- server `optimise_document_command_never_applies_half_a_group_issue_2149`
  (`rust`'s name): the `# noqa` program under `aggressive` keeps the
  assignment.

Gates: *the suites* for core and server, the e2e subsets `code_actions`,
`commands`, `vscode_parity` (`test_hint_only_optimisation_diagnostic_carries_no_apply_payload`
stays green); the crate clippy. Docs: DP10.2 aligns
`diagnostics-calculation.md` § Grouped optimisations.

#### DP7.2 — The rewrite loop reads the analyser's directive map

`opus`, S, after DP7.1.

Files: `rust/tcl-lsp-core/src/diagnostic_report.rs`.

`optimise_under_policy` sets each pass's directives to
`Directives::from_analysis(&Analyser::new().analyse(&current, name),
&current)`, `name` being `dialect.map_or("", |d| d.name)`, instead of
`Directives::scan(&current, directive_dialect)`.

Why: `Directives::scan` attributes a `# noqa` to the next line only, while
the analyser — whose map the editor's squiggles are decided under —
attributes it to every line of the command it precedes
(`apply_preceding_noqa`). For a rewrite inside a multi-line command the
directive then meant one thing to the squiggle and another to `tcl opt`,
which is #2062's complaint ("The same directive meaning two different
things on two surfaces"). `rust`'s #2119 reads the same map per pass
(`optimise_document_command`'s admit closure). Rule 4: "a surface with no
analyser run scans once through the same helpers" — the rewrite loop can
run the analyser, so it does. `Directives::scan` keeps its two callers that
cannot: `f5_model_report` (not Tcl) and `collect_rows`' abstaining path
(the analyser never runs on those bytes).

Cost: one analysis per pass — at most five for `aggressive` and
`optimiseDocument`'s `full`, one otherwise — on the batch paths only.

Test: `diagnostic_report::tests::a_noqa_reaches_every_line_of_the_command_it_precedes`:
`# noqa: O101\nproc p {} {\n    return [expr {1 + 2}]\n}\nputs [p]\n`
under `tcl9.0` and `OptimiserPolicy::all_on()` — the output keeps `return
[expr {1 + 2}]` (the directive covers the whole `proc` command, lines 1 to
3); the unmarked control contains `return 3`. Gates: `cargo test -p tcl-lsp-core --lib --
diagnostic_report`, `cargo test -p tcl-cli --test cli -- opt`, `cargo test
-p tcl-mcp -- optimize`, the crate clippy.

#### DP7.3 — Code-action end-to-end tests; the last name of the old lifter

`sonnet`, S, after DP7.1.

Files: `rust/tcl-lsp-server/tests/e2e/code_actions.rs`,
`rust/tcl-lsp-core/src/code_actions.rs`, `rust/tcl-lsp-core/tests/code_actions_depth.rs`.

- `an_optimiser_rewrite_is_offered_as_a_quick_fix`:
  `Lsp::with_config(json!({"optimiser": {"profile": "full"}}))`, open `set x
  [expr {1 + 2}]\n` (nothing after it, DP4.0), request code actions over
  line 0 → an action of kind `quickfix` whose edit's `newText` is `set x 3`.
  Negative: under the default configuration (`readability`) no such
  action. § Adapters, code actions: "An
  `Outcome::Shown` finding carrying `FindingData::Rewrite` is a code action
  like any other."
- `no_quick_fix_for_a_finding_a_noqa_silences`: `set a 1\n# noqa:
  W100\nset y [expr $a + 1]\n` over line 2 offers no "Brace expr for safety
  and performance"; the control without the directive offers it. "A fix is
  offered for a shown finding and for no other."
- The three test-section comments naming `check_diagnostic_actions`
  (`code_actions.rs` lines 3217 and 3327, `code_actions_depth.rs` line 773)
  become "compiler-check fixes: …"; afterwards `grep -rn
  check_diagnostic_actions rust` is empty.

Gates: `cargo test -p tcl-lsp-server --test e2e -- code_actions`, `cargo
test -p tcl-lsp-core --test code_actions_depth`.

#### DP8.1 — A fact code is never skipped at production

`opus`, S, after DP4.1.

Files: `rust/tcl-lsp-core/src/diagnostic_policy.rs`.

```rust
/// Codes whose findings another producer reads as a fact — O111 reads
/// W100's sites — so the analyser computes them whatever the policy says;
/// the policy step still decides whether they show.
pub const FACT_CODES: &[DiagCode] = &[DiagCode::W100];
```

`Policy::production_skip()` (and so `analyser_skip()`) leaves out every code
in `FACT_CODES`. The doc cites § Failure modes' last bullet ("The
analyser's production-time skip is safe only because its findings are not
read as facts") and § Producers that change ("Both rules consume the same
unbraced-expression fact").

Behaviour: with W100 disabled at a layer, the analyser computes W100 and
the policy step suppresses it `Disabled(layer)` rather than the analyser
skipping it; no surface's shown set changes.

Test (`apply_tests`): `production_skip_never_skips_a_fact_code` — under an
editor layer `{"diagnostics": {"W100": false}}`, `production_skip()` lacks
W100 and `code_reason(DiagCode::W100)` is
`Some(Reason::Disabled(PolicyLayer::Editor))`. Gates: `cargo test -p
tcl-lsp-core --lib -- apply_tests`.

#### DP8.2 — O111 as a producer; one report call on every publish path

`opus`, M, after DP8.1 and DP6.1.

Files: `rust/tcl-lsp-core/src/diagnostic_report.rs`,
`rust/tcl-lsp-core/src/diagnostic_policy.rs` (`Report::extend`),
`rust/tcl-lsp-server/src/lib.rs`.

Core: `with_brace_expr_hints(report: Report, policy: &Policy) -> Report` is
replaced by

```rust
/// The O111 producer over the unbraced-expression fact: one finding at the
/// span of every W100 the analyser emitted, whatever policy later decides
/// for either.
#[must_use]
pub fn brace_expr_hints(produced: &[Finding]) -> Vec<Finding>;
```

— for each `f` in `produced` with `f.producer == Producer::Analyser &&
f.code == DiagCode::W100`, `Finding { code: DiagCode::O111, span: f.span,
severity: Severity::Info, message: BRACE_EXPR_HINT.to_owned(), fixes:
Vec::new(), data: None, producer: Producer::Optimiser }`. It reads no
policy. `standalone_findings` inserts `brace_expr_hints` of the analyser's
findings between them and the compiler checks', so `tcl diag`, the MCP
tools and the truth table's core pass all carry O111. `Report::extend`
loses its last caller and is deleted. The module doc's "transitional" O111
paragraph describes the producer.

Server: one function renders every report path —

```rust
/// One document's LSP publish set: `produced` plus the report's own
/// producers under `layers` and `directives`, the analyser's skip declared
/// when an analyser ran, lifted through the LSP adapter.
fn lifted_report(
    doc: &core_report::DocumentSource<'_>,
    produced: Vec<core_policy::Finding>,
    layers: &PolicyLayers,
    directives: core_policy::Directives,
    analysed: bool,
) -> Vec<tower_lsp_server::ls_types::Diagnostic>;
```

(`document_policy` + `document_report` + `declare_analyser_skip` when
`analysed` + `lift_report`). `publish_fast_tier`,
`refine_and_lift_diagnostics`, `analysed_diagnostics_for` and
`f5_model_report` each make one call on it — the page's "the three publish
paths become one call each on the same function". The deep push and the
pull build `produced` as the analyser's findings, then
`brace_expr_hints(&produced)`, then `compiler_findings`, then the XC
findings; `publish_fast_tier` adds no O111 (O111 stays deep-tier —
`large_file_publishes_fast_tier_before_deep_tier` uses it as the deep
marker, and tier membership is `is_fast_tier`'s scheduling, not policy).
The code-action handler joins this path in DP8.3.

Preserved: O111's message, severity (`Info`), span and producer; its
deep-tier placement; `tclLsp.optimiser.enabled = false` hides it
(`OptimiserOff`); a profile or `optimiser.O111 = false` hides it
(`OptimiserProfile`); its place right after the analyser's findings, where
`append_brace_expr_perf_hints` put it (the checkpoint appended it last).

Changes — § Producers that change: "policy decides both independently:
disabling W100 does not silence O111, and the optimiser gate reaches O111
in the one place it reaches every other O-code":

- `tclLsp.diagnostics.W100 = false`, `--disable W100` and `disable: "W100"`
  keep O111 (with DP8.1).
- `# noqa: W100` keeps O111; a bare `# noqa`, `# noqa: *` and `# noqa:
  O111` silence it.
- O111 reaches `tcl diag` and the MCP diagnostics tools as an
  `OptimiserOff` suppression, visible only under `--show-suppressed` and in
  `suppressed`.
- A `# tcl-lsp: disable=W100` still removes O111: the analyser folds the
  file directive into its own skip inside `tcl-compiler` (§ Open questions
  5).

Tests: core `diagnostic_report::tests` —
`the_brace_expr_hint_follows_every_w100_the_analyser_finds` replaces
`the_brace_expr_hint_follows_every_shown_w100`: spans equal to W100's;
under `{"diagnostics": {"W100": false}}` W100 is `Disabled(Editor)` and
O111 shows; under `# noqa: W100` W100 is `InlineDirective` and O111 shows;
under `optimiser.enabled = false` O111 is `OptimiserOff`. Server: keep
`o111_brace_expr_hint_pairs_with_w100`; add `o111_survives_a_disabled_w100`
(`apply_global_config(&json!({"diagnostics": {"W100": false}}))`, then
`full_diagnostics_for` carries O111 and no W100). Gates: `cargo test -p
tcl-lsp-core --lib -- diagnostic_report`, `cargo test -p tcl-lsp-server
--lib -- o111`, the e2e test `large_file_publishes_fast_tier_before_deep_tier`
and the subsets `diagnostics`, `sslictcl`, `bigip`; `cargo xtask
diag-emission-check` (O111's construction site stays under
`rust/tcl-lsp-core/src`). The code table is unchanged (O111's description
"paired with W100" still holds), so nothing regenerates. Docs: DP10.3 adds
the W100 / O111 sentence to the how-to.

#### DP8.3 — The lightbulb reads the published report; W115's conversion follows its finding

`sonnet`, S, after DP8.2 and DP7.1.

Files: `rust/tcl-lsp-server/src/lib.rs`, `rust/tcl-lsp-core/src/code_actions.rs`,
`rust/tcl-lsp-core/tests/code_actions_depth.rs`,
`rust/tcl-lsp-server/tests/e2e/code_actions.rs`.

Server: `lifted_report` splits into `published_report(doc, produced,
layers, directives, analysed) -> core_policy::Report` and `lift_report`,
and stays their composition. The pull path's assembly of `produced` — the
analyser's published findings, `brace_expr_hints`, `compiler_findings` of
`compiler_diagnostics_for`, `xc_findings` when the XC switch is on — moves
into one helper, `published_findings`, that `analysed_diagnostics_for` and
`code_action` both call. `code_action` reads `published_report` of it
under the pull path's `DocumentSource` (the client's text, the analysis
form, the decode report, `resolved_style_line_length`), and
`code_action_report` — DP4.0's extraction, which built its own report from
the analyser's published set and the uncached compiler checks alone — is
deleted. The lightbulb's report gains the style pass, the `SslicTcl`
projection, O111, the XC findings and the declared skip; none of them
carries a `fixes` entry, so no action appears or disappears on that
account, and every fix is decided against the set the editor shows.

Core, `code_actions.rs`: `continuation_comment_actions` takes the report
and offers "Convert to per-line comments" only where a shown W115 overlaps
`range`. It offered the conversion for every continued comment, whether or
not W115 showed — against § Adapters, code actions: "A fix is offered for
a shown finding and for no other." `code_actions_depth.rs`: `report_of`
builds its report with `diagnostic_report::document_report` over a
`SourcePass::Tcl` `DocumentSource`, so the continuation tests carry the
style pass's W115; a style finding carries no `fixes`, so no other test's
actions change.

Changes: a W115 turned off at any scope or silenced by a directive offers
no conversion (§ Adapters, code actions). Preserved: every other action.

Tests:

- core `code_actions` `a_conversion_follows_a_shown_w115`: over
  `# trailing \\\nset x 1\n`, line 0, the report of `document_report`
  under `Policy::unrestricted()` offers the conversion; under a policy
  whose editor layer is `{"diagnostics": {"W115": false}}` it does not;
  over `# noqa: W115\n# trailing \\\nset x 1\n`, line 1, with the
  directives scanned, it does not.
- server e2e `no_conversion_for_a_disabled_w115`:
  `Lsp::with_config(json!({"diagnostics": {"W115": false}}))`, open
  `# trailing \\\nset x 1\n`, code actions over line 0 → no action titled
  "Convert to per-line comments". `test_simple_continuation_fix` stays the
  positive.
- server `the_lightbulb_reads_the_published_report`: for
  `set x 1   \nputs $x\n` the shown codes of
  `published_report(published_findings(…))` equal the codes
  `full_diagnostics_for` publishes, W112 among them.

Gates: `cargo test -p tcl-lsp-core --lib -- code_actions`, `cargo test -p
tcl-lsp-core --test code_actions_depth`, `cargo test -p tcl-lsp-server
--lib -- lightbulb`, the e2e subset `code_actions`; the crate clippy.

#### DP9.1 — One spelling for every reason, and the report's gaps

`sonnet`, S, after DP4.1.

Files: `rust/tcl-lsp-core/src/diagnostic_policy.rs`.

- `impl core::fmt::Display for Reason` — the one spelling every adapter
  renders (CLI rows, MCP JSON) and the truth table pins:

  | `Reason` | Spelling |
  |---|---|
  | `ReportingOff` | `reporting-off` |
  | `Excluded` | `excluded` |
  | `EncodingAbstention` | `encoding-abstention` |
  | `InlineDirective { .. }` | `inline-directive` (the finding's own row carries its line) |
  | `FileDirective` | `file-directive` |
  | `Disabled(layer)` | `disabled:<layer>` |
  | `DefaultOff` | `default-off` |
  | `OptimiserOff` | `optimiser-off` |
  | `OptimiserProfile { profile }` | `optimiser-profile:<profile.name()>` |
  | `ShimmerOff` | `shimmer-off` |
  | `Overlap { owner: OverlapOwner::Code(c) }` | `overlap:<c>` (e.g. `overlap:W110`) |
  | `Overlap { owner: OverlapOwner::Producer(p) }` | `overlap:<p>` (e.g. `overlap:sslictcl`) |

- `PolicyLayer::as_str(self) -> &'static str` — `global`, `editor`,
  `invocation`, `project`.
- `Producer::as_str(self) -> &'static str` — `analyser`, `compiler-check`,
  `optimiser`, `source-style`, `source-decode`, `sslictcl`, `xc`,
  `bigip-model`.
- `Report::gaps(&self) -> impl Iterator<Item = (DiagCode, Reason)> + '_` —
  the declared skips whose code no finding in the report carries: the codes
  the policy turned off for this document that the report cannot show as
  findings. A declared code that some other producer did emit is not a gap;
  its findings carry their own reasons.

Tests (`tests` module): `every_reason_has_one_stable_spelling` (each row of
the table above, exactly); (`apply_tests`)
`a_gap_is_a_declared_skip_no_finding_explains` — declare W210 and T100 under
a Project disable of both, with one T100 finding in the report: `gaps()`
yields W210 alone. Gates: `cargo test -p tcl-lsp-core --lib`.

#### DP9.2 — `--show-suppressed` on `tcl diag` / `lint`

`sonnet`, M, after DP9.1, DP6.1 and DP8.2.

Files: `rust/tcl-cli/src/cli.rs`, `rust/tcl-cli/src/lib.rs`,
`rust/tcl-cli/src/commands/diag.rs`, `rust/tcl-cli/tests/cli.rs`.

- `cli.rs`: `#[derive(Debug, Args)] pub struct ReportArgs {
  #[arg(long = "show-suppressed")] pub show_suppressed: bool }`, documented
  "Also list what the policy hides — every suppressed finding with its
  reason, and every code a layer or a top-of-file directive turned off —
  so a missing diagnostic has an answer", flattened into `Diag` and `Lint`
  only (the page names `tcl diag` / `lint`; `validate` lists errors and
  takes no such flag). `lib.rs` dispatches `Command::Diag { input, diag,
  report } | Command::Lint { input, diag, report } =>
  commands::diag::run_diag(input, diag, report)`.
- `diag.rs`: `collect_rows` returns `DocumentRows { shown: Vec<Row>,
  hidden: Vec<HiddenRow> }`. `HiddenRow { line: Option<u32>, column:
  Option<u32>, severity: Option<Severity>, code: String, message:
  Option<String>, reason: String }` is built from `report.suppressed()`
  (position from the finding's span, the producer's own severity, the
  finding's message, `reason.to_string()`) and from `report.gaps()`
  without `Reason::DefaultOff` (no position, severity or message). Rows sort
  by `(line, column, code)` as today; gaps follow a file's positioned rows,
  sorted by code. `run_validate` reads `shown` only.
- A gap row renders only for a code no finding carries.
  `Policy::production_skip` declares every catalogued code the per-code
  decision turns off, whichever producer emits it, so a code the compiler
  checks emit — an S100 disabled at a layer — has both a
  `Suppressed(Disabled(…))` finding and a declared gap; rendering both
  would list it twice.
- Rendering with the flag, text: a suppressed finding is
  `{file}:{line}:{column}: {"hidden":<7} {code:<8} {message} [{reason}]`,
  interleaved with the shown rows in `(line, column, code)` order; a gap is
  `{file}: {"hidden":<7} {code:<8} [{reason}]`. "no diagnostics" prints only
  when no row of either kind printed. The stderr summary becomes
  `diagnostics={n} suppressed={m} across {k} input(s)` with the flag and is
  unchanged without it.
- Rendering with the flag, JSON: `FileReport` gains
  `#[serde(skip_serializing_if = "Option::is_none")] suppressed:
  Option<Vec<SuppressedItem>>`, `SuppressedItem { line: Option<u32>,
  column: Option<u32>, severity: Option<&'static str>, code: String,
  message: Option<String>, reason: String }` — a gap has `null` position,
  severity and message.
- The exit status counts shown problems only, with or without the flag.

Preserved: without the flag, every byte of text and JSON output, the stderr
line and the exit status.

Why: § Adapters, CLI rows — "`--show-suppressed` renders `suppressed()`
too, one row per finding with its reason, which is the CLI's answer to
'why is this not firing'". A gap row is the same answer for a code the
analyser was told not to compute (§ Producers that change: "the skip is
*declared* … so a report never shows a gap it cannot explain"). The
default-off seed is omitted: it is the catalogue's baseline, identical for
every file, and listing it on every file buries the answer (§ Decisions
taken, D21).

Tests (`tests/cli.rs`, through DP5.4's `tcl()`):

- `diag_show_suppressed_lists_every_hidden_finding_with_its_reason`: the
  `noqaSuppression.tcl` fixture with `--json --show-suppressed` → the
  `suppressed` array holds W210 on the `suppressedByCode` line and S100 on
  the `dictValue` line, each `"reason": "inline-directive"`; without the
  flag the JSON has no `suppressed` key and equals today's output.
- `diag_show_suppressed_lists_a_disabled_analyser_code_as_a_gap`: `--disable
  W210 --show-suppressed --json --source 'puts $y'` → one entry `{"line":
  null, "code": "W210", "reason": "disabled:invocation"}`, and no W242 entry
  (default-off gaps are omitted).
- `diag_show_suppressed_lists_o111_as_optimiser_off`: `set a 1\nset b [expr
  $a + 1]\n` → W100 in `diagnostics`; O111 in `suppressed` with
  `"optimiser-off"`.
- `diag_show_suppressed_text_rows_keep_the_exit_status`: the text form
  prints `hidden` rows with `[inline-directive]`, and a file whose only
  findings are suppressed exits 0.

Docs: the `diag` bullet of `kcs-feature-tcl-verb-cli.md` gains the flag
(DP10.3 folds the how-to). Gates: `cargo test -p tcl-cli`, the crate clippy.

#### DP9.3 — The MCP `suppressed` array

`sonnet`, M, after DP9.1 and DP6.1.

Files: `rust/tcl-mcp/src/tools.rs`.

- `fn suppressed_to_json(finding: &Finding, reason: Reason, sm:
  &SourceMap<'_>) -> Value` → `{"code", "range": byte_range(sm,
  finding.span), "reason": reason.to_string(), "message"}`; a gap →
  `{"code", "range": null, "reason", "message": null}`.
- `analyze`, `validate`, `review` and `find-legacy` payloads gain
  `"suppressed": [...]`: `report.suppressed()` then `report.gaps()` without
  `Reason::DefaultOff`, restricted to the codes the tool's shown set is
  drawn from — every code for `analyze` and `validate`, the
  security / taint / thread-safety sets for `review`,
  `tcl_cli::CONVERTIBLE_CODES` for `find-legacy`. The four tool
  descriptions add "and a `suppressed` array: every finding the policy
  hides, with its reason".

Preserved: every existing key and value of the four payloads.

Why: § Adapters, MCP JSON — "Each payload gains a `suppressed` array of
`{code, range, reason}`, so an agent can see that a finding exists and was
suppressed rather than concluding the code is clean". `message` is added
beside the page's three keys: without it a suppressed W210 does not say
which variable (§ Decisions taken, D22).

Tests (`policy_tests`): `the_diagnostics_tools_list_what_the_policy_hides` —
`# noqa: W210\nputs $y\n` → `analyze`'s `suppressed` holds `{code: W210,
reason: inline-directive}` whose range starts on line 1 (0-based);
`disable: "W210"` over `puts $y\n` → `{code: W210, range: null, reason:
"disabled:invocation"}`; `review` over `set x hello\n# noqa: S100\nincr
x\n` lists nothing (S100 is not a review code); DP6.2's
`a_check_emitted_rewrite_is_suppressed_not_missing` gains its half — O100
in `suppressed` with `optimiser-off`. Gates: `cargo test -p tcl-mcp`, the
crate clippy.

#### DP9.4 — The truth table and its core pass

`opus`, L, after DP5.1, DP6.1, DP7.1, DP8.2 and DP9.1.

Files: `rust/tcl-lsp-core/Cargo.toml`, `rust/tcl-lsp-core/src/diagnostic_policy.rs`,
new `rust/tcl-lsp-core/src/diagnostic_policy/truth_table.rs` (AGPL
header).

**Placement.** The page: "One table from `(program, policy)` to `Report`,
in `tcl-lsp-core` beside `apply` … The table lives once and every adapter
runs it." The adapters' tests live in three other crates, so the table is
compiled for them too: `tcl-lsp-core` gains the feature `truth-table = []`
(the precedent is `tcl-irules`' `test-instrumentation`, enabled only from a
dependant's `[dev-dependencies]`), and `diagnostic_policy.rs` declares
`#[cfg(any(test, feature = "truth-table"))] pub mod truth_table;`. DP9.5–9.7
enable the feature from their crates' `[dev-dependencies]`. No lockfile
changes; `--all-features` builds (CI's nextest shards, the crate clippy)
compile it.

**Shapes.**

```rust
/// What a row expects of one code at one line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Want {
    /// Shown, at whatever severity the producer chose.
    Shown,
    /// Shown at this severity.
    ShownAt(Severity),
    /// A finding, suppressed for this reason.
    Suppressed(Reason),
    /// No finding at all; the report explains the code with this reason.
    Gap(Reason),
}

/// One expectation. `line` is 1-based, as a reader counts; a `Gap` has none.
#[derive(Debug, Clone, Copy)]
pub struct Expect {
    pub code: DiagCode,
    pub line: Option<u32>,
    pub want: Want,
}

/// One `(program, policy)` row.
#[derive(Debug, Clone, Copy)]
pub struct Row {
    /// A stable `snake_case` name, used in every failure message.
    pub name: &'static str,
    pub dialect: &'static str,
    pub program: &'static str,
    /// Bytes decoded in place of `program`, for an abstaining document.
    pub bytes: Option<&'static [u8]>,
    /// The configuration by slot, each the `tclLsp` content shape as JSON
    /// text: the user's global file, the editor slot (the editor layer on
    /// the server, a surface's invocation layer elsewhere), the project
    /// file.
    pub global: Option<&'static str>,
    pub slot: Option<&'static str>,
    pub project: Option<&'static str>,
    /// The shown set is exactly the `Shown` / `ShownAt` expectations.
    pub exhaustive: bool,
    pub expect: &'static [Expect],
}

/// Where a row is rendered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Core,
    Lsp,
    LspActions,
    LspRewrite,
    Cli,
    CliRewrite,
    Mcp,
    McpActions,
    McpRewrite,
}

pub const ROWS: &[Row];

impl Row {
    /// Whether `surface` can realise this row's configuration and program.
    pub fn runs_on(&self, surface: Surface) -> bool;
    /// The expectations as `surface` renders them.
    pub fn expected(&self, surface: Surface) -> Vec<Expect>;
    /// A layer's JSON, parsed.
    pub fn layer(json: Option<&str>) -> serde_json::Value;
    /// A layer written as an INI file (`[diagnostics]` per-code keys and
    /// `exclude`, `[diagnosticSeverity]`, `[optimiser]`, `[shimmer]`,
    /// `[features]`), for the CLI's `config.ini` / `.tcl-lsp.ini`.
    pub fn ini(json: Option<&str>) -> String;
    /// The slot as `--disable` / `--enable` / `--profile` arguments, and as
    /// MCP `disable` / `enable` / `profile` values.
    pub fn slot_flags(&self) -> SlotFlags;
    /// The text a surface analyses: `program`, or `bytes` decoded by
    /// `source_decode::decode_source`, with the decode report.
    pub fn text(&self) -> (String, Option<DecodeReport>);
}

/// A row's slot as a surface's own arguments.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SlotFlags {
    /// `diagnostics.<CODE>: false` and `optimiser.<CODE>: false`.
    pub disable: Vec<String>,
    /// `diagnostics.<CODE>: true` and `optimiser.<CODE>: true`.
    pub enable: Vec<String>,
    /// `optimiser.profile`.
    pub profile: Option<String>,
}

/// One observed rendering of a code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observed {
    pub code: DiagCode,
    pub line: Option<u32>,
    pub state: ObservedState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservedState {
    Shown(Option<Severity>),
    Suppressed(String),
    Offered(bool),
    Applied(bool),
}

/// One offered code action, in a shape every surface can build.
#[derive(Debug, Clone)]
pub struct ActionView<'a> {
    pub title: &'a str,
    pub kind: &'a str,
    /// Each edit's 1-based start line and new text.
    pub edits: Vec<(u32, &'a str)>,
}

/// Whether `actions` offer the fix of the `code` finding at `line`.
pub fn offered(code: DiagCode, line: u32, actions: &[ActionView<'_>]) -> bool;

/// Compare `observed` with `row.expected(surface)`: for every code the
/// expectations name, the observed entries of that code equal the expected
/// ones as a multiset of `(line, state)`; codes the row does not name are
/// ignored unless `exhaustive`, which forbids any other shown code.
pub fn check(row: &Row, surface: Surface, observed: &[Observed]) -> Result<(), String>;
```

**Realisability (`runs_on`).** `Core` and `Lsp` run every row.
`LspActions` runs a row with an actionable subject (W100, S100 or O101 —
the brace refactor, the shimmer `# noqa` action, the fold quick-fix).
`LspRewrite` runs a row with an O101 subject. `Cli` runs a row whose layers
carry neither `features` nor `diagnostics.exclude` (the batch verbs read no
document gate, DP5.3) and whose slot holds only `diagnostics.<CODE>`
booleans. `Mcp` adds: no project layer (an MCP `source` has no path) and
no `bytes` (the tools take decoded text). `McpActions` is `Mcp` with an
actionable subject. `CliRewrite` and `McpRewrite` run a row with an O101
subject whose slot holds only `optimiser` keys, under the same document-gate
rule, and for `McpRewrite` the same project and bytes rules.

**The surface rules (`expected`).** The expectations are written for the
editor (`Core`), and each surface derives its own by rules that restate
the page:

- `Lsp`: `Shown` / `ShownAt` stay; `Suppressed` and `Gap` become "no
  published diagnostic of this code at this line" (the LSP adapter may
  publish only `shown()`).
- `Cli`, `Mcp`: `Disabled(Editor)` becomes `Disabled(Invocation)` (the
  flags occupy the editor layer's slot); an O-code expectation that is
  `Shown`, `ShownAt`, `Suppressed(OptimiserProfile { .. })` or
  `Suppressed(Overlap { .. })` becomes `Suppressed(OptimiserOff)` (the verb
  runs with the optimiser off — step 5's first gate — and steps 1 to 4 fire
  first, so their reasons stand); `Gap(DefaultOff)` becomes absent (not
  rendered, D21). `Cli` alone: `Suppressed(EncodingAbstention)` becomes
  absent (the CLI does not analyse an abstaining document; the integrity
  pass alone runs).
- `LspActions`, `McpActions`: each actionable subject becomes
  `Offered(want is Shown or ShownAt)`, after the `Mcp` layer rule for
  `McpActions` (code actions run with the optimiser on).
- `LspRewrite`, `CliRewrite`, `McpRewrite`: each O101 subject becomes
  `Applied(want is Shown)`.

A row's program is a means. If a producer does not emit a subject code on
it where the row says, the implementer changes the program or the line —
never the wanted reason; a reason that does not hold is a defect to report.

**The rows.** Programs by name: `TRAILING` = `set x 1   \nputs $y\n`;
`UNSET` = `puts $y\n`; `LOOP` = `while {$x < 10} {puts hi}\n`; `SHIMMER`
= `set x hello\nincr x\n`; `FOLD` = `set x [expr {1 + 2}]\n`; `UNBRACED` =
`set a 1\nset b [expr $a + 1]\n`; `STREQ` = `proc p {x} {\n    if {$x ==
"foo"} { return 1 }\n    return 0\n}\n`; `UNUSED` = `proc p {} {\n    set x
1\n    return 0\n}\n`; `TAINT` = `set u [HTTP::uri]\nHTTP::respond 200
content $u\n`; `SSLIC` = `sslictcl 1\nunknown-declaration {a b}\n`; `BOM`
= the bytes `FF FE` followed by `TRAILING`; `BOM_NOQA` = the bytes `FF FE
0A` followed by `# noqa\nset x 1   \n` (the newline puts the directive on
a line of its own, after the two replacement characters). The dialect is `tcl9.0` unless the row says. Layers
are JSON; `D` abbreviates `{"diagnostics": {…}}`, `O` `{"optimiser": {…}}`.
The surfaces column is what `runs_on` computes, listed so a reviewer can
check it.

| # | Name | Program | Layers | Expectations | Surfaces |
|---|---|---|---|---|---|
| 1 | `reporting_off` | `TRAILING` | global `{"features": {"diagnostics": false}}` | W112@1 `Suppressed(ReportingOff)`; W210@2 `Suppressed(ReportingOff)` | Core, Lsp |
| 2 | `excluded` | `TRAILING` | global `D {"exclude": ["*.tcl"]}` | W112@1 `Suppressed(Excluded)`; W210@2 `Suppressed(Excluded)` | Core, Lsp |
| 3 | `encoding_abstention` | `BOM` | — | W109@1 `Shown`; W112@1 `Suppressed(EncodingAbstention)`; W210@2 `Suppressed(EncodingAbstention)`; exhaustive | Core, Lsp, Cli |
| 4 | `abstention_beats_a_directive` | `BOM_NOQA` | — | W109@1 `Shown`; W112@3 `Suppressed(EncodingAbstention)` | Core, Lsp, Cli |
| 5 | `inline_noqa_named` | `# noqa: W210\nputs $y\n` | — | W210@2 `Suppressed(InlineDirective { line: 1 })` | Core, Lsp, Cli, Mcp |
| 6 | `inline_noqa_bare` | `# noqa\nset x 1   \nputs $y\n` | — | W112@2 `Suppressed(InlineDirective { line: 1 })`; W210@3 `Shown` | Core, Lsp, Cli, Mcp |
| 7 | `inline_noqa_star` | `# noqa: *\nputs $y\n` | — | W210@2 `Suppressed(InlineDirective { line: 1 })` | Core, Lsp, Cli, Mcp |
| 8 | `file_directive_named` | `# tcl-lsp: disable=W112\nset x 1   \nputs $y\n` | — | W112@2 `Suppressed(FileDirective)`; W210@3 `Shown` | Core, Lsp, Cli, Mcp |
| 9 | `file_directive_star` | `# tcl-lsp: disable=*\nset x 1   \nputs $y\n` | — | W112@2 `Suppressed(FileDirective)`; W210@3 `Suppressed(FileDirective)` | Core, Lsp, Cli, Mcp |
| 10 | `file_directive_on_an_analyser_code` | `# tcl-lsp: disable=W210\nputs $y\n` | — | W210 `Gap(FileDirective)` | Core, Lsp, Cli, Mcp |
| 11 | `inline_noqa_on_a_check` | `set x hello\n# noqa: S100\nincr x\n` | — | S100@3 `Suppressed(InlineDirective { line: 2 })` | Core, Lsp, LspActions, Cli, Mcp, McpActions |
| 12 | `an_unrelated_noqa_leaves_a_check` | `set x hello\n# noqa: W999\nincr x\n` | — | S100@3 `Shown` | Core, Lsp, LspActions, Cli, Mcp, McpActions |
| 13 | `a_whole_file_code_ignores_an_inline_noqa` | `# noqa\r\nset x 1\r\n` | — | W118@1 `Shown` | Core, Lsp, Cli, Mcp |
| 14 | `a_whole_file_code_honours_the_file_directive` | `# tcl-lsp: disable=W118\r\nset x 1\r\n` | — | W118@1 `Suppressed(FileDirective)` | Core, Lsp, Cli, Mcp |
| 15 | `disabled_at_global` | `TRAILING` | global `D {"W112": false}` | W112@1 `Suppressed(Disabled(Global))`; W210@2 `Shown` | Core, Lsp, Cli, Mcp |
| 16 | `disabled_in_the_slot` | `TRAILING` | slot `D {"W112": false}` | W112@1 `Suppressed(Disabled(Editor))` | Core, Lsp, Cli, Mcp |
| 17 | `disabled_at_project` | `TRAILING` | project `D {"W112": false}` | W112@1 `Suppressed(Disabled(Project))` | Core, Lsp, Cli |
| 18 | `a_disabled_analyser_code_is_a_gap` | `UNSET` | slot `D {"W210": false}` | W210 `Gap(Disabled(Editor))` | Core, Lsp, Cli, Mcp |
| 19 | `default_off` | `LOOP` | — | W242 `Gap(DefaultOff)` | Core, Lsp, Cli, Mcp |
| 20 | `default_off_turned_on_at_global` | `LOOP` | global `D {"W242": true}` | W242@1 `Shown` | Core, Lsp, Cli, Mcp |
| 21 | `default_off_turned_on_in_the_slot` | `LOOP` | slot `D {"W242": true}` | W242@1 `Shown` | Core, Lsp, Cli, Mcp |
| 22 | `default_off_turned_on_at_project` | `LOOP` | project `D {"W242": true}` | W242@1 `Shown` | Core, Lsp, Cli |
| 23 | `a_project_enable_over_a_global_disable` | `TRAILING` | global `D {"W112": false}`; project `D {"W112": true}` | W112@1 `Shown` | Core, Lsp, Cli |
| 24 | `an_inline_directive_over_a_project_enable` | `# noqa: W112\nset x 1   \n` | project `D {"W112": true}` | W112@2 `Suppressed(InlineDirective { line: 1 })` | Core, Lsp, Cli |
| 25 | `irules_taint_flow` (`f5-irules`) | `TAINT` | — | IRULE3001@2 `Shown` | Core, Lsp, Cli, Mcp |
| 26 | `a_check_disabled_in_the_slot` (`f5-irules`) | `TAINT` | slot `D {"IRULE3001": false}` | IRULE3001@2 `Suppressed(Disabled(Editor))` | Core, Lsp, Cli, Mcp |
| 27 | `a_severity_override_relabels_only_its_code` | `TRAILING` | global `{"diagnosticSeverity": {"W112": "error"}}` | W112@1 `ShownAt(Error)`; W210@2 `ShownAt(Warning)` | Core, Lsp, Cli, Mcp |
| 28 | `a_tagged_code_carries_its_tag` | `UNUSED` | — | W211@2 `Shown` (the LSP pass also checks `tags == [UNNECESSARY]`) | Core, Lsp, Cli, Mcp |
| 29 | `shimmer_off` | `SHIMMER` | global `{"shimmer": {"enabled": false}}` | S100@2 `Suppressed(ShimmerOff)` | Core, Lsp, LspActions, Cli, Mcp, McpActions |
| 30 | `a_rewrite_shows_under_its_profile` | `FOLD` | slot `O {"profile": "full"}` | O101@1 `Shown` | Core, Lsp, LspActions, LspRewrite, CliRewrite, McpRewrite |
| 31 | `a_rewrite_outside_the_profile` | `FOLD` | slot `O {"profile": "readability"}` | O101@1 `Suppressed(OptimiserProfile { profile: Readability })` | Core, Lsp, LspActions, LspRewrite, CliRewrite, McpRewrite |
| 32 | `the_optimiser_switch_reaches_a_rewrite` | `FOLD` | global `O {"enabled": false}`; slot `O {"profile": "full"}` | O101@1 `Suppressed(OptimiserOff)` | Core, Lsp, LspActions, LspRewrite, CliRewrite, McpRewrite |
| 33 | `a_per_code_toggle_reaches_a_rewrite` | `FOLD` | slot `O {"profile": "full", "O101": false}` | O101@1 `Suppressed(OptimiserProfile { profile: Full })` | Core, Lsp, LspActions, LspRewrite, CliRewrite, McpRewrite |
| 34 | `a_directive_over_the_profile` | `# noqa: O101\nset x [expr {1 + 2}]\n` | slot `O {"profile": "readability"}` | O101@2 `Suppressed(InlineDirective { line: 1 })` | Core, Lsp, LspActions, LspRewrite, CliRewrite, McpRewrite |
| 35 | `a_file_directive_reaches_a_rewrite` | `# tcl-lsp: disable=*\nset x [expr {1 + 2}]\n` | slot `O {"profile": "full"}` | O101@2 `Suppressed(FileDirective)` | Core, Lsp, LspActions, LspRewrite, CliRewrite, McpRewrite |
| 36 | `a_same_span_overlap` | `STREQ` | — | W110@2 `Shown`; O120@2 `Suppressed(Overlap { owner: Code(W110) })` | Core, Lsp, Cli, Mcp |
| 37 | `an_overlap_needs_a_standing_owner` | `STREQ` | slot `D {"W110": false}` | W110 `Gap(Disabled(Editor))`; O120@2 `Shown` | Core, Lsp, Cli, Mcp |
| 38 | `a_document_overlap_owned_by_a_producer` (`sslictcl`) | `SSLIC` | — | SSLIC1101@2 `Shown`; W123@2 `Suppressed(Overlap { owner: Producer(SslicTcl) })` | Core, Lsp, Cli, Mcp |
| 39 | `o111_survives_a_disabled_w100` | `UNBRACED` | slot `D {"W100": false}` | W100@2 `Suppressed(Disabled(Editor))`; O111@2 `Shown` | Core, Lsp, LspActions, Cli, Mcp, McpActions |
| 40 | `o111_survives_a_noqa_on_w100` | `set a 1\n# noqa: W100\nset b [expr $a + 1]\n` | — | W100@3 `Suppressed(InlineDirective { line: 2 })`; O111@3 `Shown` | Core, Lsp, LspActions, Cli, Mcp, McpActions |
| 41 | `the_optimiser_switch_reaches_o111` | `UNBRACED` | global `O {"enabled": false}` | W100@2 `Shown`; O111@2 `Suppressed(OptimiserOff)` | Core, Lsp, LspActions, Cli, Mcp, McpActions |

Coverage against § The truth table: every `Reason` (rows 1, 2, 3–4, 5–7,
8–10 and 14, 15–18 with `Invocation` through the `Cli` / `Mcp` rule, 19,
32 and 41, 31 and 33, 29, 36 and 38); the three precedence pairs (24, 23,
34); the `*` wildcard in both spellings (6 and 7, 9 and 35); the
`FILE_SUPPRESS_KEY` bucket (8, 10, 14); a default-off code turned back on at
each layer (20, 21, 22); both `OverlapScope` kinds (36, 38); an abstaining
document (3, 4). The four server unit tests the page names become rows 11
and 12 (`lift_compiler_diagnostics_honours_inline_noqa_suppression`), 32
and 33 (`…_honours_optimiser_master_switch_and_per_code`), 8
(`lift_source_style_diagnostics_honours_file_suppression`) and 27
(`apply_severity_overrides_relabels_only_listed_codes`).

**The core pass.** `pub fn core_report(row: &Row) -> (String, Report)`,
in `truth_table.rs`: the text and decode report from `row.text()`; the
profile `crate::profile_for_dialect(row.dialect)` and the registry
`tcl_registry::model::ingress::static_context_for(row.dialect).commands()`;
a `PolicyBuilder` with the global, slot (as `PolicyLayer::Editor`) and
project layers, `.decode(report)`, `.dialect(profile)` and `.excluded(true)`
when a layer carries `diagnostics.exclude` (glob matching is the server's
and `issue1556_diagnostics_exclude`'s); the production skip; then
`standalone_findings` over the analysis form, the optimiser's rewrites
(`optimise_with_dialect`) converted and appended — the server's producer
set — and `document_report` with `SourcePass::Tcl { line_length:
DEFAULT_LINE_LENGTH }` under the policy with the analysis's directives, and
`declare_analyser_skip`. Tests in its `#[cfg(test)] mod tests`:

- `every_row_holds_in_the_core_report` — for each row, the report's
  findings of each named code, as `Observed` (the line from
  `LineIndex::new_lsp`, `Shown` with the resolved severity or `Suppressed`
  with `reason.to_string()`), plus each named code's `Gap` through
  `report.gaps()`, pass `check(row, Surface::Core, …)`; every failure names
  the row.
- `every_reason_is_covered` — the set of reasons in `ROWS` (with the
  `Invocation` rule applied for `Cli`) is every `Reason` variant and every
  `PolicyLayer`, so a new variant cannot land without a row.

Also, in `diagnostic_policy.rs` `policy_tests`:
`directives_agree_with_line_suppressed` — for maps built from inline and
file buckets of `*`, `W210` and `W112`, and each of W210, W112 and W118 at
lines 0–3: `directives.reason_for(code, span).is_some()` equals
`tcl_compiler::analyser::line_suppressed(code.as_str(), line, map)` for a
code outside `WHOLE_FILE_CODES`, and `line_suppressed(code.as_str(),
FILE_SUPPRESS_KEY, map)` for one inside. `Directives::hit` restates the
owner's bucket rule; this test is what keeps the two equal (§ Decisions
taken, D24).

Gates: `cargo test -p tcl-lsp-core --lib`, `cargo test -p tcl-lsp-core
--lib --features truth-table`, the crate clippy.

#### DP9.5 — The LSP and code-action passes on the server

`opus`, M, after DP9.4, DP4.2 and DP8.3.

Files: `rust/tcl-lsp-server/Cargo.toml` (`[dev-dependencies]` `tcl-lsp-core
= { path = "../tcl-lsp-core", features = ["truth-table"] }`),
`rust/tcl-lsp-server/src/lib.rs`, new
`rust/tcl-lsp-server/src/policy_truth_table.rs` (AGPL header; declared
`#[cfg(test)] mod policy_truth_table;` in `lib.rs`).

- `lib.rs`: `async fn apply_session_layers(&self, layers: &PolicyLayers)`,
  extracted from `pull_and_apply_config_values` — it sets
  `Backend::policy_layers`, merges `global` + `editor` (`global_editor`) and
  then `project` (`merged`), and calls
  `apply_global_config_with_signature_fallback(&merged, &global_editor)`.
  The pull collapses the inlay alias per layer, calls it, and goes on to
  the folders as today. Behaviour-preserving.
- `policy_truth_table.rs`:
  - `every_row_publishes_its_shown_set` — for each row that `runs_on(Lsp)`:
    a `test_backend()`, `apply_session_layers` with the row's three layers
    (the slot as `editor`), the document registered at
    `file:///truth/<name>.tcl` with its text and, for a `bytes` row, its
    decode report on the `DocumentState`; then `full_diagnostics_for` (the
    pull path, which shares `lifted_report` with the push paths). Each
    published diagnostic becomes `Observed { code, line: range.start.line +
    1, state: Shown(severity) }`; `check(row, Surface::Lsp, …)`. Row 28
    also asserts `tags == [DiagnosticTag::UNNECESSARY]`.
  - `every_row_offers_fixes_for_shown_findings_only` — for each row that
    `runs_on(LspActions)`: `code_action` over the whole document with an
    empty context. `truth_table::offered(code: DiagCode, line: u32,
    actions: &[ActionView<'_>]) -> bool` — `ActionView { title: &str, kind:
    &str, edits: Vec<(u32, &str)> }`, each edit's 1-based start line and new
    text, built by each pass from its own action type — decides a subject: W100 — an action titled "Brace expr
    for safety and performance" whose edit starts on the subject's line;
    S100 — an action whose inserted text is a `# noqa: S100` comment above
    the subject's line; O101 — a `quickfix` whose edit's new text is `set
    x 3`. Each subject becomes `Offered(that answer)`.
  - `every_rewrite_row_applies_through_optimise_document` — for each row
    that `runs_on(LspRewrite)`: the slot's `profile` as the command's
    argument and its per-code keys in the editor layer;
    `Applied(result.source contains "set x 3")`.
- The four unit tests the page names are deleted, as rows 8, 11–12, 27 and
  32–33 now cover them: `the_report_honours_an_inline_noqa_on_a_compiler_check`,
  `the_report_honours_the_optimiser_master_switch_and_per_code_set`,
  `the_report_honours_a_file_directive_on_the_style_pass`,
  `the_report_relabels_a_code_the_editor_layer_overrides`. Their helpers
  `open_policy` and `lifted_compiler_set` stay while other tests use them.

Gates: `cargo test -p tcl-lsp-server --lib`, the e2e subset `config`, the
crate clippy.

#### DP9.6 — The CLI passes

`sonnet`, M, after DP9.4, DP9.2 and DP5.2.

Files: `rust/tcl-cli/Cargo.toml` (`[dev-dependencies]` `tcl-lsp-core` with
`truth-table`), `rust/tcl-cli/tests/cli.rs`.

- `truth_table_rows_render_through_tcl_diag` — for each row that
  `runs_on(Cli)`: a `Scratch` with `xdg/tcl-lsp/config.ini` from
  `Row::ini(global)`, `proj/.tcl-lsp.ini` from `Row::ini(project)` and
  `proj/<name>.tcl` from the program or the bytes; run `tcl diag --json
  --show-suppressed --dialect <dialect>` plus the slot's `--disable` /
  `--enable` on the file, with `XDG_CONFIG_HOME` at `xdg`. The
  `diagnostics` array becomes `Shown(severity)` observations and the
  `suppressed` array `Suppressed(reason)` ones (a `null` line is a gap);
  `check(row, Surface::Cli, …)`.
- `truth_table_rewrite_rows_render_through_tcl_opt` — for each row that
  `runs_on(CliRewrite)`: `tcl opt` with the slot's `--profile` and
  `--disable` / `--enable`; `Applied(stdout contains "set x 3")`.

One spawn per row; the pass is not in the smoke tier. Gates: `cargo test -p
tcl-cli --test cli -- truth_table`.

#### DP9.7 — The MCP passes

`sonnet`, M, after DP9.4 and DP9.3.

Files: `rust/tcl-mcp/Cargo.toml` (`[dev-dependencies]` `tcl-lsp-core` with
`truth-table`), `rust/tcl-mcp/src/tools.rs` (`policy_tests`).

- `every_row_renders_through_analyze` — for each row that `runs_on(Mcp)`:
  `analyze_with` with the slot's `disable` / `enable` arguments and a
  global layer `settings_from_ini(&Row::ini(global), Layer::Global)`;
  `diagnostics` become `Shown`, `suppressed` become `Suppressed` (a `null`
  range is a gap); lines are the 0-based `range.start.line` plus one.
- `every_row_offers_fixes_for_shown_findings_only` — `code_actions_with` for
  each row that `runs_on(McpActions)`, each action's JSON turned into an
  `ActionView` and judged by `truth_table::offered`.
- `every_rewrite_row_renders_through_optimize` — `optimize_with` for each
  row that `runs_on(McpRewrite)`, with the slot's `profile` / `disable` /
  `enable`; `Applied(optimized_source contains "set x 3")`.

Gates: `cargo test -p tcl-mcp`.

#### DP10.1 — The design page describes the built tree

`opus`, M, after DP9.7.

Files: `docs/design/compiler/diagnostic-policy.md`, `docs/design/README.md`.

- The status block reads *built*: the vocabulary as the tree spells it —
  `Finding`, `Producer`, `Fix`, `FindingData`, `Severity`, `Outcome`,
  `Shown`, `Reason`, `PolicyLayer`, `CodeDecision`, `Overlap`,
  `OverlapOwner`, `OverlapScope`, `OptimiserPolicy`, `DocumentGates`,
  `Directives`, `Policy`, `PolicyBuilder`, `Report`, `ApplicableRewrite`,
  `apply`, `FACT_CODES`, `WHOLE_FILE_CODES`; `diagnostic_report`'s
  `document_report`, `standalone_findings`, `brace_expr_hints`,
  `optimise_under_policy`; `config_ini`; the truth table; `--show-suppressed`;
  the `suppressed` array — and the module docs as the compilable form.
- § Today becomes "## Before the policy step (at `rust` `3b5eba8a`)", its
  table kept as the record of what changed, with one sentence saying so;
  § Where each step lives is rewritten for the built tree: server
  (`PolicyLayers`, `document_policy`, `lifted_report`, `published_report`,
  `published_findings`, `lift_report`, the
  conversions `analyser_findings` / `compiler_findings` / `xc_findings` /
  `model_findings` / `bigip_config_findings` / `apl_presentation_findings`,
  `apply_session_layers`, `resolved_policy_layers`), `tcl-lsp-db` (the
  declared production skip), CLI (`ConfigLayers`, `invocation_layer`,
  `collect_rows`, `rows_of`, `diag_policy`, `run_opt`), MCP
  (`PolicyInputs`, `analyse_under`, `Analysed`), core, and `tcl-compiler`
  (the directive facts; the analyser's fold and W305 self-filter as open).
  Both mentions of `Policy::from_disabled_set` go.
- § The finding, § The outcome, § The policy: the sketches follow the tree
  where it differs — `Reason::Overlap { owner: OverlapOwner }`, `Report` a
  struct (`shown`, `suppressed`, `gaps`, `outcome_for`, `reason_for`,
  `declare_skipped`, `declare_analyser_skip`, `applicable_rewrites`,
  `applicable_items`), `Policy::document: DocumentGates`,
  `Policy::production_skip` / `analyser_skip` / `gap_reason`.
- § Configuration: the INI per-code keys (DP5.1); the invocation layer's
  place inside the editor slot and above the editor layer, and
  `optimiseDocument`'s named argument in it (DP4.2); the batch verbs read no
  whole-document gate (DP5.3); the session and folder skips are the layers'
  production skip (DP4.1).
- § Adapters: `tcl opt` per input (#2120, DP5.2); rewrite groups whole
  (#2149, DP7.1); the rewrite loop's directives from the analyser (DP7.2);
  the reason spellings; `--show-suppressed`'s rows and gap rows, the
  default-off omission; the `suppressed` element with `message`.
- § Producers that change: O111 *built*, with `FACT_CODES`; the analyser's
  file-directive fold declared as a gap; the W305 self-filter recorded as
  open.
- § The truth table: the module path, the `truth-table` feature, the
  `Surface` rules and where each pass lives.
- § Slices: 4 to 10 marked *built*; slice 5's "when their policies differ"
  gains "(since #2120, always)".
- § Failure modes gains "A rewrite group applied or offered in part — the
  report's `applicable_rewrites` / `applicable_items` are the only doors."
- § Anchors: remove every name § The tree at 5bc40e95 lists as gone and
  `check_actions`, `check_diagnostic_actions`, `encoding_diagnostics`,
  `supersede_analyser_diagnostics`, `default_disabled_set`,
  `settings_disabled_diagnostics`, `settings_severity_overrides`,
  `with_brace_expr_hints`; move `apply_disabled_diagnostics` to
  `rust/tcl-compiler/src/analyser/diagnostics.rs`; add
  `rust/tcl-lsp-core/src/diagnostic_report.rs`,
  `rust/tcl-lsp-core/src/diagnostic_policy/truth_table.rs`,
  `rust/tcl-cli/src/commands/policy.rs`,
  `rust/tcl-lsp-server/src/policy_truth_table.rs` and the server names
  above.
- `docs/design/README.md` line 95: "**proposal** for one diagnostic policy
  owner" becomes "one diagnostic policy owner … (built)".

Gates: `cargo xtask kcs-index-links`; `grep -rn` for each name in
§ Transitional pieces over `docs/` returns only the lane documents.

#### DP10.2 — The owner documents point at the policy step

`sonnet`, M, after DP10.1.

Files and edits:

- `docs/design/compiler/diagnostics-integration.md` — rule 1 becomes
  "**Aggregation lives in the report.** Producers emit typed findings;
  `diagnostic_report::document_report` joins them with the report's own
  producers and `diagnostic_policy::apply` decides; an adapter renders
  (`diagnostic-policy.md`)." Rule 2 becomes "**Policy has one owner.**
  `# noqa`, `# tcl-lsp: disable=`, the five configuration scopes, the seed,
  severity, the optimiser and shimmer switches, overlaps and abstention are
  applied by `apply` alone, identically whatever the finding's origin or
  the surface." Rule 5 names the overlap table (`dialect_overlaps`: W110
  over O120 at the same span, the `SslicTcl` loader over W123
  document-wide) instead of `suppress_duplicate_o120`. § Failure modes'
  "A new code family added without a lift" becomes "… without a conversion
  to `Finding`". § Anchors: the server line becomes `lifted_report`,
  `lift_report`, `document_policy`; add `rust/tcl-lsp-core/src/diagnostic_policy.rs`
  and `diagnostic_report.rs`.
- `docs/design/compiler/diagnostics-calculation.md` — § Suppression: the
  analyser builds the map; the policy step applies it and every other
  step, for every producer on every surface; a disabled code the analyser
  skips is declared and explained. § Grouped optimisations: `rust`'s #2123
  text, with "a group that loses a member … its survivors are published as
  advice with no payload" tied to `Report::applicable_rewrites`.
- `docs/design/compiler/pass-fact-ownership-matrix.md` — the
  `rust/tcl-lsp-db/src/lib.rs` row reads "final LSP diagnostic projection"
  (", suppression policy" goes); a new row
  `rust/tcl-lsp-core/src/diagnostic_policy.rs`, `diagnostic_report.rs` |
  diagnostic policy: directives applied, the five scopes, the seed,
  severity, optimiser and shimmer gates, overlaps, abstention; every
  finding kept with its reason | the LSP adapter, `tcl diag` / `lint` /
  `validate` / `opt`, the MCP tools, the code actions | `apply`,
  `PolicyBuilder`, `document_report`.
- `docs/design/contracts/shared-utility-contracts-rust.md` — § `tcl-compiler`
  — diagnostic suppression directives: the consumer paragraph becomes "one
  consumer applies the map: the policy step (`Directives::reason_for`,
  whose bucket rule `directives_agree_with_line_suppressed` pins to
  `line_suppressed`); the analyser's own W305 producer still filters
  (open)". A new owner heading `### \`tcl-lsp-core\` — diagnostic policy`
  with bullets `apply`, `PolicyBuilder`, `Report`, `document_report`,
  `standalone_findings`, `optimise_under_policy`, `settings_from_ini`,
  `merge_settings`, and its manifest row: owner "diagnostic policy";
  sources `rust/tcl-lsp-core/src/diagnostic_policy.rs`;
  `rust/tcl-lsp-core/src/diagnostic_report.rs`;
  `rust/tcl-lsp-core/src/config_ini.rs`; entry points `apply`; `Policy`;
  `PolicyBuilder`; `Report`; `Finding`; `Directives`; `dialect_overlaps`;
  `document_report`; `standalone_findings`; `optimise_under_policy`;
  `settings_from_ini`; `merge_settings`; `global_layer`;
  `project_layer_for`; axis "the dialect's overlap table; the document's
  configuration layers, resolved per code; the step order is
  release-invariant"; drift gate `none`. `cargo xtask owner-resolution`
  then fails if any of those entries loses its public declaration.
- `docs/design/contracts/config-precedence.md` — § Precedence's
  implementation paragraph: `PolicyBuilder` in `tcl-lsp-core` resolves the
  three layers per code on every surface (the server, `tcl diag` / `lint` /
  `validate` / `opt`, the MCP tools); a surface's flags take the editor
  layer's slot, over the editor setting and under the project file; the
  server still applies the merged layers for every non-policy setting
  through `Backend::apply_global_config`'s path; an INI file can turn a
  code on (DP5.1).

Gates: `cargo xtask kcs-index-links`, `cargo xtask owner-resolution`.

#### DP10.3 — The user documents promise the five scopes on every surface

`sonnet`, M, after DP10.1.

Files and edits:

- `docs/kcs/kcs-howto-suppress-diagnostics.md` — § 5's example gains
  `W242 = true` beside DP5.1's § 3 line, and § 3 gains one sentence ("a
  per-code key turns one code on or off, and wins over `disabled` in the
  same file"); § Precedence, which the checkpoint already extended to
  `tcl diag` / `lint` / `validate` / `opt` and the MCP tools, gains one
  sentence: `[features]` and `[diagnostics] exclude` are the editor's
  alone (DP5.3); § How to tell it worked gains a bullet for the CLI and
  the MCP tools — `tcl diag --show-suppressed` lists each hidden finding
  as a `hidden` row with its reason and each code a layer turned off that
  no finding carries as a gap row, and the MCP diagnostics tools return
  the same in `suppressed` — with the reason spellings as a table naming
  the scope each one points at; § 1 gains "Silencing W100 does not
  silence O111 — name both codes."
- `docs/kcs/kcs-qa-where-is-diagnostic-policy-applied.md` — the paragraph
  beginning "Today the style pass and the SslicTcl projection are the
  producers that have stopped filtering" becomes the built state: every
  producer filters nothing, every surface reads one report, and
  `--show-suppressed` / `suppressed` show what it hides.
- `docs/kcs/features/kcs-feature-tcl-verb-cli.md` — `diag` gains
  `--show-suppressed`; `opt` per input (DP5.2 wrote it).
- `docs/kcs/features/kcs-feature-mcp-server.md` — the tool table notes
  `disable` / `enable` on the diagnostics tools, the compiler checks and
  style pass in their payloads, and the `suppressed` array.
- `docs/design/contracts/xdg-config.md` and
  `docs/kcs/kcs-qa-what-config-sections-are-valid.md` — the per-code keys
  of `[diagnostics]` and `[optimiser]` (DP5.1 wrote the table rows; the Q&A
  gains the matching bullet).
- `docs/GLOSSARY.md` — "Diagnostic report" names its two renderings of
  what is hidden (`--show-suppressed`, `suppressed`) and the gap: a code the
  policy turned off that no finding carries.

Gates: `cargo xtask kcs-index-links`.

#### DP10.4 — Reconcile with `rust` after the orchestrator merges it

`opus`, M, after the orchestrator's merge of `rust` into the branch, and
after DP4.2, DP5.2 and DP7.1.

Files: the conflict set in § Boundaries, `rust`;
`rust/tcl-lsp-core/src/diagnostic_report.rs`.

- The merge commit resolves every conflict by § Boundaries' table; this item
  reviews it and fixes what it left: `optimise_under_policy` becomes a thin
  wrapper over `tcl_compiler::optimiser::optimise_source_multipass_admitting`,
  its admit closure running the analyser for the pass's directives (DP7.2),
  the policy step, and `applicable_items` (DP7.1) — one multipass loop
  owner, in the optimiser.
- `rust`'s tests the lane's items ported by name (DP4.2's three
  `optimise_document_command_*_issue_2119`, DP7.1's two `*_issue_2149`,
  DP5.2's two `opt_*` tests) exist once each.
  `published_xc_codes_are_known_diag_codes_issue_2121` runs over
  `xc_findings`.
- Every suite, the whole `e2e`, the catalogue gates and `make rust-check`.

This item runs only when the orchestrator merges `rust`; until then
§ Boundaries' table is the instruction for that merge.

#### DP10.5 — Close the lane

`sonnet`, S, after every other item.

Per `docs/design/lanes/README.md`: the lane document's content is folded
into the final commit's message (goal, decisions, deltas, the retired
names, the open questions with their answers or assumptions), the file
`docs/design/lanes/diagnostic-policy.md` is removed, and its "In flight"
entry leaves `docs/design/lanes/README.md`. The final commit is not a `wip`
checkpoint: `diagnostics: one policy owner below every surface (#2089)`,
naming #2061, #2062 and #2063 as closed. Staged by path: the two lane files
only.

### Checkpoints

Every item commits on its own when it is green, as `wip(diagnostic-policy):
DPx.y — <what it does>`, staging its files by explicit path and updating
its row in § Progress in the same commit. A slice closes with a checkpoint
whose green is wider:

| Checkpoint | After | Green means |
|---|---|---|
| C4 | DP4.0–DP4.3 | *the suites* for the lane's crates, the whole `e2e`, the crate clippy, the catalogue gates, `make rust-check` when the workspace compiles; `cargo check -p tcl-lsp-db` and its clippy for DP4.1's documentation commit. |
| C5 | DP5.1–DP5.4 | `cargo test -p tcl-cli` (all targets) and `-p tcl-cli-support`, `cargo test -p tcl-lsp-core --lib -- config_ini`, `samples_optimiser_profiles_are_regenerated`, the crate clippy, `cargo xtask kcs-index-links`. |
| C6 | DP6.1–DP6.2 | `cargo test -p tcl-mcp`, `cargo test -p tcl-cli --test cli`, `cargo test -p tcl-lsp-core --lib`, the crate clippy. |
| C7 | DP7.1–DP7.3 | *the suites* for core and server, the e2e subsets `code_actions`, `commands`, `vscode_parity`, `config`, then the whole `e2e`; `cargo test -p tcl-cli --test cli`, `cargo test -p tcl-mcp`; the crate clippy. |
| C8 | DP8.1–DP8.3 | *the suites* for every lane crate, `large_file_publishes_fast_tier_before_deep_tier`, the whole `e2e`, `cargo xtask diag-emission-check`, the crate clippy. |
| C9a | DP9.1–DP9.3 | `cargo test -p tcl-lsp-core --lib`, `cargo test -p tcl-cli`, `cargo test -p tcl-mcp`; without the flag, `tcl diag`'s output is byte-identical to C8's on the CLI fixtures. |
| C9b | DP9.4–DP9.7 | every pass green: `cargo test -p tcl-lsp-core --lib --features truth-table`, `cargo test -p tcl-lsp-server --lib`, `cargo test -p tcl-cli --test cli -- truth_table`, `cargo test -p tcl-mcp`; the crate clippy with `--all-features`; the catalogue gates; `make rust-check`. |
| C10 | DP10.1–DP10.3 | `cargo xtask kcs-index-links`, `cargo xtask owner-resolution`; the retired-name grep. |
| C10.4 | DP10.4 | after the orchestrator's merge of `rust`: *the suites*, the whole `e2e`, the catalogue gates, `make rust-check`, `make prep-pr`. |
| Final | DP10.5 | C10.4's green; the lane files gone. |

A checkpoint that cannot reach its green because another lane holds the
workspace red commits on the crate check and the crate clippy, as the
hand-off did, and says so in § Progress; the wider gates run again at the
next checkpoint.

### Transitional pieces to retire

| Piece | Where | Retired by | Note |
|---|---|---|---|
| `Policy::from_disabled_set` | `rust/tcl-lsp-core/src/diagnostic_policy.rs` | DP4.0 (hand-off step 6) | four callers move to `PolicyBuilder`; its `policy_tests` test goes with it |
| `sslictcl_diagnostics::supersede_analyser_diagnostics` | `rust/tcl-lsp-core/src/sslictcl_diagnostics.rs` | DP4.0 | its manifest mention leaves `shared-utility-contracts-rust.md`; `SUPERSEDED_ANALYSER_CODES` stays |
| `InputDocument::encoding_diagnostics` | `rust/tcl-cli-support/src/input.rs` | DP4.0 | no caller in the workspace |
| `config_ini::default_disabled_set`, `config_ini::settings_disabled_diagnostics` | `rust/tcl-lsp-core/src/config_ini.rs` | DP4.1 | the server's skip is the layers' production skip |
| `config_ini::settings_severity_overrides` | `rust/tcl-lsp-core/src/config_ini.rs` | DP4.1 | callerless since slice 4; `parse_severity_value` stays |
| the server's `skipped_codes`, and the shimmer fold in `resolved_analysis_settings` | `rust/tcl-lsp-server/src/lib.rs` | DP4.1 | `Report::declare_analyser_skip` |
| `policy::share_one_project` (a free function since DP4.0) | `rust/tcl-cli/src/commands/policy.rs` | DP5.2 | #2120 |
| `rewrite_action` (one action per finding) | `rust/tcl-lsp-core/src/code_actions.rs` | DP7.1 | becomes `rewrite_actions` over `applicable_rewrites` |
| `Directives::scan` in `optimise_under_policy` | `rust/tcl-lsp-core/src/diagnostic_report.rs` | DP7.2 | `scan` stays for `f5_model_report` and the CLI's abstaining path |
| the three `check_diagnostic_actions` comments | `code_actions.rs`, `tests/code_actions_depth.rs` | DP7.3 | |
| `diagnostic_report::with_brace_expr_hints`, `Report::extend` | `diagnostic_report.rs`, `diagnostic_policy.rs` | DP8.2 | slice 8 |
| `code_action_report` (DP4.0's extraction, a second producer set for the lightbulb) | `rust/tcl-lsp-server/src/lib.rs` | DP8.3 | the lightbulb reads `published_report` |
| the four server unit tests the page names (in their checkpoint form `the_report_honours_an_inline_noqa_on_a_compiler_check`, `the_report_honours_the_optimiser_master_switch_and_per_code_set`, `the_report_honours_a_file_directive_on_the_style_pass`, `the_report_relabels_a_code_the_editor_layer_overrides`) | `rust/tcl-lsp-server/src/lib.rs` | DP9.5 | rows 8, 11–12, 27, 32–33 |
| `rust`'s `optimiser_policy_for_command`, `Backend::group_counts`, `grouped_quick_fix_payloads` and its `lift_compiler_diagnostics` hunks | `rust/tcl-lsp-server/src/lib.rs` after the merge | the merge, reviewed by DP10.4 | the report path replaces them |
| the lane document | `docs/design/lanes/diagnostic-policy.md` | DP10.5 | folded into the final commit |

`Policy::unrestricted` stays: its callers are test hosts and its doc says
so. `Report::shown_items` stays: `lsp_edit_workspace` and the style tests
read it.

### Boundaries

**The value-transfers lane.** It owns `rust/tcl-registry`,
`rust/tcl-compiler`, `rust/tcl-cmd-core`, `rust/tcl-spectcl`,
`rust/tcl-spec-hooks` and `rust/xtask/src/value_transfers*`; this lane
edits none of them. Shared ground:

- `rust/tcl-lsp-db/src/lib.rs`: DP4.1's documentation hunks against that
  lane's `FnLatticeKey` work (committed in `f3f9390f`) and its slice 4
  `spec_pack_key` → `compilation_unit` change (to come). Order: whichever
  commits second stages only its own hunks (DP4.1's procedure); neither
  rebases the other's hunks.
- `rust/tcl-compiler/src/analyser/`: three questions for that crate's
  owner, never edited here — the file-directive fold (§ Open questions 5),
  the W305 producer's self-filter (§ Open questions 5), and whether
  `line_suppressed`'s bucket rule should gain a single-bucket form the
  policy step can call (§ Decisions taken, D24).
- `rust/tcl-compiler/src/optimiser/manager.rs`: `rust`'s
  `optimise_source_multipass_admitting` arrives with the merge; DP10.4 calls
  it and does not change it.
- `docs/design/lanes/README.md`: both lanes edit "In flight"; each edits
  only its own bullet.
- The workspace is red while that lane's `tcl-compiler` edits are in
  flight: this lane then commits on the crate check and the crate clippy
  (`--no-deps`).

**The consumer-contracts lane.** Its CC3.3 edits
`apply_initialization_options`, `did_change_configuration`,
`spec_pack_discovery` and `reload_spec_packs` in
`rust/tcl-lsp-server/src/lib.rs`; its CC3.4 edits `spectcl.rs` and the
`spectcl_check` entry of `rust/tcl-mcp/src/tools.rs`. DP4.1 edits the
disabled-set lines of the first two functions and DP6.1 edits other
functions of `tools.rs`. Order (that lane's B3): this lane's adapter
commits land first — DP4.1 before CC3.3, DP6.1 before CC3.4 — and that lane
rebases. It adds no `DiagCode`, so no catalogue regenerates on its account.

**`rust`.** The orchestrator merges `rust` into the branch. The lane's
items already carry `rust`'s semantics (DP4.2 for #2119, DP5.2 for #2120,
DP7.1 for #2123 / #2149), so each conflict resolves as follows:

| File | `rust`'s change | Resolution |
|---|---|---|
| `rust/tcl-core-types/src/diag_code.rs` | #2121: `DiagSection::Xc`, the thirteen XC rows, `xc_family_is_in_the_code_table_issue_2121` | One copy of the rows (the two sides agree code for code; keep `rust`'s comment block); both sides' tests. |
| `rust/f5-xc/src/{diagnostics.rs,translator.rs,model.rs,report.rs,json_api.rs}`, `rust/f5-xc/tests/*` | #2121: `TranslationItem::diagnostic_code: DiagCode` | `rust`'s typed translator; the lane's `XcDiagnostic::span` and `From<XcDiagnostic> for Finding` stay; the lane's string-to-code step in `get_xc_diagnostics` goes where `rust`'s typing makes it redundant; `emitted_codes_are_catalogued` stays green. |
| xtask generators (`gen_ai.rs`, `gen_editor_settings.rs`, `gen_jetbrains.rs`, `gen_vscode_package.rs`, `diag_emission.rs`) and every generated catalogue | #2121 | Either side's `xc` arms (they match); then `make codegen` regenerates every generated file — never hand-merge a generated file. The JetBrains `TclLspSettings.kt` / `TclLspSettingsPanel.kt` are hand-written on `rust`: take `rust`'s. |
| `rust/tcl-cli/src/commands/transform.rs` | #2120 per-input `run_opt` | The lane's DP5.2 `run_opt`. |
| `rust/tcl-cli/tests/cli.rs` | #2120's two tests | One copy (DP5.2 added them verbatim). |
| `rust/tcl-compiler/src/optimiser/{manager.rs,mod.rs}` | `optimise_source_multipass_admitting` | `rust`'s. |
| `rust/tcl-lsp-core/src/source_decode.rs` | `should_abstain`'s doc | `rust`'s. |
| `rust/tcl-lsp-server/src/lib.rs` | `should_abstain` at five sites; `optimiser_policy_for_command`; `Backend::group_counts`; `optimise_document_command`'s body; `grouped_quick_fix_payloads`; `lift_compiler_diagnostics`' group changes; the `optimiser_enabled` field doc; tests `optimise_document_command_honours_the_optimiser_policy_issue_2119`, `…_profile_argument_selects_the_categories_issue_2119`, `…_honours_noqa_issue_2119`, `…_never_applies_half_a_group_issue_2149`, `a_grouped_optimisation_is_never_independently_applicable_issue_2149`, `published_xc_codes_are_known_diag_codes_issue_2121` | The lane's report path: DP4.2's `optimise_document_command`, DP7.1's `lift_report` payloads; `optimiser_policy_for_command`, `group_counts`, `grouped_quick_fix_payloads` and the `lift_compiler_diagnostics` hunks go (the function is gone); `should_abstain` stays at the two surviving sites (`run_diagnostics_f5_dialect`, `f5_pull_report`); `rust`'s field-doc sentence stays; each named test keeps the lane's body (DP4.2, DP7.1 ported them by name), and the XC test is ported onto `xc_findings`. |
| `docs/design/compiler/diagnostics-calculation.md` | #2123's § Grouped optimisations | `rust`'s section; DP10.2 aligns § Suppression beside it. |
| `docs/design/contracts/lsp-diagnostics-publication.md` | #2121's XC note | `rust`'s. |

### Decisions taken

The hand-off's decisions (§ Status at hand-off › Decisions the page does
not state), each ruled on:

- **D1. Commit order 7 → 4 → 5 → 6 — stands.** It shaped the checkpoint's
  history only.
- **D2. Folder policy layers are the folder's own three — stands.** With a
  real client the scoped configuration is a superset of the unscoped one, so
  the only change is the multi-root corner the hand-off names (§ Open
  questions 7); the analyser's skip for such a folder comes from the same
  layers, so skip and policy cannot disagree. DP4.3 pins both.
- **D3. `Reason::Disabled` names `Global` / `Editor` / `Project`
  truthfully, and the inline payloads merge into the editor layer until the
  next pull — stands.** DP4.1 makes the session's analyser skip follow the
  same merge.
- **D4. `tcl diag` keeps the optimiser off by policy — stands.** An O-code
  the checks emit is an `OptimiserOff` suppression, never dropped at
  production; that is what makes it explainable under `--show-suppressed`.
- **D5. The MCP diagnostics tools keep the optimiser off too (new, DP6.1).**
  They are `tcl diag`'s peers; `code_actions` and `optimize` keep it on.
- **D6. `tcl opt` and MCP `optimize` map `--disable` / `--enable` to
  `optimiser.<CODE>` — stands.** They override the profile exactly as the
  editor's `tclLsp.optimiser.<CODE>` does.
- **D7. The MCP tools gain `enable` beside `disable` — stands.** An MCP
  `source` has no project layer, so without it a default-off code could
  never be turned on from a call.
- **D8. `optimiseDocument` keeps its own pass rule — superseded by D36.**
  "The same path" is the loop, not the pass count. Its argument now follows
  #2119 (DP4.2). Since R1 the pass count is the profile in force's on every
  surface, so `optimiseDocument uri "full"` is one pass and `"aggressive"`
  runs to the fixpoint.
- **D9. The rewrite quick-fix is `QuickFix`, titled with the finding's
  message — stands.** A group is one action titled with its first member's
  message (DP7.1).
- **D10. `SourcePass::IntegrityOnly` serves an abstaining CLI document —
  stands.** W305 on a mis-decoded file is the editor's abstention survivor
  set (§ The policy, step 2: "everything but W107, W109 and W305"). DP4.3
  pins it end to end.
- **D11. The fast tier runs the `SslicTcl` projection — stands.** Nothing
  mandates it; it is workspace-independent, the deep tier stays a strict
  superset, and the loader's codes appear one publish earlier. Flagged in
  § Behavioural deltas.
- **D12. O111 appended after every other finding — does not stand.**
  DP8.2 puts it right after the analyser's findings, where
  `append_brace_expr_perf_hints` had it.
- **D13. `run_opt` folds inputs whose policies agree — does not stand.**
  #2120, closed on `rust`, requires every input optimised as its own
  program (DP5.2).
- **D14. The style-finding lift goes through the finding's byte span —
  stands.** § Adapters: "`Span` to a UTF-16 `Range` through `lift_span`";
  the one visible difference is W107's position in a lone-`\r` file.

This plan's own decisions:

- **D15. `Policy::production_skip` is step 4 alone, and every server path
  derives the analyser's skip from its layers (DP4.1).** A family-gated
  code is never skipped by any producer, so declaring it skipped would
  explain a gap that does not exist; one derivation for session and folder
  removes the second copy of the per-code tri-state
  (`settings_disabled_diagnostics`), which rule 1 forbids.
- **D16. The declared skip covers the analyser's own file-directive fold
  (DP4.1).** The fold lives in `tcl-compiler`, outside this lane; declaring
  it (`analyser_skip`, `gap_reason`) is how the report explains the gap
  without touching the producer.
- **D17. `tcl opt`'s pass count follows the profile in force (DP5.2) —
  stands; since D36 the profile in force is `--profile` when it is given.** The
  profile that decides the category set decides the passes, so a project
  `profile = aggressive` means what it says.
- **D18. An INI file spells the per-code tri-state as `CODE = bool`
  (DP5.1).** The editor's shape is per-code booleans and `[diagnosticSeverity]`
  already takes per-code keys; an `enabled = …` list would collide with
  `[optimiser] enabled`, the master switch.
- **D19. The batch verbs read no whole-document gate (DP5.3).**
  `[features]` configures the language server's features, and neither gate
  is in the page's rule 1.
- **D20. A configuration file's `[optimiser] enabled` and `profile` reach
  `tcl opt`, MCP `optimize` and `optimiseDocument` through the same path;
  a named profile is the invocation layer, above the editor layer and under
  the project file (DP4.2).** § Adapters: "The MCP `optimize` tool and the
  server's `tcl-lsp.optimiseDocument` command take the same path"; #2119
  makes `optimiseDocument` honour the switch; DP4.0's
  `optimize_honours_the_profile_the_overrides_and_the_global_file` pins the
  switch on MCP. `tcl opt --profile` has a default, so a global `profile`
  never reaches `tcl opt` — today's behaviour, kept. *The profile half is
  superseded by D36:* a named profile is not a layer and wins over the
  project file, and `--profile` has no default, so an omitted flag lets the
  project's, then the global file's, `profile` reach `tcl opt`. The switch
  half stands.
- **D21. `--show-suppressed` belongs to `diag` / `lint`, renders hidden
  findings as `hidden` rows and gaps as position-less rows, and omits
  default-off gaps (DP9.2).** `hidden` never matches a `grep ' error '`
  pipeline; the seed is identical for every file and would bury the answer.
- **D22. An MCP `suppressed` element carries `message` beside `{code,
  range, reason}` (DP9.3).** Without it a suppressed W210 does not say which
  variable.
- **D23. Every reason has one lower-case, hyphenated spelling with an
  optional `:detail` (DP9.1).** One rendering for the CLI, the MCP JSON and
  the truth table, stable enough to grep.
- **D24. `Directives::hit` keeps restating `line_suppressed`'s bucket rule,
  pinned equal by `directives_agree_with_line_suppressed` (DP9.4).**
  Calling the owner needs a single-bucket predicate in `tcl-compiler`,
  which this lane does not edit; the test is the guard until that crate's
  owner adds one.
- **D25. The standalone producer run lives in `tcl-lsp-core`
  (`standalone_findings`, DP6.1).** #2061's structural note asks for one
  function below the three surfaces; `tcl diag`, the MCP tools and the
  truth table's core pass then run one producer set.
- **D26. W100 is a fact code: never skipped at production (DP8.1). O111
  stays deep-tier on the server (DP8.2).** Tier membership is scheduling
  (`is_fast_tier`), not policy, and the tiering e2e test uses O111 as the
  deep marker.
- **D27. The rewrite loop reads the analyser's directive map (DP7.2).** It
  is the map the squiggles are decided under, and `rust`'s #2119 reads it
  too; `scan` stays only where no analyser runs.
- **D28. Group atomicity lives in the report (DP7.1).** The report keeps
  every finding, so it knows a group's full size; one door serves the
  payload, the action and the rewrite loop.
- **D29. The server keeps its whole-document short-circuit (no report when
  `features.diagnostics` is off or `diagnostics.exclude` matches).** The
  published set is identical, and analysing a document nobody sees costs
  real time; `ReportingOff` and `Excluded` are exercised in core (rows 1–2).
- **D30. The truth table is written for the editor and derived for every
  other surface by stated rules (DP9.4).** One set of hand-checked
  expectations; the rules are the page's slot, verb-gate and abstention
  sentences, small enough to review.
- **D31. No new KCS note (DP10.3).** `--show-suppressed` and `suppressed`
  are how a reader tells that a suppression worked and which scope did
  it, so they belong in the suppression how-to's § How to tell it worked;
  STYLE.md's "One note answers one question" keeps a second question out
  of that note, and this is not a second question.
- **D32. The policy step joins the `owner-resolution` manifest (DP10.2).**
  AGENTS.md: "never add an owner-shaped implementation without updating the
  contract and its gate".
- **D33. The `tcl-lsp-db` change is documentation only (DP4.1).** The skip
  stays as rule 2's saving; what changes is that it is declared.
- **D34. The lane's open uncertainty — whether to offer the thirteen XC
  toggles in the editor settings — is answered.** `rust`'s #2121 put the
  same toggles into the catalogues.
- **D35. A folder shares the session's analyser handle when its resolved
  inputs equal the session's (DP4.1).** DP4.0 records the double analysis
  as a decision for the lane: the checkpoint gave every configured folder
  a handle of its own. Sharing restores one analysis per revision for
  diagnostics and symbols alike, and changes no published diagnostic.
- **D36. R1 — the owner's ruling of 2026-09-22: an invocation profile
  wins.** "`--profile` should win when running from the CLI." Applied on
  every surface: a profile the invocation names — `tcl opt --profile`, the
  MCP `optimize` tool's `profile`, `optimiseDocument`'s argument — is the
  profile in force, and the project file's `[optimiser] profile` (then the
  global file's) applies only when the invocation names none. The profile
  is a request parameter with a project default, not a layered decision,
  which is #2150's semantics. Built as `PolicyBuilder::requested_profile`
  (over every layer) and `PolicyBuilder::default_profile` (the surface's own
  default, `full` for `tcl opt` and `optimize`); `--profile` lost its clap
  default and the MCP `profile` left `invocation_layer`. The master switch
  and the per-code `optimiser.<CODE>` decisions keep the layer order. The
  editor's `tclLsp.optimiser.profile` stays the editor layer's value, under
  the project file, for the published set and for an `optimiseDocument`
  call that names nothing: an editor echoes a setting's default for a key
  the user never set (`config-precedence.md`), so treating the setting as a
  named profile would make a project's profile unreachable there, and
  #2150's no-argument call resolves the configured profile the same way.
  The pass count is the profile in force's `max_iterations` on all three
  surfaces (D8 superseded). Tests: core
  `a_requested_profile_is_in_force_over_every_layer`; server
  `an_invocation_profile_overrules_the_project_file` (was
  `a_project_profile_overrules_the_command_argument`),
  `a_named_profile_leaves_the_switch_and_the_codes_to_the_layers`,
  `the_pass_count_follows_the_profile_in_force`; CLI
  `opt_a_named_profile_overrules_the_project_file`,
  `opt_runs_the_passes_of_the_profile_in_force`; MCP
  `optimize_a_named_profile_overrules_the_global_file`. Documents: the
  policy page's § Configuration, `config-precedence.md`,
  `kcs-feature-tcl-verb-cli.md`, `kcs-qa-how-tcl-lsp-loads-configuration.md`.

Decisions the implementation of slices 4–7 took (DP4.1 onwards):

- **D37. The server's shimmer-switch copies go with the fold (DP4.1).**
  `Backend::shimmer_enabled` and `FolderConfig::shimmer_enabled` had one
  reader, the shimmer fold in `resolved_analysis_settings`; with the fold
  gone they were written and never consumed, while the policy reads
  `shimmer.enabled` from the layers itself. They are deleted with the fold
  rather than kept as state that looks authoritative and decides nothing.
- **D38. `F5PullInputs::disabled` goes with `f5_model_report`'s parameter
  (DP4.1).** DP4.1 keeps it beside `LiftInputs::disabled` and
  `PullRefinementInputs::disabled` because the cross-file arity predicates
  read them; those two are read there, but the F5 pull's only reader was
  the removed parameter — no arity pass runs over a model document.
- **D39. `optimiseDocument` reads no whole-document gate either (DP5.3).**
  The command built its policy without `.reporting(true)`, so
  `tclLsp.features.diagnostics = false` at any layer suppressed every
  rewrite as `ReportingOff` and the command changed nothing. `rust`'s
  #2119 command never read that toggle, no expected delta lists the change,
  and § Adapters puts the command on `tcl opt`'s path, which D19 keeps off
  both gates: the toggle turns off published squiggles, not a rewrite the
  user asked for. Pinned by `optimise_document_command_reads_no_diagnostics_feature_toggle`.
- **D40. `Analysed::report_with(more, optimiser)` forces the optimiser off
  when `optimiser` is false and otherwise keeps the layers' switch
  (DP6.1).** DP6.1 writes "`optimiser.enabled = optimiser`"; read
  literally, `code_actions` (`true`) would switch the optimiser on over a
  global `[optimiser] enabled = false` and offer rewrites the user turned
  off, which changes `code_actions`' action JSON — the item preserves it —
  and departs from the editor, whose lightbulb decides under the document's
  policy. D5's "keep it on" is read as "do not force it off".
- **D41. The slices 4–7 landing names #2061, #2062 and #2063 as closed.**
  Each issue's own reproductions and asks are pinned by a test and hold on
  the built binaries (§ *Slices 4–7 landed*). § *Goal and exit per slice*
  also lists the truth-table passes (DP9.6, DP9.7) as slice 5's and 6's
  evidence; they gate parity between surfaces the issues already had
  fixed, so they are read as the slices' evidence, not the issues'.
- **D42. Only a missing configuration file is an absent layer (review
  fixes).** `config_ini::read_layer` read any failure as "no layer", so a
  directory or an unreadable file in a `.tcl-lsp.ini`'s or `config.ini`'s
  place changed every surface's verdict in silence. A missing file is
  still absent; any other failure is reported on stderr and contributes
  nothing. The project walk stops at the first `.tcl-lsp.ini` that exists,
  readable or not: the nearest project file governs, so a grandparent's
  must not decide in a broken one's stead. The CLI remembers an absent
  layer per root, so the warning prints once. DP9's report rows are the
  later, structured form of the same answer.
- **D43. The MCP rewrite tools read the analysis form (review fixes).**
  `optimize` and `code_actions` handed the optimiser and the code-action
  provider the raw source while the producers read the analysis form, so a
  lone-`\r` source was one command to the optimiser and one line to the
  ranges. Both now read the analysis form — `tcl opt`'s input and the
  editor lightbulb's — so `optimized_source` carries `\n` endings for such
  a source, as `tcl opt`'s output does, and `changed` says whether a
  rewrite applied rather than whether the endings differ.

Decisions slices 8 and 9 took:

- **D44. A disabled fact code is still reported disabled, and the bulk fix
  reads the report (DP8.1).** Computing a W100 a layer turns off changes
  two readers of the analyser's skip that the item does not name.
  `getEffectiveConfig`'s `disabled_diagnostics` and the INI export printed
  the skip, so they would have stopped listing a W100 the user turned off;
  the export now writes what the configuration turns off
  (`Policy::disabled_codes`), and `getEffectiveConfig` reports the skip in
  force plus the fact codes the layers turn off — the same list once a
  re-pull settles. It keeps reading the skip because a re-pull writes the
  layers first and the skip last, so a client that waits for a code to
  leave the list (the `e2e` barrier) knows the analyser computes it again;
  a fact code needs no such wait. `tcl-lsp.fixAllSafeIssues` took every
  bulk-applicable fix the analysis carried, so it would have braced the
  W100 the user turned off. It now decides each pass's findings under the
  document's policy and applies the fixes of the shown findings only —
  § Adapters, code actions: "A fix is offered for a shown finding and for
  no other" — and it analyses under the document's own layers' skip rather
  than the session's, so the skip and the policy come from one set of
  layers, as `PolicyLayers::production_skip` states.
- **D45. A row whose wanted outcome does not hold records a `Defect`
  (DP9.4).** The item says to change a row's program or line and never
  its wanted reason, and to report a reason that does not hold. A failing
  row cannot land, and deleting it would drop `SameSpan`'s only row. So
  `Defect { surfaces, today, note }` keeps the wanted reason, holds the
  named surfaces to what they render today (derived by the same rules),
  and makes `check` fail once the wanted outcome holds. The fix then
  removes the marker in the same change, and the defect cannot drift
  unnoticed meanwhile. A row can record several defects, one for each set
  of surfaces with its own `today`: row 36 recorded one on `Core` and `Lsp`
  until D46, and one on `Cli` and `Mcp`.
- **D46. W110 owns an O120 whose span holds it (the owner's ruling on
  § Open questions 10).** The overlap is real and both anchors are right:
  W110 belongs on the `==` operator, O120 on the whole condition it
  rewrites. So the relation is containment, not equality, and
  `OverlapScope::WithinSpan` states it. `SameSpan` stays for an entry that
  needs equal spans. Moving either producer's anchor to satisfy the policy
  was the other fix on offer, and it is rejected: "intentional overlap
  policy such as W110 / O120 precedence is explicit and separate from fact
  production" (`value-transfers.md` § Diagnostics consume facts, rule 4),
  and an anchor moved to suit presentation is fact production bending to
  it.

### Open questions for the owner

Each with the assumption the plan proceeds on.

1. **Does a configuration file's `[optimiser] enabled = false` or `profile`
   reach `tcl opt` and MCP `optimize`?** The page's § Today marks the
   switch "n/a" for them; its § Adapters says they "take the same path".
   Assumption: yes, the same path (D20; DP4.0 pins the switch on MCP).
2. **Does a project `[optimiser] profile` overrule a named profile — `tcl
   opt --profile`, MCP `profile`, `optimiseDocument`'s argument?** `rust`'s
   #2150 lets `optimiseDocument`'s argument win outright. Assumption: the
   project wins (rule 5's slot; DP4.2's `a_project_profile_overrules_the_command_argument`
   pins it, and flips with the answer). **Closed — ruled 2026-09-22: the
   named profile wins** ("`--profile` should win when running from the
   CLI"). It is a request parameter with a project default on all three
   surfaces, as #2150 has it; the pin flipped to
   `an_invocation_profile_overrules_the_project_file` (D36).
3. **Should the CLI and MCP resolve the producer inputs their layers carry
   — `[style] line_length`, `diagnostics.genericVariablePatterns`,
   `[style] nonAscii`?** § The policy keeps producer inputs off `Policy`;
   #2089's comment records IRULE4002's patterns as a producer-input
   divergence. Assumption: out of this lane; a follow-up issue.
4. **Should `tcl diag` / `lint` and the MCP tools honour `[features]
   diagnostics = false` and `[diagnostics] exclude`?** Assumption: no
   (D19, DP5.3).
5. **The analyser's file-directive fold and its W305 self-filter** (rule 2:
   "Producers emit findings and read no policy"). The fold removes a code
   at production (so `# tcl-lsp: disable=W100` also removes O111, against
   § Producers that change's "disabling W100 does not silence O111"); the
   self-filter drops a W305 under a `# noqa` with no reason in the report.
   Both are in `tcl-compiler`. Assumption: left in place; the fold is
   declared as a gap (D16); a follow-up for that crate's owner.
6. **The `xcDiagnostics` switch** is a production skip with no `Reason`: an
   `f5-irules` document with the switch off has no XC finding and no
   explanation. Assumption: left; the XC family stays outside the report
   while its switch is off.
7. **The multi-root corner** (D2): a secondary root with no policy section
   of its own no longer inherits the primary root's `.tcl-lsp.ini` sections.
   Assumption: stands; DP4.3's
   `a_secondary_root_does_not_inherit_the_primary_project_file` pins it and
   flips with the answer.
8. **`--show-suppressed` on `validate`.** Assumption: not offered (D21).
9. **Should the server build a report when reporting is off, so an editor
   can say "diagnostics are turned off for this file"?** The page's § The
   outcome motivates `ReportingOff` with that sentence. Assumption: no
   (D29); the published set is the same.
10. **W110 never owns O120 (DP9.4).** The overlap entry is `SameSpan`, as
    the page's `OverlapScope::SameSpan` defines it, but the two spans never
    coincide on a real program. The analyser anchors W110 on the operator
    (`if {$x == "foo"}` → `==`). O120 comes only from `branch_folding`'s
    rewrite of a branch condition, and spans the whole condition word. So
    the editor shows both codes for one comparison, as it did before the
    lane: `suppress_duplicate_o120` compared equal ranges. The fix could
    anchor O120 on the operator (a `tcl-compiler` change), let the owner's
    span lie within the superseded finding's span (a new `OverlapScope`),
    or anchor W110 on the condition. Assumption: left as it is; row 36
    records the defect (D45). **Closed — ruled: containment.** Both
    anchors are right, and `OverlapScope::WithinSpan` owns an O120 whose
    span holds W110's (D46).
11. **The diagnostics verbs and tools do not run the optimiser (DP9.4).**
    `tcl diag` / `lint` / `validate` and the MCP diagnostics tools run the
    analyser, the O111 producer and the compiler checks. A rewrite only the
    optimiser emits, such as O120 or O101, therefore has no finding there
    and no declared gap. The page's rule 2 wants a production skip declared,
    and the truth table's rule wants such a code shown as an `OptimiserOff`
    suppression. There are three fixes. `standalone_findings` could run the
    optimiser, so every rewrite is decided like O100 and O111; that costs one
    optimiser run per document and changes `--show-suppressed` and
    `suppressed`. The verbs could declare the optimiser's codes as a gap,
    which buries the answer as the default-off seed would (D21). Or the
    table's rule could be narrowed to the codes a check emits. Assumption:
    left as it is; rows 36 and 37 record it on `Cli` and `Mcp` (D45).

### Review checklist per slice

Every item, every slice:

- **One policy owner below every surface.** No surface filters a finding,
  re-derives a severity, or re-checks a disabled set, a directive, the
  optimiser switch or profile, the shimmer switch or the overlap table
  outside `apply`. Grep the diff for `line_suppressed(`, `.retain(`,
  `.filter(` over findings or diagnostics, `is_optimisation()` and
  `disabled.contains` outside `diagnostic_policy.rs`; each hit is a
  producer's own calculation (rule 2's skip, the arity predicates) or a
  defect.
- **Producers read no policy.** New producer code — `brace_expr_hints`,
  `standalone_findings` — takes no `Policy`; `standalone_findings` takes
  the skip set as an input, the way the analyser does.
- **A suppressed finding stays in the report with its reason.** Nothing is
  removed between a producer and `apply`; adapters read `shown()`,
  `suppressed()`, `gaps()`, `applicable_rewrites()` and
  `applicable_items()` only.
- **No new `#[allow]`**; the crate clippy is pedantic-clean; a long
  function is split.
- **UK spelling** in identifiers and comments (optimise, behaviour,
  catalogue, recognise, normalise, licence); the MCP tool names `analyze`
  and `optimize` are identifiers and stay as spelled.
- **The AGPL header** on every new source file (`truth_table.rs`,
  `policy_truth_table.rs`), and on no fixture or generated file.
- **The page's identifiers verbatim**: `Finding`, `Producer`, `Fix`,
  `FindingData`, `Outcome`, `Reason`, `PolicyLayer`, `CodeDecision`,
  `Overlap`, `OverlapOwner`, `OverlapScope`, `OptimiserPolicy`,
  `Directives`, `Policy`, `PolicyBuilder`, `Report`, `apply`,
  `--show-suppressed`, `suppressed`.
- **Parity suites green**: the e2e subsets, `tcl-cli --test cli`,
  `tcl-mcp`, the core `--lib` and integration suites.
- **Staging by explicit path**; the message starts `wip(diagnostic-policy):`;
  § Progress is updated in the same commit; no other lane's file is staged.
- **Every deleted public name** is gone from `docs/` (outside the lane
  documents) and from the `owner-resolution` manifest in the same commit.

Slice-specific:

- **Slice 4.** The publish paths call `lifted_report` once each (after
  DP8.2) and nothing else decides; `Backend::disabled_diagnostics` has no
  writer that bypasses the layers; `f5_model_report` declares no analyser
  skip; `optimiseDocument`'s named profile is the request's own, over
  every layer, and its passes are that profile's (D36). Risks: the salsa
  `AnalyserConfig` is keyed on the skip, so a skip that changes on every
  configuration read invalidates every file's memo — compare the sorted
  sets before writing; the folder handles' `spec_pack_key` (DP4.0's
  `sync_db_config` fix); a folder whose analyser inputs equal the
  session's holding a handle of its own (D35).
- **Slice 5.** Every `tcl` spawn in `tests/cli.rs` goes through `tcl()`; a
  single-input `tcl opt` is byte-identical to its pre-DP5.2 output except
  for the untrimmed tail; an INI per-code key parses case-insensitively and
  never shadows `disabled`, `exclude`, `generic_variable_patterns`,
  `enabled` or `profile`. Risks: `samples/optimiser/*` drifting (the
  summary format must not change); a lone-`\r` input's endings.
- **Slice 6.** The four diagnostics tools and `tcl diag` call the same
  `standalone_findings`; the tools run with the optimiser off and
  `code_actions` with it on; no W305 is doubled. Risks: the bundled-pack
  overlay key and registry order (`registry(dialect)` before the analyser);
  the payload growing categories an agent did not expect (#2061 mandates
  it).
- **Slice 7.** No path offers, publishes or applies part of a group; a
  `hint_only` rewrite is never an edit; `tcl opt`'s summary still lists what
  it listed. Risks: `applicable_items` and `shown_items` confused; the cost
  of an analysis per pass on large files.
- **Slice 8.** O111 never reaches the fast tier; W100 is computed when
  disabled; `Report::extend` is gone; `diag-emission-check` finds O111's
  construction site; the lightbulb and the pull build their report through
  one helper, and no code-keyed quick-fix is offered without its shown
  finding. Risks: the tiering e2e test's timing; the lightbulb's latency
  with the style pass added.
- **Slice 9.** Without `--show-suppressed` the CLI's bytes are unchanged;
  the reason spellings match D23's table exactly; each row's expectations
  are hand-checked, never generated from the core report; `runs_on`
  computes the surfaces column. Risks: a row whose program no longer draws
  its subject after a producer change (fix the program, never the reason);
  the CLI pass's run time.
- **Slice 10.** Every sentence about the built tree names real items; the
  retired-name grep is clean; `owner-resolution` passes with the new row;
  KCS notes keep STYLE.md's fourteen rules. Risk: a document promising
  what an open question left undecided.

### Behavioural deltas expected per slice

Each change cites what mandates it; an entry marked **flagged** is a
hand-off delta nothing mandates, for the owner.

**Slice 4 — server.**

- `tcl-lsp.optimiseDocument` honours the optimiser switch, the profile's
  set, per-code overrides and the directives — slice 4 ("joins the same
  path") and #2119.
- An absent `optimiseDocument` argument uses the configured profile (was
  `full`), and so does an unrecognised one (was `readability`) — #2119 as
  merged on `rust`.
- A code action offers an optimiser rewrite as a quick-fix — § Adapters,
  code actions.
- W107's position in a lone-`\r` file is the U+FFFD's on the client's line
  model — § Adapters, LSP (D14). The pass positions it through
  `LineIndex::new_lsp` since the review fixes; before them it counted `\n`
  alone and the conversion clamped it to the end of line 0.
- `getEffectiveConfig`'s `disabled_diagnostics` lists catalogued codes only
  — § The conversions (DP4.1).
- **Flagged:** a secondary workspace root with no policy section of its own
  no longer inherits the primary root's `.tcl-lsp.ini` sections (D2, § Open
  questions 7).
- **Flagged:** the fast tier publishes the `SslicTcl` loader's codes one
  publish earlier (D11).

**Slice 5 — CLI.**

- W242 is off unless a layer turns it on — #2063, § Configuration
  ("`DEFAULT_OFF_CODES` … becomes the seed of every surface's
  resolution").
- The global `config.ini` and each file's own `.tcl-lsp.ini` apply to
  `diag` / `lint` / `validate` / `opt` — #2063; the shimmer switch and
  severity overrides from those files reach `tcl diag` — rule 1.
- W305 appears on an abstaining document — § The policy, step 2 (D10;
  DP4.3 pins it).
- `tcl opt` applies only the rewrites the policy shows — #2062; optimises
  each input separately — #2120; leaves a single input's text untrimmed and
  writes `\n` for a lone `\r` — #2120 as merged; lists each file's rewrites
  under `# file:` — #2120 as merged; runs the profile in force's passes —
  D17.
- An INI file's `CODE = true` turns a code on — § The five scopes (DP5.1).
- A `config.ini` or `.tcl-lsp.ini` the server exported (`render_config_ini`
  writes one `CODE = false` line per disabled code under `[diagnostics]`)
  now disables those codes on every surface: no parser read those lines
  before `insert_code_toggles` (DP5.1).
- A configuration file that exists but cannot be read is reported on
  stderr instead of silently contributing nothing, and an unreadable
  `.tcl-lsp.ini` ends the project-file walk (review fixes, D42).
- **Flagged:** a configuration file's `[optimiser] enabled = false` stops
  `tcl opt` rewriting (§ Open questions 1; kept). A project `profile`
  overruling `--profile` (§ Open questions 2) is reversed by the owner's
  ruling: `--profile` wins, and a project's, then the global file's,
  `profile` applies only when it is omitted (D36).
- **Flagged, reverted by DP5.3:** at the checkpoint a configuration file's
  `[features] diagnostics = false` silenced `tcl diag`.
- **Reverted by DP5.2:** the checkpoint's "fold several inputs into one text
  when their policies agree" (#2120).

**Slice 6 — MCP.**

- `analyze` / `validate` / `review` / `find-legacy`: the inline `# noqa` is
  honoured, W242 is seeded off, the global `config.ini` and `disable` /
  `enable` apply, the reported severity is the resolved one, and the
  compiler checks, the style pass and the `SslicTcl` loader report — #2061
  (and #2063 for the seed).
- `review.taint`, `security`'s IRULE3xxx and `thread_safety`'s IRULE4002
  fill; `validate` gains `style` / `performance` groups; `find-legacy` can
  report IRULE5001 — #2061.
- `code_actions` offers compiler-check fixes and optimiser rewrites and
  nothing for a silenced finding — #2061.
- `optimize` honours directives, the global file and per-code overrides —
  #2062, § Configuration.
- `optimize` and `code_actions` read a lone-`\r` source in its analysis
  form, as `tcl opt` and the editor do: `optimized_source` carries `\n`
  endings and every range is on the client's line model (review fixes,
  D43).
- **Flagged:** a global `[optimiser] enabled = false` stops `optimize`
  rewriting (§ Open questions 1; DP4.0's test pins it).
- **Flagged, reverted by DP5.3:** a global `[features] diagnostics = false`
  silenced the diagnostics tools.

**Slice 7 — code actions.**

- An O127 pair publishes `{group, edits}` on both members and never the
  flat triple; a pair that lost a member publishes no payload, offers no
  action, and is applied by no rewrite surface — #2149, #2123.
- A `# noqa` before a multi-line command keeps every rewrite inside it off
  on `tcl opt`, MCP `optimize` and `optimiseDocument`, as it does the
  squiggles — #2062 (D27).

**Slice 8 — O111.**

- `tcl-lsp.fixAllSafeIssues` applies no fix for a finding the report
  suppresses, so a `# noqa` over the command and an abstaining document
  now keep their fixes unapplied; a code a layer turns off stays
  unapplied, W100 included now that it is computed — § Adapters, code
  actions (DP8.1, D44).
- In a multi-root workspace `tcl-lsp.fixAllSafeIssues` analyses under the
  document's folder's layers, so a code a folder turns back on gets its
  bulk fixes there (D44).
- Disabling W100, or a `# noqa: W100`, keeps O111 — § Producers that
  change.
- O111 sits right after the analyser's findings again (D12).
- `tcl diag` and the MCP diagnostics tools carry O111 as an `OptimiserOff`
  suppression, visible only through `--show-suppressed` / `suppressed`.
- Unchanged, against the page: `# tcl-lsp: disable=W100` still removes O111
  (§ Open questions 5).
- A W115 turned off at any scope or silenced by a directive no longer
  offers "Convert to per-line comments" — § Adapters, code actions ("A fix
  is offered for a shown finding and for no other"; DP8.3).

**Slice 9.**

- `tcl diag` / `lint` gain `--show-suppressed`; the four MCP diagnostics
  payloads gain `suppressed` — the page's slice 9.
- No output changes without the flag.
- The editor no longer shows O120 beside the W110 it sits over (`if {$x ==
  "foo"}`), and neither the lightbulb nor the MCP `code_actions` tool
  offers O120's rewrite there; W110's own fix stands — the owner's ruling
  on § Open questions 10 (D46). `tcl opt`, MCP `optimize` and
  `optimiseDocument` are unchanged: their reports carry the optimiser's
  findings alone, so no W110 owns anything there.

**Slice 10.** Documents only.

### Progress

Each item updates its row in the commit that lands it.

| Item | Model | Size | Status | Commit | Gates |
|---|---|---|---|---|---|
| DP4.0 | opus | L | done | `slices 4–7 checkpoint green` | every suite of the lane crates, the whole `e2e`, the crate clippy, the catalogue gates, `owner-resolution`, `cargo check --workspace`; `make rust-check` left to C4 |
| DP4.1 | opus | M | done | code: `DP4.1 — the analyser's skip is the policy's on every path, and the report declares it`; documentation: `d941667f` | core `--lib` and the six integration binaries, server `--lib`, the whole `e2e`, `tcl-cli`, `tcl-cli-support`, `tcl-mcp`; the crate clippy; `cargo check -p tcl-lsp-db` and its clippy; `cargo check --workspace` |
| DP4.2 | opus | S | done (the merge of `rust`) | the merge commit | server `--lib`; the crate clippy |
| DP4.3 | sonnet | S | done (by the lane implementer: no `Agent` tool in the session) | `DP4.3 — pin the hand-off decisions no test pinned` | the four tests; server `--lib` subset, `tcl-cli --test cli -- abstaining_document`; the crate clippy |
| DP5.1 | opus | S | done | `DP5.1 — an INI layer can turn a code back on` | core `--lib -- config_ini` (31), `tcl-cli --test cli -- turns_a_code_back_on`; clippy on `tcl-lsp-core` and `tcl-cli`; `kcs-index-links` |
| DP5.2 | opus | S | done (the merge of `rust`) | the merge commit | `cargo test -p tcl-cli`; the crate clippy |
| DP5.3 | sonnet | S | done (by the lane implementer) | `DP5.3 — the batch verbs read no LSP document gate` | `tcl-cli --lib` (27), `tcl-mcp` (92), server `--lib -- optimise_document` (6); the crate clippy |
| DP5.4 | sonnet | S | done (by the lane implementer) | `DP5.4 — the CLI tests never read the machine's config.ini` | `tcl-cli --test cli` (40, and 40 again under a global `config.ini` that disables the codes the suite asserts); `tcl-cli` clippy |
| DP6.1 | opus | M | done | `DP6.1 — one standalone producer run; the MCP diagnostics tools report the editor's set` | `tcl-mcp` (92), `tcl-cli --test cli` (40), core `--lib -- diagnostic_report` (9); the crate clippy |
| DP6.2 | sonnet | S | done (by the lane implementer) | `DP6.2 — #2061's cases on the MCP tools` | `tcl-mcp` (97); `tcl-mcp` clippy |
| DP7.1 | opus | M | done (the merge of `rust`) | the merge commit | core and server `--lib`, the whole `e2e`; the crate clippy |
| DP7.2 | opus | S | done (the merge of `rust`) | the merge commit | core `--lib`, `tcl-cli`, `tcl-mcp`; the crate clippy |
| DP7.3 | sonnet | S | done (by the lane implementer) | `DP7.3 — code-action end-to-end tests; the last name of the old lifter` | `e2e -- code_actions` (93), `tcl-lsp-core --test code_actions_depth` (46); clippy on core and server |
| DP8.1 | opus | S | done — § *Slices 8–10 as built* | `DP8.1 — a fact code is never skipped at production` | core `--lib`, server `--lib`, the whole `e2e`, `tcl-mcp`, `tcl-cli --test cli`; clippy on core and server; `cargo check --workspace` |
| DP8.2 | opus | M | done — § *Slices 8–10 as built* | `DP8.2 — O111 is a producer; every publish path is one call` | core `--lib`, server `--lib`, the whole `e2e` (with `large_file_publishes_fast_tier_before_deep_tier`), `tcl-mcp`, `tcl-cli`; `diag-emission-check`; clippy on core, server, `tcl-cli` and `tcl-mcp`; `cargo check --workspace` |
| DP8.3 | sonnet | S | done — § *Slices 8–10 as built* | `DP8.3 — The lightbulb reads the published report; W115's conversion follows its finding` | core `--lib` (2328, `-- code_actions` 97) and `--test code_actions_depth` (46); server `--lib` (591, `-- lightbulb` 1) and the whole `e2e` (1598, 5 ignored, `code_actions` subset 93); clippy on core and server; `cargo fmt`; `cargo check --workspace` |
| DP9.1 | sonnet | S | done — § *Slices 8–10 as built* | `DP9.1 — One spelling for every reason, and the report's gaps` | core `--lib` (2328, `-- diagnostic_policy` 97); clippy on core; `cargo fmt`; `cargo check --workspace` |
| DP9.2 | sonnet | M | done — § *Slices 8–10 as built* | `DP9.2 — --show-suppressed on tcl diag / lint` | `tcl-cli` lib 27, `cli` 45, `compile_verbs` 11, `explorer_gui` 2, `pkg_verbs` 13, `spec_verbs` 18; clippy on `tcl-cli`; `cargo fmt`; `cargo xtask kcs-index-links`; `cargo check --workspace` |
| DP9.3 | sonnet | M | done — § *Slices 8–10 as built* | `DP9.3 — the MCP suppressed array` | `tcl-mcp` 100; clippy on `tcl-mcp`; `cargo fmt`; `cargo check --workspace` |
| DP9.4 | opus | L | done — § *Slices 8–10 as built* | `DP9.4 — the truth table and its core pass`; follow-up `DP9.4 follow-up — rows 36 and 37 record the verbs' missing rewrite` | core `--lib` and `--lib --features truth-table` (2340 each); pedantic clippy on `tcl-lsp-core` and `tcl-lsp-server` with `--all-targets --all-features`; the server passes; `cargo check --workspace` |
| DP9.5 | opus | M | done — § *Slices 8–10 as built* | `DP9.5 — the LSP and code-action passes on the server` | server `--lib` (590); the `e2e` subset `config` and the whole `e2e`; pedantic clippy on `tcl-lsp-server` with `--all-targets --all-features`; `cargo check --workspace` |
| DP9.6 | sonnet | M | done — § *Slices 8–10 as built* | `DP9.6 — the CLI passes` | `tcl-cli` lib (27), `cli` (47, `-- truth_table` 2), `compile_verbs` (11), `explorer_gui` (2), `pkg_verbs` (13), `spec_verbs` (18); `tcl-cli-support` (19); core `--lib --features truth-table` (2340); clippy on `tcl-cli` with `--all-targets --no-deps`; `cargo fmt`; `cargo check --workspace` |
| DP9.7 | sonnet | M | done — § *Slices 8–10 as built* | `DP9.7 — the MCP passes` | `tcl-mcp` (103); core `--lib --features truth-table` (2340); clippy on `tcl-mcp` with `--all-targets --no-deps`; `cargo fmt`; `cargo check --workspace` |
| Ruling, § Open questions 10 | opus | S | done — § *The owner's ruling on § Open questions 10* | `W110 owns the O120 it sits in (the ruling on question 10)` | core `--lib` (2341), server `--lib` (590), the whole `e2e`, `tcl-mcp`, `tcl-cli --test cli -- truth_table`; the crate clippy; `cargo check --workspace` |
| DP10.1 | opus | M | not started | — | — |
| DP10.2 | sonnet | M | not started | — | — |
| DP10.3 | sonnet | M | not started | — | — |
| DP10.4 | opus | M | the merge landed; the wrapper and the review remain (§ `rust` has moved under the branch, *As merged*) | — | — |
| DP10.5 | sonnet | S | not started | — | — |
| R1 | opus | S | done — the owner's rulings of 2026-09-22: the invocation profile wins (D36); unanimity decides a release-less fold (documents only; the value-transfers lane's F1) | `wip(diagnostic-policy): an invocation profile wins; unanimity decides a release-less fold` | core, server (`--lib` and `e2e`), `tcl-cli`, `tcl-mcp`; the crates' clippy; `kcs-index-links` |
| C4 | — | — | green, except `make rust-check`: its first step, `cargo fmt --all --check`, is red on the value-transfers lane's uncommitted `tcl-registry` / `tcl-compiler` files; the steps that concern this lane's crates ran individually and pass | `C4 — slice 4 checkpoint` | *the suites* of the lane's crates on DP4.1 plus DP4.3's tests; the whole `e2e` (1595, 5 ignored) on DP4.1 — DP4.3 changed no production code; the crate clippy; `cargo fmt --check` on the lane's crates; the ten catalogue gates; `cargo check -p tcl-lsp-db` and its clippy; `cargo check --workspace` |
| C5 | — | — | green | `C5 — slice 5 checkpoint` | `tcl-cli` all targets (lib 27, `cli` 40 with `samples_optimiser_profiles_are_regenerated`, `compile_verbs` 11, `explorer_gui` 2, `pkg_verbs` 13, `spec_verbs` 18), `tcl-cli-support` 19, core `--lib -- config_ini` 31; the crate clippy; `kcs-index-links`; `cargo check --workspace` |
| C6 | — | — | green | `C6 — slice 6 checkpoint` | `tcl-mcp` 97, `tcl-cli --test cli` 40, core `--lib` 2321; the crate clippy; `cargo check --workspace` |
| C7 | — | — | green | the landing commit | *the suites* for core and server with the whole `e2e` (1597, 5 ignored), `tcl-cli` and `tcl-mcp` — inside `cargo test` over the six crates below; the crate clippy |
| Landing (slices 4–7) | opus | — | done — § *Slices 4–7 landed* | `the LSP, CLI, MCP and code-action adapters (slices 4 to 7)` | `cargo test -p tcl-lsp-core -p tcl-lsp-server -p tcl-cli -p tcl-cli-support -p tcl-mcp -p tcl-lsp-db`; pedantic clippy on every touched crate; `cargo fmt --check`; the ten catalogue gates; `make rust-check` |
| Review fixes (slices 4–7) | opus | — | done — § *Review fixes for slices 4–7* | `review fixes for slices 4–7` | core `--lib`, server `--lib`, `tcl-cli`, `tcl-mcp`, the whole `e2e`; the crate clippy; `kcs-index-links`; `cargo check --workspace` |
