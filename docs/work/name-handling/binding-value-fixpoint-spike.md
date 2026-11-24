# Binding and value fixpoint spike

The probe runs the existing source binding walk and SSA/SCCP in a loop capped
at four rounds. SCCP can offer exact string contents for an original
whole-word variable read whose value binding could not determine. The next
round presents those bytes to the ordinary operand-freezing path and reruns
binding. The probe supplies no object representation, numeric receipt or
native name authority. It bypasses the source-interpretation memo during the
experiment because that memo's key has no slot for incoming SCCP values.

Each incremental run replaces the predecessor's final command with the
fixture's final command and reuses the segmented prefix. Both paths rerun the
compiler value analysis each round. Only the settled snapshot is compared.
The probe checks the full retained realm for structural equality under the
same source image and input, then compares the bytes of its ordered site
projection. The projection includes authored and materialised sites, their
evaluated heads and arguments, command targets, lookup uncertainty and
frames. Re-bound site counts compare successive projections and include
newly reached or withdrawn sites. The initial binding round has no
predecessor and reports zero.

Run from the repository root with the work branch's Rust toolchain.

```sh
cargo test --locked -p tcl-compiler --lib binding_value_fixpoint_probe -- --nocapture
```

The five original inputs are in [dynamic/](handover/oo-dispatch-probe/dynamic/).
The three companion cases compute a command head, an absolute level
and an array element name. They add an actual substituted head to e01's
substituted script and computed operands alongside the literal level and
array operand in e03 and e21. Tcl 8.6.16 produced the expected outputs for
all eight cases.

| Case | Fresh rounds | Incremental rounds | Re-bound sites per round, both paths | Sealed site bytes equal |
| --- | ---: | ---: | --- | --- |
| e01 substituted script | 1 | 1 | 0 | Yes |
| e03 caller frame | 1 | 1 | 0 | Yes |
| e04 variable alias | 1 | 1 | 0 | Yes |
| e18 dynamic procedure declaration | 1 | 1 | 0 | Yes |
| e21 array element alias | 1 | 1 | 0 | Yes |
| e01 companion, computed head | 1 | 1 | 0 | Yes |
| e03 companion, computed level | 2 | 2 | 0, 3 | Yes |
| e21 companion, computed array element | 1 | 1 | 0 | Yes |

The computed-level companion exercises the SCCP feedback edge. The other
cases issue no new operand value through the probe. One-round convergence
establishes parity for those inputs and can retain unresolved sites.
The experiment covers one Tcl profile and the incremental segmentation
entry. The production per-body scheduler is phase 3.

The focused Rust probe, workspace Rust formatting, and the license and Tcl
execution evidence checks passed. The full `rust-check` and `prep-pr` gates
remain pending for the PR stage.
