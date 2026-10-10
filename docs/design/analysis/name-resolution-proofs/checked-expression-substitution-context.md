# naming.grammar.checked-expression-substitution-context

Kind: `implementation-contract`

## Problem statement

A retained expression may use Jim variable-name grammar or Tcl 9 comments while its native expression operator syntax comes from another selected context. Reconstructing the lexer from a baseline profile shortens Unicode roots or treats inert comment brackets as commands. Array indices also need the original scanner boundaries, including empty indices; an equal foreign byte buffer supplies no original geometry.

## Question

Does the checked expression substitution bridge retain the complete supplied grammar and original lexical boundaries without claiming lazy-branch execution or reads inside a command script?

## Conclusion

The bridge parses under ExprParseContext, retains its full lexer grammar, and uses scan_var_ref for index boundaries. Rejected syntax returns None. It inventories both lazy branches and stops at each command-script boundary; the caller must separately prove reads, conversions, handler selection and completion.

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

- [rust/tcl-syntax/src/expr/substitution.rs](../../../../rust/tcl-syntax/src/expr/substitution.rs), `checked_expression_substitutions`: Own checked original lexical substitution topology under the retained context.
- [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs), `index_range_in`: Project only an actual scanner-owned index into its supplied original byte image.
- [rust/tcl-syntax/src/expr/substitution.rs](../../../../rust/tcl-syntax/src/expr/substitution.rs), `expr::substitution::tests::checked_substitutions_retain_overridden_variable_and_comment_grammar` (linked): Retain overridden Jim root/index and Tcl 9 comment grammar; reject mismatching baseline and malformed syntax.
- [rust/tcl-syntax/src/expr/substitution.rs](../../../../rust/tcl-syntax/src/expr/substitution.rs), `expr::substitution::tests::checked_substitutions_keep_command_script_ownership_separate` (linked): Inventory index reads and outer command spans while excluding variables inside the returned scripts.
- [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs), `word_parts::tests::raw_index_geometry_keeps_empty_and_foreign_source_distinct` (linked): Preserve the exact empty index extent and reject equal bytes from a foreign allocation.

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
