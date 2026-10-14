# naming.analysis.reported-qname-namespace-prefix

Kind: `implementation-contract`

## Problem statement

Analytical QName consumers need different namespace-prefix presentations. A relative prefix, an overlapping colon pair or an empty tail must not silently select another consumer purpose, and a displayed prefix must not substitute for retained source, namespace or construction ownership.

## Question

How do AfterLeadingRoot, RootRelativePrefix and LiteralPrefix preserve their selected literal QName namespace presentations, and which independent retained source, namespace and construction owners govern actual inlining or original constructor reasoning?

## Conclusion

reported_qname_namespace owns one literal prefix query with explicit metadata purposes. AfterLeadingRoot searches beyond one leading root marker; RootRelativePrefix roots a relative selected prefix; LiteralPrefix preserves the literal prefix. Missing selected prefixes report ::, with overlapping colon pairs, trailing tails, Unicode and NUL retained as reporting text. Compatibility constructor advice uses LiteralPrefix; test-only adapters exercise the other reporting purposes. Actual inlining retains complete Module/source/input and procedure-allocation correspondence and uses body namespace_context or Script::execution_namespace independently of displayed procedure names. inline_uplevel_passthrough revalidates the positioned source producer and exact selected allocating implementation before mutation; effective alias prefixes, body grammar and frame/completion restrictions remain independent. Original construction advice requires its own function metadata, positioned invocation and proved result. Tokenless catalogue advice grants no original construction or Native namespace witness. Reporting queries supply no source owner, lookup, namespace liveness, frame or constructor effect. Linked fixed controls are definitions; no current assertion result or external provider execution is attached.

## Scope

Current pure reporting-QName API, compatibility type-advice projection, test-only purpose adapters and source-owned inlining boundaries. Four existing reporting controls remain unchanged. Colon examples assert reporting String projections only. Constructed keys, written Tcl qualifier grammar, actual retained byte namespace paths and native incarnation identity remain separate. Body IDs and retained namespace contexts select actual analysis owners; report prefixes do not substitute for them. No C Tcl, Jim or BIG-IP process applies to the pure presentation question. No provider result, physical dispatch, namespace lifetime, Native getter/header/frame, construction effect or executed assertion is inferred.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not applicable to this pure presentation question. Build: No original provider process applies to this Rust reporting query.. Channel: Unicode analytical QName source/API controls only.. Dialect: tcl8.4.

External tcl8.4 execution is not applicable to the pure reporting-prefix question. No provider namespace interpretation, lookup, object/header getter, frame, constructor or body effect is tested or inferred. Independently retained source/namespace/construction owners are software API requirements, not a provider experiment.

### tcl8.5

Status: `not-tested`. Version: not applicable to this pure presentation question. Build: No original provider process applies to this Rust reporting query.. Channel: Unicode analytical QName source/API controls only.. Dialect: tcl8.5.

External tcl8.5 execution is not applicable to the pure reporting-prefix question. No provider namespace interpretation, lookup, object/header getter, frame, constructor or body effect is tested or inferred. Independently retained source/namespace/construction owners are software API requirements, not a provider experiment.

### tcl8.6

Status: `not-tested`. Version: not applicable to this pure presentation question. Build: No original provider process applies to this Rust reporting query.. Channel: Unicode analytical QName source/API controls only.. Dialect: tcl8.6.

External tcl8.6 execution is not applicable to the pure reporting-prefix question. No provider namespace interpretation, lookup, object/header getter, frame, constructor or body effect is tested or inferred. Independently retained source/namespace/construction owners are software API requirements, not a provider experiment.

### tcl9.0

Status: `not-tested`. Version: not applicable to this pure presentation question. Build: No original provider process applies to this Rust reporting query.. Channel: Unicode analytical QName source/API controls only.. Dialect: tcl9.0.

External tcl9.0 execution is not applicable to the pure reporting-prefix question. No provider namespace interpretation, lookup, object/header getter, frame, constructor or body effect is tested or inferred. Independently retained source/namespace/construction owners are software API requirements, not a provider experiment.

### tcl9.1

Status: `not-tested`. Version: not applicable to this pure presentation question. Build: No original provider process applies to this Rust reporting query.. Channel: Unicode analytical QName source/API controls only.. Dialect: tcl9.1.

External tcl9.1 execution is not applicable to the pure reporting-prefix question. No provider namespace interpretation, lookup, object/header getter, frame, constructor or body effect is tested or inferred. Independently retained source/namespace/construction owners are software API requirements, not a provider experiment.

### jim

Status: `not-tested`. Version: not applicable to this pure presentation question. Build: No original provider process applies to this Rust reporting query.. Channel: Unicode analytical QName source/API controls only.. Dialect: jim.

External jim execution is not applicable to the pure reporting-prefix question. No provider namespace interpretation, lookup, object/header getter, frame, constructor or body effect is tested or inferred. Independently retained source/namespace/construction owners are software API requirements, not a provider experiment.

### bigip

Status: `not-tested`. Version: not applicable to this pure presentation question. Build: No original provider process applies to this Rust reporting query.. Channel: Unicode analytical QName source/API controls only.. Dialect: f5-bigip.

External bigip execution is not applicable to the pure reporting-prefix question. No provider namespace interpretation, lookup, object/header getter, frame, constructor or body effect is tested or inferred. Independently retained source/namespace/construction owners are software API requirements, not a provider experiment.

## Exact evidence

- `reporting-source-0` (implementation): [rust/tcl-syntax/src/naming.rs](../../../../rust/tcl-syntax/src/naming.rs). SHA-256 `4808699cc117bb840c8ce9e9360bd7186f3f294ffb239d8ece2b75279ebf52f4`. Current pure reporting owner/delegate or independently retained source/namespace/metadata guard. Source definition only; no Rust assertion outcome, Native process or execution effect.
- `reporting-source-1` (implementation): [rust/tcl-syntax/src/naming/reporting.rs](../../../../rust/tcl-syntax/src/naming/reporting.rs). SHA-256 `009349c67795cd1cab75ab2066d12f180eda1af8f71ff2265656d776cd6e00f5`. Current pure reporting owner/delegate or independently retained source/namespace/metadata guard. Source definition only; no Rust assertion outcome, Native process or execution effect.
- `reporting-source-2` (implementation): [rust/tcl-compiler/src/inlining/mod.rs](../../../../rust/tcl-compiler/src/inlining/mod.rs). SHA-256 `d65b2d7f142c6c46beaab76082c0bb523c0362b57bd5ca049500d027742652a7`. Current pure reporting owner/delegate or independently retained source/namespace/metadata guard. Source definition only; no Rust assertion outcome, Native process or execution effect.
- `reporting-source-3` (implementation): [rust/tcl-compiler/src/inline_uplevel.rs](../../../../rust/tcl-compiler/src/inline_uplevel.rs). SHA-256 `0c9ca8564340860aa032cb2f8843acad05566eccb54d05181a5ee78920974420`. Current pure reporting owner/delegate or independently retained source/namespace/metadata guard. Source definition only; no Rust assertion outcome, Native process or execution effect.
- `reporting-source-4` (implementation): [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs). SHA-256 `d45458b7b35efb104b9b13bfb0468d01e1a2f315c9fd32ae9d35e08adb800328`. Current pure reporting owner/delegate or independently retained source/namespace/metadata guard. Source definition only; no Rust assertion outcome, Native process or execution effect.
- `reporting-source-5` (implementation): [rust/tcl-compiler/src/inlining/frame.rs](../../../../rust/tcl-compiler/src/inlining/frame.rs). SHA-256 `1f2482814c4ab1f614371990bc498025b8bc0d871a477ef4697bb8301c434d17`. Current pure reporting owner/delegate or independently retained source/namespace/metadata guard. Source definition only; no Rust assertion outcome, Native process or execution effect.
- `reporting-source-6` (implementation): [rust/tcl-compiler/src/command_binding/retained_source.rs](../../../../rust/tcl-compiler/src/command_binding/retained_source.rs). SHA-256 `914ceb69a4e2ab42af4b0ac99ede3a8c3374fc7743090488fe29b7cf0126e181`. Current pure reporting owner/delegate or independently retained source/namespace/metadata guard. Source definition only; no Rust assertion outcome, Native process or execution effect.
- `reporting-source-7` (implementation): [rust/tcl-compiler/src/command_binding/namespace_context.rs](../../../../rust/tcl-compiler/src/command_binding/namespace_context.rs). SHA-256 `f10f446e0076bac46970a2c63d4c28562a16b6d70a6e064734fad17d5aad4268`. Current pure reporting owner/delegate or independently retained source/namespace/metadata guard. Source definition only; no Rust assertion outcome, Native process or execution effect.
- `reporting-source-8` (implementation): [rust/tcl-compiler/src/registry_invocation/metadata_context.rs](../../../../rust/tcl-compiler/src/registry_invocation/metadata_context.rs). SHA-256 `d6585ad7853c73f600b5f7a1bc0ba2dd59728cce7d1d0b944ef029c14a066ffd`. Current pure reporting owner/delegate or independently retained source/namespace/metadata guard. Source definition only; no Rust assertion outcome, Native process or execution effect.
- `reporting-source-9` (implementation): [rust/tcl-compiler/src/ir.rs](../../../../rust/tcl-compiler/src/ir.rs). SHA-256 `141881e65e31d98be82d0b384d67a4e6a75da110503351d8c53d53df506c8b5b`. Current pure reporting owner/delegate or independently retained source/namespace/metadata guard. Source definition only; no Rust assertion outcome, Native process or execution effect.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/naming/reporting.rs](../../../../rust/tcl-syntax/src/naming/reporting.rs), `ReportedQNameNamespacePurpose`: Select AfterLeadingRoot, RootRelativePrefix or LiteralPrefix metadata geometry; identical projections share a purpose independently of the consumer, without written/native-name or lookup authority.
- [rust/tcl-syntax/src/naming/reporting.rs](../../../../rust/tcl-syntax/src/naming/reporting.rs), `reported_qname_namespace`: One literal last-:: prefix query preserves each selected reporting purpose and root default, including overlapping pairs, trailing tails, Unicode and NUL text.
- [rust/tcl-compiler/src/inlining/mod.rs](../../../../rust/tcl-compiler/src/inlining/mod.rs), `root_namespace_of`: Test-only AfterLeadingRoot reporting adapter; no production body namespace, Module admission or execution owner is issued.
- [rust/tcl-compiler/src/inline_uplevel.rs](../../../../rust/tcl-compiler/src/inline_uplevel.rs), `inline_uplevel_passthrough`: Require matching retained Module/source/input/Registry and an actual source-selected procedure allocation before mutation; typed body IDs choose candidates, and procedure body namespace_context supplies the caller coordinate. Reporting QNames do not select it.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `function_namespace`: Delegate constructor-advice reporting prefix selection; a label does not issue original construction or namespace evidence.
- [rust/tcl-compiler/src/inlining/frame.rs](../../../../rust/tcl-compiler/src/inlining/frame.rs), `SourceContext::for_module`: Require current retained whole Module and Logical source input before call counts or inlining eligibility.
- [rust/tcl-compiler/src/command_binding/retained_source.rs](../../../../rust/tcl-compiler/src/command_binding/retained_source.rs), `matches_original_procedure`: Require the original procedure name/formals/body/source/namespace correspondence before admission to the inline catalogue.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `TypePropagationMetadata::for_function`: Retain actual function source input/store/config; missing or foreign metadata cannot be repaired from cfg.name.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `command_word_result_type`: Original construction advice selects the independently proved construction result or actual selected invocation facts, not a reporting QName.
- [rust/tcl-compiler/src/ir.rs](../../../../rust/tcl-compiler/src/ir.rs), `Script::execution_namespace`: Prefer actual retained SourceNamespaceKey over a presentation-derived default.
- [rust/tcl-syntax/src/naming/reporting.rs](../../../../rust/tcl-syntax/src/naming/reporting.rs), `naming::reporting::tests::reported_qname_namespaces_keep_purpose_and_literal_text` (linked): All three pure purposes: no separator/empty/root, relative prefix, overlapping colons, trailing empty tail, Unicode and counted NUL text. Source-defined fixed software control only; no matching owned executable/assertion result or external provider execution is attached.
- [rust/tcl-compiler/src/inlining/mod.rs](../../../../rust/tcl-compiler/src/inlining/mod.rs), `inlining::reported_namespace_tests::procedure_body_reporting_namespace_retains_its_selected_prefix` (linked): Actual private inlining delegate selects AfterLeadingRoot on fixed labels. Source-defined fixed software control only; no matching owned executable/assertion result or external provider execution is attached.
- [rust/tcl-compiler/src/inline_uplevel.rs](../../../../rust/tcl-compiler/src/inline_uplevel.rs), `inline_uplevel::reported_namespace_tests::uplevel_caller_reporting_namespace_roots_relative_prefixes` (linked): Actual private uplevel delegate selects RootRelativePrefix on fixed labels. Source-defined fixed software control only; no matching owned executable/assertion result or external provider execution is attached.
- [rust/tcl-compiler/src/type_infer.rs](../../../../rust/tcl-compiler/src/type_infer.rs), `type_infer::reported_namespace_tests::constructor_advice_reporting_namespace_preserves_literal_prefixes` (linked): Actual private constructor-advice delegate selects LiteralPrefix on fixed labels. Source-defined fixed software control only; no matching owned executable/assertion result or external provider execution is attached.

A named test is a coverage binding, not a claim that it executed.

## Replay

Four named fixed software controls and exact current source/API bindings are available for later reconfirmation. No owned matching test execution or original external provider observation is attached. Reconfirmation must retain the same consumer purpose and independent original source/namespace/metadata owners; reporting text supplies none of them.
