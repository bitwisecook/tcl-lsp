# naming.compiler.retained-representation-metadata

Kind: `implementation-contract`

## Problem statement

Representation diagnostics combine lexical/numeric/expression policy with selected command hints. A function with missing, foreign or stale metadata cannot borrow those hints from a catalogue profile or a separately supplied SSA.

## Question

How do representation diagnostic consumers retain complete actual availability and lexical, numeric and expression policies while known replacements and missing, foreign or stale owners cannot borrow stock Normal conversion hints?

## Conclusion

ShimmerContext::for_function retains the actual FunctionUnit input, immutable command store, complete availability and exact normalized source configuration. Missing, foreign and config-drift input refuses before conversion metadata is selected. All analysed use-site, commit, expression, sharing, thunking and byte-array consumers carry this same owner through source replay, as does CompilerChecks::shimmer_family_checks. The selected expression parser and word/number policies come from that retained input/config, while independently required normal-representation invocation and original expression proofs remain separate. Known source replacement cannot inherit the stock conversion hint. ShimmerContext::standalone is an explicitly separate compatibility ingress for independently supplied SSA; it cannot repair an analysed function that lacks input. These source diagnostic candidates and refusal controls establish no actual Native conversion, opcode admission, physical object/header/cache/lifetime, entered body/frame or safe rewrite. TypePropagationMetadata::for_function retains the same actual complete availability, exact normalized source lexer configuration and separately selected numeric/list/expression grammar. WordTypingCtx and StatementTypingCtx carry it through conditional normal-result/transfer candidates and callee-result inference. Genuine current positive result hints remain available, while missing/foreign/config drift and known replacements refuse before stock type hints are selected. These analytical type candidates do not establish a reached Native Normal worker, object primary/cache, compiler admission or runtime value.

Two additional marked source definitions preserve literal dollar-prefixed and unmatched Unicode scalar roots in original SSA increment reads and byte-provenance diagnostics. Both use the genuine retained function metadata and source view rather than reconstructing names from sigils or element syntax.

## Scope

Three marked source/API controls cover complete availability and selected grammar; genuine positive source reads with missing/foreign owner refusal; and known replacement versus stock conversion hints. The shared owner feeds analysed representation consumers across all six diagnostic families and CompilerChecks. All seven providers are not tested for this API contract; actual native result/header/compiler/effect observations and separate Normal/equivalence purposes are unchanged. Two marked type controls independently vary actual command availability versus selected source grammar and require positive current results with missing/foreign/replacement refusal. Complete packages and original source tokens remain explicit premises; all seven provider observations stay not tested for this source/API contract.

These conditional SSA/representation projections establish no reached C conversion, byte-array primary/cache, native header identity, execution, current store, observer exclusion or safe edit. Existing independently pinned outcomes and original Native records retain their own exact scope; no assertion result is attached to these definitions.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No external provider execution answers this source/API implementation contract. Its selected metadata, source geometry and refusal controls are independent of actual Native execution, presence, storage/header/cache/frame/Normal purposes.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

No external provider execution answers this source/API implementation contract. Its selected metadata, source geometry and refusal controls are independent of actual Native execution, presence, storage/header/cache/frame/Normal purposes.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

No external provider execution answers this source/API implementation contract. Its selected metadata, source geometry and refusal controls are independent of actual Native execution, presence, storage/header/cache/frame/Normal purposes.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

No external provider execution answers this source/API implementation contract. Its selected metadata, source geometry and refusal controls are independent of actual Native execution, presence, storage/header/cache/frame/Normal purposes.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

No external provider execution answers this source/API implementation contract. Its selected metadata, source geometry and refusal controls are independent of actual Native execution, presence, storage/header/cache/frame/Normal purposes.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No external provider execution answers this source/API implementation contract. Its selected metadata, source geometry and refusal controls are independent of actual Native execution, presence, storage/header/cache/frame/Normal purposes.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

No external provider execution answers this source/API implementation contract. Its selected metadata, source geometry and refusal controls are independent of actual Native execution, presence, storage/header/cache/frame/Normal purposes.

## Exact evidence

- `naming-compiler-retained-representation-metadata-rust-tcl-compiler-src-compiler_checks.rs` (implementation): [rust/tcl-compiler/src/compiler_checks.rs](../../../../rust/tcl-compiler/src/compiler_checks.rs). SHA-256 `7796fff0549096389c0a0a33927b8ad28e1fc9f4c974389e8073b3c0f5052443`. Current actual source/API owner and marked assertion definition; no executed Rust or native provider result.
- `naming-compiler-retained-representation-metadata-rust-tcl-compiler-src-shimmer-source_context.rs` (implementation): [rust/tcl-compiler/src/shimmer/source_context.rs](../../../../rust/tcl-compiler/src/shimmer/source_context.rs). SHA-256 `804b47bd5cd96b2a99fd19fd724e21953485bf70b3fdf44a618cb7ede997c59a`. Current actual source/API owner and marked assertion definition; no executed Rust or native provider result.
- `naming-compiler-retained-representation-metadata-rust-tcl-compiler-src-word_subst.rs` (implementation): [rust/tcl-compiler/src/word_subst.rs](../../../../rust/tcl-compiler/src/word_subst.rs). SHA-256 `8fd27d0299f65224f11d4dc82ceb920afad5daf1d0161ce186200efbe680ba62`. Current actual source/API owner and marked assertion definition; no executed Rust or native provider result.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/shimmer/source_context.rs](../../../../rust/tcl-compiler/src/shimmer/source_context.rs), `ShimmerContext`: Retain full actual function metadata and lexer/numeric/expression policy across analysed representation consumers without granting physical conversion/cache identity.
- [rust/tcl-compiler/src/shimmer/source_context.rs](../../../../rust/tcl-compiler/src/shimmer/source_context.rs), `ShimmerContext::for_function`: Validate actual function generation/store and normalized source configuration; missing/foreign/config drift refuses rather than reconstructing a profile.
- [rust/tcl-compiler/src/shimmer/source_context.rs](../../../../rust/tcl-compiler/src/shimmer/source_context.rs), `ShimmerContext::standalone`: Explicitly select standalone compatibility metadata for independently supplied SSA, without recovering absent analysed-function ownership.
- [rust/tcl-compiler/src/compiler_checks.rs](../../../../rust/tcl-compiler/src/compiler_checks.rs), `CompilerChecks::shimmer_family_checks`: Route representation diagnostic families through the actual FunctionUnit context rather than catalogue-only selection.
- [rust/tcl-compiler/src/word_subst.rs](../../../../rust/tcl-compiler/src/word_subst.rs), `representation_expression_with_metadata_context`: Join original representation expression source geometry to actual availability and selected parser policy while preserving independent invocation/Normal proof.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `TypePropagationMetadata`: Group actual full function metadata and exact source configuration for conditional type/transfer/result candidates without a Native Normal/value grant.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `TypePropagationMetadata::for_function`: Validate actual retained input/store and normalized config before typing; preserve numeric grammar independently of command catalogue availability.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `WordTypingCtx`: Carry selected original word grammar and actual metadata through type candidates.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `StatementTypingCtx`: Carry actual availability/source grammar and independently required conditional result/transfer purposes.
- [rust/tcl-compiler/src/shimmer/source_context.rs](../../../../rust/tcl-compiler/src/shimmer/source_context.rs), `shimmer::source_context::tests::original_representation_context_keeps_complete_availability_and_selected_grammar` (linked): Complete current packages/authoring availability and selected source lexer/numeric/expression policies come from actual FunctionUnit input/config; missing/foreign/config drift refuses.
- [rust/tcl-compiler/src/shimmer/source_context.rs](../../../../rust/tcl-compiler/src/shimmer/source_context.rs), `shimmer::source_context::tests::original_representation_consumers_keep_positive_reads_and_refuse_missing_foreign_input` (linked): Genuine source read hints remain positive under their actual current function owner across diagnostic consumers; missing/foreign input cannot borrow stock Normal conversion hints or reconstruct source state.
- [rust/tcl-compiler/src/shimmer/source_context.rs](../../../../rust/tcl-compiler/src/shimmer/source_context.rs), `shimmer::source_context::tests::original_representation_known_replacement_cannot_borrow_stock_conversion_hints` (linked): A genuine known source replacement is terminal for stock conversion hints; original operand/source owners and current availability remain required independently of name/report text.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `type_infer::tests::original_type_metadata_retains_actual_availability_and_independent_source_grammar` (linked): Actual C8.4 command store and separately selected Plain lexical/numeric/list/braced grammar retain complete required-package context; config drift refuses. No Native AST/cache/header identity follows.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `type_infer::tests::original_type_consumers_keep_positive_normal_results_and_refuse_missing_foreign_metadata` (linked): Genuine current positive analytical Int/result candidates are retained; absent/foreign actual metadata and known stock replacement cannot borrow result/transfer hints. No reached Native Normal completion is observed.

A named test is a coverage binding, not a claim that it executed.

- [rust/tcl-compiler/src/shimmer/commit.rs](../../../../rust/tcl-compiler/src/shimmer/commit.rs), `typed_reads_of_statement`: Project representation expectations from the original SSA use/symbol and retained source view without reparsing a stored literal name.
- [rust/tcl-compiler/src/shimmer/byte_array.rs](../../../../rust/tcl-compiler/src/shimmer/byte_array.rs), `find_byte_array_warnings_with_context`: Keep source byte-provenance advice on actual retained function metadata and original literal store/read roots, independently of physical cache or conversion evidence.
- [rust/tcl-compiler/src/shimmer/commit.rs](../../../../rust/tcl-compiler/src/shimmer/commit.rs), `shimmer::commit::tests::retained_increment_reads_use_literal_operand_names_without_reparsing_sigils` (linked): An actual retained FunctionUnit projects increment reads from literal original SSA names $value and unmatched Unicode é(, preserving their original symbol/version rather than stripping sigils or reparsing an element.
- [rust/tcl-compiler/src/shimmer/byte_array.rs](../../../../rust/tcl-compiler/src/shimmer/byte_array.rs), `shimmer::byte_array::tests::retained_byte_provenance_uses_literal_sigil_and_unicode_store_roots` (linked): Actual retained byte-provenance diagnostics keep literal sigil and Unicode store roots distinct from their plain names while selecting the same current function metadata and original source reads.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.
