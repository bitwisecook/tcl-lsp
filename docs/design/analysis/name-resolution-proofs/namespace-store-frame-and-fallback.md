# naming.variable.namespace-store-frame-and-fallback

Kind: `native-observation`

## Problem statement

A bare Set inside namespace eval can write an existing root variable, a namespace variable or an activation-local variable. Inferring a namespace target from the written scope alone can publish the wrong source symbol. The fixed root-present, root-absent and explicit-variable controls distinguish these alternatives without relying on an earlier substitution setup.

## Question

Where does a bare namespace-eval Set write x with an existing root x or no root x, and how does explicit variable x VALUE change that target on the six tested providers?

## Conclusion

C8.4-C8.6 reuses an existing root x and creates namespace x only when the root is absent; C9.0/C9.1 creates namespace x even when root x exists. Jim bare Set creates an activation-local x and leaves both namespace global x and existing root x unchanged. Explicit variable creates namespace x on all six. These are the fixed ASCII namespace-entry/store/read controls, not arbitrary resolver or trace closure.

Original source symbol projections retain selected existing-root/fresh-namespace fallback, Jim namespace-activation local purpose and explicit namespace variable storage as distinct model facets.

## Scope

Three caught ASCII source programs supplied as native shell stdin; actual info patchlevel, exact input and streams, executable/header/library/Makefile/source-owner hashes retained. Initial namespace names are fresh. No raw zero, physical variable header, compiler admission, alias or observer claim.

Current direct test attributes and real module parents attach these source/API contracts only. They do not certify an executable inventory, passed assertion, complete Native comparison or changed-current-image result. Each original provider answer and independently closed software command retains its exact input/channel/source purpose.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6

Status: `observed`. Channel: ASCII file bytes supplied to native shell stdin. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18.

EXISTING_ROOT: bare Set updates root to VALUE and ::N::x is absent. FRESH_NO_ROOT: ::Fresh::x=VALUE and root is absent. EXPLICIT_VARIABLE: namespace x=VALUE and root stays ROOT.

### tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII file bytes supplied to native shell stdin. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

EXISTING_ROOT: ::N::x=VALUE and root stays ROOT. FRESH_NO_ROOT: namespace x=VALUE and root is absent. EXPLICIT_VARIABLE: namespace x=VALUE and root stays ROOT.

### jim

Status: `observed`. Channel: ASCII file bytes supplied to native shell stdin. Version: jim 0.84-9-g5bac7c9.

Bare Set creates activation-local x: namespace global x is absent in both cases and an existing root stays ROOT. Explicit variable creates ::Explicit::x=VALUE and leaves root ROOT.

### bigip

Status: `not-tested`.

No appliance observation for this exact question.

## Consumer bindings

- [rust/tcl-vm/src/cmd_namespace.rs](../../../../rust/tcl-vm/src/cmd_namespace.rs), `eval_in_ns`: Enter the actual selected namespace frame before variable operations; the invocation does not itself prove a variable address.
- [rust/tcl-vm/src/cmd_namespace/native_store_tests.rs](../../../../rust/tcl-vm/src/cmd_namespace/native_store_tests.rs), `cmd_namespace::native_store_tests::namespace_stores_match_native_existing_root_fresh_and_declared_controls` (linked): Compare all18 native namespace-store result rows using independent actual engine fixtures, root-present/absent initialization and original native namespace script objects.
- [rust/tcl-compiler/src/var_resolve/original_bytes.rs](../../../../rust/tcl-compiler/src/var_resolve/original_bytes.rs), `var_resolve::original_bytes::tests::namespace_runtime_root_requires_current_or_global_table_membership` (linked): Independent current/global inventory for ASCII x distinguishes C8 existing-root fallback from C9 namespace selection and declines missing membership; opaque-byte equality is separate Rust correspondence beyond the measured ASCII native controls.

- [rust/tcl-compiler/src/analyser/scope.rs](../../../../rust/tcl-compiler/src/analyser/scope.rs), `analyser::scope::tests::original_namespace_store_symbols_use_the_selected_activation_and_fallback` (linked): Original source symbol projection keeps selected C existing-root/fresh-namespace fallback and Jim namespace-activation local purpose distinct from explicit variable namespace storage; no actual Native frame or store completion is attached.

Current source linkage supplies no execution or assertion credit.

## Evidence

- [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject)
- [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.4.20)
- [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.5.19)
- [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/8.6.18)
- [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.0.4)
- [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/9.1.0)
- [rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim](../../../../rust/tcl-syntax/tests/data/native_namespace_store_trace_subject/jim)
