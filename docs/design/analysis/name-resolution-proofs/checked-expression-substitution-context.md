# naming.grammar.checked-expression-substitution-context

Kind: `implementation-contract`

## Problem statement

A retained expression may use Jim variable-name grammar or Tcl 9 comments while its native expression operator syntax comes from another selected context. Reconstructing the lexer from a baseline profile shortens Unicode roots or treats inert comment brackets as commands. Array indices also need the original scanner boundaries, including empty indices; an equal foreign byte buffer supplies no original geometry.

## Question

Does the checked expression substitution bridge retain the complete supplied grammar and original lexical boundaries without claiming lazy-branch execution or reads inside a command script?

## Conclusion

Both substitution bridges retain the complete ExprParseContext lexer grammar and use scan_var_ref for original index boundaries. Native checked_expression_substitutions requires its separate NativeSyntax parse; NativeExprSyntax::Unknown remains Unsupported(UnknownNativeSyntax). The source-only checked_source_expression_substitutions uses the shared checked lexer/Pratt worker with SourceTopology purpose and returns Option topology without Native syntax certification, preparation actions, diagnostics, result representation or entry authority. Complete lexing, selected operators, depth limits and whole-token consumption remain required; rejected or unsupported source topology returns None. Both lazy branches are inventoried and command-script boundaries retain independent ownership. The caller must separately prove reads, conversions, handler selection and completion.

## Scope

Rust implementation contract under the explicit contexts used by the linked tests. The two linked original_source_topology_tests cover complete BOM/Unicode bracket spans and both lazy branches under Unknown Native syntax, malformed lexical/index delimiters, trailing/unconsumed syntax, selected operator availability and selected comment grammar. They directly assert unchanged Native Unknown refusal. They contain no positive scalar/array/brace-root matrix or SourceTopology depth-limit assertion. The source-only Option topology supplies no preparation, Native object identity, result representation or entry authority. All linked definitions remain source coverage obligations without a current executed assertion result here. This record makes no C Tcl, Jim or BIG-IP observation claim and no successful evaluation claim.

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

- `current-original-source-purpose-c44258ad16e1b6be` (implementation): [rust/tcl-syntax/src/expr/checked.rs](../../../../rust/tcl-syntax/src/expr/checked.rs). SHA-256 `c15fc517688dbabac8152db03191ab52131736a6ebfcc6fb72a8c16a621a7be7`. Current original physical cleanup, selected source topology or Logical source operand-layout owner and its finite software controls. These source definitions supply no new Native/provider observation, runtime authority, object-reference equivalence or executed compiler/body assertion.
- `current-original-source-purpose-aea7fd4e8800e02f` (implementation): [rust/tcl-syntax/src/expr/parser.rs](../../../../rust/tcl-syntax/src/expr/parser.rs). SHA-256 `236c0d96091ecf8b6b1b935730befff90cd69774d933eb68c7101c3660788b40`. Current original physical cleanup, selected source topology or Logical source operand-layout owner and its finite software controls. These source definitions supply no new Native/provider observation, runtime authority, object-reference equivalence or executed compiler/body assertion.
- `current-original-source-purpose-2760f3974abf9410` (implementation): [rust/tcl-syntax/src/expr/substitution.rs](../../../../rust/tcl-syntax/src/expr/substitution.rs). SHA-256 `6d41034ca56825004b7f6ef806c4f0fe17acc62854d1567e213c78cc0b904a53`. Current original physical cleanup, selected source topology or Logical source operand-layout owner and its finite software controls. These source definitions supply no new Native/provider observation, runtime authority, object-reference equivalence or executed compiler/body assertion.
- `current-original-source-purpose-44d7eae1482b2716` (implementation): [rust/tcl-syntax/src/expr/mod.rs](../../../../rust/tcl-syntax/src/expr/mod.rs). SHA-256 `ea2e6c3fcdd3f993e71646ec424afd505e06ba36f0e709ead8a188af764ddaaa`. Current original physical cleanup, selected source topology or Logical source operand-layout owner and its finite software controls. These source definitions supply no new Native/provider observation, runtime authority, object-reference equivalence or executed compiler/body assertion.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

- [rust/tcl-syntax/src/expr/substitution.rs](../../../../rust/tcl-syntax/src/expr/substitution.rs), `checked_expression_substitutions`: Own checked original lexical substitution topology under the retained context.
- [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs), `index_range_in`: Project only an actual scanner-owned index into its supplied original byte image.
- [rust/tcl-syntax/src/expr/checked.rs](../../../../rust/tcl-syntax/src/expr/checked.rs), `parse_source_expression_checked`: Reuse the same lexer/Pratt/operator/complete-consumption worker for original source topology without selecting Native syntax or producing a runtime/preparation/result carrier.
- [rust/tcl-syntax/src/expr/substitution.rs](../../../../rust/tcl-syntax/src/expr/substitution.rs), `checked_source_expression_substitutions`: Expose only original source substitution extents under the retained grammar; Native checked syntax remains independently gated.
- [rust/tcl-syntax/src/expr/substitution.rs](../../../../rust/tcl-syntax/src/expr/substitution.rs), `expr::substitution::tests::checked_substitutions_retain_overridden_variable_and_comment_grammar` (linked): Retain overridden Jim root/index and Tcl 9 comment grammar; reject mismatching baseline and malformed syntax.
- [rust/tcl-syntax/src/expr/substitution.rs](../../../../rust/tcl-syntax/src/expr/substitution.rs), `expr::substitution::tests::checked_substitutions_keep_command_script_ownership_separate` (linked): Inventory index reads and outer command spans while excluding variables inside the returned scripts.
- [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs), `word_parts::tests::raw_index_geometry_keeps_empty_and_foreign_source_distinct` (linked): Preserve the exact empty index extent and reject equal bytes from a foreign allocation.
- [rust/tcl-syntax/src/expr/substitution.rs](../../../../rust/tcl-syntax/src/expr/substitution.rs), `expr::substitution::original_expression_span_tests::inclusive_expression_extent_keeps_complete_multibyte_source_and_checked_boundaries` (linked): Finite source-defined control: inclusive expression extent keeps complete multibyte source and checked boundaries. Checked inclusive expression extent -> half-open UTF8 geometry; actual nonzero multibyte matcher retains original guards, invalid boundary/overflow refuses. This body has no attached matching current owned assertion result. Its source/API coverage stays within this question; existing external answers, original captures and independent Native/runtime authority are unchanged.
- [rust/tcl-syntax/src/expr/substitution.rs](../../../../rust/tcl-syntax/src/expr/substitution.rs), `expr::substitution::original_source_topology_tests::original_source_topology_keeps_complete_unicode_bom_ranges_without_native_syntax` (linked): Under an actual Unknown Native syntax context, the source-only bridge retains complete original BOM/Unicode bracket spans in both lazy branches with no variable reads in this fixture. The Native bridge declines and the checked Native parser explicitly remains Unsupported(UnknownNativeSyntax); the context stays Unknown. This control supplies no positive scalar/array/brace-root matrix, preparation or Native object/entry authority. Source definition only; no current executed assertion or provider outcome is attached.
- [rust/tcl-syntax/src/expr/substitution.rs](../../../../rust/tcl-syntax/src/expr/substitution.rs), `expr::substitution::original_source_topology_tests::original_source_topology_refuses_incomplete_lexing_unsupported_operators_and_unconsumed_suffix` (linked): The source-only bridge declines incomplete expression, bracket, quote and array-index syntax, unsupported or unconsumed suffixes, and a selected unavailable operator. Selected Tcl 8/Tcl 9 comment grammar independently refuses or inventories the complete outer command span. The final known Native context keeps unavailable-operator refusal in both bridges. This control does not assert a SourceTopology depth limit or Native execution. Source definition only; no current executed assertion or provider outcome is attached.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "cargo",
  "test",
  "--manifest-path",
  "rust/Cargo.toml",
  "expr::substitution::tests::checked_substitutions_retain_overridden_variable_and_comment_grammar"
]
```

Run each listed selector in its owning crate; the argument vector shows the first selector. A coverage binding is not a recorded test execution. Native interpreter measurements are separate records.
