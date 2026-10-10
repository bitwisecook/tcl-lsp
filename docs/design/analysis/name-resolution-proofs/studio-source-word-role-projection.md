# naming.studio.source-word-role-projection

Kind: `implementation-contract`

## Problem statement

Decoded display text cannot preserve whether a Studio sample selector is literal, computed or expanded, or whether its command head is actually selected. Role and option explanations require the original source topology and retained Registry ownership before the sample can expose script-body or option advice.

## Question

Does Studio query roles and selected option occurrences from genuine structured source words and preserve uncertainty for computed selectors and expansion?

## Conclusion

Studio retains CommandTokens and independent effective value facets, queries structured Registry role and option APIs, and obtains braced body closers from the shared range owner. Computed heads and uncertain selectors or expansion remain unknown. Literal escapes and fixed-width dynamic option values keep their selected metadata; this view issues no runtime execution grant.

## Scope

Rust implementation contract under the explicit contexts used by the linked tests. This record makes no C Tcl, Jim or BIG-IP observation claim, no Native entry claim and no successful evaluation claim.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

This is a Rust implementation invariant; no native-language observation for this question is claimed.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-registry/src/resolved_invocation.rs](../../../../rust/tcl-registry/src/resolved_invocation.rs), `prefix_occurrences`: Own selected prefix option/value projection using the same boundary and option resolver.
- [rust/tcl-spec-studio/src/sample.rs](../../../../rust/tcl-spec-studio/src/sample.rs), `provenance`: Explain structured Registry selection without a local string option scanner.
- [rust/tcl-spec-studio/src/sample.rs](../../../../rust/tcl-spec-studio/src/sample.rs), `sample::tests::computed_selectors_and_expansion_keep_source_roles_unknown` (linked): Computed selector, expanded definition and computed head do not gain role or recursive-body authority.
- [rust/tcl-spec-studio/src/sample.rs](../../../../rust/tcl-spec-studio/src/sample.rs), `sample::tests::literal_escapes_and_dynamic_value_words_share_the_registry_owner` (linked): Escaped literal selectors and dynamic non-selector words use the Registry owner while rendering original source.
- [rust/tcl-registry/src/resolved_invocation.rs](../../../../rust/tcl-registry/src/resolved_invocation.rs), `resolved_invocation::tests::structured_option_boundary_shares_alias_abbreviation_and_value_span_rules` (linked): Selected option occurrence ranges preserve aliases, abbreviation, fixed value widths and terminator boundaries; computed selectors decline.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "--manifest-path",
  "rust/Cargo.toml",
  "sample::tests::computed_selectors_and_expansion_keep_source_roles_unknown"
]
```

Run each listed selector in its owning crate; the argument vector shows the first selector. A coverage binding is not a recorded test execution. Native interpreter measurements are separate records.
