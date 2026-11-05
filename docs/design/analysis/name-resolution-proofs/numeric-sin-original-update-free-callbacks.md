# naming.numeric.sin-original-update-free-callbacks

Kind: `native-observation`

## Problem statement

An expression function may evaluate an original custom numeric operand whose representation callback changes the caller cell and replaces a later worker. The function result and callback effects are independent; treating conversion as quiet would lose the latter.

## Question

Do sin expression routes enter the original update/free callbacks and expose the changed caller cell and later worker?

## Conclusion

Every measured sin route returns code0, the retained native function result, UPDATED/FREED keep and CHANGED worker in C and Jim. C compiled expr and runtime alias are separate from Jim direct/alias expr. The actual function result formatting is release-specific and retained verbatim. These operand callback effects do not prove actual selected-handler replacement during this same function call or an optimizer eligibility check.

## Scope

Original custom object emits3 from updater or already owns resident3 with a freer; callbacks write keep and replace worker, not the math function. One fixed sin operation has four exact route/hook windows per provider within full16-row C/Jim captures. Distinct TUs/source/program/library/executable/full outputs retained; runtime patchlevel, Jim revision, headers/compiler/configure details not recorded. BIG-IP is not tested.

## Provider answers

### tcl8.4

Status: `observed`. Channel: Original custom C object in compiled or alias expr function operand; following caller variable/worker source observations. Version: tcl8.4 8.4.20 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
operation=sin route=compiled hook=update code=0 result=0.14112000806 UPDATED CHANGED
operation=sin route=compiled hook=free code=0 result=0.14112000806 FREED CHANGED
operation=sin route=generic hook=update code=0 result=0.14112000806 UPDATED CHANGED
operation=sin route=generic hook=free code=0 result=0.14112000806 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Original custom C object in compiled or alias expr function operand; following caller variable/worker source observations. Version: tcl8.5 8.5.19 (capture association; launched patchlevel/revision unqueried); tcl8.6 8.6.18 (capture association; launched patchlevel/revision unqueried); tcl9.0 9.0.4 (capture association; launched patchlevel/revision unqueried); tcl9.1 9.1.0 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
operation=sin route=compiled hook=update code=0 result=0.1411200080598672 UPDATED CHANGED
operation=sin route=compiled hook=free code=0 result=0.1411200080598672 FREED CHANGED
operation=sin route=generic hook=update code=0 result=0.1411200080598672 UPDATED CHANGED
operation=sin route=generic hook=free code=0 result=0.1411200080598672 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### jim

Status: `observed`. Channel: Original custom Jim object in direct or alias expr function operand; following caller variable/worker source observations. Version: jim Jim0.84 (capture association; launched patchlevel/revision unqueried).

Exact relevant original rows:

```text
operation=sin route=direct hook=update code=0 result=0.14112000806 UPDATED CHANGED
operation=sin route=direct hook=free code=0 result=0.14112000806 FREED CHANGED
operation=sin route=alias hook=update code=0 result=0.14112000806 UPDATED CHANGED
operation=sin route=alias hook=free code=0 result=0.14112000806 FREED CHANGED
```

Fields, input case indices and observer order are defined by the retained probe. No current object or compiler capability follows from this capture.

### bigip

Status: `not-tested`.

No native observation of this exact question is attached for this provider.

## Consumer bindings

- [runtime/rust/src/typed_value.rs](../../../../runtime/rust/src/typed_value.rs), `scalar_number`: Original numeric preparation must preserve reached custom operand callbacks independently from the later math result.

## Evidence

- [rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks](../../../../rust/tcl-syntax/tests/data/native_scalar_math/operand_hooks)
