# Retired variable diagnostics: retained caller context

## Question

Where does Runtime lose the distinction between a substituted script evaluated
by a compiled instruction and an ordinary original-object command, and can the
three unchanged failing controls be repaired without widening naming policy?

## Evidence

[The original C/Jim question](../retired-namespace-variable-diagnostic-question/README.md)
and [the unchanged Runtime controls](validation-01/README.md) retain the measured
results. C86 reports `TCL WRITE VARNAME` for scalar writes, three-word
`TCL LOOKUP VARNAME` for compiled element writes, and four-word
`TCL LOOKUP VARNAME foo` for ordinary element writes. Runtime's dynamic cases
retain the seeded `NH UNSET`; its ordinary element cases report WRITE instead
of LOOKUP. All three Runtime controls failed. This review ran no execution,
instrumentation or build and adds no pass claim.

### Substituted compiled catch body

The selected Registry `NativeControlBody` already retains the distinction:
`script: None` requires `EVAL_STK` semantics
(`rust/tcl-registry/src/native_control_instructions.rs`). The Catch executor
constructs its original substituted operand before the protected range, then
calls `eval_generic_control_body` for this instruction
(`runtime/rust/src/interp/native_body_artifact/native_control.rs:1298`).

That generic entry calls `eval_original_body_framed`. It asks
`NativeEvalObjectProtocol::dispatches_list` before considering native source
compilation. A canonical original List therefore enters `dispatch_list_obj`
and dispatches an ordinary command instead of compiling the substituted
script. The actual instruction provenance is lost at this choice; a genuine
List header does not authorise replacing a selected `EVAL_STK` instruction.

The pinned C86 source distinguishes these entries explicitly:

- `generic/tclCompCmds.c`, `TclCompileCatchCmd`: a substituted body emits
  `INST_EVAL_STK` after `BEGIN_CATCH4`; its substitution remains outside the
  protected range.
- `generic/tclExecute.c:3011`: `INST_EVAL_STK` invokes `TclCompileObj` and
  `TclNRExecuteByteCode` directly.
- `generic/tclBasic.c`, `TclNREvalObjEx`: ordinary object evaluation first
  recognises a canonical List and invokes its original elements directly.

The generic List error path also omits the original-vector error-log tail.
`publish_native_error_objects_checked` only copies private roles when
`native.legacy_copy` has been reached; ordinary command logging updates that
state through `update_native_error_info_checked`. This explains a possible
publication boundary, but its actual dynamic control state has not been
instrumented. Setting `legacy_copy` indiscriminately would conceal the earlier
instruction mismatch and is not a proposed repair.

### Ordinary original-object element lookup

`capture_original_c_selection_using` retains the actual selected name and
checks its operation stage before and after receiver capture. On failure it
calls `original_c_lookup_var_error_using`, which eventually calls
`original_c_variable_receiver_error_using` with `reached_site: None`.
The latter groups Write/QuietWrite plus RetiredNamespace under ValueWrite,
including an actual selected element. This discards the reached early array
lookup context before the central diagnostic projection.

The original C86 `TclLookupArrayElement` dead-root branch reports LOOKUP and
retains `arrayNamePtr` in its error-code tuple. The compiled indexed caller
passes no root pointer; ordinary original-object lookup retains one. A parsed
root having an authentic compiled-local index does not change its ordinary
caller into an indexed instruction. The third unchanged Runtime control
already asserts precisely this distinction.

## Smallest proposed repair

1. At the selected dynamic Catch instruction, enter a dedicated compiled-body
   helper using the same retained substituted original object. Preserve
   substitution-before-catch ordering and the existing owning operation's
   first-cause checks. Invoke `prepare_original_c_body` with the actual current
   namespace and its existing borrowed-local-table checks, then execute the
   admitted original artifact with `execute_original_c_body`. A missing
   admission must remain explicit; do not infer an artifact or route through
   generic List dispatch. Leave ordinary `eval_generic_control_body` and
   `NativeEvalObjectProtocol::dispatches_list` unchanged.
2. In the ordinary selected receiver-capture failure adapter, carry NameLookup
   only for the actual Write/QuietWrite, DeletedNamespace, selected-element
   failure. Thread this explicit site through the existing original-name
   presenter, retaining operand checks, operand-free refusal handling and
   trace-error handling. Preserve scalar ValueWrite and later store-failure
   paths. The independently admitted indexed body continues to carry
   IndexedNameLookup. No Syntax policy extension or cell-metadata inference is
   needed.
3. Run the same three controls with unchanged source, expected rows and
   assertions. A whole-output pass must include both dynamic rows and the
   ordinary/vector root word. Retain real completion, first-cause/refusal and
   allocation-retirement checks; do not replace assertions with source-shape
   checks. Broaden only for a new measured failure or the compiled-body
   helper's actual adjacent control consumers.

## Inspected source hashes

These hashes identify the read-only source reviewed, not execution artifacts.

| Source | SHA-256 |
| --- | --- |
| Runtime `native_variable_names.rs` | `6bda6d5431990fba947f9d8a027cf23d6a725dd276d2969cf52d2c1e0a5d5c0f` |
| Runtime `native_body_artifact/native_control.rs` | `c6f85cb5d9505bbc2a5693453355e658f818501ed4f381658cce7521844a03ae` |
| Runtime `interp.rs` | `178a2aaa1faa8a6c2ac853ef1b13d04f9541d8bb2d4d2941d24a9ccca4f31995` |
| Registry `native_eval_object.rs` | `122796fbe3238e32cc426e6ddd3e8f78289aa6033194df840e3d838a1cd038b5` |
| Original C86 `generic/tclCompCmds.c` | `c42255a9d4ebb83dfb6f7d956ca411e746a434297b703194e2832eefe705a13e` |
| Original C86 `generic/tclExecute.c` | `2a29cf5e54d1b2ac4ddf56a6f38556bbfb9c8e8913800377c84a6f0bb8caa491` |
| Original C86 `generic/tclVar.c` | `2af24d8fa81db05a2805652ac4359d55ec1e2ef8bbcdad86c4000e25dfd61946` |

The native sources above are the existing local Tcl8.6.18 tree. The original
experiment's provider and input seals remain authoritative for its actual
execution. F5 was not run, and this source review grants no F5, Jim, arbitrary
private-layout or whole-suite conformance claim.
