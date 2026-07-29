<!-- Appendix to name-resolution-fix-plan.md. Dynamic-surface navigation-link audit. -->
<!--
SOURCE-LINK LEGEND: every `file:line` below is in THIS repo and resolves to a
stable permalink at the v2.1.9 code commit:
  https://github.com/bitwisecook/tcl-lsp/blob/6a6bc87e94c67416cdca6954ba2ad0ec6937bd63/<path>#L<line>
The DANGEROUS findings D1 (TclOO object-variable whole-body-span rename) and D2
(uplevel #0 query guard) were re-verified by hand against the code at that
commit — rename.rs:631-635, oo.rs:326, scope.rs:696, definition.rs:552 vs 612.
-->

# Tricky name-resolution surfaces — navigation-link audit

**Status:** findings addendum. This document is an appendix to
[name-resolution-fix-plan.md](name-resolution-fix-plan.md) (milestones M0–M6)
and [name-resolution-centralization.md](name-resolution-centralization.md). It
catalogues the *dynamic* Tcl surfaces that create a name link — of a COMMAND,
VARIABLE, CLASS, or METHOD — which the LSP navigation features
(Find-References, Rename, Go-to-Definition, Hover, Call-Hierarchy,
Document-Highlight, Go-to-Implementation) should or should not follow, and
records whether the current implementation follows it correctly.

Every claim cites a backend file:line for ground truth (the bytecode VM under
`rust/tcl-vm/src/` or the WASM runtime under `runtime/rust/src/`) and a provider
file:line for how the LSP handles it today. Each surface is tagged:

- **CONFIRMED** — an adversarial verifier traced the actual code paths and
  confirmed the gap against the implementation.
- **PLAUSIBLE** — investigation-only; grounded in code reading but not
  independently re-verified end-to-end.
- **REFUTED** — the verifier found the code already handles the surface
  correctly (a valuable negative result; still listed).

The **core question** for every surface: does it create a name link the LSP
SHOULD follow but doesn't (a **false negative** — a missed reference / missed
rename edit, which leaves a runtime-live alias/import/upvar pointing at the old
name — silent under-editing), or one it SHOULD NOT but does (a **false
positive** — a wrong or extra edit that rewrites an unrelated same-named
binding, which silently corrupts code). Honest abstention (no result) is
acceptable; a confident WRONG result is not.

The seven already-audited findings (M0 wrong-same-named-symbol scan family, M2
`invocations_of`, M1 class-name fork, M4 variable-resolution split, hover
bareword path #5, M5 `$cmd` limitation, M6 library/autoload) are referenced by
milestone where a surface here interacts with them; they are **not** re-explained.

---

> **Milestone numbers below predate the final renumbering.** The authoritative
> scheme is the 16-milestone index in
> [name-resolution-fix-plan.md](name-resolution-fix-plan.md). Remap: old
> M0→**M1**, M0.5→**M3**, M1(class)→**M4**, M2(oracle)→**M5**, M3(TclOO
> x-file)→**M6**, M4(variable)→**M2**, M5(cmd-in-var)→**M7**,
> M6(library)→**M8**, M6.5(source-site)→**M9**, "reference roles /
> ensemble / trace"→**M14**, "interp isolation / coverage"→**M15**. D1's fix is
> **M2 Stage 2.1**.

## 0. The DANGEROUS class — CONFIRMED false positives (silent code corruption)

These surfaces produce a **confirmed wrong/extra edit or a wrong reference
link** — the LSP rewrites or points at a binding that is *not* the one the
runtime uses. Unlike a missed edit (which merely under-delivers), these
actively corrupt code or mislead the user. They are the top-priority defects.

| # | Surface | Kind | What goes wrong | Milestone |
|---|---------|------|-----------------|-----------|
| D1 | **TclOO object variables** (`variable v` / `my variable v`) | variable | Rename of `$v` emits a decl edit whose range is the **entire method body** and whose replacement is just the new name — `{ return $v }` becomes `w`, **destroying the body**. Cross-method uses split into two distinct vars. | M4 |
| D2 | **`uplevel #0 { … }`** cursor resolution | variable | `lookup_var_in_scope_chain` lacks the proc-local-drop guard, so a `$g` in the body resolves to an **invisible proc-local** of the same name instead of the global the runtime reads. Drives Go-to-Def / Hover / References / Rename to the wrong var. | M4 |
| D3 | **`uplevel 1`/`2` / bare `uplevel`** body vars | variable | Body vars are attributed to the **enclosing proc's own scope**; a same-named proc-local absorbs a use that at runtime targets the caller's frame — wrong reference link and wrong rename edit. | M4 |
| D4 | **Nested `proc`/`oo::class create`** under a **qualified-name** enclosing proc | command / class / method | ~~Definition homed by a purely lexical namespace walk, not the proc's defining namespace; on name collision two defs **hash to the same `all_procs` key and one overwrites the other**, so navigation binds to the wrong same-named symbol.~~ **CLOSED** — every analyser site now homes through `command_resolution_namespace`; see [§1.8](#18-nested-proc--ooclass-create-re-homing--closed) | done (#923 idx 85) |
| D5 | **Event/idle callbacks** (`after`, `fileevent`, `bind`, `-command`) | command / variable | Body walked in the **enclosing** namespace, but Tcl runs it at **global (`::`)** scope; a `helper` in `namespace eval ::x { after 0 { helper } }` binds to `::x::helper` (wrong) instead of `::helper`. Confident-wrong only under `::x::helper`-vs-`::helper` collision. | none (NEW) |
| D6 | **`oo::objdefine` per-object method** under name collision | method | Body ignored; if a class method shares the name, `lookup_method_in_class` returns the **class method's** span even though `$o m` dispatches the per-object override at runtime. | none (NEW) |

One **PLAUSIBLE** (investigation-only, not re-verified) member of this class:

| P-D | Surface | Kind | What goes wrong | Milestone |
|-----|---------|------|-----------------|-----------|
| PD1 | **`interp eval CHILD SCRIPT`** | command / variable | Child-interp defs/calls attributed to the **parent** namespace, merging a parent command with a same-named child-interp command; rename of the parent proc wrongly edits the child body. Documented as a known contract approximation. | none |

Everything below is organized by resolution kind; the danger-class items reappear
in their sections with full detail.

---

## 1. Command-name surfaces

### 1.1 `interp alias {} ::foo {} ::bar` — the alias→target command link — **CONFIRMED** (high)

- **Real Tcl:** the alias name `::foo` is an ordinary command-table entry whose
  target is re-resolved BY NAME from the GLOBAL namespace at every call
  (late-bound), keeping the caller's frame. `rust/tcl-vm/src/exec.rs:2701-2714`
  (`Command::Alias(target)`: target resolves in the global namespace regardless
  of caller). Contract `command-resolution.md:96-103`.
- **LSP today:** Go-to-Definition **follows** the link —
  `definition.rs:222-232` (`lookup_alias` → `resolve_called_proc` against `::`)
  jumps from a `foo` call, or the alias name, to `::bar`. But
  Find-References / Rename / Call-Hierarchy never consult
  `analysis.command_aliases` (grep empty in `references.rs`, `rename.rs`,
  `call_hierarchy.rs`). The alias is recorded as `SignatureCommandAlias`
  (`rust/tcl-compiler/src/signature_scan/types.rs:129-137`) with **no span** for
  the alias name or the target word (`handlers.rs:1611-1630`); the settle pass
  makes `::foo` "known" and settles a `foo` call's `resolved_qualified_name` to
  `::foo`, never `::bar` (`scope.rs:355-401`). `invocation_references_named`
  (`references.rs:127-178`) matches by proc name only, so a `foo` call never
  matches `::bar`'s `ProcDef`; `rename_proc` (`rename.rs:654,714`) reuses that
  same matcher.
- **Gap:** false-negative-missed (asymmetric with Go-to-Def).
  Rename of `::bar` rewrites its decl and direct callers but leaves the
  `interp alias {} ::foo {} ::bar` target and the `foo 1` call pointing at the
  now-nonexistent name — **silent runtime breakage of the live alias**. Cursor
  on the alias name `::foo` in Find-References falls through to `Vec::new()`
  (`references.rs:221-269` dispatches only to var/class/proc/method resolvers).
- **Repro:** `proc ::bar {x} {}; interp alias {} ::foo {} ::bar; foo 1` — rename
  `::bar`→`::baz` leaves the alias target and the `foo 1` call stale;
  Find-References on `::bar` omits both.
- **Milestone:** none. **NEW** — see §6 proposed item **M0.5 (alias/import/rename
  link-following)**. Prerequisite: `handle_interp_alias` must record a target-word
  span before Rename can rewrite it.

### 1.2 `rename oldproc newproc` — the command-table move — **CONFIRMED** (high)

- **Real Tcl:** after the statement `oldproc` no longer resolves and `newproc`
  dispatches to `oldproc`'s original body. Contract `command-resolution.md:104-110`.
  Analyser records `renamed_commands[::newproc]=::oldproc`,
  `deleted_commands[::oldproc]` (`handlers.rs:1655-1676`) but synthesizes **no
  ProcDef** for `newproc`.
- **LSP today:** `renamed_commands` feeds only the `known`/`renamed_away`
  existence predicate (`scope.rs:347-397`) — never rewrites a `newproc` call's
  `resolved_qualified_name` to `::oldproc`. Grep of `tcl-lsp-core/src/` for
  `renamed_commands`/`rename_offsets` is empty. `resolve_called_proc`
  (`definition.rs:748-762`) searches `all_procs`/registry only → a `newproc` call
  returns None. `invocation_references_named` (`references.rs:127-178`) never
  matches `::newproc` to `oldproc`. The `rename OLD NEW` tokens are arguments, not
  invocation heads.
- **Gap:** false-negative-missed. Go-to-Def / Call-Hierarchy / References on a
  `newproc` call find nothing; References/Rename of `oldproc` omit the `rename`
  statement's OLD argument and every `newproc` call; renaming `oldproc`'s decl
  leaves `rename oldproc newproc` stale, so the runtime `rename` fails "invalid
  command name."
- **Repro:** `proc oldproc {} {}; rename oldproc newproc; newproc`.
- **Milestone:** none. **NEW — M0.5.** Fix: `definition.rs:748` map `newproc` via
  `renamed_commands`; `references.rs:127` + `rename.rs:654` include the
  `rename OLD NEW` argument spans (analyser must record those spans; only the
  offset is in `rename_offsets` today).

### 1.3 `namespace import ::src::foo` — the imported command link — **CONFIRMED** (medium)

- **Real Tcl:** the import installs a real command entry `::dst::foo` holding the
  source command's TOKEN; a source rename is FOLLOWED (import keeps working,
  `namespace origin` reports the new name). `runtime/rust/src/cmd_namespace.rs:433-435`
  (binds `dst::foo` → `Command::Imported{source: ::src::foo}`); contract
  `command-resolution.md:111-117`.
- **LSP today:** `analysis.namespace_imports` is recorded
  (`handlers.rs:2088-2125`) but consumed only by class-lattice constructor
  resolution (`class_lattice.rs:404`) and W123 suppression
  (`unresolved.rs:249-261`). `naming.rs` `resolve_command_with` is import-blind
  (grep empty), so the settle pass leaves a `dst`-side `foo` call resolved to
  `::dst::foo`. `invocation_references_named` (`references.rs:146-163`) requires
  `call_ns==target_ns` for the simple case (`"dst"!="src"`) and tail-match fails
  on `"foo"` vs `"src::foo"`.
- **Gap:** false-negative-missed. Find-References on `::src::foo` omits `dst`'s
  `foo`; Rename of `::src::foo` does not rewrite the `namespace import ::src::foo`
  pattern text (a stale-name missed edit). **Precision note:** the `dst` call must
  be *reported* by Find-References but must NOT be text-rewritten by Rename — the
  import binds under the tail name `foo`, which keeps working via the token after a
  source rename.
- **Repro:** `namespace eval src { namespace export foo; proc foo {} {} }; namespace eval dst { namespace import ::src::foo; foo }`.
- **Milestone:** none. **NEW — M0.5** (shares the alias/import following work).
  Fix: `references.rs:127-178` treat a bare call whose tail matches an in-scope
  `namespace_imports` pattern as a reference; `rename.rs` rewrite the pattern span.

### 1.4 `namespace ensemble create -map/-subcommands` — subcommand→impl dispatch — **CONFIRMED** (medium)

- **Real Tcl:** `ens sub args` resolves `sub` against the subcommand set, then
  dispatches: `-map` entry → its command prefix's first word re-resolved by the
  full rule; else the exported proc `<ns>::sub`.
  `rust/tcl-vm/src/interp.rs:1416-1428` (`dispatch_ensemble`: `map.get(&s)` word[0]
  else `format!("{}::{s}", e.namespace)`, then `invoke_command`);
  `cmd_namespace.rs:293-363` builds `EnsembleDef`.
- **LSP today:** `handle_namespace_ensemble` (`handlers.rs:1047-1089`) records ONLY
  the dispatch-command NAME into `ensemble_namespaces` (line 1056, or `-command`
  value at 1084); the `-map` dict and `-subcommands` list are walked purely to skip
  their words (`value_word_count`, line 1087). Consumers are suppression/declaration
  only: `validity.rs:392`, `unresolved.rs:235`, `item_tree.rs`. No user-ensemble
  navigation in `tcl-lsp-core` (the ensemble hits in `hover.rs:623-627,702-706` are
  builtin two-level ensembles).
- **Gap:** false-negative-missed. Go-to-Def on `sub` in `ens sub` yields nothing
  (honest abstention). Find-References / Call-Hierarchy on `::ns::sub` miss all
  `ens sub` call sites. Rename of `::ns::sub` does not update `ens sub` callers, and
  for `-map {sub ::real::impl}` does not update the `::real::impl` literal inside the
  map (map value skipped, never recorded) — leaving the ensemble runtime-live but
  pointing at an absent target.
- **Repro:** `namespace eval ::m { proc add {a b} {expr {$a+$b}}; namespace export add; namespace ensemble create }` — `m add 1 2`; and
  `namespace ensemble create -command calc -map {sum ::m::add}`.
- **Milestone:** none. **NEW** — see §6 proposed **M7 (ensemble subcommand model)**.
  Fix: `handlers.rs:1047-1089` parse `-map`/`-subcommands` into a subcommand→target
  mapping and emit an invocation/reference for each `<ns>::sub` and each `-map`/
  `-unknown` literal target; then wire the nav providers.

Related (**PLAUSIBLE**, low): the **`-unknown handlerProc`** and **`-map` target
proc names** are themselves command references — renaming those procs must update
the ensemble configuration string. Same fix location, same milestone.

### 1.5 `tailcall CMD ?arg…?` — the tailcalled command — **CONFIRMED** (medium)

- **Real Tcl:** `tailcall`'s first word is dispatched as an ordinary command in the
  issuing proc's current namespace once it unwinds (TIP 327).
  `rust/tcl-vm/src/exec.rs:2909-2949` (`run_tailcall`: `words[0]` → `dispatch_words`/
  `invoke_command`); `Op::TAILCALL` at `exec.rs:2600-2608`.
- **LSP today:** `tailcall`'s `CommandSpec`
  (`rust/tcl-registry/src/commands/tcl/tailcall_.rs:29-59`) declares **no**
  `arg_roles`, no `command_prefixes`, no Body marking. `process_command` records
  only the head token, `[...]` substitutions, and `CommandPrefix` callback heads
  (`commands.rs:305-351`) — a bareword at arg index 0 of `tailcall foo` is none of
  these. The registry drift-guard test
  (`registry.rs:2420-2467`) keys on the synopsis keyword `"cmdprefix"`, and
  `tailcall`'s synopsis says `"command"`, so it isn't caught.
- **Gap:** false-negative-missed. References / Rename / Call-Hierarchy miss every
  tailcalled call site; rename of the target leaves `tailcall foo` dangling. No
  false positive (nothing recorded). **Identical omission for `coroutine name
  command ?arg…?`** — `coroutine.rs:39` marks arg 0 as the created command name but
  never records arg 1 as an invocation.
- **Repro:** `proc a {} {}; proc b {} { tailcall a }`; namespaced:
  `namespace eval ::n { proc a {} {}; proc b {} { tailcall a } }`.
- **Milestone:** none. **NEW** — small registry-spec fix; slot into **M0.5** or a
  cross-cutting "declare CommandPrefix on command-word args" sweep. Fix: declare
  `CommandPrefix` on `tailcall` arg 0 and `coroutine` arg 1 so the existing
  `record_command_prefix_invocations` path lights up.

### 1.6 `expr {f($x)}` → user math-function `proc ::tcl::mathfunc::f` — **CONFIRMED** (medium)

- **Real Tcl:** an expr function call `f(...)` dispatches as the command
  `tcl::mathfunc::f`, resolved current-namespace-first through the live command
  table (TIP 232). `rust/tcl-vm/src/expr.rs:684-707`; `cmd_math.rs:102-121`;
  `runtime/rust/src/interp.rs:5801-5834`; `cmd_mathfunc.rs:84-90`; contract
  `command-resolution.md:125-133`.
- **LSP today:** the analyser harvests command refs from expr bodies ONLY via
  `[...]` command-substitution tokens (`collect_expr_substitutions`,
  `commands.rs:2225-2243`, caller `1632`). The `f(` bareword lexes as a plain
  string segment, never a `Cmd` token, so it is never an invocation. The only
  mathfunc-adjacent code is the external-stub `expr-func` comment overlay
  (`utils.rs:551-599`), unrelated to source-defined procs.
- **Gap:** false-negative-missed. Go-to-Def on `f` inside `expr {f(3)}` returns
  nothing; References / Call-Hierarchy on `::tcl::mathfunc::f` miss every expr site;
  Rename of the proc leaves `f(...)` dangling.
- **Repro:** `proc ::tcl::mathfunc::dbl {x} {expr {$x*2}}; set y [expr {dbl(3)}]`.
- **Milestone:** none. **NEW** — expr-body function-name resolution; slot into
  **M7** (or a small standalone). Niche (requires custom mathfuncs).
- **Status (M7, shipped):** `record_expr_function_invocations`
  (`commands.rs`) now harvests every `f(...)` bareword inside an `expr`
  (via `expr_function_calls`, the const-folder's expr parse) as a command
  invocation resolved to `::tcl::mathfunc::f` — closing the go-to-def /
  references / rename gap above for a *user*-defined
  `proc ::tcl::mathfunc::f`. Issue #968 closed the companion W123 gap this
  left open: a *built-in* function name (`sin`, `max`, …) with no user
  override drew a spurious "unknown command" hint, since only the
  user-proc path resolved (`w123_invocation_resolves`'s
  `expr_mathfunc_name_known`, `unresolved.rs`, consulting the shared
  `tcl_syntax::expr::mathfunc` name/version table). **Follow-up closed:**
  the settled qualified name was always the *global* `::tcl::mathfunc::f`,
  never accounting for a namespace-local override (`::ns::tcl::mathfunc::f`
  shadowing inside `::ns` — real per TIP 232 / verified by the VM's
  `namespace_local_mathfunc_shadows_global_in_expr` test in
  `tcl-vm/tests/tricky_resolution_e2e.rs`). Digging into that gap surfaced a
  sharper, more common bug in the same code path: `record_expr_function_invocations`'s
  walk-time guess and `finalise_invocation_resolutions`'s generic one-hop
  `{ns}::{name}` suffix-strip both assumed a mathfunc invocation's resolved
  name has the *ordinary* `{callingNamespace}::{tail}` shape — false for a
  mathfunc call, whose resolved name always carries the fixed
  `tcl::mathfunc` dispatch segment regardless of the caller's own namespace.
  The generic settling pass would then strip the wrong suffix (recovering
  `::tcl::mathfunc` as a bogus "calling namespace"), and — whenever an
  *unrelated* ordinary command/proc/class/alias/rename-target anywhere in
  the file happened to share the bare tail name (`proc sin {…}`, `proc abs
  {…}`, `proc max {…}` — all plausible user proc names) — silently
  mis-resolve `expr {sin(...)}` to that unrelated global command instead of
  `::tcl::mathfunc::sin`. W123 itself never regressed (the coarse
  `proc_tail_names` fallback happened to still suppress it either way), but
  go-to-definition, references, rename, call-hierarchy, minify, and
  completion all read `resolved_qualified_name` directly and would have
  inherited the wrong target.

  First pass fixed this with a *structural* branch in
  `finalise_invocation_resolutions`: detect the mathfunc shape by checking
  whether `resolved` ends with `::tcl::mathfunc::{name}`, then settle via a
  dedicated two-candidate rule. Re-reviewing that fix surfaced two more
  things worth doing properly rather than declaring done: (1) shape-sniffing
  a string is exactly the kind of guess this whole bug started from — a
  contrived namespace literally named `…::tcl::mathfunc` holding unrelated
  ordinary commands could still fool it — and every *other*
  `push_command_reference` caller in the analyser (`oo.rs`, both ensemble
  paths in `handlers.rs`, the existence-probe path in `commands.rs`) was
  audited to confirm mathfunc really is the only "fixed dispatch prefix,
  not the calling namespace" case among them, so a one-off flag beats a
  speculative general mechanism; and (2) the branch used
  `bareword_resolution_candidates` (no `namespace path`), while math
  functions are ordinary commands under TIP 232 and the VM's own
  `resolve_command_fqn` routes every lookup — mathfunc calls included —
  through the same `namespace path`-aware resolver
  (`tcl-vm/src/interp.rs`), so the analyser was silently narrower than the
  runtime it describes.

  Landed properly instead:
  [`SignatureCommandInvocation::is_mathfunc_call`](../../rust/tcl-compiler/src/signature_scan/types.rs)
  is set once, at record time, by the one caller that needs it
  (`push_mathfunc_command_reference`) — no shape-guessing at settlement
  time. `finalise_invocation_resolutions` (`scope.rs`) branches on the flag
  and settles via `command_resolution_candidates(&ns, path, "tcl::mathfunc::f")`
  (the same `path`-aware builder the generic case already uses, not the
  `path`-free specialisation), `known()` plus — for the global candidate
  only — the shared `tcl_expr_eval::is_known_mathfunc_in_dialect` free
  function, so the existence check and W123's own agree by construction.
  Covered by `commands.rs`'s
  `expr_function_call_ignores_unrelated_same_named_global_proc` /
  `expr_function_call_resolves_namespace_local_mathfunc_override` /
  `expr_function_call_falls_back_to_global_user_override_from_a_namespace` /
  `expr_function_call_resolves_builtin_from_inside_a_namespace` /
  `expr_function_call_honours_namespace_path`, `definition.rs`'s
  `no_definition_for_mathfunc_call_despite_unrelated_same_named_proc` /
  `mathfunc_call_jumps_to_namespace_local_override`, `references.rs`'s
  `references_do_not_cross_between_unrelated_proc_and_mathfunc_override`,
  and `rename.rs`'s
  `rename_mathfunc_override_updates_call_site_and_skips_unrelated_proc` —
  the last two close out the two consumers (go-to-def already had direct
  coverage) this section's own "Gap" line named as broken: references and
  rename now have direct e2e coverage, not just an inference from the
  analyser-level `resolved_qualified_name` being correct.

  **Known residual gap, confirmed pre-existing and generic — not fixed
  here:** `finalise_invocation_resolutions`'s `known()` predicate checks
  `all_procs.contains_key(qualified)` with no deletion gating at all (unlike
  its registry-builtin clause, which explicitly excludes a renamed-away
  name via `renamed_away`). A namespace-local mathfunc override that is
  declared and then `rename`d away before a later call resolves — and
  therefore fails to draw W123 — is one instance of this, but the identical
  probe against an ordinary same-shape case (`namespace eval ::a { proc
  helper {} {...} }; rename ::a::helper {}; namespace eval ::a { proc
  caller {} { helper } }`) shows the same non-detection, proving this
  predates every change in this section and is not mathfunc-specific. Given
  W123's own documented design bias (prefer a missed warning over a false
  positive — see `build_w123_known_names`'s profile-filter reasoning
  above), leaving it be is consistent with that stance; fixing it for real
  would mean threading `deleted_commands` through `known()` for every
  invocation kind, a separate, broader change out of scope here.

  **Consumers verified only indirectly, via the shared
  `resolved_qualified_name`/`resolution_candidates` fields being correct at
  the source, not by a dedicated end-to-end test:** hover, completion, call
  hierarchy, minify, and the dependency graphs (`tcl-lsp-core`'s
  `references.rs`, `linked_editing_range.rs`, `minify.rs`, `graphs.rs`,
  `call_hierarchy.rs`, `completion.rs` all read these same fields). Their
  own test suites (1017+ tests across `tcl-lsp-core`/`tcl-lsp-db`) passed
  unmodified throughout, so nothing regressed, but no test in this effort
  specifically targets a mathfunc call through any of them.

  **Review follow-up:** the W123 shortcut (`w123_invocation_resolves`,
  `unresolved.rs`) checked only the settled qualified name against
  `expr_mathfunc_name_known`, with no regard for *how* the invocation was
  recorded. An *ordinary* call made from inside the real `::tcl::mathfunc`
  namespace (or targeting a custom function's own body) can settle to the
  identical `::tcl::mathfunc::<name>` shape a genuine `expr` function-call
  site does, and TIP 232's command wrappers — the mechanism that makes such
  a bareword call valid *at all* — only exist from Tcl 8.5 onward,
  independent of any individual function's own earlier expr-grammar
  availability (`sin` is a valid 8.4 expr function with no 8.4 command
  form). The shortcut now also checks `is_mathfunc_call`: a genuine `expr`
  function-call site keeps the existing per-function expr-grammar check
  unchanged, while any other invocation additionally requires
  `tcl_expr_eval::mathfunc_command_wrappers_available_in_dialect` — a
  coarser, single fact (available from 8.5 onward, regardless of which
  function) rather than the per-function ceiling. Covered by
  `tests.rs`'s `w123_tp_ordinary_call_shaped_like_mathfunc_fires_under_84`
  / `w123_fp_ordinary_call_shaped_like_mathfunc_resolves_under_86` /
  `w123_tp_custom_mathfunc_body_bareword_call_fires_under_84` /
  `w123_fp_expr_function_call_unaffected_by_wrapper_availability_under_84`.

### 1.7 Literal command names in dispatch tables — **CONFIRMED** (medium)

- **Real Tcl:** a literal extracted from a dict/array/string-map value becomes the
  command head at call time and dispatches live. `rust/tcl-vm/src/exec.rs:2662-2663`
  (`dispatch_words` takes `words[0]` after `{*}` flattening); lookup at
  `interp.rs:1556`.
- **LSP today:** `command_invocations` is populated only from the head word, iRules
  `call PROC` (`commands.rs:320-338`), `[...]` substitutions
  (`record_nested_invocations_from_args`, `commands.rs:1210-1266`, which treats a
  braced *data* word as opaque), and `CommandPrefix` callbacks. `handle_set_command`
  records a braced value only as a const-string (`handlers.rs:170-177`). The W307
  heuristic *does* recognize the literal (`state.rs:1833-1865` harvests `puts` from
  `array set state {-command puts}`) but only to suppress a diagnostic — never emits
  a `CommandInvocation`.
- **Gap:** false-negative-missed. References / Rename on `::m::add` miss its
  appearance as a dict/`array set`/`string map` value; rename breaks the runtime-live
  `{*}[dict get $t $op]` / `$handlers(-command)` dispatch. **Scope note:** a
  `switch`-arm body is NOT a gap — each arm is run through `analyse_body`
  (`handlers.rs:1251,1265`), so a literal proc-name *call* in an arm is recorded
  normally. The gap is literals held as collection *values*.
- **Repro:** `set t {add ::m::add sub ::m::sub}; proc dispatch {op args} {global t; {*}[dict get $t $op] {*}$args}`.
- **Milestone:** none. Adjacent to but distinct from the M5 `$cmd` limitation (these
  names ARE literals in source, just embedded in data). **NEW — M5-adjacent** (see §6).

### 1.8 Nested `proc` / `oo::class create` re-homing — **CLOSED**

- **Real Tcl:** a nested definition qualifies its name against the *runtime-current*
  namespace of the enclosing proc (its defining namespace), and a proc body runs with
  the proc's defining namespace current. `rust/tcl-vm/src/command.rs:1067`
  (`qualify_name` against current ns); `exec.rs:1085` (`push_ns(proc.namespace)` — body
  runs in defining ns); `cmd_oo.rs:951` (class create qualifies likewise).
- **The gap that was:** definition homing used a purely lexical namespace walk that
  collected only `ScopeKind::Namespace` names and **skipped proc/method scopes**, so a
  definition made inside `proc ::x::mk {} {...}` homed to `::` rather than `::x`.
  Empirically confirmed at the time by dumping `all_procs`/`all_classes` keys:
  (A) `namespace eval ::x { proc mk {} { proc helper {} {} } }` → `::x::helper` **correct**;
  (B) `proc ::x::mk {} { proc helper {} {} }` → `::helper` **wrong** (real: `::x::helper`);
  (C) `namespace eval ::x { proc ::y::mk {} { proc helper {} {} } }` → `::x::helper`
  **wrong** (real: `::y::helper`);
  (D) `proc ::x::mk {} { oo::class create Helper {} }` → `::Helper` **wrong** (real:
  `::x::Helper`). The false positive: with a real global `::helper` also present, both
  hashed to the same `all_procs` key and one silently overwrote the other.
- **Fix:** `Analyser::command_resolution_namespace` (`analyser/scope.rs`) — built on the
  shared `advance_command_resolution_namespace` per-scope-kind rule — is now the *single*
  answer to "which namespace is current here?" for every analyser site. The purely
  lexical walk has been deleted, so a new call site cannot pick the wrong one. Converted
  in issue #923 idx 85: `proc` / TclOO `oo::class create` (earlier wave), then
  `namespace ensemble create|configure`, `oo::define`, snit `type`/`widget`, itcl
  `class`, `namespace import`, `namespace export`, `<pkg>::import` aliases, command-alias
  resolution, `apply`'s relative namespace pin, registry-definer symbol qualification,
  the imported-command body-role fallback, and the deferred method-body namespace.
- **Known limits:** a TclOO **method** body resolves globally
  (`Scope::oo_global_resolution`) because at run time it executes in the *object's*
  namespace, which is not statically known — so a `namespace import` written inside a
  TclOO method is attributed to `::`, not to the (unknowable) per-object namespace.
  That is a deliberate, tclsh-pinned approximation, not a residual of this gap.

### 1.9 `interp eval CHILD SCRIPT` — cross-interp merge — **PLAUSIBLE** (medium) — DANGEROUS (PD1)

- **Real Tcl:** a child interp is a fully separate command table + namespace + variable
  set; nothing crosses the boundary. `rust/tcl-vm/src/interp.rs:369-381`
  (`children: HashMap<String,Vm>`); `runtime/rust/src/interp.rs:486-497`. Contract
  `command-resolution.md:~138-143` pins the separation and flags the static walk as a
  KNOWN approximation.
- **LSP today:** `interp eval`'s script is `ArgRole::Body` (`interp.rs:188`) and the
  generic `dispatch_body_arguments` (`commands.rs:988+`) recurses with the UNCHANGED
  parent `scope_path` — no child-interp scope opened. So `proc foo` inside registers as
  a parent-namespace proc, and `foo` calls register as parent invocations.
- **Gap:** false-positive-wrong. References/Rename on the parent `foo` wrongly include
  the child body's `proc foo` and its call; a rename of the parent would edit the child
  body. Documented as a known contract approximation but not scheduled.
- **Repro:** `proc foo {} {puts parent}; set c [interp create]; interp eval $c {proc foo {} {puts child}; foo}`.
- **Milestone:** none. **NEW** — cross-interp isolation; slot into **M7**. (Not
  independently re-verified → PLAUSIBLE, but the danger is a wrong edit.)

### 1.10 Chained interp aliases `A → B → ::real::proc` — **PLAUSIBLE** (medium)

- **Real Tcl:** chained aliases resolve transitively at runtime — each hop is a live,
  global-anchored re-lookup (`rust/tcl-vm/src/exec.rs:2701-2735,2827-2833`;
  `runtime/rust/src/cmd_alias.rs`). Documented static limitation in
  `command-alias-resolution.md`.
- **LSP today — split-brain:** Signature-help follows the chain
  (`signature_help.rs:326-350`, `resolve_alias_chain`, MAX 8 hops, cycle detection) —
  **correct**. Go-to-Definition follows only ONE hop (`definition.rs:222-232`) and
  `resolve_called_proc` is alias-blind (`definition.rs:748-762`), so when `alias.target`
  is itself an alias it returns None → empty. Hover follows one hop
  (`hover.rs:1914-1921`). References / Rename / Call-Hierarchy: zero alias handling.
- **Gap:** false-negative-missed + split-brain. Go-to-Def/Hover on `A` land nowhere /
  on `B`, never `::real::proc`, while Signature-help resolves it — two features disagree
  on the same cursor. Renaming a middle alias `B` must update both its own definition and
  `A`'s target word; no alias model can group these.
- **Milestone:** none. **NEW — M0.5** (transitive extension of the single-hop alias work).

### 1.11 Introspection command-name arguments — **PLAUSIBLE** (low)

`namespace which -command name` / `namespace origin cmd`
(`runtime/rust/src/cmd_namespace.rs:89,106`; contract `command-resolution.md:113`),
and `info args`/`info body`/`info default PROC` (exact-name proc resolution; tclsh
errors `"procname" isn't a procedure` when absent). In all cases the name argument is a
genuine command reference, but the analyser models these only as arity/subcommand syntax
(`commands.rs:2560-2562`, `validity.rs` E003; `info_.rs:217-235,318-324`). There is **no
`ArgRole::CommandName` variant at all** — `arg_role.rs:82` only has `CommandPrefix`.

- **Gap:** false-negative-missed. Rename of a proc does not rewrite these introspection
  sites. Low impact (rare, string-return introspection, abstention not wrong edit).
- **Milestone:** none. **NEW** — introduce an `ArgRole::CommandName` reference role;
  batch with §1.12 traces under **M7**.

### 1.12 `trace add command`/`trace add execution NAME` — **PLAUSIBLE** (low)

- **Real Tcl:** execution/command traces observe dispatch, keyed by the resolved FQN of
  NAME — the literal NAME is a bona-fide command reference. Contract
  `command-resolution.md:~144-147`.
- **LSP today:** `trace_add_arg_roles` tags only the VARIABLE form's name
  (`trace.rs:62-71`, `ArgRole::VarWrite` when the type word is `variable`); the
  `command`/`execution` forms return an empty role vector. No `CommandName` role exists.
- **Gap:** false-negative-missed. Rename of the traced command leaves
  `trace add command ::foo …` dangling. **Contrast §2.6:** `trace add variable` IS
  handled — an asymmetry.
- **Repro:** `proc foo {} {}; trace add command ::foo {rename delete} onFooGone`.
- **Milestone:** none. **NEW — M7** (with the `CommandName` role).

### 1.13 `namespace inscope` / `namespace code` — **PLAUSIBLE** (low)

`namespace inscope ns {script}` evaluates with `ns` current; `namespace code {script}`
captures the current namespace for later. `rust/tcl-vm/src/cmd_namespace.rs:376-392`.
`inscope`'s script is `ArgRole::Body` but has **no** `analyser_hook` to open the `ns`
scope (unlike `namespace eval`'s `NamespaceEval` hook), so the body is walked in the
current namespace (`namespace_.rs:232-242` vs `170-187`). `namespace code` is
`arity exact(1)` with no Body role (`namespace_.rs:121-129`), so its script is not walked
at all.

- **Gap:** both — for `inscope` a wrong-namespace bind (false positive under collision) or
  a miss; for `code` the callback body's references are invisible. Both niche.
- **Milestone:** none. **NEW — M7.**

### 1.14 `namespace path` — Go-to-Def / Hover path-blindness — **PLAUSIBLE** (medium, alreadyKnown #5)

Find-References consumes the path-aware settled `resolved_qualified_name`, but
Go-to-Definition and Hover re-resolve through `proc_visible_from_namespace` →
`bareword_resolution_candidates`, the **path-free** variant (`definition.rs:728-762`),
so they never consider `namespace path` entries.

- **Gap:** both — for a call resolved via a path entry to `::a::helper`, Go-to-Def/Hover
  abstain (if neither current-ns nor global exists) or **jump to an unrelated same-named
  `::helper`** (false positive). This is **known issue #5** (flagged for hover) extending
  identically to `definition.rs`. **M0 explicitly assumes
  `resolve_called_proc`/`proc_visible_from_namespace` is already correct** and only routes
  other sites through it — so M0 does NOT close this.
- **Repro:** `namespace eval ::a { proc helper {} {} }; namespace eval ::b { namespace path ::a; helper }`.
- **Milestone:** none. **NEW** — make the definition/hover path path-aware. Slot as
  **M0.5** (it directly undercuts M0's "already correct" assumption).

### 1.15 REFUTED / SAFE command surfaces

| Surface | Verdict | Why safe |
|---------|---------|----------|
| `$cmd` / `{*}$cmd` / computed head | **REFUTED (safe)** | Dynamic heads deliberately unrecorded (`commands.rs:1133-1136`; `alias.rs:80-83` returns None on `is_dynamic_word`). Honest abstention — the documented **M5** limitation. |
| `interp alias {} al CHILD target` (cross-interp path) | **REFUTED (safe)** | `detect_interp_alias` requires both paths `{}` (`alias.rs:72-74`); child-interp forms return None → no false parent/child link. |
| `load LIB` | **REFUTED (correct)** | No Tcl-source definition exists; no `load` analyser hook → honest abstention. |
| f5-irules dialect verbs (`HTTP::`, `IP::`) | **REFUTED (correct)** | Registry-provided builtins (`commands/irules/`), kept separate from `PackageResolver`; no conflation. |
| `thread::send`/`comm send`/Tk `send { script }` | **PLAUSIBLE (verify-safe)** | Sent scripts resolve in the *target* interp's global ns; honest abstention is correct — but confirm the generic Body walk does not mis-bind sender-namespace barewords. **NEW — M7 (verify).** |

---

## 2. Variable-name surfaces

All confirmed variable gaps map to **M4** (variable-resolution spike). M4 as scoped
enumerates four data models but does NOT call out upvar/namespace-upvar link-following or
TclOO object variables specifically — those are the sharpest defects and should be named
explicit M4 deliverables.

### 2.1 `upvar ?level? otherVar localVar` — **PLAUSIBLE** (high, alreadyKnown) — DANGEROUS (both)

- **Real Tcl:** ONE variable, TWO names — `localVar` and `otherVar` are the same storage
  cell via an explicit cross-frame Link. `runtime/rust/src/vars.rs:706` (`make_upvar` →
  `link_local`, `vars.rs:652`); VM `rust/tcl-vm/src/interp.rs:2433`. Contract
  `command-resolution.md:151`.
- **LSP today:** `handle_upvar_command` (`handlers.rs:1483`) calls `define_var` only on
  the LOCAL alias name as a fresh, disconnected `VarDef`; `otherVar` is never recorded.
  `VarDef` (`types.rs:285`) has no target/alias/link field. References/Hover resolve a
  single `VarDef` (`definition.rs:552`, `references.rs:298-309`).
- **Gap:** both. Rename of `otherVar` misses `localVar` uses (leaves a runtime-live upvar
  alias pointing at the old name — the canonical silent-correctness bug); rename of
  `localVar` misses `otherVar`; Find-References/Hover on either miss the other.
- **Repro:** `proc outer {} { set counter 0; inner }; proc inner {} { upvar 1 counter c; incr c }`.
- **Milestone:** **M4** (explicitly name upvar link-following).

### 2.2 `global v` / `variable v` / `namespace upvar ns v localVar` — **PLAUSIBLE** (high, alreadyKnown)

- **Real Tcl:** each links a frame-local name to a concrete namespace/global cell; every
  `variable v` declaration across a namespace's procs denotes the SAME cell.
  `runtime/rust/src/vars.rs:670,652,727`; contract `command-resolution.md:151`.
- **LSP today:** `handle_global_command` (`handlers.rs:185`), `handle_variable_command`
  (`:208`), `handle_namespace_upvar_command` (`:1509`) each define fresh local `VarDef`s
  with no link. `define_var` (`scope.rs:683`) keys per-scope by bare name, so the
  namespace-scope `::ns::v`, a proc's `variable v`, and a `global v` are all distinct,
  unconnected `VarDef`s.
- **Gap:** both. Renaming `::ns::v` misses the `variable v` decls, `$v` uses, and
  `global`/`namespace upvar` aliases. (Two same-named `variable v` in unrelated namespaces
  are correctly NOT conflated — the intra-namespace link that SHOULD exist is what's
  absent.)
- **Repro:** `namespace eval ::app { variable count 0 }; proc ::app::bump {} { variable count; incr count }`.
- **Milestone:** **M4** (name the namespace-variable link).

### 2.3 `uplevel` — **CONFIRMED** (medium) — DANGEROUS (D2, D3)

- **Real Tcl:** the script's variable AND command names resolve in the TARGET frame's
  namespace/scope. Contract `command-resolution.md:92-95`.
- **LSP today:** `handle_uplevel_command` (`handlers.rs:990`) consumes ONLY the exact
  `['#0', bracedStr]` shape; everything else (`uplevel 1`, bare `uplevel`) falls through to
  `dispatch_body_arguments`, which recurses with the SAME `scope_path`
  (`commands.rs:1116,1119`). `record_var_read`/`define_var` write only into the scope at
  `scope_path` (`scope.rs:637-644,712-731`). For `#0`, a child `Uplevel` scope IS opened,
  but at **query time** `lookup_var_in_scope_chain` (`definition.rs:552-566`) walks the
  FULL chain including the parent `Proc` scope — it lacks the
  `in_uplevel && sc.kind==Proc { continue }` guard that `visible_variable_names` has
  (`definition.rs:607-612`).
- **Gap (D3, non-`#0`):** a `set x`/`$x` inside `uplevel 1 { … }` is attributed to the
  enclosing proc's own scope; a same-named proc-local absorbs a use that at runtime targets
  the caller's frame — wrong reference link + wrong rename edit (false positive), and the
  caller-frame var never gets the link (false negative).
- **Gap (D2, `#0`):** for `proc p {} { set g 99; uplevel #0 { puts $g } }`, cursor on `$g`
  resolves to `p`'s local `g` instead of the global — WRONG, driving Go-to-Def
  (`definition.rs:135`), Hover (`hover.rs:178`), References (`references.rs:298`),
  Document-Highlight (`references.rs:1024`), Rename (`rename.rs:173,599`) to the invisible
  proc-local.
- **Fix:** `#0` — add the missing guard at `definition.rs:552`; non-`#0` — the target frame
  is statically unknown, so abstain rather than mis-attribute (`handlers.rs:980` /
  `commands.rs:1116`).
- **Milestone:** **M4.**

### 2.4 TclOO object variables (`variable v` / `common v` / `my variable v`) — **CONFIRMED** (high) — DANGEROUS (D1)

- **Real Tcl:** at method dispatch, the class's declared vars (`c.variables`) and the
  object's own declared vars are auto-linked as `(v, "{obj_ns}::v")` into EVERY method
  frame (`rust/tcl-vm/src/cmd_oo.rs:1382-1420`, `link_vars`); `my variable v` does the same
  on demand (`cmd_oo.rs:1429-1440`, `add_link`). Because the storage key is identical for
  every method, `$v` in method A and `set v` in method B are the SAME variable spanning all
  method bodies plus the class-body declaration.
- **LSP today:** the declaration seeds a per-method isolated `VarDef` with NO cross-method
  link and — critically — its `definition_span` is the **whole method-body token span**
  (`oo.rs:321-326` passes None → `scope.rs:698` falls back to body span). References/Rename
  resolve a single method's `VarDef` (`definition.rs:552-566`, `references.rs:286-311`,
  `rename.rs:588-647`). Params escape this because they get real name spans
  (`oo.rs:315-317`); object variables do not.
- **Gap — FALSE NEGATIVE:** Find-References/Rename on `$v` in one method returns only that
  method's sites; sibling methods and the `variable v` declaration are missed → rename splits
  one instance variable into two, and the un-renamed declaration never auto-declares the new
  slot.
- **Gap — FALSE POSITIVE (corrupting, D1):** `rename_var` emits a decl edit whose range
  covers the **entire method body** with replacement text = the new variable name
  (`rename.rs:631-635`), so renaming `$v` replaces `{ return $v }` with `w`, **destroying the
  body**. Find-References/Document-Highlight likewise surface the whole method body as the
  declaration range.
- **Repro:** `oo::class create C { variable n; method get {} {return $n}; method set {x} {set n $x} }` — cursor on `$n` in `get`.
- **Milestone:** **M4** — this is a **fifth** variable-resolution context beyond the four M4
  enumerates and must be a named deliverable; the corrupting whole-body-span edit should be
  fixed even ahead of the link work.

### 2.5 `dict with` / `dict update` — **PLAUSIBLE** (low, alreadyKnown)

`handle_dict_with_command` (`handlers.rs:1572`) binds keys ONLY when `args.len()==3` and the
dict word is a const literal (`lookup_const_string`); the common dynamic case
(`dict with $someDict {…}`) binds nothing. `handle_dict_update_command` (`:1548`) binds
alias vars but with no link to the dict/key.

- **Gap:** false-negative-missed. For a dynamic dict, References/Rename/Hover on a key var
  return nothing (**honest abstention — acceptable**, not a wrong edit). `dict update` aliases
  lack a key↔alias link (minor).
- **Milestone:** **M4** (low priority).

### 2.6 REFUTED / handled variable surfaces

| Surface | Verdict | Why |
|---------|---------|-----|
| `trace add variable VAR …` | **REFUTED (handled)** | `trace_add_arg_roles` tags `(1, ArgRole::VarWrite)` (`trace.rs:62-71`); `handle_var_binding_command` records the bareword as a def/reference (`handlers.rs:1378`, `scope.rs:713-760`), so References/Rename include the trace site. Note the asymmetry with §1.12 command traces. |

---

## 3. Class / Method-name surfaces

### 3.1 `forward methodName targetCmd` — **PARTIAL** (medium): CONFIRMED refs/rename, REFUTED go-to-def

- **Real Tcl:** a forward is a call-time command alias — calling the method evaluates
  `targetCmd prefixArgs… callArgs…` with the OBJECT namespace current; the target resolves
  object-ns→global as a real command. `rust/tcl-vm/src/cmd_oo.rs:1988` (`def_forward` stores
  `forward:Some(prefix)`), `cmd_oo.rs:1343-1360`. Contract `command-resolution.md:~86-89`.
- **LSP today:** `apply_oo_forward` (`oo.rs:1165-1187`) records
  `MethodDef{kind:"forward", forward_target:(target_string, args)}` — the target as a STRING
  with NO span; consumed ONLY by arity diagnostics (`var_command.rs:490`). Zero navigation
  code reads `forward_target`.
- **Gap — CONFIRMED (rename, refs, call-hierarchy):** `rename_proc` (`rename.rs:654,709`)
  and the invocation-driven `references`/`call_hierarchy` walk `command_invocations`; the
  forward line's `backend` is never there → rename `backend`→`impl` leaves
  `forward do backend` intact (dangling forward). References/Call-Hierarchy omit the forward
  site.
- **REFUTED (go-to-def):** `definition()` is cursor-word driven, not invocation-driven —
  cursor on `backend` → `resolve_called_proc(...,"backend",...)` (`definition.rs:188`) finds
  the global proc and jumps. Go-to-Definition on the forward target **accidentally works**.
- **Repro:** `proc backend {x} {return $x}; oo::class create C { forward do backend }; set o [C new]; $o do 1`.
- **Milestone:** none. **NEW** — capture the target-token span in `apply_oo_forward`
  (`oo.rs:1170-1181`) and teach invocation-driven consumers to surface it; go-to-def needs no
  change. Slot into **M0.5** (command-alias-following family; a forward target is a command
  link).

### 3.2 `oo::objdefine obj { method … }` per-object body — **CONFIRMED** (medium) — DANGEROUS under collision (D6)

- **Real Tcl:** `oo::objdefine` adds a per-OBJECT method/mixin/forward; `$obj foo` dispatches
  the per-object method ahead of any class method. `rust/tcl-vm/src/cmd_oo.rs:108,1678-1682`;
  e2e `cmd_oo_e2e.rs:654`.
- **LSP today:** `handle_oo_objdefine` (`handlers.rs:1684-1695`) strips `$`/`{}` off the
  object name and inserts it into `objdefined_vars`; **args[1] (the body) is ignored**
  (`commands.rs:627-630` returns false, no body walk). `objdefined_vars` only suppresses the
  W308 unknown-method diagnostic (`var_command.rs:293`). No per-object `MethodDef` is created.
  Contrast `oo::define` (`handlers.rs:1947`), which merges its body into the existing ClassDef.
- **Gap:** false-negative-missed — `$o greet` for a per-object method resolves to nothing
  (`lookup_method_in_class` reads `ClassDef.methods` only, `definition.rs:152-157,291-303`).
  **False positive (D6):** if a class method shares the name, lookup returns the class method's
  span even though the per-object override runs at runtime.
- **Repro:** `oo::class create C {}; set o [C new]; oo::objdefine o { method greet {} {return hi} }; $o greet`.
- **Milestone:** none. **NEW** — requires a per-object symbol store (an object var is not a
  ClassDef), so navigation providers need somewhere to resolve per-object members; this is a
  design gap, not a wiring fix. Slot into **M3-adjacent / M7**.

### 3.3 `next` / `nextto` in an overriding method — **CONFIRMED** (low)

- **Real Tcl:** `next` dispatches the same method one hop up the MRO; `nextto` restarts at a
  named class. `rust/tcl-vm/src/cmd_oo.rs:1525,1560`.
- **LSP today:** Go-to-Definition HANDLES it (`definition.rs:161,313-335` via
  `ClassHierarchy::next_provider`). Find-References / Document-Highlight do NOT — the method
  scan (`references.rs:516,574`) looks only for `my method`/`$obj method`/name tokens
  (grep: no `next`). Rename is UNAFFECTED and correct — `override_family` (`rename.rs:542`)
  already rewrites the method decl in every family member, and `next` carries no name.
- **Gap:** false-negative-missed (References/Highlight omit `next`/`nextto` sites) — benign
  (soundness-preserving, rename already correct), but an inconsistency with Go-to-Def.
- **Repro:** `oo::class create A { method greet {} {return hi} }; oo::class create B { superclass A; method greet {} { next } }`.
- **Milestone:** none. **NEW — M3-adjacent** (low).

### 3.4 `coroutine NAME cmd` — the created command — **PLAUSIBLE** (low)

`coroutine` binds NAME as a live, renameable command
(`rust/tcl-vm/src/cmd_coro.rs:51,109,156`; registry `spec.rs:696`).
`record_registry_defined_command` (`commands.rs:1900-1935`) inserts NAME into
`created_instance_commands` — a spanless `HashSet<String>` (`types.rs:812`) used only for W123
suppression and bare object-method gating (`definition.rs:506`, `references.rs:800`). No
definition location, no reference support: a call to NAME is not linked to the `coroutine NAME`
site. (Its arg 1 command word is also unrecorded — see §1.5.)

- **Gap:** false-negative-missed (honest miss). **Milestone:** none. **NEW — M7.**

### 3.5 Split / late `oo::define` — **REFUTED same-file, CONFIRMED cross-file** (medium)

- **Real Tcl:** `oo::define` mutates the ONE existing class on the def-stack; methods
  accumulate, `oo::define C superclass Base` REPLACES the superclass list; `oo::define` on a
  not-yet-created class is a runtime error. `rust/tcl-vm/src/cmd_oo.rs:1684-1732,1916-1934`.
- **SAME-FILE — REFUTED (works today):** `handle_oo_define_command` pulls the existing ClassDef
  out of `all_classes`, merges method directives (`oo.rs:666/1140/1183/1231`), assigns
  `superclasses` = the replace semantics (`oo.rs:1203-1204`), and re-inserts
  (`handlers.rs:1985-2044`). Type-Hierarchy/Implementation build from the final merged map
  (`type_hierarchy.rs:125`, `implementation.rs:69`). Tests `handlers.rs:4260,4301` pin the merge,
  and the `::`-rooted key (`naming.rs:327-341`) holds it even when create is inside `namespace
  eval N` and define uses `::N::C`.
- **CROSS-FILE — CONFIRMED gap:** when `oo::class create C` is in file A and
  `oo::define ::C { superclass Base; method m … }` is in file B, `handle_oo_define` cannot find
  C in B's `all_classes` and fabricates a **STUB ClassDef** holding only B's members and an empty
  superclass list. `WorkspaceIndex` pushes one `WorkspaceClass` per document with no
  cross-document member merge (`workspace_index.rs:192-205`) → two same-named `::C` entries.
  False negative: cross-file-added method `m` won't unify with A's class or call sites; a late
  cross-file `superclass Base` is invisible to Type-Hierarchy anchored on A's `::C`. False
  positive: the stub is a class DEFINITION whose empty superclasses can feed a wrong supertype
  set.
- **Minor same-file edge (low):** `handle_oo_class_command`'s unconditional
  `all_classes.insert` (`handlers.rs:1921`) overwrites rather than merges, so a re-`oo::class
  create C` wipes accumulated members — bounded to redefinition/invalid code.
- **Milestone:** **M3** (cross-file method references) covers the cross-file merge; call it out
  as an explicit M3 sub-item (stub-vs-real dedup + late cross-file `superclass`).

---

## 4. Cross-file / loading surfaces

### 4.1 `source` into a `namespace eval` — re-homing — **PLAUSIBLE** (medium, alreadyKnown)

- **Real Tcl:** `source` evaluates the file in the caller's CURRENT namespace/frame — bare defs
  in `b.tcl` land in `::x`. `rust/tcl-vm/src/command.rs:227-244` (`cmd_source` → `eval_source`,
  no namespace push); `interp.rs:3220` (runs in current frame). Contract lines 134-137.
- **LSP today:** each document is analysed independently and global-rooted; `source_targets` feed
  only W120 package-require inheritance (`source_graph.rs`, `lib.rs:10385`) and path-literal
  rewriting on file rename — never namespace propagation. `b.tcl`'s `helper` is stored
  `qualified_name=::helper` (`workspace_index.rs:182-190`).
- **Gap:** false-negative-missed, asymmetric. The re-homed proc is stored under the wrong FQN
  (`::helper` not `::x::helper`). (a) Go-to-Definition on a correctly-written qualified call
  `::x::helper` misses (`workspace_index.rs:633-640`). (b) Cross-document rename seeded at the def
  resolves it as `(helper, ::helper)`; `invocations_of` won't match a `::x::helper` call site
  (`workspace_index.rs:668-679,757-779`) — the qualified live call dangles on rename. Bareword
  `helper` calls inside `::x` still match via the bare-safe fallback, so the gap **bites exactly
  the callers who write the correct qualified name**. Critically **M2 and M6 as scoped do NOT fix
  this** — M2's oracle checks candidate existence over the merged index, but the index has
  `::helper` not `::x::helper`, so `workspace_command_exists(::x::helper)` is false; M6 homes a
  library file's defs by that file's own namespace, not the source SITE's.
- **Repro:** `a.tcl: namespace eval ::x { source b.tcl }` then `::x::helper`; `b.tcl: proc helper {} {}`.
- **Milestone:** none. **NEW** — "source-site namespace propagation": re-home a sourced file's
  global-scope defs under the namespace active at the literal `source` call site. Slot as **M6.5**
  (depends on the M2 oracle and the M6 lazy-analyse tier).

### 4.2 `package require` / autoload / `tclIndex` — non-navigable resolved procs — **PLAUSIBLE** (medium, alreadyKnown)

`PackageResolver` resolves these to files but ONLY for diagnostics —
`auto_loads_command`/`package_defined_commands` feed W123 refinement
(`lib.rs:1018-1045,10307-10370`). Nothing analyses a resolved package/autoload file into
`WorkspaceIndex`, so `cross_document_definition` (`lib.rs:3555-3587`) queries only already-analysed
docs.

- **Gap:** false-negative-missed. Go-to-Def / References / Call-Hierarchy on a call resolved by a
  package or `tclIndex` returns nothing even though `PackageResolver` knows the exact defining file.
  W123 is correctly suppressed, so the command isn't flagged — just non-navigable.
- **Repro:** `package require http` then `http::geturl $u`.
- **Milestone:** **M6** (lazy-analyse-on-oracle-miss — this is exactly that tier).

### 4.3 `source [file join $dir x.tcl]` — computed path — **PLAUSIBLE** (low, alreadyKnown)

`handle_source_command` marks any path containing `$` or `[` as `is_literal=false`
(`handlers.rs:2063-2072`); the source graph follows only literal edges. The reusable
`auto_path_eval` mini-evaluator (`auto_path_eval.rs:57-72`), which can fold
`[file join [file dirname [info script]] …]`, is used for `lappend auto_path` but NOT reused for
source paths.

- **Gap:** false-negative-missed for W120 inheritance (already accepted/tested). For navigation the
  impact is smaller — source edges don't drive cross-file command re-homing anyway (§4.1), so even a
  literal edge wouldn't help navigation today. A precision opportunity, not a correctness bug.
- **Milestone:** none. **NEW** — route source paths through `auto_path_eval`-style folding.
  Low-priority follow-up to **M6.5**.

### 4.4 REFUTED / correct loading surfaces

`load LIB` (opaque C commands, honest abstention) and f5-irules dialect verbs (registry builtins,
kept separate from package resolution) — both **REFUTED (correct)**. See §1.15.

---

## 5. Summary table — every surface

Legend: **FN** = false-negative-missed, **FP** = false-positive-wrong, **both** = both directions,
**none** = no gap (safe). Sev = severity.

| # | Surface | Kind | Gap | Sev | Tag | Milestone |
|---|---------|------|-----|-----|-----|-----------|
| 1.1 | `interp alias` target link | command | FN | high | CONFIRMED | none → M0.5 |
| 1.2 | `rename old new` | command | FN | high | CONFIRMED | none → M0.5 |
| 1.3 | `namespace import` | command | FN | medium | CONFIRMED | none → M0.5 |
| 1.4 | `namespace ensemble` `-map`/sub dispatch | command | FN | medium | CONFIRMED | none → M7 |
| 1.4b | ensemble `-unknown`/`-map` target names | command | FN | low | PLAUSIBLE | none → M7 |
| 1.5 | `tailcall CMD` (+ `coroutine` arg1) | command | FN | medium | CONFIRMED | none → M0.5 |
| 1.6 | `expr {f($x)}` user mathfunc | command | FN | medium | CONFIRMED | none → M7 |
| 1.7 | literal cmd names in dispatch tables | command | FN | medium | CONFIRMED | none → M5-adj |
| 1.8 | nested `proc`/class re-homing (qualified encloser) | command/class/method | both | medium | CONFIRMED | none → M0.5 |
| 1.9 | `interp eval CHILD` merge | command/variable | FP | medium | PLAUSIBLE | none → M7 |
| 1.10 | chained interp aliases | command | FN | medium | PLAUSIBLE | none → M0.5 |
| 1.11 | `namespace origin/which`, `info args/body/default` | command | FN | low | PLAUSIBLE | none → M7 |
| 1.12 | `trace add command/execution` | command | FN | low | PLAUSIBLE | none → M7 |
| 1.13 | `namespace inscope` / `namespace code` | command/variable | both | low | PLAUSIBLE | none → M7 |
| 1.14 | `namespace path` in Go-to-Def / Hover | command | both | medium | PLAUSIBLE (#5) | none → M0.5 |
| 1.15a | `$cmd` dynamic head | command | none | — | REFUTED-safe | M5 |
| 1.15b | cross-interp alias path | command | none | — | REFUTED-safe | none |
| 1.15c | `load LIB` | command | none | — | REFUTED-correct | none |
| 1.15d | dialect verbs | command | none | — | REFUTED-correct | none |
| 1.15e | `thread::send`/`comm`/Tk `send` | command/variable | none? | — | PLAUSIBLE-verify | none → M7 |
| 2.1 | `upvar` | variable | both | high | PLAUSIBLE (known) | M4 |
| 2.2 | `global`/`variable`/`namespace upvar` | variable | both | high | PLAUSIBLE (known) | M4 |
| 2.3 | `uplevel` #0 and non-#0 | variable(/command) | both | medium | CONFIRMED | M4 |
| 2.4 | TclOO object variables | variable | both | high | CONFIRMED | M4 |
| 2.5 | `dict with`/`dict update` | variable | FN | low | PLAUSIBLE (known) | M4 |
| 2.6 | `trace add variable` | variable | none | — | REFUTED-handled | — |
| 3.1 | `forward` target | method/command | FN | medium | PARTIAL (refs CONFIRMED, gtd REFUTED) | none → M0.5 |
| 3.2 | `oo::objdefine` per-object | method/class | both | medium | CONFIRMED | none → M7 |
| 3.3 | `next`/`nextto` in refs | method | FN | low | CONFIRMED | none → M3-adj |
| 3.4 | `coroutine NAME` created command | command | FN | low | PLAUSIBLE | none → M7 |
| 3.5 | split/late `oo::define` (cross-file) | class/method | both | medium | REFUTED same-file / CONFIRMED cross-file | M3 |
| 4.1 | `source` into `namespace eval` re-homing | command/class/method | FN | medium | PLAUSIBLE (known) | none → M6.5 |
| 4.2 | `package require`/autoload/`tclIndex` | command | FN | medium | PLAUSIBLE (known) | M6 |
| 4.3 | `source` computed path | command | FN | low | PLAUSIBLE (known) | none → M6.5 |
| 4.4a | `load LIB` | command | none | — | REFUTED-correct | none |
| 4.4b | dialect libraries | command | none | — | REFUTED-correct | none |

---

## 6. Proposed NEW plan items

The confirmed/plausible gaps not covered by M0–M6 cluster into four new work items. Suggested
milestone slots are ordered by (severity × independence), matching the fix-plan's convention.

| New item | Surfaces | What it does | Slot | Depends on |
|----------|----------|--------------|------|------------|
| **M0.5 — command name-link following** | 1.1, 1.2, 1.3, 1.5, 1.8, 1.10, 1.14, 3.1 | Make References/Rename/Call-Hierarchy (and the Go-to-Def/Hover path) consult `command_aliases`, `renamed_commands`, `namespace_imports`, forward targets, and the priority-ordered candidate list, so an alias/import/rename/forward is a followed reference and the definition/hover path becomes path-aware. Requires the analyser to record the missing **spans** (alias target word, `rename OLD NEW` args, import pattern, forward target token) (nested-def homing is already wired to `command_resolution_namespace` — see §1.8). Silent-correctness class — highest priority after M0. | **M0.5** (right after M0; several sub-items directly repair M0's "resolver already correct" assumption, esp. 1.14) | M0 (shared resolver in place) |
| **M4 explicit deliverables** | 2.1, 2.2, 2.3, 2.4 | Name upvar / global / variable / namespace-upvar **link-following** and TclOO **object variables** as first-class M4 deliverables (the spike text enumerates data models but not these link kinds). Fix the D1 whole-body-span rename corruption and the D2/D3 uplevel mis-scope even ahead of the broader spike. | within **M4** | — (M4 is a spike) |
| **M3 sub-item — cross-file `oo::define` merge** | 3.5, 3.2, 3.3 | Dedup the cross-file `oo::define ::C` stub against the real ClassDef; honor a late cross-file `superclass`; add a per-object symbol store for `oo::objdefine`; add `next`/`nextto` reference sites. | within **M3** (per-object store may spill to M7) | M1, M2 |
| **M6.5 — source-site namespace propagation** | 4.1, 4.3 | Re-home a sourced file's global-scope defs under the namespace active at the literal `source` call site; reuse `auto_path_eval` folding for computed source paths. | **M6.5** (after M6's lazy tier and M2's oracle) | M2, M6 |
| **M7 — dynamic-surface reference roles & isolation** | 1.4, 1.4b, 1.6, 1.9, 1.11, 1.12, 1.13, 1.15e, 3.2, 3.4 | Introduce an `ArgRole::CommandName` reference role (traces, `namespace which/origin`, `info args/body/default`); model ensemble `-map`/`-subcommands`/`-unknown` and expr user-mathfuncs as reference sites; open per-interp/`inscope` scopes so cross-interp and `namespace inscope` bodies don't mis-bind (the `interp eval` false positive 1.9 and the `send` family). Lower-severity, mostly missed-refs plus the 1.9 wrong-edit. | **M7** | M0.5 (role plumbing), M2 |

**Highest-leverage single fixes**, ranked by corruption risk:

1. **D1** (TclOO object-variable whole-body-span rename) — actively destroys source text;
   fix `rename.rs:631-635` / `oo.rs:321-326` to seed real name spans, independent of the M4 spike.
2. **D2** (`uplevel #0` query guard) — one-line: apply the `definition.rs:607-612` guard at
   `definition.rs:552`.
3. **1.1 / 1.2 / 1.3** (alias / rename / import following) — the canonical "rename leaves a
   runtime-live binding pointing at the old name" class the core question calls out; all three need
   the same span-recording + provider-wiring pattern (M0.5).
