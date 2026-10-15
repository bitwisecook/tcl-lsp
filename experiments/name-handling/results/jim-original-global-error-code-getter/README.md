Original quiet global error-code getter
======================================

Question: what does Jim_GetGlobalVariableStr(errorCode, JIM_NONE) return for a
missing global, a stringless scalar, a real variable alias, and a failed linked
dictionary-element read, when called from an actual namespaced procedure?

The original C probe ran on 2026-10-15 at 15:30 UTC against clean Jim revision
`5bac7c99ad65864c87da513e22e2f01703fa4e03`, matching upstream HEAD rechecked at
execution. Compilation and execution completed with exit 0 and no stderr. All
247 selected provider inputs were unchanged. The manifest retains source,
configuration, executable, compiler and linked-library hashes, exact invocations,
stdout/stderr and completion. The original namespace suite passed 79/79 with its
original testing.tcl and tcltest.tcl; the C API probe uses no tcltest harness.

Missing reads return NULL and preserve the prior MARKER result. A scalar and a
real alias return the same original stringless integer header and preserve MARKER.
A linked odd-dictionary element returns NULL with the observed guest diagnostic.
All cases restore the exact actual caller frame/namespace and preserve the same
local errorCode value. NULL is a failed quiet read, not a variable-absence proof.

Fixtures copy the exact stdout and C script literals. Rust controls compare the
recorded value-header/type/String/result cells and actual caller identity; they
also check the return-receipt consumer retains that actual global value. Rust
execution and broader cache/lifecycle acceptance remain separate. Native reference
counts and missing physical-operation traces are outside this capture.

`verify.py --current-provider` checks the sealed inputs/current provider without
executing an interpreter. The rebuildable executable is excluded; its captured
hash is retained and checked if the executable is present.
