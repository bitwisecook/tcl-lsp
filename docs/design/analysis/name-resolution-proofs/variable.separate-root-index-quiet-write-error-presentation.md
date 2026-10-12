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

Status: `observed`. Version: 8.4.20. Build: Static native library SHA256 d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; probe SHA256 b911fba871914983a25d074bb68dc2242e7496738abe371c32eda8bdb0e36fa5. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"engine_patchlevel=8.4.20\npurpose=QuietWrite|selected=none|result=SENTINEL\npurpose=Write|selected=none|result=can't set \"::missing::arr(k)\": parent namespace doesn't exist\n"

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Static native library SHA256 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; probe SHA256 bbddbec89c0e37021b914d55a170e0cf692f8e876f4f05b180f3f583803ad631. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"engine_patchlevel=8.5.19\npurpose=QuietWrite|selected=none|result=SENTINEL\npurpose=Write|selected=none|result=can't set \"::missing::arr(k)\": parent namespace doesn't exist\n"

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Static native library SHA256 a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; probe SHA256 8367a7e8640aa7913219a879287a060c26cd75d83221b52e677433af1b3eaa5f. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"engine_patchlevel=8.6.18\npurpose=QuietWrite|selected=none|result=SENTINEL\npurpose=Write|selected=none|result=can't set \"::missing::arr(k)\": parent namespace doesn't exist\n"

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Static native library SHA256 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; probe SHA256 3301924e0577cb8f4fe18a05465711a02e11f6ed0c91b8866d58a4ba95c418a4. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"engine_patchlevel=9.0.4\npurpose=QuietWrite|selected=none|result=SENTINEL\npurpose=Write|selected=none|result=can't set \"::missing::arr(k)\": parent namespace doesn't exist\n"

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Static native library SHA256 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; probe SHA256 1e45ff3b8d338fc51352476f368706116ce16ee18febcf272dd1062bfb9af13b. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"engine_patchlevel=9.1.0\npurpose=QuietWrite|selected=none|result=SENTINEL\npurpose=Write|selected=none|result=can't set \"::missing::arr(k)\": parent namespace doesn't exist\n"

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-manifest.json). SHA-256 `49d48d1288142c7f8cf14a7f732db4791056057f0d8d244381375ce0dc590e7a`. Exact original compiler/header/library/executable hashes, process receipts and native source anchors where supplied.
- `e1` (input): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose.c](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose.c). SHA-256 `efe67f6038caad6144573c9b2cda7031310af2f4463f26a7c0f6d849809c8bdc`. Complete original C translation unit; selected public API and reporter stages are retained.
- `e2` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.4.20.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.4.20.stdout). SHA-256 `a3e053e31e3a13b2f4b2941b494d322e5a6c95080a367a25f6330f77496cc883`. 8.4.20 complete original captured native transcript.
- `e3` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.5.19.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.5.19.stdout). SHA-256 `fd3336675b55faafc973f2d5f2d1ef7f20f5bb10324a4f48695e037e7d9eff3b`. 8.5.19 complete original captured native transcript.
- `e4` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.6.18.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-8.6.18.stdout). SHA-256 `63b30c93ba7d57316f36855a221922e8e56ead2c1d79eea0702ee6d68f68cd1f`. 8.6.18 complete original captured native transcript.
- `e5` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-9.0.4.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-9.0.4.stdout). SHA-256 `83adbed1d6246c132eece05613773d1de05c88dc9454976c2b207700a8e7e7cb`. 9.0.4 complete original captured native transcript.
- `e6` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-9.1.0.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/part2-purpose-9.1.0.stdout). SHA-256 `8dcb3783e29fdc22e7f93116d429378303420be99b124ecf960af45ae06147f8`. 9.1.0 complete original captured native transcript.

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
