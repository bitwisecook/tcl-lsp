Original Import Source reuse and partial traversal
================================================

Question: does the installed original Import procedure complete across reuse,
alias replacement, namespace recreation and repeated patterns, and does an actual
alias publication followed by break establish complete inventory traversal?

`capture02` ran against original Jim revision
`5bac7c99ad65864c87da513e22e2f01703fa4e03`, rechecked against upstream HEAD at
execution on 2026-10-15 at 14:56 UTC. Compilation and execution completed with
exit 0 and no stderr. The original namespace suite passed 79/79 using the original
`tests/testing.tcl` and `tests/tcltest.tcl`. The C API probe uses no tcltest harness.
All 247 selected provider files were unchanged. The manifest records source,
configuration, executable, compiler and linked-library hashes and all invocation
channels and completion. `capture01` separately retains a failed network setup;
it did not compile or execute a semantic probe.

The same interpreter and installed procedure body are reused through changes to
the actual alias binding and source namespace. Calls return normally and observe
P, Q, then R. Repeating the same pattern also returns normally. These observations
do not independently prove any Runtime cache or frame receipt.

The break case replaces the actual alias slot with a native C wrapper. The wrapper
calls the saved actual original alias native implementation with unchanged argv,
then returns JIM_BREAK. The original Import returns normally after publishing p;
q remains unvisited. That partial success must not issue a full-traversal receipt.
A Runtime refusal of that receipt is a separate architecture control, not an
original Tcl error or a claim that original execution failed.

Runtime fixtures are exact script literals and stdout derived from `capture02`.
`verify.py --current-provider` checks the sealed package and current provider
hashes without executing an interpreter. The compiled probe is rebuildable and
excluded; its capture-time hash remains recorded and is checked when present.
Public Runtime and VM Import entry acceptance remains a separate obligation.
