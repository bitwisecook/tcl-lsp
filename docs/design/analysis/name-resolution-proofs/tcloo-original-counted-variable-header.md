# naming.tcloo.original-counted-variable-header

Kind: `native-observation`

## Problem statement

Equal printed variable names cannot establish preservation of the original counted object. A direct declaration/query control must retain its original allocation, public query and explicit same-object comparison separately from byte equality.

## Question

Does direct TclOO variable declaration with the original counted a-NUL-z object return that same original object through info class variables, and what counted bytes does the direct query retain?

## Conclusion

C8.6.18/C9.0.4/C9.1.0 record DECLARE0 and QUERY0/list-parse 0/count 1/same-object 1 with counted bytes61007a. The same-object Boolean directly compares the sole public query list element pointer with the original allocated declaration object; it is not inferred from equal text. C8.4.20/C8.5.19/Jim record the original stock-TclOO-unavailable NOT_APPLICABLE branch. All six external processes and six actual compiler builds exit 0; stderr is empty. This exact local pointer comparison and byte getter observe only this original declaration/query object within one fresh interpreter. No cross-interpreter object equality, header fields/type/cache equivalence, source-word materialisation, physical variable store/read, class lifetime/method dispatch or compiler/frame/admission follows. BIG-IP and Rust assertions are not executed by this proof.

## Scope

The original ASCII LF C driver creates a public Tcl_NewStringObj from three original bytes61 00 7a, keeps declaration/query argv references, calls Tcl_EvalObjv(flags0) separately for oo::define C variable and info class variables, calls Tcl_ListObjGetElements on the direct public result and compares its sole member pointer with the original. Counted result bytes use Tcl_GetStringFromObj. C providers use full Tcl_Init; Jim core/static initialisation records its measured unavailable branch. No textual Tcl source constructor substitutes for the original counted object; no addresses or private fields are emitted.

## Provider answers

### tcl8.4, tcl8.5, jim

Status: `observed`. Channel: Direct original counted-object Tcl_EvalObjv argv4/flags0, direct public Tcl_ListObjGetElements comparison and counted getter; measured Jim unavailable branch. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; jim 0.84-9-g5bac7c9.

Exact original stock-TclOO-unavailable NOT_APPLICABLE branch. No original direct declaration/query or object-comparison Boolean is observed; no absence of arbitrary extension API is inferred. External process exit 0/empty stderr; whole unchanged VERSION/DECLARE/QUERY or NOT_APPLICABLE rows retained.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Direct original counted-object Tcl_EvalObjv argv4/flags0, direct public Tcl_ListObjGetElements comparison and counted getter; measured Jim unavailable branch. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Exact public DECLARE code 0; direct QUERY code 0, list parser code 0, element count 1, direct sole-member/original pointer-comparison Boolean 1, counted getter bytes61 00 7a. The explicit Boolean observes this original object within the same interpreter only; equal bytes alone are not its authority. External process exit 0/empty stderr; whole unchanged VERSION/DECLARE/QUERY or NOT_APPLICABLE rows retained.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `cmd_oo::tests::variable_declaration_introspection_retains_original_counted_object` (linked): Actual NativeCore C8.6/C9.0/C9.1 Runtime declaration/query control compares the retained original counted object directly, independently of byte equality or arbitrary Logical repinning. This binding claims no executed Rust outcome.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `cmd_oo::tests::unknown_method_preserves_original_selector_for_returned_slot_data` (linked): A separate Rust dispatcher control keeps the actual original counted selector header through an overridable unknown method. Native300 directly observed only the declaration/query boundary; no extra native intermediate observer is claimed.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 2 | 0 | 0 | 0 |
| `isolated` | 2 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_oo_variable_header300/8.4.20](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.4.20)
- [rust/tcl-registry/tests/data/native_oo_variable_header300/8.5.19](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.5.19)
- [rust/tcl-registry/tests/data/native_oo_variable_header300/8.6.18](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/8.6.18)
- [rust/tcl-registry/tests/data/native_oo_variable_header300/9.0.4](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/9.0.4)
- [rust/tcl-registry/tests/data/native_oo_variable_header300/9.1.0](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/9.1.0)
- [rust/tcl-registry/tests/data/native_oo_variable_header300](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300)
- [rust/tcl-registry/tests/data/native_oo_variable_header300/jim](../../../../rust/tcl-registry/tests/data/native_oo_variable_header300/jim)
