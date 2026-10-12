# Current owner-resolution controls and full gate

All 29 owner-resolution controls pass (exit 0), including the two exact Python
proof-route controls and existing public/private, module, alias and dispatch
checks. The captured process took 122.589 seconds including compilation;
the test harness reports 13.21 seconds of test execution.

The rebuilt ordinary xtask binary then ran the full owner-resolution gate once
and exited 1 with 158 obligations, all public-entry declaration checks. Exact
comparison with the published 164-obligation receipt removes the two uncovered
completion bullets and four existing Python proof-route obligations. No
obligation is added. The remaining public-entry failures are unresolved.
The gate process took 20.446 seconds including its ordinary binary rebuild.

[result.json](result.json) retains every actual obligation and the six exact
removals. [owner-resolution.log](owner-resolution.log) contains emitted checker
lines; the complete Cargo output remains in the controls/gate stdout and stderr
logs. Runtime dead-code warnings are retained; no warning baseline is inferred.
Commands, start/end times, exits and timeout bounds are in the process receipts.
The controls binary and ordinary gate binary have separate hash inventories.

All 7,686 scoped hashes match before/after: prior checker inputs plus the
conservative Rust/runtime source and manifest pool specified by the snapshot.
That pool includes unlinked test/runtime context, excludes external Cargo
registry sources and is not a whole-repository or native source-to-build claim.
The owner and contract files remained frozen throughout validation.

No source fixes, formatting, inventory writes or other gates ran. In particular,
recognising the existing complete Python proof-checker route does not execute
that proof gate or establish evidence coverage. Required wider gates remain open.
