# C8.6 procedure birth hooks and mathfunc bootstrap

The [mathfunc question](../../../../experiments/name-handling/clock-bootstrap-epoch/registration-repair/mathfunc-bootstrap/README.md)
captures empty-body min/max births, not the real init procedure bodies.
Its scalar probes start with those names absent, publish `proc <name> args {}`,
observe Present hooks and retain compiler/resolver epochs `(17, 0)`.
The same [C-init capture](../../../../experiments/name-handling/clock-bootstrap-epoch/registration-repair/c-init.stdout.txt)
instead reports both actual init min/max procedures with NULL hooks at `(17, 0)`.
Scalar clock add/format captures also have Present hooks; clock init stubs have
NULL hooks. These distinctions concern definition shape, not namespace spelling.

Exact original C8.6.18 anchors:

- `generic/tclProc.c:174–181` creates the command through
  TclNRCreateCommandInNs before inspecting the no-op header condition.
- `generic/tclBasic.c:2335–2465` installs a new object command with NULL
  compileProc; collisions delete the displaced command first.
- `generic/tclProc.c:280–332` installs TclCompileNoOp only when the original
  body is not tclProcBodyType, the original argument spelling is bare `args`
  with surrounding ASCII spaces, and TclParseAllWhiteSpace consumes the full
  counted body. The argument comparison uses its C-string spelling. There is
  no command-name/mathfunc test in this selection.
- `generic/tclBasic.c:3237–3246` increments compileEpoch when deleting an old
  command whose compileProc is non-NULL. The later TclCompileNoOp assignment
  in tclProc.c has no epoch increment. Fresh captured min/max births displace
  no token; their measured epochs remain unchanged. Shadow/resolver effects
  are separate, so this is not an assertion that every fresh name preserves epochs.
- `generic/tclBasic.c:342–376,916–930` constructs the builtin mathfunc roster,
  which excludes min/max; `library/init.tcl:87–123` defines their nonempty
  procedure bodies. Those bodies fail the no-op whitespace condition.

Three Rust owners inspected: shared `rust/tcl-registry/src/native_procedure.rs`,
`runtime/rust/src/interp.rs`, and `runtime/rust/src/namespace.rs`.
The shared procedure_header_compilation_bytes selector at lines 418–477 already
distinguishes original formals/body/representation without matching the name.
Interp's original procedure publication at 5048–5076 selects the header from
the original objects and maps NoOp to ProcedureNoOp, Absent to Absent, and
unavailable evidence to Unknown. Namespace's set_native_compiler_recipe at
640–655 installs the attachment without advancing compiler epoch.

The retained bootstrap mismatch is earlier: the mathfunc installer publishes
synthetic min/max stock tokens before init. Their compiler registration is
unselected. Replacing a displaced Unknown token makes compiler epoch unavailable
through namespace's increment_native_command_epoch and the shared mutation
policy. An incoming NoOp/NULL attachment cannot authentically restore that
independent epoch. The [minimal replacement refusal](../../../../experiments/name-handling/clock-bootstrap-epoch/registration-repair/runtime-mathfunc-min-replacement.stderr.txt)
and [prior real-init refusal](../../../../experiments/name-handling/results/foreach-counter-object-lifetime/runtime-8.6-real-init.stderr.txt)
are captured; an exact internal instruction location for real init is unmeasured.

The bounded repair authors the actual C8.6 constructor's mathfunc admission
roster in the shared registry/bootstrap owner; the installer consumes it,
so library-owned min/max are absent until actual proc publication. Keep the
existing object-based header selector and birth/update attachment door. Do not
add a min/max compiler descriptor, infer a hook from display names, convert
Unknown to Absent, apply a blanket hook policy to all procedures, or introduce
a generic handler fallback to retain synthetic pre-init availability.

The focused regression verifies C8.6 pre-init min/max absence and both shapes
in fresh interpreters, inspecting actual installed generations and relative
epochs. Replacing the actual Present token advances the compiler epoch alone.
The [new discrete receipt](../../../../experiments/name-handling/results/native-c86-mathfunc-bootstrap/README.md)
retains original C controls, exact init max source, focused1/1 pass, source
currency and fresh-runner real-init exit0 with exact `REACHED\n`, empty stderr.

That startup control now reaches; this is no broader conformance verdict.
Other constructor inventories, precompiled body variants, other names/profiles,
VM installer consumption, clock support workers and clock map order remain
open. The first compile failure and all prior failed captures remain preserved.
