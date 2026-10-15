Original Jim Catch/Try output-name getter stages
==============================================

Question: which actual interpreter result is assigned after the message-name
String getter, and can an empty options name leave a retired earlier error code
unused?

Capture01 freshly rechecked upstream at 16:59 UTC on 2026-10-15 against clean
local 5bac7c99ad65864c87da513e22e2f01703fa4e03. Its pre-hashed driver compiled
the exact C probe against that provider's original headers/libjim.a. Compile
and execution both exited 0 without stderr. All 247 source/configuration/library/
executable/harness inputs remained unchanged. The original namespace suite
passed 79/79 with the same revision's original testing.tcl/tcltest.tcl. Manifests
retain compiler, executable and linked-library hashes, invocation, timing and
raw channels. No Runtime or VM implementation supplied an expected answer.

The 20 raw rows cover four actual object-vector cases for each Catch and Try:
resident names; an actual custom message-name updater publishing MSGNAME; an
actual options-name updater publishing OPTNAME; and an updater returning an
empty options name while replacing the global errorCode. The latter retires the
earlier borrowed captured code, which original execution never accesses because
the options name is empty. Getter traces inspect actual original header native
counts without adding observation references.

Both commands assign MSGNAME after the reached message-name getter. An options
name getter occurs after the BODY message store; Try's handler therefore returns
BODY. Empty options names skip options publication, including when the captured
error code has retired. The shared original Try capture07 additionally retains
the clause/trap/first-variable-list ordering controls; this package leaves its
sealed 144-file closure unchanged.

The Runtime control uses its genuine extension descriptor/updater and existing
physical String/assignment owners. It checks the observed completion, result,
stored message, handler result, getter count and options publication. This is a
software extension-stage control, not equivalence of the C-extension ABI to the
Jim ObjType ABI. Native counts, first-match member reference roles, arbitrary
callbacks, compiled Catch/Try and full result identity remain separate coverage.
Rust tests remain unexecuted. The rebuildable probe is excluded; its captured
hash is checked when an executable is present.
