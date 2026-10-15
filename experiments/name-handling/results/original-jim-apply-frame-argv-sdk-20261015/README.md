# Original Jim apply frame argv and lazy object getters

This fixed seven-case SDK capture observes the original full-extension
Jim 0.84-9-g5bac7c9 build identified by the exact library and provider hashes in
[the unchanged capture manifest](manifest.json). The revision label comes from
the earlier capture; this run reverified those bytes and made no fresh upstream
revision query. The probe uses a genuine original list child holding integer
namespace 42 and a separate original integer argument 84.

[probe.c](probe.c) records both objects' `bytes` presence and `typePtr` before
`apply` and immediately afterward, before inspecting the result or invoking a
result string getter. [stdout.tsv](stdout.tsv) preserves all 57 original rows:
one version row and eight rows for each of seven cases. COMMENT, INVALID,
MISSING, WRONG_ARITY and EMPTY leave both original objects stringless. RETURN
and LEVEL reach namespace access and materialise its bytes, while the original
argument's bytes remain null. Later explicit getter rows return `42` and `84`;
they do not establish earlier materialisation.

Before any result string getter, RETURN's actual result pointer and LEVEL's
actual result List member at index 1 each equal the original argument pointer.
The LEVEL result later renders as `{apply lambdaExpr} 84`. RESULT_IDENTITY
payload `31` is hex-encoded `1`; `30` is hex-encoded `0`. The other result and
getter payloads are hex-encoded bytes; BEFORE/AFTER rows instead report the
original byte-pointer presence and type name. These observations distinguish
actual argv member transport from materialising the lambda or argument merely
to construct a frame label.

Seven original nonbinary files are copied byte for byte, including three empty
channel files. The capture manifest and runner retain their original absolute
paths as provenance. The runner depends on the earlier local capture and is
not a portable SDK reconstruction. The original executable and static library
were hash-checked during archival but are not included. Five previously
retained provider snapshots are referenced by exact hash through the
[earlier SDK archive](../original-jim-lambda-context-sdk-20261014/README.md);
these selected source/header/build files are not the complete SDK or extension
build closure.

Run `python3 verify.py --repository-root /path/to/tcl-lsp` to check retained
files and referenced snapshots without an SDK or executable.
`--current-provider` additionally checks the capture's original local provider
paths and library when available. The [archive receipt](archive-receipt.json)
records the portable path map without rewriting the capture manifest.

This observation covers these seven original Jim cases only. It does not accept
a Runtime or VM implementation, establish C Tcl/F5 agreement, cover Jim
coroutines or other unmeasured frame/lifecycle paths, or close a full suite.
