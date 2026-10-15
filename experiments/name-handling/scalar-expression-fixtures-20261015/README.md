# Original scalar expression fixture source

The retained fixed [probe](probe.tcl) ran against original Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and pinned Jim. All six processes exited zero; catches inside the probe preserve individual failures. The [original manifest](manifest.json) retains actual commands, matching Tcl source libraries, provider/init hashes before and after, stdout/stderr hashes and capture time. All six empty stderr channels are retained.

All six original providers reject bare `one` as the other operand of `$x eq one` and accept the quoted literal in `$x eq "one"`. A spaced braced variable name is valid on all six. The nested braced-name example is accepted by original Tcl 9.0/9.1 and refused by original Tcl 8.4/8.5/8.6 and Jim; Unicode bare variable spelling in this probe is accepted by Jim and refused by the five C providers. Every actual success/error is preserved. Literal `||` inside a quoted operand is data rather than an OR node.

The probe preceded replacing invalid bare fixture operands with quoted literals and transporting actual expression contexts. Expected names, values and adversarial scalar subjects remain independent assertions.

This evidence concerns actual expression/name grammar and literal values. It supplies no F5 execution, reached lookup, frame, Native/Rust acceptance or edit authority. See the [name-handling contract](../../../docs/design/contracts/name-handling.md), [pattern catalogue](../patterns/README.md) and [result index](../results/README.md).

Original outputs remain immutable. `SHA256SUMS` covers all evidence and replay support. Run `python replay.py --out /tmp/new-scalar-expression-fixtures-20261015-capture` for a separate reproduction with the retained providers and init files. The runner verifies provider hashes, refuses an existing output directory and never overwrites these original outputs. Any replay is separate evidence.
