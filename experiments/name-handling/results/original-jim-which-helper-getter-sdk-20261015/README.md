# Original Jim namespace which public and helper getter order

[probe.c](probe.c) invokes unchanged original `Jim_EvalNamespace` with an
original List body and genuine integer namespace42. The sixteen invocations
compare the public `namespace which` front door and the existing direct
`{namespace which}` helper head, command/variable query, absolute/relative name,
and cold/hot use of the same original List. Before the hot invocation the
original namespace and name Strings are invalidated. A genuine custom name
updater observes the actual caller namespace header before supplying payload
String bytes. AFTER samples precede every result getter.

[stdout.tsv](stdout.tsv) retains97 rows: one version row and six rows for each
of sixteen invocations. All eight public invocations and all four direct-helper
cold invocations observe resident namespace bytes at the payload name getter.
The four direct-helper hot invocations observe null namespace bytes there.
Their two absolute cases remain stringless afterward; their two relative cases
materialise the namespace afterward. Exact command/variable result bytes remain
in the raw rows; these header observations do not establish a live variable,
command allocation or general namespace availability.

The direct helper's hot/cold context is independent of the public front door.
An absolute canonical payload alone does not remove an earlier reached
command-head namespace getter. The [direct canonical API capture](../original-jim-canonical-getter-order-sdk-20261015/README.md)
has its own three-case scope and cannot substitute for these full helper/public
invocations. No Runtime optimisation is accepted by either capture.

Seven original nonbinary capture files are copied byte for byte, including the
three empty channels. The manifest and capture runner retain their original
absolute paths as provenance; their local prior-capture dependency is not a
portable SDK reconstruction. The original executable and static library were
hash-checked but are excluded. Five exact provider snapshots are reused from
the [earlier original SDK archive](../original-jim-lambda-context-sdk-20261014/README.md);
these selected source/header/build files are not the complete SDK/extension or
external-library build closure. The version label is the earlier observed
Jim0.84-9-g5bac7c9 build, reverified by source/library bytes; this capture made no
fresh upstream query. The [separate remote HEAD receipt](../original-jim-remote-head-recheck-20261015/README.md)
retains its own timestamp and does not rebuild this SDK.

Run `python3 verify.py --repository-root /path/to/tcl-lsp` to check the exact
retained files and five referenced snapshots. `--current-provider` also checks
the original local library/provider paths if available. The
[archive receipt](archive-receipt.json) supplies portable relative references
without rewriting original capture receipts.

These observations do not accept Runtime/VM implementation behaviour or establish
C Tcl/F5 agreement, allocation/lifecycle schedules, Jim coroutine behaviour, a
complete SDK build closure or a full conformance suite.
