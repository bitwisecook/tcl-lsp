# Signed scan values and expression precedence, 20 November 2026

This work landed on `work` between `ab381fa1` and `ec32f8e4`. It adds W154 (operator mix that groups differently from how it reads), W155 (any non-associative grouping, hint, off by default) and W156 (signed `binary scan`/`scan` value compared or consumed as unsigned, replacing IRULE5008), the expression precedence hover tree, the parenthesise and remove-redundant-parentheses refactors, the minifier's parenthesis pass, the anchored-glob F5 `matches` fix in `f5-xc`, and registry integer argument domains (`arg_domains`, SpecTcl `-domain`, vocabulary 2.3).

## Scope

- `rust/tcl-syntax/src/expr/precedence.rs`, `rust/tcl-syntax/src/scan.rs`, `rust/tcl-syntax/src/glob.rs`.
- `rust/tcl-compiler/src/analyser/diagnostics/usage/precedence_hazards.rs` and `rust/tcl-compiler/src/analyser/diagnostics/dataflow/signed_scans.rs` with their tests.
- `arg_domains` in `rust/tcl-registry`, its SpecTcl loader and Spec Studio field.
- `rust/tcl-lsp-core` hover, refactor and minify additions for expression parentheses.

## How to measure

```bash
cargo test -p tcl-compiler --lib -- signed_scans precedence_hazards
cargo test -p tcl-syntax --lib -- precedence glob scan
cargo test -p tcl-registry
```

The first takes about 15 seconds once built. Oracle evidence is in `evidence/tcl/binary-scan-signed-byte-comparison.md`, `scan-integer-signedness.md`, `signed-scan-downstream-consumers.md`, `expr-precedence-groupings.md` and `string-match-bracket-set-edges.md`; BIG-IP facts are in `docs/design/f5/bigip-irule-parser-measurements.md`.

## Baseline failures

Each was measured by stashing the change and running the same command on the clean parent `3dcd870a`. No test failed with the change that passed without it.

- [`lists/signed-scan-precedence-lib-failing-2026-11-20.txt`](../lists/signed-scan-precedence-lib-failing-2026-11-20.txt): 1,158 names from one `cargo test --lib` run over `tcl-compiler`, `tcl-registry`, `tcl-spectcl` and `tcl-lsp-core` (names carry no crate prefix). Registry 7 of 2,065, LSP core 426 of 3,663 (including 24 `minify::` tests), SpecTcl 1 of 357, the rest compiler. SpecTcl `cache::tests::the_two_tiers_share_one_identity` is flaky: it failed on the baseline and passed with the change.
- [`lists/signed-scan-precedence-other-failing-2026-11-20.txt`](../lists/signed-scan-precedence-other-failing-2026-11-20.txt): `f5-xc` (XC101 uncatalogued, the golden differential, eight model tests), `tcl-spec-studio` (one versions test and the SpecTcl round trip, 19 glob `pattern_type` differences) and the `tcl-mcp` review test.
- Gates failing at baseline: strict clippy in `tcl-codegen` (`codegen/native_compiler_pass.rs:301` `assigning_clones`, plus `too_many_lines` in `embedded_operands.rs` and `original_member_rename.rs`), licence headers on `compiler_checks/payload.rs`, `payload_contract_tests.rs` and `registry/commands/closed_evidence/table.rs`, segmentation drift at `usage/unbraced_words.rs:102` and `dataflow/value_domains.rs:201`, and the registry-axes pin for `diagnostics/usage.rs` (observed 0, pinned 1).
- `make codegen` regenerates `.tclspec` files under `rust/tcl-spectcl/equivalence/` with unrelated drift; it was left uncommitted.

## Queue

1. **Methods versus procedures in the per-function pass.** `CompilationUnit` builds SSA for procedures but method bodies in `cu.methods` get no SSA blocks, so every SSA-driven check (W233, W210, W156) is silent inside `oo::define … method` and `itcl` bodies. Inside procedures, commands after a call the binder cannot prove stay unlowered generic calls behind `<unseen-call>` markers, so SSA names no variable after them; W156 works round this with marker-aware fallbacks (`beside_marker`, `UnresolvedScans`), which other dataflow checks lack. Review which per-function analyses run for each unit kind, give method bodies the same pass as procedures (with `self`/`my` and the object's variables as entry facts), and decide whether lowering should continue past an unproven call rather than each check re-deriving facts. Smallest fixture: `oo::class create C { method m {d} { binary scan $d c b; if {$b == 0x81} {} } }` should report W156 as the same body in a `proc` does. Files: `rust/tcl-compiler/src/compilation_unit/function_build.rs`, `rust/tcl-compiler/src/cfg.rs`, `rust/tcl-compiler/src/analyser/diagnostics.rs`.
2. W156 consumers not modelled: `table` values (unsigned 64-bit, but negative `incr` deltas are legitimate) and `class match` (W304 already covers it).
