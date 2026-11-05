# naming.class.original-renamed-definition-error-context

Kind: `native-observation`

## Problem statement

A relative rename inside a class definition uses the actual definition namespace. A class label or expected short name cannot supply the original public definition error context or surviving command query result.

## Question

What complete public definition errorInfo and command-query results follow a relative rename inside the authentic TclOO class definition namespace?

## Conclusion

C8.6/C9.0/C9.1 return the caught error code1/resultfoo with complete errorInfo naming ::oo::define::def as the class definition frame; both possible queried public command names are empty after the failed class creation. Stock C8.4/C8.5 report ORIGINAL0/NOT_APPLICABLE:stock_TclOO_unavailable. Jim reports ORIGINAL2 with the same unavailable value from its original return branch. All six native processes/compiler commands exit0 with empty stderr. The public errorInfo/name queries do not observe private class token/worker identity, own-table closure or native definition frame equivalence. The linked Runtime control keeps evaluation boundaries explicit: for unavailable C8.4/C8.5 source, native top-level Tcl_EvalEx ORIGINAL0 and raw Runtime eval_str Return are compared as distinct completions with the same original result bytes. Supported TclOO controls and Jim unavailable return retain their respective full original code/result. No evaluator normalisation, private class/worker identity or stock TclOO substitution is inferred.

A second marked Runtime definition binds the reached renamed-class errorInfo substring in the default software fixture.

## Scope

One whole unchanged ASCII LF original chooses stock TclOO availability, creates its base and performs the relative rename inside the authentic class definition before an explicit error. Whole catch result/errorInfo and both name queries remain in the counted original result. C and Jim original completion codes remain distinct; Jim class machinery is not substituted for TclOO. BIG-IP and Rust assertions are not tested. The Runtime original-source comparison explicitly installs the shared NativeScriptedLibrary::ALL distribution for its selected Jim full-initialisation source ingress, matching the original driver's full core/static-extension setup. This conformance bootstrap cannot turn a core-only constructor, authoring profile or metadata source carrier into a loaded library, current helper binding or Native worker. The exact original sources, provider rows and completion boundaries remain unchanged; the linked assertion is not a claimed executed pass.

This fixture uses Interp::new through leak_free, without selecting or replaying six original providers. Only the late renamed-definition portion belongs to this binding; four earlier superclass/mixin failures retain their own purpose. It does not compare the whole original public result, query bytes, native holder/header, source offsets or executed provider state. No passing assertion or runtime entry admission is attached.

## Provider answers

### tcl8.4, tcl8.5

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.4 8.4.20; tcl8.5 8.5.19.

Exact ORIGINAL code/result: 0 / NOT_APPLICABLE:stock_TclOO_unavailable. Whole errorInfo/public queries retained without private object/worker inference.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Exact ORIGINAL code/result: 0 / 1 foo {foo
    while executing
"error foo"
    (in definition script for class "::oo::define::def" line 1)
    invoked from within
"oo::class create abc {superclass base; rename abc def; error foo}"} {} {}. Whole errorInfo/public queries retained without private object/worker inference.

### jim

Status: `observed`. Channel: External C API driver: fresh full Tcl_Init or Jim core/static extensions per control; independent exact prelude, original counted source, query evaluations in one interpreter; source is not surrounded by catch/eval/puts; C Tcl_EvalEx flags0, Jim original counted object Jim_EvalObj; public counted hex completions only. Version: jim 0.84-9-g5bac7c9.

Exact ORIGINAL code/result: 2 / NOT_APPLICABLE:stock_TclOO_unavailable. Whole errorInfo/public queries retained without private object/worker inference.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `cmd_oo::tests::original_renamed_definition_matches_native_source_and_raw_return_boundaries` (linked): Compare untouched whole native321 source/public result fields; unavailable C8.4/C8.5 explicitly keep embedding completion0 separate from raw Runtime Return, supported TclOO/Jim retain original completion boundaries. No output or oracle normalisation. Selected Jim full-init conformance ingress explicitly installs the shared source distribution; bootstrap is distinct from core-only and metadata purposes, with unchanged original rows and no executed pass claim.

- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `Interp::oo_define_body_with_argv`: Retain the reached original definition argv and stable software object creation identity through the body, then publish a reached definition-error frame.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `Interp::add_def_script_frame`: Resolve the current software object command name by its retained creation identity before appending the definition-error frame; fallback spelling and current object identity remain distinct.
- [runtime/rust/src/cmd_oo.rs](../../../../runtime/rust/src/cmd_oo.rs), `cmd_oo::tests::define_script_errorinfo_and_super_mixin_messages` (linked): Only the final relative-rename portion of this default Interp software fixture binds this question: after abc becomes def inside its definition namespace, reached errorInfo contains class "::oo::define::def". The preceding superclass/mixin errors belong to their independent nonclass question; this substring assertion is not a whole native errorInfo or holder comparison.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 1 | 0 | 0 |
| `isolated` | 1 | 1 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_oo_renamed_definition321](../../../../rust/tcl-registry/tests/data/native_oo_renamed_definition321)
- [rust/tcl-registry/tests/data/native_oo_renamed_definition321/8.4.20/renamed-definition](../../../../rust/tcl-registry/tests/data/native_oo_renamed_definition321/8.4.20/renamed-definition)
- [rust/tcl-registry/tests/data/native_oo_renamed_definition321/8.5.19/renamed-definition](../../../../rust/tcl-registry/tests/data/native_oo_renamed_definition321/8.5.19/renamed-definition)
- [rust/tcl-registry/tests/data/native_oo_renamed_definition321/8.6.18/renamed-definition](../../../../rust/tcl-registry/tests/data/native_oo_renamed_definition321/8.6.18/renamed-definition)
- [rust/tcl-registry/tests/data/native_oo_renamed_definition321/9.0.4/renamed-definition](../../../../rust/tcl-registry/tests/data/native_oo_renamed_definition321/9.0.4/renamed-definition)
- [rust/tcl-registry/tests/data/native_oo_renamed_definition321/9.1.0/renamed-definition](../../../../rust/tcl-registry/tests/data/native_oo_renamed_definition321/9.1.0/renamed-definition)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_oo_renamed_definition321/jim/renamed-definition](../../../../rust/tcl-registry/tests/data/native_oo_renamed_definition321/jim/renamed-definition)
