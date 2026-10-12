# Do the seeded-parameter controls exercise the forwarded source input?

The fixture compiles, but none of the six selected controls reaches the source
input or SCCP assertions. Each fails at the shared helper's
`cu.function("::p").expect("procedure")` in
`rust/tcl-compiler/tests/compiler_analysis_residual.rs:2040`. Procedure lookup
therefore remains a separate unresolved boundary. This run does not establish
handler widening or safe-call constant preservation. Whether the failure
predates these changes is unknown without a counterfactual run.

The [input review](input-review.json) confirms both memo constructors forward
`req.source_metadata_input`, and the seeded helper forwards
`fu.source_metadata_input()` after the procedure lookup. No fallback or
fabricated input was added. The unchanged original three-field E0063 attempt
remains in the [C source question](../../c-source-namespace-advice/v1-integration-compile.log);
the [byte comparison](original-compile-log-byte-match.status) exits zero.

The command was `cargo test -j2 -p tcl-compiler --test compiler_analysis_residual
seeded_parameter -- --nocapture`, bounded by `timeout --signal=TERM
--kill-after=10s 180s`. It sourced `/tmp/name-env.sh` and used
`CARGO_TARGET_DIR=/workspace/tcl-lsp/tmp/name-target-issues` with incremental
compilation and development/test debug information disabled.

Compilation completed in 1.37 seconds; tests completed in 0.33 seconds: zero
passed, six failed, 113 filtered out. Exit status was 101; no timeout occurred.
The [exact log](seeded-parameter.log) and [status](seeded-parameter.status) retain
the failures. The fixture's [format check](fixture-rustfmt-check.status) passes.

The [before](source-before.json) and [after](source-after.json) receipts match
across 4,395 production Rust, Cargo, and selected fixture inputs, with no changed
paths. Both use HEAD `1b3a01d66c315088bc3726dbb1ee7be2898af897` and source inventory
SHA-256 `1dcf700e033dbf53954f250a2065083106f900e88d9e9ce60159a5e6c16d3b5f`.
No source edits or additional test runs were made by this job. The Cargo lease
was released after the bounded run.
