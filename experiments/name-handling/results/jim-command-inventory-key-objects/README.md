# Original Jim command inventory key objects

The original C SDK ran 24 controls at 2026-10-15 12:17 UTC. The execution-time
upstream HEAD was `5bac7c99ad65864c87da513e22e2f01703fa4e03`, equal to the
source-built provider. Compilation and execution completed with exit 0 and empty
stderr. All recorded provider source, configuration, library, and harness inputs
were unchanged. The original `jimsh tests/namespace.test` also completed 79/79.

The C observer obtains an actual command hash key before making that same key an
integer with no resident String. It executes original `info -nons` for commands,
procs, and aliases, with and without `-all`, and with no pattern, an exact match,
an unrelated exact match, or a glob. It records the actual result member's pointer
equality to the retained key, key primary/String state and native reference counts
before, after, and after releasing the result. The alias setup leaves its original
name in the result slot; the next invocation withdraws that existing role before
publishing its List. That explains its distinct initial count.

The accepted cells show that `-all` without a pattern leaves the key String absent,
an unrelated exact pattern leaves it absent, and selected result Lists own the
actual key. Ordinary enumeration or a matching pattern reaches its String getter.
The exact complete original SDK roster sizes are retained as observations, not
used to assert that another host has the same registered command set.

`capture01` is immutable raw evidence. `capture.py` retains exact invocations,
timestamps, stdout/stderr, completion, source/executable hashes, linked libraries,
provider build inputs, and original harness pairing. It uses the original SDK's
`jim.h` and `libjim.a`; no replacement Jim or tcltest implementation is used.
The compiled observer can be rebuilt from its retained source; its measured hash
is retained even when that local generated binary is unavailable.

Run `python3 verify.py` to verify the archive and the exact Runtime fixture.
`--current-provider` additionally checks every still-provisioned original SDK
input. These observations supply no Runtime/VM execution acceptance, full table
iterator or repeated pattern-getter proof, other Tcl/F5 agreement, or Import
traversal/normal-completion authority.
