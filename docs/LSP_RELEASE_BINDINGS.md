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
| #2243, #2247, #2286, #2290, #2297 | Reports loaded; current implementation requires verification. | Pending |
| #2305, #2306, #2323, #2324, #2325 | Reports loaded; structured name/frame implementation requires verification. | Pending |
| #2367, #2370, #2410, #2412, #2414, #2415, #2455 | Reports loaded; current interprocedural read/write projection requires verification. | Pending |
| #2407, #2408 | Reports describe conservative summary precision limits. | Pending |
| #1778, #1796 | Shared structured name and frame owners already exist; remaining consumers and composed distances require verification. | Pending |
| #2332 | Full issue fetched: external variable-substitution reads must make procedure summaries impure. | Pending |

The first build required fresh dependency downloads (`sha2` 0.11.1 was absent
from the cache). Network-enabled execution resolved the dependency limitation;
no dependency versions were changed.

Fresh compiler build completed successfully. The focused release audit has five
passing cases and four reproduced failures; existing name-lookup witnesses have
seven passes and one failure. Draft fixes are not counted as verified.
