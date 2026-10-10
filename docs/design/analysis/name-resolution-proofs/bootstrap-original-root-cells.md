# Which root cells and definition/array states are recorded immediately after each actual native interpreter constructor?

Proof ID: `naming.bootstrap.original-root-cells`

## Problem statement

A name catalogue cannot prove that a root variable cell is allocated or defined. Native constructors can allocate undefined special cells, and C arrays differ from Jim dictionaries. The model must distinguish allocation, definition and representation.

## Question

Which root cells and definition/array states are recorded immediately after each actual native interpreter constructor?

## Provider answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 8.4.20 | observed | Constructor allocation/definition rows are retained in core-roots.tsv; this is not a Tcl_Init, shell startup or complete command/module inventory. |
| tcl8.5 8.5.19 | observed | Constructor allocation/definition rows are retained in core-roots.tsv; this is not a Tcl_Init, shell startup or complete command/module inventory. |
| tcl8.6 8.6.18 | observed | Constructor allocation/definition rows are retained in core-roots.tsv; this is not a Tcl_Init, shell startup or complete command/module inventory. |
| tcl9.0 9.0.4 | observed | Constructor allocation/definition rows are retained in core-roots.tsv; this is not a Tcl_Init, shell startup or complete command/module inventory. |
| tcl9.1 9.1.0 | observed | Constructor allocation/definition rows are retained in core-roots.tsv; this is not a Tcl_Init, shell startup or complete command/module inventory. |
| jim not recorded by this retained invocation | observed | Constructor allocation/definition rows are retained in core-roots.tsv; this is not a Tcl_Init, shell startup or complete command/module inventory. |
| bigip not recorded | not-tested | No appliance observation for this question. |

## Conclusion

The retained core-roots table records constructor root allocations, including undefined C special cells and C-array versus Jim-dictionary storage. It does not establish complete command registration, cell contents or allocation chronology from bucket order.

## Scope

Native bootstrap constructor root tables only.

## Evidence and reconfirmation

The [original source](../../../../rust/tcl-registry/tests/data/native_bootstrap/probe.c) and [capture manifest](../../../../rust/tcl-registry/tests/data/native_bootstrap/capture-manifest.json) retain the source and provider fingerprints. Peer observation files contain the complete recorded outputs. These are retained observations; no new capture is asserted by this page.

Reconfirmation requires the exact provider release, retained source program and observer window. Use the exact source, provider versions and hashed build/observer purposes recorded in the manifest. Physical probes need matching native internal headers and libraries; no generic shell command reproduces them. Retained observations are not a fresh reconfirmation; failed or missing providers supply no passing assertion.

The Rust fixture assertions validate the shared implementation against the retained rows; they are distinct from the original native capture.
