# Original C descendant publication in unset and command-delete callbacks

These SDK captures run the genuine C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and
9.1.0 providers identified in the unchanged [manifest](manifest.json).
Each provider has eight independent interpreter cases: four actual variable-unset
callbacks and four actual command `deleteProc` callbacks during deletion of
`::retiring`. The original probe records 105 stdout rows per provider. Every
case reaches its callback exactly once, creates/enters the selected descendant,
reads its scalar and procedure, and reenters the same post-create child.

The four targets are an absent child, its trailing-separator operand, a deeper
descendant and a name which had a child before deletion. The child's measured
parent chain contains the actual original ancestor pointer in every case.
`Tcl_FindNamespace` still finds that parent during the callback. The separate
public `namespace exists` query reports the parent present in C8.4 and absent
in C8.5–9.1. The new child remains queryable and its scalar/procedure usable.
After outer deletion, parent, child, scalar and procedure are absent.

`EXISTING_CHILD` does not retain the earlier child's pointer. It measures the
original ancestor and the post-create child's reentry identity, so it does not
establish reuse or replacement of that earlier child allocation.

[analysis.json](analysis.json) decodes the unchanged raw per-provider stdout.
The original source, capture runner, manifest, receipts, stdout/stderr and
compiler outputs are copied byte-exact; [archive-receipt.json](archive-receipt.json)
records their original paths and hashes. Original libraries and executable
hashes were verified at archival time, but binaries are excluded.

The [earlier unset-only archive's provider snapshots](../original-dying-descendant-sdk-20261014/archive-receipt.json)
are reused by exact path/hash reference: 35 original namespace/variable/header,
init and build-file snapshots across the five providers. Every reference is
verified against both that archive and its identified original file. These
snapshots identify the selected source basis; they are not a complete SDK or
rebuild closure. No older archive or receipt is rewritten. In C8.4, `probe.c`
includes original `tclInt.h` for the then-internal `Tcl_FindNamespace` declaration.

Reproduce against each matching original SDK and Tcl library:

```sh
cc -std=c99 -I/path/to/tcl/generic -I/path/to/tcl/unix probe.c /path/to/tcl/unix/libtcl8.6.a -lm -ldl -lpthread -lz -o /tmp/dying-descendant-command-probe
TCL_LIBRARY=/path/to/tcl/library /tmp/dying-descendant-command-probe
```

`verify.py` checks retained and referenced archive files without requiring the
historical `/tmp` paths. The unchanged manifests preserve those paths as capture
provenance. This record establishes only its original SDK observations, not a
Runtime/VM implementation result, Jim or F5 behaviour, other callback shapes or
full-suite conformance. The dying final-target trailing-parent return rule is
independent of physical descendant publication and teardown.
