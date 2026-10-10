# naming.namespace.jim-flat-command-context-source-controls

Kind: `native-observation`

## Problem statement

Jim flat command lookup and namespace helpers can share written names with C namespace-tree operations while requiring different ownership purposes. Guest completion must be separated from any internal root holder or epoch claim.

## Question

What completion and result does current Jim return for the exact original flat-namespace script combining canonical namespaces, command/procedure enumeration, imports, variable lookup, lexical variables and alias origin?

## Conclusion

Current Jim 0.84-9-g5bac7c9 returns catch code 0 and exactly {} n {child::q p} p {::n::child::q ::n::p} ::n::absent LOCAL INNER ::n::p. This single guest result answers only the fixed source composition; it does not establish internal root object pointers, header/cache state or allocation epochs.

## Scope

One original fixed ASCII LF source-file CLI process on the recorded current Jim executable. C Tcl and BIG-IP are not measured for this question. No raw-zero/opaque object-vector input, compilation admission, internal root context liveness or arbitrary namespace behavior is inferred. The original generic receipt input_channel label mentions scalar trace callbacks; the exact probe, command vector and boundary field establish this namespace-script channel instead.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust source-advice invariant.

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Actual executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; exact selected header, library, Makefile and source pins, environment and original process argv are retained in the provider receipt.. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process.. Dialect: Jim Tcl.

OBSERVATION catch code 0 returns exactly {} n {child::q p} p {::n::child::q ::n::p} ::n::absent LOCAL INNER ::n::p. Actual queried patchlevel is 0.84-9-g5bac7c9; outer process status is 0 and stderr is empty. The receipt input_channel text mentions scalar trace callbacks, but the retained probe and boundary field identify this flat namespace script; the original receipt bytes are unchanged.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native interpreter observation establishes this Rust source-advice invariant.

## Exact evidence

- `source` (input): [rust/tcl-registry/tests/data/native_jim_flat_namespace_original/probe.tcl](../../../../rust/tcl-registry/tests/data/native_jim_flat_namespace_original/probe.tcl). SHA-256 `ede521c410d8b4bb7713fdac4070a34e864c00843854aa640a9c5ccf9326eb9c`. Exact original complete ASCII LF source-file input; no object-vector or native compilation admission is recorded.
- `capture-runner` (provider): [rust/tcl-registry/tests/data/native_jim_flat_namespace_original/capture.py](../../../../rust/tcl-registry/tests/data/native_jim_flat_namespace_original/capture.py). SHA-256 `ad5af8dadd1e08410aeb5f0b8d0c3bb995130dee5ab57de6c7db9e8aaf579ee1`. Exact retained Root capture runner; its original absolute queue path is preserved. No replay is launched by this review.
- `provider-queue` (provider): [rust/tcl-registry/tests/data/native_jim_flat_namespace_original/queue.json](../../../../rust/tcl-registry/tests/data/native_jim_flat_namespace_original/queue.json). SHA-256 `3eb6871f7e4def2770d65bde01d20ccbe7e7840b3128da03ba072c473292db7e`. Exact provider queue consumed by the retained runner. Only provider process receipts actually present in this fixture establish a run.
- `jim-receipt` (provider): [rust/tcl-registry/tests/data/native_jim_flat_namespace_original/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_jim_flat_namespace_original/jim/receipt.json). SHA-256 `465779dea35b80c07df050f9e691cc13c00942f2bf4f0c12a987598a4dda6cdc`. Original process argv, exit, actual queried patchlevel, executable/source/runner/raw-stream digests and selected SDK/build pins.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_jim_flat_namespace_original/jim/stdout](../../../../rust/tcl-registry/tests/data/native_jim_flat_namespace_original/jim/stdout). SHA-256 `e2afdc47cd7bfc3fddb8c24e292aa39d380e004320c3a295db0010c81c151216`. Exact complete raw stdout bytes for the original process; catch/setup observations are distinct from outer process status.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_jim_flat_namespace_original/jim/stderr](../../../../rust/tcl-registry/tests/data/native_jim_flat_namespace_original/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact complete raw stderr bytes for the original process; catch/setup observations are distinct from outer process status.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [runtime/rust/src/interp/native_jim_namespace.rs](../../../../runtime/rust/src/interp/native_jim_namespace.rs), `interp::native_jim_namespace::tests::jim_flat_command_context_and_helper_enumeration_match_native_controls` (linked): The exact retained original ASCII namespace-script composition and result are checked independently from the root-holder/epoch ownership controls. No counted object input or internal native pointer identity follows from the CLI observation.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "/workspace/.proofs/native-jim-flat-namespace151/capture.py"
]
```

The original capture runner and process argument vectors retain capture-machine absolute paths. The archived source, runner and provider queue are byte-identical. Replay requires independently verified matching SDK/environment/executable pins and fresh output locations. This review verifies immutable bytes without launching a process. ASCII source-file completion/results do not establish counted object-vector admission, internal object identity, header/cache state, compiler recipes, arbitrary callback scheduling or appliance behavior.
