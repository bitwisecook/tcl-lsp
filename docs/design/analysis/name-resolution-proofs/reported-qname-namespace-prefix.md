# naming.analysis.reported-qname-namespace-prefix

Kind: `implementation-contract`

## Problem statement

Analytical QName consumers need different namespace-prefix presentations. A relative prefix, an overlapping colon pair or an empty tail must not silently select another consumer purpose, and a displayed prefix must not substitute for retained source, namespace or construction ownership.

## Question

How do AfterLeadingRoot, RootRelativePrefix and LiteralPrefix preserve their selected literal QName namespace presentations, and which independent retained source, namespace and construction owners govern actual inlining or original constructor reasoning?

## Conclusion

reported_qname_namespace owns one literal prefix query with explicit metadata purposes. AfterLeadingRoot searches beyond one leading root marker; RootRelativePrefix roots a relative selected prefix; LiteralPrefix preserves the literal prefix. Missing selected prefixes report ::, with overlapping colon pairs, trailing tails, Unicode and NUL retained as reporting text. Compatibility constructor advice uses LiteralPrefix; test-only adapters exercise the other reporting purposes. Actual inlining retains complete Module/source/input and procedure-allocation correspondence and uses body namespace_context or Script::execution_namespace independently of displayed procedure names. inline_uplevel_passthrough revalidates the positioned source producer and exact selected allocating implementation before mutation; effective alias prefixes, body grammar and frame/completion restrictions remain independent. Original construction advice requires its own function metadata, positioned invocation and proved result. Tokenless catalogue advice grants no original construction or Native namespace witness. Reporting queries supply no source owner, lookup, namespace liveness, frame or constructor effect. One named pure reporting-QName syntax control passed in the separately retained software receipt; other linked controls remain definitions, and no external provider execution is attached.

## Scope

Current pure reporting-QName API, compatibility type-advice projection, test-only purpose adapters and source-owned inlining boundaries. Four existing reporting controls remain unchanged. Colon examples assert reporting String projections only. Constructed keys, written Tcl qualifier grammar, actual retained byte namespace paths and native incarnation identity remain separate. Body IDs and retained namespace contexts select actual analysis owners; report prefixes do not substitute for them. No C Tcl, Jim or BIG-IP process applies to the pure presentation question. No provider result, physical dispatch, namespace lifetime, Native getter/header/frame, construction effect or execution beyond the named software control is inferred.

## Provider answers

### tcl8.4

Status: `not-tested`. Channel: Unicode analytical QName source/API controls only. Version: tcl8.4 not applicable to this pure presentation question.

External tcl8.4 execution is not applicable to the pure reporting-prefix question. No provider namespace interpretation, lookup, object/header getter, frame, constructor or body effect is tested or inferred. Independently retained source/namespace/construction owners are software API requirements, not a provider experiment.

### tcl8.5

Status: `not-tested`. Channel: Unicode analytical QName source/API controls only. Version: tcl8.5 not applicable to this pure presentation question.

External tcl8.5 execution is not applicable to the pure reporting-prefix question. No provider namespace interpretation, lookup, object/header getter, frame, constructor or body effect is tested or inferred. Independently retained source/namespace/construction owners are software API requirements, not a provider experiment.

### tcl8.6

Status: `not-tested`. Channel: Unicode analytical QName source/API controls only. Version: tcl8.6 not applicable to this pure presentation question.

External tcl8.6 execution is not applicable to the pure reporting-prefix question. No provider namespace interpretation, lookup, object/header getter, frame, constructor or body effect is tested or inferred. Independently retained source/namespace/construction owners are software API requirements, not a provider experiment.

### tcl9.0

Status: `not-tested`. Channel: Unicode analytical QName source/API controls only. Version: tcl9.0 not applicable to this pure presentation question.

External tcl9.0 execution is not applicable to the pure reporting-prefix question. No provider namespace interpretation, lookup, object/header getter, frame, constructor or body effect is tested or inferred. Independently retained source/namespace/construction owners are software API requirements, not a provider experiment.

### tcl9.1

Status: `not-tested`. Channel: Unicode analytical QName source/API controls only. Version: tcl9.1 not applicable to this pure presentation question.

External tcl9.1 execution is not applicable to the pure reporting-prefix question. No provider namespace interpretation, lookup, object/header getter, frame, constructor or body effect is tested or inferred. Independently retained source/namespace/construction owners are software API requirements, not a provider experiment.

### jim

Status: `not-tested`. Channel: Unicode analytical QName source/API controls only. Version: jim not applicable to this pure presentation question.

External jim execution is not applicable to the pure reporting-prefix question. No provider namespace interpretation, lookup, object/header getter, frame, constructor or body effect is tested or inferred. Independently retained source/namespace/construction owners are software API requirements, not a provider experiment.

### bigip

Status: `not-tested`. Channel: Unicode analytical QName source/API controls only. Version: bigip not applicable to this pure presentation question.

External bigip execution is not applicable to the pure reporting-prefix question. No provider namespace interpretation, lookup, object/header getter, frame, constructor or body effect is tested or inferred. Independently retained source/namespace/construction owners are software API requirements, not a provider experiment.

## Consumer bindings

- [rust/tcl-syntax/src/naming/reporting.rs](../../../../rust/tcl-syntax/src/naming/reporting.rs), `ReportedQNameNamespacePurpose`: Select AfterLeadingRoot, RootRelativePrefix or LiteralPrefix metadata geometry; identical projections share a purpose independently of the consumer, without written/native-name or lookup authority.
- [rust/tcl-syntax/src/naming/reporting.rs](../../../../rust/tcl-syntax/src/naming/reporting.rs), `reported_qname_namespace`: One literal last-:: prefix query preserves each selected reporting purpose and root default, including overlapping pairs, trailing tails, Unicode and NUL text.
- `rust/tcl-compiler/src/inlining/mod.rs`, `root_namespace_of`: Test-only AfterLeadingRoot reporting adapter; no production body namespace, Module admission or execution owner is issued.
- [rust/tcl-compiler/src/inline_uplevel.rs](../../../../rust/tcl-compiler/src/inline_uplevel.rs), `inline_uplevel_passthrough`: Require matching retained Module/source/input/Registry and an actual source-selected procedure allocation before mutation; typed body IDs choose candidates, and procedure body namespace_context supplies the caller coordinate. Reporting QNames do not select it.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `function_namespace`: Delegate constructor-advice reporting prefix selection; a label does not issue original construction or namespace evidence.
- `rust/tcl-compiler/src/inlining/frame.rs`, `SourceContext::for_module`: Require current retained whole Module and Logical source input before call counts or inlining eligibility.
- [rust/tcl-compiler/src/command_binding/retained_source.rs](../../../../rust/tcl-compiler/src/command_binding/retained_source.rs), `matches_original_procedure`: Require the original procedure name/formals/body/source/namespace correspondence before admission to the inline catalogue.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `TypePropagationMetadata::for_function`: Retain actual function source input/store/config; missing or foreign metadata cannot be repaired from cfg.name.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `command_word_result_type`: Original construction advice selects the independently proved construction result or actual selected invocation facts, not a reporting QName.
- [rust/tcl-compiler/src/ir.rs](../../../../rust/tcl-compiler/src/ir.rs), `Script::execution_namespace`: Prefer actual retained SourceNamespaceKey over a presentation-derived default.
- [rust/tcl-syntax/src/naming/reporting.rs](../../../../rust/tcl-syntax/src/naming/reporting.rs), `naming::reporting::tests::reported_qname_namespaces_keep_purpose_and_literal_text` (linked): All three pure purposes: no separator/empty/root, relative prefix, overlapping colons, trailing empty tail, Unicode and counted NUL text. This named fixed syntax control passed in the separately retained software receipt; no external provider execution is attached.
- `rust/tcl-compiler/src/inlining/mod.rs`, `inlining::reported_namespace_tests::procedure_body_reporting_namespace_retains_its_selected_prefix` (linked): Actual private inlining delegate selects AfterLeadingRoot on fixed labels. Source-defined fixed software control only; no matching owned executable/assertion result or external provider execution is attached.
- [rust/tcl-compiler/src/inline_uplevel.rs](../../../../rust/tcl-compiler/src/inline_uplevel.rs), `inline_uplevel::reported_namespace_tests::uplevel_caller_reporting_namespace_roots_relative_prefixes` (linked): Actual private uplevel delegate selects RootRelativePrefix on fixed labels. Source-defined fixed software control only; no matching owned executable/assertion result or external provider execution is attached.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `type_infer::reported_namespace_tests::constructor_advice_reporting_namespace_preserves_literal_prefixes` (linked): Actual private constructor-advice delegate selects LiteralPrefix on fixed labels. Source-defined fixed software control only; no matching owned executable/assertion result or external provider execution is attached.

## Evidence

- [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs)
- [rust/tcl-syntax/src/naming/reporting.rs](../../../../rust/tcl-syntax/src/naming/reporting.rs)
- [rust/tcl-compiler/src](../../../../rust/tcl-compiler/src)
- [rust/tcl-compiler/src/command_binding/retained_source.rs](../../../../rust/tcl-compiler/src/command_binding/retained_source.rs)
- [rust/tcl-compiler/src/command_binding/namespace_context.rs](../../../../rust/tcl-compiler/src/command_binding/namespace_context.rs)
- [rust/tcl-compiler/src/registry_invocation/metadata_context.rs](../../../../rust/tcl-compiler/src/registry_invocation/metadata_context.rs)
