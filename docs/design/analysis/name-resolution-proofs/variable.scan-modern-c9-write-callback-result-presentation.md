# naming.variable.scan-modern-c9-write-callback-result-presentation

Kind: `native-observation`

## Problem statement

Tcl 9 rejects legacy trace syntax, so the earlier scan-presenter row cannot answer whether a reached scan write callback preserves a later destination/result. A separate modern trace script is necessary for that concrete path.

## Question

Modern Tcl 9 scan write-callback failure and result getters What exact stages and results occur in the retained native probe?

## Conclusion

The two independent C9 modern-trace captures retain a reached failing write callback, scan completion/result getter stages and later output. They answer only that modern syntax/script and cannot repair or replace the legacy rejection rows, establish arbitrary object hooks, or certify compiler completion.

## Scope

Tcl9.0.4 and9.1.0 only; modern trace add variable write, public counted source evaluation and private result header/character cache stages; C84-86/Jim/BIG-IP not tested by this probe.

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

Status: `observed`. Version: 9.0.4. Build: Static native library; probe. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"VERSION|9.0.4\nCASE|0\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420226669727374223a20424f4f4d\nAFTER-STRING|string|1|1|-1\nCHARS|23\nAFTER-LENGTH|string|1|1|23\nOUTPUT|2\n"

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Static native library; probe. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"VERSION|9.1.0\nCASE|0\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420226669727374223a20424f4f4d\nAFTER-STRING|string|1|1|-1\nCHARS|23\nAFTER-LENGTH|string|1|1|23\nOUTPUT|2\n"

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `e0` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/manifest.json). Exact original compiler/header/library/executable hashes, process receipts and native source anchors where supplied.
- `e1` (input): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/probe.c](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/probe.c). Complete original C translation unit; selected public API and reporter stages are retained.
- `e2` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/9.0.4-run.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/9.0.4-run.stdout). 9.0.4 complete original captured native transcript.
- `e3` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/9.1.0-run.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/9.1.0-run.stdout). 9.1.0 complete original captured native transcript.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cc",
  "-std=c11",
  "-I/path/to/recorded/generic",
  "-I/path/to/recorded/unix",
  "rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/modern-c9-write-trace/probe.c",
  "/path/to/recorded/libtcl.a",
  "-lpthread",
  "-ldl",
  "-lm",
  "-lz",
  "-o",
  "/tmp/original-variable-probe"
]
```

Use each manifested compiler command, provider headers/library and original flags; replace only stale provisioning paths. Run /tmp/original-variable-probe once per selected provider in a fresh process. Require byte-exact complete stdout, recorded stderr and process status; do not omit failed selection or pre/post object observations. Private-header probes additionally require their recorded STDC_HEADERS/HAVE_UNISTD_H flags. Rebuilding changes artifact hashes and is a fresh observation. Jim and BIG-IP are not probed by these C translation units. No Rust equivalence is inferred.
