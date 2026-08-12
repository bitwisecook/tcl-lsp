# Tk widget instance-command typing

How a Tk/ttk widget's *instance*
command (`.t instate …`, `$w tag configure …`, `$listbox curselection`) gets
resolved back to the widget class that created it, so the registry's real
`SubCommand` table for that widget becomes reachable from highlighting,
hover, completion and diagnostics. It is the sibling of
[`tcloo-object-typing.md`](tcloo-object-typing.md) for Tk: same underlying
question ("what does this receiver dispatch to?"), different identity model.

## The problem

`ttk::treeview .t` creates a *new* Tcl command named `.t` whose subcommands
(`instate`, `tag`, …) are exactly the registry's `ttk::treeview` `CommandSpec`
subcommands. Without something connecting the two, `.t instate {selected}
{ … }` and `$w tag configure hidden -foreground grey` get no subcommand-aware
highlighting, hover, completion, or arity / unknown-subcommand diagnostics,
for any widget whose `CommandSpec::subcommands` is non-empty (`treeview`,
`notebook`, `listbox`, `text`, `canvas`, `entry`, …).

`tcloo-object-typing.md`'s own corpus evidence (`experiments/tcloo_diag`)
shows this is not a corner case: of 8241 unresolved `$var` receivers, 71%
are "unbound", **dominated by snit `$self`/components and Tk widget paths
(`$win.c`) — not TclOO**.

## Why this needs its own model, not TclOO's

A TclOO object's identity is a *value* — `$obj` holds an opaque handle,
tracked through the SSA type lattice (`OBJECT(class)`) and the interprocedural
VTA-lite union (`object_types::object_handle_classes`). A Tk widget's
identity is a **path string** — `.t` is not a value flowing through
assignment so much as a name that becomes independently dispatchable the
moment `ttk::treeview .t` runs, and that name is reused *as source text*
(bareword or via a variable holding the identical string) everywhere else.
Two consequences:

- **Namespaces don't apply, but interpreters do.** The Tk window hierarchy is
  not namespace-scoped — unlike a command or class name, `.t` means the same
  window regardless of which Tcl namespace the referencing code runs in.
  Widget-path identity needs no namespace-qualification step (the
  *constructor command name*, `ttk::treeview` itself, still resolves via the
  normal namespace-aware command-resolution algorithm in `tcl_syntax::naming`
  — that part is unchanged and reused as-is).  It is, however, scoped to the
  **interpreter**: `TkCreateMainWindow` (Tk 9.0.4 `generic/tkWindow.c`) gives
  every interpreter that initialises Tk a fresh `TkMainInfo` with its own
  widget-path `nameTable` and its own `.` root, and both `Tk_NameToWindow` and
  the geometry managers' `TkSetGeometryContainer` resolve through that
  per-application table.  So `.t` in a `child eval { … }` body and `.t` in the
  parent script are two unrelated windows.  See
  [Interpreter domains](#interpreter-domains-issue-1141) below.
- **No MRO.** Every Tk/ttk widget `CommandSpec` is already a complete,
  self-contained subcommand table (confirmed: `ttk::treeview` and
  `ttk::notebook` each separately declare their own `instate`, rather than
  inheriting it from a shared `ttk::widget` base spec). Dispatch resolution
  for a widget instance needs no class-hierarchy walk — the widget's own
  spec **is** the answer.

That second point means the registry-side fix is a **self-referential**
binding: the object class a widget's constructor creates is the constructor
command's own spec, not a separate class. `ObjectClassSpec::instance_methods`
and `CommandSpec::subcommands` are already the same type
(`&'static [SubCommand]`), so a widget's `ObjectClassSpec` can point at the
literal same `SUBCOMMANDS` static its own `CommandSpec::subcommands` uses —
zero duplication, zero drift risk, and `CommandRegistry::instance_method`
resolves it with **no code changes**.

## Existing machinery this reuses (verified on this branch)

- `CommandSpec::creates_instance_at: Option<u8>` + `CommandSpec::object_class:
  Option<&'static ObjectClassSpec>` (`rust/tcl-registry/src/spec.rs`) already
  exist and are already registry-driven, not hardcoded — used today by
  `report::report`, `struct::graph`/`struct::tree` (`tcl-registry/src/commands/tcllib/`)
  and `ticklecharts`. Zero Tk widget files populate them.
- `Analyser::record_registry_factory_instance`
  (`rust/tcl-compiler/src/analyser/commands.rs`) already reads these two
  fields generically for *any* command and writes the bareword name into
  `AnalysisResult::instance_classes: HashMap<String, String>` and
  `AnalysisResult::created_instance_commands: HashSet<String>` — the same
  sets TclOO's `CLASS create NAME` idiom feeds. `is_plain_created_name`
  already accepts a leading `.` (only rejects `%`, `$[]{}() "` and empty).
  **Tk widget constructors are syntactically identical to the tcllib
  `struct::graph g` shape** (positional name, no `new`/`create` keyword) —
  so bareword tracking needs *zero new analyser code*, only registry data.
- `definition.rs::receiver_instance_class` + `created_instance_commands`
  already resolve a **bareword** receiver's class and are already consumed
  by go-to-definition, find-references, and (via the same shared resolver)
  hover's entry point — proven by the existing test
  `definition_resolves_bare_created_instance_command_method` (`Dog create
  rex` → `rex bark` jumps to the method). Only `obj_method_hover_text`
  itself needs extending (it only renders from `analysis.all_classes` —
  user classes — never the registry).
- `semantic_tokens.rs::insert_object_method_overrides` handles `$var`
  (via `object_types::object_handle_classes`) and `[cmd]` receivers, but its
  receiver-token match has a bare `_ => Vec::new()` arm — bareword heads
  never reach a lookup. This is the literal gap the golden fixture pins:
  `"named_object", // TODO(phase-3): resolve via created_instance_commands`.
- `completion.rs` has **no** bareword path and **no** registry-class path at
  all (`strip_instance_var` only accepts `$`/`${...}`; `oo_method_completions`
  reads only `analysis.all_classes`).
- `validity.rs`'s W001 (`emit_w001_unknown_subcommand`) and E002/E003
  (`emit_arity_diagnostics`) both resolve `cmd_name` fresh via
  `registry.get_for_dialect` on every call, with no notion of a
  variable-tracked class. **The `.`-prefix bail-out at `validity.rs:653-662`
  is unrelated to this gap** — it guards `<ensemble> .path` (`grid .w`,
  `entry .e`), i.e. an argument that's a path being *created*, not a command
  head being dispatched. `.t instate …` already silently abstains today via
  the ordinary "no registry spec literally named `.t`" path
  (`validity.rs:637-641`), not the dot-check. So no existing bail-out needs
  removing — a *new* resolution step is what's missing.
- TclOO's own W308 (unknown method) / E001 (bare dispatch) / E002-E003
  (method arity) in `var_command.rs` is the template for how to add a
  **sound**, ambiguity-abstaining diagnostic on top of a class-tracking map:
  it only fires when the receiver's class set has exactly one member
  (`class_names.len() == 1`), never on `{*}`-expanded calls, and reuses
  `validity::arity_verdict`/`shift_arity` so wording matches the ordinary
  registry-command diagnostics.
- `object_types.rs`'s `harvest_unit` already reads `creates_instance_at`/
  `object_class` generically for its own (unsound, highlight-only,
  doc-level-union) `$var` tracking — extending it to widgets needs no new
  matching logic, only the same registry data.
- `type_infer.rs` **deliberately never lattice-types factory-return values**
  (`type_infer.rs:222-228`): doing so would leak one call's class onto a
  same-named variable in another proc via `var_command::aggregate_object_types`'s
  object-insensitive aggregation (this is the FP-OBJ-04 regression the
  comment names explicitly). Factory-return provenance is kept in the
  syntactic, highlight-only `object_types::object_handle_classes` map
  instead. **This fix follows the same discipline**: widget constructors are
  not added to the SSA type lattice.

## The model

1. **Registry data** (~34 widget-constructor `CommandSpec`s under
   `rust/tcl-registry/src/commands/tk/`): each gets
   `creates_instance_at: Some(0)`. The 9 with a real `SUBCOMMANDS` table
   (`treeview`, `notebook`, `listbox`, `text`, `canvas`, `entry`, `menu`,
   `panedwindow`, `spinbox`) additionally get a self-referential
   `object_class: Some(&<WIDGET>_CLASS)` whose `instance_methods` is the
   *same* `SUBCOMMANDS` slice the spec's own `subcommands` field already
   uses — no duplication. The remaining ~25 option-only widgets (`button`,
   `frame`, `label`, …) get `creates_instance_at` alone (`object_class`
   stays `None`): there is nothing to dispatch against yet, but the
   binding is still useful for definition/references/W123 suppression via
   `record_registry_factory_instance`'s existing `cmd_name` fallback.
2. **`tk_checks.rs`'s hardcoded `WIDGET_COMMANDS` list** replaced by
   `Analyser::is_widget_command`, a registry query
   (`creates_instance_at.is_some() && required_package == Some("Tk")`).
   Not optional cleanup: the hardcoded list had already drifted from the
   registry — it named `"ttk::scrollbar"` and `"ttk::labelframe"`, neither
   of which has a registered `CommandSpec` on this branch. One source of
   truth removes the drift class, not just this instance of it.
3. **Everything downstream of `creates_instance_at`/`object_class` needed
   zero new tracking code** — `commands.rs::record_registry_factory_instance`
   and `object_types.rs::harvest_unit` already read these two fields
   generically (built for tcllib factories), so both the bareword case
   (`ttk::treeview .t` then `.t instate …`) and the `set w [ctor .path]`
   return-value-capture case (`commands.rs::registry_factory_class_from_subst`,
   likewise already generic) work from registry data alone. Proven directly
   by tests added to `object_types.rs` and `commands.rs`, with no
   production-code change in either file for this part.
4. **Bareword receiver support in `insert_object_method_overrides`**
   (`semantic_tokens.rs`): a new `TokenType::Esc` match arm that queries the
   *same* `object_classes: &ObjectClassMap` parameter the existing `$var`
   arm already reads (point 3 means it already contains widget bareword
   bindings) — no new parameter threading. Also closes the golden fixture's
   long-standing bareword TODO for the *registry* case; the separate
   `named_object` pure-user-TclOO-class case (`C create obj; obj mrun`)
   is unaffected and correctly stays `Abstain` (`object_types.rs` has no
   `all_classes` access, by design — see Non-goals).
5. **`obj_method_hover_text` gains a registry fallback**: when
   `analysis.all_classes.get(class)` misses, try
   `registry.instance_method(class, method)` before giving up. Required a
   new `registry: Option<&CommandRegistry>` parameter (already available at
   the one call site).
6. **`completion.rs` gains both**: bareword-receiver detection, via
   `crate::definition::receiver_instance_class` (the exact function
   go-to-definition/hover already share — not a new resolver), and a new
   `registry_method_completions` alongside the renamed, registry-aware
   `method_completions` (formerly the user-class-only `oo_method_completions`).
7. **A new widget-instance diagnostic module**,
   `analyser/diagnostics/widget_command.rs`, structurally parallel to
   `var_command.rs`'s `TclOO` W308/E001/E002-E003 trio but two-phase like
   `tk_checks.rs`'s own TK1001 flush (not inline): `validity.rs`'s
   `emit_w001_unknown_subcommand` *records* a `WidgetDispatchSite` at the
   exact point it would otherwise silently abstain (`cmd_name` isn't a
   registered command); `Analyser::flush_widget_dispatch_diagnostics`
   resolves every recorded site after the whole file has been walked, so a
   helper proc *defined* before the widget it dispatches (but not *called*
   until after) still resolves — pinned by
   `resolves_when_widget_created_after_the_proc_that_uses_it_is_defined`.
   Reuses the W001/E002/E003 codes (an unknown-subcommand / arity problem,
   not a new diagnostic class) and `validity::arity_verdict` for identical
   wording to every other arity check. `configure`/`cget` are treated as
   universally valid and never arity-checked (see Non-goals) since no
   widget spec declares them.
8. **`instance_classes` collision-safety, scoped to the registry-driven
   binding sites only.** The diagnostic in (7) cannot safely trust
   `instance_classes`' existing whole-file, last-write-wins contract (two
   different procs could legitimately create two *different* widget
   classes under the same literal path, e.g. `.t`) — that would repeat the
   exact class of bug `docs/design/tcloo-object-typing.md` and the
   `FP-OBJ-04` precedent (`experiments/`, `fp/obj.rs`) warn against for
   interprocedural unions. `Analyser::bind_registry_instance_class`
   (`commands.rs`) makes exactly the two registry-driven insertion sites
   inside `record_registry_factory_instance` collision-aware: a name bound
   to two different classes anywhere in the file is dropped from
   `instance_classes` and tracked in the new
   `AnalysisResult::ambiguous_instance_names`, never re-added. The `TclOO`
   user-class binding sites in `record_instance_creation` (Patterns A/B)
   are untouched, keeping their long-documented best-effort contract
   exactly as before — this is a narrow, additive safety improvement, not
   a semantic change to the shared field.

## Interpreter domains (issue #1141)

TK1001 (geometry-manager conflict) and TK1002 (widget path references a
non-existent parent) are not questions about *source text* — they are
questions about **Tk runtime state**: "has this parent window been created
yet?", "has this container already been claimed by another geometry
manager?".  That state is per-interpreter.

What C Tk actually does (Tk 9.0.4 sources, read directly — Tk cannot be run
headless in this environment, so this half is source-derived, not
live-tested):

- `TkCreateMainWindow` (`generic/tkWindow.c`) runs once **per interpreter**
  that initialises Tk — `Tk_Init` / `Tk_SafeInit` → `Initialize` →
  `TkCreateMainWindow`.  It allocates a fresh `TkMainInfo`, calls
  `Tcl_InitHashTable(&mainPtr->nameTable, TCL_STRING_KEYS)`, and seeds it
  with its own `"."` root entry.
- `Tk_NameToWindow` resolves a widget path through
  `((TkWindow *) tkwin)->mainPtr->nameTable` — that per-application table,
  never a global one.
- `pack`'s container bookkeeping ends in
  `TkSetGeometryContainer(interp, containerPtr->tkwin, "pack")`
  (`generic/tkPack.c`), i.e. the "cannot use geometry manager X inside Y"
  claim lives on a `TkWindow` drawn from that same table.

So `interp create child; load {} Tk child; child eval { frame .top }` and a
parent-side `frame .top` create **two unrelated windows that share a path
string**.  The interpreter isolation this rests on is verified live against
tclsh 9.0.4: a command created inside `child eval { … }` never appears in the
parent's `info commands`, and a widget-creation command *is* how a widget
path becomes a command.

Before #1141 the analyser held one flat, file-wide `tk_created_widgets` /
`tk_geometry` pair, so both diagnostics were decided across every
interpreter at once — a false TK1001 (a parent-side `pack` "conflicting"
with a child-side `grid`) and a missed TK1002 (a parent created in one
interpreter vouching for a child widget in another).

The fix keys the accumulator by **interpreter domain**
(`Analyser::tk_domains: BTreeMap<String, TkDomainState>` in
`analyser/tk_checks.rs`), reusing the campaign's existing synthetic-key
mechanism rather than inventing a parallel one:

- The domain identity is the same `@interp@<path>[#<epoch>]` name
  `isolate_interp_eval_body` (`analyser/handlers.rs`) already mints for the
  synthetic scope a child body's procs and variables home under — the one
  place both `interp eval PATH { … }` and the handle form `NAME eval { … }`
  pass through.  The main interpreter is the empty key.
- That helper now pushes an `InterpFrame { key, domain, resolved }` rather
  than a bare path string, so any analyser state that models per-interpreter
  runtime state can ask "which interpreter am I in?" without a second,
  drift-prone notion of interpreter identity.
- Folding the deletion *epoch* into the identity means `interp delete c;
  interp create c` genuinely ends the hierarchy: the recreated child starts
  empty, as it does in C.
- What creates or targets an interpreter still comes entirely from registry
  data — the `InterpCreate` / `InterpDelete` / `InterpEval` /
  `InterpAlias` / `InterpHide` / `InterpExpose` `AnalyserHookId`s stamped on
  `interp`'s subcommands, plus the handle form recognised from the tracked
  `interpreters` map.  No command name is matched in the analyser.

**Unknowable targets widen rather than accuse.**  `interp eval $i { … }`
whose `$i` cannot be resolved (not a tracked `set VAR [interp create …]`
binding) still gets its own domain — two such bodies must not merge — but
the frame is marked `resolved: false`.  TK1002's existence question then
treats an unresolved domain on *either* side as possibly the same
interpreter, so a widget created there suppresses the warning.  A warning's
false positive is the expensive direction, and an unknowable handle is
exactly where the analyser has no grounds to insist a parent is missing.
TK1001 stays strictly per-domain in every case (never merging an unresolved
domain into another), for the same reason: a conflict we cannot justify is
not worth reporting.

**Known limit: Tk activation is still whole-file.**  `package require Tk`
loads Tk into the interpreter that runs it, so `interp create c;
c eval { package require Tk }` makes Tk available in `c` and *not* in the
parent.  `Analyser::has_tk_require` reads `result.package_requires` for the
whole file, so a child-only load activates the TK checks everywhere.  Left
as-is deliberately: the alternative — gating activation on the main
interpreter — silently drops the genuine diagnostics inside the child, which
is the worse failure, and making activation itself per-domain needs a model
of `load {} Tk child` / `::safe::loadTk` that nothing else in the analyser
wants yet.  Over-activation costs nothing on a file that has no Tk-shaped
commands in its non-Tk domains, which is every realistic file.

**Known limit: receiver typing.** `AnalysisResult::instance_classes` — the
*receiver typing* map this document's W001/E002/E003 work is built on — is
still whole-file
and name-keyed, not interpreter-keyed.  A `.t` created as a `ttk::treeview`
in the parent and as a `listbox` in a child collapses to one entry.  That is
harmless in the direction that matters: `bind_registry_instance_class` drops
a name bound to two *different* classes entirely, so the receiver simply
fails to resolve (silent abstention).  Two same-class widgets in two
interpreters share a subcommand table anyway, so nothing is lost there
either.  Making it interpreter-keyed is a real (if low-value) follow-up; it
is recorded here rather than fixed because the map's collision-dropping
already fails closed, unlike the TK1001/TK1002 accumulators, which failed
*open* in both directions.

## Deliberate abstentions

Consistent with this codebase's stated philosophy — *"prefer silence over a
wrong narrowing at every stage"* (`tcloo-object-typing.md`) and *"the
compiler only inlines what it can prove is safe"* (`AGENTS.md`) — the
following are explicit non-goals, not accidental gaps:

- **`set w .t` (a bare literal path assigned to a variable, with the
  constructor called separately, possibly on `.t` directly rather than via
  `$w`)** needs a two-hop fact (constant-propagate `$w` to the literal `.t`,
  then look up `.t`'s creating class) that SCCP's existing `LatticeValue::Const`
  could in principle supply. It is not wired: the dominant real-world shape
  is direct bareword reuse or `set w [ctor .path]` capture, both of which
  are covered. Nothing false-positives as a result — the receiver simply
  stays unresolved.
- **`rename`** of a widget's instance command (`rename .t .oldT`) breaks the
  association — `instance_classes` is keyed on the name observed at
  creation time. Tcl itself allows this (the C-level widget survives; only
  its Tcl command name changes), but it is rare in real code and the
  existing command-resolution contract treats `rename` uniformly as
  "mutates the command table" for *command* dispatch, not receiver typing.
  Abstain after a `rename` of a tracked path; do not attempt to follow it.
- **`interp alias`** onto a widget path is a command-table fact, not a
  value-flow fact, and is out of scope for the same reason.
- **Safe/sub-interpreters** (`interp create`, `interp eval $child {…}`) for
  **receiver typing**: `instance_classes` is still whole-file and
  name-keyed, not interpreter-keyed — see
  [Interpreter domains](#interpreter-domains-issue-1141) for why that is
  fail-closed here.  The *window
  hierarchy* half (TK1001 / TK1002) is no longer deferred: it is keyed by
  interpreter domain.
- **Interprocedural flow for diagnostics** (a widget path passed as a proc
  argument, `proc configureWidget {w} { $w instate … }`): the diagnostic's
  receiver resolution is whole-file (via `instance_classes`), not
  proc-scoped like `var_command.rs`'s own TclOO tracking — but made *sound*
  for that wider scope by collision-dropping (item 8 above) rather than by
  narrowing the scope. A widget path threaded through a proc parameter
  therefore *does* get diagnosed today (unlike TclOO's `$obj` parameters,
  which `var_command.rs` never resolves at all), as long as its literal
  creation-time name is never reused for a different widget class
  elsewhere in the file. `object_types.rs`'s separate proc-parameter
  propagation edge (highlight-only) is unaffected either way — it was
  already unsound-for-diagnostics by the same FP-OBJ-04 reasoning that
  governs every other class it carries, so it stays highlight-only,
  consistent with everything else that flows through it.
- **`{*}`-expanded calls** never resolve (matches the existing
  `!site.has_expand` gate in `var_command.rs`).
- **`upvar`-aliased widget-path variables** are untyped by construction:
  `type_infer.rs::is_scope_alias_call` already widens `upvar`/`global`/
  `variable`/`namespace upvar` targets away from a nominal type, and this
  change adds no separate tracking that would re-introduce one.
- **`unknown`, tracing (`trace add command`/`trace add variable`), `source`,
  `package require` gating beyond the registry's existing per-command
  `dialects`/`required_package` fields, `auto_index`/`auto_load`, and the
  `::tcl`/`::mathop` namespaces** have no interaction with this mechanism:
  widget creation is always a direct call to a real, statically-named
  registry command, never reached through the `unknown` fallback or
  auto-loading, and namespace-qualification of the *constructor* command
  name (as opposed to the widget path it creates) is handled by the shared
  `tcl_syntax::naming` resolver.
- **`class_lattice.rs`**, the TclOO class-*set* lattice prototype, is not
  part of this model. Widget-path identity is string-keyed with no MRO, so
  it does not need the prototype's machinery.

## Consumer summary

| Consumer | Bareword | `$var` (constructor return) | Registry-class rendering |
|---|---|---|---|
| go-to-definition / references | already worked (existing `receiver_instance_class`) | already worked (`registry_factory_class_from_subst` was already generic) | n/a (jumps to registry doc, not source) |
| hover | already resolved the receiver; needed a registry fallback (item 5) | same | item 5 |
| semantic tokens | new bareword arm (item 4) | already worked via `object_types.rs`'s existing generic `creates_instance_at` read | already worked (`registry.instance_method`) |
| completion | new (item 6) | new (item 6) | new (item 6) |
| diagnostics (W001/E002/E003) | new (item 7), whole-file + collision-safe (item 8) | new (item 7), same | new (item 7) |

## Testing

- **Registry** (`tcl-registry/tests/registry_commands.rs`):
  `tk_widget_constructors_declare_creates_instance_at` (all 34 widgets) and
  `tk_widgets_with_subcommands_self_reference_their_object_class` (the 9
  with a real subcommand table — round-trips `object_class.instance_methods`
  against `subcommands` by literal slice identity, `std::ptr::eq`, and
  proves cross-widget subcommand names don't leak, e.g. `ttk::treeview`
  never accepts `curselection`).
- **Propagation** (`tcl-compiler/src/object_types.rs`,
  `analyser/commands.rs` inline `#[cfg(test)]` modules):
  `bareword_widget_path_is_a_handle` / `var_captured_widget_path_is_a_handle`
  (highlight-only map); `bareword_widget_constructor_binds_instance_class` /
  `var_captured_widget_constructor_binds_instance_class` /
  `simple_widget_without_subcommands_still_binds_instance_class`
  (`instance_classes`/`created_instance_commands`) — all pass with zero
  production-code changes beyond the registry data, proving item 3 above.
- **Semantic tokens** (`tcl-lsp-core/src/semantic_tokens.rs`): two new rows
  (`widget_bareword`, `widget_var_captured`) in the shared
  `TCLOO_DISPATCH_CASES` golden fixture table, run by the existing
  `tcloo_dispatch_pattern_fixture` test alongside every prior TclOO/snit/itcl
  case (unchanged pass/abstain verdicts).
- **Hover** (`hover.rs`): `obj_method_hover_fires_for_bareword_widget`,
  `obj_method_hover_fires_for_var_captured_widget`,
  `obj_method_hover_none_for_widget_without_registry` (no panic without a
  registry — the fallback is `Option`-gated, not a hard dependency).
- **Completion** (`completion.rs`):
  `widget_bareword_completion_offers_subcommands`,
  `widget_var_captured_completion_offers_subcommands`, and the critical
  soundness proof `bareword_completion_does_not_leak_unrelated_variable_class`
  (an unrelated `set b [Bar new]` must not make bareword `b` complete as
  `Bar`).
- **`tk_checks.rs`**: all 9 pre-existing TK1001/TK1002/TK1003 tests stay
  green unchanged, plus two new ones —
  `tk1002_fires_for_ttk_and_listbox_constructors` (coverage beyond the one
  widget the old suite happened to test) and
  `unknown_command_is_never_treated_as_a_widget_constructor` (the
  registry-driven query cannot drift the way the old hardcoded list did).
- **Widget diagnostics** (`analyser/diagnostics/widget_command.rs`, 11
  tests): positive W001/E002/E003 firing for both bareword and `$var`
  receivers; silence for a known subcommand and for `configure`/`cget`;
  **`abstains_when_receiver_is_ambiguous_across_procs`** — `.t` created as
  both a `ttk::treeview` and a `listbox` in two different procs fires
  nothing anywhere, proving the collision-safety in item 8; **`resolves_when
  _widget_created_after_the_proc_that_uses_it_is_defined`** — proves the
  two-phase design is load-bearing, not stylistic, by constructing exactly
  the "proc defined before, called after" case a naive single pass would
  get wrong; `{*}`-expansion abstention; and
  `tcloo_dispatch_is_unaffected` — a plain `$obj method` dispatch keeps
  firing `var_command.rs`'s W308, never this module's W001.
- **Regression baseline**: the full existing `cargo test -p tcl-compiler
  --lib` suite (4133 tests after the additions above, 0 failures) and
  `cargo test -p tcl-registry` (320 tests, 0 failures) both stay green,
  plus a clean `cargo check --workspace` across all ~40 crates.
- **Interpreter domains** (issue #1141), `tk_checks.rs`'s
  `tests::interp_domains` module — 19 cases covering TP/FP/TN/FN in both
  directions: the false TK1001 across isolated interpreters and the true
  same-interpreter one; the missed TK1002 in both directions (parent only in
  the child, parent only in the parent) and the same-domain true negative;
  accumulation across two evals into one live interpreter; the handle form
  (`child eval`) agreeing with the literal form; the empty path staying in
  the current interpreter; nested paths (`s t`); delete-and-recreate starting
  a fresh hierarchy; a safe child; a tracked `$handle` binding; and the
  unresolved-target widening (both that it abstains, and that it does not
  fall silent when the parent exists nowhere at all).  End-to-end coverage of
  the published diagnostics is in
  `rust/tcl-lsp-server/tests/e2e/tk_dialect.rs` (6 further cases, including
  the literal `dialog1.tcl` shape from the audit).

## Sources

Builds directly on [`tcloo-object-typing.md`](tcloo-object-typing.md) (VTA,
Sundaresan et al. OOPSLA'00) and the command-resolution contract at
[`docs/design/contracts/command-resolution.md`](contracts/command-resolution.md)
for the namespace/alias/rename/trace boundary rules reused as-is for
constructor-command-name resolution.

Per-interpreter window hierarchies (issue #1141) are read from the Tk 9.0.4
C sources: `generic/tkWindow.c` (`TkCreateMainWindow`, `Tk_NameToWindow`) and
`generic/tkPack.c` (`TkSetGeometryContainer`).  The interpreter-isolation
half is verified live against tclsh 9.0.4.
