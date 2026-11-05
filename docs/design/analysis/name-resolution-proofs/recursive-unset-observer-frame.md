# naming.optimiser.recursive-unset-observer-frame

Kind: `native-observation`

## Problem statement

Procedure-frame deletion unsets local variables and can run callbacks. Reusing one activation changes both when those callbacks run and the frame visible to a callback. The original probe uses legacy trace syntax, while v2 selects trace add on C8.5+; unsupported guest operations remain recorded separately.

## Question

Do the recorded unset observers see the same callback frame levels after the recursive-to-loop replacement?

## Conclusion

With the supported C trace forms, all five C releases record original tag/frame pairs {0 2}, {1 1}, {2 0}, versus loop {0 0}, {1 0}, {2 0}. Jim rejects the trace operation in both variants. C9 v1 rejects the removed legacy form; v2 supplies its independently captured supported form. These are callback-frame observations, not a general trace-registration or Native entry grant.

## Scope

Exact original and authored-loop programs in both retained ASCII public C Tcl_Eval/Jim_Eval inputs. These native programs are specification comparisons, not transformations extracted from a passing Rust editor. All provider process exits0 are harness completion; each row retains its independent guest completion and result bytes. No BIG-IP observation, pointer across processes or unknown object representation claim.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII source embedded in NUL-terminated C literals passed directly to Tcl_Eval/Jim_Eval; no file/character channel conversion. Custom result objects use public object APIs. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

v2 C trace tag/frame original(0,2),(1,1),(2,0) versus loop(0,0),(1,0),(2,0); Jim operation unavailable.

### jim

Status: `unsupported`. Channel: ASCII source embedded in NUL-terminated C literals passed directly to Tcl_Eval/Jim_Eval; no file/character channel conversion. Custom result objects use public object APIs. Version: jim 0.84-9-g5bac7c9.

Both exact variants return guest Error for trace; callback frames are unmeasured.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-compiler/src/optimiser/tail_call.rs](../../../../rust/tcl-compiler/src/optimiser/tail_call.rs), `optimiser::tail_call::tests::original_frame_observation_and_release_obligations_withdraw_tail_rewrites` (linked): Require original frame observer/argv object release and producer obligations separately from exact self allocation and formal topology.

## Evidence

- [rust/tcl-compiler/tests/data/native_tail_frame_release/v1](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v1)
- [rust/tcl-compiler/tests/data/native_tail_frame_release/v2](../../../../rust/tcl-compiler/tests/data/native_tail_frame_release/v2)
