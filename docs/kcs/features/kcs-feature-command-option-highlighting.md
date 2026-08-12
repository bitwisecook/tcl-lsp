# KCS: feature — Command option highlighting

> **Audience:** User
> **Type:** Functionality

## Applies to

all-editors, analyser

## Summary

Words that read as `-option` switches, and the values they take, are given
their own semantic-token colours — an option token for the switch and an
option-value token for its argument — so a command's flags stand out from its
positional strings.

## How to use

- **Editor**: Applied automatically as part of semantic highlighting. No
  configuration; toggles with `tclLsp.features.semanticTokens`.

## Operational context

There are two layers, most-precise first:

1. **Registry-declared options.** For a command the registry knows
   (`lsort -index 2`, `file delete -force …`), only the switches the command
   actually declares are highlighted, so a stray `puts -foo` stays a plain
   string. Value-taking options colour their value too (an enum value as an
   enum member, otherwise an option value).

2. **Object-method options.** For an object handle bound by a constructor —
   the standard `TclOO` / Tk pattern `set chart [ticklecharts::chart new]`
   then `$chart Xaxis -name {v} -type value -min 0.4` — the handle's class is
   tracked from the `new` call, the method is resolved against the class's
   registry model, and its declared options and values are coloured precisely.
   Direct `[Class new] method …` dispatch resolves the same way. See
   [ObjectClassSpec](../../../docs/design/compiler/command-registry.md).

   Resolution also reaches **user-defined classes** (not only registry-modelled
   ones): the method is resolved against a class hierarchy (MRO over
   `superclass` / `mixin`), so `$obj method` on a class defined in the workspace
   colours the method, and for an `oo::configurable` receiver its `configure` /
   `cget` `-property` options are coloured from the class's declared properties.
   The hierarchy is **workspace-merged** — a `project_class_index` unions every
   file's classes — so a direct `[Class new] method …` dispatch on a class
   defined in *another* file resolves too (the mro_eval cross-file lever); the
   single-file path falls back to the local hierarchy.

   Object provenance is also **interprocedural**, computed as a small
   name-keyed type-propagation fixpoint (VTA-lite) over four edges:
   - **proc parameter** — a proc called with an object argument (a tracked
     `$var` handle or a direct `[Class new]`) binds its parameter to that class,
     so `set p [Pin new]; connect $p` resolves `$dev configure -node …` inside
     `connect {dev}`;
   - **proc return** — a proc that returns an object
     (`proc make {} {return [C new]}; set o [make]`) types the receiver;
   - **assignment (aliasing)** — `set b $a` copies `a`'s class to `b`;
   - **constructor parameter** — an object passed *into* a constructor
     (`Wrap new $p`) binds the constructor's parameter, which via the aliasing
     edge carries onto an instance variable it is stored in, so a *different*
     method dispatching on that variable resolves (the dependency-injection
     shape).

   The class flows through chains of these edges to a fixpoint. (Measured note:
   the aliasing/constructor-param edges resolve real patterns but are rare in
   the surveyed corpora, which are snit-dominated — see
   `experiments/tcloo_diag/RESULTS.md`; snit `$self`/component support is the
   larger outstanding lever.)

   A **self-call** inside a class body resolves against the enclosing class's
   MRO (the class named at the definer head that opened the body) — by far the
   most common object-dispatch form. This covers `my method …` (`TclOO`),
   `$self method …` (snit), and `$this method …` (itcl); `my`/`$self configure
   -prop` colours the property option too. Recognising the enclosing type for
   snit / itcl bodies is driven by the registry's definer-family grammar, not a
   hardcoded definer list. On the surveyed corpora, snit `$self` is the single
   largest dispatch form after `TclOO` `my`, and enabling it roughly doubled the
   overall dispatch-resolution rate (`experiments/tcloo_dispatch/RESULTS.md`).

   A snit/itcl object bound by its **named constructor** (`set o [foo create x]`)
   types the variable as that class — the signature scan records snit types (like
   itcl and `TclOO`) as classes — so `$o method` resolves like any handle.

   A snit **component** typed either by `install NAME using TYPE …` or by snit's
   bare-word constructor `set NAME [TYPE inst …]` (`$type $name` creates an
   instance) resolves `$NAME method …` in the body. Because snit method bodies
   aren't lowered into the compiler CFG, these come from a source scan; the
   component's class most often lives in another file, so they land in the
   workspace (project) mode where the merged hierarchy resolves them. The
   bare-constructor form is gated for soundness — it fires only when `TYPE` is a
   known snit-family class whose first argument is not a `typemethod` (so a
   `set x [Type info …]` type-command call is not mistaken for a construction).

   **A Tk/ttk widget's own instance command** (`.t instate …`, `$w tag
   configure …`, `$listbox curselection`) resolves the same way, keyed on
   the *path* rather than a `TclOO`/tcllib object identity: a widget
   constructor (`ttk::treeview .t`, `listbox .l`) is registry data
   (`CommandSpec::creates_instance_at` + a self-referential `object_class`
   whose `instance_methods` is the literal same slice as the constructor's
   own `subcommands` — no separate hand-maintained method table), so both
   the bareword form (`.t instate …`, reusing the literal path text) and the
   `$var`-captured return-value form (`set lb [listbox .l]; $lb
   curselection`) resolve their subcommands, options, and arg values
   precisely, the same as any other object method (issue #927).

   **Not resolved (correctly abstains, never mis-highlights):** a bareword
   named-object dispatch onto a *user*-defined `TclOO` class (`Foo create
   obj; obj method`, where the *object command* itself is the head — the
   registry-modelled shape above, including Tk widgets, does resolve; a
   user class does not), and `installhull $win` (the already-created form,
   which names no widget type — the `installhull using TYPE …` form does
   resolve, typing the implicit `hull` component). The receiver's class is found from a handle typed by
   the SSA lattice — including a handle **retrieved from an object collection**:
   a `Pins` dict filled with `[Pin new]` in one method makes
   `[dict get $Pins $pin] configure -node …` resolve to `Pin` in another (issue
   #797), via the container element-typing in the type lattice. A **loop over an
   object collection** binds its value variable to an element too, so
   `dict for {k pin} $Pins {$pin configure …}`, `dict map …`, and
   `foreach pin $Pins {$pin …}` resolve the same way — including a loop nested
   in a command substitution (`return [dict map {k v} $Pins {$v …}]`), found by
   a syntactic scan. A *heterogeneous* collection (several element classes) keeps
   the whole set and resolves the method against any member.

3. **Generic fallback.** For any *unknown* command head (an object method on a
   class not modelled in the registry, or a receiver that arrives as a proc
   parameter), a word shaped like a clean option — a leading `-` then a
   letter — is highlighted as an option and its following literal value as an
   option value.

The heuristic is careful about Tcl's own conventions: a negative number
(`-5`, `-1.6`) or a special-float literal (`-inf`, `-nan`) is **not** an
option; a substitution word (`-$var`, `-{$var}`, `-[cmd]`) keeps its own
highlight; and the `--` end-of-options marker stops option scanning, so a
following `-literal` is treated as a positional operand.

### Computed command heads are not command tokens

A command whose *head word* is itself computed — a `$obj method …` object
dispatch, a `[dict get $Pins $pin] method …` / `[Class new] method …`
substitution, or a multi-fragment `chartV$node …` word — has a command name
that is only known at runtime. Such a head is **not** painted as a resolved
command token. Instead the head word is tokenised as ordinary code: a `[…]`
substitution recurses into its inner script (so `[dict get $Pins $pin]` shows
`dict` as a builtin, `get` as its subcommand, and `$Pins` / `$pin` as
variables), and a `$var` head reads as a variable. This is an accurate picture
of the dynamic dispatch rather than a misleading command highlight (issue
#797). The words *after* the head (the method and its `-option` pairs) are
still highlighted by the object-method / generic passes above.

## File-path anchors

  (registry options), `insert_object_method_overrides` (object methods, incl.
  `collection_head_element_classes` for `[dict get …]` receivers and
  `user_class_provides_method` / `insert_user_configure_options` for user
  classes), `insert_generic_option_overrides` (fallback), `head_is_computed` +
  `collect_script` (computed heads are tokenised, not painted as command tokens)
  `object_collection_classes` (handle & collection → class provenance)
  container element-typing in the SSA lattice (`TypeLattice::element_class`)
- `rust/tcl-registry/src/spec.rs` — `ObjectClassSpec`
- `rust/tcl-registry/src/commands/ticklecharts/mod.rs` — the ticklecharts pack
- `rust/tcl-registry/src/commands/tk/*.rs` — every widget-constructor
  `CommandSpec` sets `creates_instance_at`; the 9 with a real subcommand
  table (`ttk__treeview.rs`, `ttk__notebook.rs`, `listbox.rs`, `text.rs`,
  `canvas.rs`, `entry.rs`, `menu.rs`, `panedwindow.rs`, `spinbox.rs`)
  additionally set a self-referential `object_class`
  (registry-driven `instance_classes`/`created_instance_commands` binding,
  covers both Tk widgets and tcllib factories), `bind_registry_instance_class`
  (collision-safe insertion)
  widget-instance W001/E002/E003 diagnostic (`docs/design/tk-widget-instance-typing.md`)
  `registry_method_completions`

## Failure modes

- A `-option` on an unmodelled object method is highlighted by shape only, so
  an invalid switch is not distinguished from a valid one.
- snit/itcl **`$self` / `$this` self-calls**, **named-constructor** objects
  (`set o [foo create x]`), **installed components** (`install ax using Ax`),
  the **implicit hull** (`installhull using TYPE …`), and
  **bare-constructor components** (`set c [Type inst]`) all resolve, but
  `installhull $win` (the already-created form, which names no widget type)
  and a **bareword named-object** command onto a *user*-defined class
  (`Foo create obj; obj method`) are not yet provenance-tracked — they use the
  generic fallback (`experiments/tcloo_diag/RESULTS.md`).
- A receiver whose class is only bound in *another file* (a cross-file instance
  variable, global, or parameter) is not tracked: object provenance is computed
  per file, so only the workspace class *hierarchy* crosses files today, not the
  handle/collection provenance maps. This applies to Tk widget paths too.
- A Tk widget path built dynamically rather than passed through a literal
  constructor call — `set w .t; ttk::treeview $w; $w instate …` (the path
  lives in a variable *before* the constructor runs, rather than being
  captured from it) — is not tracked: the provenance scan looks for a
  literal `widgetCmd .path` constructor statement or a `set var [widgetCmd
  .path]` return-value capture, not an arbitrary string later fed into a
  constructor call. Renaming a widget's instance command (`rename .t
  .oldT`) also breaks the association, since it is keyed on the name
  observed at creation time.
- A negative-number argument that a command genuinely treats as a value is
  correctly *not* highlighted as an option — by design.

## Test anchors

  `generic_option_scan_stops_at_double_dash`, `object_method_options_resolve_via_registry`,
  `direct_constructor_dispatch_resolves_method`, `command_substitution_head_recurses_not_command_token`,
  `variable_command_head_is_a_variable`, `computed_command_head_does_not_overlap`,
  `collection_dispatch_resolves_user_configurable_method`, `user_object_handle_method_resolves`,
  `dict_for_loop_var_dispatch_resolves`, `dict_map_in_return_dispatch_resolves`,
  `cross_file_constructor_dispatch_resolves`, `interproc_param_dispatch_resolves`,
  `my_self_call_resolves`, `snit_self_call_resolves`, `snit_install_component_dispatch_resolves`,
  `snit_bare_constructor_dispatch_resolves`, `snit_typemethod_call_does_not_type_handle`,
  `my_configure_property_options_resolve`, `proc_return_object_dispatch_resolves`
  `collection_class_bridges_across_methods`, `spicegentcl_configurable_device_shape_resolves`,
  `interproc_param_from_object_arg_is_a_handle`, `interproc_param_flows_through_call_chain`,
  `aliasing_copies_handle_class`, `constructor_param_typed_from_object_arg`,
  `instance_var_from_constructor_param_bridges_methods`, `snit_named_constructor_types_handle`
- `rust/tcl-lsp-db/src/lib.rs` — `cross_file_object_dispatch_resolves_via_project_index`
  `list_of_objects_lindex_types_element`, `heterogeneous_object_collection_drops_element_class`
  `bareword_widget_path_is_a_handle`, `var_captured_widget_path_is_a_handle`
- `rust/tcl-registry/src/commands/ticklecharts/mod.rs` — `chart_factory_and_methods_resolve`
  `tk_widget_constructors_declare_creates_instance_at`,
  `tk_widgets_with_subcommands_self_reference_their_object_class`
  `bareword_widget_constructor_binds_instance_class`,
  `var_captured_widget_constructor_binds_instance_class`,
  `simple_widget_without_subcommands_still_binds_instance_class`
  `obj_method_hover_fires_for_var_captured_widget`
  `widget_var_captured_completion_offers_subcommands`,
  `bareword_completion_does_not_leak_unrelated_variable_class`
  covering W001/E002/E003 firing and abstention, including
  `abstains_when_receiver_is_ambiguous_across_procs` and
  `resolves_when_widget_created_after_the_proc_that_uses_it_is_defined`

## Discoverability

- [KCS feature index](README.md)
- [Command registry](../../../docs/design/compiler/command-registry.md)
- [Semantic tokens](kcs-feature-semantic-tokens.md)
- [TclOO object-type tracking — design](../../design/tcloo-object-typing.md)
- [Tk widget instance-command typing — design](../../design/tk-widget-instance-typing.md)
