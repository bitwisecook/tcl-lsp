# Lane: diagnostic policy — slices 1 to 7

Tracking document for slices 1 to 7 of
[`docs/design/compiler/diagnostic-policy.md`](../compiler/diagnostic-policy.md)
§ *Slices* (issue #2089). Slices 1–3 are landed and accepted; slices 4–7
stop at the hand-off point recorded in *Status at hand-off* below. Protocol: [README.md](README.md) — the tree
compiles before every commit, files are staged by explicit path, every
checkpoint is `wip(diagnostic-policy): …`, the orchestrator pushes.

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
on `claude/spectcl-optimization-discussion-5qhf42`, verified only as
§ *Gate results at hand-off* says. Read this section top to bottom before
touching anything: the code compiles, but none of the four surfaces'
end-to-end suites has been run against it.

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

- **No end-to-end suite has been run on this state.** The five crates
  (`tcl-lsp-core`, `tcl-lsp-server`, `tcl-cli`, `tcl-cli-support`,
  `tcl-mcp`) compile with `--all-targets` and no warnings; that is all.
- **CLI parity tests are drafted, not in the tree.** The three tests below
  (§ *Drafted CLI tests*) were written for `rust/tcl-cli/tests/cli.rs` and
  never compiled: `diag_seeds_the_default_off_codes_like_the_editor`,
  `diag_resolves_the_project_and_global_layers_per_input_file` (#2063),
  `opt_applies_only_the_rewrites_the_policy_shows` (#2062). They isolate the
  global layer with `XDG_CONFIG_HOME`.
- **MCP parity tests are not written.** Planned, against the `*_with`
  forms with `PolicyInputs { global: json!({}), invocation }`:
  `analyze` honours an inline `# noqa` (W210 before/after), honours
  `disable`, seeds W242 off and `enable: "W242"` brings it back; `optimize`
  skips a `# noqa: O102` fold and a `# tcl-lsp: disable=*` document;
  `code_actions` offers no "Brace expr" for a `# noqa: W100` line and
  offers the O102 fold as a `quickfix` on a shown rewrite.
- **Not started:** the `tcl-lsp-db` `file_analysis` commit (the coordinator's
  own instruction: keep the skip, document it as the declared production
  skip; stage only those hunks — the value-transfers lane edits
  `FnLatticeKey` in the same file), removal of `Policy::from_disabled_set`
  (no caller left after slice 5 — delete it and its doc), removal of
  `sslictcl_diagnostics::supersede_analyser_diagnostics` (no caller left;
  keep `SUPERSEDED_ANALYSER_CODES`, `dialect_overlaps` reads it),
  `InputDocument::encoding_diagnostics` (no caller left in the workspace).
- **The style-finding lift now goes through the finding's byte span** (the
  slice-3 tracking note's deferred W107-in-a-lone-`\r`-file position
  change): expected identical for every other code; unverified.

### Remaining steps, in order

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
- No test, clippy, xtask or `make rust-check` run on this state.

### Drafted CLI tests

For `rust/tcl-cli/tests/cli.rs` (uses that file's `Command`, `PathBuf`
imports and its existing helpers):

```rust
/// A scratch directory for one test, removed on drop.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("tcl-cli-{tag}-{nanos}"));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        Self(dir)
    }

    /// Write `text` at `rel` (directories created) and return its path.
    fn write(&self, rel: &str, text: &str) -> PathBuf {
        let path = self.0.join(rel);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
        std::fs::write(&path, text).expect("write file");
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

/// Run the built `tcl` binary with `args` and `env`, tolerating a non-zero
/// exit, and return its stdout.
fn run_tcl_env(args: &[&str], env: &[(&str, &std::ffi::OsStr)]) -> Vec<u8> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tcl"));
    command.args(args);
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().expect("failed to spawn tcl binary").stdout
}

/// `(file label, code)` pairs of a `diag --json` report.
fn diag_codes_by_file(out: &[u8]) -> Vec<(String, String)> {
    let report: serde_json::Value = serde_json::from_slice(out).expect("diag JSON");
    report
        .as_array()
        .expect("report array")
        .iter()
        .flat_map(|file| {
            let label = file["file"].as_str().expect("file").to_owned();
            file["diagnostics"]
                .as_array()
                .expect("diagnostics")
                .iter()
                .map(move |d| (label.clone(), d["code"].as_str().expect("code").to_owned()))
        })
        .collect()
}

/// A `while` whose counter the body never touches: W242, the one code the
/// catalogue declares default-off.
const UNPROVABLE_LOOP: &str = "set i 0\nwhile {$i < 3} {\n    puts $i\n}\n";

/// The catalogue's default-off codes are off for `tcl diag` as they are in
/// the editor, and `--enable` turns one on — the seed is the lowest layer,
/// under every flag (`docs/design/compiler/diagnostic-policy.md`
/// § Configuration).
#[test]
fn diag_seeds_the_default_off_codes_like_the_editor() {
    let scratch = Scratch::new("default-off");
    let no_config = scratch.write("config/.keep", "");
    let xdg = no_config.parent().expect("config dir").as_os_str();
    let env: &[(&str, &std::ffi::OsStr)] = &[("XDG_CONFIG_HOME", xdg)];
    let off = diag_codes_by_file(&run_tcl_env(
        &["diag", "--json", "--source", UNPROVABLE_LOOP],
        env,
    ));
    assert!(
        !off.iter().any(|(_, code)| code == "W242"),
        "W242 is default-off and must not fire unasked: {off:?}"
    );
    let on = diag_codes_by_file(&run_tcl_env(
        &["diag", "--json", "--enable", "W242", "--source", UNPROVABLE_LOOP],
        env,
    ));
    assert!(
        on.iter().any(|(_, code)| code == "W242"),
        "`--enable W242` must reach a default-off code: {on:?}"
    );
}

/// The project and global layers resolve per input file (issue #2063): a
/// file under a `.tcl-lsp.ini` that turns W112 off reports none while its
/// sibling from another directory still does; the global `config.ini` is
/// the lowest layer and a project file turns a code it disabled back on.
#[test]
fn diag_resolves_the_project_and_global_layers_per_input_file() {
    let scratch = Scratch::new("layers");
    let trailing = "set x 1   \nputs $x\n";
    scratch.write("quiet/.tcl-lsp.ini", "[diagnostics]\nW112 = false\n");
    let quiet = scratch.write("quiet/nested/a.tcl", trailing);
    let loud = scratch.write("loud/b.tcl", trailing);
    let config = scratch.write("xdg/tcl-lsp/config.ini", "[diagnostics]\nW112 = false\n");
    let xdg = config
        .parent()
        .and_then(|p| p.parent())
        .expect("xdg root")
        .as_os_str();
    let empty = scratch.write("empty-xdg/.keep", "");
    let no_global = empty.parent().expect("empty xdg").as_os_str();

    let per_file = diag_codes_by_file(&run_tcl_env(
        &[
            "diag",
            "--json",
            quiet.to_str().unwrap(),
            loud.to_str().unwrap(),
        ],
        &[("XDG_CONFIG_HOME", no_global)],
    ));
    let has = |rows: &[(String, String)], file: &PathBuf, code: &str| {
        rows.iter()
            .any(|(label, c)| label.ends_with(file.file_name().unwrap().to_str().unwrap()) && c == code)
    };
    assert!(
        !has(&per_file, &quiet, "W112"),
        "the project file above a.tcl turns W112 off: {per_file:?}"
    );
    assert!(
        has(&per_file, &loud, "W112"),
        "b.tcl sits under no project file and keeps W112: {per_file:?}"
    );

    // The global file reaches both; a project file turns the code back on.
    scratch.write("loud/.tcl-lsp.ini", "[diagnostics]\nW112 = true\n");
    let with_global = diag_codes_by_file(&run_tcl_env(
        &[
            "diag",
            "--json",
            quiet.to_str().unwrap(),
            loud.to_str().unwrap(),
        ],
        &[("XDG_CONFIG_HOME", xdg)],
    ));
    assert!(
        !has(&with_global, &quiet, "W112"),
        "global and project both disable W112 for a.tcl: {with_global:?}"
    );
    assert!(
        has(&with_global, &loud, "W112"),
        "a project `W112 = true` overrules the global `W112 = false`: {with_global:?}"
    );

    // An inline `--source` has no path, so no project layer: the global
    // file alone decides, and `--enable` in the invocation layer overrules it.
    let inline = diag_codes_by_file(&run_tcl_env(
        &["diag", "--json", "--source", trailing],
        &[("XDG_CONFIG_HOME", xdg)],
    ));
    assert!(
        !inline.iter().any(|(_, code)| code == "W112"),
        "the global layer reaches an inline source: {inline:?}"
    );
    let flagged = diag_codes_by_file(&run_tcl_env(
        &["diag", "--json", "--enable", "W112", "--source", trailing],
        &[("XDG_CONFIG_HOME", xdg)],
    ));
    assert!(
        flagged.iter().any(|(_, code)| code == "W112"),
        "`--enable` sits above the global file: {flagged:?}"
    );
}

/// `tcl opt` applies only the rewrites the document's policy shows (issue
/// #2062): a `# noqa` on the command keeps its fold off, a top-of-file
/// `# tcl-lsp: disable=*` keeps every rewrite off, and two inputs whose
/// directives differ are optimised each under its own policy rather than
/// folded into one text where the first file's directive would govern the
/// second.
#[test]
fn opt_applies_only_the_rewrites_the_policy_shows() {
    let scratch = Scratch::new("opt-policy");
    let empty = scratch.write("xdg/.keep", "");
    let xdg = empty.parent().expect("xdg").as_os_str();
    let env: &[(&str, &std::ffi::OsStr)] = &[("XDG_CONFIG_HOME", xdg)];
    let folding = "set x [expr {1 + 2}]\nputs $x\n";

    let plain = String::from_utf8(run_tcl_env(&["opt", "--source", folding], env)).unwrap();
    assert!(plain.contains("set x 3"), "the control folds: {plain}");

    let marked = format!("# noqa: O102\n{folding}");
    let kept = String::from_utf8(run_tcl_env(&["opt", "--source", &marked], env)).unwrap();
    assert!(
        kept.contains("[expr {1 + 2}]"),
        "a `# noqa` on the command keeps the fold off: {kept}"
    );

    let silenced = scratch.write("silenced.tcl", &format!("# tcl-lsp: disable=*\n{folding}"));
    let open = scratch.write("open.tcl", folding);
    let both = String::from_utf8(run_tcl_env(
        &["opt", silenced.to_str().unwrap(), open.to_str().unwrap()],
        env,
    ))
    .unwrap();
    assert!(
        both.contains("[expr {1 + 2}]"),
        "the silenced file's expression survives: {both}"
    );
    assert!(
        both.contains("set x 3"),
        "the open file's expression folds under its own policy: {both}"
    );
}
```

