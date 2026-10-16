# Separate qualified-formal negative fixture correction

Status: oracle-backed test-premise correction only. No production changes, Cargo, candidate execution, main writes, source instrumentation or Git. This patch is separate from the Logical alias template repair. Count 0.

## Actual original provider evidence

Root's original six-provider probe is `/tmp/lsp-qualified-formals-20261016T1520/manifest.json`, with exact `probe.tcl` and individual original stdout files. This package records and verifies every stdout SHA against that manifest, plus the probe/manifest/source hashes. Root's original runner used 45 seconds per provider and reports original-source support match true. No probe was rerun here.

All five original C releases 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 reject qualified formal names `::n` and `ns::n`, and array formal `a(b)`, at procedure creation with no `p` installed. The selected existing test helper `original_reference_unit` explicitly uses C Tcl 8.6. Jim 0.84-9-g5bac7c9 at 5bac7c99ad65864c87da513e22e2f01703fa4e03 accepts/invokes these formals; its original output is retained and is not used to infer the C fixture or any Jim compiler result.

The original C8.6.18 source owner is `tmp/tcl8.6.18/generic/tclProc.c`, source tag core-8-6-18 from Root's provider-pairing capture. Exact lines 508–529 are retained separately here with complete-file hash and source manifest UUID. The original scalar-formal validation rejects an element spelling ending in `)` after `(`, and rejects `::` in the formal name, then goes to `procError`. See [the original local source](/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclProc.c:508). Root's original stdout confirms the reached creation error and absence, rather than just a source prediction.

## Invalid premise and preserved assertions

Frozen ordinary report012 passes its main positive before panicking when it unwraps an authored body card for `proc p {::n} {upvar 1 $::n x; set x 1}`. C8.6 rejects that procedure's formal grammar; the unavailable source card is the correct producer result. This patch keeps that exact original source as an explicit unavailable-body-card assertion.

The original negative no-VarWrite(`::n`) assertion remains byte-for-byte intact after selecting a separate valid C fixture `proc p {n} {upvar 1 $::n x; set x 1}`. Its actual formal is `n`, while the target source reads qualified global `::n`. An added no-VarWrite(`n`) assertion verifies that source global read cannot be treated as the actual formal. An added ordinary upvar-alias-empty assertion keeps the physical alias refusal explicit. No original assertion is deleted. Every other original source/assertion in the test remains unchanged; no fake body card, fallback or production guard change is introduced.

The negative `::n` assertion alone would be insufficient to prove the absence of an effect on actual formal `n`, hence the separate added assertion. The rejected original source and the supported body with an independent global target are distinct controls.

## Root validation obligation

Fresh ordinary selector, preserving existing execution budget/environment and singleton listing:

`interprocedural::tests::original_authored_alias_reports_keep_source_formals_frames_and_unknown_debts`

For Root only: `cargo test --locked -p tcl-compiler --lib original_authored_alias_reports_keep_source_formals_frames_and_unknown_debts -- --nocapture --test-threads=1`.

TMP rustfmt and patch dry-run pass; the beforeimage matched current main at release. No success, whole-suite acceptance or Jim implementation result is claimed by this package.
