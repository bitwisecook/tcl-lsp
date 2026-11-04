# Reviewed empty-loop stage repair: bounded receipt

The narrow production patch excludes argument evaluation and proved-name
bookkeeping from the opaque-callee argument inventory. These IR stages retain
word and SSA effects, but do not dispatch the source command's handler. The
callee resolver, source-purpose gates and Native admission are unchanged.

The tested production snapshot is `f2b927c5ce08b5b8da4df2ba2a52c022391b99a4`
plus tracked patch SHA-256
`e663f5806d5f0772448501c8c3ae5ad27dcabd5eda2a68de27c6dec44bd34440`.
Its production compiler rlib is
`5ebe921aece1516754f0502a7746c6435cf237bb207351d7b34bb1bc78d27a8e`.
The final test source is
`dee5210db7ffec7d72d9ab4119079f5c0557db6f83cce4edab47c18658aaec67`.
`candidate.patch` includes that exact public regression source.

| Check | Observed result |
| --- | --- |
| Production Clippy | Pass; 62.01 seconds on first complete-input run |
| Combined unit and two integration no-run | Pass; 146.07 seconds after exact fixture restoration |
| C Tcl-profile diagnostic matrix | All 100 exact verdicts pass, across 20 fixed inputs and Tcl 8.4–9.1 |
| Existing short-circuit matrix | Both tests pass; 185 source/admission rows |
| Existing callee-owner controls | Three pass |
| Read-local guard controls | Seven tests pass; fourteen cases |
| Existence-query owner controls | Nine pass |
| Final distinct-sentinel CFG control | Pass on the exact unchanged production rlibs |
| Jim exact combined report | Still no W210; standalone-procedure and top-level controls both warn |

The exact reported C-profile input now warns on procedure `x` at 79..81 and
top-level `y` at 105..107. Safe guarded reads, existing/formal bindings, aliases,
real unknown calls and nested writers stay quiet. Unsafe guards and later real
reads retain their exact source spans. The complete fixed sources and all 100
actual/expected verdicts remain in the final test and compressed runtime log.

The extra CFG fixture initially demanded an iterator binding after an unknown
call. Actual producer inspection shows argument evaluation followed by a
conservative residual wrapper instead. The corrected fixture requires that
shape, keeps argument evaluation, and refuses an invented iteration binding.
Its real opaque call uses `sentinel`, distinct from loop binder `x`; sentinel
survives both callbacks. With the always-resolved callback, wrapper `x` is
correctly absent. All prior failed fixture results are retained. The final
filter was linked directly against the exact production libraries without
Cargo or target writes; all 138 library hashes remain unchanged. The first
100 functional inputs/assertions are byte-identical across fixture revisions.

The complete source and binary freeze closes unchanged. The old 138 rlibs,
eight proc-macro libraries and two native archives are retained as one verified
213,842,676-byte compressed archive, with all member hashes in the ledger.
Sparse cleanup required restoring exact compile inputs: two production text
fixtures, 76 test-support captures and two Rust include sources. Those missing
input failures are retained and are not counted as product failures or passes.

Current shared compiler source later changed materially. Its independent run
at `804e4a282a48e3ad3e2b4f4f9a120b0cf8246723` stops at an unchanged shared
102/100-line lint in `retained_source_transition_value_advice`; no current
integration build ran yet. These snapshot results are not current-head
validation. The coordinator owns that lint repair and the next revalidation.
Jim's combined-program coverage gap remains open; no #2441 closure is claimed.
Native Tcl execution evidence is recorded separately under `evidence/tcl`.
