Original Jim error-code capture before output assignment
========================================================

Question: when catch/try writes its result to the global errorCode variable,
does its options list use the earlier global value or reread the new value?

Captures01/02 rechecked upstream at execution time on 2026-10-15 16:01 and 16:14 UTC. The
upstream HEAD matched clean local 5bac7c99ad65864c87da513e22e2f01703fa4e03.
The exact pre-hashed C input/driver compiled against the original headers and
libjim.a. All four compilation/execution stages exited 0 with no stderr. All 247 selected
provider files were unchanged; source, executable, compiler, configuration and
linked-library hashes, invocations, completion and channels are retained.

The probe uses original core commands plus original static extensions, matching
the provider's InitStaticExtensions path. Each separately executed original
tests/namespace.test passed 79/79 using that revision's original testing.tcl and
tcltest.tcl. No substitute Tcl harness is used.

Both catch and try publish BOOM to global errorCode but put the earlier SAVED
object into their options list. That member is the exact original header also
held by the keep variable, and is distinct from the new global value. The keep
reference is deliberate: the original C implementation borrows its captured
error-code pointer without retaining it, so the probe must preserve its lifetime
instead of inspecting a dangling original pointer after replacement.

Capture02 additionally observes successful bodies whose overwritten old error-code
object has no keep reference. Their options omit error metadata, so the original
never reads the now-retired captured pointer. This is a positive control for
reaching only the metadata selected by the raw exit code. Capture01 is retained
unchanged, and its six rows are the exact prefix of capture02.

Fixtures are exact capture02 primary stdout and both exact body literals extracted
from the C input. Reference-count traces, every options-constructor allocation stage,
unretained dangling-pointer behaviour, callbacks and compiled paths remain
outside this capture. Rebuildable executable bytes are excluded, with their
captured hash checked when present. Rust controls remain unexecuted.
