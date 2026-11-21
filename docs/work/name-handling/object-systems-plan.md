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

## Defining a system entirely in SpecTcl

The aim is that snit, itcl, critcl, stooop, NX, Jim's `oo` and a user's own system are each a SpecTcl declaration with no Rust of their own. Rust implements a fixed set of **mechanisms** once; a **profile** selects and parameterises them. The shipped profiles are written in SpecTcl and compiled into the registry like any other pack, so the Rust per-family code disappears. A behaviour no mechanism covers (XOTcl's guarded mixins, say) is added as a new mechanism once and is then available to every profile. Members produced by running arbitrary Tcl at definition time (a loop that calls `method`) are outside any profile and stay with the analyser's ordinary evaluation.

### Mechanisms

| Mechanism | Choices a profile makes |
| --- | --- |
| Member sources | a definition script with a member vocabulary (exists); a per-member command naming its class (`oo::define C method`, `C method m` in Jim); a qualified-name body (`itcl::body C::m`); procedures named into the class namespace taking a receiver formal (stooop `proc C::m {this}`); a variable dictionary (Jim) |
| Member effects | the existing eight, plus delegation, component, option (with handler members such as snit `-configuremethod`), hierarchical method names (snit `method {a b}`) and reshaping (remove, rename, re-export, change class, copy) |
| Parameter grammars | Tcl formals; TclOO/itcl formals (itcl constructors also take an initialiser block); NX parameter specifications (`-name:integer,required`); C declarations for critcl. Each maps a parameter to an arity contribution and an argument domain (`arg_domains`) |
| Lifecycle | factories (exists); bare-word construction (exists); generated names (`%AUTO%`, `::oo::ObjN`); constructor argument forwarding; base construction (`next`, itcl initialiser, snit `install`); destruction routes; the TclOO rule that an empty constructor or destructor body deletes it |
| Dispatch | lookup order (TclOO filters/mixins/class chain, itcl `inherit` order, single class, delegation fallback); export rule (pattern, declared access, all); unknown handling (handler method, open set, prefix matching) |
| Body environment | namespace (defined entity, object, type), command path (`::oo::Helpers`), implicit variables, how instance variables become visible (declared and automatic, as in itcl and snit; per-method `my variable`; namespace variables) |
| Receiver helpers | commands with a semantic role — self call (`my`, NX `:m`), next call (`next`, `nextto`, `chain`), introspection (`self`, `info`), method callback (`callback`, `mymethod`), variable link (`link`, `classvariable`) — and implicit receiver variables (`$self`, `$this`, `$win`) |
| Options | declared names and defaults, `configure`/`cget` routing, per-option handlers and value types (snit `-type`) |
| Command definers | a command that defines a command: name word, parameter list under a grammar, a foreign or Tcl body. Covers critcl `cproc`/`ccommand`/`cconst` and a user's own `proc` wrappers |
| Availability | package names (`itcl` and `Itcl`), dialects, versions |

### Inheritance and lookup order as code

Lookup order differs between systems in ways a fixed enum would not survive; measured in [`oo-dispatch-probe/inheritance/`](handover/oo-dispatch-probe/inheritance/results.txt):

| Hierarchy | TclOO 8.6, 9.0, 9.1 | itcl 4.3.5 (Tcl 9.0, 9.1) | C3 |
| --- | --- | --- | --- |
| Diamond D(B,C), B(A), C(A) | D B C A | definition refused: "inherits base class ::A more than once" | D B C A |
| Conflict Z(X,Y), X(A,B), Y(B,A) | Z X Y B A | refused (repeated base) | refused (inconsistent order) |
| Distinct bases D2(B2,C2) | D2 B2 C2 | D2 B2 C2 | D2 B2 C2 |
| Object mixin F, class mixin M, object method, class D | F M object D B C A; a class filter runs first | — | — |

TclOO walks depth first, left to right, and keeps the last occurrence of a repeated class, so it accepts orders C3 refuses. itcl refuses any repeated base.

A profile therefore defines lookup as Tcl code, run by the repository's own interpreter (`tcl-vm`, already a dependency of `tcl-spectcl` and `tcl-compiler`):

- **Inputs.** A relation graph built from the profile's `Relation` effects: for each class, its ordered lists per relation slot (superclass, mixin, filter, and any slot the profile declares, such as an object-level class or delegation target), the same for each object, and which members each class defines.
- **Outputs.** `linearise` returns the ordered classes for a class; `call_chain` returns, for an object and a method name, the ordered implementations with their kind (filter, mixin, object, class, unknown handler). Either may return a structured refusal (code, message, offending paths), which becomes a diagnostic at the definition — itcl's repeated base, C3's inconsistent order.
- **Partial knowledge.** A relation the analyser cannot prove arrives as an `unknown` element. The shipped procedures keep the known prefix and end the result with an open marker, so method checks abstain past it rather than guess; a custom procedure must do the same, and the loader checks it does.
- **Sandbox.** Lists, dictionaries, control flow, `proc` and `error` only; no I/O, introspection or time; a command budget through the VM's existing limits; results cached per profile and graph.
- **Standard library.** SpecTcl ships `lookup::c3`, `lookup::dfs-keep-last` (TclOO), `lookup::dfs-refuse-repeats` (itcl), `lookup::single` and `lookup::none`, plus helpers such as `lookup::implementations`, as procedures a profile can call or compose with its own mixin and filter placement. The shipped profiles use them.
- **Checks.** A profile carries `lookup_test GRAPH EXPECTED` rows; the rows for shipped profiles are the oracle measurements above. Shipped algorithms also have native Rust implementations for speed, kept equal to the SpecTcl procedures by a differential test over generated graphs; a custom profile runs in the VM.
- **Consumers.** Method resolution (W308, E002), the target of `next`/`nextto`/`chain` and its arity, hover and go-to-definition, find-references and rename across overrides, the optimiser's devirtualisation, and interprocedural summaries of the called method.

The same mechanism suits other rules that are naturally code rather than data — the export predicate, generated instance names, unknown-method routing — but data stays the default where a table is enough.

```tcl
object_system mini {
    lookup_order {
        proc linearise {graph class} { lookup::dfs-keep-last $graph $class superclass }
        proc call_chain {graph object method} {
            set chain [linearise $graph [dict get $graph objects $object class]]
            return [lookup::implementations $graph $chain $method]
        }
    }
    lookup_test {classes {D {superclass {B C}} B {superclass A} C {superclass A} A {}}} \
                {linearise D} {D B C A}
}
```

### Sketches

Proposed syntax, extending the existing `definition_body` block. The words and value names are illustrative, not settled.

A user's own system, built on namespaces with a `new` factory:

```tcl
object_system mini {
    member_namespace   defined-entity
    export_rule        all
    lookup_order       single
    instance_variables declared-automatic
    implicit_vars      {self}
    receiver_helper    self -role self-call
    destruction        {method destroy}
}

command mini::class {
    arg 0 -role Name
    arg 1 -role Body
    definition_body {
        system mini
        member method -roles {0 Name 1 ParamList 2 Body} -effect {callable -receiver instance -role method}
        member field  -roles {0 VarWrite} -effect {state-declaration per-instance}
        manufacturer new -constructor-args-from 1
    }
}
```

critcl as a command definer. The type names, default syntax and the hidden interpreter parameter must be taken from critcl's documentation and checked against a critcl build before the profile is written:

```tcl
param_grammar critcl-c {
    shape    type-name-pairs
    type int        -domain {Range -2147483648 2147483647}
    type double     -domain Double
    type Tcl_Obj*   -domain Any
    type Tcl_Interp* -implicit
}

command critcl::cproc {
    arg 0 -role Name
    arg 1 -role ParamList -grammar critcl-c
    arg 3 -role ForeignBody -language c
    defines_command -name-at 0 -params-at 1
}
```

snit's delegation, replacing today's `configuration` rows:

```tcl
member delegate -roles {0 DelegationKind 1 Name 3 ComponentRef} \
                -effect {delegation -keyword-at {2 to} -options {except as using}}
```

### Checking a profile

A profile ships with conformance fixtures in the style of `oo-dispatch-probe/`: each body form with a dataflow witness, each dispatch route used correctly and wrongly, run under the real package where one is available (itcl 4.3.5 is in this container; snit and stooop come from tcllib; critcl and NX need their packages). The analyser must be silent on the correct uses and report the wrong ones. Spec Studio validates a profile's coverage of the mechanisms it selects.

## Order

1. Write the profile facts as registry fields, populated for the existing seven families so behaviour is unchanged; replace each family `match` in the registry with a fact read. Tighten the registry-axes `definition_body` ratchet as compiler sites move; retire stale waivers (`original_members.rs` and `retained_member_source.rs` already have `MemberEffect::Relation` available).
2. Move the compiler's family and keyword sites onto the facts, one consumer at a time, each with the probe fixtures as conformance tests checked against tclsh 8.6–9.1 (and itcl 4.3.5).
3. Bodies into the per-function pass with profile entry facts (closes the body and W123 gaps).
4. One class fact and one method-call checker for source and registry classes; typed analyser model; named objects and handles on one path.
5. Reshaping transitions, options, delegation and components.
6. SpecTcl `object_system`, `param_grammar` and `defines_command` support in the loader, renderer and Spec Studio; rewrite TclOO, itcl, snit, Clay and Jim as shipped `.tclspec` profiles and delete their Rust grammars and family code.
7. New profiles: itcl completeness, snit delegation, stooop, XOTcl/NX, critcl command definer, Tk megawidgets.
