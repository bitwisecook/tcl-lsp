# naming.core.original-member-body-colours

Kind: `implementation-contract`

## Problem statement

Method parameter and body colours can lose their original worker words when reporting maps are erased, or infer parameter grammar from a method display label instead of the independently retained source dialect.

## Question

Do readonly method parameter/body colours consume canonical complete worker words and the independently selected source dialect?

## Conclusion

OriginalTokenRoles retains the actual source member declaration and its Registry-selected whole ParamList and Body NativeWords. Complete image/configuration, unexpanded source geometry and the exact single token remain required; body syntax additionally requires the actual string token. The formal syntax dialect comes from canonical source metadata and a conflicting or absent dialect stays unavailable. Semantic consumers share these operands with the existing original source/list and formal syntax owners. Erased class maps and invocation projections do not supply another route or name, and changed whole source cannot retain these roles.

## Scope

Readonly semantic source geometry for genuine current member declarations, including required/default formal syntax and original body variable source roles. It supplies no evaluated ParamList, formal binding, method activation, entered body, runtime cell, native compiler/header, Normal, command lookup or edit permission.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this implementation question.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this implementation question.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this implementation question.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this implementation question.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No observation for this implementation question.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No observation for this implementation question.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No observation for this implementation question.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/analyser/types/original_members.rs](../../../../rust/tcl-compiler/src/analyser/types/original_members.rs), `OriginalSourceMethodMetadata::parameters_word`: Retain the independent original whole formal-list worker operand without reconstructing it from a name or reporting span.
- [rust/tcl-compiler/src/analyser/types/original_members.rs](../../../../rust/tcl-compiler/src/analyser/types/original_members.rs), `OriginalSourceMethodMetadata::body_word`: Retain the independent original whole body worker operand as source syntax only.
- [rust/tcl-compiler/src/analyser/types/original_members.rs](../../../../rust/tcl-compiler/src/analyser/types/original_members.rs), `OriginalSourceMethodMetadata::source_dialect`: Retain the actual independently selected source grammar; missing or incompatible dialect supplies no formal syntax grammar.
- [rust/tcl-lsp-core/src/semantic_tokens/original.rs](../../../../rust/tcl-lsp-core/src/semantic_tokens/original.rs), `OriginalTokenRoles::member`: Select canonical original worker token/operand geometry and preserve independently retained formal grammar without borrowing reporting maps.
- [rust/tcl-lsp-core/src/semantic_tokens/original.rs](../../../../rust/tcl-lsp-core/src/semantic_tokens/original.rs), `semantic_tokens::original::original_source_schema_tests::original_semantic_method_roles_keep_canonical_worker_words` (linked): Genuine member whole formal/body words and source dialect preserve required/default parameter and body variable colours after reporting class maps and invocation projections are erased; changed whole source refuses retained roles.

A named test is a coverage binding, not a claim that it executed.

## Replay

The exact linked Rust selector exercises readonly canonical worker syntax. No Rust execution receipt or native provider observation is attached to this implementation contract.
