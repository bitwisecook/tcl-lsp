# naming.variable.empty-array-root-lexical-reference

Kind: `native-observation`

## Problem statement

Removing an array variable root from a source reference produces $(k). C Tcl can interpret this as an element of an empty-named array, while Jim can select expression sugar. Treating both engines identically can reject a valid C edit or emit a different Jim program. These ASCII source controls distinguish lexical, braced, scalar and inert forms; they do not establish arbitrary name edit authority.

## Question

After set {(k)} VALUE, what do $(k), ${(k)}, ${} and inert {$(k)} return on the six selected engines?

## Conclusion

C Tcl 8.4.20 through 9.1.0 reads VALUE through $(k) and ${(k)}; ${} errors because the empty root is an array. Jim 0.84-9-g5bac7c9 treats $(k) as expression syntax and errors, reads VALUE through ${(k)}, and reads its empty-root dictionary as k VALUE through ${}. The inert form returns literal $(k) on all six. This is the tested ASCII lexical/value distinction, with no command existence, edit, Native header or compiler admission grant.

The marked Core geometry definition preserves an independent array index when the original qualified root is edited to empty.

## Scope

Five fixed caught ASCII source controls passed as exact file bytes to shell stdin; actual info patchlevel output, ELF/header/source-owner/Makefile hashes and process stdout/stderr retained. BIG-IP not tested. No raw zero, opaque non-UTF8, source channel conversion or original object representation claim.

This is a software source-edit geometry binding only. Original Native lexical reference rows remain unchanged; geometry does not establish alias/cell/observer currentness, successful application, evaluation or edit admission.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: ASCII file bytes supplied directly to native shell stdin. Version: tcl8.4 8.4.20; tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

EMPTY_LEXICAL=VALUE; EMPTY_BRACED=VALUE; EMPTY_SCALAR=error variable is array; EMPTY_LITERAL=$(k); NAMED_INDEX=ARRAY.

### jim

Status: `observed`. Channel: ASCII file bytes supplied directly to native shell stdin. Version: jim 0.84-9-g5bac7c9.

EMPTY_LEXICAL=error syntax expression (k); EMPTY_BRACED=VALUE; EMPTY_SCALAR=k VALUE; EMPTY_LITERAL=$(k); NAMED_INDEX=ARRAY.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Consumer bindings

- [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs), `decompose_spanned`: Retain the selected C array-reference or Jim expression component geometry without inferring a runtime value or array cell.
- [rust/tcl-lexer/src/word_parts.rs](../../../../rust/tcl-lexer/src/word_parts.rs), `word_parts::template_policy_tests::jim_expression_components_retain_parentheses_and_native_token_sites` (linked): The shared lexer retains Jim expression parentheses while C selects an array reference with an empty root; this is lexical geometry only, separate from observed native values.

- [rust/tcl-lsp-core/src/original_name_edit.rs](../../../../rust/tcl-lsp-core/src/original_name_edit.rs), `original_variable_root_name_edit`: Keep original variable-root source geometry and the independently selected index when constructing the lexical edit, including an empty root.
- [rust/tcl-lsp-core/src/original_name_edit.rs](../../../../rust/tcl-lsp-core/src/original_name_edit.rs), `original_name_edit::original_variable_root_edit_tests::original_c_empty_array_root_edit_preserves_the_independent_index` (linked): Original C lexical root edit changes only the qualified root to empty while retaining the exact independent array index and original span; edit geometry supplies no runtime permission.

No assertion outcome is attached to these source bindings; software outcomes retain their independently pinned command and image scope.

## Evidence

- [rust/tcl-lexer/tests/data/native_empty_array_root](../../../../rust/tcl-lexer/tests/data/native_empty_array_root)
- [rust/tcl-lexer/tests/data/native_empty_array_root/8.4.20](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/8.4.20)
- [rust/tcl-lexer/tests/data/native_empty_array_root/8.5.19](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/8.5.19)
- [rust/tcl-lexer/tests/data/native_empty_array_root/8.6.18](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/8.6.18)
- [rust/tcl-lexer/tests/data/native_empty_array_root/9.0.4](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/9.0.4)
- [rust/tcl-lexer/tests/data/native_empty_array_root/9.1.0](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/9.1.0)
- [rust/tcl-lexer/tests/data/native_empty_array_root/jim](../../../../rust/tcl-lexer/tests/data/native_empty_array_root/jim)
