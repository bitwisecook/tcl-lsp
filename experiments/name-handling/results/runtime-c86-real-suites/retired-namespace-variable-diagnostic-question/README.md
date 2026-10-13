# Retired namespace variable diagnostics

The unchanged real C86 `all.tcl` orchestration selected `var.test`, reached its `var-1.15` start, then refused with `original variable lookup diagnostic native handler policy is not selected`. No final test counts were emitted. [The orchestration record](../glob-type-union-runtime/orchestration-02/record.json) establishes this reached boundary, not the exact failing inner getter.

The original `var-1.15` setup creates namespace scalar `test_ns_var::foo`, links it into procedure `p`, reads it, deletes the namespace, then attempts scalar and element writes through the retained link. `var-1.14` is the preceding live-name control; it does not select retired-cell diagnostics.

[Five-row input](input.tcl) retains these operations and adds adjacent result/errorCode observations. Its cases distinguish scalar and element writes in literal procedure bodies, dynamically constructed scripts, and the original command sequence. [Two-row input](variable-head-input.tcl) forces ordinary command dispatch with a variable command head. These are bounded new probes, not adapted Tcltest execution or original C API/private object evidence.

Fresh [capture-01](capture-01/provenance.json) and [capture-02](capture-02/provenance.json) cover C8.4.20, C8.5.19, C8.6.18, C9.0.4, C9.1.0 and pinned Jim. Every provider reached all seven rows and eight before-write markers. C84/C85 reject the retired-link writes with the expected message and `NONE`. C86/C9 retain the same messages and distinguish these completions:

| Reached write | Error-code words |
| --- | --- |
| Scalar, all three entries | `TCL WRITE VARNAME` |
| Element, literal or dynamically compiled script | `TCL LOOKUP VARNAME` |
| Element, variable command head | `TCL LOOKUP VARNAME foo` |

Jim permits the first scalar or element write and returns `3`. In the original sequence, its scalar write succeeds; the following element write fails with `variable isn't array` and `NONE`. Jim has a separate behavior, not the C retired-namespace reporting policy. F5 was not run.

The [source observations](source-observation.md) and [source/probe record](source-provenance.json) separate branch text from final completion. The central `native_variable_failure_code` supports `RetiredNamespace` at `ValueWrite`, but has no C86+ `NameLookup` case. Runtime's reached element failure explicitly requests `NameLookup`; the rejected projection becomes the observed typed host refusal.

A blanket new tuple is insufficient: the same element spelling has three or four error-code words depending on the actual caller. The minimal prospective policy needs an independently retained indexed/ordinary lookup reporting context, including whether the original root word is issued. Original combined/separate name geometry alone cannot manufacture that context. The existing array-root `IsArray` proof supports its own `ValueWrite` tuple and does not establish this early retired-root purpose.

Prospective implementation owners are the central Syntax diagnostic policy and Runtime's actual failure-site adapter. Exact Runtime caller evidence is still unexecuted; no Syntax/Runtime source changes or builds were made for this question. Future controls should compare these retained rows, preserve original operand spelling and assert that unsupported/missing caller context refuses instead of borrowing the other tuple.
