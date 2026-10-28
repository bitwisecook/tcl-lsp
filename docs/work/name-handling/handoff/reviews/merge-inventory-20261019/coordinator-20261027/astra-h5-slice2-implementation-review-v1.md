# Astra re-review: H5 slice-2 implementation at 40a8a6c8c - ACCEPT-WITH-NOTES (2026-10-28)

Suites: tcl-spectcl owned_application/owned_evaluator/install::tests::private 23/23 PASS. UNRUN (shared tree briefly broken by another role's uncommitted registry_invocation.rs edit): tcl-vm closed_native_execution, tcl-engine-tclvm closed_native_execution, tcl-compiler application_input/closed_consumer/native_authored_bracket.

Deviations: (1) for_shared_registry over the exact private Arc, answering store pinned by pointer - ACCEPT (stronger than v2); (2) structural labels admitted, jumps refused - ACCEPT; (3) writing hook -> Source (sandbox has no output command; quietness still enforced on every answer) - ACCEPT; (4) store cap 64, refuse-not-evict with Budget - ACCEPT (document per-store leak size).

All ten required changes verified with file:line evidence (inventory application.rs:325-423; original vs candidate :263-282 and owned_application.rs:240-242; single checker closed_native_execution.rs:19-22,306-315; exhaustive FunctionAsm/Instruction destructuring owned_application.rs:169-315 with PUSH rule matching subst.rs:463-467; compiler_generation + service pointer pin; two phases with host retirement; whole-word admission with V9_1 admits_answer; memo by (profile, pack key); receipt with runtime_cross_checked false; minor items).

Notes (non-blocking): (1) owned_application.rs:177 `lvt: _` leaves the local-slot table unclosed while LOAD_SCALAR1/4 are admitted - add `|| !lvt.is_empty()` to the refusal (and drop LOAD_SCALAR1/4 if a plain top-level module never emits them); (2) record the measured per-store leak beside MAX_OWNED_HOOK_REGISTRIES; (3) run the three UNRUN suites.

Proven for N17: the $arr($a:b) fixture answered by a quiet private C9.1 pack hook yields `good`, selected by ordinary O129 gates, rendered into a byte-corresponding candidate, run once on a fresh pinned memory-only VM with no live evaluator and the original never executed, producing the C Tcl transcript. Evidence of restricted execution, not fold soundness against the runtime original.
Open: runtime-backed original cross-check; MAP/RANGE; non-plain dispatch; nested scripts/templates; interpolation sites; literal actions beyond Set/ChannelWrite(stdout); optimiser fixture wiring; final backend/candidate join; detached metrics; production server path for N17.
