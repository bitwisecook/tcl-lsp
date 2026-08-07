# KCS: Classes and members installed dynamically are missing from the outline

> **Audience:** Contributor
> **Type:** Issue

## Applies to

all-editors, analyser

## Related issues in the class-factory / object-dispatch cluster

Five issues share the class-factory / object-dispatch name-resolution
subsystem this note documents. Status as of the #1312/#1303/#1304/#1305/#1306
cluster fix:

- **#1312** (fixed) — a *named* object handle (`ClassName create objName`, as
  opposed to `[ClassName new]`) resolved no members: the dispatch resolver
  never consulted `AnalysisResult::created_instance_commands` /
  `instance_classes` for a bareword receiver. This is the plain-class base
  case — no metaclass or factory is involved. See the "named-object dispatch"
  decision rule below and the `issue_1312_named_object_dispatch` test module.
- **#1305** (fixed) — a `rename`d metaclass command manufactured nothing: the
  factory lookup matched the creation call's head against the literal text a
  metaclass was *created* with, so a call through a renamed command found no
  record. Now resolves through the same `rename` / `interp alias` hop-walk
  (`indirection::walk`) the W307/W308 method check and the LSP navigation
  providers already shared. See decision rule 16 below.
- **#1303** (open) — an object handle bound by calling the class command
  itself, dispatched through the metaclass's `unknown` method (Tk's
  `::tk::IconList .il` idiom), resolves no members. Triage checklist item 11
  below already isolates this from the factory/class resolution proper.
- **#1304** (open) — a class made by a cross-file metaclass resolves only
  when the metaclass's defining file is *open* in the editor; an ordinary
  `oo::class` in an unopened file resolves fine. Suspected mechanism (per the
  ticket, unconfirmed): `reschedule_peers` only reaches documents with a
  diagnostics slot (open ones), and the startup scan indexes each file before
  the class-factory oracle publishes.
- **#1306** (open) — a metaclass created under a computed name
  (`Meta create ${ns}::class {…}` inside a proc, called with a literal
  argument) is never indexed — the shape tcllib's `oo::dialect` (and clay,
  practcl) uses. `is_dynamic_word` correctly rejects the dynamic-looking name
  per decision rule 2 below; the gap is that the analyser does not follow a
  literal argument through a proc's parameter binding into the creation call.

## Symptom

A class, a method, or a proc that a real interpreter genuinely creates is
absent from the document outline, and go-to-definition, find-references, and
hover all return nothing at its call sites. The class factory case has a
cross-file variant: the outline is empty for a file that creates classes
through a metaclass **declared in another file**. Sometimes a `W308 Unknown
method` fires on a call that tclsh runs happily, or the outline shows a
nonsense entry named after the unsubstituted source text (`${ptype}`,
`@dynclass@1859`).

All four shapes below come from real corpora (issue #923 audit cluster C3)
and all four run correctly under tclsh 8.6.16 and 9.0.4:

```tcl
# 1. A class factory — a class whose own superclass is oo::class.
oo::class create Megawidget {
    superclass oo::class
    self method create {name superclasses body} {
        next $name [list superclass MegawidgetClass {*}$superclasses]\;$body
    }
}
Megawidget create SimpleWidget {} { method CreateHull {} { my TraceOption a b } }

# 2. A member whose signature arrives through {*} expansion.
oo::define S { method {*}{foo {} {return 1}} }

# 3. A class (or an object) named by a foreach loop variable over a literal list.
foreach class {chart timeline} { oo::define $class { method RenderTsb {} {…} } }
foreach o {::a ::b} { oo::objdefine $o { method probe {} {…} } }

# 4. A command head built from a namespace held in a variable.
set ns ticklecharts
${ns}::setdef _options id -default $value
```

## Operational context

Each of these is a place where a name is *written* dynamically but is in fact
statically determined. The analyser's job is to tell those apart from names
that genuinely are not knowable until run time, and to abstain only on the
latter.

- **Class factories.** `oo::class`, `oo::abstract`, `oo::configurable`, and
  `oo::singleton` carry `Traits::IS_OO_METACLASS` in the
  [command registry](../design/compiler/command-registry.md). A class whose
  own superclass chain reaches one of them is *also* a class factory —
  that is TclOO language semantics, not per-command knowledge — so
  `Analyser::user_metaclass_of_class` walks the recorded superclass chain
  from the registry seed. Tk's `library/megawidget.tcl` is the canonical
  real user of the idiom.
- **The factory's word layout.** A factory that overrides the manufacturer
  (`self method create {name superclasses body}`) changes where the body
  word sits — argument 3, not the builtin `create Name Body` argument 2 —
  and splices superclasses the caller never wrote. Both facts are read off
  the override's own `next` call, never assumed, and recorded **once** on
  the metaclass's own `ClassDef::factory` rather than re-derived at each
  creation call.
- **The metaclass in another file.** Because the factory description is a
  derived fact rather than a re-walk of local state, it can be published to
  the rest of the workspace. The LSP merges every file's factories
  (`ItemTree::class_factories` → `project_class_factories`) and sets the
  result on each `SourceFile::workspace_class_factories`, so a file that
  writes `::tk::Megawidget create IconList FocusableWidget {…}` and nothing
  else — Tk's own `library/iconlist.tcl` — records the class it really
  makes. Without an index entry the call stays unrecognised: it is
  shape-identical to `interp create`.
- **`{*}` expansion.** Tcl applies `{*}` while *parsing*, so a `{*}`-marked
  braced literal is not one word but the elements of the list it holds. The
  member grammar's fixed argument layout applies to those elements.
- **`foreach` over a literal list.** The loop variable takes a different
  concrete value each iteration, so a command that installs a name from it
  installs several. Which commands those are is registry data:
  `Traits::INSTALLS_NAMED_DEFINITION`.
- **A composite command head.** The lexer merges `${ns}::setdef` into one
  `Var` token whose text is `ns}::setdef`. The variable's name ends at the
  brace the lexer left in place; everything after is an ordinary word
  suffix.

## Decision rules / contracts

1. **Metaclass-ness propagates; the seed stays in the registry.** Do not
   add a command name to the analyser to recognise a factory. The registry
   trait plus the recorded superclass chain is the whole rule.
2. **Read a factory's argument layout, do not guess it.** A prologue built
   from anything the analyser cannot read yields
   `ClassDef::inheritance_unknown`, which makes W308 abstain the same way an
   out-of-index superclass already does. Recording a guessed superclass
   list is worse than recording none.
3. **Reading the *whole* prologue is a precondition.** Every piece of the
   definition word the factory hands `next` must be accounted for — a
   `[list <member> …]` group that was parsed, the factory's own `$param`
   read, or separator text. "Found no nested command" is **not** the same
   as "injects nothing": `next $name "superclass Base\n$body"` injects a
   superclass with no command substitution at all, and calling that a
   known-empty injection lets W308 fire on every inherited method. Only a
   prologue whose pieces are all accounted for may report an empty
   injection.
4. **A relative superclass resolves in its owner's namespace.** `superclass
   Meta` inside `::n::DerivedMeta` names `::n::Meta`. Resolve it through
   the shared `class_hierarchy::resolve_class_name` — the same
   current-namespace-then-global rule the class lattice and MRO builder
   use — never a local pair of `name` / `::name` lookups. That also
   inherits its sound-by-abstention behaviour on an ambiguous tail, so a
   same-named class in an unrelated namespace is never cross-linked.
5. **Only reference-only members are injected.** `superclass`, `mixin`,
   `filter`, `export`, … (the definition grammar's `all_args_ref` set) name
   existing entities, so each injected word keeps a real source span —
   either in the factory's own body or in the creation call's arguments. A
   *definition* member cannot be injected: it would have no honest span.
6. **`{*}` of a literal splices; `{*}` of a substitution abstains.**
   `method {*}{foo {} {…}}` defines a real `foo`;
   `constructor {*}[info class constructor ::Base]` is equally real but has
   no statically-knowable element list, so the member is left unrecorded
   rather than invented with wrong parameters.
7. **A resolved dynamic head is a reference, never a rename target.** Its
   span spells only part of the name, so the invocation is marked
   `indirect`: find-references reports it, rename skips it. Rewriting the
   span would splice the new name over the substitution — the same
   corruption as
   [the closing-delimiter issue](kcs-issue-highlight-drops-closing-delimiter.md).
8. **A whole-word `$cmd` head belongs to the flow-sensitive engine.** The
   walk must not pre-resolve it from its lexical last-write-wins map; the
   CFG/SSA value model settles it. Only the composite shapes that engine
   skips are folded during the walk.
9. **The `foreach` simulation stays narrow.** It re-dispatches only
   `Traits::INSTALLS_NAMED_DEFINITION` commands, once per literal element.
   Re-walking the whole body per iteration would duplicate every diagnostic
   and scope entry, and there is no fixpoint — the element list is a
   bounded literal.
10. **The trait is a promise the dispatch must keep.** Stamping
   `INSTALLS_NAMED_DEFINITION` on a spec whose analyser hook has no arm in
   the re-dispatch match is worse than not stamping it — the registry says
   the command is re-run per element and nothing does.
   `every_installer_spec_lands_on_a_live_redispatch_arm` fails the build
   for exactly that.
11. **A cross-file factory must be *proved*, never guessed.** The workspace
   index narrows the abstention; it does not remove it. A dynamic head
   (`$meta create …`) is rejected before any lookup. The name is resolved
   through Tcl's own current-namespace-then-global candidate order and only
   an exact candidate hit counts, so a global `Megawidget create …` never
   reaches an indexed `::tk::Megawidget`. A locally-written class of the
   same qualified name shadows the index, as the interpreter would. With no
   entry, nothing is recorded and nothing is diagnosed.
12. **A cross-document factory's literal spans are not the caller's.** They
   index the *metaclass's* document, so they are re-homed onto a token of
   the creation call — and an injected member whose registry spec actually
   reads those tokens (`MemberSpec::retraction`) collapses the injection to
   `inheritance_unknown` rather than being applied against a substituted
   span.
13. **The oracle must reach the per-item path too.** A `Meta create …`
   inside a proc body is classified by the whole-file walk from the index,
   so the isolated body pass must carry the same index or the two
   strategies disagree about whether a class exists. It travels on
   `analyse_proc_body_isolated` and is part of `ItemBodyKey::body_env` so
   salsa cannot serve a stale verdict.
14. **The workspace index is a fixpoint, not one pass.** The query that
   collects a file's factories reads the very index the host publishes from
   it, so a metaclass manufactured by another file's metaclass is provable
   only on a round whose index already names its maker. One publish is
   therefore one link deep. The host iterates until a round moves nothing
   (issue #1296); termination holds because a round writes only what some
   file proved, a declaration cycle proves neither of its halves, and the
   loop is capped regardless. A workspace with no metaclass settles in one
   round that writes nothing.
15. **Re-dispatch must be idempotent per source site.** The ordinary body
   walk already covered the first element, so a site is visited twice under
   the loop variable's own key. One source site can never declare the same
   member twice, so `(objdefine_offset, member name)` is an exact
   "already recorded" identity — no iteration counter needed, and a genuinely
   separate block on the same object still accumulates.
16. **A `rename`d metaclass command is resolved through the same hop-walk
   every other command-table consumer uses, never re-derived.** The factory
   record stays keyed on the name the metaclass was *created* with —
   `class_factory_for_command` does not re-key it — so a call through
   `rename ::R::M ::R::Mk` resolves by falling back to
   `indirection::walk` (the shared `rename` / `interp alias` chain-follower
   the W307/W308 method check and the LSP's navigation providers already
   used) only when the direct literal-text lookup misses, then retrying the
   same local-then-workspace lookup against the resolved name (issue #1305).
   Order-gated exactly as every other `indirection::walk` consumer is: a
   `rename` written *after* the creation call does not retroactively cover
   it.
17. **A named object handle is not a class-factory concern, but shares the
   dispatch resolver.** `ClassName create objName` (as opposed to
   `[ClassName new]`) is plain `TclOO`, no metaclass involved — the analyser
   already recorded the binding in `instance_classes` /
   `created_instance_commands`. The gap was that three separate consumers
   never read it for a *bareword* receiver: the W307/W308 method-check
   (`record_var_or_cmd_command_site`'s `TokenType::Esc` arm, gated on both
   maps exactly like the LSP's `receiver_instance_class`), and the
   semantic-token / project-token-aggregation object-class map (merged in
   as a `NamedInstanceMap`, `object_types::harvest_unit`'s own
   `Statement::Call` arm only ever modelled a *registry* naming factory, not
   a user class). Go-to-definition, hover, completion, and references
   already resolved this shape before the fix (they read
   `instance_classes`/`created_instance_commands` directly, not through
   either of those two paths) — issue #1312.

## File-path anchors

- `rust/tcl-compiler/src/analyser/handlers.rs` —
  `handle_oo_class_command`, `class_factory_for_command`,
  `class_factory_for_candidates` (the local-then-workspace lookup, tried
  once for the literal head and once more for its `rename` target — issue
  #1305), `class_factory_of`, `user_metaclass_of_class`,
  `manufacturer_layout`, `manufacturer_word_positions`,
  `manufacturer_injected_template`, `injected_member_from_group`,
  `template_injected_member`, `resolve_factory_member`,
  `prologue_pieces`, `literal_list_words`, `handle_oo_define_command`,
  `handle_oo_objdefine`, `record_object_methods`,
  `handle_foreach_command`, `simulate_remaining_foreach_iterations`,
  `resolve_dynamic_word`
- `rust/tcl-compiler/src/analyser/indirection.rs` — `walk` (the shared
  `rename` / `interp alias` hop-walk `class_factory_for_command` falls back
  to — issue #1305)
- `rust/tcl-compiler/src/analyser/class_hierarchy.rs` —
  `resolve_class_name`, `build_tail_index` (the shared owner-aware
  relative-name resolution the chain walk reuses)
- `rust/tcl-compiler/src/analyser/oo.rs` —
  `splice_static_member_expansions`, `extract_method_def`
- `rust/tcl-compiler/src/analyser/commands.rs` —
  `resolve_dynamic_command_head`, `head_is_whole_word_variable`,
  `record_var_or_cmd_command_site` / `record_bareword_instance_dispatch_site`
  (the named-object `TokenType::Esc` W307/W308 site — issue #1312),
  `record_instance_creation` (moved outside `structure_only` so the
  project-wide token aggregation's lightweight per-file pass records
  `instance_classes` / `created_instance_commands` too — issue #1312)
- `rust/tcl-compiler/src/analyser/diagnostics/var_command.rs` —
  `live_classes_at_dispatch` (the bareword-site `instance_classes` lookup —
  issue #1312), `class_reachable_by_indirection` (the `indirection::walk`
  consumer #1305's fix now shares the pattern of)
- `rust/tcl-lsp-core/src/semantic_tokens.rs` — `NamedInstanceMap`,
  `named_instances_from_analysis`, `WorkspaceTokenFacts`,
  `full_with_cu_and_facts`, `range_with_cu_and_facts` (the semantic-token
  half of issue #1312)
- `rust/tcl-lsp-db/src/lib.rs` — `FileTokenFacts::named_instances`,
  `project_named_instance_index` (the project-wide token-aggregation half of
  issue #1312), `file_class_factories`,
  `project_class_factories`, `SourceFile::workspace_class_factories`,
  `ItemBodyKey::body_env`
- `rust/tcl-lsp-server/src/lib.rs` — `sync_workspace_class_factories`
- `rust/tcl-compiler/src/analyser/diagnostics/var_command.rs` —
  `has_external_super_or_mixin` (the W308 abstention)
- `rust/tcl-registry/src/traits.rs` — `Traits::INSTALLS_NAMED_DEFINITION`,
  `Traits::IS_OO_METACLASS`

## Failure modes

- Assuming the builtin `create Name Body` layout for a factory that
  overrides the manufacturer walks the *superclass* word as a script and
  loses every member of the new class.
- Recording a factory-made class with an empty superclass list when the
  prologue could not be read turns every inherited method into a false
  `W308 Unknown method`. Set `inheritance_unknown` instead.
- Splicing `{*}` over a substituted word invents a parameter list and a body
  span that point at the wrong text — worse than the omission it replaces.
- Folding a whole-word `$cmd` head during the walk makes the walk and the
  flow-sensitive engine disagree about which spans are safe to rewrite, and
  a rename then corrupts the source.
- Reading a composite `Var` token's whole text as a variable name looks up a
  variable that cannot exist, so a statically-resolvable head silently
  resolves to nothing with no diagnostic at all.
- Treating "no nested command in the prologue" as "the prologue injects
  nothing" turns a string-built `superclass` into a claim that the class has
  no superclass, and every inherited method then draws a false `W308`.
- Resolving a relative `superclass` with a bare `name` / `::name` pair skips
  the declaring class's own namespace, so a namespaced metaclass chain never
  reaches the registry seed and its factory calls record nothing.

## Triage checklist

1. Run the shape under `tclsh9.0` *and* `tclsh8.6` and confirm what the
   interpreter really creates (`info class superclasses`, `info class
   methods -all`, `info class constructor`). The oracle decides, not the
   source text.
2. Check whether the class reached `AnalysisResult::all_classes` at all, and
   under what key — an `@dynclass@<offset>` key means the target word was
   treated as unresolvable.
3. If a factory is involved, check whether its own `ClassDef` was recorded
   *before* the creation call, and whether its superclass chain reaches a
   registry metaclass.
4. If members are missing, check whether the member command's words were
   `{*}`-expanded and whether the expanded word is a braced literal.
5. If a call site resolves to nothing, check whether the head's `Var` token
   text contains a `}` — a composite head whose variable name is the text up
   to that brace.
6. Before claiming a fix, confirm the invocation's `indirect` flag matches
   whether the span really spells the name.
7. If a factory-made class reports no superclass, check whether its
   prologue was *read* or merely *found empty* — `inheritance_unknown`
   distinguishes them.
8. If a namespaced factory chain does not resolve, check the owner
   namespace the `superclass` word was written in, not just its bare and
   global spellings.
9. If a *cross-file* factory call records nothing, check in order: is the
   head literal? is the metaclass's file in the workspace scan? does
   `project_class_factories` carry its qualified name? does the call's name
   resolve to that exact qualified name under the enclosing namespace? Each
   "no" is a deliberate abstention, not a bug.
10. Chain **depth** across files is not a reason for a "no" at step 9. A
    metaclass that is itself manufactured by a *third* file's metaclass is
    proved one link per publish round, so the host iterates the publish to a
    fixpoint (issue #1296). If `project_class_factories` is missing a
    qualified name you can see being created, the question is whether some
    file *proved* it, not how deep it sits — a round adds an entry only on
    proof, never on a guess. Before the fixpoint landed this presented as a
    three-level cross-file chain resolving to nothing from a call site while
    `documentSymbol` on the declaring file reported the class correctly,
    which is the partial-resolution shape that makes it easy to miss.
11. If the class resolves but a *handle* bound from it does not, the
    construction form is the thing to look at, not the factory. A handle
    bound by calling the class command itself — dispatched through the
    metaclass's `unknown` method, as every Tk megawidget is
    (`::tk::IconList .il`) — is a separate, open gap: issue #1303. `create`
    and `new` on the same class bind the handle correctly.

## Test anchors

- `rust/tcl-compiler/tests/analyser.rs` — the `class_factories` module
  (TP/TN cases covering all four shapes plus their abstentions), including
  `a_workspace_indexed_metaclass_creates_real_classes`,
  `the_cross_file_result_matches_the_same_file_result`,
  `the_per_item_walk_agrees_with_the_whole_file_walk_cross_file`,
  `a_dynamic_metaclass_name_still_abstains`,
  `a_metaclass_defined_in_another_file_abstains_rather_than_guessing`,
  `a_workspace_metaclass_is_not_reached_by_a_same_tailed_bare_name`,
  `a_local_metaclass_shadows_the_workspace_one`, and — for the chained case
  (issue #1296) —
  `a_second_link_metaclass_publishes_a_factory_once_the_first_is_indexed`,
  `the_third_link_records_the_class_and_its_members`,
  `oo_define_after_the_fact_extends_a_chained_factory_made_class`,
  `an_unproved_second_link_still_abstains`, and — for the renamed-metaclass
  case (issue #1305) —
  `a_renamed_metaclass_still_manufactures_its_class`,
  `the_control_without_a_rename_records_the_same_class`,
  `a_rename_written_after_the_creation_call_does_not_apply` (FN guard),
  `a_rename_with_no_call_through_the_new_name_manufactures_nothing`
  (FN guard)
- `rust/tcl-compiler/tests/analyser.rs` — the `issue_1312_named_object_dispatch`
  module (named-object dispatch, issue #1312):
  `named_object_draws_w308_on_an_unknown_method`,
  `named_object_and_handle_object_draw_the_same_w308_message`,
  `named_object_calling_a_real_method_draws_no_w308` (TN),
  `an_ordinary_proc_call_never_draws_w307_or_w308` (FP guard),
  `interp_create_bareword_dispatch_draws_no_diagnostic` (FP guard —
  `created_instance_commands` without `instance_classes` must stay silent),
  `per_item_analysis_agrees_with_whole_file_analysis`
- `rust/tcl-lsp-core/src/semantic_tokens.rs` — `tcloo_dispatch_pattern_fixture`
  (the `named_object` golden-fixture row, flipped from `Abstain` to
  `Resolve` — issue #1312)
- `rust/tcl-lsp-server/tests/e2e/issue1312_named_object_dispatch.rs` — the
  named-object ticket end to end: W308, the semantic-token `method`
  classification, go-to-definition, and completion
- `rust/tcl-lsp-server/tests/e2e/issue1305_renamed_metaclass.rs` — the
  renamed-metaclass ticket end to end (go-to-definition through the rename,
  plus the un-renamed control)
- `rust/tcl-lsp-db/tests/class_factory_fixpoint.rs` — the publish loop itself:
  `a_cross_file_three_level_chain_converges_and_records_the_class`,
  `a_single_publish_is_one_link_deep` (the regression, asserted directly),
  `a_chain_deeper_than_three_still_converges`,
  `a_cycle_terminates_and_proves_nothing`,
  `a_workspace_with_no_metaclass_costs_no_extra_round`
- `rust/tcl-lsp-server/tests/e2e/issue1296_metaclass_chain.rs` — the ticket
  end to end, plus the five-level, edit-path, and abstention cases
- `editors/vscode/src/test/issue1296MetaclassChain.test.ts` — the same
  through a real VS Code session
- `rust/tcl-lsp-server/src/lib.rs` —
  `a_cross_file_metaclass_resolves_after_the_workspace_scan`,
  `a_dynamic_metaclass_head_abstains_even_with_the_scan_index`,
  `a_workspace_with_no_metaclass_never_sets_the_factory_oracle`
- `rust/tcl-compiler/src/analyser/handlers.rs` —
  `every_installer_spec_lands_on_a_live_redispatch_arm`,
  `foreach_objdefine_records_per_object_facts_for_every_literal_element`,
  `foreach_objdefine_does_not_double_record_the_first_element`,
  `foreach_objdefine_does_not_duplicate_body_diagnostics`,
  `foreach_objdefine_over_a_dynamic_list_abstains`
- `rust/tcl-compiler/src/analyser/handlers.rs` —
  `two_dynamic_oo_define_targets_sharing_a_variable_name_never_merge`,
  `dynamic_oo_define_target_does_not_touch_a_same_named_literal_class`
- `rust/tcl-lsp-core/tests/references_rename.rs` —
  `rename_rewrites_the_defining_literal_and_never_the_dispatch_span_945`
