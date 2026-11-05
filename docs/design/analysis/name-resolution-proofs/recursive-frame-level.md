# naming.optimiser.recursive-frame-level

Kind: `native-observation`

## Problem statement

Recursive calls push frames while a loop reuses one frame. An optimization that changes this topology can alter frame introspection even when arguments and normal result flow otherwise look similar. These fixed public evaluator controls isolate info level, with ASCII source and no custom objects.

## Question

Does replacing the measured recursive procedure with its authored loop preserve the info level result?

## Conclusion

All six tested providers return 3 from the original recursion and 1 from the authored loop. This proves a frame-introspection difference for the two exact programs; it is not an emitted optimizer edit or a general tailcall equivalence proof.

## Scope

Exact original and authored-loop programs in both retained ASCII public C Tcl_Eval/Jim_Eval inputs. These native programs are specification comparisons, not transformations extracted from a passing Rust editor. All provider process exits0 are harness completion; each row retains its independent guest completion and result bytes. No BIG-IP observation, pointer across processes or unknown object representation claim.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `observed`. Channel: ASCII source embedded in NUL-terminated C literals passed directly to Tcl_Eval/Jim_Eval; no file/character channel conversion. Custom result objects use public object APIs. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0; jim 0.84-9-g5bac7c9.

info level original3 versus loop1.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-compiler/src/optimiser/tail_call.rs](../../../../rust/tcl-compiler/src/optimiser/tail_call.rs), `optimiser::tail_call::tests::original_frame_observation_and_release_obligations_withdraw_tail_rewrites` (linked): Require original frame observer/argv object release and producer obligations separately from exact self allocation and formal topology.

## Evidence

- [rust/tcl-compiler/tests/data/native_tail_frame_release/v1](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v1)
- [rust/tcl-compiler/tests/data/native_tail_frame_release/v2](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v2)
