# naming.variable.counted-formal-foreach-slot-execution-frontier

Kind: `native-observation`

## Problem statement

Two original formal objects k\x00a and k\x00b have equal lengths and CString prefixes. Their formal-list storage and invocation arity can prevent the foreach body from running at all; observing a failed call must not be used to infer which loop variable cell was selected.

## Question

When a procedure is defined with those two counted original formal objects, does invocation with ONE TWO reach the body, and if it does, what static/dynamic foreach and later write results are produced?

## Conclusion

All five C builds accept definition but reject the two-argument call with wrong # args for p k, so these controls establish no C foreach-body cell correspondence. The manifested Jim build reaches the body and returns {ONE CHANGED ONE CHANGED} ONE DYNAMIC ONE DYNAMIC. Original formal installation, body execution and source/static/runtime cell ownership remain independent.

## Scope

Two retained windows per six manifested native builds: original definition via object argv and one original invocation. Formal objects are counted k00a and k00b, exact body bytes in body.hex/probe.c. Jim revision/version is not recorded beyond build hashes. No C loop success, compiler LVT or observer/release capability is inferred.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: See historical references.. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

| case | formal condition | window | code | result |
| --- | --- | --- | --- | --- |
| nul-equal-length-foreach | two-original-formals | original-definition | 0 | "" |
| nul-equal-length-foreach | two-original-formals | compiled-and-dynamic-invocation | 1 | "wrong # args: should be \"p k\"" |

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: See historical references.. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

| case | formal condition | window | code | result |
| --- | --- | --- | --- | --- |
| nul-equal-length-foreach | two-original-formals | original-definition | 0 | "" |
| nul-equal-length-foreach | two-original-formals | compiled-and-dynamic-invocation | 1 | "wrong # args: should be \"p k\"" |

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: See historical references.. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

| case | formal condition | window | code | result |
| --- | --- | --- | --- | --- |
| nul-equal-length-foreach | two-original-formals | original-definition | 0 | "" |
| nul-equal-length-foreach | two-original-formals | compiled-and-dynamic-invocation | 0 | "{CHANGED CHANGED CHANGED TWO} CHANGED CHANGED CHANGED DYNAMIC" |

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: See historical references.. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

| case | formal condition | window | code | result |
| --- | --- | --- | --- | --- |
| nul-equal-length-foreach | two-original-formals | original-definition | 0 | "" |
| nul-equal-length-foreach | two-original-formals | compiled-and-dynamic-invocation | 0 | "{CHANGED CHANGED CHANGED TWO} CHANGED CHANGED CHANGED DYNAMIC" |

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: See historical references.. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Tcl.

The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

| case | formal condition | window | code | result |
| --- | --- | --- | --- | --- |
| nul-equal-length-foreach | two-original-formals | original-definition | 0 | "" |
| nul-equal-length-foreach | two-original-formals | compiled-and-dynamic-invocation | 0 | "{CHANGED CHANGED CHANGED TWO} CHANGED CHANGED CHANGED DYNAMIC" |

### jim

Status: `observed`. Version: not recorded (manifest label Jim). Build: See historical references.. Channel: Native C/Jim object construction and counted evaluation as retained in probe.c. Dialect: Jim Tcl.

The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

| case | formal condition | window | code | result |
| --- | --- | --- | --- | --- |
| nul-equal-length-foreach | two-original-formals | original-definition | 0 | "p" |
| nul-equal-length-foreach | two-original-formals | compiled-and-dynamic-invocation | 0 | "{ONE CHANGED ONE CHANGED} ONE DYNAMIC ONE DYNAMIC" |

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `e0` (provider): [rust/tcl-vm/tests/data/native_foreach_slots/manifest.json](../../../../rust/tcl-vm/tests/data/native_foreach_slots/manifest.json). Six original native builds with compile/run statuses, two-window log hashes, header/library/binary hashes.
- `e1` (input): [rust/tcl-vm/tests/data/native_foreach_slots/probe.c](../../../../rust/tcl-vm/tests/data/native_foreach_slots/probe.c). Retained exact probe.c; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e2` (input): [rust/tcl-vm/tests/data/native_foreach_slots/body.hex](../../../../rust/tcl-vm/tests/data/native_foreach_slots/body.hex). Retained exact body.hex; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e3` (input): [rust/tcl-vm/tests/data/native_foreach_slots/README.md](../../../../rust/tcl-vm/tests/data/native_foreach_slots/README.md). Retained exact README.md; original source/object inputs and reporter boundary, independent of Rust implementation.
- `e4` (observation): [rust/tcl-vm/tests/data/native_foreach_slots/8.4.20.tsv](../../../../rust/tcl-vm/tests/data/native_foreach_slots/8.4.20.tsv). Complete retained semantic TSV for 8.4.20; The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.
- `e5` (observation): [rust/tcl-vm/tests/data/native_foreach_slots/8.5.19.tsv](../../../../rust/tcl-vm/tests/data/native_foreach_slots/8.5.19.tsv). Complete retained semantic TSV for 8.5.19; The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.
- `e6` (observation): [rust/tcl-vm/tests/data/native_foreach_slots/8.6.18.tsv](../../../../rust/tcl-vm/tests/data/native_foreach_slots/8.6.18.tsv). Complete retained semantic TSV for 8.6.18; The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.
- `e7` (observation): [rust/tcl-vm/tests/data/native_foreach_slots/9.0.4.tsv](../../../../rust/tcl-vm/tests/data/native_foreach_slots/9.0.4.tsv). Complete retained semantic TSV for 9.0.4; The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.
- `e8` (observation): [rust/tcl-vm/tests/data/native_foreach_slots/9.1.0.tsv](../../../../rust/tcl-vm/tests/data/native_foreach_slots/9.1.0.tsv). Complete retained semantic TSV for 9.1.0; The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.
- `e9` (observation): [rust/tcl-vm/tests/data/native_foreach_slots/Jim.tsv](../../../../rust/tcl-vm/tests/data/native_foreach_slots/Jim.tsv). Complete retained semantic TSV for Jim; The exact definition and invocation rows are both retained. A definition code 0 does not certify body execution.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `compiled_foreach_matches_twelve_original_native_slot_records`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-vm/src/interp/native_compiled_locals.rs](../../../../rust/tcl-vm/src/interp/native_compiled_locals.rs), `interp::native_compiled_locals::tests::compiled_foreach_matches_twelve_original_native_slot_records` (linked): Compares all twelve original definition/invocation tuples, including every C arity failure; it does not claim twelve successful loop bodies.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cc",
  "-DSTDC_HEADERS=1",
  "-DHAVE_UNISTD_H=1",
  "-I/path/to/recorded/generic",
  "-I/path/to/recorded/unix",
  "rust/tcl-vm/tests/data/native_foreach_slots/probe.c",
  "/path/to/recorded/libtcl.a",
  "-lm",
  "-ldl",
  "-lpthread",
  "-lz",
  "-o",
  "/tmp/native-variable-proof"
]
```

Compile retained probe.c using each exact manifest command, substituting only authentic source/header/archive/output paths; Jim independently requires USE_JIM and its manifested libjim/link flags. Execute a fresh process per provider, require exit 0 and empty stderr, and project the retained definition/invocation code/result fields to the two corresponding TSV rows. The original raw JSON log is referenced by SHA256 but is not retained as complete content, so a full raw-output comparison requires recovery of that log. All C invocation arity errors and the exact Jim successful body result are part of the pass criterion; do not require universal guest code 0. No Rust execution is inferred.
