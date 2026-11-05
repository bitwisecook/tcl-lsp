# naming.variable.original-traced-array-root-materialisation

Kind: `native-observation`

## Problem statement

A write trace can be registered before an array has a value, and a callback can write a later element while array set processes its original pairs. The complete original public result is needed without inferring private shell, binding or receiver identity.

## Question

What completion and final element values does the whole original program return after registering a write trace on an unset root and executing array set whose callback writes a later pair?

## Conclusion

Each pinned C8.4–C9.1 provider completes the exact original with code0 and final public list FIRST LAST. The original registers mutate as a write trace on ::heldArray before array set, writes CALLBACK to its j element when k is reported, then supplies the original pairs k FIRST j LAST and queries both final values. The measured answer is the whole completion and final list; no separate callback log, private root-shell state or physical receiver/member identity is observed. Jim reaches invalid command name "trace" and returns guest code1 for the same original, without an array write-callback answer. All six standalone driver processes exit0 with empty stderr and retain independent VERSION and ORIGINAL counted-hex rows. These public values supply no installed trace generation, header/cell identity, entered frame, Native argv or corresponding Rust assertion pass.

## Scope

One unchanged original ASCII LF source runs independently in six fresh fully initialised provider interpreters under the unchanged counted-source CAPI driver, without an added source catch/eval/puts envelope. Six compile receipts/ELFs and six whole process streams/receipts retain source SHA4ffc50f2014c35f4bf451199b9916153f104e79fdf153b28c7280db8ea75177b and required archive/header/build/source pins. The initial request and expanded capture request remain separate original inputs. No private shell-materialisation observer, alias/foreign receiver, opaque/NUL/Unicode variable name, BIG-IP execution or executed software control is supplied. The linked Runtime public comparison uses all six untouched originals and whole completion/final values, preserving Jim trace failure. Separate software CellContents undefined-root markers retain the actual producer-issued VarHome and binding identity. VariableReceiver and retained NativeScalarAliasEntry may materialise only that selected original undefined root; an ordinary scalar, retired owner or same-spelled foreign link cannot issue or borrow the marker. These software ownership controls neither turn FIRST LAST into a provider private-shell/cell/header observation nor claim an executed Rust pass.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: Fresh full Tcl_Init or Jim core/static extensions; unchanged original counted source, independent VERSION and ORIGINAL hex completion/result rows; no added source envelope. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Host exit0 and empty stderr; original guest completion 0 with exact public result 'FIRST LAST'. The final element values are FIRST and LAST in this complete original program. Callback-private state, root/member/cell/header identity and a general materialisation rule are unobserved.

### jim

Status: `observed`. Channel: Fresh full Tcl_Init or Jim core/static extensions; unchanged original counted source, independent VERSION and ORIGINAL hex completion/result rows; no added source envelope. Version: jim 0.84-9-g5bac7c9.

Host exit0 and empty stderr; original guest completion 1 with exact public result 'invalid command name "trace"'. The original trace command is unavailable, so no Jim array write-callback outcome is measured.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [runtime/rust/src/frame.rs](../../../../runtime/rust/src/frame.rs), `VarTable::mark_undefined_root`: Issue an undefined-root marker only from the actual selected original root producer and matching VarHome; same spelling at a foreign home refuses.
- [runtime/rust/src/frame.rs](../../../../runtime/rust/src/frame.rs), `CellContents::publish_selected_array`: Materialise only an independently owned selected original undefined-root marker or existing array, preserving genuine scalar/retirement refusal.
- [runtime/rust/src/frame/rmw.rs](../../../../runtime/rust/src/frame/rmw.rs), `VariableReceiver::ensure_array`: Use the actual retained receiver root spelling and shared CellContents marker, independently of current same-spelled table lookup.
- [runtime/rust/src/frame/native_scalar_alias.rs](../../../../runtime/rust/src/frame/native_scalar_alias.rs), `NativeScalarAliasEntry::ensure_array`: Retain exact original alias contents/binding and its producer-issued undefined-root marker; a foreign or scalar entry cannot supply materialisation.
- [runtime/rust/src/cmd_array/undefined_root.rs](../../../../runtime/rust/src/cmd_array/undefined_root.rs), `cmd_array::undefined_root::original_traced_array_root_materialisation_matches_six_native_source_results` (linked): All six whole untouched native345 source completion/final value rows compare under authentic NativeCore without host refusal; Jim unsupported trace remains guest1. The binding supplies no executed software pass or private provider root/member identity.
- [runtime/rust/src/frame/rmw.rs](../../../../runtime/rust/src/frame/rmw.rs), `frame::rmw::tests::selected_undefined_root_becomes_array_without_changing_its_binding` (linked): The software original producer-issued root marker retains its exact selected binding through array materialisation and original member store. Native345 public final values do not observe this private binding identity.
- [runtime/rust/src/frame/native_scalar_alias.rs](../../../../runtime/rust/src/frame/native_scalar_alias.rs), `frame::native_scalar_alias::tests::retained_alias_materialises_only_its_original_undefined_root` (linked): The retained software alias materialises its own genuine undefined root while an ordinary scalar receiver refuses and preserves its value; public Native values do not grant alias/root identity.
- [runtime/rust/src/frame/native_scalar_alias.rs](../../../../runtime/rust/src/frame/native_scalar_alias.rs), `frame::native_scalar_alias::tests::same_spelled_foreign_link_cannot_issue_an_undefined_root_marker` (linked): A same-spelled actual foreign-home link cannot issue the current-home undefined-root marker; both direct and retained alias receivers refuse without replacing the original link. This is host software ownership, not a native private observation.
- [runtime/rust/src/cmd_array.rs](../../../../runtime/rust/src/cmd_array.rs), `cmd_array::tests::original_array_set_selects_root_before_parity_and_keeps_callback_results` (linked): The typed software array receiver selection and callback result control includes the exact unchanged native345 write-trace source; public FIRST LAST remains independently captured, with other software receiver/ordering premises kept separate.

## Exact original-source349 software outcomes

These finite results belong to the original source349 body/question/document pairs and the owned Runtime executable. Passing, failing, aborted/incomplete and unstarted selections retain their separate meanings. They establish no outcome for another source image or external provider.

| Original selection | Passed | Failed | Aborted or incomplete | Not started |
| --- | ---: | ---: | ---: | ---: |
| `grouped` | 1 | 0 | 0 | 0 |
| `isolated` | 1 | 0 | 0 | 0 |

Exact original body hashes and process outcomes join the unchanged original receipts, streams, frozen source, own inventory/admission and complete executable bytes. The grouped abort is independent of the isolated diagnostics.

## Evidence

- [rust/tcl-registry/tests/data/native_array_traced_root345](../../../../rust/tcl-registry/tests/data/native_array_traced_root345)
- [rust/tcl-registry/tests/data/native_array_traced_root345/8.4.20/original-traced-root-array-set](../../../../rust/tcl-registry/tests/data/native_array_traced_root345/8.4.20/original-traced-root-array-set)
- [rust/tcl-registry/tests/data/native_array_traced_root345/8.5.19/original-traced-root-array-set](../../../../rust/tcl-registry/tests/data/native_array_traced_root345/8.5.19/original-traced-root-array-set)
- [rust/tcl-registry/tests/data/native_array_traced_root345/8.6.18/original-traced-root-array-set](../../../../rust/tcl-registry/tests/data/native_array_traced_root345/8.6.18/original-traced-root-array-set)
- [rust/tcl-registry/tests/data/native_array_traced_root345/9.0.4/original-traced-root-array-set](../../../../rust/tcl-registry/tests/data/native_array_traced_root345/9.0.4/original-traced-root-array-set)
- [rust/tcl-registry/tests/data/native_array_traced_root345/9.1.0/original-traced-root-array-set](../../../../rust/tcl-registry/tests/data/native_array_traced_root345/9.1.0/original-traced-root-array-set)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_array_traced_root345/jim/original-traced-root-array-set](../../../../rust/tcl-registry/tests/data/native_array_traced_root345/jim/original-traced-root-array-set)
