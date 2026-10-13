# Corrected VM same-position control

Each listed exact control ran once on its pinned current library binary from
the [same selected build](../build/README.md). Full stdout/stderr and exact
invocations are retained beside [validation](validation.json).

| Exact control | Passed | Failed | Filtered |
| --- | --- | --- | --- |
| `interp::oo_cleanup::active_frame::tests::original_method_cleanup_refuses_a_genuinely_new_same_position_context_without_retiring_it` | 1 | 0 | 1197 |

The genuine inner method call compares the full captured SamePosition cause
and retains stored Some; the outer call observes that same full cause and
cleared storage. Existing birth/callback/cleanup assertions passed in this
scoped Rust VM control. Production VM code is unchanged by the fixture repair.
This is no native C/Jim or F5 TMM/MCPD oracle result and does not certify the
other forward/procedure refusal controls. Earlier measured diagnostics and
original failures remain unchanged.

All 11,803 conservative source/data hashes and selected binary hashes stayed
unchanged across the one build and all controls. Standalone Runtime is
excluded; no source edit, retry or full suite accompanied this capture.
