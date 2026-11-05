# naming.tcloo.original-nonclass-superclass-mixin-errors

Kind: `native-observation`

## Problem statement

A live TclOO object name does not establish a class suitable for superclass or mixin configuration. Each exact definition operation needs its own public rejection result.

## Question

What public errors do the original superclass and mixin controls report when their operand is an ordinary TclOO object?

## Conclusion

C8.6.18/C9.0.4/C9.1.0 return exact original 1 {only a class can be a superclass} 1 {may only mix in classes}. C8.4.20/C8.5.19/Jim return stock-TclOO-unavailable NOT_APPLICABLE. The caught errors do not expose private class allocation, own-method tables or dispatch identity. All six original case processes exit 0 with empty stderr; their complete unchanged counted public VERSION/ORIGINAL rows and six actual driver builds remain retained. No Rust assertion pass or BIG-IP result follows.

## Scope

One unchanged original ASCII LF source is passed directly to the native202 counted-source driver in each fresh full Tcl_Init or Jim core/static-extension interpreter. Original source contains its authored availability branch and catches; the external launcher adds no eval/catch/puts envelope. Public returned guest list bytes and caught codes are retained independently of private objects/frames.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `observed`. Channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; jim 0.84-9-g5bac7c9.

Exact whole original counted result: NOT_APPLICABLE stock-TclOO-unavailable. Outer original completion code0; external process exit0 and empty stderr. The source availability branch is measured; no TclOO operation in that branch is observed.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Exact whole original counted result: 1 {only a class can be a superclass} 1 {may only mix in classes}. Outer original completion code0; external process exit0 and empty stderr. All caught operation codes/value bytes are retained; no private object/header/frame claim follows.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `cmd_oo::tests::define_script_errorinfo_and_super_mixin_messages` (linked): Selected public nonobject/nonclass superclass/mixin diagnostics and renamed definition errorInfo keep the native observation boundaries; no private class-table grant.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_oo_context296](../../../../rust/tcl-registry/tests/data/native_oo_context296)
- [rust/tcl-registry/tests/data/native_oo_context296/8.4.20](../../../../rust/tcl-registry/tests/data/native_oo_context296/8.4.20)
- [rust/tcl-registry/tests/data/native_oo_context296/8.5.19](../../../../rust/tcl-registry/tests/data/native_oo_context296/8.5.19)
- [rust/tcl-registry/tests/data/native_oo_context296/8.6.18](../../../../rust/tcl-registry/tests/data/native_oo_context296/8.6.18)
- [rust/tcl-registry/tests/data/native_oo_context296/9.0.4](../../../../rust/tcl-registry/tests/data/native_oo_context296/9.0.4)
- [rust/tcl-registry/tests/data/native_oo_context296/9.1.0](../../../../rust/tcl-registry/tests/data/native_oo_context296/9.1.0)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_oo_context296/jim](../../../../rust/tcl-registry/tests/data/native_oo_context296/jim)
