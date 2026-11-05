# naming.variable.jim-dict-update-reference-controls

Kind: `native-observation`

## Problem statement

The first dictionary mapping assignment, an array-valued second destination, an explicit upvar alias, Jim reference-formal syntax and a direct multiword worker can produce different public results. The exact captured foreach/catch variable-source input channel must remain explicit alongside independently retained specimens.

## Question

How do native dictionary update, array-root links and Jim reference formals reach the first mapped assignment and body?

## Conclusion

The exact five-control foreach/catch variable-source file runs in all six recorded providers with zero process exit and empty stderr. C8.5, C8.6, C9.0, C9.1 and Jim complete the scalar dict update with ok NEW and an entered body. Their two-mapping array case differs: C8.5+ advances ok to NEW, catches the blocked array assignment and leaves the body unentered; Jim returns normally, enters the body and makes blocked readable as scalar OTHER. C8.4 has no dict command in the captured installation, leaving these dict controls unexecuted after the caught command error. The ordinary upvar alias write fails against an array in all five C providers and succeeds with scalar OTHER in Jim. The &original formal produces a caught missing-local error in C and returns {{first NEW}} in Jim. The literal multiword command {dict update} is absent in all C providers and completes the DIRECT update in Jim. The original rows retain every caught result and value. Independent variable-container controls retain the same first-target NEW value in C8.5+. The exact public observations establish no input-envelope causal difference. BIG-IP is not tested. No bytecode optimisation, object/shimmer/cache/header identity, native argv, frame-memory link or caller-storage mechanism is inferred from these public observations.

## Scope

Original fixed ASCII LF CLI file reports PATCHLEVEL, then iterates five exact script values through catch $script. All controls share one fresh provider process but use distinct authored dictionary and alias names. The outer catch code and inner caught code/result/value tuple are separately retained; process exit zero does not imply every guest operation succeeded. Stock C8.4 dictionary absence applies only to this installation, with the alias and reference-formal controls still executed. No raw/counting NUL, non-ASCII, continuation or control escape is exercised. Exact input source, command, executable and required library/header/build/source pins are checked independently. The public variable controls supply no class-carrier authority. Other provider, input envelope and independent fixture results remain independent. No Rust assertion pass, source-carrier authority, representation identity, bytecode causal explanation, actual frame/storage allocation, broad parity or edit authority follows.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original fixed ASCII LF CLI source with five caught public controls; no internal object/header/argv/cache or frame-memory claim. Version: tcl8.4 8.4.20.

Actual stock 8.4.20 lacks dict here: scalar and array controls catch that absence before entering their bodies; alias/reference controls still execute, and the multiword worker is absent. Exact five rows: RESULT update_scalar 0 {1 {invalid command name "dict"} BEFORE 0 {first NEW}}; RESULT update_array_second 0 {1 {invalid command name "dict"} BEFORE 0 1 {can't read "blocked": variable is array} {first NEW second OTHER}}; RESULT alias_array_replacement 0 {1 {can't set "original": variable is array} 1 1 {can't read "aliasSource": variable is array}}; RESULT reference_formal 0 {1 {can't read "original": no such variable} {first NEW}}; RESULT update_direct_multiword 0 {1 {invalid command name "dict update"} BEFORE 0 {first DIRECT}}. Every outer catch code is zero, while each nested operation retains its own code/result/value tuple. Process exit is zero with empty stderr. These foreach/catch variable-source outputs do not replace the distinct older eval-wrapped fixture or establish a bytecode/object/frame/storage causal mechanism.

### tcl8.5

Status: `observed`. Channel: Original fixed ASCII LF CLI source with five caught public controls; no internal object/header/argv/cache or frame-memory claim. Version: tcl8.5 8.5.19.

Actual stock 8.5.19completes the scalar update; the array-second control catches assignment failure after ok becomes NEW. The alias array write and reference-formal read fail, and the multiword worker is absent. Exact five rows: RESULT update_scalar 0 {0 1 NEW 1 {first NEW}}; RESULT update_array_second 0 {1 {can't set "blocked": variable is array} NEW 0 1 {can't read "blocked": variable is array} {first NEW second OTHER}}; RESULT alias_array_replacement 0 {1 {can't set "original": variable is array} 1 1 {can't read "aliasSource": variable is array}}; RESULT reference_formal 0 {1 {can't read "original": no such variable} {first NEW}}; RESULT update_direct_multiword 0 {1 {invalid command name "dict update"} BEFORE 0 {first DIRECT}}. Every outer catch code is zero, while each nested operation retains its own code/result/value tuple. Process exit is zero with empty stderr. These foreach/catch variable-source outputs do not replace the distinct older eval-wrapped fixture or establish a bytecode/object/frame/storage causal mechanism.

### tcl8.6

Status: `observed`. Channel: Original fixed ASCII LF CLI source with five caught public controls; no internal object/header/argv/cache or frame-memory claim. Version: tcl8.6 8.6.18.

Actual stock 8.6.18completes the scalar update; the array-second control catches assignment failure after ok becomes NEW. The alias array write and reference-formal read fail, and the multiword worker is absent. Exact five rows: RESULT update_scalar 0 {0 1 NEW 1 {first NEW}}; RESULT update_array_second 0 {1 {can't set "blocked": variable is array} NEW 0 1 {can't read "blocked": variable is array} {first NEW second OTHER}}; RESULT alias_array_replacement 0 {1 {can't set "original": variable is array} 1 1 {can't read "aliasSource": variable is array}}; RESULT reference_formal 0 {1 {can't read "original": no such variable} {first NEW}}; RESULT update_direct_multiword 0 {1 {invalid command name "dict update"} BEFORE 0 {first DIRECT}}. Every outer catch code is zero, while each nested operation retains its own code/result/value tuple. Process exit is zero with empty stderr. These foreach/catch variable-source outputs do not replace the distinct older eval-wrapped fixture or establish a bytecode/object/frame/storage causal mechanism.

### tcl9.0

Status: `observed`. Channel: Original fixed ASCII LF CLI source with five caught public controls; no internal object/header/argv/cache or frame-memory claim. Version: tcl9.0 9.0.4.

Actual stock 9.0.4completes the scalar update; the array-second control catches assignment failure after ok becomes NEW. The alias array write and reference-formal read fail, and the multiword worker is absent. Exact five rows: RESULT update_scalar 0 {0 1 NEW 1 {first NEW}}; RESULT update_array_second 0 {1 {can't set "blocked": variable is array} NEW 0 1 {can't read "blocked": variable is array} {first NEW second OTHER}}; RESULT alias_array_replacement 0 {1 {can't set "original": variable is array} 1 1 {can't read "aliasSource": variable is array}}; RESULT reference_formal 0 {1 {can't read "original": no such variable} {first NEW}}; RESULT update_direct_multiword 0 {1 {invalid command name "dict update"} BEFORE 0 {first DIRECT}}. Every outer catch code is zero, while each nested operation retains its own code/result/value tuple. Process exit is zero with empty stderr. These foreach/catch variable-source outputs do not replace the distinct older eval-wrapped fixture or establish a bytecode/object/frame/storage causal mechanism.

### tcl9.1

Status: `observed`. Channel: Original fixed ASCII LF CLI source with five caught public controls; no internal object/header/argv/cache or frame-memory claim. Version: tcl9.1 9.1.0.

Actual stock 9.1.0completes the scalar update; the array-second control catches assignment failure after ok becomes NEW. The alias array write and reference-formal read fail, and the multiword worker is absent. Exact five rows: RESULT update_scalar 0 {0 1 NEW 1 {first NEW}}; RESULT update_array_second 0 {1 {can't set "blocked": variable is array} NEW 0 1 {can't read "blocked": variable is array} {first NEW second OTHER}}; RESULT alias_array_replacement 0 {1 {can't set "original": variable is array} 1 1 {can't read "aliasSource": variable is array}}; RESULT reference_formal 0 {1 {can't read "original": no such variable} {first NEW}}; RESULT update_direct_multiword 0 {1 {invalid command name "dict update"} BEFORE 0 {first DIRECT}}. Every outer catch code is zero, while each nested operation retains its own code/result/value tuple. Process exit is zero with empty stderr. These foreach/catch variable-source outputs do not replace the distinct older eval-wrapped fixture or establish a bytecode/object/frame/storage causal mechanism.

### jim

Status: `observed`. Channel: Original fixed ASCII LF CLI source with five caught public controls; no internal object/header/argv/cache or frame-memory claim. Version: jim 0.84-9-g5bac7c9.

Actual Jim 0.84-9-g5bac7c9 completes scalar/array dictionary controls, explicit alias replacement, reference-formal and multiword worker with the exact public tuples below. Exact five rows: RESULT update_scalar 0 {0 1 NEW 1 {first NEW}}; RESULT update_array_second 0 {0 1 NEW 1 0 OTHER {first NEW second OTHER}}; RESULT alias_array_replacement 0 {0 OTHER 0 0 OTHER}; RESULT reference_formal 0 {0 {{first NEW}} {first NEW}}; RESULT update_direct_multiword 0 {0 1 DIRECT 1 {first DIRECT}}. Every outer catch code is zero, while each nested operation retains its own code/result/value tuple. Process exit is zero with empty stderr. These foreach/catch variable-source outputs do not replace the distinct older eval-wrapped fixture or establish a bytecode/object/frame/storage causal mechanism.

### bigip

Status: `not-tested`.

No appliance execution answers this exact source question; C and Jim captures supply no BIG-IP result.

## Consumer bindings

- [runtime/rust/src/cmd_array.rs](../../../../runtime/rust/src/cmd_array.rs), `cmd_array::tests::dictionary_update_first_original_mapping_reaches_caller_storage` (linked): Port distribution test ingress checks the original first-mapping caller-value projection under C8.5+ and Jim; no absent core worker, internal bytecode cause or unmeasured native input is inferred.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_variable_container_runtime187](../../../../rust/tcl-registry/tests/data/native_variable_container_runtime187)
- [rust/tcl-registry/tests/data/native_variable_container_runtime187/8.4.20](../../../../rust/tcl-registry/tests/data/native_variable_container_runtime187/8.4.20)
- [rust/tcl-registry/tests/data/native_variable_container_runtime187/8.5.19](../../../../rust/tcl-registry/tests/data/native_variable_container_runtime187/8.5.19)
- [rust/tcl-registry/tests/data/native_variable_container_runtime187/8.6.18](../../../../rust/tcl-registry/tests/data/native_variable_container_runtime187/8.6.18)
- [rust/tcl-registry/tests/data/native_variable_container_runtime187/9.0.4](../../../../rust/tcl-registry/tests/data/native_variable_container_runtime187/9.0.4)
- [rust/tcl-registry/tests/data/native_variable_container_runtime187/9.1.0](../../../../rust/tcl-registry/tests/data/native_variable_container_runtime187/9.1.0)
- [rust/tcl-registry/tests/data/native_variable_container_runtime187/jim](../../../../rust/tcl-registry/tests/data/native_variable_container_runtime187/jim)
