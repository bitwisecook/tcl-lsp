# naming.numeric.increment-original-update-free-callbacks

Kind: `native-observation`

## Problem statement

Incrementing an original custom object can enter its updateString or freeIntRep callback, which modifies the caller frame and replaces a later worker. Treating scalar conversion as a quiet value getter would ignore observable effects.

## Question

Do compiled/direct and aliased generic incr routes enter original update/free callbacks before the following caller-cell and worker observations?

## Conclusion

All five C and the original Jim capture return4 and observe UPDATED/FREED caller keep plus the replacement worker CHANGED in both tested routes. C uses compiled incr versus runtime alias; Jim uses direct versus alias. This proves reached native callback effects for the exact custom operands, not a source compiler completion certificate, general closed handler footprint or an emitted optimizer rewrite.

## Scope

Original custom type has either missing bytes/updateString producing3 or resident3/freeIntRep. Callbacks set keep and delete/recreate worker; the procedure then lists result/keep/worker. Four rows per provider from two routes×two hooks, distinct exact C and Jim TUs, full source/build/output associations retained. C initializes its selected library; Jim registers core commands. Runtime patchlevel/revision/header/compiler/configure metadata is unqueried; BIG-IP is not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Custom original Tcl_Obj updateString/freeIntRep reached by compiled incr or alias to incr inside ASCII procedure source. Version: tcl8.4 8.4.20 (capture association; launched patchlevel/revision unqueried); tcl8.5 8.5.19 (capture association; launched patchlevel/revision unqueried); tcl8.6 8.6.18 (capture association; launched patchlevel/revision unqueried); tcl9.0 9.0.4 (capture association; launched patchlevel/revision unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
route=compiled hook=update code=0 result=4 UPDATED CHANGED
route=compiled hook=free code=0 result=4 FREED CHANGED
route=generic hook=update code=0 result=4 UPDATED CHANGED
route=generic hook=free code=0 result=4 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### jim

Status: `observed`. Channel: Custom original Jim_Obj updateString/freeIntRep reached by direct or alias incr; callbacks enabled only during entered source. Version: jim Jim0.84 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
route=direct hook=update code=0 result=4 UPDATED CHANGED
route=direct hook=free code=0 result=4 FREED CHANGED
route=alias hook=update code=0 result=4 UPDATED CHANGED
route=alias hook=free code=0 result=4 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### bigip

Status: `not-tested`.

No native observation of this exact question is attached for this provider.

## Consumer bindings

- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `native_scalar_getter`: Selected getter conversion must retain reached original callback effects and independent result/cache obligations.

## Evidence

- [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks)
