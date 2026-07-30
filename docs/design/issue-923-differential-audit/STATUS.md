# Issue #923 — Differential Audit & Fix Campaign — STATUS (paused, tracked)

Written so a fresh Claude Code session (or any engineer) with zero prior
context can pick this up from the repo alone.

**2026-07-23 update — campaign paused at a clean boundary, remaining work
tracked as GitHub issues:** By explicit user request, this campaign is
pausing here rather than continuing through the full remaining backlog.
18 of 22 tcllib findings and 23 of 85 main-wave findings are fixed, tested,
and pushed to this branch (see §3); every commit's own gate (fmt, targeted
clippy, `xtask resolution-drift`, full test suite of every touched crate)
passed clean, so what's landed is solid to build on. The remaining known
findings are now durably tracked as GitHub issues rather than left to rely
solely on this document + the `data/` JSON files:

- [#1017](https://github.com/bitwisecook/tcl-lsp/issues/1017) — tcllib idx
  128 (package_loading): `pkgIndex.tcl` reachability guards ignored,
  over-suppressing W123.
- [#1018](https://github.com/bitwisecook/tcl-lsp/issues/1018) — tcllib idx
  24 (autoindex): `hover()` missing the cross-document/autoload fallback
  tiers `definition()`/`references()` already have.
- [#1019](https://github.com/bitwisecook/tcl-lsp/issues/1019) — rollup
  tracking issue for all 62 remaining main-wave findings (1 tier-1 — idx
  79, investigated but not fixed, needs receiver-type inference — plus 61
  tier-2, feature-clustered).

**2026-07-23 update — rebased onto `origin/rust` tip (`9a031ac`):** PR
#1020 (this branch) was rebased onto the current `rust` tip to pick up
several other sessions' independent, overlapping fixes for issue #923
(ensemble `-map`/W129, `enclosing_class_at`, mathop/mathfunc registry
unification, and more) landed in the interim. All 76 replayed commits were
reconciled commit-by-commit, full workspace `cargo fmt --check` +
`cargo clippy --workspace --all-targets -- -D warnings` pass clean.
Derived catalogs (`docs/generated/wasm-command-backing.md`,
`editors/zed/src/generated/tcl_commands.json`) were regenerated post-rebase
and their gate tests pass.

CI on the post-rebase push surfaced 4 test failures across the `rust-tests`
and `lsp-e2e` checks (run
[30035264703](https://github.com/bitwisecook/tcl-lsp/actions/runs/30035264703)).
Each was individually root-caused (pre-existing vs. rebase-introduced,
verified by re-running against both the pre-rebase branch tip and
`origin/rust` alone) and filed with full repro rather than fixed outright,
per this session's stop-at-a-working-checkpoint instruction:

- [#1021](https://github.com/bitwisecook/tcl-lsp/issues/1021) — 2
  pre-existing `origin/rust` tests (`#975`, `#1014`) plus their e2e twin,
  failing only once combined with this branch (cross-feature interaction,
  not a regression from this branch's own commits).
- [#1022](https://github.com/bitwisecook/tcl-lsp/issues/1022) — W210
  false-fires inside a dynamically-resolved `-safe` interp eval body;
  confirmed pre-existing on this branch's own pre-rebase tip (predates the
  rebase, latent idx=9 gap).
- [#1023](https://github.com/bitwisecook/tcl-lsp/issues/1023) — Codex
  review: `link` modelled as an unconditional global `CommandSpec`, plus
  its own idx=113 e2e regression test; confirmed via pre/post-rebase
  bisection to be a genuine rebase-introduced regression, not latent.
- [#1024](https://github.com/bitwisecook/tcl-lsp/issues/1024) — Codex
  review: `scan_script` forced into quoted-body mode unconditionally by the
  idx=125 fix, mis-scanning genuine script bodies.
- [#1025](https://github.com/bitwisecook/tcl-lsp/issues/1025) — Codex
  review: `interp create` value-flow path uses `split_whitespace`,
  mis-splitting a braced/nested interpreter path.

This document and `data/06-main-audit-results-COMPLETE-105of105.json` /
`data/07-remaining-tcllib-findings-14.json` remain the source of full
detail (repro steps, oracle output, root_cause_hint) for every finding —
the GitHub issues summarize and link back here, they don't replace it.
Sections below (§6a/§6b/§8) are otherwise left as they were mid-campaign
for historical accuracy; treat the GitHub issues above as the current
front door for picking this back up.

**2026-07-22 update:** PR #963 (the original incarnation of this branch,
through commit `2676cc1`) merged into `origin/rust` as `9ec4cff` on
2026-07-20. Three more commits landed on `claude/tcl-lsp-issue-923-qzkfqz`
*after* that PR closed (the merge-note doc commit, then idx 105 and idx 106
— see §3) without ever being attached to a PR. Per this session's standing
branch-restart instructions, that's now been corrected: the branch was
`git rebase --onto`'d from `origin/rust`'s current tip (dropping the
already-merged history, keeping the 3 orphaned commits) and
force-with-lease-pushed, so **`claude/tcl-lsp-issue-923-qzkfqz` now sits
directly on top of current `origin/rust`** (`db2dcf6`/`25d6a09`/`2c48bcc`,
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
other corpora — the "main wave"). 18 tcllib findings are fixed, tested, and
pushed to this branch (§3/§6a); 2 tcllib findings remain, each with a
detailed `root_cause_hint` but no refined plan (§6a). The main-wave audit
(other 7 corpora, 105 findings total) is now **fully complete and triaged**
(§6b): 85 CONFIRMED (1 critical, 23 high, 60 medium, 1 low), 20 REFUTED.
Twenty-four of these are **fixed**: idx 0 (medium, PR #1068 on
`claude/commandregistry-compiler-fixes-tshu8d-quickfixes`), idx 61
(critical, §3's `438e56f`), idx 9
(high, §3's `51d0a35`), idx 10 (high, §3's `2330862`), idx 18 (high, §3's
`1f5fe71`), idx 29 (high, already resolved by idx 18's fix, pinned in
§3's `d218463`), idx 31 (high, §3's `89b75a5`), idx 32 (high, §3's
`8646964`), idx 33 (high, same root cause as idx 18, pinned in §3's
`1135d75`), idx 39 (high, §3's `8827a17`), idx 46 (high, partial — §3's
`ccec425`), idx 52 (high, §3's `03aef3d`), idx 56 (high, §3's `741f9ba`),
idx 63 (high, partial — §3's `65dda01`), idx 68 (high, §3's `134c31c`),
idx 70 (high, §3's `d5e4d65`), idx 71 (high, §3's `2339d4a`), idx 76
(high, §3's `0bde16e`), idx 77 (high, §3's `51a630f`), idx 84 (high,
partial — §3's `7115bc8`), idx 86 (high, §3's `99cf07f`), idx 90 (high,
§3's `7d476f5`), idx 95 (high, §3's `ef36c73`), idx 94 (high, §3's
`959bca8`); the other 61
main-wave findings are clustered by feature/root-cause with a
priority-ordered table in
§6b, ready for a future session to pick up efficiently. Nothing
is lost — the raw data,
the exact scripts that
produced it, and everything needed to resume are in
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
| `25d6a09` | tcllib idx 105 | (rebased from `1973832`) W123 false positive + harmful "replace with `exit`" quickfix for a bare `exists`/`get` call inside a proc defined under `::tcl::dict` (the ensemble's dynamically-mapped implementation namespace) — new `CommandSpec::implementation_namespace` field plus per-subcommand standalone `CommandSpec`s (`dict::qualified_specs`) so `::tcl::dict::exists` resolves as a real, independently-callable command the way C Tcl actually implements the ensemble. |
| `2c48bcc` | tcllib idx 106 | (rebased from `c6936d7`) `namespace ensemble create -map`/`-subcommands` targets were never resolved for definition/hover/references/rename — new `AnalysisResult::ensemble_subcommand_targets` (per-ensemble subcommand→target-proc map, populated in `handlers.rs`) threaded through **both** command-invocation-recording pipelines (top-level `process_command` and nested-`[...]`-substitution `push_collected_heads`) as an existence-probed reference, then consumed by `tcl-lsp-core`'s definition/hover/references providers via the same `instance_method_at_cursor` cursor-shape helper TclOO method dispatch already used (`receiver method` and `ensemble subcommand` share the identical syntax). Tier 2 (cross-file ensemble resolution) intentionally scoped out as a separate follow-up. |
| `c022921` | tcllib idx 3 | `rename OLD NEW` treated either argument as unconditionally dynamic the instant it contained `$`/`[`, even when the value was a compile-time constant (`set old ::foo_impl; rename $old ::foo`). New `Analyser::resolve_rename_arg` tries `resolve_const_word` (a pure single `Var`/literal token) then a new `text::fold_interpolation_single` (multi-token concatenation) against the existing `lookup_const_string` lattice — no new lattice, mirrors `resolve_expansion_count`'s precedent for `{*}$var`. Deliberately out of scope (documented via two FN tests): a `foreach` loop variable and a bare proc parameter are never constant-tracked, so the tcllib `json::SwitchTo` idiom this finding was mined from stays unresolved — needs interprocedural constant propagation, a separate follow-up. Also confirmed unaffected (pre-existing, separately-scoped gaps): hover and same-file references through any rename, even the always-worked fully-literal case. |
| `264cfdf` | tcllib idx 110 | `namespace eval $ns [list namespace unknown $handler]` (tcllib's `namespacex::hook::Set` idiom) never installed as far as the analyser could tell — the `[...]` body is a `Cmd`-kind token `analyse_body`'s literal-`{...}`-only body walk never enters, and the generic nested-substitution scan resolves the segment's head to `list`, never dispatching `AnalyserHookId::NamespaceUnknown` — so a call the handler chain resolves at runtime drew a false W123. New `Analyser::detect_list_wrapped_namespace_unknown`, called from `handle_namespace_eval_command` (shared by `namespace eval`/`namespace inscope`), descends the `Cmd` body one level with the same `cmd_fragments`/`descend_token`/`segments_from_tree` idiom already used three times elsewhere for nested-substitution discovery, and on an exact `list namespace unknown ?HANDLER?` match calls the existing `handle_namespace_unknown_command` unmodified. Deliberately narrow (pinned via a dedicated test): does not recognise the same idiom built via `concat`/`format`/`linsert`/a helper proc. |
| `183baef` | tcllib idx 113 | A bareword call to a sibling TclOO method/classmethod/property inside another method's body only actually dispatches when `oo::Helpers::link` (a genuine core TclOO builtin since 8.6) installed a per-object-namespace alias for it — `lookup_class_member`/`class_member_hover_text` matched unconditionally, resolving calls real tclsh errors "invalid command name" on. New `link` `CommandSpec` (mirrors `next`/`self`/`classvariable`; also fixes a pre-existing spurious W002 on legitimate `link` usage, since the only prior "link" spec was the unrelated EDA-Synopsys command) + new `ClassDef::linked_members` populated by `Analyser::collect_oo_links` (shallow, top-level-only method-body scan) gate the three lookup arms; the two-element `link {alias target}` form also closes a related false negative (hover/definition on the alias previously returned nothing). Incidental registry hygiene needed for a clean `command-backing` gate: classified the `::tcl::dict::*` names idx 105 left unclassified (genuinely backed via `dict`'s single handler) and a pre-existing, unrelated `zipfs` gap. |
| `78ea6e2` | tcllib idx 9 | `set s [interp create -safe]` never bound `s` to the interpreter it created, so a later `interp alias $s name {} target` / `interp eval $s {…}` / `$s eval {…}` (the idiom tcllib's doctools.tcl actually uses) abstained outright — spurious "unknown command" + zero go-to-definition. New scope-chain-aware `Analyser::interp_var_bindings` map (mirrors `const_strings`, not the flat `instance_classes`) populated by `handle_set_command`, consumed by `handle_interp_alias`'s cross-domain branch, `handle_interp_eval_command`, and `handle_interp_handle_eval_command`. A pathless `interp create` gets a synthetic per-call-site `@autoname@<offset>` key (same convention as `@dynns@`/`@dynclass@`). Also fixed two bugs found live while researching: nested `interp create` inside `[...]` never reached its handler at all (worked around the same way TclOO's `record_instance_creation` does, by detecting the `set VAR [interp create ...]` shape directly rather than routing through the general nested-dispatch machinery); and `interp eval $var {…}`'s dynamic-path handling keyed its isolated child scope by raw variable text, collapsing unrelated procs sharing a variable name into one domain — closed for the now-tracked subset. Deliberately out of scope: the fully-untracked dynamic-path case (e.g. a bare proc parameter) stays as conservative as before; `interp delete $var` still uses the blunt file-wide `dynamic_interp_ops` flag rather than precisely bumping one interpreter's epoch. |
| `af8c3d5` | tcllib idx 120 | `ActiveRecord find ...` (a classmethod called on the class's own bound command) and the same call inherited by a non-overriding subclass (`Table find ...`) never resolved — `receiver_instance_class` only ever recognised a `$var`/created-instance-command receiver, never a bare word naming a class directly. Three-part, two-crate fix: (1) `tcl-compiler/oo.rs` gains `apply_oo_self` — stock TclOO's own `self method NAME ARGS BODY` spelling (ooutil's `classmethod` counterpart) had no `apply_oo_subcommand` arm at all, a separate gap found while researching this finding; new `MethodDef::is_self_method` marks it as NOT inherited by a subclass (unlike ooutil's `classmethod`, confirmed via tclsh); `collect_method_body` now unwraps `self`/`private` via the existing `unwrap_wrapper_member`, so their bodies get walked for diagnostics for the first time too. (2) `definition.rs`'s `receiver_instance_class` also resolves a bare class-name word (via the existing `resolve_written_class_name`); new `MethodBucket` (`Instance`/`Class`) keeps the two receiver kinds from cross-resolving — bundled in, since the signature was already changing: instance dispatch no longer falls back to `class_methods` either, closing a pre-existing false positive on `rec1 find` (an instance calling a classmethod); `completion.rs` picked up the same bucket-awareness via a new shared `receiver_method_bucket` helper. (3) `references.rs`'s `find_obj_method_call_sites` gains the class's own bound-command names (and, when not `is_self_method`, every inheriting subclass's) as a receiver set separate from its existing `instance_classes`-keyed one, so references/rename now find every class-command call site too. Deliberately out of scope: the `self { … }` block form; mixin-only classmethod propagation (ooutil follows `superclass` only); `hover.rs`'s `obj_method_hover_text` staying un-bucketed (no MRO walk there at all, so only the direct-declaration case benefits). |
| `a824ff8` | tcllib idx 116 | `apply {{params} body ns}` runs `body` in `ns`, not wherever the `apply` call is lexically written — a bareword call inside that body resolved against its lexical nesting purely by coincidence of the pre-existing "lexically nearest" fallback, since the `Scope` subtree `handle_apply_command` builds for `ns` sits under fresh, body-span-less namespace wrapper nodes the ordinary span-containment walk can never reach. New `AnalysisResult::namespace_overrides: Vec<(Span, String)>` (flat, span-keyed runtime-context pins), consulted by `innermost_namespace_at`/`namespace_context_at` ahead of the lexical walk, threaded through their ~13 call sites across `tcl-lsp-core`. Also resolves one hop through a `$var` or `[list {params} $body ns]` indirection via new `Analyser::resolve_dynamic_apply_lambda` + `lookup_const_string_in_namespace` (the `const_strings` analogue of `lookup_var_in_namespace`). Wired into `per_item.rs`'s incremental rebase/graft — required, not optional, for the fix to survive on-keystroke analysis. Deliberately out of scope, documented in `definition.rs`'s module doc: `apply` reached only via a registry `command_prefixes` slot (`coroutine co ::apply $lambda`); a proc that re-injects its own arguments as a script via a captured `uplevel`-namespace + trace/callback (tcllib generator.tcl's `finally` — the exact idiom the finding's own repro traces through, unmodelable without hardcoding a specific library); and `$var`-to-`$var` indirection deeper than one hop. |
| `438e56f` | **main-wave** idx 61 (critical) | `if {$cond} mymod::foo` / `uplevel 1 mymod::qux` — an unbraced (bareword) body — is a legitimate, statically-known zero-arg call, but `dispatch_body_arguments` only ever recursed a *braced* body into `analyse_body`, so it was invisible to `command_invocations` entirely: go-to-definition/hover still resolved it (independent cursor-token walk), but references/rename silently missed the call site — an LSP-presented "complete" rename left it referring to the old, now-nonexistent name, breaking the program at runtime. Fixed by dispatching a genuinely-static bareword body (`Esc`-kind, single word, no `$`/`[`) through the ordinary `process_command` path, reusing the existing `has_substitution` guard (widened from `pub(super)`). New `dispatch_one_body_argument` extracted to keep the caller under the line-count lint. This is the **first fixed finding from the main audit wave** (see §6b), not the tcllib list — everything else in §6a stays tcllib-only. |
| `51d0a35` | **main-wave** idx 9 (high) | A cursor placed directly on a variable's own bareword declaration/write token (a proc/method parameter, a `catch script name` result-var reusing an existing variable) resolved to nothing at all across definition/hover/references/rename, even though every `$name` read of the same variable resolved fine (independent cursor-token walk) — a rename from such a cursor silently produced zero edits, the worst failure mode (no error, no signal). Root cause traced empirically (a throwaway debug scaffold against real analyser output, not just the finding's own hint): (1) `scope_chain_at`'s `body_span`-keyed containment walk never reaches a proc/method scope for a byte offset inside its own *parameter list*, which sits textually before `body_span` starts; (2) a `catch` result-var reusing an existing variable records its own bareword token in `VarDef.references`, never `definition_span`. Fixed by replacing `definition.rs`'s narrow, rename-only `var_name_at_definition_offset` (scope-chain-gated, `definition_span`-only) with `var_def_at_declaration_offset`: an unconditional whole-scope-tree search matching byte-offset against every `VarDef`'s `definition_span` *and* every `references` span — safe without scope-visibility filtering since a byte-offset span match is unambiguous by construction. Wired into `definition()`, `hover_with_profile()` (extracted into a new `variable_hover` helper for the line-count lint), `references()`'s `variable_references`, and both of `rename.rs`'s call sites plus `rename_var`'s own internal re-lookup — closing a latent gap in rename that predates this session, found while tracing the same root cause. Secondary, independently-confirmed half of the same finding: `tcl::prefix` (TIP 265, Tcl 8.6+) had no `CommandSpec` at all unlike sibling ensemble `tcl::mathop` — the VM already implements it (`tcl-vm/src/cmd_prefix.rs`), but hover/completion/signature-help had nothing to show. New `tcl-registry/src/commands/tcl/prefix_.rs` registers the ensemble + its 3 subcommands (`all`, `longest`, `match`), including `match`'s `-exact`/`-message`/`-error` options so the existing generic leading-option arity skip doesn't miscount them as positional args. |
| `2330862` | **main-wave** idx 10 (high) | `is_tcl_source`'s extension allowlist (gating `collect_tcl_files`, the sole discovery mechanism `scan_workspace_folders` uses for un-opened files) omitted `.test` — the standard tcltest extension every mined corpus, and tcllib's own test suite, use throughout (`test/argparse.test`). A proc's call sites living in an un-opened `.test` file were invisible to cross-document find-references, so a rename built on the same reference set silently left the `.test` file unrenamed. One-line fix (add `"test"` to the allowlist), covered at both the predicate level and the `collect_tcl_files` disk-walk integration point. Deliberately out of scope: the finding's own secondary, lower-severity observation (`package require`-then-`source` redefinition returns both candidate proc declarations with no execution-order "last-definition-wins" modelling) — more defensible since both are textually real declarations. |
| `1f5fe71` | **main-wave** idx 18 (high) | A bareword proc/class name reachable only through a wildcard `namespace import NS::*` never resolved — in-document or cross-document, regardless of how many commands the source namespace exports (the finding's own repro claimed a working single-command case broke when an unrelated second command was added; a research agent's exhaustive trace found no path where the single-command case ever worked either — a complete feature absence, not a threshold bug). Root causes: `namespace export` was never recorded anywhere in the analyser, and no resolution path (in-document `resolve_called_proc`/`resolve_class_target_at`, or cross-document `resolve_workspace_symbols`/`WorkspaceIndex`) ever consulted `namespace_imports` for a user proc/class (only a narrower registry-builtin-only path in `hover.rs` did). Fixed: new `AnalyserHookId::NamespaceExport` → `handle_namespace_export_command` → `AnalysisResult::namespace_exports`, wired through `per_item.rs` like its sibling `namespace_imports`; bareword-through-wildcard-import resolution added at the same priority tier as exact-namespace resolution, for both procs and classes, in-document and cross-document (new `WorkspaceGlobImport`/`WorkspaceNamespaceExport` + `WorkspaceIndex::resolve_wildcard_import`), gated on the source namespace actually exporting the name (an unexported sibling stays unresolved, tclsh9.0/8.6-verified `invalid command name`). Implemented via a Workflow (implement → two independent adversarial verify passes), which caught and fixed two real issues before this commit: (1) `resolve_wildcard_import` wrongly restricted candidate imports to the *calling* document — `namespace import` binds to the namespace, not the file, so a shared "imports.tcl" pattern (import statement in a third file, separate from both the call site and the export) was still unresolved; fixed by dropping the file-scoping filter (matches the pre-existing exact-import path, which is workspace-global already). (2) The same function re-scanned every workspace-wide glob import/export on *every* invocation inside the cross-document find-references hot loop — O(invocations × workspace-wide glob-import count); fixed with a new `WildcardImportIndex` per-namespace grouping built once per query, mirroring the loop's pre-existing `defined`/`links` hoisting. |
| `d218463` | **main-wave** idx 29 (high, test-only) | Found while probing an already-refuted hypothesis, this finding turned out to be the exact same root cause as idx 18. Empirically verified (built server + tclsh9.0-matched repros) that idx 18's fix already resolves both of idx 29's confirmed failure modes: a same-file decoy no longer wins over the real exported+imported target by lexicographic tie-break (`fallback_proc_by_simple_name`'s pre-existing leniency), and a cross-document TclOO class miss already resolves via idx 18's shared `workspace_command_exists` path (which covers classes exactly like procs, just not unit-tested for that case in the idx 18 diff itself). No production changes — pinned both as permanent, dedicated regression tests. |
| `89b75a5` | **main-wave** idx 31 (high) | A proc declared twice, verbatim, in the same document (plain Tcl's own "last redefinition wins" semantics, tclsh9.0/8.6-verified — georgtree_tclopt's `tclopt.tcl` declares `::tclopt::List2array` at two line ranges) broke cross-document find-references/rename when queried from the earlier, *shadowed* declaration's own name token — go-to-definition already worked there (falls through to ordinary word-text resolution), but `resolve_workspace_symbols` only checked `all_procs.values().find(covers(name_span))`, and `all_procs` (keyed by qualified name) retains only the winning declaration's span on a duplicate insert, so the shadowed token could never match. Applying the resulting incomplete rename is worse than a no-op: the un-rewritten cross-file caller silently starts running the dead shadowed definition (still lying around under the old name) instead — demonstrated end-to-end in the finding's own repro (real program output changed, no error surfaced). Fixed with a new `AnalysisResult::proc_declaration_sites: Vec<(String, Span)>`, the flat never-deduplicated companion to `all_procs` recording every proc declaration's own name span (including shadowed ones) in source order, wired through `per_item.rs` like every other span-bearing field; `resolve_workspace_symbols` now also checks this list, resolving through `all_procs` to whichever definition currently wins. Deliberately scoped to procs only — `oo::class create`'s reopen semantics are additive in real Tcl (configures the same class object, not a fresh shadow), a materially different, unverified question this fix doesn't speculate on. The same-document path (`resolve_proc_target_at`) was already correct (confirmed with a TN regression guard). |
| `8646964` | **main-wave** idx 32 (high) | A TclOO class body with 2+ separate `variable` statements (georgtree_tclopt's `::tclopt::Mpfit` declares `variable funct m ftol ...` then, separately, `variable Pars` — the same idiom recurs in all four real optimiser classes in `tclopt.tcl`) only kept the LAST statement's names as recognised instance variables — go-to-definition/hover/find-references on any name from an earlier statement returned nothing, even though tclsh9.0 proves both statements' names are simultaneously live (`variable` in a class body means "always present in every method", additive across statements, never a reset — the same declaration a `variable` command inside a method body itself makes, just issued once for the class). `apply_oo_subcommand`'s `"variable"` arm assigned `class_def.variables = sub_args.to_vec()` on every call, discarding earlier statements' names — unlike the sibling `"export"`/`"unexport"` arms right next to it, which already correctly `.extend()`. One-line fix (`=` → `.extend()`). Also confirmed (order-swap + plain-vs-`{*}`-command-head repros) that the finding's own dynamic-dispatch reference-tracking observation is a separate, pre-existing, correctly-abstained limitation, untouched here. |
| `1135d75` | **main-wave** idx 33 (high, test-only) | A class *instantiation* call (`GSA new`, the real corpus's `arbitaryTest.tcl` idiom) reached only through a cross-document wildcard `namespace import NS::*` — the finding's own root-cause citation is `WorkspaceIndex::index_command_links`'s glob-pattern skip, the exact mechanism idx 18 already fixed; found independently before idx 18 landed. Verification hit a wrinkle: the `lsp_client.py` CLI script initially reported this (and even a simpler same-document, non-wildcard, fully-qualified call) as still broken; cross-checking with the `tcl-lsp-core::definition::definition()` unit-level harness and the `Lsp::tcl()` e2e harness (both proven reliable all campaign) showed both resolve correctly — a CLI tooling artifact, not a real regression, now documented in the new test's own comment as a heads-up. No production changes — pinned as permanent, dedicated regression coverage (a class-instantiation call specifically, distinct in shape from idx 18/29's own tests). |
| `8827a17` | **main-wave** idx 39 (high) | `rename OLD NEW`'s own `OLD` word is a genuine reference to the command being renamed (the same shape `ArgRole::CommandName` models for `info body PROC`), but `handle_rename` never recorded it as a `command_invocation` — `references()`/`rename()` build exclusively from that list, so the token was invisible to both (hover/go-to-definition already resolved it via an independent cursor-token walk). Real corpus shape: a tcltest `-setup`/`-body`/`-cleanup` idiom (`proc gaussfunc {...} {...}` ... `rename gaussfunc ""`) — applying the LSP's own incomplete rename `WorkspaceEdit` renames the proc but leaves the `rename` statement's `OLD` word stale, crashing a previously-passing test at runtime with no diagnostic warning. Fixed with a direct `push_command_reference` call from `handle_rename` rather than a registry retag (`ArgRole::Name` → `CommandName`): the generic `record_command_name_invocations` pass skips dynamic words outright, while `handle_rename` already constant-folds both arguments itself, so a direct push captures strictly more, including the deleting form `rename OLD {}`. Two more bugs surfaced and fixed while wiring this in: (1) the pre-existing `rename_target_spans`/`WorkspaceCommandLink.target_span` mechanism already produced an edit for this same token via a separate path, causing an exact duplicate `WorkspaceTextEdit` in cross-document rename — removed outright (strictly narrower than the new reference, mirroring `interp alias`'s existing `target_span: None` precedent) and replaced with a new `AnalysisResult::rename_offsets` field (mirroring `alias_offsets`) to keep the nested-link check working; (2) `rename puts myputs` started falsely W123-flagging its own `puts` word, because the recorded deletion offset sat textually before the new reference in the same statement, and `registry_name_deleted_before`'s ordering check read that as "deleted before this call" — fixed by anchoring the deletion offset just past `OLD`'s own token instead of the statement's start. |
| `ccec425` | **main-wave** idx 46 (high, partial) | `handle_source_command` recorded `is_literal: false` unconditionally for any `source` path containing `$`/`[` — even the audit's own "simplest possible case" control, a straight-line same-file `set p "e.tcl"; source $p` with zero branches and zero external input. An untracked `source` edge isn't just a missed resolution: `refresh_source_rehoming` silently drops it, so the sourced document keeps its default `::`-only analysis while its definitions still leak into every caller's go-to-definition as if unconditionally global — the finding demonstrates both a false negative (the real, qualified call resolves to 0 locations) and a false positive (a nonexistent name resolves confidently to a declaration) on the same file pair. Fixed by trying the same last-write-wins constant-string lattice already proven for `rename`'s OLD/NEW words (idx 3) before falling back to the existing dynamic path; `resolve_rename_arg` renamed to `resolve_dynamic_word` (already fully generic, nothing rename-specific about it) and shared by both callers. Covers a bare `source $p` and a multi-token concatenation (`source ${base}.tcl`) alike. Deliberately out of scope, pinned with a dedicated FN test: a variable wrapped inside a `[file join/dirname ...]` command substitution (`fold_interpolation_single` rejects any word containing `[` by design), and any variable whose constant value originates in a *different* file (the corpus's own primary ehuddle.tcl shape, `variable edir [file dirname [file normalize [info script]]]` in one file consumed via `source [file join $edir ...]` in another) — both need interprocedural constant propagation across files, the same class of follow-up idx 3/116/120 already deferred. |
| `03aef3d` | **main-wave** idx 52 (high) | A class created via `oo::class create` then extended by every method through a *separate*, later `oo::define ClassName { ... }` block (the real corpus shape — `ticklecharts::chart`) broke go-to-definition/references/rename/hover for any `my methodName` internal-dispatch call or class-member lookup landing inside that separate block; external `$obj method` dispatch was unaffected (a different resolution path). Root cause: `handle_oo_define_command` reused the existing `ClassDef.body_span` unchanged when extending an already-known class, so it stayed pinned to the original `oo::class create` block — a single `Span` can't represent two textually disjoint regions. Found the *same* bug independently duplicated across ~7 "is this offset inside a class body" containment checks scattered through `tcl-lsp-core` (`definition.rs` x3, `references.rs`, `rename.rs` x3, `hover.rs`, `call_hierarchy.rs`, `implementation.rs`, `type_definition.rs`). Fixed with a new `AnalysisResult::class_body_spans: Vec<(String, Span)>` (multi-span analogue of `ClassDef::body_span`, mirroring `proc_declaration_sites`' plumbing) populated at all four class-creation sites (`oo::class create`, `oo::define`, `snit::type`, `itcl::class`), plus one canonical `enclosing_class_at` helper every duplicated check now delegates to instead of keeping its own copy (one, in `implementation.rs`, was simply identical and deleted outright). `ClassDef.body_span` itself is untouched — still the class's primary/creation-site span for hover-on-the-class-name / document-symbol / rename-target purposes. Also discovered and explicitly left alone (a separate, pre-existing gap, not part of this finding's own claim): `hover()` has no resolution path for a plain `my methodName` call at all, only for a *linked* bareword sibling call or `constructor`/`destructor` — reproduces identically in a single, unsplit class body. |
| `741f9ba` | **main-wave** idx 56 (high) | A proc installed directly into `::oo::Helpers` (the documented "TclOO Tricks" idiom — nico-robert/ticklecharts installs `classvar`/`callback` this way, 29 real call sites) is bare-callable from every TclOO method body via TclOO's own fixed runtime namespace path; go-to-definition/hover already resolved it (a lenient, namespace-gate-free fallback), but find-references/rename share one namespace-gated match function (`invocation_references_named`) whose `call_ns == target_ns` check has no way to represent a second, non-lexical search member — `call_ns` is a single accumulated string (`"::"` for a method body), never `"oo::Helpers"`. A rename applied verbatim would crash the next invocation at runtime ("invalid command name") while the tool reported it as complete and safe. Fixed with a new `innermost_scope_reaches_oo_helpers` scope-chain query (mirrors `command_resolution_namespace_at`'s own traversal) consulted from the gate to accept `target_ns == "oo::Helpers"` specifically when the call site is inside a method body — a fixed `TclOO`-implementation constant, not a per-command special case (unlike the real `namespace path` command, already modelled separately via `namespace_paths`). |
| `65dda01` | **main-wave** idx 63 (high, partial) | Three findings bundled under one idx. (1) The primary claim ("go-to-definition AND find-references both zero-result... on the real unmodified corpus file") is idx 52's own `enclosing_class_at`/`class_body_spans` root cause — already fixed on this branch; confirmed independently with idx 63's own `foo::widget`/`bar`/`baz` repro and pinned as a permanent regression test, no new production change needed. (2) Independently, a `my methodName` call inside a `switch` arm body was invisible to find-references (and, transitively, rename, which reaches `my`-dispatch sites through the same `references::method_references_for_class`) — the real corpus's "assigned `Add`-dispatcher" idiom (`switch ... { barSeries { my AddBarSeries {*}$args } ... }`) trips this: `scan_my_method_region`'s hand-rolled re-segmentation scan already recurses into `[...]` command substitutions to find nested `my` calls, but a switch arm's braced body is neither a substitution nor reachable any other way. Fixed by extracting `switch`'s arm-boundary logic (previously analyser-internal, both of `switch`'s argument forms) into a new shared `Analyser::switch_arm_bodies`, consumed from `scan_my_method_region` to descend into each arm the same way it already descends into `[...]` regions; `rename()` needed no separate change, since it already funnels through the same scan. (3) Deliberately out of scope, per the finding's own "secondary, more tangential" framing: a false W001 diagnostic when a locally-defined class shadows a hardcoded ticklecharts registry entry — unrelated production code (`widget_command.rs`) and root cause (registry/local-class collision) from (1)/(2). |
| `134c31c` | **main-wave** idx 68 (high) | Find-References/Rename never unified a proc's `global`/`variable`/`namespace upvar` alias with the canonical cell it points at — only *other aliases* of a target were ever found (`collect_alias_spans` matches on `link_target`, but a plain `set`/declaration is never given one). Real corpus shape: nico-robert/pix's `isEqual`/`tolComp` (`global tolComp` inside the proc, defaulted if unset, overridden by a top-level `set ::tolComp` before each call) — Rename from either side previously rewrote only its own half, silently decoupling the proc's default from the caller's override. Fixed with two new bidirectional `tcl-compiler::analyser::scope` helpers: `qualified_name_for_var_decl` (the reverse of the pre-existing `lookup_var_in_namespace` — given a declaration's span, finds the qualified name an alias would target it by) and `lookup_var_by_qualified_name` (a superset of `lookup_var_in_namespace` that also matches a *literal* `::`-qualified `set`'s verbatim-stored key — discovered empirically that `define_var` never re-qualifies a name it's given, so `set ::tolComp val` stores `"::tolComp"` as its own key, not the bare tail `variable`/`global` aliases use; a new `lookup_var_by_literal_qualified_name` fallback walk covers that spelling). `linked_var_reference_spans` now folds the canonical cell in when querying from an alias, and every alias in when querying from the canonical cell, either direction, either spelling. |
| `d5e4d65` | **main-wave** idx 70 (high) | `foreach varList1 list1 ?varList2 list2 ...? body` (the parallel/lock-step multi-list form, arity-validated by the registry's own `foreach` spec) only ever bound the *first* varList — every subsequent varList/list pair's names were silently dropped. Real corpus consequence on nico-robert/pix's `docs/pixdoc.tcl`: `foreach dirName {...} name {...} {...}` never bound `name` at all, so go-to-definition on `$name` inside the loop body fell through to a coincidentally same-named but wholly unrelated *later* `foreach name {...}` 300+ lines away (`foreach` introduces no new analyser scope, correctly modelling Tcl's lack of block scoping, so all top-level `name`s share one flat lookup table). Fixed by looping `for i in (0..args.len()-1).step_by(2)` binding every `args[i]` varList, mirroring the arity spec's own stride of 2. Verified against real tclsh9.0/8.6 that two sequential top-level `foreach name {...}` statements genuinely share one global storage cell — so the fully correct fixed `references()` set unifies both loops rather than artificially excluding the second. |
| `2339d4a` | **main-wave** idx 71 (high) | `textDocument/references` dropped every call site in the *same* document a query was issued from whenever that document has no local declaration to anchor on (a proc reached only through a `source`d-in or workspace-sibling declaration) — `cross_document_references` unconditionally excludes the current document from its workspace-index lookup, assuming the (empty, in this case) single-document pass already covers it. Real corpus shape: nico-robert/pix's `test_context.test` sources `data_b64.test` then calls `isEqual` bare, twice; both of its own calls were invisible to find-references. Mirrored the identical fix pattern rename's "M8" consumer-document fallback already uses (resolve through the workspace oracle with an empty exclude-URI): `cross_document_references`'s body is now shared via a new `gather_reference_targets(..., exclude_uri)` helper, and a new sibling `workspace_resolved_references` calls it with `exclude_uri = ""` instead of the current URI, used whenever the single-document pass finds nothing local to anchor on. The `.test`-extension half of this finding was already fixed by idx 10. |
| `0bde16e` | **main-wave** idx 76 (high) | The finding's own headline hypothesis (LSP guessing the wrong class among structurally-similar TclOO classes for a genuinely dynamic `switch`-dispatched call) is REFUTED — the LSP already correctly abstains there. Tracing why uncovered a distinct CONFIRMED gap on the exact same class (nico-robert/tomato's real classes all use idx 52's two-block `oo::class create` + separate `oo::define` convention): a definite, single-target `my methodName` internal-dispatch call had no hover at all, even though go-to-definition/find-references already resolved it (cursor-shape-driven, via `enclosing_class_at`/`method_dispatch_definition`) — hover only had the word-match-driven `class_member_hover_text`, gated on `ClassDef::linked_members` (idx 113's `oo::Helpers::link` idiom only), which a plain un-linked `my` call never populates. Fixed with a new `inst == "my"` branch in `hover_with_profile`, mirroring `instance_method_definition`'s existing one, resolving via `enclosing_class_at` and rendering through the existing `obj_method_hover_text`. |
| `51a630f` | **main-wave** idx 77 (high) | The entire CFG/SSA dataflow diagnostic family (W210 read-before-set, W211 unused-variable, W220 dead-store, W233, interval-bounds, unused-param, constant-branch) silently never ran on any TclOO/snit method body — `emit_cfg_ssa_diagnostics_with_cu`'s per-function loop only ever iterated `cu.procedures`, never `cu.methods`. Real corpus crash: nico-robert/tomato's `Vector3d.tcl::* {type}` reads `$other`, a variable belonging to a sibling method, never bound in `*`'s own scope — tclsh8.6/9.0.4 both crash with `can't read "other"` the instant `*` runs on an object operand; the identical shape inside a plain `proc` already fired W210. Fixed with a new `emit_method_body_diagnostics` loop over `cu.methods`, threading two suppression sets into `extra_known_defined`/`cross_event_vars` (both verified empirically against false positives): `MethodDef::instance_vars` (TclOO auto-binds class-level `variable` names in every method with no visible statement in the body) and `MethodDef::params` (a method's own params — `emit_read_before_set_diagnostics`/`emit_return_phi_undef_w210` both special-case a real parameter via a separate `ir_module.procedures` lookup a method's qualified name is never in). Full existing tcl-compiler suite (4540 tests) re-verified clean, confirming no false positives across the corpus of existing TclOO-shaped tests. |
| `7115bc8` | **main-wave** idx 84 (high, partial) | `namespace ensemble configure` (as opposed to `create`) was entirely invisible to `handle_namespace_ensemble` (`if args[1] != "create" { return; }`), so the real `tk/library/systray.tcl` (and `print.tcl`/`fileicon.tcl`/`accessibility.tcl`) idiom of splicing `systray`/`sysnotify` into the pre-existing, registry-builtin `tk` ensemble via `namespace ensemble configure tk -map [dict merge [namespace ensemble configure tk -map] {systray ::tk::systray sysnotify ::tk::sysnotify::sysnotify}]` drew 5 false W001 diagnostics and risked wrong go-to-definition/hover navigation to an unrelated same-tail-name decoy. Fixed: `handle_namespace_ensemble` now also accepts `configure NAME ?opts?` (NAME resolved the same namespace-aware way a `-map` target already is, not a blind prefix); a new `dict_merge_literal_tail` (mirrors idx 110's `cmd_fragments`/`descend_token`/`segments_from_tree` idiom) narrowly recognises the real `dict merge ARG {literal}` splice shape and extracts the literal tail's own pairs, since naively word-splitting the *whole* dynamic value's raw text (as the pre-existing per-element-only `is_dynamic_word` guard would have allowed) produces actively wrong subcommand/target pairs and spurious command references — caught by a dedicated safety test before shipping; a new `statically_mapped_ensemble_subcommand_known` (validity.rs) wires the already-populated `ensemble_subcommand_targets` (idx 106) into W001 for the first time, alongside the pre-existing `dynamic_ensemble_subcommand_known` (idx 105's `implementation_namespace` mechanism, confirmed inapplicable to `tk systray`'s genuine 2-level nesting). Deliberately not fixed, and confirmed via a probe to be a separate, general, pre-existing limitation rather than idx-84-specific: the finding's 3rd-word case (`tk systray create`/`configure`/`exists`/`destroy`) still risks wrong navigation, since `instance_method_at_cursor`'s ensemble-subcommand check requires exactly one token immediately before the cursor word and can't engage for a 3-word shape at all — the identical risk already reproduces today against the long-registered `tk busy <subcommand>` entry. See the dedicated note after this table. |
| `99cf07f` | **main-wave** idx 86 (high) | `proc`'s own name argument never attempted constant-folding at all (unlike `rename`'s operands, fixed for idx 3), so `proc ::$wtype {...}` with `wtype` a known constant registered under the raw, garbled text instead of the resolved name. The real `tk/library/accessibility.tcl` idiom compounds this inside `foreach wtype {button entry ...} { rename ::$wtype ::tk::accessible::orig_$wtype ; proc ::$wtype {args} {...} }`: the loop variable binds a *different* value each iteration, a fact the single-value `const_strings` scope map can't represent generally (`foreach` shares one flat cell for the whole loop — confirmed against tclsh, no per-iteration scope to key a value under). Go-to-definition on a post-loop `button` call site fell through to the stale, pre-rename `proc button` declaration; the outline showed a garbled `Function ${wtype}(args)` entry instead of the real per-element wrapper names. Fixed in two parts: `handle_proc_command` now constant-folds its name argument via the same `resolve_dynamic_word` path `rename` already used; `handle_foreach_command` recognises the narrow, fully-literal single-pair shape (mirroring the idx 110 precedent of recognising one specific idiom rather than generalising the underlying lattice) — it binds the loop variable to the first literal element before its one normal body walk (letting the existing constant-fold resolve that iteration for free), then narrowly re-dispatches just the body's own `rename`/`proc` sub-commands once per remaining literal element, the two handlers whose resolution go-to-definition/references/symbols actually depend on. Every other command in the body keeps the single evaluation the normal walk already gives it, so diagnostics/scope entries for anything else aren't duplicated; a bounds guard (mirroring `cmd_fragments`'s own) keeps a direct unit-level `handle_foreach_command` call with a synthetic out-of-bounds span safe. W113's message now names the resolved proc, not the raw dynamic text, now that a dynamic name can resolve. |
| `20304ef` | — | **Not campaign work** — a rebase of this branch onto `origin/rust`'s newer tip (8 upstream commits, 9 of the resulting 58 replayed commits needed manual conflict resolution — genuine overlapping work, merged rather than either side picked blindly) surfaced a real cross-commit semantic-drift bug via the post-rebase full validation gate, unrelated to any specific tracked finding: `rename_method_in_class` hardcoded `is_classmethod: false` in its call to `references::method_references_for_class` — correct reasoning at the time origin/rust wrote it ("only ever reached via an external `$obj method` call site... always an instance-method receiver"), invalidated once this branch's own idx 120 fix (already on this branch pre-rebase) extended `receiver_instance_class` to also resolve a bare class-name receiver. Fixed by threading through the caller's own `receiver_method_bucket` determination (added by idx 120 for the identical disambiguation elsewhere) instead of the now-stale assumption; `class_q`/`method`/`is_classmethod` bundled into one tuple parameter to stay under the 7-argument clippy lint. |
| `7d476f5` | **main-wave** idx 90 (high) | `tcl::OptProc name optlist body` (the `opt` package's automatic-option-parsing proc definer — `tk/library/safetk.tcl` redefines a throwaway stub this way) had no `AnalyserHookId` at all, so `all_procs` kept the stub's `{}`-arity `ProcDef` for every real call: false E003 on every call, wrong hover/go-to-definition/references/document-symbol signature. Runtime mechanism (tclsh9.0/8.6-verified): installs a real proc via `uplevel 1 [list ::proc $name args ...]` — the Tcl-level formal is always the single `args` catch-all, `optlist` itself never arity-checked; `optlist`'s own descriptors are bound as LOCAL VARIABLES by `::tcl::OptKeyParse`, a leading `-` on a flag descriptor stripped for the bound name. Fixed with a new `AnalyserHookId::OptProc` + registry spec gaining `analyser_hook`/`arg_roles`/`body_kind`/`command_table_effect`/proc-matching traits; new `handle_opt_proc_command` (mirrors `handle_proc_command`'s glue as a largely separate function — the arity/local-binding stories diverge too much for a shared abstraction) records `ProcDef.params` as `[args]` always, while a combined `args` + dash-stripped-optlist-locals list feeds the body's own scope/hover. Three more gaps found and fixed while wiring this in: (1) `resolve_analyser_hook` blocked *any* `::`-qualified head unconditionally — correct for existing bareword-only hooks (`proc`, preserved), wrong for this brand-new, namespaced one, since real corpus code commonly writes it `::tcl::OptProc` fully qualified (now also fixes the identical latent gap for `::oo::define`/`::oo::objdefine`); (2) the synthetic `args` local (no literal `args` word is ever written) needed a definition-span anchor colliding with neither the proc name nor `optlist`'s own descriptor sub-spans — a zero-width span at `optlist`'s own opening brace; (3) the separate `signature_scan` background/cross-file indexer also dispatches on `Traits::DEFINES_PROCEDURE` (needed for hover), whose shared `handle_proc` would have recorded `optlist`'s own words as the cross-file arity too — new `handle_opt_proc` (keyed off the same `AnalyserHookId::OptProc` stamp) fixes that independently. |
| `0c03128` | — | **Not campaign work** — `tcl::prefix` (added by idx 9's own fix, `51d0a35`) was never folded into `editors/zed/src/generated/tcl_commands.json`; the full workspace test suite's own `gen_editor_catalogs::tests::committed_catalogs_match_generated` gate catches exactly this drift and was failing. Regenerated via `cargo xtask gen-editor-catalogs`. |
| `7953d5e` + `ef36c73` | **main-wave** idx 95 (high) | `tk.tcl:594-596`'s `$w ${dir}view scroll ...` (a subcommand synthesized by string-concatenating `$dir` with literal `view`) itself correctly abstains from any false "unknown subcommand" diagnostic — but Rename Symbol on `dir` corrupted the source: `tcl-lexer`'s `Var` token span for a non-degenerate `${name}` form deliberately stops one byte short of the closing `}` (`${a{b}}` names `a{b}`, whose content can itself legitimately end in `}`, so the span convention leaves the outer delimiter unconsumed rather than risk misreading it as content), which made the raw span unsafe to reuse as a rename *edit range*: `build_var_ref_replacement` already emits a self-closed `${new}` string, so replacing only the short span left the source's own original `}` sitting right after it, corrupting `${direction}` into `${direction}}view` — real tclsh8.6/9.0 both fail to even parse the enclosing proc ("extra characters after close-brace"). Fixed with a new `var_ref_edit_span` helper that extends the span to include the closing brace only when the source confirms it's actually there and unconsumed, mirroring `SourceMap::token_text`'s own degenerate-`${}`-empty-name check so it never mis-fires on a span that already legitimately includes the brace. Pushed as two commits (the outage described in §8 blocked a local `git push`, so this landed via the GitHub API directly from locally-verified file contents): `7953d5e` (the e2e regression tests) and `ef36c73` (the core `var_ref_edit_span` fix + its own unit tests) — zero diff between the two once reconciled locally. |
| `959bca8` | **main-wave** idx 94 (high) | A bare `$var` body of any `ArgRole::Body`-marked argument (`eval $cmd`, `uplevel #0 $cmd …`) dynamically evaluates $var's value as a script at runtime — the same "value is a command prefix" shape `{*}$cmd` already gets via `head_expanded`, just reached through a different syntactic position (a command's body argument, not its own head). Real corpus shape (`tk/library/tearoff.tcl`'s `MenuDup`): `set cmd [list menu $dst -type $type]; ...; eval $cmd`. `command_invocations` never saw this at all — `analyse_body` only ever recurses a literal `Str` body, so hover/go-to-definition resolved via their independent cursor-token walk while references/rename silently missed the call site. Fixed with a new `TokenType::Var` branch in `dispatch_one_body_argument` — generic across every `ArgRole::Body` argument, not eval/uplevel-specific by name — registering a `ConstDispatchSite` (`head_expanded: true`) for the existing CFG/SSA `settle_const_dispatches` machinery (issue #945 faults 1–2) to resolve. That machinery's own `value_provenance.rs` had a separate, narrower gap the finding's own minimal repro exposed: it never folded a `[list W1 W2 ...]` value into a constant at all — fixed with a new, deliberately narrow `fold_literal_list_call` (every element must be a plain literal word) whose first element's own span anchors `literal_span` directly, giving a fully rename-safe result (real tclsh9.0/8.6-verified: renaming `greetD` in `set cmdD [list greetD World]; eval $cmdD` correctly rewrites just that one word, and the transformed script still executes, printing "D World"). Computing that span surfaced a second general bug in the same family as idx 95: `Cmd`-token spans (a `[...]` substitution) also deliberately exclude their own closing `]`, so the existing `word_content_base` helper underflows for a `[list ...]`-shaped value — worked around locally by reading the token's own unaffected *start* offset directly. Empirically confirmed the finding's own speculative "second, compounding gap" (per-proc `FunctionUnit`s not reaching `settle_one_site` with usable SSA data) does not exist — pinned as a regression test, not a fix. Also corrected a stale pre-existing test (`references_do_not_treat_a_dynamic_bareword_body_as_a_static_call`, now `references_resolve_a_constant_var_body_through_its_real_value_not_its_literal_text`) whose own assertion that `if {1} $cb` (with `set cb foo`) must never resolve was contradicted by real tclsh9.0/8.6 (both call `foo`). |
| `e0ebda9` | tcllib idx 121 | `record_instance_creation`/`class_from_constructor_subst` only recognised a literal class-name bareword at a `new`/`create` constructor call — tcllib's `httpd/httpd.tcl:1970-1994` instead flows the class name through a single, unconditional `set` one line earlier (`set class ::Derived; set obj [$class create NAME]`), so `instance_classes` never bound `obj`, leaving hover/go-to-definition/references on a later `$obj method` call silently empty (tclsh9.0/8.6-verified the dispatch itself works). Fixed by extending idx 94's `ConstDispatchSite` settle-late discipline: a new `PendingInstanceClassSite` is recorded when a constructor's class head is a plain `$var`, then settled once the CFG/SSA `CompilationUnit` exists via `value_provenance::const_contributors`, binding `instance_classes` only when every reaching definition agrees on one known class (abstains soundly on a branch-ambiguous or genuinely dynamic value). The settle call must run *before* `emit_var_command_diagnostics` in the same pass (unlike `settle_const_dispatches`, which only feeds `command_invocations`) — that pass reads `instance_classes` to suppress W307/validate W308, and settling too late left a newly-discovered W307 false positive on the very call site the fix was meant to clear. That W307/W308 gate turned out to have its own independent copy of the same gap: `harvest_constructor_object_types` (the SSA type-lattice's separate constructor scanner) hit the identical `$var`-headed limitation — extended with a new `harvest_indirect_constructor_class` reusing the same `class_var_head_constructor_subst` shape-parse and `const_contributors` resolution, so the type lattice agrees with hover/definition on the exact same dispatch. |
| `c031d1d` | tcllib idx 122 | W210 false-fired for an `upvar`-populated variable whenever the writing proc is called from inside a `while`/`if` **condition** rather than a bare statement — real tcllib repro `cmdline.tcl`'s `getopt`/`getKnownOpt` chain, `while {[set err [getopt argv $opts opt arg]]} { ... }` (tclsh9.0/8.6-verified the condition's own substitution, including the write, completes before the body runs). Three compounding gaps: (1) `condition_command_out_vars` only recognised 4 hardcoded builtins (`catch`/`scan`/`gets`/`regexp`), never the general known-upvar-proc/global-write-proc resolution every other embedded-substitution site already gets — fixed with a new `CfgBuilder::condition_out_vars` unioning both, used by `lower_if`/`lower_while`. (2) A `while`/`for` whose condition is *purely* a command substitution freezes the whole loop into an opaque `Statement::Barrier` (no `defs` field at all) instead of calling `lower_while`/`lower_for` — its own `uses_of` textually scans the un-lowered condition+body text for `$var` reads, but nothing populated defs, so fix (1) alone couldn't reach it; fixed by pushing a synthetic `<cond>` `Statement::Call` carrying `condition_out_vars`'s result immediately before the barrier. (3) `upvar_defs_from_text`/`global_write_defs_from_text` only checked the outermost command's own first word, so a wrapping command around the real call (`set err [getopt ...]`, the actual tcllib shape) hid it — a pre-existing gap in the general (non-condition) mechanism too, confirmed via a plain `set x [set err [getopt ...]]` statement; fixed by recursing into each matched token's own inner text, bounded by a new `MAX_EMBEDDED_SUBST_DEPTH` guard. |
| `72a0d6d` | tcllib idx 18 | W210 false-fired for a variable an `upvar`+`uplevel` "custom control structure" proc populates, whenever the analyser only knows about that proc's own *caller* — real tcllib repro `modules/page/util_flow.tcl`'s `::page::util::flow`, a plain wrapper `proc` that hands its own `fvar`/`nvar`/`script` params unchanged to a snit constructor one call-hop away, which does the actual `upvar`/`uplevel` work (tclsh9.0/8.6-verified the idiom runs cleanly). `detect_upvar_procs` only ever scanned each proc's own body for *direct* `upvar` statements, with no transitive notion that a pass-through wrapper is itself an upvar-write target one frame further out. Fixed with a new `upvar_info::transitive_upvar_info_for` (+ `walk_script_for_transitive_calls`/`walk_stmt_for_transitive_calls`, mirroring `collect_upvar_targets`'s own body-walk shape): for a candidate proc not yet known, matches any `Statement::Call` to an already-known upvar proc whose call-site args feed the callee's own upvar-source parameter with a bare `$paramName` naming one of the candidate's own parameters unchanged (a callee with an unresolvable target widens the candidate the same way). `detect_upvar_procs` runs this to a fixed point after its original scan, so an N-hop wrapper chain resolves too, bounded by proc count. Deliberately out of scope: the real `page::util::flow` reaches its constructor via `uplevel 1 [list ... ]`, one layer more indirect than a plain proc-to-proc call — this fix covers the direct `Statement::Call` shape only, matching what the finding's own "minimal repro confirmed" language implies was actually verified. |
| `ef53d47` | tcllib idx 125 | `VarReferenceScanner::scan_script_uncached` re-tokenised already-extracted value/argument text with the general-purpose top-level lexer, which applies command-word rules (a bare `{…}` is a fresh, non-substituting brace-quoted word) to text that actually originated *inside* a double-quoted string, where braces are ordinary characters and substitution continues straight through them — real tcllib repro `htmlparse.tcl`'s `::htmlparse::parse`: `set vroot hmstart; eval "$cmd {$vroot} {} {} {$html}"` (tclsh9.0/8.6-verified `{$vroot}` is an ordinary substitution, same as the bare `$cmd`/`$html` beside it). The re-scan swallowed `$vroot` into literal text instead, false-firing W220 "never read" while hover/definition/references (reading the primary parser's own quote-context-tracking CST) agreed it's live. Fixed with a new `Lexer::as_quoted_body` builder (`tcl-lexer`, seeds `in_quote = true` from byte 0, reusing `parse_quoted`'s own dispatch) and a `var_refs.rs` `scan_tokens` mode split: the outermost scan (every external `scan_word`/`scan_script` call) uses quoted-body dispatch, while a nested `[…]` substitution's own content — genuinely a fresh Tcl command, where `[foo {$bar}]`'s `{$bar}` really is a literal non-substituting argument — keeps the ordinary top-level dispatch. Surfaced two latent false positives the bug had been accidentally masking: `itcl::class`/`snit::widget`/`snit::type`/`snit::widgetadaptor` were missing the registry's `body_kind: Structural` classification `oo::class` already carries, so their body argument fell through to the same flat, now-more-accurate scan and (with no suppression path at that enclosing-scope layer) W210'd every `$this`/instance/`common` variable reference inside — fixed by adding the missing classification to all four specs. |

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
  idx 121 (dynamic TclOO constructor class) turned out **not** to need this
  mechanism — fixed instead by consulting `const_contributors` directly
  (a new `PendingInstanceClassSite`/`settle_pending_instance_class_sites`,
  mirroring idx 94's `ConstDispatchSite`; see §3's `e0ebda9` row), exactly
  the alternative this note used to flag as more likely. Still a candidate
  for a genuinely nameless/unrepresentable construct.
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
| `06-main-audit-results-COMPLETE-105of105.json` | **Complete.** All 105 main-wave findings (idx 0–104) differentially audited: 85 CONFIRMED, 20 REFUTED. **Triaged** — see §6b for the severity/corpus/feature breakdown, the full up-to-date fixed/remaining list, and priority-ordered tables (kept current there; not duplicated here to avoid drift). |
| `07-remaining-tcllib-findings-14.json` | The 14 tcllib CONFIRMED findings not yet fixed (full detail: summary, failure_scenario, oracle_output, lsp_output, root_cause_hint, repro_path — repro files themselves are gone, scratchpad-only, but the hints are detailed enough to rebuild a repro in minutes). |
| `08-research-plans-PARTIAL-8of14.json` | **Partial — 8 of 14 done.** Refined, current-code-verified fix plans for 8 of the 14 remaining tcllib findings (idx 3, 9, 105, 106, 110, 113, 116, 120), produced by a research-only agent fan-out (no file edits) that re-checked each root-cause hint against the *current* (post-merge) code and proposed concrete changes + test scenarios. idx 24, 128 do not have refined plans yet — use `07`'s `root_cause_hint` field directly for those, which is still quite detailed. (idx 18, idx 121, idx 122, and idx 125 are now fixed — see §3/§6a.)

---

## 6. Remaining work, prioritized

### 6a. tcllib — 2 CONFIRMED findings, not yet fixed

**idx 105, 106, 3, 110, 113, 9, 120, 116, 121, 122, 18, and 125 are done**
(fixed, tested, pushed — see §3's `25d6a09` / `2c48bcc` / `c022921` /
`264cfdf` / `183baef` / `78ea6e2` / `af8c3d5` / `a824ff8` / `e0ebda9` /
`c031d1d` / `72a0d6d` / `ef53d47`
rows); removed from the table below. 2 remain, neither with a refined plan
left — use `07`'s `root_cause_hint` directly for both.

All in `data/07-remaining-tcllib-findings-14.json`. Suggested order (by
severity):

| idx | severity | feature | one-line summary | refined plan? |
|---|---|---|---|---|
| 128 | medium | package_loading | `PackageResolver::parse_pkg_index` ignores `if {...} { return }` reachability guards in `pkgIndex.tcl`, over-suppressing W123. | no |
| 24 | medium | autoindex | `hover()` never falls back to the cross-document/autoload resolution tiers that `definition()`/`references()` already use. | no |

Each of these follows the exact same playbook as the 12 already-fixed
findings: read root_cause_hint (no refined plan remains for either of these
2) → confirm
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
REFUTED.** Of the 85 CONFIRMED: **24 fixed** (idx 0, medium — PR #1068
on `claude/commandregistry-compiler-fixes-tshu8d-quickfixes`; idx 61,
critical — §3's
`438e56f`; idx 9, high — §3's `51d0a35`; idx 10, high — §3's `2330862`;
idx 18, high — §3's `1f5fe71`; idx 29, high, same root cause as idx 18 —
§3's `d218463`; idx 31, high — §3's `89b75a5`; idx 32, high — §3's
`8646964`; idx 33, high, same root cause as idx 18 — §3's `1135d75`; idx
39, high — §3's `8827a17`; idx 46, high, partial — §3's `ccec425`; idx
52, high — §3's `03aef3d`; idx 56, high — §3's `741f9ba`; idx 63, high,
partial — §3's `65dda01`; idx 68, high — §3's `134c31c`; idx 70, high —
§3's `d5e4d65`; idx 71, high — §3's `2339d4a`; idx 76, high — §3's
`0bde16e`; idx 77, high — §3's `51a630f`; idx 84, high, partial — §3's
`7115bc8`; idx 86, high — §3's `99cf07f`; idx 90, high — §3's `7d476f5`;
idx 95, high — §3's `ef36c73`; idx 94, high — §3's `959bca8`),
**61 remaining**.

**2026-07-30 update — PR A3 (`claude/commandregistry-compiler-fixes-tshu8d-mathfunc`)
fixed six more, all tier 2:** idx 24, idx 30, idx 48, idx 54, idx 103, and
idx 104 — see the "tier-2 findings fixed by PR A3" subsection after the
tier-2 table for what each fix does and which of the audit's claims were
already stale. idx 81 was **re-verified as already fixed** by the
mathfunc-aware W123 / W002 work that landed on `rust` after the audit ran
(`is_mathfunc_call` + `is_known_mathfunc_in_dialect`) and is counted with
them. That makes **30 fixed, 55 remaining**.

**By corpus** (confirmed only): ticklecharts 20, tk 17, argparse 10,
SpiceGenTcl 10, tclopt 13 (6+7, split across two inconsistent corpus-label
strings in the raw data — same corpus), tomato 7, pix 8.

**By feature** (confirmed only): tricky_indirection 14, tclOO 13,
namespaces 11, proc_args 10, upvar 7, source 6, tcl_mathop 5, rename 4,
package_loading 3, uplevel 3, tracing 3, aliasing 2, safe_interp 2, eval 1,
autoindex 1.

#### Priority tier 1 — critical + high (24 findings, 23 already fixed)

Fix these first — each is either data-loss-risk (a rename that silently
breaks the program, idx 61) or a full-zero-results failure of a core
navigation feature (go-to-definition/references/hover returning nothing)
on a common real-world idiom. `severity`/`summary` are the audit's own
classification; read the finding's full `root_cause_hint` in the JSON
before starting each — the one-line summaries below are index-and-locate,
not a substitute.

| idx | severity | corpus | feature | one-line summary |
|---|---|---|---|---|
| 61 | critical | ticklecharts | uplevel | **FIXED** (`438e56f`) — unbraced `if`/`uplevel` body bareword call sites invisible to references/rename. |
| 9 | high | argparse | tcl_mathop | **FIXED** (`51d0a35`) — variable bareword declarations (proc params, `catch` result-vars) unresolved by definition/hover/references/rename; `tcl::prefix` had no `CommandSpec`. |
| 10 | high | argparse | source | **FIXED** (`2330862`) — `.test` (tcltest) files were invisible to the background workspace scan, so cross-document references/rename missed call sites living in them. |
| 18 | high | SpiceGenTcl | namespaces | **FIXED** (`1f5fe71`) — a bareword class/proc name reachable only through a wildcard `namespace import NS::*` never resolved (in-doc or cross-document). |
| 29 | high | tclopt | namespaces | **FIXED** (already resolved by `1f5fe71`'s idx 18 fix — same root cause; pinned with dedicated regression tests in `d218463`). |
| 31 | high | tclopt | tricky_indirection | **FIXED** (`89b75a5`) — references/rename from a shadowed duplicate proc declaration silently dropped cross-file callers. |
| 32 | high | tclopt | tricky_indirection | **FIXED** (`8646964`) — a class's 2nd+ `variable` statement silently discarded the 1st's names instead of accumulating. |
| 33 | high | tclopt | tricky_indirection | **FIXED** (already resolved by `1f5fe71`'s idx 18 fix — same root cause; pinned with a dedicated test in `1135d75`). |
| 39 | high | tclopt | rename | **FIXED** (`8827a17`) — `rename`'s own `OLD` word was omitted from references/rename; also fixed a duplicate-edit bug and a W123 false positive found while wiring it in. |
| 46 | high | ticklecharts | source | **PARTIALLY FIXED** (`ccec425`) — a same-file constant-variable `source` target now resolves; a variable wrapped in `[file join ...]` or originating in a different file still doesn't (needs interprocedural constant propagation, pinned with an FN test). |
| 52 | high | ticklecharts | tricky_indirection | **FIXED** (`03aef3d`) — `my`-dispatch (go-to-definition/references/rename/hover) broke when a class's methods are added via a separate `oo::define` block instead of the original `oo::class create` body; the note's own `[self class]`/`[self method]` mechanic was REFUTED (no bug), found and fixed independently while building a faithful repro. |
| 56 | high | ticklecharts | tclOO | **FIXED** (`741f9ba`) — find-references/rename now reach a bare call to a proc installed directly into `::oo::Helpers` (`classvar`/`callback`); go-to-definition/hover already worked via a lenient fallback. |
| 63 | high | ticklecharts | proc_args | **PARTIALLY FIXED** (`65dda01`) — the primary zero-results claim was already idx 52's root cause (pinned with a regression test); the separate switch-arm find-references/rename gap it also exposed is now fixed; a tangential W001 registry-collision false positive is still open. |
| 68 | high | pix | proc_args | **FIXED** (`134c31c`) — Find-References/Rename never unified a proc's `global` alias with its own canonical `set` declaration (only *other aliases* of a target were ever found, never the target's own direct declaration); now bidirectional, qualified or unqualified spelling. |
| 70 | high | pix | tricky_indirection | **FIXED** (`d5e4d65`) — the parallel/lock-step multi-list `foreach` form only ever bound the first varList; every subsequent one (and its uses) was invisible, and on the real corpus file resolved to a coincidentally same-named unrelated later loop instead. |
| 71 | high | pix | source | **FIXED** (`2339d4a`) — find-references dropped every call site in the same document a query was issued from whenever that document has no local declaration (a proc reached only through a `source`d-in/sibling file); the `.test`-extension half was already fixed by idx 10. |
| 76 | high | tomato | tclOO | **FIXED** (`0bde16e`) — the headline "wrong class guessed" hypothesis is REFUTED (correct abstention); tracing it found hover had no resolution path at all for a plain `my methodName` call, unlike already-working go-to-definition/references. |
| 77 | high | tomato | tclOO | **FIXED** (`51a630f`) — the whole CFG/SSA dataflow diagnostic family (W210 and siblings) never ran on any TclOO/snit method body; the crash-causing unbound `$other` read now flags. |
| 79 | high | tomato | proc_args | **INVESTIGATED, NOT FIXED** (session of 2026-07-23) — see the dedicated note right after this table for why: the audit's own "definition/hover resolve, references/rename don't" framing is now stale (idx 113 already narrowed `lookup_class_member` to require `link`, so definition/hover now *also* abstain on this shape), but the underlying rename-safety risk it flagged is still real and a sound fix needs receiver-type inference this campaign doesn't have. |
| 84 | high | tk | namespaces | **PARTIALLY FIXED** (`7115bc8`) — `namespace ensemble configure` (as opposed to `create`) was invisible to the analyser, so the real `tk/library/systray.tcl` (and `print.tcl`/`fileicon.tcl`/`accessibility.tcl`) idiom of splicing `systray`/`sysnotify` onto the pre-existing `tk` ensemble drew 5 false W001s and risked wrong go-to-definition/hover navigation for the 2-word shape; both now fixed. The 3rd-word case (`tk systray create`/…) remains open — a separate, general, pre-existing limitation, not idx-84-specific; see the dedicated note after this table. |
| 86 | high | tk | rename | **FIXED** (`99cf07f`) — `tk/library/accessibility.tcl`'s `foreach wtype {...} { rename ::$wtype ::tk::ac... }` loop-generated rename/proc targets weren't tracked (nor was a plain `proc ::$wtype {...}` outside any loop — `proc`'s name never attempted constant-folding at all); go-to-definition on a post-rename call fell through to the stale original, and the outline showed a garbled `${wtype}`-named entry. |
| 90 | high | tk | safe_interp | **FIXED** (`7d476f5`) — `tcl::OptProc` (the `opt` package's automatic-option-parsing proc definer) had no `AnalyserHookId` at all, so `all_procs` kept a stub `{}`-arity `ProcDef` for every real call — false E003, and wrong hover/go-to-definition/references/document-symbol signature. |
| 94 | high | tk | tricky_indirection | **FIXED** (`959bca8`) — a bare `$var` body of an `eval`/`uplevel` call (`tk/library/tearoff.tcl`'s `MenuDup`: `set cmd [list menu $dst ...]; eval $cmd`) dynamically evaluates $var's value as a script, but `command_invocations` never saw it — references/rename silently missed the call site while hover/go-to-definition resolved via their independent cursor-token walk; a "complete" rename left the program calling a now-nonexistent name at runtime. |
| 95 | high | tk | tricky_indirection | **FIXED** (`ef36c73`) — `tk.tcl:594-596`'s `$w ${dir}view scroll ...` (a subcommand synthesized by string-concatenation) itself correctly abstains, but renaming `dir` corrupted the source: the `Var` token's lexer span for `${dir}` excludes its own closing `}`, so the rename edit range left the original `}` behind, producing `${direction}}view` — a parse error under real tclsh. |

**idx 79, in detail (investigated, deliberately not fixed):** the finding's
own repro is nico-robert/tomato's `Vector3d.tcl` `constructor {args}`, whose
"copy constructor" branch (`llength $args == 1` + a `TypeOf ... Isa
Vector3d` runtime guard) reinterprets `args` as a live object handle and
calls `[$args X]` / `[$args Y]` / `[$args Z]` — tclsh8.6/9.0.4-verified this
really dispatches. The audit found go-to-definition/hover resolved this call
site (crediting `definition.rs::lookup_class_member`'s then-current "cursor
inside class body, word matches any member name" fallback) while
references/rename — sharing a narrower core,
`references.rs::method_references_for_class` (only `my`-headed calls plus
`$var method` where `var` is tracked in `instance_classes` from a literal
`set var [Cls new/create]`) — never included it, so renaming `X` produced a
`WorkspaceEdit` touching only the declaration; applying it and running under
tclsh reproduced `unknown method "X"` on the copy-constructor call shape.

Re-verified this session (2026-07-23) with a from-scratch repro matching the
finding's shape exactly, at the `tcl-lsp-core::definition`/`references`/
`hover` unit-test level: **the audit's own "definition/hover resolve, this
one path, references/rename don't" framing no longer holds.** Idx 113 (an
earlier, independently-motivated, correct fix in this same campaign —
"require `oo::Helpers::link` before resolving bareword `TclOO` member
calls") added exactly the `ClassDef::linked_members` gate current
`lookup_class_member` (`rust/tcl-lsp-core/src/definition.rs`) checks before
matching *any* word against a class member — and this repro's class uses
plain `method X {}` / `method Y {}` / `method Z {}` with no
`oo::Helpers::link` anywhere, so `linked_members` is empty for it. Confirmed
empirically: `definition()`/`hover()` on `$other X` (the copy-constructor's
own dispatch site, an exact analogue of `Vector3d.tcl`'s `$args X`) now
return nothing at all — idx 113 already narrowed away the exact fallback
this finding's "definition/hover work" half depended on. All four
navigation features now abstain *consistently* on this shape, not
inconsistently as the audit observed.

That said, the finding's own most severe claim — **rename silently breaking
running code** — is untouched by idx 113 and remains fully live today:
renaming `method X` from its own declaration (definitely resolvable, always
was) still produces a `WorkspaceEdit` touching only the declaration, since
`method_references_for_class` still has no path to `$var method` calls
through an untracked receiver. A sound fix needs to answer "could `$other`
really be an instance of *this* class at this call site" — which requires
receiver-type inference this campaign's registry-driven, no-hardcoding
analyser doesn't have (unlike the bareword case idx 113 fixed, where real
Tcl's own `link`-or-error rule makes the answer statically knowable with no
type inference at all). Two directions were considered and rejected for this
session:

- **Reintroduce a broadened `lookup_class_member`-style match for
  `$var method`** (the audit's own suggested direction) — rejected: this is
  the *exact* class of unsound, receiver-blind heuristic idx 113 correctly
  removed for the bareword case, just relocated to explicit-receiver calls;
  extending it to *rename* specifically (not just navigation) means a wrong
  guess doesn't just navigate somewhere unhelpful, it silently rewrites
  unrelated code.
- **A `rename_blocked`-style safety gate** (mirroring issue #945 fault 1's
  established "provenance not fully writable → abstain outright" pattern):
  detect that a class's own method bodies contain an untyped-receiver
  `$var method` call matching one of its members, and refuse to rename that
  member rather than emit a silently-partial edit. Not attempted this
  session — the detection logic (walk every method body of the
  renamed-from class for this exact shape, distinguishing "receiver's class
  is genuinely unknown" from "receiver is provably a different, unrelated
  class") is closer to a small feature than a quick fix, and risks either
  false abstention (blocking genuinely safe renames whenever a class has any
  internal `$var method` helper pattern) or reintroducing the same
  false-positive risk as the first option, depending on how conservatively
  it's tuned.

Left open for a future session with more room to design the safety-gate
direction carefully — not attempted lightly, and not silently: this note is
that record.

**idx 84, in detail (partially fixed — `CONFIGURE`-tracking landed, 3rd-word
navigation deliberately deferred):** the finding's own repro is real:
`tk/library/systray.tcl:481-483` (and `print.tcl:1451-1452`,
`fileicon.tcl:9322-9323`, `accessibility.tcl:1274-1275`) splice
`systray`/`sysnotify` into the built-in `tk` ensemble via `namespace
ensemble configure tk -map [dict merge [namespace ensemble configure tk
-map] {systray ::tk::systray sysnotify ::tk::sysnotify::sysnotify}]` — a
**`CONFIGURE`**, not `CREATE`, statement on a *pre-existing*,
registry-builtin ensemble. `handle_namespace_ensemble` bailed immediately
unless `args[1] == "create"`, so this idiom (and the `-map`/`-subcommands`
recording idx 106 built for exactly this purpose) was entirely invisible.
Two consequences, both confirmed via tclsh9.0/8.6: (1) W001 fired 5 false
"Unknown subcommand" diagnostics on 100%-legitimate `tk systray`/`tk
sysnotify` calls; (2) go-to-definition/hover on the 2-word shape (`tk
systray`, `tk sysnotify`) fell through to `fallback_proc_by_simple_name`
and could silently resolve to an unrelated same-tail-name decoy proc
elsewhere in the workspace (proven with two independent decoys), or — with
no decoy present — simply abstained despite the mapping being a literal,
static, in-source fact.

Fixed, both consequences, for the `CONFIGURE` shape (`7115bc8`):
`handle_namespace_ensemble` now also accepts `configure NAME ?opts?`,
resolving `NAME` the same namespace-aware way a `-map` target already is
(`resolve_command_qualified_name`, not a blind prefix — `configure`'s
ensemble is a *reference* to something pre-existing, not a fresh
declaration the way `create`'s default is); a dynamic `NAME` abstains. The
real idiom's `-map` value is itself one whole dynamic `[dict merge ...
{literal}]` substitution, not a literal list — the pre-existing
per-*element* `is_dynamic_word` guard doesn't protect a whole-value
dynamic, and naively word-splitting the expression's raw text on
whitespace produces actively wrong subcommand/target pairs (fragments of
`dict`/`merge`/`namespace`/`ensemble`/`configure`/the ensemble name itself
land at odd indices and get recorded as bogus command references) — worse
than abstaining. Caught by a dedicated safety-regression test *before* it
shipped (asserts no spurious reference is recorded from an unrecognised
dynamic shape). New `Analyser::dict_merge_literal_tail` (mirroring idx
110's `cmd_fragments`/`descend_token`/`segments_from_tree` idiom for
descending a `[...]` `Cmd`-kind token) recognises exactly `dict merge ARG
{literal}` — deliberately narrow, matching the idx 110 precedent: does not
recognise `dict set`/`dict replace`/`concat`/a helper proc, or `dict
merge` with more than 2 operands — and extracts the literal tail's own
pairs, which are statically known regardless of what `ARG` (typically a
self-referential query of the ensemble's current map) evaluates to.
Separately, `AnalysisResult::ensemble_subcommand_targets` was already
populated for definition/hover/references navigation (idx 106) but was
**never consulted by W001 at all** — new
`Analyser::statically_mapped_ensemble_subcommand_known` (validity.rs),
checked alongside the pre-existing `dynamic_ensemble_subcommand_known`
(idx 105's `implementation_namespace` mechanism, confirmed inapplicable
here — see the finding's own `root_cause_hint`: `tk systray`'s genuine
2-level nesting doesn't fit that field's single-hop `<ns>::<subcommand>`
convention).

**Deliberately not attempted, and confirmed to be a separate, general,
pre-existing issue, not idx-84-specific:** the finding's *3rd-word* case —
`tk systray create`/`configure`/`exists`/`destroy` — is unaffected by the
above; cursor-on-"create" still risks the same wrong-navigation. Root
cause: `instance_method_at_cursor`'s ensemble-subcommand check requires
the word immediately before the cursor word to be *exactly one token* (so
`tk systray` → head=`"tk"`, sub=`"systray"` matches, but for `tk systray
create` the text before "create" is `"tk systray"`, two tokens, so the
check returns `None` and falls through to the generic call-resolution path
regardless of what `ensemble_subcommand_targets` contains). A probe this
session confirmed this is **not** specific to `systray`/`sysnotify`: the
identical risk already reproduces today against the long-registered,
pre-existing `tk busy <subcommand>` entry (`tk busy hold .win` with a
decoy `::decoy::hold` proc resolves to the decoy, unchanged by this fix) —
a general limitation of the single-predecessor-token design for any
3-word-or-more ensemble dispatch, not something `handle_namespace_ensemble`'s
`CREATE`/`CONFIGURE` gap caused or this fix's scope covers. A sound fix
needs `instance_method_at_cursor`/`ensemble_subcommand_target` to walk
back through *multiple* predecessor tokens, chaining through
`ensemble_subcommand_targets` recursively (`"tk"` → `"systray"` → resolve
`::tk::systray`, then `::tk::systray` → `"create"` → …) — a materially
larger, more invasive change than warranted for one finding, left open for
a future session (same "investigate, document, defer" treatment as idx 79
above).

Also considered and **rejected** for this session: statically registering
`systray`/`sysnotify` as flat `SubCommand` entries in `tk_cmd.rs`
(mirroring `busy`/`fontchooser`), which would additionally suppress W001
for a *plain consumer* script that calls `tk systray create` without ever
containing the splice statement itself (the realistic common case — Tk's
own C/library bootstrap does the splice, invisible to the LSP). Not done:
this session's own methodology requires verifying every registry fact
(arity, synopsis) against a real tclsh/Tk oracle, and no live
Tk-with-display environment is available in this sandbox (the finding's
own `lsp_output` notes the identical constraint for its `no_bootstrap_repro.tcl`
case: "LSP-only since real Tk/display isn't available in this sandbox") —
guessing `tk sysnotify`/`tk systray`'s exact arity from memory and shipping
it as an unverified registry fact risks a false arity diagnostic for real
users, a worse failure mode than the narrower, but fully oracle-grounded,
`CONFIGURE`-tracking fix actually shipped. Left as a documented, low-risk
follow-up for a session with Tk oracle access.

#### Priority tier 2 — medium + low (60 + 1 = 61 findings, 1 already fixed), grouped by feature for clustering

Group findings sharing a feature/root-cause together in one fix pass the
way idx 107+115 and idx 118+119 were — many of these look like they share
a root cause within a feature group (e.g. the three `upvar`-adjacent W210
false-positives in ticklecharts, idx 57/58/59; the two `tclopt`
mixin/oo::configurable class-scoping findings, idx 34/36).

| feature | count | idx (severity) |
|---|---|---|
| tclOO | 10 | 15, 16, 34, 35, 36, 53, ~~54~~ **FIXED**, 55, 96, 97 (all medium) |
| namespaces | 8 | 3, 19, 43, 44, 64, 65, 75, 85 (all medium) |
| tricky_indirection | 7 | 0 (medium, **FIXED** — see below), 1, 2, 14, 49, 50, 51 (all medium) |
| upvar | 7 | 7, 22, 57, 58, 59, 98, 99 (all medium) |
| proc_args | 7 | 11, 28, 37, 62, 67, 78, ~~104~~ **FIXED** (all medium) |
| tcl_mathop | 4 | ~~30~~ **FIXED**, 80, ~~81~~ **ALREADY FIXED**, ~~103~~ **FIXED** (all medium) |
| package_loading | 3 | 4, 42, 72 (all medium) |
| source | 3 | 27, 41, 102 (all medium) |
| tracing | 3 | 47, ~~48~~ **FIXED**, 92 (all medium) |
| rename | 2 | 5, 45 (all medium) |
| aliasing | 2 | 21, 89 (all medium) |
| uplevel | 2 | 38 (medium), 100 (low) |
| eval | 1 | ~~24~~ **FIXED** (medium) |
| autoindex | 1 | 73 (medium) |
| safe_interp | 1 | 91 (medium) |

**idx 0 — FIXED** (PR #1068, branch
`claude/commandregistry-compiler-fixes-tshu8d-quickfixes`). The audit's
headline `{*}$validateHelper` indirection claim was already correct
abstention (REFUTED by the audit itself); the real bug it uncovered was a
literal `apply {{params} {body}}` inside a `[…]` command substitution
reporting `W123 Unknown command '<the parameter list>'`. `apply` reached that
way never went through `AnalyserHookId::Apply`; the substitution collectors
reached its arguments through the shared registry-aware `descend_command`,
which resolved `ArgRole::Body` only — so the whole `{params body}` list was
re-segmented as script source and the parameter list became a command head.
Once `ArgRole::LambdaLiteral` stopped that, an FN took its place: the
lambda's real body was walked by nothing at all.

Fixed by giving the substitution walk the same dispatch the top level has:
`dispatch_nested_segment` gained an `AnalyserHookId::Apply` arm beside its
existing `Proc` / `OoDefine` arms, so a substitution-position `apply` runs
`handle_apply_command` — the lambda's own scope, rooted at the lambda's
namespace, with its parameters bound there. `descend_command` deliberately
still resolves `ArgRole::Body` only. A first attempt (corrected on Codex
review of the PR) descended the lambda's body *element* from
`descend_command` instead; that fixed the span but handed the body to the
collectors as an ordinary body, which they walk in the **enclosing** scope —
so a lambda-body `set` became a local of the calling proc and an
explicit-namespace lambda resolved its calls in the caller's namespace. Full
write-up, including why the sub-span alone is not enough, in
[the `apply` lambda KCS note](../../kcs/kcs-issue-apply-lambda-body-not-highlighted-via-list-quoting.md)
(instance 8).

Each idx's full detail (summary, failure_scenario, oracle_output,
lsp_output, root_cause_hint) is in
`data/06-main-audit-results-COMPLETE-105of105.json`, keyed by `idx`. Follow
the same playbook as every fix already landed: re-confirm against current
code and a real tclsh oracle → registry-driven fix reusing §4's mechanisms
→ TP/FP/TN/FN unit tests + lsp_e2e test → validation gates → commit.

#### Tier-2 findings fixed by PR A3 (2026-07-30)

Landed together on `claude/commandregistry-compiler-fixes-tshu8d-mathfunc`
alongside issue #974 (`expr` math functions in hover / completion) and issue
#1054 (hover type inference built without the document's dialect), because the
mathfunc findings share one root cause with #974 and the word-recognition ones
share one shared helper. Regression coverage for all of them is
`rust/tcl-lsp-core/tests/mathfunc_and_word_recognition.rs` (27 TP/FP/TN cases)
plus new `rust/tcl-lsp-server/tests/e2e/{hover,completion}.rs` cases.

| idx | what was wrong | what changed |
|---|---|---|
| 24 (eval) | The cursor-word scan matched a `$level`-shaped substring inside an inert `#` comment or a brace-quoted data word Tcl emits verbatim, and hover / go-to-definition / find-references / rename resolved it to a real declaration — contradicting the LSP's own semantic tokens and its own W220. | New `tcl-lsp-core/src/inert_text.rs` holds two **conservative** inertness proofs: `offset_in_comment` (a `#` in command position, quote-aware) and `offset_in_data_brace` (a braced argument word whose registry `ArgRole` does not `carries_script`, descending through the ones that do). Both answer "inert" only when the position provably is, so no genuine reference can be lost. Consumed through one new shared helper, `definition::lookup_var_read_at`, wired into hover, definition, references, document-highlights, and prepare-rename. **Codex review of PR #1073 (P2)** sharpened `offset_in_comment`: a `{` is structural only at *word-start* position, so a mid-word brace (`puts a{# $v` — one bare word `a{#`, `$v` genuinely substituted, oracle-confirmed on 9.0.4 and 8.6.16) no longer restores command position and no longer suppresses a real read. The scan also now starts out of command position after a backslash continuation. The finding's *secondary* claim (a W210 false positive on `uplevel N set $var value`) is a separate analyser gap and is **not** covered here. |
| 30 (tcl_mathop) | Find-references returned 0 locations at a `tcl::mathfunc` `NAME(…)` call site while a query from the declaration worked (asymmetric); hover never resolved there at all; and a **bare** call to a `tcl::mathfunc`-namespaced proc wrongly resolved, on code tclsh rejects outright. | `resolve_proc_target_at` now resolves a mathfunc cursor through the analyser's own `is_mathfunc_call` invocation record (which already carries the settled `{ns}::tcl::mathfunc::NAME` two-candidate resolution), so references / rename / call-hierarchy / linked-editing all agree with definition and hover. The bare-call false positive is gone: `fallback_proc_by_simple_name` skips a `tcl::mathfunc` namespace, per `tcl_registry::mathfunc::is_in_mathfunc_namespace` (oracle: `li {10 20 30} 1` → `invalid command name "li"` on 8.6 and 9.0). |
| 48 (tracing) | Hover / references / definition returned nothing with the cursor on a variable's bare **declaring** token (`cmd` in `foreach cmd $tracecmds`, a `set` left-hand side). | **Already fixed** on `rust` by idx 9's `var_def_at_declaration_offset` work; re-verified and pinned with regression tests for both the real corpus shape and the trivial isolation the finding gives. |
| 54 (tclOO) | `${ns}::setdef` — the literal `::setdef` fragment after a substitution was read as a genuinely *absolute* name, so resolution looked for a global proc called `::setdef` and every consumer reported nothing. | The one shared word-bounding rule (`hover::word_char_bounds` → new `word_char_bounds_kinded`) now reports a word as the **residual tail of a computed name** when the left scan stopped on a `}` / `]` and the word starts `::`; `find_word_span_at_position` returns the name it spells (`setdef`) with the span narrowed past the `::`, so ordinary namespace-aware resolution finds it *and* rename cannot eat the separator. `WORD_DELIMS` is deliberately unchanged — a **single** colon is an ordinary Tcl name character (oracle: `set a:b 42`, `proc p:q {x} …`, `set arr(k:1) v`, `dict get $d x:y` all work on 8.6 and 9.0), so adding `:` to the delimiter set would have been wrong. |
| 81 (tcl_mathop) | Every built-in `expr` math function drew a false `W123 Unknown command`, several with actively-wrong "did you mean" fixes. | **Already fixed** on `rust` after the audit ran: `SignatureCommandInvocation::is_mathfunc_call` plus `is_known_mathfunc_in_dialect` gate the W123 check. Verified, not re-fixed. |
| 103 (tcl_mathop) | Zero `CommandSpec` entries for `tcl::mathfunc::*`, so the generic per-dispatch-site W002 availability check was blind to the plain-command spelling. | The registry data (`tcl-registry/src/commands/tcl/mathfunc_generated.rs`) **already landed** on `rust`. What PR A3 adds is the query layer the LSP needed on top of it — `tcl-registry/src/mathfunc.rs` — and a test pinning that both qualified spellings exist and gate correctly (`isinf` 9.0+, the command table itself 8.5+ per TIP 232). |
| 104 (proc_args) | A proc parameter's own name token and its **default-value literal** both resolved to an unrelated same-named command (`{destroy destroy}` in `tk.tcl`'s `::tk::RestoreFocusGrab` showed Tk's `destroy` documentation). | The name token was already answered by idx 9's declaration-offset search; the remaining data words are now guarded generally by `definition::parameter_list_position_at`, consulted by both hover and go-to-definition. A *literal* parameter list holds no command references at all, so the guard needs no per-command knowledge. **Codex review of PR #1073 (P2)** narrowed it from the whole name-token-to-body region to the actual literal parameter-list **word**: `proc`'s parameter list is an ordinary Tcl word and may be computed (`proc p [makeargs] {…}` runs `[makeargs]` at definition time — oracle-confirmed on 9.0.4 and 8.6.16, as are the `$params` and `"m n"` forms), so a word containing `$` / `[` / `"` / a backslash escape stays navigable. A computed list also suppresses the bareword-*declaration* step, because the analyser records a stub `VarDef` named after the whole word (`"[makeargs]"`) that would otherwise make go-to-definition point at the cursor's own token instead of resolving the call. |

### 6c. Broader mandate coverage check

The mandate named specific "tricky Tcl feature" dimensions. Rough coverage
so far (from mined+fixed findings): namespaces ✓✓✓, rename ✓ (idx 3 done),
unknown ✓ (idx 110 done; interp-create angle done), aliasing ✓ (idx 113
done), safe-/sub-interpreters ✓✓ (idx 111, 9 both done), tracing ✓✓ (idx
115, 116 both done), tricky indirection ✓✓ (idx
118/119 done), tclOO ✓✓ (idx 120, 121 both done), upvar ✓ (idx 122 done),
uplevel ✓ (idx 18 done), eval ✓ (idx 125 done), `::tcl`/`::tcl::mathop` namespaces ✓
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

**Superseded by the 2026-07-23 pause update at the top of this doc:** the
remaining findings this section walks through are now tracked as GitHub
issues [#1017](https://github.com/bitwisecook/tcl-lsp/issues/1017),
[#1018](https://github.com/bitwisecook/tcl-lsp/issues/1018), and
[#1019](https://github.com/bitwisecook/tcl-lsp/issues/1019) — start there.
The steps below are kept for their still-accurate mechanical detail
(branch state, oracle environment, disk-space gotchas, validation gate),
just no longer the primary "what's left" pointer.

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
   - §6a: 2 remaining tcllib findings (idx 128/24), no
     refined plan for either — use `07`'s `root_cause_hint` directly.
   - §6b: 62 remaining main-wave findings (idx 61, idx 9, idx 10, idx 18,
     idx 29, idx 31, idx 32, idx 33, idx 39, idx 46, idx 52, idx 56, idx
     63, idx 68, idx 70, idx 71, idx 76, idx 77, idx 84, idx 86, idx 90,
     idx 95, and idx 94 are fixed — idx 46, idx 63, and idx 84 only
     partially, see their §3/§6b rows for what's still open — so far),
     fully triaged into a
     priority-ordered critical/high table (1 remaining, start here) and a feature-clustered medium/low
     table (61 findings, group by feature when fixing). Likely the
     higher-leverage queue given its size and the presence of several
     zero-results go-to-definition/references failures on common
     real-world idioms.
5. Follow the playbook in §2/§4. Commit after each finding (or tightly
   related cluster of findings), same granularity as the fixes already
   landed — don't batch unrelated fixes into one commit.
6. Periodically re-run `cargo test --workspace` (full, not scoped) and
   `cargo clippy --workspace --all-targets -- -D warnings` as a sweep, not
   just the scoped per-crate checks used while iterating — the mandate's
   validation bar is workspace-wide. Watch disk space (§4's gotcha): a cold
   `--workspace` test build can exhaust the session's allowance on its own;
   `rm -rf /home/user/tcl-lsp/target/debug/incremental` is the cheap, safe
   recovery (regenerates on next build, and frees the bulk of it — no need
   to nuke all of `target/` first). **Use the absolute path** — the
   workspace's `target/` lives at the repo root
   (`/home/user/tcl-lsp/target`), *not* under `rust/` even though every
   crate manifest does; running this (or `df`/`du`) from a `rust/`-relative
   cwd silently no-ops (the path just doesn't exist there) and looks like
   cleanup happened when it didn't — confirmed the hard way this session.
   **`incremental/` isn't always the bulk of it**: after enough
   `--workspace` iterations, `target/debug/deps`/`target/debug/examples`
   (hundreds of stale, hash-versioned test/example binaries across the
   workspace's 41 crates) can dwarf `incremental/` itself (one session
   this campaign hit 20G in `deps/` alone vs. 2.6G in `incremental/`,
   filling the disk enough to take the whole `Bash` tool down — every
   invocation failed at the harness's own preflight step before any
   command ran, since it couldn't even create its own output-capture
   directory). If clearing `incremental/` alone doesn't recover enough
   room, `rm -rf /home/user/tcl-lsp/target` (confirmed `.gitignore`d) is
   the complete, still-safe fix — slower next cold build, nothing lost.
   If `Bash` itself is down and you're stuck reasoning about *why*, check
   `df -h /` the moment anything recovers enough to run one command —
   don't assume a `tasks`-scratch-dir-specific cause without checking; it
   was the whole root filesystem both times this session hit it, not a
   dedicated small tmpfs. If `Bash` is completely down and staying down,
   pushing a small, already-locally-verified fix directly via the GitHub
   MCP `push_files` tool (from a repo you already have `add_repo`'d) is a
   viable fallback for *already-finished* work sitting uncommitted — but
   treat it as a last resort for files small enough to reconstruct
   confidently, and reconcile with `git fetch` + a diff against the new
   remote HEAD (not `reset --hard` blind) the moment `Bash` recovers, to
   confirm byte-for-byte what actually landed matches local before trusting
   it (see idx 95's `7953d5e`/`ef36c73` commits and the surrounding
   session transcript for the full story — it worked, but took real care).
7. Both queues (§6a's 2 tcllib findings, §6b's 62 main-wave findings) are
   independent — fix from whichever queue makes sense, no need to exhaust
   one before starting the other. Keep this document's counts current as
   findings get fixed: move a finished idx out of §6a/§6b's tables and into
   §3's commit table, same pattern as every fix so far.
8. Keep the Stop-hook's implicit contract: uncommitted work is a liability —
   commit and push after every completed fix, don't let it accumulate.
9. When ready to submit, open a PR from this branch (no PR is currently open
   for it — #963 is closed/merged and superseded, see the top of this doc).
