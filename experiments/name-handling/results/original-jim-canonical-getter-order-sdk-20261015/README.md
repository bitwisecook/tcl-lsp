# Original direct Jim canonical namespace getter order

[probe.c](probe.c) directly calls the unchanged original `JimCanonicalNamespace`
API in three fixed cases. [stdout.tsv](stdout.tsv) retains25 rows: one version
row and eight rows per case. A genuine custom original name type updater
observes the original namespace header before supplying its String. Namespace
and name AFTER samples precede the result getter.

For ABSOLUTE with an original integer namespace42, namespace bytes remain null
at the name getter and afterward; the result is a distinct CString-extent
`target` object. For RELATIVE, the namespace is still stringless at the name
getter, then materialises42; the distinct result preserves counted bytes
`42::child\0tail`, including the language NUL and suffix. ROOT_RELATIVE uses
the genuine empty namespace object, already resident in this control, and
returns the same original name pointer with counted `child\0tail` bytes.
Explicit namespace getter rows occur later. RESULT_IDENTITY payload31 encodes
`1`, payload30 encodes`0`; other result payloads are hex bytes, while header
samples report byte-pointer presence and original type names.

This is a direct API experiment, separate from public `namespace which` or
helper command dispatch. In particular, it does not establish that an absolute
public command operand can avoid an earlier command-head namespace getter. The
[separate public/helper capture](../original-jim-which-helper-getter-sdk-20261015/README.md)
measures that independent reached-operation boundary.

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
