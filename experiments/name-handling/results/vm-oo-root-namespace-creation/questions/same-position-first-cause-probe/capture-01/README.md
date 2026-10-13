# Same-position first-cause diagnostic: resource preflight refusal

The environment-gated diagnostic was prepared and temporarily applied only to
`rust/tcl-vm/src/interp/oo_cleanup/active_frame/tests.rs` and
`rust/tcl-vm/src/interp.rs`. Original arguments, assertions, observer flow,
cleanup and clone/clear/return predicates were unchanged. The selected exact
control was
`interp::oo_cleanup::active_frame::tests::original_method_cleanup_refuses_a_genuinely_new_same_position_context_without_retiring_it`.

Root stopped this capture during conservative source archive preparation,
before any VM build, after the concurrent standalone Runtime build reported
ENOSPC. No VM Cargo process, native test binary, test list or control ran.
This is a resource preflight refusal, not a VM compile or assertion failure.
There is no returned-cause or settlement measurement and no fixture correction,
production repair, oracle result or proof renewal.

The runner's finally block restored both owners byte-exact. Complete original,
instrumented and restored hash inventories show zero final source changes.
The original question15 and all prior evidence remain unchanged. The
conservative archive was interrupted and moved unchanged to ignored `tmp/`:
[execution.json](execution.json) retains its SHA256,107,757,267byte count and
incomplete status. It is generated context, not a semantic result, complete
source image or compiled closure. The unpublished receipt and current seal
were explicitly amended; no raw stdout or semantic outcome was removed. No actual Cargo
artifact graph or binary dep-info was reached.

[Execution](execution.json), [prepared plan](plan.json),
[logging-only patch](instrumentation.patch) and [runner](run.py) retain the
preflight, full owner hashes and restoration. The named synthetic gate is
`NH_VM_FIRST_CAUSE_DIAGNOSTIC`; no host environment was dumped. Source and
issues-cache leases are released. A future capture requires fresh source/cache
readiness and separate evidence; this directory records only this refusal.
