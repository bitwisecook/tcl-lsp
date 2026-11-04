# Current shared-source revalidation

Production source `ad4d43160916c343e8376763cfbefba9fc184ad5` plus the unchanged
IR-stage patch `e663f5806d5f0772448501c8c3ae5ad27dcabd5eda2a68de27c6dec44bd34440`
and final regression source
`dee5210db7ffec7d72d9ab4119079f5c0557db6f83cce4edab47c18658aaec67`.
The production compiler rlib SHA-256 is
`d4012741853e0713f64960e6b46711abf7668a50abe74092235b4bb51251de66`.

Production Clippy passes in 43.01 seconds. The two integration-only no-run
targets pass in 170.07 seconds; the compiler unit-test target is not rebuilt.
The exact diagnostic matrix passes all three tests: 100 functional verdicts
across twenty sources and five Tcl profiles, plus the corrected stage and
distinct-sentinel control. The existing short-circuit target passes both
tests and all 185 source/admission rows. Runtime uses emitted binaries outside
the compiler lock. All 15,227 source files, both executable hashes and all 145
production dependency files remain byte-identical through runtime and the
read-only Jim owner probe. Full workspace gates remain separate.

The Jim combined procedure loop has no original target: runtime Conditional,
unknown true, targets zero, ordinary call and opaque inventory containing x.
The same Tcl 8.6 position retains one closed Registry target and typed loop
stages. Jim's standalone procedure also retains the closed target and typed
stages. Both inputs have no Logical declaration compatibility and no Native
entry. The stage repair does not manufacture that missing Jim binding. No
source-purpose, callee-availability or Native gate is widened, and #2441 stays
open for the remaining Jim-profile combined-program gap. These observations
use public APIs; private metadata calls were removed rather than exposed.
