# naming.source.original-command-token-memo

Kind: `implementation-contract`

## Problem statement

Repeated readonly analyser queries prepare and stamp the same original command vector under one immutable source interpretation. Reusing an unrelated source, configuration or realm would substitute its provenance.

## Question

How can repeated analyser queries share original lexical preparation without reusing a foreign source or interpretation?

## Conclusion

The immutable command realm owns a bounded memo of unchanged lexical segmentation, word construction and original source stamping. Complete image/channel, full lexer configuration, invocation offset and representative tokens identify the query. Clones of the same realm share immutable results; a new realm starts with an independent cache. A zero-written-argument call obtains its original whole head extent from the retained source record only after complete source-image/config/site correspondence and original argv count one; captured alias prefixes do not become written argument geometry.

## Scope

Rust preparation and retention only. At most 256 entries are retained, including missing results. Beyond the bound queries use unchanged ordinary preparation. Derived memo state is excluded from semantic equality; no Input, lookup, frame, Normal or compiler purpose is granted by a hit. Existing depth and default test-stack remain unchanged. Timing and successful depth execution require independent actual validation.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation is attached to this Rust preparation invariant.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation is attached to this Rust preparation invariant.

### tcl8.6

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation is attached to this Rust preparation invariant.

### tcl9.0

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation is attached to this Rust preparation invariant.

### tcl9.1

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Tcl.

No interpreter observation is attached to this Rust preparation invariant.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No interpreter observation is attached to this Rust preparation invariant.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No interpreter observation is attached to this Rust preparation invariant.

## Exact evidence

No evidence artifact is attached.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-compiler/src/realm/original_tokens.rs](../../../../rust/tcl-compiler/src/realm/original_tokens.rs), `OriginalCommandTokenCache::get_or_prepare`: Share immutable complete results only under exact keys, retaining no lock during original lookup and bounding retained entries.
- [rust/tcl-compiler/src/realm/original_tokens.rs](../../../../rust/tcl-compiler/src/realm/original_tokens.rs), `CommandBindingRealm::retained_original_tokens`: Retain unchanged lexical preparation and original source stamping under the same immutable owner.
- [rust/tcl-compiler/src/analyser/commands.rs](../../../../rust/tcl-compiler/src/analyser/commands.rs), `Analyser::retained_invocation_tokens`: Consume the immutable retained carrier for repeated source-role, body, reference and diagnostic queries.
- [rust/tcl-compiler/src/realm/original_tokens.rs](../../../../rust/tcl-compiler/src/realm/original_tokens.rs), `realm::original_tokens::tests::original_token_memo_shares_only_the_same_immutable_query` (linked): The same complete query and immutable realm clone return the same retained Arc; the genuine namespace carrier retains nested bindings and cache state does not affect realm equality.
- [rust/tcl-compiler/src/realm/original_tokens.rs](../../../../rust/tcl-compiler/src/realm/original_tokens.rs), `realm::original_tokens::tests::original_token_memo_retains_full_source_config_and_representatives` (linked): Same spans with changed complete content, full lexer configuration, representative token or source channel do not reuse the first carrier.
- [rust/tcl-compiler/src/realm/original_tokens.rs](../../../../rust/tcl-compiler/src/realm/original_tokens.rs), `realm::original_tokens::tests::original_token_memo_does_not_reuse_a_foreign_or_unknown_realm` (linked): An independent realm and an actual unknown-entry realm do not reuse the original carrier; unknown-entry binding withdrawal stays present.
- [rust/tcl-compiler/src/realm/original_tokens.rs](../../../../rust/tcl-compiler/src/realm/original_tokens.rs), `realm::original_tokens::tests::original_token_memo_has_bounded_retention` (linked): 257 complete source queries with their actual token vectors retain at most 256 entries while uncached preparation still returns its ordinary result.
- [rust/tcl-compiler/src/realm/original_tokens.rs](../../../../rust/tcl-compiler/src/realm/original_tokens.rs), `realm::original_tokens::tests::original_token_memo_keeps_zero_written_arguments_and_captured_prefix_separate` (linked): An alias with a captured unset option retains one original written head at a zero-argument call, shares only the exact immutable query, and declines stale complete source or a site whose original vector has arguments. This is source acquisition and does not execute unset.

A named test is a coverage binding, not a claim that it executed.

## Replay

Linked Rust tests are not an execution claim. The unchanged 80-level/default-stack selector is measured independently, with timeout reported distinctly from a passing result.
