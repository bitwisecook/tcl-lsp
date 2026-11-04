# naming.variable.separate-root-index-quiet-write-error-presentation

Kind: `native-observation`

## Problem statement

A missing namespace can prevent a write before value installation. Quiet flags and error-reporting flags may select the same failure but preserve different interpreter results; successful value mutation cannot be inferred from receiver-name conversion.

## Question

Quiet versus error-reporting separate root/index Tcl_ObjSetVar2 lookup What exact stages and results occur in the retained native probe?

## Conclusion

All five C releases reject the missing namespace in both flag modes. Quiet selection preserves SENTINEL; TCL_LEAVE_ERR_MSG publishes the exact array diagnostic. This is a failed lookup/presentation control, not a successful store or normal-completion proof.

## Scope

Two original root/index string-object writes per C release, fresh interpreter/result sentinel, public Tcl_ObjSetVar2, ten output tuples; no Jim/BIG-IP.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Static native library; probe. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"engine_patchlevel=8.4.20\npurpose=QuietWrite|selected=none|result=SENTINEL\npurpose=Write|selected=none|result=can't set \"::missing::arr(k)\": parent namespace doesn't exist\n"

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Static native library; probe. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"engine_patchlevel=8.5.19\npurpose=QuietWrite|selected=none|result=SENTINEL\npurpose=Write|selected=none|result=can't set \"::missing::arr(k)\": parent namespace doesn't exist\n"

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Static native library; probe. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"engine_patchlevel=8.6.18\npurpose=QuietWrite|selected=none|result=SENTINEL\npurpose=Write|selected=none|result=can't set \"::missing::arr(k)\": parent namespace doesn't exist\n"

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Static native library; probe. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"engine_patchlevel=9.0.4\npurpose=QuietWrite|selected=none|result=SENTINEL\npurpose=Write|selected=none|result=can't set \"::missing::arr(k)\": parent namespace doesn't exist\n"

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Static native library; probe. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"engine_patchlevel=9.1.0\npurpose=QuietWrite|selected=none|result=SENTINEL\npurpose=Write|selected=none|result=can't set \"::missing::arr(k)\": parent namespace doesn't exist\n"

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Historical references

- `e0` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-manifest.json). Exact original compiler/header/library/executable hashes, process receipts and native source anchors where supplied.
- `e1` (input): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose.c](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose.c). Complete original C translation unit; selected public API and reporter stages are retained.
- `e2` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.4.20.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.4.20.stdout). 8.4.20 complete original captured native transcript.
- `e3` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.5.19.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.5.19.stdout). 8.5.19 complete original captured native transcript.
- `e4` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.6.18.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.6.18.stdout). 8.6.18 complete original captured native transcript.
- `e5` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-9.0.4.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-9.0.4.stdout). 9.0.4 complete original captured native transcript.
- `e6` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-9.1.0.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-9.1.0.stdout). 9.1.0 complete original captured native transcript.

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
  "rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose.c",
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
