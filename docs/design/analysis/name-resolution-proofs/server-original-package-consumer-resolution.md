# naming.server.original-package-consumer-resolution

Kind: `implementation-contract`

## Problem statement

A diagnostic consumer that reacquires a package by name/default version can silently select a different provider or dependency preference from the original require. W123 suppression must be based on the exact original requirement context rather than a package label or a blanket package-command inventory.

## Question

Does the Server original package consumer select provider commands and transitive dependencies using the same complete requirement constraints and propagated preference?

## Conclusion

The original Server consumer passes complete PackageRequirementAdvice to package closure, preserving original identity, alternatives, exact matching and independently supplied preference. The selected provider's source preference propagates to its dependencies. Diagnostic refinement preserves unresolved typed subjects for commands outside that selected closure. Source inheritance carries complete records in both directions with original positions and independently resolved URI edges; an unresolved source edge retains abstention. These are advisory consumer joins, not loader execution or source-command dispatch receipts. defined_original_commands shares conditional source procedure candidates with local/workspace completion. Canonical headers, moved/replaced slots and withdrawals retain authored source purpose; package preference/dependency availability stays independent. Class candidates share OriginalSourceClassPublications under the complete retained source input. Surviving conditional factory slots keep their canonical original naming purpose; known moves/deletes/replacements do not recover withdrawn original class labels. Actual source availability remains independent of package preference and successful loader or factory execution.

## Scope

Current Server implementation contract. The concrete regression exercises original W123 refinement with exact w1.2 versus caller-Latest w1.0, the selected provider's local Latest dependency preference and the actual unresolved original command-byte set. It does not execute Tcl loaders, assert an LSP push/pull roundtrip or verify native provider behavior. Source inheritance and positioned records are inspected production owners; the test contains no claimed execution result. The source fixture selects its explicit versioned original name recipe consistently with package resolution; separately retained Logical analysis remains a negative original-inventory control. Exact source/header constraints and dependency preference do not prove package loading or Native publication.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust package-consumer integration contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust package-consumer integration contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust package-consumer integration contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust package-consumer integration contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Tcl.

This is a Rust package-consumer integration contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: Jim Tcl.

This is a Rust package-consumer integration contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

This is a Rust package-consumer integration contract. No C Tcl, Jim or BIG-IP execution of this question is claimed.

## Exact evidence

- `server-package-consumers` (implementation): [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs). SHA-256 `1f4ab37e5d4cd6aed65cd27efdce10bc4326d3e87da7243c000cc20e265011c9`. Inspected shared original inheritance/refinement consumer and constraint/dependency-preference regression.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `compute_source_inheritance`: Carry original complete ancestor and positioned descendant requirement advice with configured/ancestor preference.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `refine_original_w123_diagnostics`: Refine typed unresolved commands only through the selected complete package/provider dependency closure.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `defined_original_commands`: Share conditional original procedure and class factory candidates for readonly package/autoload command advice.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_package_consumers_keep_constraints_and_dependency_preference` (linked): Exact w1.2 leaves beta_command and dep_old unresolved; caller-Latest w1.0 selects beta and leaves old_command plus both unrelated dependency commands unresolved. The source fixture selects its explicit versioned original name recipe consistently with package resolution; separately retained Logical analysis remains a negative original-inventory control. Exact source/header constraints and dependency preference do not prove package loading or Native publication.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_package_procedures_use_shared_conditional_source_slots` (linked): Package suggestions share authentic moved/deleted/replaced procedure source slots under authored policy without successful loader/native publication.
- [rust/tcl-lsp-server/src/lib.rs](../../../../rust/tcl-lsp-server/src/lib.rs), `tests::original_package_classes_use_shared_current_factory_slots` (linked): Actual Server package command inventory renders moved New, omits deleted class, and selects the independently genuine replacement procedure Old; actual C8.4 source vocabulary produces no class suggestion. This readonly advice proves no loader, factory success, installed token or current Native world.

A named test is a coverage binding, not a claim that it executed.

## Replay

The linked assertion must be executed under the coherent Rust workspace before claiming a passing result. Source hashes bind the inspected implementation and test; a source edit or formatter requires review/reissue. Package graph advice and provider selection do not establish that a Tcl loader or source file executed.
