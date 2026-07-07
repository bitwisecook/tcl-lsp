# KCS: An array-element write target is not highlighted as a variable

> **Audience:** Contributor
> **Type:** Issue

## Applies to

all-editors

## Symptom

An array element used as a write target — `set arr(key) 1`,
`incr count(hits)`, `append log(err) x`, or `unset arr(key)` — is not
highlighted as a variable. The `arr(key)` word stays a plain string, even
though the corresponding read (`$arr(key)`) highlights as one whole-word
variable (issue #813).

## Operational context

Semantic tokens retag a command's write-target argument (registry role
`ArgRole::VarWrite`) as a `Variable` declaration. The retag only fires for a
word the walker can safely paint as one token. The gate was
`is_plain_var_name`, which rejects any word containing `(`, so every array
element — literal (`arr(key)`) or computed (`arr($i)`) — was left to the
default classifier and rendered as a string.

Excluding a **computed** subscript is deliberate: `arr($i)` lexes into several
tokens (`arr(`, `$i`, `)`), and the inner `$i` must survive as its own
variable sub-token. But a **literal** element (`arr(key)`) lexes as a single
[`TokenType::Esc`](../../rust/tcl-lexer/src/tokens.rs) word, so it can be
retagged as one variable token with no risk of clobbering an inner
substitution.

## Decision rules / contracts

1. A literal array element (`name(index)` with no `$` / `[` substitution and no
   nested delimiter in either part) is retagged as a whole-word `Variable`
   declaration, matching how `$arr(key)` reads highlight. This is decided by
   `is_literal_array_element` in
   [`rust/tcl-lsp-core/src/semantic_tokens.rs`](../../rust/tcl-lsp-core/src/semantic_tokens.rs).
2. A computed subscript (`arr($i)`) is **not** retagged — it is left to the
   default classifier so its inner `$var` sub-tokens survive.
3. Only the `VarWrite`-role retag (`insert_var_decl_overrides`) accepts array
   elements. Declaration commands whose names must be scalars or whole arrays
   (`global`, `variable`, `upvar` locals) keep using `is_plain_var_name`, since
   an element is not a legal name there.

## File-path anchors

- `rust/tcl-lsp-core/src/semantic_tokens.rs` — `is_literal_array_element`,
  `insert_var_decl_overrides`
- `rust/tcl-lexer/src/lexer.rs` — `$arr(idx)` variable lexing

## Failure modes

- Retagging a **computed** subscript (`arr($i)`) as one token would swallow the
  inner `$i`, dropping its variable sub-token — the reason the gate excludes it.
- A subscript containing a nested `(` / `)` / brace could produce an overlapping
  span if painted as one token; `is_literal_array_element` rejects those.

## Triage checklist

1. Decode the semantic tokens for `set arr(key) 1` and confirm the `arr(key)`
   word is one `Variable` token with the `declaration` modifier.
2. Confirm `set arr($i) 1` still emits the inner `$i` as a variable sub-token.
3. Confirm the tokens do not overlap.

## Test anchors

- `rust/tcl-lsp-core/src/semantic_tokens.rs` — `literal_array_element_write_is_variable_declaration`,
  `namespaced_array_element_write_is_variable_declaration`,
  `unset_array_element_is_variable`, `is_literal_array_element_classifies`,
  `array_element_write_not_retagged`

## Related

- [KCS index](README.md)
- [Semantic Tokens feature](features/kcs-feature-semantic-tokens.md)
- [highlight drops closing delimiter](kcs-issue-highlight-drops-closing-delimiter.md)
