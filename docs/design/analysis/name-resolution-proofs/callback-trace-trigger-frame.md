# naming.callback.trace-trigger-frame

Kind: `native-observation`

## Problem statement

A trace prefix is installed in ::A while the operation that fires it runs in ::B, and matching prefix heads also exist at root. Resolving a prefix at installation can bind the wrong callback even when prefix bytes remain intact. Variable, command and execution trace purposes each need an actual triggering-frame observation.

## Question

Which namespace resolves unqualified prefix heads for the exact variable write, command rename and execution enter traces installed in ::A and triggered by ::B procedures?

## Conclusion

All five C providers resolve each of the three callback heads in ::B and report the triggering caller at level1 in ::B. The selected Jim guest rejects trace as unavailable for each case. These results establish the measured triggering lookup purposes; they do not establish arbitrary script equivalence, prefix storage, native argv objects or callback results.

## Scope

Exact retained ASCII source file executed by C Tcl8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim0.84-9-g5bac7c9 native CLI providers. Full source/output/error/returncode and launcher/native executable SHA are retained. No opaque byte input, object header, native body/compiler grant, unknown future table or BIG-IP appliance is measured.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Selected native executable; launcher. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Tcl.

trace 0 VALUE {B 1 ::B} | command-trace 0 {} {B 1 ::B} | execution-trace 0 VALUE {B 1 ::B}

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Selected native executable; launcher. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Tcl.

trace 0 VALUE {B 1 ::B} | command-trace 0 {} {B 1 ::B} | execution-trace 0 VALUE {B 1 ::B}

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Selected native executable; launcher. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Tcl.

trace 0 VALUE {B 1 ::B} | command-trace 0 {} {B 1 ::B} | execution-trace 0 VALUE {B 1 ::B}

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Selected native executable; launcher. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Tcl.

trace 0 VALUE {B 1 ::B} | command-trace 0 {} {B 1 ::B} | execution-trace 0 VALUE {B 1 ::B}

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Selected native executable; launcher. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Tcl.

trace 0 VALUE {B 1 ::B} | command-trace 0 {} {B 1 ::B} | execution-trace 0 VALUE {B 1 ::B}

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Selected native executable; launcher. Configure/compiler flags are not recorded for this capture.. Channel: ASCII source file through selected native CLI. Dialect: Jim Tcl.

trace 1 {invalid command name "trace"} {} | command-trace 1 {invalid command name "trace"} {} | execution-trace 1 {invalid command name "trace"} {}

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
  "naming.callback.trace-trigger-frame",
  "--provider",
  "8.4",
  "--executable",
  "/absolute/path/to/native/tclsh8.4",
  "--output",
  "/absolute/path/outside/repository"
]
```

Repeat with 8.5, 8.6, 9.0, 9.1 and jim independently selected native CLI providers. Exact entire stdout, stderr and exit must match, including companion controls and the Jim caught unsupported trace diagnostics. No automatic provider fallback; global concurrency2 and per-process timeout60s are unchanged. Reconfirmation does not replay internal objects or unmeasured BIG-IP.
