# naming.diagnostic.original-command-suggestion-source

Kind: `implementation-contract`

## Problem statement

A nearest command spelling is a heuristic candidate. A profiled catalogue or reconstructed String word can widen the actual source context and produce a replacement at the wrong written extent.

## Question

How does command suggestion advice use actual-context candidates and the authenticated original whole source word without proving the suggested command is present?

## Conclusion

build_w123_known_names filters catalogue candidates under the actual complete context. original_command_suggestion_fix authenticates the emitting source subject and its original whole word, then renders one faithful ASCII source word using the independently selected complete source grammar. Unsupported value domains withdraw a proposal. The replacement requires review; a candidate spelling establishes no current command cell, source target, handler or successful execution.

## Scope

One linked source control retains the complete braced misspelled word extent, renders a two-word candidate as exactly one valid source word under the selected grammar, withdraws unsupported Unicode proposals and distinguishes actual older-context admitted candidates from excluded newer commands. Heuristic ranking, current presence, runtime semantics and edit equivalence remain independent. Reporting profile labels cannot replace the actual ContextRegistry.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This source diagnostic applicability, original word geometry and typed transport question supplies no native provider execution observation.

## Consumer bindings

- [rust/tcl-compiler/src/analyser/diagnostics/unresolved.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/unresolved.rs), `Analyser::build_w123_known_names`: Filter heuristic candidates through actual context and require authenticated original whole-word rendering for reviewed proposals; no target presence or handler grant.
- [rust/tcl-compiler/src/analyser/diagnostics/unresolved.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/unresolved.rs), `original_command_suggestion_fix`: Filter heuristic candidates through actual context and require authenticated original whole-word rendering for reviewed proposals; no target presence or handler grant.
- [rust/tcl-compiler/src/analyser/diagnostics/unresolved.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/unresolved.rs), `analyser::diagnostics::unresolved::require_ordering_tests::reviewed_command_suggestions_keep_original_word_extent_and_full_context` (linked): Actual full-context candidate filtering and whole braced misspelled word span produce one faithful ASCII source word with review required; unsupported Unicode rendering withdraws and an excluded newer command stays outside the older context.
