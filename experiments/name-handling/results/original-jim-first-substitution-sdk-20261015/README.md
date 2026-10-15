# Original Jim first-substitution and namespace materialisation

This fixed ten-case SDK capture observes the original full-extension
Jim 0.84-9-g5bac7c9 build identified by the exact library and provider hashes in
[the unchanged capture manifest](manifest.json). The revision label comes from
the earlier capture; this run reverified those bytes and made no fresh upstream
revision query. The probe uses genuine original list children with an integer
namespace object whose value is 42.

[probe.c](probe.c) records the namespace object's `bytes` presence and `typePtr`
before `apply`, immediately after it, and only then calls a string getter.
[stdout.tsv](stdout.tsv) retains all 41 original rows: one version row plus
four rows for each of ten cases. Four missing-variable substitutions in the
first word or an argument fail while the namespace object's bytes remain null.
Three successful command cases materialise the bytes. A bracketed `set` reaches
command lookup and materialises the bytes before its missing-variable error.
Comment-only and invalid-brace controls leave the bytes null. The later getter
returns `42` in every case; it does not establish an earlier materialisation.
The table's result payloads are hex-encoded bytes; `BEFORE`/`AFTER` instead
report byte-pointer presence and the original type name.

The seven original nonbinary capture files are copied byte for byte, including
the three empty channel files. The capture manifest and runner retain their
original absolute paths as provenance. The runner also depends on the previous
local capture directory and is not a portable SDK reconstruction. The original
executable and static library were hash-checked during archival but are not
included. Five previously retained provider snapshots are referenced by exact
hash through the [earlier SDK archive](../original-jim-lambda-context-sdk-20261014/README.md);
these selected source/header/build files are not the complete SDK or extension
build closure.

Run `python3 verify.py --repository-root /path/to/tcl-lsp` to check retained
files and referenced snapshots without an SDK or executable.
`--current-provider` additionally checks the original manifest's local provider
paths and library when available. The [archive receipt](archive-receipt.json)
records the portable path map without rewriting the capture manifest.

This observation covers these ten original Jim cases only. It does not accept a
Runtime or VM implementation, establish C Tcl/F5 agreement, cover Jim coroutines
or other unmeasured evaluation paths, or close a full conformance suite.
