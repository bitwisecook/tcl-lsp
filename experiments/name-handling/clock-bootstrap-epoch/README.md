# C Tcl 8.6 clock bootstrap and command-entry epochs

Question: does the real C8.6 constructor install the clock stub command entries
that `init.tcl` later publishes, and what independent compiler/resolver epochs
do those births or replacements change?

`probe.c` links the actual Tcl 8.6.18 static library and reads its private
`Interp`/`Command` fields. Each scenario starts with a fresh `Tcl_CreateInterp`.
`run.py` retains compile arguments, source/library/header/binary hashes, exact
invocations, process status and streams in `records.json`. The selected
`TCL_LIBRARY` is the unmodified matching release library. `c-init` executes
real `Tcl_Init`, without a substitute init script.

| Actual C8.6 entry after `Tcl_CreateInterp` | Exists | `compileProc` |
| --- | --- | --- |
| `::clock` | Yes, core ensemble | Present |
| `::tcl::clock::{add,format,scan}` | No | No entry |
| `::tcl::clock::{clicks,seconds,milliseconds,microseconds}` | Yes | Present |
| `::tcl::clock::{getenv,Oldscan,ConvertLocalToUTC,GetDateFields,GetJulianDayFromEraYearMonthDay,GetJulianDayFromEraYearWeekDay,ParseFormatArgs}` | Yes | NULL |

Authentic init creates the three stub procedures, whose compiler hooks are NULL,
and leaves measured compiler/resolver epochs at `(17, 0)`. The fresh-name,
add/format/scan birth and getenv replacement scenarios also leave `(17, 0)`.
Replacing the actual compiled seconds worker changes only compiler epoch:
`(17, 0)` becomes `(18, 0)`. Each mutation scenario executes the ordinary
source shape `proc boot {} {proc <name> args {}; set marker after; puts $marker}`
and prints `after`. These private-header observations establish the measured
registration/hook/epoch distinctions, not arbitrary handler side effects.

## Frozen native runtime control

The runner `/tmp/name-native-c84-declared-cell-runner` has SHA-256
`571dcc78f4b0b7f4e21f2850bceca2d61813440718a63d7cb01da751f752c473`.
Its pre-init inventory contains all seven synthetic public-operation workers,
including add/format/scan, and none of the seven C support workers above.
`runtime-replacement` publishes the add stub and then reaches the specialised
`set` command-entry guard. It exits 2 with:

```
run_script: execution refused: native original command-entry epochs native handler policy is not selected
```

The independently captured `runtime-fresh` control prints `after` and exits 0.
`runtime-startup` uses `--init` and the real matching library, exits 2 with the
same command-entry refusal during library initialisation, and never prints
`REACHED`. No guest catch or Tcltest verdict is obtained from that refusal.
The frozen runner is a different binary from the earlier retained compiled-alias
runner; each capture has its own binary receipt. Inspected runtime source hashes
record current source currency, not an independently authenticated runner build
receipt or proof that every current source byte belongs to the frozen binary.

## Source inspection and bounded repair proposal

These statements classify current source; they are not an executed repair:

- `runtime/rust/src/cmd_clock.rs::install` publishes the three pre-init workers
  absent in C8.6. Its reading workers also lack authored registration descriptors.
- `Interp::seal_native_compiler_attachments` asks
  `CommandRegistry::native_compilation_for_registration` for each actual stock
  token. The missing clock member descriptors select `NativeCompilerRecipe::Unknown`.
- The replacement door retires the displaced token. Its independent
  `NativeCompilerCacheMutation::CommandToken { hook: Unknown }` changes the
  interpreter compiler epoch to unavailable, rather than guessing a number.
- `NativeCompilationGuard::BeforeArguments` then needs current epochs;
  `body_instruction_source_is_current` refuses before the next specialised
  command's arguments. The authentic init's `::tcl::initClock` foreach creates
  add/format/scan stubs, supplying the source trigger. The minimal replacement
  control isolates that trigger without replacing real init.

The proposed repair selects registration topology from the actual retained C
release: keep the C8.6 add/format/scan entries absent until real init publishes
them; install the reading workers and public clock as the actual stock ensemble;
provide their registry-owned compiler descriptors and registration mappings.
C8.6 support workers, when implemented, require separate measured NULL-hook
registration descriptors. They must not be inferred from public clock operations.
The existing descriptor constructor is `NativeCompilationSpec { grammar,
operation, body }`: an explicitly selected `NativeCompilationGrammar::NoHook`
authenticates NULL hook presence only. Its operation/body policy and runtime
name/variable/world side effects need independent ownership; NULL hooks do not
authenticate no handler effects. `namespace_no_hook_compiler!` demonstrates the
existing separate terminal descriptor and public implementation-path pattern.

A repair must preserve Unknown for unmeasured registrations, retain the genuine
Present hook on reading workers, and preserve independent epoch invalidation.
The seconds/getenv controls distinguish those cases. No runtime/registry source
was changed, and no successful repaired startup is claimed. Tcl 8.4, Tcl 9.x,
Jim, hosted F5 and VM execution are outside this question.

Replay from the repository root:

```
python3 experiments/name-handling/clock-bootstrap-epoch/run.py
```

This directory supplements the canonical name-resolution proof authority;
its evidence does not update other provider questions.
