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

That second point means the registry-side binding is **self-referential**:
the object class a widget's constructor creates is the constructor
command's own spec, not a separate class. `ObjectClassSpec::instance_methods`
and `CommandSpec::subcommands` are already the same type
(`&'static [SubCommand]`), so a widget's `ObjectClassSpec` can point at the
literal same `SUBCOMMANDS` static its own `CommandSpec::subcommands` uses —
zero duplication, zero drift risk, and `CommandRegistry::instance_method`
resolves it with no widget-specific code at all.

## Machinery this reuses

- `CommandSpec::creates_instance_at: Option<u8>` + `CommandSpec::object_class:
  Option<&'static ObjectClassSpec>` (`rust/tcl-registry/src/spec.rs`) are
  registry-driven, not hardcoded — also used by `report::report`,
  `struct::graph` / `struct::tree` (`tcl-registry/src/commands/tcllib/`) and
  `ticklecharts`.
- `Analyser::record_registry_factory_instance`
  (`rust/tcl-compiler/src/analyser/commands.rs`) reads those two fields
  generically for *any* command and writes the bareword name into
  `AnalysisResult::instance_classes: HashMap<String, String>` and
  `AnalysisResult::created_instance_commands: HashSet<String>` — the same
  sets TclOO's `CLASS create NAME` idiom feeds. `is_plain_created_name`
  accepts a leading `.` (it rejects only `%`, `$[]{}() "` and empty).
  **Tk widget constructors are syntactically identical to the tcllib
  `struct::graph g` shape** (positional name, no `new`/`create` keyword), so
  bareword tracking is registry data, not analyser code.
- `definition.rs::receiver_instance_class` + `created_instance_commands`
  resolve a **bareword** receiver's class, and are consumed by
  go-to-definition, find-references, and — through the same shared resolver
  and its offset-aware wrapper `receiver_instance_class_at` — hover and
  completion. Pinned by
  `definition_resolves_bare_created_instance_command_method` (`Dog create
  rex` → `rex bark` jumps to the method).
- `semantic_tokens.rs::insert_object_method_overrides` handles `$var` (via
  `object_types::object_handle_classes`), `[cmd]`, and bareword receivers.
- `validity.rs`'s W001 (`emit_w001_unknown_subcommand`) and E002/E003
  (`emit_arity_diagnostics`) resolve `cmd_name` fresh via
  `registry.get_for_dialect` on every call, with no notion of a
  variable-tracked class. **The `.`-prefix bail-out in `validity.rs` is
  unrelated to this model** — it guards `<ensemble> .path` (`grid .w`,
  `entry .e`), an argument that is a path being *created*, not a command
  head being dispatched. `.t instate …` reaches the ordinary "no registry
  spec literally named `.t`" path instead, which is where the widget
  resolution step hooks in.
- TclOO's own W308 (unknown method) / E001 (bare dispatch) / E002–E003
  (method arity) in `var_command.rs` is the template for a **sound**,
  ambiguity-abstaining diagnostic over a class-tracking map: it fires only
  when the receiver's class set has exactly one member
  (`class_names.len() == 1`), never on `{*}`-expanded calls, and reuses
  `validity::arity_verdict` / `shift_arity` so wording matches the ordinary
  registry-command diagnostics.
- `object_types.rs`'s `harvest_unit` reads `creates_instance_at` /
  `object_class` generically for its own (unsound, highlight-only,
  doc-level-union) `$var` tracking, so widgets need no new matching logic
  there either.
- `type_infer.rs` **deliberately never lattice-types factory-return values**
  (`type_infer.rs:222-228`): doing so would leak one call's class onto a
  same-named variable in another proc via `var_command::aggregate_object_types`'s
  object-insensitive aggregation (this is the FP-OBJ-04 regression the
  comment names explicitly). Factory-return provenance is kept in the
  syntactic, highlight-only `object_types::object_handle_classes` map
  instead. **This model follows the same discipline**: widget constructors are
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
2. **Widget-command identity is a registry query**, not a name list:
   `Analyser::is_widget_command` (`analyser/tk_checks.rs`) asks
   `creates_instance_at.is_some() && required_package == Some("Tk")`. A
   hardcoded list is not an option here — the previous one had drifted to
   naming `ttk::scrollbar` and `ttk::labelframe`, neither of which has a
   registered `CommandSpec`. One source of truth removes the drift class,
   not just one instance of it.
3. **Nothing downstream of `creates_instance_at` / `object_class` is
   widget-specific.** `commands.rs::record_registry_factory_instance` and
   `object_types.rs::harvest_unit` read those two fields generically (they
   were built for the tcllib factories), so both the bareword case
   (`ttk::treeview .t` then `.t instate …`) and the `set w [ctor .path]`
   return-value-capture case (`commands.rs::registry_factory_class_from_subst`,
   likewise generic) fall out of the registry data alone.
4. **Bareword receivers in `insert_object_method_overrides`**
   (`semantic_tokens.rs`): a `TokenType::Esc` match arm queries the *same*
   `object_classes: &ObjectClassMap` parameter the `$var` arm reads — point 3
   means it already carries widget bareword bindings, so no extra parameter
   is threaded. The separate `named_object` pure-user-TclOO-class case
   (`C create obj; obj mrun`) is a different question and correctly stays
   `Abstain`: `object_types.rs` has no `all_classes` access, by design (see
   Deliberate abstentions).
5. **`obj_method_hover_text` has a registry fallback**: when
   `analysis.all_classes.get(class)` misses, it tries
   `registry.instance_method(class, method)` before giving up. The
   `registry: Option<&CommandRegistry>` parameter is `Option` so the
   fallback is not a hard dependency.
6. **`completion.rs` resolves bareword receivers** through
   `crate::definition::receiver_instance_class_at` — the offset-aware
   wrapper over the exact resolver go-to-definition and hover share, not a
   parallel one. There is a single completion entry point,
   `method_completions`; the registry path is a *fallback inside*
   `method_items`, which calls `registry_method_items(registry, class_q)`
   when `analysis.all_classes` does not know the class. A registry-modelled
   class models no class/instance distinction, so `MethodBucket` is moot on
   that path.
7. **The widget-instance diagnostic module**,
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
   wording to every other arity check. `configure` / `cget` are treated as
   universally valid and never arity-checked (see Deliberate abstentions)
   since no widget spec declares them.
8. **`instance_classes` collision-safety, at the registry-driven binding
   sites only.** The diagnostic in (7) cannot safely trust
   `instance_classes`' whole-file, last-write-wins contract: two different
   procs may legitimately create two *different* widget classes under the
   same literal path, e.g. `.t`. Trusting it would repeat exactly the class
   of bug [`tcloo-object-typing.md`](tcloo-object-typing.md) and the
   `FP-OBJ-04` precedent (`experiments/`, `fp/obj.rs`) warn against for
   interprocedural unions. `Analyser::bind_registry_instance_class`
   (`commands.rs`) makes the two registry-driven insertion sites inside
   `record_registry_factory_instance` collision-aware: a name bound to two
   different classes anywhere in the file is dropped from
   `instance_classes`, recorded in
   `AnalysisResult::ambiguous_instance_names`, and never re-added. The
   `TclOO` user-class binding sites in `record_instance_creation` (Patterns
   A/B) deliberately do **not** go through it — they keep their documented
   best-effort contract, so this is a narrow guarantee at two call sites
   rather than a semantic change to the shared field.

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

A single file-wide accumulator would therefore decide both diagnostics across
every interpreter at once, producing a false TK1001 (a parent-side `pack`
"conflicting" with a child-side `grid`) and a missed TK1002 (a parent created
in one interpreter vouching for a child widget in another).

The accumulator is instead keyed by **interpreter domain**
(`Analyser::tk_domains: BTreeMap<String, TkDomainState>`, declared in
`analyser/state.rs` with the state type and logic in
`analyser/tk_checks.rs`), reusing the existing synthetic-key mechanism rather
than a parallel one:

- The domain identity is the same `@interp@<path>[#<epoch>]` name
  `interp_domain_name` mints and `isolate_interp_eval_body`
  (`analyser/handlers.rs`) uses for the synthetic scope a child body's procs
  and variables home under — the one place both `interp eval PATH { … }` and
  the handle form `NAME eval { … }` pass through.  The main interpreter is
  the empty key.
- That helper pushes an `InterpFrame { key, domain, resolved }` rather than a
  bare path string, so any analyser state that models per-interpreter runtime
  state can ask "which interpreter am I in?" without a second, drift-prone
  notion of interpreter identity.  `key` is the path qualified against the
  enclosing frames (`s`, `s t`); `domain` is the synthetic identity; `resolved`
  is `false` once this frame *or any frame enclosing it* targeted a path that
  could not be resolved statically.
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
  fail-closed here.  The *window hierarchy* half (TK1001 / TK1002) is not
  an abstention: it is keyed by interpreter domain.
- **Interprocedural flow for diagnostics** (a widget path passed as a proc
  argument, `proc configureWidget {w} { $w instate … }`): the diagnostic's
  receiver resolution is whole-file (via `instance_classes`), not
  proc-scoped like `var_command.rs`'s own TclOO tracking — but made *sound*
  for that wider scope by collision-dropping (item 8 above) rather than by
  narrowing the scope. A widget path threaded through a proc parameter
  therefore *does* get diagnosed (unlike TclOO's `$obj` parameters,
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

| Consumer | Bareword (`ttk::treeview .t` → `.t instate`) | `$var` (constructor return) | Registry-class rendering |
|---|---|---|---|
| go-to-definition / references | `receiver_instance_class` | `registry_factory_class_from_subst` | n/a — jumps to the registry doc, not to source |
| hover | `receiver_instance_class`, then `obj_method_hover_text` | same | `registry.instance_method` fallback (item 5) |
| semantic tokens | `TokenType::Esc` arm of `insert_object_method_overrides` (item 4) | `object_types.rs`'s generic `creates_instance_at` read | `registry.instance_method` |
| completion | `receiver_instance_class_at` → `method_completions` (item 6) | same | `registry_method_items` fallback inside `method_items` |
| diagnostics (W001/E002/E003) | `widget_command.rs`, whole-file + collision-safe (items 7–8) | same | same |

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
  (`instance_classes` / `created_instance_commands`). These hold with no
  widget-specific production code in either file, which is what item 3 above
  asserts.
- **Semantic tokens** (`tcl-lsp-core/src/semantic_tokens.rs`): the
  `widget_bareword` and `widget_var_captured` rows in the shared
  `TCLOO_DISPATCH_CASES` golden fixture table, run by
  `tcloo_dispatch_pattern_fixture` alongside every TclOO / snit / itcl case.
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
- **`tk_checks.rs`**: the TK1001/TK1002/TK1003 suite, including
  `tk1002_fires_for_ttk_and_listbox_constructors` (coverage across widget
  families, not one widget) and
  `unknown_command_is_never_treated_as_a_widget_constructor` (the
  registry-driven query cannot drift the way a hardcoded list would).
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
  `tcloo_dispatch_is_unaffected` — a plain `$obj method` dispatch fires
  `var_command.rs`'s W308, never this module's W001.
- **Interpreter domains** (issue #1141), `tk_checks.rs`'s
  `tests::interp_domains` module — 20 cases covering TP/FP/TN/FN in both
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
  `rust/tcl-lsp-server/tests/e2e/tk_dialect.rs` (15 further cases, including
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
