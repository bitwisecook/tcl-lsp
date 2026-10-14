# Literal and empty variable names in read/write operations

[probe.tcl](probe.tcl) executes five finite name cases in fresh original C Tcl8.4.20/8.5.19/8.6.18/9.0.4/9.1.0 and Jim processes. [results.json](results.json) preserves exact source/executable hashes, provider identity queries, original C init.tcl identities, argv, stdout, stderr and process exit codes. The checked Jim source revision is retained in the results. No Tcltest or replacement init.tcl is used.

| Source case | C8.4 | C8.5–9.1 | Jim |
| --- | --- | --- | --- |
| Set/increment the empty name |5|5|5|
| Bind/read the empty loop name |VALUE|VALUE|VALUE|
| dict with a literal dollar name |Command unavailable|VALUE|VALUE|
| dict with the empty name |Command unavailable|VALUE|VALUE|
| Bind/read a literal dollar loop name |VALUE|VALUE|VALUE|

The Tcl8.4 dict rows preserve `invalid command name "dict"`; they are not successful dict observations. All interpreter processes exit0 with empty stderr because the source captures each operation's result. Each case's catch status remains explicit.

These reductions answer legal variable-name payload questions. They motivate current SSA classified-read, read-before-write and selected LoopVarList controls in rust/tcl-compiler/src/ssa.rs; the current source and patch identities are recorded independently by implementation validation. Actual source roles, full input/configuration, current Registry/Module ownership and ambiguity/refusal controls remain necessary. The results do not establish Rust cell allocation, frame ownership, Native compilation, whole inlining eligibility or F5 behaviour.

The independently retained [frame-local observations](../frame-local-name-geometry/README.md) include unbound literal dollar and empty-name controls plus C/Jim qualification policy. The [F5 operation packet](../../f5/questions/ssa-empty-read-write-names.md) is unanswered and requires genuine TMM context and versioned same-branch answers.
