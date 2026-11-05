# naming.tcloo.original-constant-link-introspection

Kind: `native-observation`

## Problem statement

Constant enumeration, global/upvar links and formal shadowing have distinct source/storage roles. Public introspection must retain the exact original controls and selected provider.

## Question

What public caught values does the original TclOO constant-link introspection source return for inspect, retarget and formal-shadow methods?

## Conclusion

C9.0.4/C9.1.0 return exact original 0 {X 1 1} 0 {{} 1} 0 {formal {}}. C8.4.20/C8.5.19/C8.6.18/Jim return const-or-stock-TclOO-unavailable NOT_APPLICABLE. These finite returned values do not identify private local cells, object headers, Native frame/formal binding or a general constant-link inventory. All six original case processes exit 0 with empty stderr; their complete unchanged counted public VERSION/ORIGINAL rows and six actual driver builds remain retained. No Rust assertion pass or BIG-IP result follows. A marked direct software frame/cell control binds existing namespace-alias target selection before local capture.

## Scope

One unchanged original ASCII LF source is passed directly to the native202 counted-source driver in each fresh full Tcl_Init or Jim core/static-extension interpreter. Original source contains its authored availability branch and catches; the external launcher adds no eval/catch/puts envelope. Public returned guest list bytes and caught codes are retained independently of private objects/frames. This constructed FrameStack/Namespaces fixture does not execute the original TclOO inspect/retarget/formal-shadow source, prove native constant-link identity or grant a physical procedure/object frame. Exact original public caught values remain independent. Source linkage supplies no executable availability or assertion outcome.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, jim

Status: `observed`. Channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; jim 0.84-9-g5bac7c9.

Exact whole original counted result: NOT_APPLICABLE const-or-stock-TclOO-unavailable. Outer original completion code0; external process exit0 and empty stderr. The source availability branch is measured; no TclOO operation in that branch is observed.

### tcl9.0, tcl9.1

Status: `observed`. Channel: Exact original counted source: full Tcl_Init/Tcl_EvalEx flags0 or Jim counted Jim_EvalObj. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

Exact whole original counted result: 0 {X 1 1} 0 {{} 1} 0 {formal {}}. Outer original completion code0; external process exit0 and empty stderr. All caught operation codes/value bytes are retained; no private object/header/frame claim follows.

### bigip

Status: `not-tested`.

No appliance execution answers this exact question.

## Consumer bindings

- [runtime/rust/src/vars.rs](../../../../runtime/rust/src/vars.rs), `publish_variable_mapped`: Prepare a local alias using the actual selected existing namespace target and retain its software scalar entry/origin before capture, without reconstructing it from the printed local name.
- [runtime/rust/src/cmd_info.rs](../../../../runtime/rust/src/cmd_info.rs), `cmd_info::tests::info_consts_includes_only_tcloo_instance_links` (linked): Selected public constant-link enumeration/status values retain independent method/formal/global/upvar relationships; no general cell or native header identity grant.
- [runtime/rust/src/vars.rs](../../../../runtime/rust/src/vars.rs), `vars::tests::stable_alias_preparation_follows_existing_namespace_alias_before_capture` (linked): A direct FrameStack/Namespaces fixture sets G9, makes namespace alias X to G, then prepares local X in a new frame. The retained local link has the actual global home/name G, reads9 and writes10 through the same software cell. No TclOO original process or native link pointer is observed.

## Evidence

- [rust/tcl-registry/tests/data/native_oo_context296](../../../../rust/tcl-registry/tests/data/native_oo_context296)
- [rust/tcl-registry/tests/data/native_oo_context296/8.4.20](../../../../rust/tcl-registry/tests/data/native_oo_context296/8.4.20)
- [rust/tcl-registry/tests/data/native_oo_context296/8.5.19](../../../../rust/tcl-registry/tests/data/native_oo_context296/8.5.19)
- [rust/tcl-registry/tests/data/native_oo_context296/8.6.18](../../../../rust/tcl-registry/tests/data/native_oo_context296/8.6.18)
- [rust/tcl-registry/tests/data/native_oo_context296/9.0.4](../../../../rust/tcl-registry/tests/data/native_oo_context296/9.0.4)
- [rust/tcl-registry/tests/data/native_oo_context296/9.1.0](../../../../rust/tcl-registry/tests/data/native_oo_context296/9.1.0)
- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/native_oo_context296/jim](../../../../rust/tcl-registry/tests/data/native_oo_context296/jim)
