# Generic minifier/analysis continuation blocker

Original unchanged test: `f5-cli --test irule`
`minify_aggressive_survives_deeply_nested_irule`.

Its deterministic source is `when HTTP_REQUEST {`, 500 nested `if {1} {`
lines, `pool /Common/p`, then all closing braces. Exact original input is
`minify-500-input-20261023.irule.gz`; no depth or assertion changed.
Production CLI command and SHA hashes: `minify-interruption-20261023.json`.

The production child consumes a core without returning for634s, then was
externally stopped with SIGTERM to continue the required trace/LSP checks.
The harness consequently prints FAIL/-1; classify this as an incomplete
interrupted measurement, not a natural semantic assertion failure or a pass.
No timeout budget or expectation was relaxed.

The CLI calls `tcl_lsp_core::minify::minify_with_profile` directly with its
actual F5 profile/registry and Aggressive options. The generic owner then
runs `original::analysis_for_profile` and the retained-analysis planner. There
is no F5 wrapper inference to repair here. The original contract remains a
clean exit0 or an honest reported refusal2 at depth500; the generic
minifier/compiler/performance owners must restore bounded work while retaining
source/identity guards. Do not reduce depth, ignore the test or bypass owners.
