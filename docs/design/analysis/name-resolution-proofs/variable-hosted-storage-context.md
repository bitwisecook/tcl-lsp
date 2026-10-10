# naming.variable.hosted-storage-context

Kind: `implementation-contract`

## Problem statement

A source profile can advertise iRules commands while the supplied entry belongs to another hosted interpreter or has no worker context. Selecting namespace storage from that profile can fabricate per-TMM cells and carry equal contents across different host contexts.

## Question

Which inputs select hosted variable storage, and can source naming advice or a registry profile supply those inputs?

## Conclusion

Hosted storage policy is retained separately from source naming and the registry profile. Namespace cells use the explicitly supplied hosted context and retained namespace components, while the independently selected authored worker publication receipt retains its own namespace and observer boundary. Missing TMM namespace geometry supplies no storage domain. Frame entry and restoration preserve the context; joins of unequal contexts withdraw contents, representations, binding certainty and observer closure. Source advice supplies no worker, RULE_INIT publication, runtime cell, Normal completion or cross-worker value agreement.

## Scope

Rust implementation contract for explicitly supplied hosted execution contexts, actual namespace geometry and separately supplied worker/publication receipts. Source-only vendor inputs and floating profiles do not attest an appliance build or select an executing worker. No BIG-IP appliance observation or RULE_INIT broadcast law is asserted.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust implementation contract; no native or appliance observation for this question is attached.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/source_analysis_entry.rs](../../../../rust/tcl-compiler/src/command_binding/source_analysis_entry.rs), `SourceAnalysisEntry`: Host policy is supplied independently of source syntax, name input, physical compiler and catalogue.
- [rust/tcl-compiler/src/var_resolve.rs](../../../../rust/tcl-compiler/src/var_resolve.rs), `bind_cell_identity`: Use retained namespace components and independently selected host context; an authored publication receipt remains a separate issuer.
- [rust/tcl-compiler/src/var_resolve.rs](../../../../rust/tcl-compiler/src/var_resolve.rs), `ResolveContext::join`: Unequal hosted contexts withdraw precise contents and observers instead of meeting equal reported names or values.
- [rust/tcl-registry/src/f5/storage.rs](../../../../rust/tcl-registry/src/f5/storage.rs), `namespace_storage_domain_in_path`: One shared host-domain classifier consumes already resolved component geometry.
- [rust/tcl-compiler/src/command_binding/source_analysis_cache.rs](../../../../rust/tcl-compiler/src/command_binding/source_analysis_cache.rs), `SourceAnalysisCacheKey`: Retain the independent host context in the complete immutable entry key.
- [rust/tcl-compiler/src/lowering/mod.rs](../../../../rust/tcl-compiler/src/lowering/mod.rs), `set_source_analysis_options`: Transport the host context through the owned entry and subsequent lowerer source options.
- [rust/tcl-compiler/src/var_resolve.rs](../../../../rust/tcl-compiler/src/var_resolve.rs), `var_resolve::worker_execution_ingress_tests::hosted_storage_requires_selected_context_and_retained_namespace_geometry` (linked): Explicit host context and exact root/static components select the domain independently of catalogue profile; unrelated contexts and unavailable components cannot select worker storage.
- [rust/tcl-compiler/src/var_resolve.rs](../../../../rust/tcl-compiler/src/var_resolve.rs), `var_resolve::worker_execution_ingress_tests::different_hosted_contexts_with_equal_values_withdraw_cell_contents` (linked): Entered frames and restoration retain the context, while unequal host contexts withdraw equal known bytes and observer/binding certainty.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `command_binding::hosted_storage_entry_tests::hosted_storage_entry_is_independent_of_registry_and_source_name_policy` (linked): Entry options retain independent host policy without a worker, publication provider or registry-profile inference.
- [rust/tcl-registry/src/f5/storage.rs](../../../../rust/tcl-registry/src/f5/storage.rs), `f5::storage::tests::hosted_domain_uses_exact_resolved_components` (linked): Exact root/static components differ from nested, raw-zero and literal-colon components; non-TMM contexts do not acquire TMM storage classification.

A named test is a coverage binding, not a claim that it executed.

## Replay

The linked Rust selectors exercise context, geometry and join premises. No executed Rust receipt or native appliance protocol is attached to this record. Actual worker publication and the unanswered appliance publication question remain independent.
