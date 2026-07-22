# Issue #923 — Differential Audit & Fix Campaign — STATUS (active)

Written so a fresh Claude Code session (or any engineer) with zero prior
context can pick this up from the repo alone.

**2026-07-22 update:** PR #963 (the original incarnation of this branch,
through commit `2676cc1`) merged into `origin/rust` as `9ec4cff` on
2026-07-20. Three more commits landed on `claude/tcl-lsp-issue-923-qzkfqz`
*after* that PR closed (the merge-note doc commit, then idx 105 and idx 106
— see §3) without ever being attached to a PR. Per this session's standing
branch-restart instructions, that's now been corrected: the branch was
`git rebase --onto`'d from `origin/rust`'s current tip (dropping the
already-merged history, keeping the 3 orphaned commits) and
force-with-lease-pushed, so **`claude/tcl-lsp-issue-923-qzkfqz` now sits
directly on top of current `origin/rust`** (`db2dcf6`/`e77879b`/`9bd26c8`,
new SHAs from the rebase) — no new branch name needed, and it's ready for a
fresh PR. **Keep developing on this same branch name going forward**; the
old "cut a fresh branch with a new suffix" advice below (§3, §8) is
superseded — only re-do the restart dance if a PR from *this* branch merges
and gets built on further without a rebase first.

**Branch:** `claude/tcl-lsp-issue-923-qzkfqz` — rebased directly onto
`origin/rust`, carrying only the not-yet-PR'd work: a merge-note doc commit
plus each fixed-and-pushed finding since the rebase (see §3 for the full,
current commit list).

**tl;dr:** A deep differential-audit campaign against real-world Tcl code
found 107 confirmed LSP correctness bugs total (22 in tcllib, 85 across 7
other corpora — the "main wave"). 14 tcllib findings are fixed, tested, and
pushed to this branch (§3/§6a); 6 tcllib findings remain, each with a
detailed `root_cause_hint` but no refined plan (§6a). The main-wave audit
(other 7 corpora, 105 findings total) is now **fully complete and triaged**
(§6b): 85 CONFIRMED (1 critical, 23 high, 60 medium, 1 low), 20 REFUTED. Two
of these are **fixed**: idx 61 (critical, §3's `d825d1d`) and idx 9 (high,
§3's `26e4ea3`); the other 83 main-wave findings are clustered by
feature/root-cause with a priority-ordered table in §6b, ready for a
future session to pick up efficiently. Nothing is lost — the raw data, the
exact scripts that produced it, and everything needed to resume are in
this directory.

---

## 1. The original mandate

Verbatim (this is the standing instruction driving all of this work):

> Dig deep into https://github.com/bitwisecook/tcl-lsp/issues/923 and tell me
> whats missing... verify against georgetree's and nico-robert's projects as
> well as tcllib and tk projects. verify everything is functioning properly.
> Consider tricky Tcl features like namespaces, rename, unknown, aliasing,
> safe-/sub-interpreters, tracing, tricky indirection, tclOO, upvar, uplevel,
> eval, the ::tcl and ::mathop namespaces, source, package loading,
> autoIndex, args in proc definitions and other difficult Tcl surfaces. Use
> plain, idiomatic, modern 2026 rust. This project is not yet in production
> so you can rearchitect and redesign things at will. Each fix must be
> considered deeply and the fixes must be looking at the general case, not
> just hacking in a specific check for the problem. Leverage the full
> compiler stack where possible, making fixes that utilise it to carry
> information correctly and bring information to the deepest layers with the
> richest information. The command registry, and other registries are the
> centralised information, you may add flags, structures, hooks or other
> required information across the registries to enable the compiler to make
> decisions without having that information encoded in the compiler itself,
> it must be fetched from the registry where that's possible and centralises
> information. No putting `if <command name string> then <action>` in the
> compiler/runtime stack. Create relevant TP/FP/TN/FN tests, lsp_e2e tests
> and vscode tests to cover all of this work. Do not add any new clippy
> allows with this work, clippy allows are a code smell and need to be
> properly fixed, the bar for them is very high, it must be shown that they
> are actually improving the readability of the code. C Tcl 9 is the truth
> oracle by default, or the version specific/dialect specific interpreter
> where possible.

Session mode: **Ultracode** was active (Workflow-tool multi-agent
orchestration expected/encouraged, cost not a primary constraint).

A later `/goal complete the exhaustive tests` made this a standing directive
(the session's Stop hook would not let the agent stop until it held) — that
goal is **not yet complete**; this pause is a deliberate user interrupt, not
goal completion.

---

## 2. Methodology established (follow this for all remaining work)

**Three-way differential testing**, per finding:

1. Mine a real, tricky pattern from an actual corpus file (not invented).
2. Build a minimal, faithful repro `.tcl` script.
3. Run it under a **real C Tcl interpreter** — `tclsh9.0` by default (Tcl
   9.0.4, built from source), `tclsh8.6` when version-sensitivity matters —
   to get **ground truth**. Never assume Tcl semantics; verify them.
4. Run the identical file through the built `tcl-lsp-server` via real LSP
   JSON-RPC (see `.claude/skills/lsp-client/lsp_client.py`, and note the fix
   already applied to it below).
5. Diff oracle vs LSP behaviour. Classify: **CONFIRMED** (real bug, provably
   diverges from tclsh), **REFUTED** (LSP is actually correct, or the
   "finding" doesn't reproduce), **PLAUSIBLE** (suspicious but not nailed
   down), **INCONCLUSIVE**.
6. For every CONFIRMED bug that gets fixed: implement it **registry-driven**
   (see §4), add unit tests covering **TP/FP/TN/FN** shapes, add an
   `lsp_e2e` test (real JSON-RPC round-trip, see
   `rust/tcl-lsp-server/tests/preview_tickets_e2e.rs` for ~14 worked
   examples), and a VS Code test where it adds distinct value (fixture under
   `editors/vscode/testFixture/`, test in
   `editors/vscode/src/test/previewTickets.test.ts`).
7. Run the validation gates before committing (see §6) — every commit on
   this branch so far is fully green: `cargo fmt --check`, targeted
   `clippy -D warnings` (zero new warnings, **zero new `#[allow]`**),
   `cargo xtask resolution-drift`, and the full test suite of every touched
   crate.

**Architecture bar** (from the mandate, non-negotiable): no
`if cmd_name == "foo"` string branching in the analyser/compiler. Fixes must
be driven by registry data (`tcl_registry::CommandSpec`/`SubCommand` fields,
hooks, traits) or by analysis **state** already recorded from a previous
pass (e.g. "is this command name a tracked interpreter handle", not "is this
command literally spelled `interp`"). See §5 for the specific shared
mechanisms this campaign already added — reuse them before inventing a new
one.

---

## 3. What's already fixed and merged (this branch)

Commits ahead of the `2c7693b` fork point, all pushed (the first four are the
campaign's own audit-finding fixes; the rest are the merge, this handoff doc,
and — landed *after* the pause, so they don't change anything else in this
document — routine PR maintenance):

| Commit | Finding(s) | Summary |
|---|---|---|
| `9448af9` | tcllib idx 127 | `param_name_spans_for_token`: braced param-list token span fed straight into the old span-scanner without accounting for the lexer's inner-end convention — every proc/method/lambda parameter after the **first** silently lost its declaration span. |
| `49f4eec` | tcllib idx 107, 115 | New `tcl_compiler::analyser::lookup_var_in_namespace` (namespace-path tree walk, reuses `advance_command_resolution_namespace`) — a fully- or relatively-qualified `$::ns::var` / `$ns::var` reference never resolved at all; only bare names were handled. |
| `37e6886` | tcllib idx 111 | (a) `interp create -safe NAME` never registered `NAME` as a known command (missing leading-option skip in the generic `defines_command_at` consumer — fixed via a new, reusable `CommandSpec`/`SubCommand::leading_option_word_count`). (b) `NAME eval { … }` (the interpreter's own object command) was never recognised as an isolated child-interp body, only literal `interp eval NAME { … }` was. |
| `26e5553` | tcllib idx 118, 119 | `namespace eval $name { … }` and `oo::define $class …` both keyed their scope/ClassDef by the dynamic argument's **raw written text**, so two unrelated call sites using the same variable name collided. Fixed with synthetic per-call-site keys (`@dynns@<offset>` / `@dynclass@<offset>`), mirroring the pre-existing `@interp@<path>` pattern. Also widened a stub `ClassDef`'s `body_span` (was name-token-sized, causing document-symbol range corruption independent of the merge bug). |
| (pre-existing, found+fixed same session before the above, folded into `9448af9`'s predecessor commit — see full history) | — | Nested definitions of a registry builtin (the "rename builtin away, install same-named shadow, restore it" idiom) permanently outranked the real builtin everywhere in the workspace. Gated via new `AnalysisResult::offset_is_inside_any_definition_body`. |
| `4fc2b84` | — | Merge of `origin/rust` (mainline) into this branch. **Not my work** — picked up commit `02386f6` "fix(lsp): resolve superclass/mixin/inherit as class references (issue #923) (#962)" which landed on mainline from a parallel effort on the same issue while this session was running. Merged cleanly, fully re-verified (all test suites, clippy, fmt, drift gate) after merge. |
| `4a33bea` | — | This handoff document (`STATUS.md` + the `data/`/`scripts/` snapshot) — captured when the campaign was paused. |
| `136d270` | — | CI fix: linked `STATUS.md` from `docs/design/README.md`'s index (`cargo xtask kcs-index-links` gate). |
| `55f9cb7` | — | **Not campaign work** — response to `chatgpt-codex-connector[bot]`'s automated review of PR #963, triaged and fixed after the pause per the standing PR-subscription protocol (basic maintenance of an already-open PR, distinct from resuming the audit). Three bugs, all confirmed against tclsh9.0 first: (a) `leading_option_word_count` didn't stop at a `--` terminator, so `interp create -- -safe` mis-consumed `-safe` as a flag instead of the literal name; (b) `workspace_command_exists_for_call` dropped *every* `command_links` entry under a builtin shadow, not just nested/conditional ones — fixed with a new `WorkspaceCommandLink.nested` field mirroring `WorkspaceProc.nested`; (c) `self.interpreters` wasn't invalidated by a plain `rename` of an interpreter's own handle command, so a later unrelated command reusing the freed name could be misidentified as isolated interpreter-eval. None of these correspond to a tracked audit finding (§5/§6 below) — the finding/idx inventory and remaining-work counts are unchanged by this commit. |

**The rows above (`9448af9`..`55f9cb7`) are now squashed into `origin/rust`
as `9ec4cff`** (PR #963) — their SHAs are historical (still visible on the
closed PR / in reflogs) but are no longer ancestors of this branch's current
tip; see the 2026-07-22 update at the top of this document. Rows below are
the branch's actual current content, on top of `origin/rust`:

| Commit | Finding(s) | Summary |
|---|---|---|
| `db2dcf6` | — | (rebased from `7289cd6`) The "PR #963 merged" doc update itself, folded into this restart. |
| `e77879b` | tcllib idx 105 | (rebased from `1973832`) W123 false positive + harmful "replace with `exit`" quickfix for a bare `exists`/`get` call inside a proc defined under `::tcl::dict` (the ensemble's dynamically-mapped implementation namespace) — new `CommandSpec::implementation_namespace` field plus per-subcommand standalone `CommandSpec`s (`dict::qualified_specs`) so `::tcl::dict::exists` resolves as a real, independently-callable command the way C Tcl actually implements the ensemble. |
| `9bd26c8` | tcllib idx 106 | (rebased from `c6936d7`) `namespace ensemble create -map`/`-subcommands` targets were never resolved for definition/hover/references/rename — new `AnalysisResult::ensemble_subcommand_targets` (per-ensemble subcommand→target-proc map, populated in `handlers.rs`) threaded through **both** command-invocation-recording pipelines (top-level `process_command` and nested-`[...]`-substitution `push_collected_heads`) as an existence-probed reference, then consumed by `tcl-lsp-core`'s definition/hover/references providers via the same `instance_method_at_cursor` cursor-shape helper TclOO method dispatch already used (`receiver method` and `ensemble subcommand` share the identical syntax). Tier 2 (cross-file ensemble resolution) intentionally scoped out as a separate follow-up. |
| `b56d0f9` | tcllib idx 3 | `rename OLD NEW` treated either argument as unconditionally dynamic the instant it contained `$`/`[`, even when the value was a compile-time constant (`set old ::foo_impl; rename $old ::foo`). New `Analyser::resolve_rename_arg` tries `resolve_const_word` (a pure single `Var`/literal token) then a new `text::fold_interpolation_single` (multi-token concatenation) against the existing `lookup_const_string` lattice — no new lattice, mirrors `resolve_expansion_count`'s precedent for `{*}$var`. Deliberately out of scope (documented via two FN tests): a `foreach` loop variable and a bare proc parameter are never constant-tracked, so the tcllib `json::SwitchTo` idiom this finding was mined from stays unresolved — needs interprocedural constant propagation, a separate follow-up. Also confirmed unaffected (pre-existing, separately-scoped gaps): hover and same-file references through any rename, even the always-worked fully-literal case. |
| `b8f8d1e` | tcllib idx 110 | `namespace eval $ns [list namespace unknown $handler]` (tcllib's `namespacex::hook::Set` idiom) never installed as far as the analyser could tell — the `[...]` body is a `Cmd`-kind token `analyse_body`'s literal-`{...}`-only body walk never enters, and the generic nested-substitution scan resolves the segment's head to `list`, never dispatching `AnalyserHookId::NamespaceUnknown` — so a call the handler chain resolves at runtime drew a false W123. New `Analyser::detect_list_wrapped_namespace_unknown`, called from `handle_namespace_eval_command` (shared by `namespace eval`/`namespace inscope`), descends the `Cmd` body one level with the same `cmd_fragments`/`descend_token`/`segments_from_tree` idiom already used three times elsewhere for nested-substitution discovery, and on an exact `list namespace unknown ?HANDLER?` match calls the existing `handle_namespace_unknown_command` unmodified. Deliberately narrow (pinned via a dedicated test): does not recognise the same idiom built via `concat`/`format`/`linsert`/a helper proc. |
| `538f3af` | tcllib idx 113 | A bareword call to a sibling TclOO method/classmethod/property inside another method's body only actually dispatches when `oo::Helpers::link` (a genuine core TclOO builtin since 8.6) installed a per-object-namespace alias for it — `lookup_class_member`/`class_member_hover_text` matched unconditionally, resolving calls real tclsh errors "invalid command name" on. New `link` `CommandSpec` (mirrors `next`/`self`/`classvariable`; also fixes a pre-existing spurious W002 on legitimate `link` usage, since the only prior "link" spec was the unrelated EDA-Synopsys command) + new `ClassDef::linked_members` populated by `Analyser::collect_oo_links` (shallow, top-level-only method-body scan) gate the three lookup arms; the two-element `link {alias target}` form also closes a related false negative (hover/definition on the alias previously returned nothing). Incidental registry hygiene needed for a clean `command-backing` gate: classified the `::tcl::dict::*` names idx 105 left unclassified (genuinely backed via `dict`'s single handler) and a pre-existing, unrelated `zipfs` gap. |
| `edd0119` | tcllib idx 9 | `set s [interp create -safe]` never bound `s` to the interpreter it created, so a later `interp alias $s name {} target` / `interp eval $s {…}` / `$s eval {…}` (the idiom tcllib's doctools.tcl actually uses) abstained outright — spurious "unknown command" + zero go-to-definition. New scope-chain-aware `Analyser::interp_var_bindings` map (mirrors `const_strings`, not the flat `instance_classes`) populated by `handle_set_command`, consumed by `handle_interp_alias`'s cross-domain branch, `handle_interp_eval_command`, and `handle_interp_handle_eval_command`. A pathless `interp create` gets a synthetic per-call-site `@autoname@<offset>` key (same convention as `@dynns@`/`@dynclass@`). Also fixed two bugs found live while researching: nested `interp create` inside `[...]` never reached its handler at all (worked around the same way TclOO's `record_instance_creation` does, by detecting the `set VAR [interp create ...]` shape directly rather than routing through the general nested-dispatch machinery); and `interp eval $var {…}`'s dynamic-path handling keyed its isolated child scope by raw variable text, collapsing unrelated procs sharing a variable name into one domain — closed for the now-tracked subset. Deliberately out of scope: the fully-untracked dynamic-path case (e.g. a bare proc parameter) stays as conservative as before; `interp delete $var` still uses the blunt file-wide `dynamic_interp_ops` flag rather than precisely bumping one interpreter's epoch. |
| `6f54b9b` | tcllib idx 120 | `ActiveRecord find ...` (a classmethod called on the class's own bound command) and the same call inherited by a non-overriding subclass (`Table find ...`) never resolved — `receiver_instance_class` only ever recognised a `$var`/created-instance-command receiver, never a bare word naming a class directly. Three-part, two-crate fix: (1) `tcl-compiler/oo.rs` gains `apply_oo_self` — stock TclOO's own `self method NAME ARGS BODY` spelling (ooutil's `classmethod` counterpart) had no `apply_oo_subcommand` arm at all, a separate gap found while researching this finding; new `MethodDef::is_self_method` marks it as NOT inherited by a subclass (unlike ooutil's `classmethod`, confirmed via tclsh); `collect_method_body` now unwraps `self`/`private` via the existing `unwrap_wrapper_member`, so their bodies get walked for diagnostics for the first time too. (2) `definition.rs`'s `receiver_instance_class` also resolves a bare class-name word (via the existing `resolve_written_class_name`); new `MethodBucket` (`Instance`/`Class`) keeps the two receiver kinds from cross-resolving — bundled in, since the signature was already changing: instance dispatch no longer falls back to `class_methods` either, closing a pre-existing false positive on `rec1 find` (an instance calling a classmethod); `completion.rs` picked up the same bucket-awareness via a new shared `receiver_method_bucket` helper. (3) `references.rs`'s `find_obj_method_call_sites` gains the class's own bound-command names (and, when not `is_self_method`, every inheriting subclass's) as a receiver set separate from its existing `instance_classes`-keyed one, so references/rename now find every class-command call site too. Deliberately out of scope: the `self { … }` block form; mixin-only classmethod propagation (ooutil follows `superclass` only); `hover.rs`'s `obj_method_hover_text` staying un-bucketed (no MRO walk there at all, so only the direct-declaration case benefits). |
| `7ae4e6c` | tcllib idx 116 | `apply {{params} body ns}` runs `body` in `ns`, not wherever the `apply` call is lexically written — a bareword call inside that body resolved against its lexical nesting purely by coincidence of the pre-existing "lexically nearest" fallback, since the `Scope` subtree `handle_apply_command` builds for `ns` sits under fresh, body-span-less namespace wrapper nodes the ordinary span-containment walk can never reach. New `AnalysisResult::namespace_overrides: Vec<(Span, String)>` (flat, span-keyed runtime-context pins), consulted by `innermost_namespace_at`/`namespace_context_at` ahead of the lexical walk, threaded through their ~13 call sites across `tcl-lsp-core`. Also resolves one hop through a `$var` or `[list {params} $body ns]` indirection via new `Analyser::resolve_dynamic_apply_lambda` + `lookup_const_string_in_namespace` (the `const_strings` analogue of `lookup_var_in_namespace`). Wired into `per_item.rs`'s incremental rebase/graft — required, not optional, for the fix to survive on-keystroke analysis. Deliberately out of scope, documented in `definition.rs`'s module doc: `apply` reached only via a registry `command_prefixes` slot (`coroutine co ::apply $lambda`); a proc that re-injects its own arguments as a script via a captured `uplevel`-namespace + trace/callback (tcllib generator.tcl's `finally` — the exact idiom the finding's own repro traces through, unmodelable without hardcoding a specific library); and `$var`-to-`$var` indirection deeper than one hop. |
| `d825d1d` | **main-wave** idx 61 (critical) | `if {$cond} mymod::foo` / `uplevel 1 mymod::qux` — an unbraced (bareword) body — is a legitimate, statically-known zero-arg call, but `dispatch_body_arguments` only ever recursed a *braced* body into `analyse_body`, so it was invisible to `command_invocations` entirely: go-to-definition/hover still resolved it (independent cursor-token walk), but references/rename silently missed the call site — an LSP-presented "complete" rename left it referring to the old, now-nonexistent name, breaking the program at runtime. Fixed by dispatching a genuinely-static bareword body (`Esc`-kind, single word, no `$`/`[`) through the ordinary `process_command` path, reusing the existing `has_substitution` guard (widened from `pub(super)`). New `dispatch_one_body_argument` extracted to keep the caller under the line-count lint. This is the **first fixed finding from the main audit wave** (see §6b), not the tcllib list — everything else in §6a stays tcllib-only. |
| `26e4ea3` | **main-wave** idx 9 (high) | A cursor placed directly on a variable's own bareword declaration/write token (a proc/method parameter, a `catch script name` result-var reusing an existing variable) resolved to nothing at all across definition/hover/references/rename, even though every `$name` read of the same variable resolved fine (independent cursor-token walk) — a rename from such a cursor silently produced zero edits, the worst failure mode (no error, no signal). Root cause traced empirically (a throwaway debug scaffold against real analyser output, not just the finding's own hint): (1) `scope_chain_at`'s `body_span`-keyed containment walk never reaches a proc/method scope for a byte offset inside its own *parameter list*, which sits textually before `body_span` starts; (2) a `catch` result-var reusing an existing variable records its own bareword token in `VarDef.references`, never `definition_span`. Fixed by replacing `definition.rs`'s narrow, rename-only `var_name_at_definition_offset` (scope-chain-gated, `definition_span`-only) with `var_def_at_declaration_offset`: an unconditional whole-scope-tree search matching byte-offset against every `VarDef`'s `definition_span` *and* every `references` span — safe without scope-visibility filtering since a byte-offset span match is unambiguous by construction. Wired into `definition()`, `hover_with_profile()` (extracted into a new `variable_hover` helper for the line-count lint), `references()`'s `variable_references`, and both of `rename.rs`'s call sites plus `rename_var`'s own internal re-lookup — closing a latent gap in rename that predates this session, found while tracing the same root cause. Secondary, independently-confirmed half of the same finding: `tcl::prefix` (TIP 265, Tcl 8.6+) had no `CommandSpec` at all unlike sibling ensemble `tcl::mathop` — the VM already implements it (`tcl-vm/src/cmd_prefix.rs`), but hover/completion/signature-help had nothing to show. New `tcl-registry/src/commands/tcl/prefix_.rs` registers the ensemble + its 3 subcommands (`all`, `longest`, `match`), including `match`'s `-exact`/`-message`/`-error` options so the existing generic leading-option arity skip doesn't miscount them as positional args. |

Run `git log --oneline 2c7693b..9ec4cff` for the exact list (this branch's
own history — `9ec4cff` is where PR #963 landed on `origin/rust`, see below);
each commit message has full rationale.

**Superseded — kept for history only:** this note used to say to merge
`origin/rust` into this branch before resuming, because other sessions/PRs
were landing #923 fixes on `origin/rust` in parallel (see the `02386f6`
merge above) while this branch was still an open PR. That's now moot: PR
#963 (this branch) itself **merged into `origin/rust` as `9ec4cff` on
2026-07-20** — this branch's content and `origin/rust` are no longer
diverging, they're the same up to that commit. **Resume on a fresh branch
cut from `origin/rust`**, not by pushing more commits onto
`claude/tcl-lsp-issue-923-qzkfqz` (whose PR is closed/merged and won't take
more commits meaningfully). The general caution still applies going
forward, just against the new branch instead: `git fetch origin rust`
before starting each session and check for anything landed since, since
other #923 work may still be arriving in parallel.

---

## 4. Shared mechanisms added this campaign (reuse these first)

Before implementing any remaining fix, check whether it can reuse one of
these rather than inventing something new:

- **`tcl_compiler::analyser::lookup_var_in_namespace`**
  (`rust/tcl-compiler/src/analyser/scope.rs`) — given a fully-qualified
  namespace path and a base variable name, walks the whole scope tree
  (namespace can be `namespace eval`-reopened anywhere) and returns the
  `VarDef`. Never matches a proc's own locals even if its command-resolution
  namespace happens to coincide.
- **`CommandSpec::leading_option_word_count` /
  `SubCommand::leading_option_word_count`** (`rust/tcl-registry/src/spec.rs`)
  — given a declared `options: &[OptionSpec]` table and the argument words,
  returns how many leading words are option flags (handles unique-prefix
  matching and value-taking options), so a fixed-index argument consumer
  (`defines_command_at`, positional `arg_types` hints) can skip past
  `?-flag? ?-opt val?` prefixes generically instead of assuming position 0.
- **The `@interp@<path>` / `@dynns@<offset>` / `@dynclass@<offset>` pattern**
  (`rust/tcl-compiler/src/analyser/handlers.rs`) — the general answer to "this
  construct's identity can't be statically resolved to a real Tcl name, and
  two unrelated occurrences must never be treated as the same thing just
  because they look alike." Mint a synthetic key using the `@`-prefixed
  (unrepresentable in real Tcl) marker plus either the interpreter's
  qualified path or the argument token's own byte offset. Already used for:
  child-interpreter domains, dynamic `namespace eval` targets, dynamic
  `oo::define` targets, and (idx 9) a pathless `set VAR [interp create
  ...]`'s auto-generated interpreter name (`@autoname@<offset>`).
  **Candidate for reuse**: idx 121 (dynamic TclOO constructor class — though
  that one may be better solved by consulting `const_contributors`, see its
  research plan) and potentially others.
- **`AnalysisResult::offset_is_inside_any_definition_body`**
  (`rust/tcl-compiler/src/analyser/types.rs`) — "is this byte offset inside
  any recorded proc/class body", i.e. "does this only exist conditionally,
  at call time, not load time." Used to stop a nested/shadow definition from
  outranking a real builtin or an unconditional top-level one.
- **`isolate_interp_eval_body`** (`rust/tcl-compiler/src/analyser/handlers.rs`)
  — shared body-isolation helper for both literal `interp eval PATH { … }`
  and handle-form `NAME eval { … }`. If a third spelling of "run this script
  in an isolated child-interpreter scope" turns up, extend this, not a new
  parallel implementation.
- **`innermost_namespace_at`** (`rust/tcl-lsp-core/src/definition.rs`) — thin
  wrapper over `tcl_compiler::analyser::command_resolution_namespace_at`,
  the one canonical implementation of "what namespace does an unqualified
  command/qualifier at this byte offset resolve against" (handles the
  `oo::class` method global-only special case, `uplevel #0` reset, deep
  nesting). Empirically verified (this session, against real tclsh) to be
  the *same* rule a variable qualifier resolves against too.

### Gotchas learned the hard way

- **Lexer span convention**: a braced/bracketed/quoted word token's span
  starts *at* the opening delimiter but ends *one byte short* of the closing
  one (`Token.content_offset` + `SourceMap::token_text()`/manual
  `content_offset`-based stripping is the only correct way to get the inner
  content — never hand-slice `source[tok.span]`). This exact bug (idx 127)
  cost real user-visible correctness; it's an easy trap to fall into again.
- **`resolution-drift` xtask gate**: flags *any* new `.name ==` /
  `.name == word`-shaped scan near `all_procs`/`all_classes`, **including in
  test code** — there is no test exemption. A deliberate, reviewed test
  assertion needs a `// drift-ok: <reason>` comment on the line or one of
  the two lines above it (see `two_dynamic_namespace_eval_blocks_...` test
  in `handlers.rs` for a worked example).
- **Disk space**: this environment's writable disk is a **fixed per-session
  allowance** (this session's was ~38G total, shared between `/tmp` and the
  repo checkout). `cargo build`/`test`/`clippy --workspace` pulls in a large,
  unrelated dependency tree (wasmtime, cranelift, an MCP server, a fuzzer, a
  debugger — this is a big workspace, ~37+ crates, not just the LSP). A
  cold full-workspace build alone can consume the **entire** allowance
  (hit ENOSPC twice this session). Mitigations that worked:
  - Prefer scoped builds while iterating: `cargo test -p tcl-compiler -p
    tcl-lsp-core -p tcl-lsp-server` etc., **not** `--workspace`, unless doing
    a final pre-commit sweep.
  - `cargo xtask <gate>` (e.g. `resolution-drift`) only pulls in
    `tcl-registry`+deps — much cheaper than a full build, use it liberally.
  - On ENOSPC: `rm -rf /home/user/tcl-lsp/target` — always safe, it's pure
    build cache, regenerates on next build. This alone freed ~28G both times.
  - `df -h /` before/after big builds to keep an eye on it; the reported
    `Avail` misleads at 0 with the size column still showing plenty — that's
    the allowance, not disk hardware, being exhausted.
- **`Workflow` tool + `args` parameter**: passing a large JSON array via the
  `args:` input parameter failed at runtime (`FINDINGS.map is not a
  function`) despite being passed correctly as a JSON array, not a
  stringified one — root cause not fully diagnosed, but the reliable
  workaround is to **embed the data directly in the script body** as a JS
  literal (`const FINDINGS = [...]`) rather than relying on `args`, then
  invoke via `Workflow({scriptPath: <file-you-wrote-yourself>})`. All four
  scripts in `scripts/` in this directory use (or should be adapted to use)
  this pattern.
- **`tclsh9.0`**: the built Tcl 9.0.4 binary needs `LD_LIBRARY_PATH` set to
  find `libtcl9.0.so` — there's a wrapper at `/usr/local/bin/tclsh9.0` that
  does this. **This wrapper will not exist in a fresh session/container** —
  see §7 for how to recreate the whole oracle environment.
- **`.claude/skills/lsp-client/lsp_client.py`**: fixed this session (commit
  `8a9a7cb`, already on the branch) to answer server-initiated LSP requests
  (`workspace/configuration`, etc.) — without this, `pull_and_apply_config`
  hangs forever and cross-file workspace indexing silently never runs,
  producing convincing-looking but **false** "cross-file resolution is
  broken" bugs. If cross-file LSP behaviour ever looks suspicious again,
  suspect the test harness before the server.

---

## 5. Data inventory (`data/`)

All the raw research output, so nothing has to be re-mined or re-audited
from scratch:

| File | Contents |
|---|---|
| `01-mined-findings-per-corpus.json` | Raw mining output, one entry per corpus (8 corpora), each with its list of mined tricky-pattern candidates. |
| `02-mined-findings-flattened-130.json` | Same data flattened to one list, 130 total candidate findings across all 8 corpora. |
| `03-tcllib-audit-input-25.json` | The 25 tcllib-corpus findings selected for the first audit wave (indices 105–129 in the flattened numbering). |
| `04-tcllib-audit-results-COMPLETE-25of25.json` | **Complete.** All 25 tcllib findings differentially audited: 22 CONFIRMED, 3 REFUTED. This is the source for the tcllib triage in §6. |
| `05-main-audit-input-105.json` | The 105 findings from the other 7 corpora (SpiceGenTcl, argparse, tclopt, ticklecharts, pix, tomato, tk) selected for the second audit wave (indices 0–104). |
| `06-main-audit-results-PARTIAL-49of105.json` | Superseded by the `COMPLETE` file below — kept for history (idx 0–48 only, the first half of the wave). |
| `06-main-audit-results-COMPLETE-105of105.json` | **Complete.** All 105 main-wave findings (idx 0–104) differentially audited: 85 CONFIRMED, 20 REFUTED. **Triaged** — see §6b for the severity/corpus/feature breakdown and priority-ordered tables. idx 61 (critical, §3's `d825d1d`) and idx 9 (high, §3's `26e4ea3`) are fixed; the other 83 CONFIRMED findings are open work. |
| `07-remaining-tcllib-findings-14.json` | The 14 tcllib CONFIRMED findings not yet fixed (full detail: summary, failure_scenario, oracle_output, lsp_output, root_cause_hint, repro_path — repro files themselves are gone, scratchpad-only, but the hints are detailed enough to rebuild a repro in minutes). |
| `08-research-plans-PARTIAL-8of14.json` | **Partial — 8 of 14 done.** Refined, current-code-verified fix plans for 8 of the 14 remaining tcllib findings (idx 3, 9, 105, 106, 110, 113, 116, 120), produced by a research-only agent fan-out (no file edits) that re-checked each root-cause hint against the *current* (post-merge) code and proposed concrete changes + test scenarios. idx 18, 24, 121, 122, 125, 128 do not have refined plans yet — use `07`'s `root_cause_hint` field directly for those, which is still quite detailed.

---

## 6. Remaining work, prioritized

### 6a. tcllib — 6 CONFIRMED findings, not yet fixed

**idx 105, 106, 3, 110, 113, 9, 120, and 116 are done** (fixed, tested,
pushed — see §3's `e77879b` / `9bd26c8` / `b56d0f9` / `b8f8d1e` /
`538f3af` / `edd0119` / `6f54b9b` / `7ae4e6c` rows); removed from the
table below. 6 remain, none with a refined plan left — use `07`'s
`root_cause_hint` directly for all of them.

All in `data/07-remaining-tcllib-findings-14.json`. Suggested order (by
severity):

| idx | severity | feature | one-line summary | refined plan? |
|---|---|---|---|---|
| 121 | medium | tclOO | `set class ::Derived; set obj [$class create NAME]` — single-hop variable-indirected class name never resolved (needs `const_contributors`/SSA wiring, or the `@dyn...@` pattern). | no — use root_cause_hint |
| 122 | medium | upvar | W210 false-positive: call-by-name upvar writes from a user proc invoked inside an `if`/`while` **condition** aren't recognised (only 4 hardcoded builtins are, in `cmd_substitution_out_vars`). | no |
| 18 | medium | uplevel | W210 false-positive for `upvar`+`uplevel` custom-control-structure idiom, when the actual upvar is one proc-call hop away from the literal call site. | no |
| 125 | medium | eval | W220 false-positive: `{$var}` inside a double-quoted string mis-tokenized as non-substituting brace-quoted (re-lexing loses quote context). | no |
| 128 | medium | package_loading | `PackageResolver::parse_pkg_index` ignores `if {...} { return }` reachability guards in `pkgIndex.tcl`, over-suppressing W123. | no |
| 24 | medium | autoindex | `hover()` never falls back to the cross-document/autoload resolution tiers that `definition()`/`references()` already use. | no |

Each of these follows the exact same playbook as the 8 already-fixed
findings: read root_cause_hint (no refined plan remains for any of these
6) → confirm
still-reproduces against current code → check `tclsh9.0`/`tclsh8.6` ground
truth if not already fully confirmed → registry-driven fix reusing §4's
mechanisms where applicable → unit tests (TP/FP/TN/FN) + lsp_e2e test →
validation gates → commit.

### 6b. Main audit wave — COMPLETE (105/105) and triaged

All 105 findings (idx 0–104, the other 7 corpora: SpiceGenTcl, argparse,
tclopt, ticklecharts, pix, tomato, tk) are differentially audited and
merged into `data/06-main-audit-results-COMPLETE-105of105.json` (idx 0–48
from the original `06-...-PARTIAL-49of105.json` batch, idx 49–104 from the
`wf_61c6b92a-e22` workflow's completed resume run). **85 CONFIRMED, 20
REFUTED.** Of the 85 CONFIRMED: **2 fixed** (idx 61, critical — §3's
`d825d1d`; idx 9, high — §3's `26e4ea3`), **83 remaining**.

**By corpus** (confirmed only): ticklecharts 20, tk 17, argparse 10,
SpiceGenTcl 10, tclopt 13 (6+7, split across two inconsistent corpus-label
strings in the raw data — same corpus), tomato 7, pix 8.

**By feature** (confirmed only): tricky_indirection 14, tclOO 13,
namespaces 11, proc_args 10, upvar 7, source 6, tcl_mathop 5, rename 4,
package_loading 3, uplevel 3, tracing 3, aliasing 2, safe_interp 2, eval 1,
autoindex 1.

#### Priority tier 1 — critical + high (24 findings, 2 already fixed)

Fix these first — each is either data-loss-risk (a rename that silently
breaks the program, idx 61) or a full-zero-results failure of a core
navigation feature (go-to-definition/references/hover returning nothing)
on a common real-world idiom. `severity`/`summary` are the audit's own
classification; read the finding's full `root_cause_hint` in the JSON
before starting each — the one-line summaries below are index-and-locate,
not a substitute.

| idx | severity | corpus | feature | one-line summary |
|---|---|---|---|---|
| 61 | critical | ticklecharts | uplevel | **FIXED** (`d825d1d`) — unbraced `if`/`uplevel` body bareword call sites invisible to references/rename. |
| 9 | high | argparse | tcl_mathop | **FIXED** (`26e4ea3`) — variable bareword declarations (proc params, `catch` result-vars) unresolved by definition/hover/references/rename; `tcl::prefix` had no `CommandSpec`. |
| 10 | high | argparse | source | Cross-document find-references (and rename-safety) misses call sites living in a file only reached via a dynamic `source` target. |
| 18 | high | SpiceGenTcl | namespaces | A bareword class/proc name only reachable through a wildcard `namespace import NS::*` never resolves. |
| 29 | high | tclopt | namespaces | Probing the same `namespace import`-adjacent mechanic as idx 18 from a different angle — read both together. |
| 31 | high | tclopt | tricky_indirection | A proc defined twice, verbatim, at two different line ranges in the same file — which declaration wins is not modelled correctly. |
| 32 | high | tclopt | tricky_indirection | Two related sub-questions, LSP gets them backwards from what's actually correct — read the full note. |
| 33 | high | tclopt | tricky_indirection | Go-to-definition/find-references fail on a bareword class/command reference gated by a runtime condition. |
| 39 | high | tclopt | rename | `textDocument/rename`/`references` omit the target-name argument of `rename` itself as a reference site. |
| 46 | high | ticklecharts | source | A `source` target built from a namespace variable derived through another indirection layer — go-to-definition/cross-file resolution fails. |
| 52 | high | ticklecharts | tricky_indirection | Two distinct results bundled in one finding — `[self class]`/`[self method]`-based dynamic class-def dispatch. |
| 56 | high | ticklecharts | tclOO | ticklecharts installs `classvar`/`callback` directly into `::oo::Helpers` — the doc/registry model for TclOO helper installation doesn't account for this. |
| 63 | high | ticklecharts | proc_args | Go-to-definition AND find-references both zero-result for an internal `my AddBarSeries`/sibling-method-call idiom. |
| 68 | high | pix | proc_args | Find-References (and Rename) never unifies a proc's `global` declaration with its call sites the same way it does for other variable-scope forms. |
| 70 | high | pix | tricky_indirection | The parallel/lock-step multi-list `foreach` form (`foreach dirName {LIST1} fileName {LIST2} {...}`) breaks resolution. |
| 71 | high | pix | source | `source [file join [file dirname [info script]] ...]` — the canonical "load my sibling file" idiom — has a real gap somewhere in the chain despite the base case resolving. |
| 76 | high | tomato | tclOO | LSP guesses the wrong class among several structurally-similar TclOO classes for a dynamically-dispatched call. |
| 77 | high | tomato | tclOO | `method * {type}` (operator-overload dispatch) reads `$other` in its isa-object branch incorrectly / the resolver doesn't follow it. |
| 79 | high | tomato | proc_args | `constructor {args}` reinterprets its single `args` list as multiple different logical shapes depending on caller — the parameter model doesn't track this. |
| 84 | high | tk | namespaces | `tk/library/systray.tcl` (and print.tcl, fileicon.tcl, accessibility.tcl) splices a namespace-qualified name dynamically; resolution fails. |
| 86 | high | tk | rename | `tk/library/accessibility.tcl`'s `foreach wtype {...} { rename ::$wtype ::tk::ac... }` loop-generated rename targets aren't tracked. |
| 90 | high | tk | safe_interp | `tk/library/safetk.tcl` declares a throwaway 0-arg `proc ::safe::loadTk {}` stub then redefines it with the real signature later — arity/definition tracking picks the wrong one. |
| 94 | high | tk | tricky_indirection | `tearoff.tcl`'s `-tearoffcommand`/`cget`/`upvar`-adjacent indirection mechanics both reproduce, need a combined fix. |
| 95 | high | tk | tricky_indirection | `tk.tcl:594-596`'s `$w ${dir}view scroll ...` — a subcommand synthesized by string-concatenation at the call site — isn't resolved. |

#### Priority tier 2 — medium + low (60 + 1 = 61 findings), grouped by feature for clustering

Group findings sharing a feature/root-cause together in one fix pass the
way idx 107+115 and idx 118+119 were — many of these look like they share
a root cause within a feature group (e.g. the three `upvar`-adjacent W210
false-positives in ticklecharts, idx 57/58/59; the two `tclopt`
mixin/oo::configurable class-scoping findings, idx 34/36).

| feature | count | idx (severity) |
|---|---|---|
| tclOO | 10 | 15, 16, 34, 35, 36, 53, 54, 55, 96, 97 (all medium) |
| namespaces | 8 | 3, 19, 43, 44, 64, 65, 75, 85 (all medium) |
| tricky_indirection | 7 | 0, 1, 2, 14, 49, 50, 51 (all medium) |
| upvar | 7 | 7, 22, 57, 58, 59, 98, 99 (all medium) |
| proc_args | 7 | 11, 28, 37, 62, 67, 78, 104 (all medium) |
| tcl_mathop | 4 | 30, 80, 81, 103 (all medium) |
| package_loading | 3 | 4, 42, 72 (all medium) |
| source | 3 | 27, 41, 102 (all medium) |
| tracing | 3 | 47, 48, 92 (all medium) |
| rename | 2 | 5, 45 (all medium) |
| aliasing | 2 | 21, 89 (all medium) |
| uplevel | 2 | 38 (medium), 100 (low) |
| eval | 1 | 24 (medium) |
| autoindex | 1 | 73 (medium) |
| safe_interp | 1 | 91 (medium) |

Each idx's full detail (summary, failure_scenario, oracle_output,
lsp_output, root_cause_hint) is in
`data/06-main-audit-results-COMPLETE-105of105.json`, keyed by `idx`. Follow
the same playbook as every fix already landed: re-confirm against current
code and a real tclsh oracle → registry-driven fix reusing §4's mechanisms
→ TP/FP/TN/FN unit tests + lsp_e2e test → validation gates → commit.

### 6c. Broader mandate coverage check

The mandate named specific "tricky Tcl feature" dimensions. Rough coverage
so far (from mined+fixed findings): namespaces ✓✓✓, rename ✓ (idx 3 done),
unknown ✓ (idx 110 done; interp-create angle done), aliasing ✓ (idx 113
done), safe-/sub-interpreters ✓✓ (idx 111, 9 both done), tracing ✓✓ (idx
115, 116 both done), tricky indirection ✓✓ (idx
118/119 done), tclOO ✓ (idx 120 done, idx 121 open), upvar (open, idx 122), uplevel
(open, idx 18), eval (open, idx 125), `::tcl`/`::tcl::mathop` namespaces ✓
(idx 127's host procs were the bug, mathop dispatch itself was already
correct), source (not specifically probed yet — consider mining more
`source`-heavy patterns), package loading (open, idx 128), autoIndex (open,
idx 24), proc args ✓ (idx 127). Once the remaining tcllib + main-wave
findings are exhausted, consider a dedicated mining pass for `source` and
`autoIndex` specifically if coverage still looks thin.

---

## 7. Recreating the verification environment

None of this survives a fresh container — re-run before auditing/fixing
anything new:

```sh
# Rust toolchain (need >= 1.97)
rustup update stable

# Tcl 9.0.4 oracle, built from source in-tree (tmp/ is gitignored, this
# does NOT survive a fresh checkout — this is exactly how it was done this
# session, verified from the actual build dir's git remote/tag):
mkdir -p /home/user/tcl-lsp/tmp && cd /home/user/tcl-lsp/tmp
git clone --branch core-9-0-4 https://github.com/tcltk/tcl tcl9.0.4
cd tcl9.0.4/unix && ./configure && make -j
# No `make install` needed/used — run it straight out of the build dir via
# a wrapper (this is the ACTUAL wrapper this session used, verbatim):
cat > /usr/local/bin/tclsh9.0 <<'EOF'
#!/bin/bash
export LD_LIBRARY_PATH="/home/user/tcl-lsp/tmp/tcl9.0.4/unix:${LD_LIBRARY_PATH:-}"
exec /home/user/tcl-lsp/tmp/tcl9.0.4/unix/tclsh "$@"
EOF
chmod +x /usr/local/bin/tclsh9.0

# Tcl 8.6 oracle — Debian package, already provides /usr/bin/tclsh8.6
apt-get install -y tcl8.6

# Reference corpora (only needed if mining NEW findings, not for fixing the
# already-mined ones in data/):
mkdir -p corpora && cd corpora
git clone https://github.com/georgtree/SpiceGenTcl
git clone https://github.com/georgtree/argparse
git clone https://github.com/georgtree/tclopt
git clone https://github.com/nico-robert/ticklecharts
git clone https://github.com/nico-robert/pix
git clone https://github.com/nico-robert/tomato
git clone https://github.com/tcltk/tk
git clone https://github.com/tcltk/tcllib  # tcllib-2.0

# Build the LSP server + CLI once
cd /home/user/tcl-lsp && cargo build -p tcl-lsp-server -p tcl-cli
```

The `.claude/skills/lsp-client/lsp_client.py` skill (already fixed and
committed on this branch) drives the built server over real JSON-RPC for
manual differential checks — see its own docstring for usage
(`python3 lsp_client.py diagnostics|definition|hover|references file.tcl
[line col]`).

---

## 8. Immediate next steps for whoever picks this up

1. **Branch is already current:** `claude/tcl-lsp-issue-923-qzkfqz` was
   rebased directly onto `origin/rust`'s tip on 2026-07-22 (see the update at
   the top of this doc and §3) — just `git fetch origin
   claude/tcl-lsp-issue-923-qzkfqz && git checkout claude/tcl-lsp-issue-923-qzkfqz
   && git reset --hard origin/claude/tcl-lsp-issue-923-qzkfqz` (or clone
   fresh) and keep developing here. No new branch name needed unless a PR
   from this branch merges and more work lands without a rebase first — if
   that happens, repeat the same restart-and-rebase dance (§3's 2026-07-22
   note) rather than inventing a new suffix.
2. `git fetch origin rust && git log origin/rust -10` — check for any *other*
   new #923 work landed on mainline since this branch's current base (see
   §3) before starting; merge (not rebase, once you've started your own new
   commits on top) if anything new is there.
3. Recreate the tclsh9.0/8.6 oracle environment (§7).
4. Pick the next finding to fix — two ready queues, both fully triaged:
   - §6a: 6 remaining tcllib findings (idx 121/122/18/125/128/24), no
     refined plan for any — use `07`'s `root_cause_hint` directly.
   - §6b: 83 remaining main-wave findings (idx 61 and idx 9 are fixed so
     far), fully triaged into a priority-ordered critical/high table (22
     remaining, start here) and a feature-clustered medium/low table (61
     findings, group by feature when fixing). Likely the higher-leverage
     queue given its size and the presence of several zero-results
     go-to-definition/references failures on common real-world idioms.
5. Follow the playbook in §2/§4. Commit after each finding (or tightly
   related cluster of findings), same granularity as the fixes already
   landed — don't batch unrelated fixes into one commit.
6. Periodically re-run `cargo test --workspace` (full, not scoped) and
   `cargo clippy --workspace --all-targets -- -D warnings` as a sweep, not
   just the scoped per-crate checks used while iterating — the mandate's
   validation bar is workspace-wide. Watch disk space (§4's gotcha): a cold
   `--workspace` test build can exhaust the session's allowance on its own;
   `rm -rf target/debug/incremental` is the cheap, safe recovery (regenerates
   on next build, and frees the bulk of it — no need to nuke all of `target/`
   first).
7. Both queues (§6a's 6 tcllib findings, §6b's 84 main-wave findings) are
   independent — fix from whichever queue makes sense, no need to exhaust
   one before starting the other. Keep this document's counts current as
   findings get fixed: move a finished idx out of §6a/§6b's tables and into
   §3's commit table, same pattern as every fix so far.
8. Keep the Stop-hook's implicit contract: uncommitted work is a liability —
   commit and push after every completed fix, don't let it accumulate.
9. When ready to submit, open a PR from this branch (no PR is currently open
   for it — #963 is closed/merged and superseded, see the top of this doc).
