# naming.variable.original-array-observer-renamed-alias-controls

Kind: `native-observation`

## Problem statement

Does renaming or aliasing the original array handler preserve its whole-array read, requiring point-aware compiler observers rather than Registry role lookup by head spelling?

## Question

Do the exact builtin, renamed original array handler and supported interpreter alias return the same k OLD whole-array value, and what is the caught alias-setup result under each retained provider?

## Conclusion

All six providers return k OLD through the builtin and renamed original array handler. All five C releases also set up array_alias successfully and return k OLD through it. Jim rejects interpreter alias setup, so its source has no alias-read result.

## Scope

Six fresh CLI processes retain six actual version rows and 23 observable control fields: twelve direct read results, six caught setup records and five caught alias-read records. These are not 23 caught commands. Original ASCII source and its version-prefixed executed source retain separate exact hashes; all controls share each process in source order. Output is Tcl list presentation of values/status, not native getter bytes or object identity. No trace callback, compiler point-read inventory, dead-store proof, physical cell/header/cache, source-model completion, compiler admission or Native Normal is established. Jim alias setup refusal is separate from its successful builtin/rename reads; BIG-IP is not tested. The linked compiler selector retains an independent implementation purpose and supplies no executed Rust result here.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Fixed ASCII LF source-file CLI, one fresh process per provider. An independently hashed version prefix precedes the original sequential builtin/rename/conditional alias controls; Tcl list output retains exact values and caught setup status. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Builtin and renamed original array get each return k OLD. Interpreter alias setup succeeds with result array_alias, and the caught alias read succeeds with k OLD.

### jim

Status: `observed`. Channel: Fixed ASCII LF source-file CLI, one fresh process per provider. An independently hashed version prefix precedes the original sequential builtin/rename/conditional alias controls; Tcl list output retains exact values and caught setup status. Version: jim 0.84-9-g5bac7c9.

Builtin and renamed original array get each return k OLD. Interpreter alias setup fails with the actual interp arity message; the source skips alias evaluation, so no successful alias read is observed.

### bigip

Status: `not-tested`.

No appliance observation of these exact original array rename/alias controls.

## Consumer bindings

- [rust/tcl-compiler/src/place_bridge.rs](../../../../rust/tcl-compiler/src/place_bridge.rs), `place_bridge::tests::element_observers_follow_original_renamed_and_aliased_array_reads` (linked): Fifteen source controls in five C authoring contexts independently require authentic Native registration ownership, actual selected original array child target, concrete whole-array read inventory and a real element store before point-aware observation. This is a compiler-source assertion, not native transcript replay.

## Exact original-source Compiler software outcomes

The table reports only admitted selectors attached to this question by exact comments in the retained original test bodies. Each result is limited to its named source snapshot, owned Compiler image and selector invocation. No other source/image, unselected assertion, external-provider observation or performance conclusion follows.

| Original operation | Passed | Failed | Incomplete |
| --- | ---: | ---: | ---: |
| `integration-compiler-focused-consumer-tests344` | 0 | 1 | 0 |

Exact original-source body hashes, receipts, streams, inventories and owned images preserve the per-selector events and independent producer/admission limits. The existing provider answers retain their own experimental scope.

## Exact original-source347 software outcomes

These results belong only to the retained source347 body/question pairs and their owning Compiler or Core image. They do not establish an outcome for current source, unselected controls or external providers.

| Original operation | Passed | Failed |
| --- | ---: | ---: |
| `integration-compiler-focused-consumer-tests347` | 0 | 1 |

`Exact original body hashes, producer/list/admission receipts, raw logs and owned images` retain the finite per-selector results and independent failed Runtime producer.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_original_array_observer/8.6.18](../../../../rust/tcl-registry/tests/data/native_original_array_observer/8.6.18)
- [rust/tcl-registry/tests/data/native_original_array_observer/9.1.0](../../../../rust/tcl-registry/tests/data/native_original_array_observer/9.1.0)
- [rust/tcl-registry/tests/data/native_original_array_observer/8.4.20](../../../../rust/tcl-registry/tests/data/native_original_array_observer/8.4.20)
- [rust/tcl-registry/tests/data/native_original_array_observer/9.0.4](../../../../rust/tcl-registry/tests/data/native_original_array_observer/9.0.4)
- [rust/tcl-registry/tests/data/native_original_array_observer/8.5.19](../../../../rust/tcl-registry/tests/data/native_original_array_observer/8.5.19)
- [rust/tcl-registry/tests/data/native_original_array_observer/jim](../../../../rust/tcl-registry/tests/data/native_original_array_observer/jim)
