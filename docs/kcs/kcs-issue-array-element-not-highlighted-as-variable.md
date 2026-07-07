# KCS: A by-reference variable-name argument is not highlighted as a variable

> **Audience:** Contributor
> **Type:** Issue

## Applies to

all-editors

## Symptom

A variable name passed to a command by reference is not highlighted as a
variable. This covers both directions:

- **Write targets** — `set arr(key) 1`, `incr count(hits)`,
  `append log(err) x`, `unset arr(key)`.
- **Read references** — `info exists arr(key)`, `array names arr`,
  `array get arr`.

The name word stays a plain string, even though the corresponding
substitution (`$arr(key)`) highlights as one whole-word variable (issue #813).
A read of a plain scalar (`info exists scalar`) is not highlighted either.

The same is true for an array element passed to a *user proc* or *stubbed
command* that takes a variable name by reference — `myset arr(key) 1` where
`myset` does `upvar $varName …`, or a `# tcl-lsp: stub … :var` command.

## Operational context

Whether an argument *is* a variable-name spot is not a text-shape question —
the [command registry](../design/compiler/command-registry.md) already answers
it. `insert_var_decl_overrides` walks the argument indices the registry marks
with the `ArgRole::VarWrite` role, so the target is known to be a variable
before the walker looks at its text. The only remaining question is token
**geometry**: can the word be painted as one whole-word `Variable` token, or
does it contain an inner substitution that must survive as its own sub-token?

The old gate answered that with `is_plain_var_name`, which rejects any word
containing `(`, so every array element — literal (`arr(key)`) or computed
(`arr($i)`) — was dropped to the default classifier and rendered as a string.
That over-rejected: a **literal** element (`arr(key)`) lexes as a single
[`TokenType::Esc`](../../rust/tcl-lexer/src/tokens.rs) word and is perfectly
safe to paint as one token.

## Decision rules / contracts

1. The retag does **not** re-derive "is this a variable" from the word's text —
   the declared role is the authority. It retags the word whenever the segmenter
   reports the word is a single unquoted `Esc` token
   (`SegmentedCommand::single_token_word`), matching how `$arr(key)` reads
   highlight. This covers scalars (`x`), literal array elements (`arr(key)`),
   and namespaced names (`::ns::arr(key)`) uniformly, with no array-syntax
   parsing. Decided in `insert_var_role_overrides`,
   [`rust/tcl-lsp-core/src/semantic_tokens.rs`](../../rust/tcl-lsp-core/src/semantic_tokens.rs).
2. Both **write** and **read** roles are retagged, by *direction*:
   - `ArgRole::VarWrite` (`set`, `incr`, `lappend`, `dict with`, …) → `Variable`
     **with** the `declaration` modifier (it declares / writes the variable).
   - `ArgRole::VarRead` (`info exists arr(key)`, `array names arr`,
     `array get arr`, `dict with $d`, …) → a plain `Variable` **without** the
     `declaration` modifier (it references an existing variable). A word that is
     a *value*, not a name (`dict get $d k` — `$d` is a value), carries no read
     role, so it is not retagged; only its `$d` substitution is a variable.
3. A word with an inner substitution (`arr($i)`, `$dynamic`) is multi-token, so
   `single_token_word` is `false` and the word is left to the default
   classifier — its inner `$var` sub-tokens survive.
3. The registry-agnostic list walkers (`global` / `variable` names, `upvar`
   locals, parameter lists, loop-variable lists) still use `is_plain_var_name`,
   because those pull names out of list positions where an element genuinely may
   not be a name.
4. The static registry role is not the *only* source of a variable-name spot.
   The same retag also unions two dynamic sources, both carried by the
   `VarNameArgRoles` index passed into the token walk:
   - **Stub roles** — a `# tcl-lsp: stub NAME {arg:var …}` (write) or `:var_read`
     (read) declaration. These are derived from the document source at
     token-collection time, so they apply on every path (local and workspace)
     with no analysis needed.
   - **Inferred user-proc roles** — a proc parameter the analyser inferred to
     alias a caller variable via `upvar $param` (writing it →
     `ProcArgTrait::VarWrite`, reading it → `ProcArgTrait::VarRead`).
     Available from the single-file analysis locally, and from the workspace-
     merged `project_proc_var_index` salsa query when a project is open, so a
     proc defined in another file resolves too. `ProcArgTrait::DynamicNameLocal`
     (the param's *value* names a callee-local variable) is excluded from both
     directions — a literal name there is not the caller's variable.
   The same single-token `Esc` geometry gate applies to every source, so a
   computed subscript stays multi-token and its inner `$var` survives.

## File-path anchors

- `rust/tcl-lsp-core/src/semantic_tokens.rs` — `insert_var_role_overrides`,
  `VarNameArgRoles`, `add_stub_var_roles`, `proc_var_write_indices` /
  `proc_var_read_indices`, `ArgOverride::VarDecl` / `VarRef`
- `rust/tcl-lsp-db/src/lib.rs` — `project_proc_var_index` (workspace-merged
  inferred roles), `semantic_tokens` / `semantic_tokens_project` wiring
- `rust/tcl-compiler/src/analyser/param_traits.rs` — proc parameter trait
  inference (`upvar` → `VarWrite` / `VarRead`)
- `rust/tcl-registry/src/stub_overlay.rs` — `# tcl-lsp: stub … :var` / `:var_read`
  roles
- `rust/tcl-compiler/src/segmenter.rs` — `SegmentedCommand::single_token_word`
  (per-word single-token geometry, whose representative `argv` token spans the
  whole word)
- `rust/tcl-lexer/src/lexer.rs` — `$arr(idx)` variable lexing

## Failure modes

- Retagging a **computed** subscript (`arr($i)`) as one token would swallow the
  inner `$i`, dropping its variable sub-token — the `single_token_word` gate
  excludes it because such a word is multi-token.

## Triage checklist

1. Decode the semantic tokens for `set arr(key) 1` and confirm the `arr(key)`
   word is one `Variable` token with the `declaration` modifier.
2. Confirm `set arr($i) 1` still emits the inner `$i` as a variable sub-token.
3. Confirm the tokens do not overlap.

## Test anchors

- `rust/tcl-lsp-core/src/semantic_tokens.rs` — `literal_array_element_write_is_variable_declaration`,
  `namespaced_array_element_write_is_variable_declaration`,
  `unset_array_element_is_variable`, `array_element_write_not_retagged`,
  `varread_role_highlights_variable_name`,
  `varread_value_argument_is_not_a_variable`,
  `stub_var_arg_highlights_array_element`,
  `stub_var_read_arg_highlights_as_reference`,
  `user_proc_var_name_arg_highlights_array_element`,
  `user_proc_var_read_arg_highlights_as_reference`,
  `user_proc_var_name_computed_subscript_survives`

## Related

- [KCS index](README.md)
- [Semantic Tokens feature](features/kcs-feature-semantic-tokens.md)
- [highlight drops closing delimiter](kcs-issue-highlight-drops-closing-delimiter.md)
