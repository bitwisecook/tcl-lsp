# Object systems: one model, specialised per system

A plan, not yet a contract. Measured gaps are in the [signed scan and precedence handover](handover/signed-scan-precedence.md) (queue items 2 and 3) and the fixtures in [`oo-dispatch-probe/`](handover/oo-dispatch-probe/README.md). The existing handle → class carrier is [object-type-lattice.md](../../design/compiler/object-type-lattice.md).

## Where things stand

The registry already describes class definitions as data: `DefinitionBodyGrammar` (`tcl-registry/src/definer.rs`) with `MemberSpec` rows whose `MemberEffect` says what a member does (`Callable`, `Forward`, `StateDeclaration`, `Relation`, `Visibility`, `Retraction`, `InitScript`, `Configuration`), plus manufacturers, built-in object methods, implicit variables and a member-body namespace path. SpecTcl's `definition_body { … }` block can declare one, and Jim's classes are declared that way.

What stops this being a generic model:

1. **Behaviour is keyed on a closed `DefinerFamily` enum** (`TclOo`, `Snit`, `Itcl`, `Clay`, `JimClass`, `SpecTcl`, `SslicTcl`). The registry matches on it for namespace policy, export rule, command installation and member shape (`definer.rs` `manufactures_runtime_commands`, `members_are_tcloo_methods`, `member_current_namespace`, `receiver_namespace_commands`; `definer/command_installation.rs`; `definer/native_declarations.rs`). The compiler matches on it or names one grammar directly in about twenty places (`signature_scan/walker.rs` `handle_oo_class`/`handle_snit_type`/`handle_itcl_class`/`handle_jim_class`, `analyser/original_definer.rs`, `command_binding/source_transition_advice/source_class.rs`, `command_binding/receiver_self.rs` and `receiver_variable_inventory.rs` on `TCLOO_GRAMMAR`, `analyser/oo/snit.rs`, `signature_scan/ctx.rs` `FACTORY_SKIP_NONCOMMAND_HEADS`, `interprocedural/method_summaries.rs`, and in `tcl-lsp-core` and `tcl-lsp-server`). SpecTcl's `family` word must name one of the seven, and an omitted family means TclOO, so a new system has to borrow another's behaviour.
2. **Two class models.** Source-defined classes go through the definer grammar and W308/E002 (`analyser/diagnostics/var_command/method_dispatch.rs`, `method_resolution.rs`); registry classes (Tk widgets, ticklecharts) go through `ObjectClassSpec` and W001 (`analyser/diagnostics/widget_command.rs`). Arity, unknown-method and prefix rules are implemented twice.
3. **The analyser's class model is stringly typed.** `MethodDef.kind` and `visibility` are strings and `ClassDef` keeps separate lists per member kind (`analyser/types/class_members.rs`, `class_def.rs`), so each system's rules are re-derived from keywords.
4. **Missing concepts.** No delegation (snit `delegate`/`expose` are `Configuration` rows), no components, no unified option/property surface (itcl public variables, snit options and TclOO 9 properties are three things), no next-in-chain helper other than TclOO's `next` (itcl `chain`), no self-call convention other than `my` (XOTcl/NX `:method`, snit `$self`, itcl `$this`). The receiver helpers (`my`, `self`, `next`) are TclOO-named traits.
5. **Method bodies are second-class** in the per-function pass (no SSA, relative heads refused), and named objects (`C create o`) and handle objects (`set o [C new]`) take different checking paths.

## Target model

Each object system is a registry profile: data in Rust specs or a SpecTcl `object_system` declaration. The compiler reads only the profile's facts, never a family or keyword. `DefinerFamily` becomes an identifier used for display and grouping, and SpecTcl accepts any name.

### Profile facts

Every current `match` on the family becomes one of these:

| Fact | Values | Systems |
| --- | --- | --- |
| Definition shape | script of members; dictionary of variables then two-word member commands; per-member commands (`oo::define C method …`) | TclOO, itcl, snit, NX; Jim; TclOO one-member forms, `itcl::body` |
| Member body namespace | the defined entity; the runtime receiver; plus a helper path (`::oo::Helpers`) | itcl/snit; TclOO; TclOO |
| Instance installation | object command named at a word; handle returned; `%AUTO%`; bare-word construction (`C name`); two-word dispatch | TclOO `create`/`new`; snit/itcl; Jim |
| Constructor rules | explicit constructor member; empty body deletes it (TclOO); arguments forwarded from the factory; chained base construction | all |
| Destruction | `destroy` method; `itcl::delete object`; `rename o {}` | all |
| Export rule | all public; lowercase-initial exported (TclOO); declared access (itcl `public`/`protected`/`private`, TclOO 9 `private`) | |
| Resolution order | filters, mixins, class chain, object (TclOO); `inherit` list (itcl); none plus delegation (snit) | |
| Chain helper | `next`/`nextto`; `chain`; none | TclOO; itcl; snit |
| Self helpers | command (`my`, `self`, NX `:name`); implicit variables (`$self`, `$type`, `$win`, `$selfns`, itcl `$this`) | |
| Option surface | `-name` through `configure`/`cget`, with declared names and defaults | itcl public variables, snit options, TclOO 9 properties, Tk widgets |
| Delegation | method or option, to a component, with `except`/`as`, `*` | snit, itcl 4 |
| Reshaping | class change, copy, mixin/superclass slot operations, `deletemethod`, `renamemethod`, `export`/`unexport` | TclOO |
| Unknown handling | `unknown` method; open member set; prefix matching | TclOO; Jim/snit; Tk |

New `MemberEffect` variants cover what `Configuration` hides today: `Delegation { kind, target_component, exceptions, rename }`, `Component { name, installer }` and `Option { name, default, accessors }`. `RelationSlot` stays for ancestry, mixins and filters.

### One class fact for source and registry classes

Registry classes (`ObjectClassSpec`) and source classes produce the same class fact: callable members with arity, visibility and the receiver they need, state declarations, relations, options and delegations. W308, W001 and E002 for method calls come from one checker that reads this fact and the profile's export, resolution and unknown rules. The analyser's string fields become the registry's enums.

### Objects as values

A receiver is one typed value however it was made: a command named by `C create o`, a handle from `C new`, `%AUTO%`, or `my`/`$self`/`$this` inside a body. The handle lattice in object-type-lattice.md carries it. Reshaping operations are world transitions on the class or object fact (add, remove, rename, re-export, change class, copy) rather than a fall to "unknown".

### Bodies in the per-function pass

Every callable member body becomes a function unit with the same pass as a procedure. Entry facts come from the profile: instance variables from `StateDeclaration` rows, implicit variables, formals, and a command-resolution path (object namespace, helper path, then `::`). Relative heads resolve through that path, refused only when the document can install a command in a receiver namespace. This closes the dataflow gap (W233, W210, W156) and the false W123 on `my`, `self`, `next`, `link` and `callback`.

### Systems to describe

| System | Work |
| --- | --- |
| TclOO 8.6–9.1 | Move `TCLOO_GRAMMAR` uses out of the compiler; add one-member `oo::define`/`oo::objdefine` forms, `initialise`, `classmethod`, `private`, `-export`/`-unexport`, `oo::configurable` properties, `oo::singleton`/`oo::abstract`, `link`, `callback`, `classvariable`, reshaping. |
| itcl 4 | `itcl::body`, `chain`, `public`/`protected`/`private`, public variables as options, `itcl::delete`, both `itcl` and `Itcl` package names, itcl 4 `delegate`/`component`. Oracle: itcl 4.3.5 bundled with the Google Cloud SDK in this container (see the probe README). |
| snit | Components, `install`, `delegate method … to` and `delegate option … to`, `expose`, options with `-configuremethod`/`-cgetmethod`, `%AUTO%`, `typemethod`, widget adaptors. Oracle: tcllib once its download works here. |
| Jim `oo` | Already a SpecTcl profile; move `analyser/oo/jim_classes.rs` and `jim_stock_class*.rs` onto profile facts. |
| Clay | Keep its conditional inventory; express it through the same profile. |
| XOTcl / NX | New profile: `nx::Class create C { :property x; :public method m {} {} }`, colon-prefixed self calls, `:require`, `-class` relations. Needs the packages for oracle evidence. |
| stooop | Members are procedures named `C::m` taking `this`; `new C …`, `delete $o`. Replace the single structural spec. |
| Tk megawidgets | TclOO subclasses of `::tk::Megawidget`; covered once TclOO and registry classes share one fact. |
| critcl | Not a class system: a command definer. `critcl::cproc name {type name ?default?} rtype body` defines a Tcl command whose arity and argument domains come from the C parameter list (`int`, `long`, `double`, `bool`, `Tcl_Obj*`, defaults, optional trailing). This feeds `arg_domains`. `critcl::ccommand` defines a command with unknown arity unless declared. `critcl::cconst` and `critcl::cdata` define constant-result commands. The C bodies are not analysed. |

### SpecTcl

Add an `object_system NAME { … }` block (or widen `definition_body`) carrying the profile facts above, with the new member effects as `member -effect` values and a vocabulary bump. Spec Studio gets the matching fields (registry field, loader, renderer, studio, coverage, as for any SpecTcl addition). A user library such as apave can then declare its own system without Rust changes.

## Order

1. Write the profile facts as registry fields, populated for the existing seven families so behaviour is unchanged; replace each family `match` in the registry with a fact read. Tighten the registry-axes `definition_body` ratchet as compiler sites move; retire stale waivers (`original_members.rs` and `retained_member_source.rs` already have `MemberEffect::Relation` available).
2. Move the compiler's family and keyword sites onto the facts, one consumer at a time, each with the probe fixtures as conformance tests checked against tclsh 8.6–9.1 (and itcl 4.3.5).
3. Bodies into the per-function pass with profile entry facts (closes the body and W123 gaps).
4. One class fact and one method-call checker for source and registry classes; typed analyser model; named objects and handles on one path.
5. Reshaping transitions, options, delegation and components.
6. SpecTcl `object_system` block and Spec Studio support; move Jim onto it.
7. New profiles: itcl completeness, snit delegation, stooop, XOTcl/NX, critcl command definer, Tk megawidgets.
