# Original Jim tailcall selection and reached namespace getters

[probe.c](probe.c) executes four actual original Jim `apply` invocations using a
genuine integer namespace42. [stdout.tsv](stdout.tsv) preserves all19 original
rows, including the version row, native worker/deletion callback events and
completion bytes. AFTER namespace-header samples precede result String getters.

The cold/hot pair uses the same original lambda, body and `::return` target.
Only the integer namespace String is invalidated before the hot call. Cold
AFTER has resident namespace bytes; hot AFTER remains stringless. Both calls
return completion code2 and bytes `VALUE`. This is the observed direct original
API completion, not a converted Normal result.

DEFERRED_DELETE registers a genuine native `selected` worker with a real
`deleteProc`, then runs `defer {rename ::selected {}}; tailcall $target VALUE`.
The actual worker logs deleted0/invoked1. The actual deletion callback follows
with deleted1/invoked1, and the final completion is code0/`VALUE`. Thus this
control observes the selected original worker surviving issuer cleanup until
its invocation; a later spelling lookup alone cannot explain that observation.
The missing-target control completes with code1 and the retained original error
bytes. COUNTS/INVOKE/DELETE fields are decimal counters, result payloads are hex
bytes, and namespace samples report byte-pointer presence and type name.

This is a bounded selected-role and getter-order observation. It does not
establish every command-cache, replacement, alias, coroutine, allocation or
callback lifecycle case, nor accept a Runtime/VM tailcall implementation.

Seven original nonbinary files are copied byte for byte, including three empty
channels. The unchanged manifest retains its original local paths and exact
Jim0.84-9-g5bac7c9 source/library/executable identities. The revision label is
from the earlier capture; this run reverified those bytes and made no fresh
upstream query. The executable and static library were verified but are excluded.
Five exact provider snapshots are referenced from the
[earlier original SDK archive](../original-jim-lambda-context-sdk-20261014/README.md).
These selected files are not a complete SDK/extension or external-library build
closure; the capture runner's original absolute prior-capture dependency is not
a portable SDK reconstruction.

Run `python3 verify.py --repository-root /path/to/tcl-lsp` to verify the retained
files and five referenced snapshots. `--current-provider` additionally verifies
original local provider/library paths if present. The
[archive receipt](archive-receipt.json) supplies portable relative references
without rewriting original capture bytes. No C Tcl/F5 agreement, full SDK,
full lifecycle, Runtime/VM or full-suite acceptance follows from these controls.
