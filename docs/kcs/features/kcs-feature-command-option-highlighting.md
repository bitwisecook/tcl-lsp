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

   Object provenance is also **interprocedural**: when a proc is called with an
   object argument (a tracked `$var` handle or a direct `[Class new]`), its
   parameter is bound to that class, iterated to a fixpoint so the class flows
   through a chain of calls. So `set p [Pin new]; connect $p` makes `$dev` (the
   parameter) resolve `$dev configure -node …` inside `connect` — the
   param-receiver case the mro_eval experiment measured as ~60% of unresolved
   dispatches. A proc that **returns** an object (`proc make {} {return [C
   new]}; set o [make]`) types the receiving variable likewise.

   A **`my method …` self-call** inside a class body resolves against the
   enclosing class's MRO (the class named at the `oo::class create` /
   `oo::define` head that opened the body) — by far the most common TclOO
   dispatch form. `my configure -prop` colours the property option too.

   **Not resolved (correctly abstains, never mis-highlights):** a bareword
   named-object dispatch (`Foo create obj; obj method`), and non-TclOO object
   systems (`snit::type`, `itcl::class`) — their method word stays a plain
   string rather than a guessed callable. The receiver's class is found from a handle typed by
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

- `rust/tcl-lsp-core/src/semantic_tokens.rs` — `insert_option_and_subcommand_overrides`
  (registry options), `insert_object_method_overrides` (object methods, incl.
  `collection_head_element_classes` for `[dict get …]` receivers and
  `user_class_provides_method` / `insert_user_configure_options` for user
  classes), `insert_generic_option_overrides` (fallback), `head_is_computed` +
  `collect_script` (computed heads are tokenised, not painted as command tokens)
- `rust/tcl-compiler/src/object_types.rs` — `object_handle_classes` /
  `object_collection_classes` (handle & collection → class provenance)
- `rust/tcl-compiler/src/type_infer.rs`, `rust/tcl-compiler/src/types.rs` —
  container element-typing in the SSA lattice (`TypeLattice::element_class`)
- `rust/tcl-compiler/src/object_types.rs` — object-handle → class provenance
- `rust/tcl-registry/src/spec.rs` — `ObjectClassSpec`
- `rust/tcl-registry/src/commands/ticklecharts/mod.rs` — the ticklecharts pack

## Failure modes

- A `-option` on an unmodelled object method is highlighted by shape only, so
  an invalid switch is not distinguished from a valid one.
- A receiver passed through a proc parameter (or a dict value) is not
  provenance-tracked, so it uses the generic fallback rather than the precise
  registry path.
- A negative-number argument that a command genuinely treats as a value is
  correctly *not* highlighted as an option — by design.

## Test anchors

- `rust/tcl-lsp-core/src/semantic_tokens.rs` — `unknown_head_options_classified_generically`,
  `generic_option_scan_stops_at_double_dash`, `object_method_options_resolve_via_registry`,
  `direct_constructor_dispatch_resolves_method`, `command_substitution_head_recurses_not_command_token`,
  `variable_command_head_is_a_variable`, `computed_command_head_does_not_overlap`,
  `collection_dispatch_resolves_user_configurable_method`, `user_object_handle_method_resolves`,
  `dict_for_loop_var_dispatch_resolves`, `dict_map_in_return_dispatch_resolves`,
  `cross_file_constructor_dispatch_resolves`, `interproc_param_dispatch_resolves`,
  `my_self_call_resolves`, `my_configure_property_options_resolve`, `proc_return_object_dispatch_resolves`
- `rust/tcl-compiler/src/object_types.rs` — `collection_of_objects_is_tracked`,
  `collection_class_bridges_across_methods`, `spicegentcl_configurable_device_shape_resolves`,
  `interproc_param_from_object_arg_is_a_handle`, `interproc_param_flows_through_call_chain`
- `rust/tcl-lsp-db/src/lib.rs` — `cross_file_object_dispatch_resolves_via_project_index`
- `rust/tcl-compiler/src/lowering/mod.rs` — `lowers_oo_configurable_class_body`
- `rust/tcl-compiler/src/type_infer.rs` — `dict_of_objects_retrieval_types_element`,
  `list_of_objects_lindex_types_element`, `heterogeneous_object_collection_drops_element_class`
- `rust/tcl-compiler/src/object_types.rs` — `scalar_handle_from_constructor`
- `rust/tcl-registry/src/commands/ticklecharts/mod.rs` — `chart_factory_and_methods_resolve`

## Discoverability

- [KCS feature index](README.md)
- [Command registry](../../../docs/design/compiler/command-registry.md)
- [Semantic tokens](kcs-feature-semantic-tokens.md)
