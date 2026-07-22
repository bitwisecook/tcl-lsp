# KCS: `apply`'s lambda body is not highlighted when reached through `[list ...]`

> **Audience:** Contributor
> **Type:** Issue

## Applies to

all-editors

## Symptom

Commands inside an `apply {argList body}` lambda render as one opaque,
unhighlighted string when `apply` is not the literal head of the call — most
commonly a pkgIndex.tcl-style entry:

```tcl
package ifneeded myPackage 1.0.0 [list apply {dir {
    source [file join $dir src font.tcl]
    source [file join $dir src utils.tcl]
}} $dir]
```

`package`, `ifneeded`, and `list` highlight normally; `source`, `file`,
`join`, and the `dir` parameter inside the lambda do not — the whole
`{dir {...}}` blob paints as a plain string. A *direct* `apply {dir {...}}
$val` call (no `[list ...]` wrapper) highlights correctly.

Reported as issue #954. A first fix (merged as `cb10e7c90`, shipped in
v2.1.11) corrected a narrower, related bug — a bare (unbraced) single-name
argument list (`apply {dir {...}}`, where `dir` has no braces of its own)
painting as a string instead of a `Parameter` — but the issue was reopened:
the reporter's actual repro was the `[list apply {...} $dir]` shape above,
which the first fix never touched.

## Operational context

`apply`'s first argument is a 2- or 3-element Tcl list — `{argList body
?namespace?}` — not a script directly. Highlighting it correctly requires
two separate things to work:

1. Recognising that the argument *is* this lambda-literal shape (as opposed
   to a plain `ArgRole::Body` script), so the highlighter splits it — element
   0 is a parameter list, element 1 is the body to recurse into as a script —
   rather than re-segmenting the whole `{argList} {body}` blob as one script.
   The pre-fix code did exactly that: it re-parsed `"dir {puts $dir}"` as a
   *script*, reading `dir` as a command name and `{puts $dir}` as `dir`'s one
   argument. Since `dir` never resolves to a registered command, recursion
   stopped there and the real body was never reached.
2. Recognising `apply` as the callee *at all* when it is not the literal head
   of the current statement. `[list apply {…} $dir]` is the idiomatic way to
   build a deferred command around a dynamic value — `list` quotes each
   argument so the result, evaluated later, invokes `apply` with exactly
   those words — precisely because a literal `apply {…} $dir` can't be
   written when `$dir`'s value must be captured at index-time rather than
   substituted textually. The same idiom shows up for any deferred callback
   (`button .b -command [list doSomething $x]`, `after idle [list apply
   {…} $x]`), not just pkgIndex.tcl.

Both are registry-driven, with **no command name compared anywhere in the
consumer**:

- [`ArgRole::LambdaLiteral`](../../rust/tcl-registry/src/arg_role.rs) is a
  role distinct from `ArgRole::Body` for exactly this shape. `apply`'s spec
  declares `arg_roles: &[(0, ArgRole::LambdaLiteral)]`
  ([`rust/tcl-registry/src/commands/tcl/apply.rs`](../../rust/tcl-registry/src/commands/tcl/apply.rs)).
  A generic `ArgRole::Body` walker (the semantic-token highlighter's default
  body recursion, folding, formatting, minification, declaration-scanning,
  the interprocedural call-graph scanner, the iRules object-reference
  walker) never touches a `LambdaLiteral`-tagged argument, so none of them
  mis-parses the parameter word as a command; each was given its own small
  `LambdaLiteral`-aware branch that splits the list first (via the shared
  [`tcl_compiler::lambda_literal::split_lambda_literal`](../../rust/tcl-compiler/src/lambda_literal.rs))
  and recurses only into the real body element.
- `Traits::BUILDS_COMMAND_PREFIX` is set on `list`'s own spec
  ([`rust/tcl-registry/src/commands/tcl/list_.rs`](../../rust/tcl-registry/src/commands/tcl/list_.rs)):
  "when this command's first argument is a literal command name, the result
  invokes that command with the remaining arguments appended verbatim." It is
  **not** set on `concat` — `concat`'s plain string-join doesn't give the same
  per-word quoting guarantee for a dynamic value, so recognising it the same
  way would be unsound. `insert_lambda_literal_overrides`
  ([`rust/tcl-lsp-core/src/semantic_tokens.rs`](../../rust/tcl-lsp-core/src/semantic_tokens.rs))
  checks this trait to decide whether a segmented command's own head (`list`)
  is quoting a further command, then resolves *that* command's own literal
  first argument against `ArgRole::LambdaLiteral` the same way the direct
  case does — recursively, so it generalises to any future command sharing
  the lambda-literal shape, not just `apply`.
- The same trait feeds
  [`extract_list_quoted_prefix_head`](../../rust/tcl-compiler/src/signature_scan/command_prefix.rs),
  a third shape alongside the existing bareword and braced-multi-word
  `ArgRole::CommandPrefix` recognisers — so `-command [list doSomething $x]`
  callbacks feed find-references / call-hierarchy / arity-checking / W123
  exactly like a literal `-command doSomething` would.

`package ifneeded`'s own `script` argument carried **no role at all** before
this fix — a *literal* `package ifneeded name ver {script}` (no `[list ...]`
wrapper) was equally unhighlighted. It is now `arg_roles: &[(2,
ArgRole::Body)]` with `body_kind: BodyKind::Structural` (the script runs
later, in the global namespace via `uplevel #0`, never the definer's frame —
mirrors `timer in`/`timer idle`'s precedent, no dedicated lowering hook
needed).

## Decision rules / contracts

1. A command whose argument is a `{paramList body ?ns?}` list (not a plain
   script) declares `ArgRole::LambdaLiteral`, never `ArgRole::Body` — tagging
   it `Body` makes generic Body-role walkers (SSA's caller-scope scan
   included) try to read the list as a script and misattribute or corrupt
   whatever they find.
2. The `[list head arg1 arg2 …]` command-quoting idiom is recognised via a
   registry trait (`Traits::BUILDS_COMMAND_PREFIX`) on `list` itself, checked
   generically wherever a Body/LambdaLiteral/CommandPrefix argument position
   needs to see through it — never by comparing a segmented command's head
   text to the literal string `"list"`.
3. Recognising the quoting idiom is bounded to positions the registry already
   marks as carrying a script/callback (`ArgRole::Body`,
   `ArgRole::LambdaLiteral`, `ArgRole::CommandPrefix`) — not universally for
   every bare `[list ...]` anywhere in the source. This matches the existing
   precision posture of the braced-multi-word `CommandPrefix` shape (also a
   heuristic, also scoped to declared role positions) and avoids false
   positives on `[list ...]` used purely to build ordinary data.
4. A shape recogniser that is safe to be more permissive as a *highlighting*
   heuristic (worst case: a cosmetic misclassification) is not automatically
   safe to reuse unchanged as an *analyser/diagnostic* fact (worst case: a
   spurious diagnostic on data that was never actually invoked as a command).
   The semantic-token layer, folding, formatting, minification, and
   declaration-scanning all recurse into a `LambdaLiteral` body
   unconditionally; the deeper SSA/CFG-based analyser (`register_body_unit`,
   `LoweringHookId::Apply`) stays scoped to the *direct* literal-call shape
   only — it does not follow the `[list ...]` indirection, by design.
5. This class of bug lives entirely in registry data plus one generic,
   reusable split primitive
   ([`split_lambda_literal`](../../rust/tcl-compiler/src/lambda_literal.rs)).
   Do not special-case `"apply"` (or any other command name) in a walker to
   work around a missing registry role.

## File-path anchors

- `rust/tcl-registry/src/arg_role.rs` — `ArgRole::LambdaLiteral`
- `rust/tcl-registry/src/commands/tcl/apply.rs` — `apply`'s spec
- `rust/tcl-registry/src/commands/tcl/list_.rs` — `Traits::BUILDS_COMMAND_PREFIX`
- `rust/tcl-registry/src/commands/tcl/package_.rs` — `ifneeded`'s `Body` +
  `BodyKind::Structural`
- `rust/tcl-registry/src/traits.rs` — the trait bit and its doc
- `rust/tcl-compiler/src/lambda_literal.rs` — `split_lambda_literal`, the
  shared list-element splitter
- `rust/tcl-lsp-core/src/semantic_tokens.rs` —
  `insert_lambda_literal_overrides`, `collect_lambda_literal`
- `rust/tcl-lsp-core/src/folding.rs`, `formatting/engine.rs`, `minify.rs`,
  `declaration.rs` — each command's own `LambdaLiteral`-aware branch
- `rust/tcl-compiler/src/signature_scan/command_prefix.rs` —
  `extract_list_quoted_prefix_head`
- `rust/tcl-compiler/src/interprocedural.rs`,
  `rust/tcl-compiler/src/analyser/param_traits.rs` — the same split applied
  to the (best-effort, text-based) call-graph / param-trait scanners
- `rust/tcl-irules/src/walker.rs` — the iRules object-reference walker's
  branch (feeds `bigip-cleanup` liveness)

## Failure modes

- Tagging a lambda-literal-shaped argument `ArgRole::Body` instead of
  `ArgRole::LambdaLiteral` makes every generic Body-role walker treat the
  parameter word as if it might be a command name — usually harmless (the
  parameter name rarely collides with a real command), but silently wrong,
  and a genuine collision (a parameter literally named `global` inside a
  declaration-scanning walker, say) produces a bogus result.
- Recognising `[list cmd ...]` unconditionally (not gated on the registry
  role of the *enclosing* argument position) would make `[list puts
  $x]`-as-plain-data read as if it invoked `puts` — over-broad.
- Extending `Traits::BUILDS_COMMAND_PREFIX` to `concat` (which also carries
  `Traits::PRODUCES_CANONICAL_LIST`, a *different*, more permissive
  const-folding trait) would be unsound: `concat`'s plain-join semantics
  don't guarantee a dynamic trailing argument stays a distinct word the way
  `list`'s per-element quoting does.
- Assuming the analyser's deep SSA/CFG modelling of `apply` bodies
  (`register_body_unit`) also follows `[list apply ...]` indirection
  overclaims the fix — it deliberately does not, to keep the
  correctness-sensitive analyser conservative (see decision rule 4).

## Triage checklist

1. Decode the semantic tokens for a minimal repro (`apply {p {body}} arg`
   and the same wrapped in `[list apply {p {body}} arg]`) and confirm the
   body's inner tokens (a builtin, a variable) appear as their own token
   kinds in *both* forms, not just the direct one.
2. Check whether the command carries `ArgRole::LambdaLiteral` (not `Body`) at
   the right index — `registry.arg_indices_for_role(head, args,
   ArgRole::LambdaLiteral)`.
3. For the list-quoted form, confirm `list` carries
   `Traits::BUILDS_COMMAND_PREFIX` and that the resolved inner head (`list`'s
   own first argument) itself carries `ArgRole::LambdaLiteral`.
4. If a *different* consumer (folding, formatting, minify, declaration,
   call-graph, iRules object references) also queries `ArgRole::Body`
   generically, confirm it has its own `LambdaLiteral`-aware branch — the
   role change silently stops the generic walker from seeing the argument at
   all otherwise.

## Test anchors

- `rust/tcl-lsp-core/src/semantic_tokens.rs` —
  `list_quoted_apply_lambda_body_recurses`,
  `list_quoted_apply_lambda_false_positive_guards`,
  `package_ifneeded_literal_script_recurses_as_body`
- `rust/tcl-compiler/src/lambda_literal.rs` — `split_lambda_literal`'s own
  unit tests (params/body/namespace splitting, dynamic-lambda guard)
- `rust/tcl-lsp-core/tests/command_prefix_integration.rs` —
  `list_quoted_prefix_records_head_span_and_baked_count`,
  `list_quoted_prefix_dynamic_head_is_not_recorded`,
  `cmd_substitution_with_non_list_head_is_not_recorded_as_prefix`
- `rust/tcl-lsp-core/src/folding.rs` —
  `apply_lambda_body_folds_and_recurses_into_nested_blocks`
- `rust/tcl-lsp-core/src/formatting/engine.rs` —
  `apply_lambda_body_indents_and_params_normalise`,
  `apply_lambda_namespace_element_preserved`
- `rust/tcl-lsp-core/src/minify.rs` — `apply_lambda_body_minifies`
- `rust/tcl-lsp-core/src/declaration.rs` —
  `global_declared_inside_apply_lambda_body_is_found`
- `rust/tcl-compiler/src/interprocedural.rs` —
  `call_inside_apply_lambda_body_is_a_direct_call`
- `rust/tcl-compiler/src/analyser/param_traits.rs` —
  `eval_param_inside_apply_lambda_body_records_eval_trait`
- `rust/tcl-irules/tests/script_bearing_args.rs` —
  `a_pool_used_only_inside_an_apply_lambda_body_is_referenced`,
  `the_walker_descends_into_every_script_bearing_role`
- `rust/tcl-lsp-server/tests/e2e/issue954_followup.rs` — full end-to-end
  coverage against the packaged native server
- `editors/vscode/src/test/issue954Followup.test.ts` — VS Code semantic
  tokens provider coverage

## Related

- [KCS index](README.md)
- [subcommand script body not highlighted](kcs-issue-subcommand-script-body-not-highlighted.md)
- [Command registry design doc](../design/compiler/command-registry.md)
- [Semantic Tokens feature](features/kcs-feature-semantic-tokens.md)
- [Glossary](../GLOSSARY.md)
