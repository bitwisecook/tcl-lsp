# Original Jim namespace Code helper getters and identity

[probe.c](probe.c) executes sixteen fixed original Jim cases through
`Jim_EvalNamespace` with an original List body, genuine integer42 namespace and
actual public `namespace code` or compound `namespace code` helper command.
[stdout.tsv](stdout.tsv) retains217 exact rows: one version row and32 cold/hot
phases. Each pair keeps the same command-head object/cache and original
namespace; the hot phase invalidates only that namespace String. Each phase
receives a fresh original custom argument and List body. The argument updater
observes namespace bytes before supplying its own valid owned String. AFTER
header samples precede every result/String/List getter.

Public and direct-cold lookup materialise the caller namespace before the
argument getter. Direct-hot literal cache can leave it stringless: an already
scoped argument returns that same original argument, including counted NUL data,
while the plain branch later reaches caller namespace current and produces an
actual four-member List whose last child is the incoming original argument.
The String-first operation changes the custom argument primary to String;
the probe preserves that genuine change. Wrong public/direct arity, actual
helper replacement and actual helper deletion retain their separately measured
getter, completion and original-header observations. These results do not prove
another scripted namespace helper or general command/cache behavior.

The accepted [manifest](manifest.json), [capture driver](capture.py), source,
outputs and sixth provider source [nshelper.tcl](nshelper.tcl) are unchanged
original bytes. Five earlier exact provider snapshots and the independently
retained helper source are referenced by relative paths in the
[archive receipt](archive-receipt.json). They are selected source/header/build
inputs, not a complete Jim SDK/extensions/system-library build closure. The
original executable/static library are hash-verified and excluded from this
nonbinary archive. Original absolute capture paths remain provenance; the
capture driver depends on that original environment and is not a portable SDK
reconstruction.

The independent [upstream receipt](upstream-check-0312/receipt.json) records the
actual03:13:03UTC read-only remote/local HEAD observation matching
`5bac7c99ad65864c87da513e22e2f01703fa4e03`. It supplies freshness at its retained
time and does not update/rebuild the provider or establish a timeless latest
claim.

Two excluded trials remain exact evidence assets. [Attempt1](attempt1/trial.json)
failed compilation because it used the wrong `Jim_ListLength` signature and
never executed. [Attempt2](attempt2/trial.json) invalidated authoritative String
bytes after String-first changed the argument primary; its process-success
assertion failed, and the exact process exit was not retained. Its empty output
is not accepted hot-behavior evidence. The accepted probe instead uses a fresh
argument/List for each phase. These preserved inputs and failure records are
excluded from the217-row semantic result.

Run `python3 verify.py --repository-root /path/to/tcl-lsp` to check exact retained
bytes and six referenced snapshots. `--current-provider` additionally checks
original local library/provider bytes when available. Runtime and VM execution,
C Tcl/F5 agreement, final-free/coroutine/trace chronology, full lifecycle and
full Native conformance remain independent.
