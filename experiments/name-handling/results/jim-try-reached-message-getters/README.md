# Original Jim Try message after reached getters

Question: when original clause and trap String updaters publish another result, which message does the selected Try handler receive?

The original SDK probe creates actual custom `Jim_ObjType` String updaters, actual List pattern/member headers, and an actual `Jim_EvalObjVector` argument vector. Its `body_error` callback sets a genuine global List errorCode and publishes BODY. Controlled updaters produce the same selected clause/pattern bytes as their resident controls, optionally publishing CLAUSE or TRAP. No observation reference is added by the getter trace. Type, actual selected-header role, native reference count, and the resident result before/after are recorded at each reached updater. Reference observations are bounded to this configured probe and its explicit original caller/vector roles.

`capture01` contains nine completed cases. `capture02` repeats those cases with before/after updater traces; every OBS row is byte-identical. `capture03` retains those nine OBS rows and adds interleaving and unselected-body controls. `capture04` adds actual original Int header controls for unselected script and longer trap; `capture05` adds an actual original Int member in a later trap after an earlier match. `capture06` adds actual message-name, options-name and first variable-list getter controls. `capture07` repeats all 17 OBS rows with explicit actual header-role labels. All seven compile/run exit 0. Every capture freshly rechecks upstream against the local original revision, records the exact compiler/link invocation, binary/libraries/source/configuration hashes and raw streams, and proves all 247 selected provider inputs unchanged. The original namespace suite with original testing.tcl/tcltest.tcl passes 79/79 each time. No Runtime or VM interpreter is executed by these captures.

OBS fields: mode, raw exit, updater count, final result, message, handled, other. Bytes are hexadecimal; dash means the original API returned NULL. Modes:

- 0: resident `on error` control; BODY is bound.
- 1–2: actual clause String updater, unchanged/publishing CLAUSE.
- 3: resident trap pattern control.
- 4–5: actual trap-member String updater, unchanged/publishing TRAP.
- 6: clause then trap-member updaters; TRAP is bound.
- 7: an earlier `on error` already matches, then a later trap updater publishes TRAP; the first handler still receives TRAP.
- 8: longer trap pattern; its member updater is not reached, no handler matches, BODY remains.
- 9: a trap updater publishes TRAP, then a later clause updater publishes CLAUSE; the first handler receives CLAUSE.
- 14: selected message-name updater publishes MSGNAME at its reached length getter; MSGNAME is bound.
- 15: selected options-name updater publishes OPTNAME after the BODY message is already bound; the handler still receives BODY.
- 16: first selected variable-list updater publishes VARS before a later trap publishes TRAP; the originally selected handler receives TRAP.
- 11: same unselected handler control with a genuine original Int header; it remains stringless and Int.
- 12: longer pattern with an original Int member; it remains stringless and Int.
- 13: earlier matching on clause followed by trap with an original Int member; that later member gets String materialized and becomes String primary.
- 10: two matching on clauses; the second handler's custom String updater is never reached. Its original header remains stringless with its original custom type.

These observations require visiting validation/comparison stages in input order while retaining the first match. The message is read from the actual interpreter result after the reached message-name length getter. They establish no arbitrary custom ObjType implementation in Rust. Rust controls using an actual result publication and the existing private clause issuer are software stage controls, not a donation of the SDK's custom-type capability. Full callback, signal, ignored-code, allocation, and native-reference parity remains outside this capture.

Run `python3 verify.py` for a hash-only retained-artifact check; it never executes providers. `SHA256SUMS` seals the retained package.

The VM controls consume capture07. Three resident original-Int cells preserve actual later/unreached getter and primary-type assertions. Three separately named software-stage controls use genuine clause/captured-name issuers and actual result/shimmer/context owners: they discriminate a stale result, late name-header reselection, and foreign caller acceptance. They do not emulate the original custom ObjType updater or grant that capability. All Rust controls remain unexecuted until the root validation lease runs them. Shared equality's internal getters retain their existing generic currentness; this tranche adds surrounding caller/header/backing checks and nested caller checks to List conversion, and does not establish every equality callback stage.
