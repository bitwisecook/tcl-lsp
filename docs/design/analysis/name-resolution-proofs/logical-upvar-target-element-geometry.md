# naming.compiler.logical-upvar-target-element-geometry

Kind: `implementation-contract`

## Problem statement

A first-parenthesis split can truncate literal scalar names that have no final closing parenthesis. Logical source alias advice needs the shared literal-name element grammar independently of substitution syntax or an executed link.

## Question

How does genuine Logical upvar source advice preserve literal scalar and element-target geometry, including unmatched parentheses, Unicode and dynamic bases, without stripping substitution sigils or asserting an executed caller-frame link?

## Conclusion

Analyser::upvar_link_target delegates the literal original combined name to Syntax split_element_ref, which requires a final closing parenthesis before selecting an array base. Scalar a(b and a(b)c retain their complete original units; a(b) selects base a. Unicode scalar/element inputs retain the same whole geometry, and an already rooted scalar remains rooted. For a($key), only the static array base is source advice; the runtime key is not inferred. No substitution sigil is stripped, so $computed remains dynamic and refuses. Empty or dynamic bases and unqualified relative-frame targets remain unavailable; only the selected global or rooted source target has a stable Logical path. The fixed complete plain Logical analyses assert positive retained input and readonly reported link coordinates, independent of any Native variable name protocol, physical cell, caller frame, current value, completed alias installation or execution.

## Scope

One genuine whole-source Logical control covers eight original operands: unmatched/trailing parenthesis scalars, balanced elements, Unicode variants, dynamic element key, rooted scalar and dynamic base. It selects existing shared literal-name parsing and positive Logical input without native name/compiler/read purpose. Separate shared parser and caller-level refusal guards retain their own purpose. No C/Jim/BIG-IP process or physical link/frame/value is observed; all seven native providers are not tested.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Consumer bindings

- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `split_element_ref`: Split an already literal resolved combined name only when its final closing parenthesis exists; do not strip substitution sigils or infer a runtime element key.
- [rust/tcl-compiler/src/analyser/handlers.rs](../../../../rust/tcl-compiler/src/analyser/handlers.rs), `Analyser::upvar_link_target`: Retain positive Logical readonly scalar/element target geometry and independent global/rooted versus dynamic/relative-frame refusal.
- [rust/tcl-compiler/src/analyser/handlers.rs](../../../../rust/tcl-compiler/src/analyser/handlers.rs), `analyser::handlers::tests::logical_upvar_targets_share_literal_scalar_and_element_geometry` (linked): Eight genuine complete plain Logical analyses assert positive retained input, whole scalar/Unicode geometry versus balanced array base, dynamic key omission and dynamic-base refusal. Readonly symbolic target labels do not install Native links or prove caller frames/storage values.

## Evidence

- [rust/tcl-compiler/src/analyser/handlers.rs](../../../../rust/tcl-compiler/src/analyser/handlers.rs)
- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs)
