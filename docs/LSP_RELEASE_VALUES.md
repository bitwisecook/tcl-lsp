# LSP value and diagnostic release audit

Audit base: `961f17a32` on `work`. A source-level review found existing focused
regression cases for the reported recent issues; a test name below is evidence
of coverage, not a claim that it has been rerun in this checkout.

| Issue | Current evidence and status |
| --- | --- |
| #2498, #2500, #2504 | `compiler_checks::list_element_shimmer_controls` covers list elements and signed infinity; execution pending. |
| #2499 | `w212_allows_indirect_read_through_a_computed_name_2499` covers the reported form; execution pending. |
| #2503 | `selected_nested_expression_controls` covers quoted and array-index operands and a direct expression operand; execution pending. |
| #2505 | `w301_list_built_uplevel_script_is_silent_across_lines_2505` covers multiline, single-line and unsafe quoted controls; execution pending. |
| #2506 | `a_comparison_result_is_not_tainted_2506` covers comparison result flow; execution pending. |
| #2507 | `w100_suggests_scan_for_a_hexadecimal_conversion_2507` covers the advisory and a non-hex control; execution pending. |
| #2131 | `type_infer` uses the `tcl-syntax` `MathResultClass` owner; execution pending. |
| #2288 | `fp_sty_14_prefixed_list_built_body_no_w105_2288` exists; execution pending. |
| #2440 | Selected script-argument roles avoid scanning the containing body. The first integration run confirmed the proc body warning disappeared and that the literal and proven `$m` use still warn; the corrected multi-case assertion is running. |
| #2436 | The lexer exposes the nested parse cut, but analyser diagnostics currently use its flat warning stream; reproduction and proposed integration pending. |
| #2387, #2437, #2438, #2451–#2454, #2427 | Awaiting reproduction against current branch. |

The initial compiler test build was stopped before completion to honour the
shared machine's compiler-build concurrency limit. The later `--lib` test
build failed on 43 existing test-compilation errors, including unavailable
sparse-checkout fixtures and private-module and ambiguous empty-array assertions
(`/tmp/lsp-values-2440.log`). The focused integration test is being rerun after
correcting its expectation for a proven variable use. Full gates have not run
in this checkout.
