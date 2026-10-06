# Native array default controls

`controls.tsv` retains the original 88 state and callback observations.
`usage.tsv` contains 16 original result-byte observations from Tcl 9.0.4 and
9.1.0; `usage-sources.json` and `usage-manifest.json` retain scripts and exact
executable hashes. Each row contains version, case, original source bytes in
hex and result bytes in hex.

Literal calls with an accepted outer arity use the selected private worker.
Dynamic calls use the real public ensemble rewrite. An ARRAY trace that
evaluates a callback clears that rewrite before the worker's narrower arity
check. The callback's own wrong-argument header belongs to its original command.
Outer arity failures retain the public ensemble usage.

Run each source in `usage-sources.json` on a fresh selected Tcl interpreter and
capture its final result with `binary encode hex`. No error globals are read.
The Runtime test `array_default_usage_matches_native_literal_dynamic_and_callback_entries`
consumes every row. The shared reset-order probe is maintained separately in
`rust/tcl-registry/tests/data/native_ensemble_rewrite`.
