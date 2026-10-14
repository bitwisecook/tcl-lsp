# Original variable references for debugger replay

`original-variable-references.tcl` measures original source substitutions on C Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 and JimTcl 0.84-9-g5bac7c9. All six scripts exit 0. `providers.json` identifies selected executables, source-tree library files and filesystem dynamic libraries; every before/after SHA-256 agrees. No tcltest is used. Tcl 9 embedded library bytes are covered by the executable hash.

All six return SCALAR, SPACE, CASH, OPEN and EMPTY for the corresponding whole scalar references, and ELEMENT for both array spellings. Only C Tcl 9 reads `${a{b}c}` as NESTED; C Tcl 8 and Jim stop at the first brace and report a missing `a{b` variable. Compound `${x}suffix` produces SCALARsuffix, and a substituting index selects `a(SCALAR)` and fails because that element is absent.

The debugger is fixed to Tcl 9.0. Its Rust scalar expectations use the recorded C 9.0 results. Replay refuses compound text and index substitutions because it does not execute source. Its snapshot contains array roots rather than element values; array-element refusal is the replay capability contract, not an assertion that original interpreters cannot read elements. Rust controls are unexecuted until the parent captures their actual results.
