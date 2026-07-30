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

## Follow-up: codex review of the initial fix

The first pass at this fix (the sections above) shipped four gaps a codex
review of the PR caught — each a case where a *consumer* of the shared
`LambdaLiteral` split treated the lambda's fresh call frame as if it were
the same frame as its surroundings:

1. **List-quoted detection wasn't actually gated on the enclosing role**
   (decision rule 3 above describes the *intended* bound, but the shipped
   `insert_lambda_literal_overrides` recognised `[list apply {…} $x]`
   whenever `list`'s own first argument resolved to a `LambdaLiteral`-bearing
   spec, with no check on what argument slot the whole `[list …]`
   substitution itself occupied). `set data [list apply {x {puts $x}}
   value]` — `list` here only ever returns a value; nothing invokes
   `apply` — painted `x` as a `Parameter`, `puts` as a `Function`, and
   `apply` as a call-site reference, exactly the "over-broad" failure mode
   already called out below. Fixed by threading a `deferred_role` flag from
   [`collect_script`] through to [`insert_lambda_literal_overrides`]:
   `true` only when the `[…]` substitution being recursed into is itself the
   whole value of an argument slot the registry marks `Body` /
   `LambdaLiteral` / `CommandPrefix` (computed per-command by
   `deferred_role_arg_starts`, never inherited across recursion levels — a
   bare `list apply {…} $x` statement inside a real, executing body is
   exactly as inert as one at the top level).
2. **Call-graph resolution used the wrong namespace.** `interprocedural.rs`'s
   `scan_source_for_calls` recursed into a lambda body with `ctx` unchanged,
   so a bare call inside `apply {{} {helper}}` resolved relative to the
   *enclosing* proc's namespace instead of the lambda's own (the global
   namespace by default, or its optional third element) — Tcl always
   evaluates a two/three-element `apply` lambda in `::` or the given
   namespace, never the caller's. Fixed by building a synthetic caller qname
   from `elems.namespace` (or `"::"` when absent) for that one recursive call.
3. **Minify/format reprocessed a lambda element's raw, undecoded spelling.**
   `apply {{} puts\ hi}`'s real body (after Tcl's list-element decode) is
   `puts hi` — two words — but `minify.rs`/`formatting/engine.rs` fed the
   *raw* source span (`puts\ hi`, backslash intact) straight to the
   script minifier/formatter, which re-parses a bare `\ ` as an
   escaped-space-within-one-word, silently changing what would execute.
   Fixed with `split_lambda_literal_decoded` (a `lambda_literal.rs` sibling
   of `split_lambda_literal` that also collapses a non-literal element's
   backslashes, mirroring `tcl_syntax::list::split_list`), and re-quoting a
   reconstructed lambda literal's elements with
   `tcl_syntax::list::list_element` rather than an unconditional bare `{}`
   wrap — which also fixes a related bug where a multi-word parameter list
   (`apply {{x y} …}`) lost the braces grouping it into one list element on
   reassembly.
4. **`param_traits.rs` and `declaration.rs` treated the lambda's fresh frame
   as the enclosing frame.** `scan_deep` recursed into a lambda body with the
   *enclosing* proc's own `param_set`/`traits`, so `proc f {body} { apply {x
   {eval $body}} 1 }` wrongly marked `f`'s unrelated `body` param as
   evaluated (name collision only, the lambda's own param is `x`), while the
   real forwarding case (`apply {x {eval $x}} $body`) was missed. Fixed by
   inferring the lambda's own traits in isolation
   (`infer_param_traits_deep_with_config` against just the lambda's own
   param list and body) and propagating a lambda param's trait back to an
   enclosing param only when the actual argument at that position is a bare
   reference to it. Symmetrically, `declaration.rs`'s
   `collect_declarations_in_region` recursed into a lambda's body
   unconditionally whenever it fell inside the *current scan region*, so a
   `global x` declared inside a lambda was "visible" (by pure lexical/`Span`
   containment) to a cursor on an unrelated `$x` elsewhere in the enclosing
   proc. The analyser's scope tree has no `apply`-body scope kind for
   `scan.visible` to reflect this with, so the fix checks the cursor's raw
   byte offset against the lambda's own body span directly before recursing.

A subsequent self-review for the same bug class, prompted by the codex
findings above, found a fifth instance the review itself hadn't flagged
(likely out of its sampled scope, not a different root cause):

5. **The iRules object-reference walker inherited the caller's `set`-bound
   constants into the lambda frame.** `tcl-irules/src/walker.rs` tracks
   `set var literal` bindings in a `BindingScope`, propagated into nested
   bodies via `scope.child()` (a full clone) — correct for `if`/`foreach`/
   `switch`, which share the enclosing frame, but the `apply` lambda
   recursion used the identical `scope.child()`, so `set poolName /Common/x;
   apply {{} { pool $poolName }}` resolved `$poolName` inside a *zero-param*
   lambda from the enclosing binding, even though that variable is genuinely
   undefined inside the lambda's fresh frame at runtime. A false positive
   here means `bigip-cleanup` treats an actually-dead pool as still
   referenced. Fixed with `lambda_frame_scope`: an empty `BindingScope`, with
   only the lambda's own params bound from their actual arguments (resolved
   against the *enclosing* scope via the same `resolve_arg_value` ordinary
   references use) — mirrors the `param_traits.rs` fix in (4) exactly, one
   layer down (constant propagation instead of trait inference).

A deeper follow-up search for the same bug class (this time deliberately
broad rather than reactive to a specific review) found a sixth instance,
this time in a dimension none of the above touch — TclOO self-dispatch —
and, separately, positively confirmed the deep SSA/CFG/taint analyser has
no instances of its own:

6. **`my`/`$self`/`$this` dispatch resolved *through* an apply lambda's fresh
   frame.** `semantic_tokens.rs`'s `collect_lambda_literal` recursed into the
   body with the *enclosing* `ScriptCtx` unchanged, so `enclosing_class`
   (which correctly persists into an `if`/`foreach` body — those share the
   calling method's frame) also persisted into an `apply` lambda's body,
   which does not: `oo::class create C { method helper {} {} method run {} {
   apply {{} {my helper}} } }` painted `my helper` inside the lambda as a
   resolved call to `C`'s `helper` method, even though a bare (namespace-less)
   `apply` lambda runs in `::`, where `my` isn't defined — that call would
   actually raise "invalid command name my" at runtime. Fixed by clearing
   `oo_grammar` / `enclosing_class` / `scoped_env` before recursing into the
   lambda body, mirroring `folding.rs`'s pre-existing `None` reset for the
   same recursion (which this search re-confirmed is correct and was the
   template for the fix, not a new finding).
7. **The deep SSA/CFG/taint analyser: searched, no instances found.** Despite
   the doc's decision rule 4 above describing its `apply` handling as
   deliberately narrow (direct-call-shape only), a targeted search of
   `handlers.rs`'s `handle_apply_command`, `lowering/mod.rs`'s `lower_apply`,
   `per_item.rs`'s deferred-body machinery, `diagnostics/validity.rs`'s arity
   check, `bounds_checks.rs`'s scope-boundary scan, and `var_escape`'s
   conservative bail-out set confirmed each already treats the lambda's frame
   as genuinely isolated (fresh scope rooted at the lambda's own namespace,
   params bound only from the lambda's own element 0, no enclosing-proc state
   threaded in) — this line of the codebase predates the recent fixes and
   was already correct. Unlike the highlighting/analysis-text consumers
   above (which recurse by re-segmenting a body-text slice and so had to be
   *taught* isolation one file at a time), this layer gets it for free
   structurally: `lower_apply` builds each lambda body as its own
   `FunctionUnit` with its own `CfgFunction`/SSA graph — a bare `$var` inside
   the lambda has no shared symbol table to wrongly resolve against, and
   `taint_interproc.rs` seeds its proc-name set from
   `cu.ir_module.procedures.keys()` only, so a body unit's taint always falls
   back to its own bare facts rather than an inherited merge. `var_escape` /
   `memory_ssa` / `shimmer` don't visit body units at all (they iterate
   `analysable_functions()`, procedures only) — a coverage gap, not a leak,
   since every `apply` call site is already a `Statement::Barrier` those
   passes treat as clobber-everything regardless.

   One latent, currently-unreachable landmine for the same bug class: like
   `var_escape`/`memory_ssa`/`shimmer`, `class_lattice.rs`'s
   `collect_assign_kinds` / `build_class_values` iterate `top_level +
   procedures + methods`, excluding body units — so its `NsContext::enclosing`
   (a pure lexical-span lookup against `namespace eval` ranges, with no
   `apply`-frame awareness at all) never runs on an apply-lambda body's
   statements today. If a future change ever widens that iteration to cover
   body units (mirroring how taint's `analysable_body_function_units()` was
   added), a bare class name inside an `apply {…}` lexically nested in a
   `namespace eval NS {…}` block would resolve via the lexically-enclosing
   `NS` instead of Tcl's actual default (`::`, or the lambda's own third
   element) — the exact bug class fixed above, just for class-value
   inference. Not a fix here since there's nothing live to fix; a flag for
   whoever touches that iterator next.

An eighth instance surfaced from a different direction — the issue-923
differential audit's finding **idx 0** (georgtree/argparse), which found the
*original*, pre-role misparse still live in one walker:

8. **The `[…]`-substitution collectors re-segmented the whole lambda list as
   a script.** `apply` reached through the analyser's substitution walk —
   `set r [apply {{name opt args} {…}} …]`, `puts [apply {…} …]`, anything
   where `apply` is not the outermost command of a statement — never went
   through `AnalyserHookId::Apply`. Instead `record_command_invocations` /
   `collect_segment_recursive`
   ([`rust/tcl-compiler/src/analyser/commands.rs`](../../rust/tcl-compiler/src/analyser/commands.rs))
   reached its arguments through the shared registry-aware
   [`descend_command`](../../rust/tcl-compiler/src/parsing/syntax/descend.rs),
   which resolved `ArgRole::Body` only. Two symptoms, one cause:
   - a **false positive** `W123 Unknown command 'name opt args'` on the
     lambda's own parameter list, because an earlier revision of the
     collector descended the whole `{params body}` word as script source;
   - once the `LambdaLiteral` role stopped that, a **false negative** in its
     place: the lambda's real body was walked by *nothing*, so a genuinely
     unknown command inside it escaped W123 and every other per-command
     check whenever the `apply` sat inside a `[…]`.

   Fixed in `descend_command` itself rather than in either collector — it is
   the single registry-aware descent entry point, so both callers (and any
   future one) get it at once. It now resolves `ArgRole::LambdaLiteral`
   arguments alongside `ArgRole::Body` ones and descends **only the braced
   body element**, via the same `split_lambda_literal` /
   `LambdaLiteralElements::braced_body` primitive every other consumer uses.
   `descend_span` (new, beside `descend_token`) descends a list *element*'s
   content span rather than a whole delimited word. No command name appears
   anywhere in the change.

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
- Renaming (`rename apply myapply`) or aliasing (`interp alias {} myapply {}
  apply`) `apply` before calling it — `myapply {x {puts $x}} 5` — defeats
  every `LambdaLiteral`-aware consumer above, even though real Tcl treats the
  rename/alias as fully transparent and runs the lambda exactly as it would
  under the literal name. Confirmed at HEAD: the direct call's semantic
  tokens split `{x {puts $x}}` into a `parameter`, a `function`, and a
  `variable` token; both the renamed and aliased forms instead emit one
  opaque `string` token for the whole list, identical to what an
  unregistered command's argument would get. All eight consumers resolve a
  segmented command's head by exact literal string against
  `CommandRegistry::get` (a plain by-name lookup — see
  `rust/tcl-registry/src/registry.rs`), so a renamed or aliased head simply
  never matches `apply`'s spec. This is a different mechanism from issue
  #973 (the analyser's `known()` predicate in `scope.rs` not gating W123's
  existence check on deletion) — #973 is one analyser-side predicate
  partially growing rename/alias-awareness; this registry-head lookup has
  none at all, in any of the eight consumers. See the "Known limitations"
  note in [the command registry design doc](../design/compiler/command-registry.md#known-limitations)
  for the project-wide scope of this exposure (issue #1002).

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
  `list_quoted_apply_lambda_false_positive_guards` (includes the
  enclosing-role gate: `set data [list apply {…} value]` must not paint the
  lambda as executable),
  `package_ifneeded_literal_script_recurses_as_body`,
  `my_call_inside_apply_lambda_body_does_not_resolve`
- `rust/tcl-compiler/src/parsing/syntax/descend.rs` —
  `descend_command_resolves_the_lambda_body_element`,
  `descend_command_skips_untrustworthy_lambda_shapes`
- `rust/tcl-compiler/src/analyser/diagnostics/tests.rs` —
  `apply_lambda_in_command_substitution_does_not_report_its_parameter_list`
  (the audit's own repro), `apply_lambda_body_in_command_substitution_is_walked`,
  `plain_body_argument_in_command_substitution_still_walked`,
  `apply_lambda_parameters_named_like_commands_draw_no_unknown_command`
- `rust/tcl-compiler/src/lambda_literal.rs` — `split_lambda_literal`'s own
  unit tests (params/body/namespace splitting, dynamic-lambda guard) and
  `split_lambda_literal_decoded`'s (`decodes_bare_body_backslash_escape`,
  `braced_elements_are_not_decoded`, `decodes_bare_namespace_backslash_escape`,
  `decoded_dynamic_lambda_is_not_split`)
- `rust/tcl-lsp-core/tests/command_prefix_integration.rs` —
  `list_quoted_prefix_records_head_span_and_baked_count`,
  `list_quoted_prefix_dynamic_head_is_not_recorded`,
  `cmd_substitution_with_non_list_head_is_not_recorded_as_prefix`
- `rust/tcl-lsp-core/src/folding.rs` —
  `apply_lambda_body_folds_and_recurses_into_nested_blocks`
- `rust/tcl-lsp-core/src/formatting/engine.rs` —
  `apply_lambda_body_indents_and_params_normalise`,
  `apply_lambda_namespace_element_preserved`,
  `apply_lambda_body_with_backslash_escape_decodes`
- `rust/tcl-lsp-core/src/minify.rs` — `apply_lambda_body_minifies`,
  `apply_lambda_body_with_backslash_escape_decodes`,
  `apply_lambda_multi_word_param_list_keeps_its_braces`
- `rust/tcl-lsp-core/src/declaration.rs` —
  `global_declared_inside_apply_lambda_body_is_found`,
  `global_declared_inside_apply_lambda_body_is_not_visible_outside_it`
- `rust/tcl-compiler/src/interprocedural.rs` —
  `call_inside_apply_lambda_body_is_a_direct_call`,
  `call_inside_apply_lambda_body_resolves_in_lambda_namespace`,
  `call_inside_apply_lambda_body_resolves_in_explicit_namespace`
- `rust/tcl-compiler/src/analyser/param_traits.rs` —
  `eval_inside_apply_lambda_body_does_not_leak_to_unrelated_enclosing_param`,
  `eval_param_forwarded_into_apply_lambda_records_eval_trait`
- `rust/tcl-irules/tests/script_bearing_args.rs` —
  `a_pool_used_only_inside_an_apply_lambda_body_is_referenced`,
  `the_walker_descends_into_every_script_bearing_role`,
  `apply_lambda_does_not_inherit_enclosing_set_bindings`,
  `apply_lambda_param_resolves_via_forwarded_actual_argument`
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
