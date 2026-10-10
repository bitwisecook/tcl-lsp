# Does the shared descriptor lookup preserve an already selected byte operand while applying exact, strict, ambiguous-prefix and dialect availability rules?

Proof ID: `naming.selector.selected-byte-descriptor-lookup`

## Problem statement

A native selector may contain a counted zero, invalid UTF-8 or surrogate units. Repairing it with Unicode replacement before querying authored descriptor names can select a different entry. Input extent and dialect availability must stay independent from descriptor lookup.

## Question

Does the shared descriptor lookup preserve an already selected byte operand while applying exact, strict, ambiguous-prefix and dialect availability rules?

## Scope

Rust shared owner and consumers; no native experiment establishes this Rust contract. C Index/CString and native ensemble hash/prefix extents are independently selected purposes.

## Measured answers

| Provider | Status | Answer |
|---|---|---|
| tcl8.4 not recorded | not-tested | No observation for this precise question. |
| tcl8.5 not recorded | not-tested | No observation for this precise question. |
| tcl8.6 not recorded | not-tested | No observation for this precise question. |
| tcl9.0 not recorded | not-tested | No observation for this precise question. |
| tcl9.1 not recorded | not-tested | No observation for this precise question. |
| jim not recorded | not-tested | No observation for this precise question. |
| bigip not recorded | not-tested | No observation for this precise question. |

## Conclusion

KeywordTable::resolve_bytes and CommandSpec byte facades share the descriptor prefix/availability owners with String callers, while preserving selected bytes. Neither facade clips NUL, repairs Unicode, authorizes object access, or issues a native handler. VM/runtime selectors use their independent name/materialization entry before this lookup.

## Evidence


## Source anchors

No interpreter-source anchor is asserted for this question; the retained probe and outcomes establish the narrow observation.

## Shared owners and tests

- `rust/tcl-registry/src/abbrev.rs`: `KeywordTable::resolve_bytes` — Selected byte table lookup.
- `rust/tcl-registry/src/spec.rs`: `CommandSpec::resolve_subcommand_bytes_for_dialect` — Dialect-scoped selected byte lookup.
- `rust/tcl-registry/src/abbrev.rs`: `abbrev::tests::byte_keywords_preserve_selected_extent_and_do_not_repair_unicode`. OpaqueFF differs from replacement-character descriptor; NUL/surrogate inputs retain their extent; exact/prefix/strict outcomes are distinct.
- `rust/tcl-registry/src/spec.rs`: `spec::tests::byte_subcommand_resolution_retains_dialect_and_operand_boundaries`. The same byte prefix is unique under8.6 and ambiguous under9.1; opaque suffixes do not select an authored descriptor.

Native output is evidence for the interpreter operation. Rust tests must independently pass to establish implementation correspondence.

## Reconfirmation

```text
cargo test -p tcl-registry spec::tests::byte_subcommand_resolution_retains_dialect_and_operand_boundaries -- --exact
```

Use the fully qualified selector when executing --exact. Native outcomes are not claimed; these tests check the Rust descriptor/consumer contract separately.
