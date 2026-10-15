# Original Jim pending tailcall cancellation and selected evaluation

[probe.c](probe.c) executes two fixed original Jim `apply` cases with a genuine
native selected command and `deleteProc`. [stdout.tsv](stdout.tsv) retains16
original rows. The original integer84 payload header is sampled before and
after evaluation, before any result String getter, and remains stringless.

SELECTED_EXECUTED uses `defer {rename ::selected {}}; tailcall $target $payload`.
The genuine worker records invoked1 while deleted0, then the deletion callback
records deleted1/invoked1. At deletion the actual current frame pending List is
null. The result is code0 and bytes `VALUE`.

PENDING_CANCELLED adds `error CANCELLED` to that real defer body. The selected
command deletion callback runs exactly once without worker invocation. At that
callback the actual current frame pending List is nonnull with two elements,
and its second member is the genuine unchanged payload pointer. The retained
result is code1 with bytes `CANCELLED`. PENDING_MEMBER_AT_DELETE's last field
is the observed payload reference count, not its final retirement. Header,
callback and List observations retain their exact original decimal columns;
RESULT payloads are hex bytes.

These controls distinguish an uninvoked pending command retiring before its
still-held List from a consumed evaluation's List having already left the
frame when the command retires. They do not prove argument final-free order,
all reference/cache/alias/coroutine cases, a Runtime public `defer` or trace
implementation, full lifecycle conformance, or Runtime/VM acceptance.

The seven capture files and all eight `upstream-check` files are exact copies.
The unchanged [original manifest](manifest.json) retains its original paths,
source/library/executable hashes and its own no-fresh-query scope. The later
[network receipt](upstream-check/network-receipt.json) records an independent
actual HEAD query from01:56:05.905387 to01:56:06.319137 UTC: the returned
5bac7c99ad65864c87da513e22e2f01703fa4e03 matches the recorded local commit
and earlier HEAD. That proves only the checked time; it does not rebuild the
SDK or establish later upstream state. The preceding failed proxy query and
successful local commit check remain in their unchanged original
[receipt](upstream-check/receipt.json), separately from the successful query.

Five exact provider snapshots are referenced from the [earlier original SDK
archive](../original-jim-lambda-context-sdk-20261014/README.md). The executable
and static library hashes were reverified but binaries are excluded. These
selected snapshots are not a full SDK, extension, compiler or external-library
build closure, and the original capture runner's local paths remain unchanged.

Run `python3 verify.py --repository-root /path/to/tcl-lsp` for retained bytes
and provider references. `--current-provider` additionally verifies the local
source/library paths when available. The [archive receipt](archive-receipt.json)
adds portable references without rewriting original evidence. No C Tcl/F5,
full SDK or unexecuted software agreement follows from this transport.
