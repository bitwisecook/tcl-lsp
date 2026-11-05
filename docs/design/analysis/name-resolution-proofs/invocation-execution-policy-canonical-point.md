# naming.invocation.execution-policy-canonical-point

Kind: `implementation-contract`

## Problem statement

A retained native compiler dialect stores an explicit canonical point, while an equivalent C profile may store its typed release with no explicit point. Structural equality then rejects an authentic original compiler inventory. Comparing only a release would instead ignore distinct grammar, character, variable, namespace or build policies. The comparison must normalise only the representation of an independently selected execution point and preserve every other field and authority boundary.

## Question

Can an explicit canonical C execution point match an equivalent typed C profile without accepting different policies or granting compiler authority?

## Conclusion

InvocationDialect::has_same_execution_policy resolves both points with the existing execution_point owner, then compares every full policy field after storing those selected points. It declines unknown or conflicting points, foreign families, release differences, build differences and other field changes. The source compiler query still requires the original compiler/source owner and complete unchanged vector; this equality neither creates a native entry nor changes compiler admission, original-object, cache or Normal facts.

DialectProfile::runtime_core_family centralises core-provider and hosted F5 trunk package family selection. F5 tmsh/iApps keep F5Tcl and iRules keeps F5Irules independently of compatible Tcl8.4 signatures; compatibility supplies no C Native name/string recipe or actual execution point. F5 expression/character and InvocationDialect projections share that family owner. Body scheduling queries independently require execution_point, so conflicting point/release records refuse.

## Scope

Pure complete-policy comparison of selected InvocationDialect records, consumed by the source compiler inventory query. C profile versus explicit canonical representation, Jim exact explicit point, and negative character/grammar/namespace/family/release/unknown/build axes. No native interpreter executes this pure implementation contract; fixed Rust tests independently assert the selected source comparisons.

The added hosted-family control and extended scheduling negatives are software metadata/admission obligations without an execution receipt or BIG-IP observation. A selected family or scheduling descriptor does not create a provider, original Native entry, physical current frame or completed body/handler.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim

Status: `not-tested`.

No observation for this question.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-registry/src/invocation_words.rs](../../../../rust/tcl-registry/src/invocation_words.rs), `InvocationDialect::execution_point`: Resolve selected actual-family point; foreign compatible C base supplies no C implementation.
- [rust/tcl-registry/src/invocation_words.rs](../../../../rust/tcl-registry/src/invocation_words.rs), `InvocationDialect::has_same_execution_policy`: Normalise point representation only and retain exact equality of every other invocation policy field.
- [rust/tcl-registry/src/invocation_words.rs](../../../../rust/tcl-registry/src/invocation_words.rs), `invocation_words::tests::execution_policy_comparison_normalises_only_the_selected_point` (linked): All five C profile records match equivalent explicit point in both directions, while independent policy and foreign/build negatives reject.

- [rust/tcl-dialect/src/profile.rs](../../../../rust/tcl-dialect/src/profile.rs), `DialectProfile::runtime_core_family`: Resolve actual core provider or hosted F5 trunk package family once; catalogue signature compatibility supplies no execution point.
- [rust/tcl-registry/src/invocation_words.rs](../../../../rust/tcl-registry/src/invocation_words.rs), `InvocationDialect::of_profile`: Retain the shared actual runtime core family independently of typed compatible Tcl signature release.
- [rust/tcl-registry/src/body_execution.rs](../../../../rust/tcl-registry/src/body_execution.rs), `BodyExecutionSpec::deferred_entry_frame`: Require the independently selected actual execution_point and family before returning a scheduling-frame descriptor; conflicting or unsupported points refuse without donating a physical entered frame.
- [rust/tcl-registry/src/invocation_words.rs](../../../../rust/tcl-registry/src/invocation_words.rs), `invocation_words::tests::hosted_fork_profiles_keep_their_core_family_without_c_native_recipes` (linked): Hosted F5 trunk/fork package profiles retain their actual core family independently of compatible Tcl8.4 signatures; missing actual execution point grants no C Native name/string recipe. C and Jim profile controls retain their own family. Software profile/admission assertions only; no appliance or executed outcome.

These source bindings establish no executed assertion result; exact software outcomes belong to the independently pinned Rust validation receipts.

## Evidence

- [rust/tcl-registry/src/invocation_words.rs](../../../../rust/tcl-registry/src/invocation_words.rs)
