# Original branch expression source grammar

Proof ID: `naming.optimiser.original-branch-expression-source-grammar`. Kind: `implementation-contract`.

## Problem statement

A branch-equivalence query can retain a source lexer policy whose variable boundaries differ from its independently selected numerical and expression policy. Parsing the original condition with a catalogue grammar can change its source AST or import unsupported numeral syntax, while a parseable tree alone does not license replacing runtime operands.

## Question

How does original branch-equivalence parsing retain the actual complete source lexer grammar independently of numeric/native expression policy and subsequent executable rewrite obligations?

## Conclusion

branch_original_expression uses shared ExprParseContext and the actual PassContext lexer configuration for original source topology. The retained lexer overlay controls variable closing and name boundaries independently of the selected numeral/operator policy; neither axis donates the other. Explicit standalone missing-profile parsing preserves its runtime numeral boundary. branch_execution_equivalent passes this original tree to the existing contextual rewrite-equivalence owner, retaining its separate source/operand/numeric/preparation requirements. Unchanged closed literals and refused variable substitutions remain separate software inputs; source AST topology supplies no Native preparation or editing licence.

## Scope

Two marked source/API controls compare retained lexical/numeric axes and independent rewrite permission. Mixed C8/C9 profile/config fixtures test software policy transport, not public C Tcl/Jim completion, original operand objects, source preparation, installed header/cache, compiler instruction or entered execution. All seven external providers are not tested, and no assertion execution is claimed.

## Provider answers

### tcl8.4

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.4.

No external provider execution answers this original branch source-parser contract. Selected software lexical/numeric fixtures supply no Native result, object, source preparation, handler, frame or executable rewrite equivalence.

### tcl8.5

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.5.

No external provider execution answers this original branch source-parser contract. Selected software lexical/numeric fixtures supply no Native result, object, source preparation, handler, frame or executable rewrite equivalence.

### tcl8.6

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl8.6.

No external provider execution answers this original branch source-parser contract. Selected software lexical/numeric fixtures supply no Native result, object, source preparation, handler, frame or executable rewrite equivalence.

### tcl9.0

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.0.

No external provider execution answers this original branch source-parser contract. Selected software lexical/numeric fixtures supply no Native result, object, source preparation, handler, frame or executable rewrite equivalence.

### tcl9.1

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: tcl9.1.

No external provider execution answers this original branch source-parser contract. Selected software lexical/numeric fixtures supply no Native result, object, source preparation, handler, frame or executable rewrite equivalence.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

No external provider execution answers this original branch source-parser contract. Selected software lexical/numeric fixtures supply no Native result, object, source preparation, handler, frame or executable rewrite equivalence.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: f5-bigip.

No external provider execution answers this original branch source-parser contract. Selected software lexical/numeric fixtures supply no Native result, object, source preparation, handler, frame or executable rewrite equivalence.

## Exact evidence

- `naming-optimiser-original-branch-expression-source-grammar-definition` (implementation): [rust/tcl-compiler/src/optimiser/branch_folding.rs](../../../../rust/tcl-compiler/src/optimiser/branch_folding.rs). SHA-256 `caec7d03cd7aa76b4432ae8bfbb8203631bf024fbd7c396421623909fa1a2c41`. Current source parser and authored software control definitions; no executed assertion or external provider result.

## Source inspection

The listed Rust source defines the current original source parser and its conditional equivalence query. No upstream Tcl excerpt or external Native experiment observes this software branch projection.

## Consumer bindings

- [rust/tcl-compiler/src/optimiser/mod.rs](../../../../rust/tcl-compiler/src/optimiser/mod.rs), `PassContext::lexer_config`: Retain the source lexical policy for original expression topology independently of the selected numerical/runtime expression policy.
- [rust/tcl-compiler/src/optimiser/branch_folding.rs](../../../../rust/tcl-compiler/src/optimiser/branch_folding.rs), `branch_original_expression`: Build shared ExprParseContext from the selected policy and overlay only the retained source lexer grammar before shared source parsing. Explicit standalone missing-profile parsing preserves its separate runtime numeral boundary.
- [rust/tcl-compiler/src/optimiser/branch_folding.rs](../../../../rust/tcl-compiler/src/optimiser/branch_folding.rs), `branch_execution_equivalent`: Pass the original source topology to the existing contextual expression rewrite-equivalence owner, preserving original operand, numeric, preparation and policy requirements.
- [rust/tcl-compiler/src/optimiser/mod.rs](../../../../rust/tcl-compiler/src/optimiser/mod.rs), `PassContext::expression_rewrite_equivalence_at`: Retain the independent actual expression rewrite prerequisites; parseable source topology alone cannot issue operand preparation, Native entry or executable edit permission.
- [rust/tcl-syntax/src/expr/checked.rs](../../../../rust/tcl-syntax/src/expr/checked.rs), `ExprParseContext`: Keep actual variable lexical grammar, operator/host and numeral policies as separate parser axes; the parser reexport or descriptive profile cannot replace retained source grammar.
- [rust/tcl-compiler/src/optimiser/branch_folding.rs](../../../../rust/tcl-compiler/src/optimiser/branch_folding.rs), `optimiser::branch_folding::tests::original_branch_source_parser_keeps_retained_grammar_and_numeric_axes` (linked): Original branch source AST topology follows the retained C8/C9 variable closing policy and literal Unicode/parenthesis names independently of the actual selected numeric policy. A lexer overlay cannot donate new numeral forms, and a numeric policy cannot donate different source-name boundaries.
- [rust/tcl-compiler/src/optimiser/branch_folding.rs](../../../../rust/tcl-compiler/src/optimiser/branch_folding.rs), `optimiser::branch_folding::tests::original_branch_source_topology_keeps_equivalence_permission_separate` (linked): Closed unchanged literal expressions retain the independently selected rewrite-equivalence premise; an original scalar-name AST without the required operand proof cannot justify replacing it with a constant. Missing policy remains refused despite parseable topology; no Native process or edit equivalence is measured.

A named test is a coverage binding, not a claim that it executed.

## Replay

The named controls bind current source/API definitions without execution claims. Actual Native observations and independent Rust compilation/test receipts retain their own scopes.
