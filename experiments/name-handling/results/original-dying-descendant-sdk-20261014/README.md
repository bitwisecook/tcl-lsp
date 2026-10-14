# Original C descendant publication during namespace teardown

These SDK observations execute the genuine C Tcl 8.4.20, 8.5.19, 8.6.18,
9.0.4 and 9.1.0 interpreters identified in [manifest.json](manifest.json).
Each independent interpreter deletes `::retiring`; an actual unset callback
enters a missing child through the original `namespace eval` command, creates
a scalar and a procedure, reads both, and reenters the child.

The original C namespace pointer recorded before deletion is compared with the
created child's actual parent chain. All four cases and five providers retain
that original ancestor, including a deeper descendant and a trailing-separator
operand. The native `Tcl_FindNamespace` API still returns the ancestor during
the callback. The separate public `namespace exists` query reports the parent
as present in C8.4 and absent in C8.5 through the audited C9.1 release; each new
child remains present and its scalar/procedure remain usable. Reentry selects
the same physical child. After the outer deletion finishes, the parent, child,
scalar and procedure are all absent.

`EXISTING_CHILD` starts with a same-named child before deletion. This probe
records its original ancestor and the post-create child's reentry identity.
It does not retain the original child's pointer, so it does not establish
whether that earlier child allocation was reused or replaced.

[analysis.json](analysis.json) keeps decoded case observations alongside the
unchanged per-provider stdout/stderr/compiler output and original receipts.
[archive-receipt.json](archive-receipt.json) identifies exact retained bytes and
provider source snapshots. Original static-library and executable hashes are
verified; binaries are not archived. Captured `/tmp` paths are historical
provenance. The retained source snapshots and library/init hashes identify each
actual provider independently of a command label or an implementation run.

For C8.4 the probe includes its original `tclInt.h`, which declares the actual
then-internal `Tcl_FindNamespace` API; later releases use the public header.
Reproduce against each original SDK library and its original Tcl library:

```sh
cc -std=c99 -I/path/to/tcl/generic -I/path/to/tcl/unix probe.c /path/to/tcl/unix/libtcl8.6.a -lm -ldl -lpthread -lz -o /tmp/dying-descendant-probe
TCL_LIBRARY=/path/to/tcl/library /tmp/dying-descendant-probe
```

This evidence concerns physical publication and cleanup under an actual dying
ancestor. It does not establish Runtime or VM acceptance, Jim semantics,
callback-owned direct C namespace creation, F5 behaviour or full-suite
conformance. The existing final-target trailing-parent rule is a distinct case.
