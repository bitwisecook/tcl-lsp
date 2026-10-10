# naming.variable.uplevel-original-compilation-opcode-and-concat-frontier

Kind: `native-observation`

## Problem statement

Uplevel can be a generic command or a retained opcode with independently selected frame/script stack geometry. An explicit/default literal level and multi-fragment script affect compiler selection; assuming that a portable operation represents a native opcode on every release loses the original boundary.

## Question

Do the six retained explicit/default/concat/dynamic/invalid/sole bodies emit uplevel and concat instructions under C9.0 and C9.1?

## Conclusion

C9.0 emits neither opcode for all six captured bodies. C9.1 emits uplevel for the explicit/default/concat bodies, concat only for the multi-fragment body, and neither for dynamic/invalid/sole variants. These12 recorded artifact frontiers establish compiler stack/selection geometry only, not level validation, body completion, frame identity or arbitrary portable/native correspondence.

## Scope

Exact twelve ASCII source/artifact entries for C9.0.4 and C9.1.0, binary and wrapper-source hashes plus retained instructions.tsv. Original source/log wrappers are referenced outside the repository and are absent here. Earlier C releases, Jim and BIG-IP not queried for this compilation question; no unsupported inference.

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

Status: `observed`. Version: 9.0.4. Build: Shell SHA cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18; full build/source wrapper not retained. Channel: ASCII procedure source through original compiler artifact presenter. Dialect: Tcl.

Six completed artifact captures; exact retained selection flags and wrapper provenance:

```json
[
  {
    "engine": "9.0",
    "label": "explicit",
    "source": "/workspace/.proofs/2286-body-native-uplevel91/9.0-explicit.tcl",
    "source_sha256": "2a21355a3aba921b798a1499b317457bc707592a723f76a0e50ecfbdc6243620",
    "exit": 0,
    "log": "/workspace/.proofs/2286-body-native-uplevel91/9.0-explicit.log",
    "uplevel": false,
    "concat": false,
    "binary_sha256": "cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18"
  },
  {
    "engine": "9.0",
    "label": "default",
    "source": "/workspace/.proofs/2286-body-native-uplevel91/9.0-default.tcl",
    "source_sha256": "b0e93e289f9b829f889fe169f3fae484f118fd7b85ee177bef520c6a37328b57",
    "exit": 0,
    "log": "/workspace/.proofs/2286-body-native-uplevel91/9.0-default.log",
    "uplevel": false,
    "concat": false,
    "binary_sha256": "cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18"
  },
  {
    "engine": "9.0",
    "label": "concat",
    "source": "/workspace/.proofs/2286-body-native-uplevel91/9.0-concat.tcl",
    "source_sha256": "7094b8fa51d60011e17a5d0e89c8df3bbf07cae91ae5d7dadc081e2bb0fea265",
    "exit": 0,
    "log": "/workspace/.proofs/2286-body-native-uplevel91/9.0-concat.log",
    "uplevel": false,
    "concat": false,
    "binary_sha256": "cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18"
  },
  {
    "engine": "9.0",
    "label": "dynamic",
    "source": "/workspace/.proofs/2286-body-native-uplevel91/9.0-dynamic.tcl",
    "source_sha256": "93a07b4bb538531ea464758b2fbca650b925114a4351b991af60c8d36708f4a7",
    "exit": 0,
    "log": "/workspace/.proofs/2286-body-native-uplevel91/9.0-dynamic.log",
    "uplevel": false,
    "concat": false,
    "binary_sha256": "cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18"
  },
  {
    "engine": "9.0",
    "label": "invalid",
    "source": "/workspace/.proofs/2286-body-native-uplevel91/9.0-invalid.tcl",
    "source_sha256": "1e44003e6eb400c1b207aad5713c4abd8ca9be4fd12675302837327bb3494ee2",
    "exit": 0,
    "log": "/workspace/.proofs/2286-body-native-uplevel91/9.0-invalid.log",
    "uplevel": false,
    "concat": false,
    "binary_sha256": "cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18"
  },
  {
    "engine": "9.0",
    "label": "sole",
    "source": "/workspace/.proofs/2286-body-native-uplevel91/9.0-sole.tcl",
    "source_sha256": "c8e14cdfd7d5aa7a161b35c51efefc02bb1e2dd353b0f40b3b4aa5f0deefe8d8",
    "exit": 0,
    "log": "/workspace/.proofs/2286-body-native-uplevel91/9.0-sole.log",
    "uplevel": false,
    "concat": false,
    "binary_sha256": "cc28851d04548dce27006a317d22b78c67e5491204f66f924c20d1f3febf6c18"
  }
]
```

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Shell SHA d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c; full build/source wrapper not retained. Channel: ASCII procedure source through original compiler artifact presenter. Dialect: Tcl.

Six completed artifact captures; exact retained selection flags and wrapper provenance:

```json
[
  {
    "engine": "9.1",
    "label": "explicit",
    "source": "/workspace/.proofs/2286-body-native-uplevel91/9.1-explicit.tcl",
    "source_sha256": "2a21355a3aba921b798a1499b317457bc707592a723f76a0e50ecfbdc6243620",
    "exit": 0,
    "log": "/workspace/.proofs/2286-body-native-uplevel91/9.1-explicit.log",
    "uplevel": true,
    "concat": false,
    "binary_sha256": "d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c"
  },
  {
    "engine": "9.1",
    "label": "default",
    "source": "/workspace/.proofs/2286-body-native-uplevel91/9.1-default.tcl",
    "source_sha256": "b0e93e289f9b829f889fe169f3fae484f118fd7b85ee177bef520c6a37328b57",
    "exit": 0,
    "log": "/workspace/.proofs/2286-body-native-uplevel91/9.1-default.log",
    "uplevel": true,
    "concat": false,
    "binary_sha256": "d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c"
  },
  {
    "engine": "9.1",
    "label": "concat",
    "source": "/workspace/.proofs/2286-body-native-uplevel91/9.1-concat.tcl",
    "source_sha256": "7094b8fa51d60011e17a5d0e89c8df3bbf07cae91ae5d7dadc081e2bb0fea265",
    "exit": 0,
    "log": "/workspace/.proofs/2286-body-native-uplevel91/9.1-concat.log",
    "uplevel": true,
    "concat": true,
    "binary_sha256": "d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c"
  },
  {
    "engine": "9.1",
    "label": "dynamic",
    "source": "/workspace/.proofs/2286-body-native-uplevel91/9.1-dynamic.tcl",
    "source_sha256": "93a07b4bb538531ea464758b2fbca650b925114a4351b991af60c8d36708f4a7",
    "exit": 0,
    "log": "/workspace/.proofs/2286-body-native-uplevel91/9.1-dynamic.log",
    "uplevel": false,
    "concat": false,
    "binary_sha256": "d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c"
  },
  {
    "engine": "9.1",
    "label": "invalid",
    "source": "/workspace/.proofs/2286-body-native-uplevel91/9.1-invalid.tcl",
    "source_sha256": "1e44003e6eb400c1b207aad5713c4abd8ca9be4fd12675302837327bb3494ee2",
    "exit": 0,
    "log": "/workspace/.proofs/2286-body-native-uplevel91/9.1-invalid.log",
    "uplevel": false,
    "concat": false,
    "binary_sha256": "d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c"
  },
  {
    "engine": "9.1",
    "label": "sole",
    "source": "/workspace/.proofs/2286-body-native-uplevel91/9.1-sole.tcl",
    "source_sha256": "c8e14cdfd7d5aa7a161b35c51efefc02bb1e2dd353b0f40b3b4aa5f0deefe8d8",
    "exit": 0,
    "log": "/workspace/.proofs/2286-body-native-uplevel91/9.1-sole.log",
    "uplevel": false,
    "concat": false,
    "binary_sha256": "d4c2dbd11cd88994b7ca992eb87cdc855750f75d5d9f42b681b4dd4c321c791c"
  }
]
```

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No appliance observation for this question.

## Exact evidence

- `e0` (provider): [rust/tcl-registry/tests/data/native_uplevel_compilation/provenance.json](../../../../rust/tcl-registry/tests/data/native_uplevel_compilation/provenance.json). SHA-256 `680808f88cec21780f182ba04dadfdd6985730e604ffb9f134aff3b92b05867d`. 12 original shell artifact records, source/binary hashes and opcode flags.
- `e1` (observation): [rust/tcl-registry/tests/data/native_uplevel_compilation/instructions.tsv](../../../../rust/tcl-registry/tests/data/native_uplevel_compilation/instructions.tsv). SHA-256 `a20ae4aae4b2e7d6eb5a145366d379b2f89a7d9e565e2f6e08e2dc4364a1cbc1`. Exact six body spellings and opcode/concat answers per queried provider.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/native_instruction_plan.rs](../../../../rust/tcl-registry/src/native_instruction_plan.rs), `original_uplevel_instruction_keeps_level_and_script_stack_geometry`: Independent current Rust comparison at the stated semantic boundary.
- [rust/tcl-registry/src/native_instruction_plan.rs](../../../../rust/tcl-registry/src/native_instruction_plan.rs), `native_instruction_plan::tests::original_uplevel_instruction_keeps_level_and_script_stack_geometry` (linked): Checks actual C9.1 level/script/concat operands and refusal variants against the original frontier table.

A named test is a coverage binding, not a claim that it executed.

## Replay

The original per-case procedure/disassembler wrappers and artifact logs are not retained, only their hashes/flags and exact body table. A fresh replay must retain a matching-build original procedure disassembler and complete source for all six bodies, then compare uplevel/concat presence independently in C9.0 and C9.1. Running instructions.tsv as Tcl or assuming a generic call implies this opcode is invalid.
