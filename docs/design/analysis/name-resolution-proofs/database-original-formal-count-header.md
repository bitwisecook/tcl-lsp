# naming.database.original-formal-count-header

Kind: `implementation-contract`

## Problem statement

Body-free declaration and project headers can reconstruct argument counts from mutable reporting parameter labels and silently replace the selected C/Jim formal grammar. Copying a full body/source receipt into a header would also couple body-only edits to declaration metadata and widen the meaning of the header.

## Question

How do declaration headers and database arity projections preserve independently selected original formal count/origin/grammar while remaining body-free metadata?

## Conclusion

SourceDeclarationSignature and ItemSig retain SourceFormalCountProjection: count, origin and selected ParameterGrammar only. Genuine original ordered formal storage remains separate from explicit authored metadata; computed, malformed or unowned signatures retain Unknown/open count. ItemTree::from_analysis uses the immutable original declaration projection independently of reporting parameter labels and maps. FlatMethod retains its independently selected computed/count metadata. Database proc_arity, project_command_arities and project_original_command_arities consume this same header projection. Body-only changes preserve declaration/header equality and database arity output. The Eq projection cannot recreate a whole-source, formal-value or callee-entry receipt and supplies no native handler, successful binding/call, physical header/frame or Normal completion. SourceDeclarationSignature::from_original_procedure is the public body-free issuer from independently retained canonical original procedure metadata. It preserves publication geometry, declaration coordinates, formal origin/count and parameter grammar without source-body bytes or installed identity.

## Scope

Three marked Rust controls compare required-after-default grammar across five C source profiles and Jim: C minimum3 versus Jim minimum2, with the same maximum3 and independently retained grammar/origin. Changing only the procedure body preserves item signatures/file declarations and both database arity projections. Clearing reporting parameter labels/all_procs does not replace the immutable original count header; Unknown remains open with no selected grammar. These are source/header/cache-equality controls, separate from finite native Formal185 argc0..5 observations and from successful original argument binding.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

This body-free source formal count/origin/parameter-grammar and database metadata contract supplies no native procedure call, binding, header or frame observation.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

This body-free source formal count/origin/parameter-grammar and database metadata contract supplies no native procedure call, binding, header or frame observation.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

This body-free source formal count/origin/parameter-grammar and database metadata contract supplies no native procedure call, binding, header or frame observation.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

This body-free source formal count/origin/parameter-grammar and database metadata contract supplies no native procedure call, binding, header or frame observation.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

This body-free source formal count/origin/parameter-grammar and database metadata contract supplies no native procedure call, binding, header or frame observation.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This body-free source formal count/origin/parameter-grammar and database metadata contract supplies no native procedure call, binding, header or frame observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This body-free source formal count/origin/parameter-grammar and database metadata contract supplies no native procedure call, binding, header or frame observation.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/signature_scan/formal_count.rs](../../../../rust/tcl-compiler/src/signature_scan/formal_count.rs), `SourceFormalCountProjection`: Carry only body-free count, source/authored/unknown origin and independently selected parameter grammar; no original word or source receipt.
- [rust/tcl-compiler/src/analyser/item_tree.rs](../../../../rust/tcl-compiler/src/analyser/item_tree.rs), `SourceDeclarationSignature::formal_count_projection`: Retain the immutable original declaration count projection independently of mutable reporting labels.
- [rust/tcl-compiler/src/analyser/item_tree.rs](../../../../rust/tcl-compiler/src/analyser/item_tree.rs), `ItemSig`: Keep body-free formal count metadata separate from procedure body bytes.
- [rust/tcl-compiler/src/analyser/item_tree.rs](../../../../rust/tcl-compiler/src/analyser/item_tree.rs), `ItemTree::from_analysis`: Build current headers from genuine original declaration projection while preserving body-edit equality.
- [rust/tcl-compiler/src/analyser/item_tree.rs](../../../../rust/tcl-compiler/src/analyser/item_tree.rs), `FlatMethod`: Retain independently selected method count/computed metadata without a runtime receiver or entered method.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `proc_arity`: Consume the shared body-free count projection rather than reparse parameter reporting labels.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `project_command_arities`: Project compatibility/report arity metadata from the same selected body-free header.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `project_original_command_arities`: Retain original declaration keys and shared count/grammar projection without whole-source or binding authority.
- [rust/tcl-compiler/src/analyser/item_tree.rs](../../../../rust/tcl-compiler/src/analyser/item_tree.rs), `SourceDeclarationSignature::from_original_procedure`: Issue a body-free source header from genuine canonical original procedure metadata, without installation or argument entry.
- [rust/tcl-compiler/src/analyser/item_tree.rs](../../../../rust/tcl-compiler/src/analyser/item_tree.rs), `item_tree::tests::formal_count_headers_retain_c_and_jim_grammar_without_body_bytes` (linked): Five C source profiles and Jim retain their distinct original required-after-default minimum3/minimum2 grammar with maximum3; changing body bytes preserves item signatures/file declarations and original count projection.
- [rust/tcl-compiler/src/analyser/item_tree.rs](../../../../rust/tcl-compiler/src/analyser/item_tree.rs), `item_tree::tests::original_formal_count_header_ignores_reported_parameter_labels` (linked): Cleared reporting params and all_procs cannot replace the original {a args} count header; Unknown projection stays open with no parameter grammar.
- [rust/tcl-lsp-db/src/lib.rs](../../../../rust/tcl-lsp-db/src/lib.rs), `tests::formal_count_project_headers_keep_the_selected_c_and_jim_contract` (linked): Five C source profiles and Jim retain selected grammar/origin and minimum3/minimum2 count in both original-key and reporting arity maps; body-only edits preserve header and project arity equality.

A named test is a coverage binding, not a claim that it executed.

## Replay

Marked selectors bind body-free metadata and current source grammar coverage. Exact Rust outcomes remain independently retained; no native provider result or original whole-source receipt is attached to this header contract.
