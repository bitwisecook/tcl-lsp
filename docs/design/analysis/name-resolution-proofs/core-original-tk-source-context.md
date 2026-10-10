# naming.core.original-tk-source-context

Kind: `implementation-contract`

## Problem statement

A standalone UI source consumer can receive a profile whose version differs from the command store. Rebuilding either input or attaching captured constructor arguments to later written operands can misreport widget source.

## Question

Does standalone Tk source advice retain the explicitly supplied profile, grammar and command generation through activation, nested traversal and UI operand selection?

## Conclusion

Tk source analysis retains the supplied profile and full lexer configuration with the exact provided Registry generation in one resolved input. Activation and nested command traversal consume the actual retained analysis and shared original source schema. Widget operand selection requires genuine written anchors; captured and expanded inputs cannot borrow later written widget spans. Known command shadowing blocks nominal body traversal. Model certainty remains relative to this source advice and proves no live widget, package load, handler, frame or edit equivalence.

## Scope

Rust UI source modelling only. The fixed controls vary a supplied C8.6 profile against a newer C9.1 store, distinguish captured and prefixless aliases, and preserve known body-command shadowing.

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

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/tk_preview.rs](../../../../rust/tcl-lsp-core/src/tk_preview.rs), `analyse_tk_ui`: Use one retained source analysis for UI activation, geometry and source-relative certainty.
- [rust/tcl-lsp-core/src/tk_preview.rs](../../../../rust/tcl-lsp-core/src/tk_preview.rs), `tk_source_analysis`: Build actual input from the explicitly supplied profile, full grammar and exact Registry generation.
- [rust/tcl-lsp-core/src/tk_preview.rs](../../../../rust/tcl-lsp-core/src/tk_preview.rs), `tk_source_head`: Require the selected source schema and genuine written operand positions.
- [rust/tcl-lsp-core/src/tk_preview.rs](../../../../rust/tcl-lsp-core/src/tk_preview.rs), `source_requires_tk`: Select package activation through the retained source schema instead of nominal source text.
- [rust/tcl-lsp-core/src/executable_regions.rs](../../../../rust/tcl-lsp-core/src/executable_regions.rs), `visit_analysis_executable_commands`: Traverse potential UI source regions under the same actual Analysis owner.
- [rust/tcl-lsp-core/src/tk_preview.rs](../../../../rust/tcl-lsp-core/src/tk_preview.rs), `tk_preview::tests::original_tk_source_context_keeps_explicit_store_version_and_operand_boundaries` (linked): Supplied C8.6 profile retains the newer command generation while unavailable ttk construction is omitted; captured widget names own no later span, prefixless aliases retain written names, and a known if replacement hides its apparent body.

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
