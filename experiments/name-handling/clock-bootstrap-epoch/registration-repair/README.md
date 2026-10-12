# C8.6 clock registration repair

Question: can the runtime preserve the actual C8.6 constructor's deferred clock
map and known compiler currency when the library publishes clock procedures,
and does that suffice for authentic matching library initialisation?

This directory is a separate question. The parent probe, frozen runner records,
inputs and raw outcomes remain untouched. `probe.c` adds actual ensemble API
observations and independent mathfunc entries to the retained private-header
probe; each C scenario starts with a fresh actual Tcl 8.6.18 interpreter.

## Authentic constructor and library observations

The actual C8.6 ensemble maps all seven public operations to their exact
`::tcl::clock::<member>` targets before init, including add/format/scan, whose
command entries do not yet exist. Its namespace is `::tcl::clock`, flags are
6 (prefixes and compilation enabled), and the explicit subcommand list is
NULL. Matching real `Tcl_Init` publishes the three NULL-hook procedures without
changing that map or measured compiler/resolver epochs `(17, 0)`.

The runtime now selects that C8.6 topology through registry-owned descriptors,
registering the four actual reading workers and retaining deferred map targets
without fabricating callback tokens. Its reading-worker compiler descriptors
retain Present hooks independently of handler effects and opcode acceptance.
Operation/body selection remains Invoke/Inherit; unmodelled compiler syntax
and unmeasured registrations retain uncertainty.

Map membership, target names and flags match; ordered map bytes do not. Actual
C8.6 creates `add clicks format microseconds milliseconds scan seconds`, while
the runtime emits `clicks seconds milliseconds microseconds add format scan`.
Reading-worker registration order also differs from C's. Full constructor
conformance and physical hash/table-order equivalence remain unproved. A future
repair needs registry-owned original bootstrap order, not sorting live maps.

Reading descriptors' implementation paths select C8.6 and later. The original
source excerpts independently retain the four hooks and actual registrations
for C8.6.18, C9.0.4 and C9.1.0. C9.0 creates the public ensemble in its actual
`init.tcl`; C9.1 constructs it from its C map. These are source observations,
not executions of C9 releases or a migration of runtime C9 bootstrap topology.
Earlier releases receive no private reading descriptor from this path.

## Runtime outcomes

`records.json` retains exact commands, library selection, inputs, exit codes,
streams and binary/source hashes for the final separately built native runner.

| Control | Exit | Reached outcome |
| --- | --- | --- |
| Scalar clock add procedure birth | 0 | `after` |
| Fresh command birth | 0 | `after` |
| Pre-init inventory/map | 0 | Four reading entries; seven mapped targets; prefixes enabled |
| Deferred targets published and dispatched | 0 | `deferred` for add/format/scan prefixes |
| Public and private seconds reads | 0 | Two host clock readings |
| Synthetic mathfunc min replacement | 2 | Original command-entry epoch refusal |
| Real matching library init | 2 | Original command-entry epoch refusal; no `REACHED` |
| Calendar operation before init | 1 | Guest error: invalid command `::tcl::clock::format` |

The scalar regression test passed: all three clock procedure births retain
known compiler/resolver epochs, and replacing the actual Present-hook seconds
worker advances only compiler epoch by one. This tests relative epoch changes;
it does not claim the runtime's initial numerical stamp equals C's 17.

The initial broader procedure/foreach test first panicked on an eager index of
an empty local-temporary vector. The independently correct lazy selection of
that local temporary avoids the index panic, but the same compiled loop then
aborted with an invalid-pointer allocator report. The standalone compiled-loop
input, raw streams and binary receipt reproduce SIGABRT (`-6`) independently
of the test harness. This native iterator ownership question remains open;
the scalar regression does not conceal or resolve it. Test attempt logs retain
the earlier panic and abort; those logs have no independent test-binary hash.
The separate [compiled foreach question](compiled-foreach/README.md) retains
stock C8.4–9.1 and pinned Jim outcomes for the exact unchanged input.

## Remaining support boundaries

Actual C8.6 starts without mathfunc min/max; real init defines both before its
clock bootstrap. C scalar births of these mathfunc procedures leave epochs
`(17, 0)` and produce Present-hook procedure entries, distinct from the clock
NULL-hook births. The runtime's separate mathfunc installer pre-publishes
synthetic min/max entries without selected compiler registration descriptors.
The minimal replacement independently poisons compiler currency and refuses
at the following specialised set. This establishes another bootstrap mismatch
consistent with the retained real-init refusal; no exact internal init
instruction location or successful init completion is claimed here.
The independent [mathfunc bootstrap question](mathfunc-bootstrap/README.md)
links its exact source, raw captures and shared build provenance.

The seven authentic C8.6 support workers remain unimplemented: getenv, Oldscan,
ConvertLocalToUTC, GetDateFields, GetJulianDayFromEraYearMonthDay,
GetJulianDayFromEraYearWeekDay, and ParseFormatArgs. Their actual NULL hooks
authenticate neither handler effects nor a no-effect policy. This repair
provides no synthetic registrations or receipts for them. Calendar library
execution, mathfunc bootstrap/procedure compilation, and compiled iterator
ownership require their own evidence and implementation. Jim, hosted F5, VM
and other runtime releases are outside the repaired topology question.

## Source receipts and replay

`before-owners/` preserves the exact five changed owners before this repair;
`build-selected-sources.json` maps small exact gzip snapshots to original source
paths and hashes. Source-only inventories cover the wider Rust inputs before
and after the final runner build. One unrelated compiler regression test changed
during that build, recorded in `build-inventory-comparison.json`; selected
runtime/registry build owners remained unchanged. The later definer test-import
correction is a separate validation source change, not part of this runner
build's receipt. These receipts record actual source/build observations, not
independent authentication that arbitrary source bytes belong to a binary.

`original-source-excerpts.json` provides exact original line ranges and hashes
of full C/library source files. `build-records.json` records focused test/build
attempts and their outcomes, including unrelated registry-test compilation
blockers rather than treating them as passes.

Replay into a required fresh output directory with an already available runner:

```
python3 experiments/name-handling/clock-bootstrap-epoch/registration-repair/capture.py --question registration --output-dir /tmp/name-clock-registration-replay-new --runner /tmp/name-clock-registration-repair-runner-final
```

`capture.py` refuses an existing destination and compiles its C probe to a path
inside the new output directory. Its shell mode executes the unchanged foreach
input in actual reference shells. The hash-pinned `run.py` is preserved as
original capture provenance; replay must use `capture.py` to retain original
result bytes. No Cargo build is performed by the capture tool.

The retained source snapshots, rather than an evolving checkout, define the
captured question. This supplements the canonical name-resolution authority;
it supplies no affirmative answer to the separate unresolved provider questions.
