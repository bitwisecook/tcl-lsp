# naming.source.original-configured-class-transition-roles

Kind: `implementation-contract`

## Problem statement

A TclOO class configuration command can be reached through a genuine alias or move. Re-resolving its displayed command name or target label loses the original selected command, whole argv, and transition lineage needed to project source roles.

## Question

When may source consumers obtain Registry-authored roles for the exact configured-class transition already retained by the analysis?

## Conclusion

`source_configured_class_words_at` requires the current resolved input, matching body lexer configuration, current document image, and a source-current `OriginalSourceConfiguredClassReference`. It derives the role words from that reference’s retained transition consumer, preserving the actual original command and its alias/move lineage. The configured target’s report label does not choose a Registry descriptor. Missing or stale source ownership, foreign configuration, or unavailable transition roles return no projection. This API supplies conditional source roles only; it does not establish class installation, method dispatch, runtime mutation, a physical object, Native admission, or edit permission.

## Scope

This is a read-only projection for a typed configured-class source transition. It does not provide a complete mutation census or make absence of a reference prove that no mutation exists.

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
| Original configured-class command and lineage | `OriginalSourceConfiguredClassReference` in `rust/tcl-compiler/src/command_binding/source_transition_advice/source_class.rs` | `source_configured_class_at` in `rust/tcl-compiler/src/registry_invocation/source_structure.rs` | The owner must match the same source image, lexer configuration, resolved context, and full input. |
| Exact original argv and selected source schema | The reference’s retained transition consumer and existing `source_transition_words_from_advice` projection | `source_configured_class_words_at` in `rust/tcl-compiler/src/registry_invocation/source_structure.rs` | No reconstruction from reported command/class strings; unsupported or incomplete roles abstain. |
| LSP member hazard use | Existing member metadata consumer in `rust/tcl-lsp-core/src/references/member_metadata.rs` | Existing source-role path | This role projection does not itself complete member reference coverage or grant rename authority. |

## Replay

No test or provider was run for this contract. Static source review and patch checks are recorded in the candidate manifest and do not establish runtime behavior.
