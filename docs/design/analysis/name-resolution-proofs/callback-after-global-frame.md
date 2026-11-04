# naming.callback.after-global-frame

Kind: `native-observation`

## Problem statement

An after idle script is scheduled by a procedure in ::A with competing global and ::A callback names. Reusing the installer namespace would choose a different delayed implementation; timing alone says when it runs, not where its head resolves.

## Question

When the exact ::A::queue schedules after idle delayed and update reaches it, which namespace resolves delayed?

## Conclusion

All six retained providers select the root delayed procedure and report namespace ::. C update returns empty while Jim update returns the callback list; the lookup conclusion does not equate those results. This is reached deferred-global lookup, not proof that any queued callback must run.

## Scope

Exact retained ASCII source file executed by C Tcl8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim0.84-9-g5bac7c9 native CLI providers. Full source/output/error/returncode and launcher/native executable SHA are retained. No opaque byte input, object header, native body/compiler grant, unknown future table or BIG-IP appliance is measured.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Selected native executable; launcher. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Tcl.

after 0 {} {ROOT 1 ::}

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Selected native executable; launcher. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Tcl.

after 0 {} {ROOT 1 ::}

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Selected native executable; launcher. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Tcl.

after 0 {} {ROOT 1 ::}

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Selected native executable; launcher. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Tcl.

after 0 {} {ROOT 1 ::}

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Selected native executable; launcher. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Tcl.

after 0 {} {ROOT 1 ::}

### jim

Status: `observed`. Version: 0.84-9-g5bac7c9. Build: Selected native executable; launcher. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Jim Tcl.

after 0 {ROOT 1 ::} {ROOT 1 ::}

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `source` (input): [rust/tcl-registry/tests/data/native_callback_lookup_scope/scope.tcl](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/scope.tcl). Exact complete ASCII source for all three lookup purposes; each question is restricted to its own outcome row.
- `capture` (provider): [rust/tcl-registry/tests/data/native_callback_lookup_scope/receipt.json](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/receipt.json). Actual six completed processes, version stdout, selected launcher and independently recorded native executable hashes, exact source SHA and limits.
- `output-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.4.stdout](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.4.stdout). Exact provider outcome row(s) for this question, with companion controls retained unchanged.
- `error-tcl8.4` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.4.stderr](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.4.stderr). Exact empty stderr retained; guest errors are caught in stdout.
- `output-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.5.stdout](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.5.stdout). Exact provider outcome row(s) for this question, with companion controls retained unchanged.
- `error-tcl8.5` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.5.stderr](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.5.stderr). Exact empty stderr retained; guest errors are caught in stdout.
- `output-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.6.stdout](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.6.stdout). Exact provider outcome row(s) for this question, with companion controls retained unchanged.
- `error-tcl8.6` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.6.stderr](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl8.6.stderr). Exact empty stderr retained; guest errors are caught in stdout.
- `output-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl9.0.stdout](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl9.0.stdout). Exact provider outcome row(s) for this question, with companion controls retained unchanged.
- `error-tcl9.0` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl9.0.stderr](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl9.0.stderr). Exact empty stderr retained; guest errors are caught in stdout.
- `output-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl9.1.stdout](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl9.1.stdout). Exact provider outcome row(s) for this question, with companion controls retained unchanged.
- `error-tcl9.1` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl9.1.stderr](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/tcl9.1.stderr). Exact empty stderr retained; guest errors are caught in stdout.
- `output-jim` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/jim.stdout](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/jim.stdout). Exact provider outcome row(s) for this question, with companion controls retained unchanged.
- `error-jim` (observation): [rust/tcl-registry/tests/data/native_callback_lookup_scope/jim.stderr](../../../../rust/tcl-registry/tests/data/native_callback_lookup_scope/jim.stderr). Exact empty stderr retained; guest errors are caught in stdout.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "python3",
  "scripts/dev/replay-grammar-native-source.py",
  "--proof",
  "naming.callback.after-global-frame",
  "--provider",
  "8.4",
  "--executable",
  "/absolute/path/to/native/tclsh8.4",
  "--output",
  "/absolute/path/outside/repository"
]
```

Repeat with 8.5, 8.6, 9.0, 9.1 and jim independently selected native CLI providers. Exact entire stdout, stderr and exit must match, including companion controls and the Jim caught unsupported trace diagnostics. No automatic provider fallback; global concurrency2 and per-process timeout60s are unchanged. Reconfirmation does not replay internal objects or unmeasured BIG-IP.
