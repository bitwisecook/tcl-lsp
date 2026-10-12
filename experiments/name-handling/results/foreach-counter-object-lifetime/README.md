# Private compiled foreach counter objects

Question: does separate ownership of private iterator state avoid the retained
third-advance String-capacity collision while preserving object/header cleanup,
and what completes in the exact native controls?

The original [failed question](../../clock-bootstrap-epoch/registration-repair/compiled-foreach/README.md),
input, SIGABRT streams and frozen binary receipt remain unchanged. This directory
records a new source/build/runtime question. It reuses the exact input with
SHA-256 `adbf44511437984ac055a50a009bf85591101864b471fe7a52068612829b5083`.
The prior [stock C/Jim controls](../../clock-bootstrap-epoch/registration-repair/compiled-foreach/reference-shells/records.json)
are retained original observations; no new reference input or oracle run was
needed for this repair. Their real stock startup differs from native core-only
execution. C8.4's missing clock namespace leaves later publication/set unreached;
the other reference shells completed without printing a result. F5 is unmeasured.

## Original C ownership and runtime representation

Actual C8.6 `tclExecute.c` creates two private null-string stack objects.
Their otherwise unused union fields carry iterNum/iterMax and a borrowed
ForeachInfo pointer. The step increments the counter before variable assignment;
end/error stack cleanup retires the two object headers and value-list headers.
ForeachInfo belongs to bytecode auxiliary data, not to the private header's
String destructor. Exact C source ranges and full-file hashes are retained in
[original-source-excerpts.json](original-source-excerpts.json), with raw tabs.

The Rust object owner uses a NULL-type String's internal_rep as buffer capacity.
The old iterator put private counter values there through change_type. On the
third advance, the old value two exceeded the canonical empty String's exact
size one; the String owner attempted to realloc its static empty-string pointer.

`native_each.rs` now retains both genuine Owned empty object headers, their
cleanup order and the actual recipe's lifetime-bound borrow. Private position
and maximum are typed Rust state outside the String capacity field. The
StackLists path consumes that position as its iterator state. No Tcl type is
invented, no raw capacity/pointer field is overloaded, and no native receipt,
compiler hook, fallback or source identity is donated by this private state.
This adapter preserves scoped object roles without claiming C union/physical
layout equality. C8.4/8.5 local-temporary iterator handling is unchanged.

## Current outcomes

[records.json](records.json) retains every exact command, status, stream,
input/binary hash and matching library inventory.

| Current native runner control | Exit | Outcome |
| --- | --- | --- |
| Exact original C8.6 procedure/foreach input | 0 | Empty stdout/stderr; ordinary process completion |
| Same input, C8.5 adjacent LocalGroupCopy path | 2 | Original command-entry epoch refusal |
| Same input, C9.0 StackLists path | 2 | Original command-entry epoch refusal |
| Same input, C9.1 StackLists path | 2 | Original command-entry epoch refusal |
| Exact matching real C8.6 init plus original marker input | 2 | Library-init command-entry epoch refusal; no marker reached |

The one meaningful regression passed. It first executes the three-item
publication body through an actual compiled StackLists instruction and checks
result `after`. It separately advances the same recipe's private cursor three
times while both owned headers retain canonical empty storage and zero String
capacity. Allocation-only leases then verify retirement without extending native
references; final object/buffer residual, double-free count and OOM flag are zero.
The first short exact test filter matched zero tests; [build records](build-records.json)
retain it explicitly as an unexecuted test, followed by the fully qualified
one-test pass. The native runner was rebuilt once and frozen separately.

The only production change is native_each.rs. Together, the verified object
invariant and the newly completed exact C8.6 input attribute removal of this
observed abort to the private-state repair. This does not supply a native memory
backtrace of the earlier failure or prove every iterator callback/lifetime case.
Other profiles and real init remain refused; their exact internal failure
locations are unmeasured. No clock-map/mathfunc correction or full-startup claim
is part of this question.

## Source currency and verification limits

[source-before.json](source-before.json), [source-frozen.json](source-frozen.json)
and [source-after-build.json](source-after-build.json) record selected owners.
Their exact before snapshots and the edited frozen owner are retained as gzip
files. Selected owners and all reachable repository-local runtime dependency
hashes remained unchanged during test/build, as recorded in
[build-source-comparison.json](build-source-comparison.json).

The separate whole-Rust inventory observed temporary edits in compiler
declaration_flow.rs and declaration_layout.rs. [Dependency selection](runtime-dependency-selection.json)
excludes tcl-compiler from this standalone runtime test/example build. Those
unlinked edits are recorded rather than claiming an identical whole workspace.
Receipts describe actual source/build observations; they do not independently
authenticate arbitrary source bytes as binary provenance.

Focused rustfmt, the fully qualified regression, one runner build and the five
bounded process controls completed. Existing build warnings remain. Clippy,
workspace rust-check/prep-pr gates and broader suites were not run for this job.
The remaining bootstrap refusals, map/registration ordering and general native
iterator ownership questions require their own scoped evidence and support.
