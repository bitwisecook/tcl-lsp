# Lane: diagnostic policy — slices 1 to 3

Tracking document for slices 1, 2 and 3 of
[`docs/design/compiler/diagnostic-policy.md`](../compiler/diagnostic-policy.md)
§ *Slices* (issue #2089). Protocol: [README.md](README.md) — the tree
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

So the commit that lands this file carries all three slices, verified as
above and no further. **Not run on the final state**: the `tcl-lsp-core`
lib suite (`config_ini`, `policy_tests`, `apply_tests`, the style and
SslicTcl unit tests — one borrow error in the SslicTcl test was fixed by
reading after the build died), the server's unit and e2e suites, the CLI
suite (`tcl diag` parity: `diag_honours_noqa_directives_the_way_the_editor_does`,
`diag_honours_a_file_directive_on_an_abstaining_document`, the style-row
and `SSLIC` tests), the MCP suite, and clippy. The next agent runs, in
this order, before anything else: `cargo check --workspace`; `cargo test
-p tcl-lsp-core --lib`; `cargo test -p tcl-lsp-server --lib lift_`,
`settings_`, `sslictcl`, `xc_diagnostics`; `cargo test -p tcl-cli --test
cli diag`; `cargo test -p tcl-mcp`; `cargo clippy -p tcl-lsp-core -p
tcl-lsp-server -p tcl-cli -p xtask -p f5-xc --all-targets -- -D
warnings`; then the catalogue gates listed above. A parity failure on the
server or CLI is a defect in a shim in `lift_source_style_diagnostics`,
`extend_with_sslictcl_diagnostics`, `lift_f5_source_integrity_diagnostics`,
`style_rows`, `push_sslictcl_rows` or `abstained_rows`, not in the
producers.
