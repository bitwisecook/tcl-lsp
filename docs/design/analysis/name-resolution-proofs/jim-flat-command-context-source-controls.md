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

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, bigip

Status: `not-tested`.

No native interpreter observation establishes this Rust source-advice invariant.

### jim

Status: `observed`. Channel: One fixed complete ASCII LF source-file CLI argument in a fresh native shell process. Version: jim 0.84-9-g5bac7c9.

OBSERVATION catch code 0 returns exactly {} n {child::q p} p {::n::child::q ::n::p} ::n::absent LOCAL INNER ::n::p. Actual queried patchlevel is 0.84-9-g5bac7c9; outer process status is 0 and stderr is empty. The receipt input_channel text mentions scalar trace callbacks, but the retained probe and boundary field identify this flat namespace script; the original receipt bytes are unchanged.

## Consumer bindings

- [runtime/rust/src/interp/native_jim_namespace.rs](../../../../runtime/rust/src/interp/native_jim_namespace.rs), `interp::native_jim_namespace::tests::jim_flat_command_context_and_helper_enumeration_match_native_controls` (linked): The exact retained original ASCII namespace-script composition and result are checked independently from the root-holder/epoch ownership controls. No counted object input or internal native pointer identity follows from the CLI observation.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 0 | 1 | 0 | 0 |
| `isolated` | 0 | 1 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_jim_flat_namespace_original/probe.tcl](../../../../rust/tcl-registry/tests/data/native_jim_flat_namespace_original/probe.tcl)
- [rust/tcl-registry/tests/data/native_jim_flat_namespace_original/capture.py](../../../../rust/tcl-registry/tests/data/native_jim_flat_namespace_original/capture.py)
- [rust/tcl-registry/tests/data/native_jim_flat_namespace_original/jim/stdout](../../../../rust/tcl-registry/tests/data/native_jim_flat_namespace_original/jim/stdout)
- [rust/tcl-registry/tests/data/native_jim_flat_namespace_original/jim/stderr](../../../../rust/tcl-registry/tests/data/native_jim_flat_namespace_original/jim/stderr)
