# naming.numeric.sqrt-original-function-selection-timing

Kind: `native-observation`

## Problem statement

An operand update/free callback can replace sqrt itself while the entered expression is preparing its original argument. The chosen function may already be selected; a later global replacement is not proof that this same call used the new worker.

## Question

After original numeric operand callbacks replace sqrt with a handler returning99, which sqrt result is returned by compiled and generic expr routes?

## Conclusion

All five C releases return their original sqrt(3) result rather than99 while keep confirms UPDATED/FREED. Both compiled and alias expr routes therefore preserve the observed original selected function for this call despite the reached replacement. C8.4 replaces its registered math function and later C replaces ::tcl::mathfunc::sqrt. This narrow timing observation does not imply general command selection order, no callback effects, or a successful Rust rewrite.

## Scope

Four original custom operand route/hook windows per C release with exact replacement-handler99 controls. C8.4 public Tcl_CreateMathFunc replacement versus C8.5+ object command delete/create are distinct source branches. Full embedded captured source/output/library/executable associations/run0 retained; header/compiler/configure/runtime patch query absent. Jim and BIG-IP are not tested for this exact replacement-timing question.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Custom original C numeric operand update/free callback replaces selected math function during compiled or alias expr evaluation. Version: tcl8.4 8.4.20 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
route=compiled hook=update code=0 result=1.73205080757 UPDATED
route=compiled hook=free code=0 result=1.73205080757 FREED
route=generic hook=update code=0 result=1.73205080757 UPDATED
route=generic hook=free code=0 result=1.73205080757 FREED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Custom original C numeric operand update/free callback replaces selected math function during compiled or alias expr evaluation. Version: tcl8.5 8.5.19 (capture association; launched patchlevel/revision unqueried); tcl8.6 8.6.18 (capture association; launched patchlevel/revision unqueried); tcl9.0 9.0.4 (capture association; launched patchlevel/revision unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
route=compiled hook=update code=0 result=1.7320508075688772 UPDATED
route=compiled hook=free code=0 result=1.7320508075688772 FREED
route=generic hook=update code=0 result=1.7320508075688772 UPDATED
route=generic hook=free code=0 result=1.7320508075688772 FREED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### jim, bigip

Status: `not-tested`.

No native observation of this exact question is attached for this provider.

## Consumer bindings

- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `scalar_number`: Separates original operand conversion effects from the independently selected entered function recipe.

## Evidence

- [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing.c](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing.c)
- [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-manifest.json](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-manifest.json)
- [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-8.4.20.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-8.4.20.txt)
- [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-8.5.19.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-8.5.19.txt)
- [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-8.6.18.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-8.6.18.txt)
- [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-9.0.4.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-9.0.4.txt)
- [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-9.1.0.txt](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks/math-timing-9.1.0.txt)
