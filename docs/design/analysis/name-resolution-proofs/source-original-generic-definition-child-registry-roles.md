# naming.source.original-generic-definition-child-registry-roles

Kind: `implementation-contract`

## Problem statement

A nested command in a selected definition body may have no independent document-root source tape entry. Reconstructing its Registry roles from a label, raw string, or parent command grammar can misidentify the selected child schema and lose the child’s original word geometry.

## Question

When may an existing analysis expose Registry-authored source roles for a generic child command inside a definition body?

## Conclusion

`source_registry_words_in_definition_body_at` first retains the complete parent body and its member-region plan. `generic_definition_child_source_words` then requires the same current document image, full resolved input, lexer configuration, context registry, parent source body, and exact original child `NativeWord` vector. It selects the child through the actual resolved context’s authoring realm, using the existing Registry `resolve_invocation_words_in_context` owner, and the parent’s retained invocation dialect; it returns roles only when that selected schema reports complete authored source roles. The parent owner attached to `OriginalRegistryWords` is provenance for revalidating this source projection. It grants no execution, entered frame, dispatch, object identity, edit permission, or complete reference census. Unsupported nested regions, dynamic or changed words, mismatched input/context, and incomplete roles remain unavailable. The retained parent member plan currently supports only the complete original parent region; this contract makes no claim that every deeper or partial nested region is covered.

## Scope

This contract concerns conditional read-only source role projection for one exact child argv under an existing analysis. It supplies no Native authority, semantic dispatch, runtime behavior, mutation, rename permission, provider result, or whole-source completeness claim.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No provider execution for this source contract.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

No provider execution for this source contract.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

No provider execution for this source contract.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

No provider execution for this source contract.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

No provider execution for this source contract.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No provider execution for this source contract.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No appliance execution for this source contract.

## Exact evidence

No provider evidence or test execution is attached.

## Owner-resolution map

| Purpose | Existing owner | Consumer | Limitation |
| --- | --- | --- | --- |
| Complete parent body and child-word correspondence | `OriginalSourceScriptBody` and `OriginalSourceDefinitionMemberRegion` in `rust/tcl-compiler/src/registry_invocation/source_scoped_body.rs` | `source_registry_words_in_definition_body_at` in `rust/tcl-compiler/src/registry_invocation/source_structure.rs` | Only complete retained parent plans and exact original child vectors qualify. |
| Actual child Registry schema and authored roles | `ResolvedAnalysisInput::context_registry`, parent `InvocationDialect`, and `ResolvedInvocation::authored_source_argument_roles` | `generic_definition_child_source_words` in `rust/tcl-compiler/src/registry_invocation/source_structure.rs` | Missing selection, dynamic/changed source, or incomplete roles return unavailable. |
| Parent provenance for later revalidation | `OriginalRegistryWords::definition_parent` | Existing source-structure consumers | Provenance only; it is not an execution or dispatch receipt. |

## Replay

No test or provider was run for this contract. Static source review and patch checks are recorded in the candidate manifest and do not establish runtime behavior.

The same `OriginalRegistryWords::with_source_schema` reborrow uses that existing
context-owned authoring resolver. Neither initial capture nor later revalidation
hardcodes `RuleLoader` or invents an InterpreterRuntime scope. This is conditional
source syntax selection only: it does not assert that the nested command is
reached, bind its runtime implementation or inherit a Native invocation realm.
