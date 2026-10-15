# Original fixed nonnumeric equality values

The retained fixed [probe](probe.tcl) ran against original Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and pinned Jim. All six processes exited zero; catches inside the probe preserve individual failures. The [original manifest](manifest.json) retains actual commands, matching Tcl source libraries, provider/init hashes before and after, stdout/stderr hashes and capture time. All six empty stderr channels are retained.

For the fixed nonnumeric literal `GET`, the observed numeric comparison operator, textual equality and exact Switch agree for the five fixed left-hand values on all six providers. The separate numeric `01`, spaced `1` and exponent examples demonstrate why this agreement cannot be assumed for potentially numeric fixed values. The shared selected numeral grammar must independently rule out a numeric literal before that operator is considered a string-equality source proposal.

The observations preceded the bounded known-nonnumeric controls. They do not generalise to unknown values or establish complete transformation equivalence.

This evidence concerns actual expression/name grammar and literal values. It supplies no F5 execution, reached lookup, frame, Native/Rust acceptance or edit authority. See the [name-handling contract](../../../docs/design/contracts/name-handling.md), [pattern catalogue](../patterns/README.md) and [result index](../results/README.md).

Original outputs remain immutable. `SHA256SUMS` covers all evidence and replay support. Run `python replay.py --out /tmp/new-nonnumeric-expression-literals-20261015-capture` for a separate reproduction with the retained providers and init files. The runner verifies provider hashes, refuses an existing output directory and never overwrites these original outputs. Any replay is separate evidence.
