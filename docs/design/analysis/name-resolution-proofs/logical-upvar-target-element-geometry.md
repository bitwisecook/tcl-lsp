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

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Exact evidence

- `naming-compiler-logical-upvar-target-element-geometry-handlers.rs` (implementation): [rust/tcl-compiler/src/analyser/handlers.rs](../../../../rust/tcl-compiler/src/analyser/handlers.rs). SHA-256 `e8af8ad1520e09091efee1bd8f6e8d2da453b05b14f78975e3ec9e4fc74aba5f`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.
- `naming-compiler-logical-upvar-target-element-geometry-naming.rs` (implementation): [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs). SHA-256 `77c05067c4a55d218010c4225b664fbb07d16624293457299a52d96e3912c6fc`. Current source/API owner and fixed marked assertion body; no executed Rust or native provider observation.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs), `split_element_ref`: Split an already literal resolved combined name only when its final closing parenthesis exists; do not strip substitution sigils or infer a runtime element key.
- [rust/tcl-compiler/src/analyser/handlers.rs](../../../../rust/tcl-compiler/src/analyser/handlers.rs), `Analyser::upvar_link_target`: Retain positive Logical readonly scalar/element target geometry and independent global/rooted versus dynamic/relative-frame refusal.
- [rust/tcl-compiler/src/analyser/handlers.rs](../../../../rust/tcl-compiler/src/analyser/handlers.rs), `analyser::handlers::tests::logical_upvar_targets_share_literal_scalar_and_element_geometry` (linked): Eight genuine complete plain Logical analyses assert positive retained input, whole scalar/Unicode geometry versus balanced array base, dynamic key omission and dynamic-base refusal. Readonly symbolic target labels do not install Native links or prove caller frames/storage values.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
