# naming.diagnostics.original-w216-literal-source-replacement

Kind: `implementation-contract`

## Problem statement

A W216 source proposal for array indexing must preserve literal root dollar/quote/bracket/backslash/Unicode/newline characters and original substituting index components. An inserted unwritten ::set is an independent selected-head premise rather than existing command proof.

## Question

When can W216 render a source proposal that preserves the exact literal root and original index substitution under retained lexical grammar, and why does the fallback ::set still require independent review?

## Conclusion

The shared literal_quoted_source_fragment quotes literal root characters and requires selected escape decoding to reproduce the exact source string. substituting_quoted_source_fragment preserves original variable/command/expression components and literal escape spelling, escaping only required quote characters, then requires shared parser component equivalence under complete LexerConfig. build_w216_replacement validates the whole original index syntax; empty/parenthesised unsupported root or malformed/incompletely retained substitution refuses a proposal. A bare-name proposal retains its genuine selected syntax; the quoted fallback is [::set "root(index)"], whose unwritten stock ::set meaning remains an explicit RequiresReview premise. Source component equality does not grant runtime current cell/key, inserted-head availability, safe-edit authority or Native execution. Fixed public stock-set values and known replacement outcomes are independent observed examples.

## Scope

Two marked fixed source/API controls cover actual retained C/Jim grammar, literal dollars/quotes/brackets/backslashes/Unicode/split lines, quoted/nested-command index components and exact malformed refusal. One named Syntax helper control passed in the separate scoped receipt; the compiler diagnostic control has no new executed result. all seven providers are not tested for this renderer contract. The public eight-example value question does not establish all indices or editor applicability.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Consumer bindings

- [rust/tcl-syntax/src/backslash.rs](../../../../rust/tcl-syntax/src/backslash.rs), `literal_quoted_source_fragment`: Render an exact literal quoted source fragment only if selected escape grammar round-trips its characters.
- [rust/tcl-syntax/src/backslash.rs](../../../../rust/tcl-syntax/src/backslash.rs), `substituting_quoted_source_fragment`: Keep authentic original substitution components and verify shared-parser equivalence under complete retained config, without evaluating them.
- [rust/tcl-compiler/src/analyser/diagnostics/usage.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/usage.rs), `build_w216_replacement`: Validate authentic array-index source geometry and render the bounded proposal while preserving independent unwritten stock-set RequiresReview premise.
- [rust/tcl-syntax/src/backslash.rs](../../../../rust/tcl-syntax/src/backslash.rs), `backslash::tests::quoted_fragments_preserve_literal_characters_and_original_substitutions` (linked): Actual selected grammar round-trips literal root characters and preserves original substituting variable/command/expression components; malformed/incompletely retained components refuse.
- [rust/tcl-compiler/src/analyser/diagnostics/tests.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/tests.rs), `analyser::diagnostics::tests::w216_literal_root_replacement_preserves_bytes_and_index_source` (linked): Genuine fullsource diagnostics propose exact literal root/index spellings and retain explicit RequiresReview for unwritten ::set; selected bytes/components are source controls, not actual Native lookup/current cell or safe editor edit.

## Evidence

- [rust/tcl-compiler/src/analyser/diagnostics/tests.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/tests.rs)
- [rust/tcl-compiler/src/analyser/diagnostics/usage.rs](../../../../rust/tcl-compiler/src/analyser/diagnostics/usage.rs)
- [rust/tcl-syntax/src/backslash.rs](../../../../rust/tcl-syntax/src/backslash.rs)
