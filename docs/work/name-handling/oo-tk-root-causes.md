# TclOO and Tk naming: root causes

The symptoms are in queue items 2, 3 and 5 of the [signed scan and precedence handover](handover/signed-scan-precedence.md), with fixtures in [`oo-dispatch-probe/`](handover/oo-dispatch-probe/README.md). This page groups them by cause. Behaviour marked *measured* comes from the discriminating fixtures in `oo-dispatch-probe/discriminating/` on `1e64752b` (`tcl9.0`); code marked *read* was checked at the cited lines; *inferred* means the exact gate was not pinned.

## 1. No single command-lookup context per frame

A command name in a frame resolves through the current namespace, its `namespace path`, an implicit helper path (`::oo::Helpers` in TclOO member bodies), the object's own namespace (`my`), then `::`. That context is not one value. At least six resolvers each see part of it:

| Resolver | Sees |
| --- | --- |
| `original_command_slot_paths_for_bytes` (`command_binding/original_command_table/command_keys.rs:459`), behind the W123 presence query | current namespace and its namespace path only |
| `source_keys_checked` (`command_binding/module_bindings/target_resolution.rs:287`) | receiver path only after native and registry paths have answered |
| `original_receiver_command_lookup` (`command_binding/source_name_lookup.rs:814`) | the full receiver path; used by navigation and references, not by diagnostics |
| `oo_global_resolution`/`oo_method_frame` (`analyser/scope/command_namespaces.rs`) | the TclOO frame; feeds resolution candidates; its comment says W123 keys on it, but W123 does not call it |
| `requires_runtime_command_namespace` (`ir_helpers/nested_bodies.rs:118`) | nothing per name: any relative head in a runtime-selected body refuses |
| W307/W308 emitters | class tables and handle facts |

Symptoms: false W123 on `my`, `self`, `next`, `link` and `callback`, only in bodies that are entered (*measured*: an uncalled method is silent, the same method once called reports; the presence query looks the head up in the defining namespace — *read* via the TclOO investigation, `deferred_method/retained_entries.rs:646`). Method bodies without SSA, because the guard refuses whole bodies rather than asking a per-name resolver (*read*, `compilation_unit/body_units.rs:236-260`). `ExecutionNamespace` has no "receiver frame with a known path" variant; lowering collapses the grammar's runtime-receiver namespace into `RuntimeSelected` (`lowering/declarations/class_members.rs:700-706`) even though only object-local publications are unknown.

Fix: one frame lookup context (namespace, namespace path, helper path from the profile, receiver namespace contents with their own unknowns) built once per frame and passed to every resolver; delete the partial resolvers. The full census of resolvers and the plan are in [command lookup is not centralised](command-lookup-centralisation.md).

## 2. Instance identity has no single value type

TclOO objects are recorded as a `SourceObjectInstanceProof` in three stores (named commands, variable cells, method receivers; `command_binding/object_instance.rs`), `[C new]` gets only a class-name string in the type lattice (`command_binding/invocation_binding.rs:978-1007`), and the analyser keeps a fourth, proof-less candidate list (`object_types/logical_source_candidates.rs`). Tk paths live in a plain `HashSet<String>` (`analyser/tk_checks.rs:141`, *read*), separately as command-binding instance definitions created by `CREATES_WIDGET` with partial knowledge, and not in the object lattice at all; `destroy` is an unknown transition, nothing reads the set except the parent-exists check, and only literal paths are recorded. Images and fonts are recorded nowhere.

Symptoms: W307 on `[listbox .lb2] size`, on `$w get` with `w` set to a literal path, and on `$img put`; nothing followed through `$w.ok` or `$d.ok`; duplicate paths, upper-case names (`is_widget_path` in `tcl-registry/src/tk_geometry.rs` accepts them, *read*) and use after `destroy` unreported.

Fix: one instance value — TclOO objects, other systems' objects, Tk widgets, images, fonts — carried by the handle lattice, created by registry-declared creation effects, removed by declared destruction effects, with names that may be literal, built (`$w.ok`) or returned.

## 3. Guards refuse at coarse granularity

A class-layer `oo::define` (the `Configure` arm, `command_binding/module_bindings/invocation_transitions.rs:491-510`, *read*) invalidates dispatch for every object and removes the class's definition and bounded constructions; any non-preserving define, move or delete does the same (`binding_transitions.rs`). Bounded construction is all or nothing per class body, and a configured class definition must be methods only (`deferred_method/class_definitions.rs:632-650`). The method-body SSA guard (cause 1) is the same pattern at body granularity.

Symptoms: W307 on `[C new] m` when the class uses `filter`, slot operations (`oo::define C variable`, `superclass -append`), `private variable`, or a constructor calling `next` (*measured*: `filter` alone triggers it; `superclass` or a later `oo::define C method` alone does not).

Fix: transitions that change exactly the facts they touch (one class's members, one relation, one object's class), with a per-name unknown rather than a whole-world reset.

## 4. Checks keyed on top-level retained statements

Widget method checks, named-object method checks and `after` script arity are made only for commands at the top level of a script. *Measured*:

| Call | Top level | In `catch {…}` | In `[…]` |
| --- | --- | --- | --- |
| `.b invokee` (W001) | reported | — | silent |
| `$lb insertt` with `set lb [listbox .lb]` (W001) | reported | silent | — |
| `o m` after `C create o` (E002) | reported | silent | silent |
| `after 10 {tick}` (E002) | reported | silent | — |
| `f` with `proc f {a}` (E002) | reported | reported | reported |
| `$o m` after `set o [C new]` (E002) | reported | reported | — |

The widget path goes through `flush_widget_dispatch_diagnostics` → `source_registered_instance_words_at` → `registered_original_words_at` (`registry_invocation/source_structure/advice_queries.rs:572`, *read*), which needs the command realm to own a recorded command or a retained body segment at that offset; that nested scripts and substitutions lack one is *inferred*. Earlier records that named objects "lose method checking" and that classes calling `C nosuch` are unchecked came from fixtures that wrap the call in `puts [catch {…}]`; the cause is this nesting, not the object kind.

Fix: every checker reads the same per-invocation record, which must exist for every command the walk reaches, however deeply nested.

## 5. Two class models and option data on the wrong object

Registry classes (Tk widgets) and source classes (TclOO and friends) are checked by different code (W001 versus W308/E002; see the [object systems plan](object-systems-plan.md)). For Tk, creation is checked against the constructor command's option table (TK1003, `analyser/tk_checks.rs`), but a later `pathName configure` goes through the instance-method path, which checks only unknown methods, option relations and arity (`analyser/diagnostics/widget_command.rs:110-181`). The shared `configure` method declares no options (`CLASSIC_WIDGET_CONFIGURE`, `tcl-registry/src/commands/tk/common.rs:298`, *read*); the table is on the constructor and `ObjectClassSpec` has no link to it. `menu entryconfigure` declares options that no instance checker reads.

Symptoms: every `configure`, `itemconfigure`, `entryconfigure` and `tag configure` typo; `ttk::button .b; .b configure -bg red` silent although `ttk::button .b -bg red` is reported (*measured*).

Fix: one class fact whose methods carry their option tables (a widget's `configure`/`cget` share the class's options), checked by one method-call checker.

## 6. Values are hints, not grammars

The only unknown-option checker is the Tk constructor one; `lsort -inreger` is silent (*measured*). Closed value sets exist (W127 through `OptionValue::enumerated`), but Tk specs write their enumerations as display hints (`pack -side` is `OptionValue::value("top|bottom|left|right")`, `tcl-registry/src/commands/tk/pack.rs:80`, *read*). Structured values have no grammar at all: `-sticky` letter sets, `wm geometry`, text indices, canvas coordinate counts, event patterns and virtual event names.

Fix: a generic unknown-option check for every option-taking command and method, closed sets declared as such, and value grammars (letter set, geometry, index, coordinate list, event pattern) as registry value kinds checked by one emitter.

## 7. Definitions recorded without walking their bodies

The one-member forms (`oo::define C method|constructor|self method …`, `oo::objdefine o method …`) go to `parse_oo_define_inline_in` (`analyser/oo/definition_body.rs:507`), which records the member and checks formals but never walks the body (*read*). The 9.0 `initialise` script is walked but its diagnostics are not produced (*inferred*: no retained source owner for that script). `itcl::body` is untraced.

Fix: every callable member, however declared, becomes a body unit through the same path (object systems plan, member sources).

## 8. Data present, lint missing

Tk variable-linking options are declared as global variable names (`OptionValue::global_var_name()`, `VariableScope::Global`, `tcl-registry/src/hover.rs:423-445`) and resolved as globals, but nothing warns when the name is also a procedure local, which is the common bug (the entry shows the global, empty).

## Shared fixes, in order

1. One frame lookup context (cause 1); removes the false W123 and lets method bodies take the procedure pass.
2. One per-invocation record for every reached command, nested or not (cause 4).
3. One instance value with declared creation and destruction effects (cause 2), and fine-grained transitions (cause 3).
4. One class fact and one method-call checker with option tables on methods (cause 5), and value grammars with a generic option check (cause 6).
5. Body units for every member source (cause 7) and the variable-link lint (cause 8).
