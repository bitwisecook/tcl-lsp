# naming.variable.scan-error-result-presentation-and-later-output

Kind: `native-observation`

## Problem statement

A scan destination can fail because its namespace is absent or its write callback errors. The command result object, its string/character getters and later output assignment can differ by release and stage; a canonical error label cannot stand in for the actual object or control path.

## Question

Scan failure result presentation and later output assignment What exact stages and results occur in the retained native probe?

## Conclusion

The retained fifteen C case transcripts distinguish one missing destination, two missing destinations and a failing legacy write trace, with exact completion, result-object stages, result bytes and later output state. Tcl 9 legacy trace syntax rejects before the scan; that row is not a positive Tcl 9 write-callback result. The separate modern C9 control answers that path.

## Scope

Three fixed scripts/provider, C84-91 public evaluation plus private result-header/character cache reporting; no Jim/BIG-IP. Header fields and semantic results remain independent observations.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Static native library SHA256 d71ed42efd90354cdc99f73ba39b30e2cdedda463768a7fd025114b50db60011; probe SHA256 e2bdf9ced144bb07e8e7956d09ea288c6b3c925ce1c3955e4efa8abb62c54228. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"VERSION|8.4.20\nCASE|0\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|636f756c646e277420736574207661726961626c6520223a3a616273656e743a3a666972737422\nAFTER-STRING|string|1|1|-1\nCHARS|39\nAFTER-LENGTH|string|1|1|39\nOUTPUT|2\nCASE|1\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|636f756c646e277420736574207661726961626c6520223a3a616273656e743a3a666972737422636f756c646e277420736574207661726961626c6520223a3a616273656e743a3a7365636f6e6422\nAFTER-STRING|string|1|1|-1\nCHARS|79\nAFTER-LENGTH|string|1|1|79\nOUTPUT|2\nCASE|2\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|636f756c646e277420736574207661726961626c652022666972737422\nAFTER-STRING|string|1|1|-1\nCHARS|29\nAFTER-LENGTH|string|1|1|29\nOUTPUT|2\n"

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Static native library SHA256 99e0d524e54498713498e43d6cf80fa085471913cb38a53b505e18c0469417df; probe SHA256 f437c0e0b0f122b3feb3cfa1ad621348b14498ce45a508972caf971260bed15f. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"VERSION|8.5.19\nCASE|0\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|636f756c646e277420736574207661726961626c6520223a3a616273656e743a3a666972737422\nAFTER-STRING|string|1|1|-1\nCHARS|39\nAFTER-LENGTH|string|1|1|39\nOUTPUT|2\nCASE|1\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|636f756c646e277420736574207661726961626c6520223a3a616273656e743a3a666972737422636f756c646e277420736574207661726961626c6520223a3a616273656e743a3a7365636f6e6422\nAFTER-STRING|string|1|1|-1\nCHARS|79\nAFTER-LENGTH|string|1|1|79\nOUTPUT|2\nCASE|2\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|636f756c646e277420736574207661726961626c652022666972737422\nAFTER-STRING|string|1|1|-1\nCHARS|29\nAFTER-LENGTH|string|1|1|29\nOUTPUT|2\n"

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Static native library SHA256 a3a8abdeadd8aafa007d3bc9fa199b9cf48d43d01735ff41b3fcf33abb4019d6; probe SHA256 ec04db8fbebd996fac0caee301ccdf77e51cdfe3a071157646dd0e0d295a0526. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"VERSION|8.6.18\nCASE|0\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420223a3a616273656e743a3a6669727374223a20706172656e74206e616d65737061636520646f65736e2774206578697374\nAFTER-STRING|string|1|1|-1\nCHARS|59\nAFTER-LENGTH|string|1|1|59\nOUTPUT|2\nCASE|1\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420223a3a616273656e743a3a6669727374223a20706172656e74206e616d65737061636520646f65736e2774206578697374\nAFTER-STRING|string|1|1|-1\nCHARS|59\nAFTER-LENGTH|string|1|1|59\nOUTPUT|2\nCASE|2\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420226669727374223a20424f4f4d\nAFTER-STRING|string|1|1|-1\nCHARS|23\nAFTER-LENGTH|string|1|1|23\nOUTPUT|2\n"

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Static native library SHA256 2ba08ecf7197e16d99e303c0d95dde29c59f391a22fd02378edc2697567e0e04; probe SHA256 786d5e386b4a4b656f52d5afe441c62b9e622de5efd7bbe05dad113a0fc6c133. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"VERSION|9.0.4\nCASE|0\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420223a3a616273656e743a3a6669727374223a20706172656e74206e616d65737061636520646f65736e2774206578697374\nAFTER-STRING|string|1|1|-1\nCHARS|59\nAFTER-LENGTH|string|1|1|59\nOUTPUT|2\nCASE|1\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420223a3a616273656e743a3a6669727374223a20706172656e74206e616d65737061636520646f65736e2774206578697374\nAFTER-STRING|string|1|1|-1\nCHARS|59\nAFTER-LENGTH|string|1|1|59\nOUTPUT|2\nCASE|2\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|626164206f7074696f6e20227661726961626c65223a206d757374206265206164642c20696e666f2c206f722072656d6f7665\nAFTER-STRING|string|1|1|-1\nCHARS|51\nAFTER-LENGTH|string|1|1|51\nOUTPUT|2\n"

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Static native library SHA256 8b2dba836908287f95f26a442bb065601daac300fdeee48b15e64a5c03894e33; probe SHA256 8b206d9fc4d0b1facca6a59e64ceda20286c0f69c5580b1a8182017018252d39. Channel: Original native object/C API construction and recorded C evaluation, as specified by probe.c; not a document ingress. Dialect: Tcl.

Exact captured transcript:

"VERSION|9.1.0\nCASE|0\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420223a3a616273656e743a3a6669727374223a20706172656e74206e616d65737061636520646f65736e2774206578697374\nAFTER-STRING|string|1|1|-1\nCHARS|59\nAFTER-LENGTH|string|1|1|59\nOUTPUT|2\nCASE|1\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|63616e27742073657420223a3a616273656e743a3a6669727374223a20706172656e74206e616d65737061636520646f65736e2774206578697374\nAFTER-STRING|string|1|1|-1\nCHARS|59\nAFTER-LENGTH|string|1|1|59\nOUTPUT|2\nCASE|2\nCODE|1\nBEFORE-STRING|string|1|1|-1\nHEX|626164206f7074696f6e20227661726961626c65223a206d757374206265206164642c20696e666f2c206f722072656d6f7665\nAFTER-STRING|string|1|1|-1\nCHARS|51\nAFTER-LENGTH|string|1|1|51\nOUTPUT|2\n"

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/manifest.json](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/manifest.json). SHA-256 `f968d1f1f56c58e024b7031e36f9c2d7e70f650d03a14f2a32c1fb6ad04c1cd1`. Exact original compiler/header/library/executable hashes, process receipts and native source anchors where supplied.
- `e1` (input): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/probe.c](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/probe.c). SHA-256 `aef5792cdeb95ce9150ea40a004fdd42f59ee74aade5a7c17c72d95b4e663a20`. Complete original C translation unit; selected public API and reporter stages are retained.
- `e2` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/8.4.20-run.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/8.4.20-run.stdout). SHA-256 `6ab89933f7a3d4af625c31481b6ba817966727ff134b4b818d6861c8c19c4b63`. 8.4.20 complete original captured native transcript.
- `e3` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/8.5.19-run.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/8.5.19-run.stdout). SHA-256 `c73131461327172dcc74df263f984a80f9ccc0974e4966dd95a58368f0c979fd`. 8.5.19 complete original captured native transcript.
- `e4` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/8.6.18-run.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/8.6.18-run.stdout). SHA-256 `c95356a303ce44747d05d50b7ea0a956ad32fc2318c181f42d68de7250f24a6f`. 8.6.18 complete original captured native transcript.
- `e5` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/9.0.4-run.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/9.0.4-run.stdout). SHA-256 `1b566d3a2759ec9bef83e8e0ed39b617832ce36e206e4cf960094019d8f224e3`. 9.0.4 complete original captured native transcript.
- `e6` (observation): [rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/9.1.0-run.stdout](../../../../rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/9.1.0-run.stdout). SHA-256 `7ff3dbbcc9243f18e10ba7efcd6216caf9e1010ebc7c90d282780d1fb53cc373`. 9.1.0 complete original captured native transcript.

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
  "rust/tcl-syntax/tests/data/native_generic_variable_consumers/scan-presenter/probe.c",
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
