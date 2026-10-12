# Retained authored-read binding

Production change in script_binds.rs forwards the same realm-issued retained binding to the already verified complete reconstructed original command vector. The syntax-only producer is unchanged; all factory/source/currentness/formal guards remain.

Owner regression passes1/1, proving unbound refusal, same-binding acceptance, changed-vector refusal and existing stale source/formal/input/context refusals. The Tcltest filter executes once:6pass/3fail. Direct/alias/original-program genuine-missing-read controls still fail firstC8.4 with actual0 versus expected1/1/2. Packaged2 is not executed because its prescribed green-filter precondition is unmet. No W210 fix or conformance/full-gate pass is claimed.

Source-before/frozen/after and complete before/frozen script_binds bytes are retained. After equals frozen; only the deliberate script_binds production/regression change differs from before. Shared special_vars matches the root frozen hash. Next existing factory guard in Native fixtures remains unmeasured; no additional speculative trace was run.
