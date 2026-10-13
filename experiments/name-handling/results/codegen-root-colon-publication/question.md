# Can a procedure's constructed key be sent as its original publication operand?

Compare root `proc :audit` with the transport-shaped `proc :::transport` using
the exact [input](input.tcl) through direct standard input on five C Tcl release
selectors and Jim. Which simple command names become callable?

All six captured providers make `:audit` callable and leave `audit` absent.
All six make `transport` callable and leave `:transport` absent for the
transport-shaped declaration. Each successful call returns its distinct marker;
each absent call returns caught completion 1 with an invalid-command message.
The two declarations share one fresh process per provider and use distinct names.

The [raw capture](results/capture.json), per-provider stdout/stderr and provenance
retain the exact launched executable digests, argument vectors and input/output
hashes. The selector labels identify the configured binaries; this capture did
not query patchlevels, revisions, build configuration or startup environment.
All process exits are 0 and all stderr streams are empty.

The existing [root/colon publication proof](../../../../docs/design/analysis/name-resolution-proofs/procedure-original-root-and-colon-holder-publication.md)
independently observes the C5 root-colon distinction. Its Jim child-interpreter
controls are unsupported; this direct-input Jim observation has its own scope.
The shared owner's
[`original_root_procedure_publication_preserves_a_literal_colon`](../../../../rust/tcl-syntax/src/naming/native.rs#L3464)
asserts that original `:source` and `:::source` select different simple names
across the five C policies.

These are finite original CLI publication/call results. They do not measure a
Rust implementation, emitted WASM, private command generation, physical native
allocation, compiler admission, frame correspondence or BIG-IP execution.
The [audit](audit.md) identifies the software transport boundary separately.
