# naming.minifier.retained-lexical-context

Kind: `implementation-contract`

## Problem statement

A minifier can check the retained source and grammar yet discard the actual availability context when reconstructing a Realm or selecting roles and case layout from a heterogeneous Registry.

## Question

Does Logical minification preserve its actual full editing context through roles, case layout and rewritten-source analysis, while refusing unsupported axes?

## Conclusion

The Logical compatibility branch requires a current complete source image, the actual full lexer configuration, equal analyser/unit profile policies and a structurally matching supported ResolvedContext assembled over the retained command store. MinifyEnv borrows that actual immutable ContextRegistry and retained command Realm. Authored role and case queries use the shared structured Registry resolver under that full context; dynamic values and unknown expansion cardinality retain their separate facets. Compact and aggressive intermediate text is analysed using the same complete ResolvedAnalysisInput. Any unsupported context or configuration preserves the entire input with a typed refusal, an empty symbol map and zero rewrites; it does not enter a Native syntax fallback.

## Scope

Explicitly selected Logical compatibility minification. Source metadata shape supplies no Native implementation, namespace, frame, compiler, value, Normal, observer or edit permission. Current Native and hosted original-input planners keep their existing independent terminal paths. Remaining legacy semantic/insertion passes support only the stated profile-default configuration and full matching context; arbitrary keyed, package, floor, provider or authoring axes remain refused.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust editing-context invariant.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust editing-context invariant.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust editing-context invariant.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust editing-context invariant.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No native interpreter observation establishes this Rust editing-context invariant.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native interpreter observation establishes this Rust editing-context invariant.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native interpreter observation establishes this Rust editing-context invariant.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-lsp-core/src/minify/original.rs](../../../../rust/tcl-lsp-core/src/minify/original.rs), `minify_with_analysis`: Validate current source and all actual input axes; unsupported Logical contexts/configurations return exact source with typed refusal before any pass.
- [rust/tcl-lsp-core/src/minify.rs](../../../../rust/tcl-lsp-core/src/minify.rs), `lexical_default`: Carry actual immutable ContextRegistry, full lexer configuration and retained Realm through MinifyEnv.
- [rust/tcl-lsp-core/src/minify.rs](../../../../rust/tcl-lsp-core/src/minify.rs), `lexical_source_schema`: Select authored roles and case layout through the shared structured resolver under the retained full ResolvedContext and exact structured value/cardinality facets.
- [rust/tcl-lsp-core/src/minify/original.rs](../../../../rust/tcl-lsp-core/src/minify/original.rs), `analysis_with_input`: Reanalyse changed compatibility text with the same immutable complete ResolvedAnalysisInput instead of rebuilding it from a label.
- [rust/tcl-lsp-core/src/minify/original.rs](../../../../rust/tcl-lsp-core/src/minify/original.rs), `minify::original::tests::lexical_roles_use_retained_context_instead_of_heterogeneous_last_spec` (linked): The actual context selects Tcl expression roles and traits over the heterogeneous store, preserving supported expression minification.
- [rust/tcl-lsp-core/src/minify/original.rs](../../../../rust/tcl-lsp-core/src/minify/original.rs), `minify::original::tests::lexical_context_refusal_retains_source_for_every_tier` (linked): Identical profile/configuration/store with a foreign availability context returns exact source, empty symbols and zero rewrites in all three tiers.
- [rust/tcl-lsp-core/src/minify/original.rs](../../../../rust/tcl-lsp-core/src/minify/original.rs), `minify::original::tests::lexical_custom_configuration_refuses_every_tier_without_fallback` (linked): A current full-source analysis with an independently changed quoting configuration returns exact source with a configuration refusal for all tiers.
- [rust/tcl-lsp-core/src/minify/original.rs](../../../../rust/tcl-lsp-core/src/minify/original.rs), `minify::original::tests::lexical_intermediate_source_retains_actual_input_generation` (linked): Changed source is reanalysed with the same immutable full input and context generation; current image checks distinguish the original and rewritten text.
- [rust/tcl-lsp-core/src/minify/original.rs](../../../../rust/tcl-lsp-core/src/minify/original.rs), `minify::original::tests::lexical_case_shape_retains_dynamic_subject_and_expansion_uncertainty` (linked): The selected case descriptor accepts an unknown subject value at a known ordinal; unknown expansion cardinality remains absent.

A named test is a coverage binding, not a claim that it executed.

## Replay

Fixed Rust selectors exercise the retained Logical/Native/hosted lexical context and independent minifier premises. Separate exact-source validation receipts establish their outcomes; this implementation record supplies no native observation.
