# naming.compiler.no-hook-unknown-mode

Kind: `implementation-contract`

## Problem statement

An unknown caller compilation mode can mean either direct argv evaluation or bytecode compilation. If the selected stock proc registration has a NULL compileProc, treating that uncertainty as an arbitrary compiler can withdraw command state before proc arguments run. The absence of a compiler must be established from the selected registration; a renamed label, caller words, or handler effects do not establish it.

## Question

How does the shared compiler selection distinguish a selected NoHook registration from a hook-bearing unknown mode while retaining original full argv?

## Conclusion

Only an independently selected NoHook descriptor is Generic under Direct, BytecodeObject and Unknown modes. Original native word ownership, dialect selection and a valid nonzero operand ordinal remain required. A renamed authored label cannot select the stock registration, and a selected hook-bearing return remains Unknown when compilation mode is unknown. No generic transport result certifies handler success or body entry.

## Scope

Registry pure selection contract tested against the current C5 descriptor roster and a complete NativeValue original word vector containing an opaque byte. These Rust assertions are independent of source inspection and of actual native execution.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this question.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::select`: Project selected NoHook under unknown mode without a native entry or successful invocation grant.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::select_native_words`: Preserve authentic original full-vector, native source protocol and selected registration isolation.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `native_compilation::tests::selected_no_hook_registration_keeps_generic_argv_under_unknown_mode` (linked): Only an independently selected NoHook descriptor is Generic under Direct, BytecodeObject and Unknown modes. Original native word ownership, dialect selection and a valid nonzero operand ordinal remain required. A renamed authored label cannot select the stock registration, and a selected hook-bearing return remains Unknown when compilation mode is unknown. No generic transport result certifies handler success or body entry.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "-p",
  "tcl-registry",
  "selected_no_hook_registration_keeps_generic_argv_under_unknown_mode"
]
```

Run against the selected source configuration. A linked selector records coverage, not a claim of test execution.
