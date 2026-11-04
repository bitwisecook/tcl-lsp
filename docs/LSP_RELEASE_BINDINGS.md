# LSP binding and frame issue verification

The audit starts from shared work commit `961f17a32`. The original checkout's
older binary is not evidence for the current work branch. Verification uses an
isolated Cargo target and fixed-input regressions, with the pinned C Tcl
8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 interpreters and JimTcl `82fb4d0`.

| Issues | Current evidence | Status |
| --- | --- | --- |
| #2501, #2502 | Fresh fixed-input loop/output and dictionary-unpack diagnostic checks pass. | Verified baseline |
| #2246, #2371 | C Tcl and Jim confirm the missing-variable reads; fresh compiler emits no W210. | Reproduced residual |
| #2374, #2441 | Distinct literal array elements and empty-loop variables emit W210; C Tcl and Jim confirm the errors. | Verified baseline |
| #2376 | C Tcl and Jim confirm the short-circuit expression is safe; fresh compiler warns on its skipped read. | Reproduced residual |
| #2379 | Absent-array size/names/get emit no W210, matching C Tcl and Jim. | Verified baseline |
| #2294 | Work already carries/rebases `BodyFragment::proven_sites`; whole-file W230 is absent in the parity witness, before testing per-item parity. | Residual under investigation |
| #2268 | Existing witness fails its dead-store control: an unused literal element has no W211. The computed-element read control passes. | Reproduced residual |
| #2289, #2296 | Existing child-interpreter creator and computed-array-key tests pass in `name_lookup_issue_witnesses.rs`. | Verified baseline |
| #2243 | Memoised/uncached seeded-global parity still requires a fresh database regression. | Pending |
| #2247 | Qualified dictionary implementation emits neither W002 nor W211; W143 correctly advises the public spelling. | Verified baseline |
| #2286 | All four existing reported tcltest setup/body/cleanup fixtures emit no W210. | Verified baseline |
| #2290 | Latest public `tclmeasure.tcl` emits no W210; upstream source differs from the historical line references. | Current source verified |
| #2297 | Caller-variable read through upvar produces no W220. | Verified baseline |
| #2305, #2306 | Qualified-root writes and same-frame alias writes preserve branch output and produce no stale I230. | Verified baseline |
| #2323 | Catch followed by its result guard still reports W210 for the successfully assigned file handle. | Reproduced residual |
| #2324, #2325 | Computed typed receivers and expanded deferred callbacks still require focused verification. | Pending |
| #2367, #2370, #2410, #2412, #2414, #2415 | Reported fixed programs preserve original execution after optimisation; associated W211/W220 defects are absent. | Verified baseline |
| #2455 | The nested expression read produces no W220. | Verified baseline |
| #2407, #2408 | Reports describe conservative summary precision limits. | Pending |
| #1778 | The reported distinct colon-edge commands preserve their original distinct output under C Tcl 8.6–9.1 optimisation. Additional lifetime and index consumers require verification. | Partial verification |
| #1796 | ResolveContext selects exact frames, but UpvarInfo still collapses all beyond-caller distances. | Confirmed remaining foundation |
| #2332 | Qualified and variable-linked substitution reads are impure with global effects; explicit set reads remain conservatively impure with unknown effects. | Verified baseline |

The first build required fresh dependency downloads (`sha2` 0.11.1 was absent
from the cache). Network-enabled execution resolved the dependency limitation;
no dependency versions were changed.

Fresh compiler build completed successfully. The focused release audit has five
passing cases and four reproduced failures; existing name-lookup witnesses have
seven passes and one failure. Draft fixes are not counted as verified.

The expanded baseline uses the already-built isolated `961f17a32` compiler
artifacts. Nine fixed programs have paired original/optimised oracle evidence
across C Tcl 8.4–9.1 and JimTcl. Original and rewritten outcomes agree in every
row. C Tcl 8.4/8.5 reject the colon-tail declaration; JimTcl has distinct
namespace-frame/colon-name behaviour that the rewrite also preserves. These are
not interchangeable oracle expectations.

The first W210 rebuild after integration stopped in an existing registry build
error: `security_floor.rs` used a non-exported `crate::CommandForm` type. The
registry owner supplied `60f93a19d`; the diagnostic fixes await a retry.
`rust-check` first stopped on formatting in an uncommitted audit fixture
(subsequently formatted). `prep-pr` stopped while npm fetched dependencies
through the sandbox-restricted proxy. These are failed gates, not green checks.
