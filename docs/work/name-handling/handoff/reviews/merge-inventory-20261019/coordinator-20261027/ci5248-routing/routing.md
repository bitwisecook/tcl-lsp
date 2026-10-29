# CI 5248 (WORK 83a0b2351) vs origin/rust 5203: pass→fail routing

Headline: 10,962 pass→pass, **2,835 pass→fail**, 10,299 unobserved (shards 1/2/4 killed, lsp-e2e hung), 216 absent.
Zero pass→fail against origin/rust is a hard release gate. Every row below is a test that passed on origin/rust.

Your list: `roles/<role>.tsv` (binary_id, selector, CI job, panic site if the 128-byte sample had one).
Full data: `compare.json`; overview: `summary.md` (same directory).

## How to work it (after finishing the item you are on now)

1. Sample: for each binary in your list, run a handful of its failing selectors locally with
   `safe-cargo.sh test -p <crate> --test <target> -- <filter>` (lib targets: `--lib <filter>`),
   one target at a time, output filtered to panic/assert lines. Group the failures by panic message.
2. Report the clusters (message, count, files) before the big fixes, so root can spot cross-role causes.
3. Fix root causes in owned files. Where a cause sits in another role's file, report the exact hunk.
4. A test change is acceptable only under the existing rules: it contradicts tclsh, or it falls under a
   recorded decision (standalone units are "correct all affected tests", non-vacuously: the corrected test
   must still prove its original property, typically by supplying the complete command world or dialect).
   Never delete, `#[ignore]`, or weaken a test to clear the list.

Common panic sites seen across roles (likely shared causes):
- `rust/tcl-compiler/tests/support/body_identity.rs:18` "fixture body \"::p\" absent" / "original authored fixture body" (65 rows)
- `rust/tcl-vm/src/exec.rs:1792` `CompilationAdmission(NativePreflightRequired)` (29) and `exec.rs:1905` (23)
- `type_propagation.rs:89`, `var_escape_typeinfer.rs:124/164`, `var_escape_residual2.rs:143`
- `StringUpdater requires checked child storage` (tcl-vm value.rs)
- LSP rename: "completed original command world is unavailable" / "without original coverage"

## Role extras

- lsp: three tests hung and were never observed. In lsp-e2e 1/3: `tests::an_edit_during_a_config_apply_re_resolves_its_diagnostics_inputs` and `tests::default_dialect_change_retries_after_concurrent_edit_1854`. In lsp-e2e 2/3: `tests::a_late_resolve_cannot_stamp_the_slot_backwards` (SLOW > 9960 s). A hang is a release blocker on its own, so diagnose these first, with a bounded timeout. test-ext-multi-folder: 4 failures (`raw/extmulti.log`).
- compiler-codegen: in shard 2, the `tcl-cli::value_transfers_cli` `opt_*` tests ran SLOW for more than 120 s, and the shard was killed after 5 h 34 min. Find out why.
- vm-runtime: `runtime-rust-tests` has 125 libtest failures, listed in `roles/vm-runtime-runtime-rust.txt` (interp 58, cmd_namespace 18, cmd_oo 12). spectcl-compat `spec_corpus` has 2 failures. wasm-real-link has 1 failure.
- compiler-names: owns the tcl-compiler lib `analyser::diagnostics` cluster (257). Also routed here for triage: tcl-cli (except value_transfers), bpf-tcl, f5-xc, bigip, tcl-registry and tcl-lexer.
- compiler: owns `tcl-compiler::analyser class_factories` (84), taint (79 plus 26 taint_depth), mro_lattice and tcl-irules.
