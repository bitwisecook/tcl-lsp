// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Object classes and handles: the report package, Tk widget constructors, instance methods, handle bindings, definer families and manufacturers.

use super::*;

// Report package command specs and scoped-body / object model.

/// `report::defstyle` carries the scoped style-definition environment, with the
/// report configuration methods and their operations as registry data.
#[test]
fn report_defstyle_has_scoped_body_environment() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let spec = reg
        .get_for_surface("report::defstyle", ds)
        .expect("report::defstyle registered in tcl8.6");
    let env = spec
        .body_scope
        .expect("report::defstyle carries a body scope");
    assert!(
        env.include_sibling_definitions,
        "styles callable in sibling bodies"
    );
    // A representative slice of the 19 report methods.
    for cmd in [
        "top",
        "topdatasep",
        "data",
        "botdata",
        "columns",
        "size",
        "pad",
        "justify",
    ] {
        assert!(env.is_command(cmd), "`{cmd}` is a scoped command");
    }
    // Separators support enable/disable; data lines do not.
    let top = env.command("top").unwrap();
    assert!(top.subcommand("enable").is_some(), "separator enables");
    let data = env.command("data").unwrap();
    assert!(data.subcommand("set").is_some(), "data line sets");
    assert!(
        data.subcommand("enable").is_none(),
        "data line does not enable"
    );
    // Config methods are plain (no ensemble ops).
    assert!(env.command("columns").unwrap().subcommands.is_empty());
}

/// `report::report` is a documented object factory with instance methods and
/// option values.
#[test]
fn report_report_object_class_is_modelled() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let spec = reg
        .get_for_surface("report::report", ds)
        .expect("report::report registered");
    assert_eq!(
        spec.creates_instance_at,
        Some(0),
        "names its object at arg 0"
    );
    assert!(spec.hover.is_some(), "carries hover");
    assert_eq!(spec.arg_role_at(0), Some(ArgRole::Name));
    let class = spec
        .object_class
        .expect("report::report has an object class");
    for m in [
        "destroy",
        "printmatrix",
        "columns",
        "size",
        "pad",
        "justify",
        "top",
        "data",
    ] {
        assert!(class.instance_method(m).is_some(), "method `{m}` modelled");
    }
    // Option values on the padding / justification methods.
    let pad = class.instance_method("pad").unwrap();
    let pad_vals: Vec<&str> = pad.arg_values_at(1).iter().map(|v| v.value).collect();
    assert!(pad_vals.contains(&"both"), "pad where-values: {pad_vals:?}");
    let justify = class.instance_method("justify").unwrap();
    let jvals: Vec<&str> = justify.arg_values_at(1).iter().map(|v| v.value).collect();
    assert!(jvals.contains(&"center"), "justify values: {jvals:?}");
}

/// Every Tk/ttk widget constructor names the widget path it creates at
/// arg 0, so a later `.w <subcommand> …` / `$w <subcommand> …` dispatch can
/// resolve back to this same spec (see
/// `docs/design/analysis/tk-widget-instance-typing.md`).
#[test]
fn tk_widget_constructors_declare_creates_instance_at() {
    let (reg, ds) = reg_and_set("tcl8.6");
    let widgets = [
        // Classic widgets.
        "button",
        "canvas",
        "checkbutton",
        "entry",
        "frame",
        "label",
        "labelframe",
        "listbox",
        "menu",
        "menubutton",
        "message",
        "panedwindow",
        "radiobutton",
        "scale",
        "scrollbar",
        "spinbox",
        "text",
        "toplevel",
        // Raw ttk widgets.
        "ttk::button",
        "ttk::combobox",
        "ttk::entry",
        "ttk::frame",
        "ttk::label",
        "ttk::notebook",
        "ttk::progressbar",
        "ttk::scale",
        "ttk::separator",
        "ttk::sizegrip",
        "ttk::treeview",
        // ttk_extra widgets.
        "ttk::checkbutton",
        "ttk::menubutton",
        "ttk::panedwindow",
        "ttk::radiobutton",
        "ttk::spinbox",
    ];
    for name in widgets {
        let spec = reg
            .get_for_surface(name, ds)
            .unwrap_or_else(|| panic!("{name} registered"));
        assert_eq!(
            spec.creates_instance_at,
            Some(0),
            "{name} names its widget path at arg 0"
        );
    }
}

/// Widgets with a non-empty `subcommands` table bind a self-referential
/// `object_class` — the created widget's instance command dispatches
/// through the *same* `SubCommand` slice as the constructor's own
/// `subcommands`, so `registry.instance_method` resolves real widget
/// subcommands with no separate hand-maintained method table (and no
/// drift between the two — a drift guard, since `instance_methods` and
/// `subcommands` are asserted to be the literal same slice).
#[test]
fn tk_widgets_with_subcommands_self_reference_their_object_class() {
    let (reg, ds) = reg_and_set("tcl8.6");
    // (widget name, a subcommand it declares, an unrelated widget's
    // subcommand it must NOT accept).
    let cases = [
        ("ttk::treeview", "instate", "curselection"),
        ("ttk::notebook", "instate", "curselection"),
        ("listbox", "curselection", "instate"),
        ("text", "tag", "curselection"),
        ("canvas", "bind", "curselection"),
        ("entry", "icursor", "curselection"),
        ("menu", "add", "curselection"),
        ("panedwindow", "add", "curselection"),
        ("spinbox", "icursor", "curselection"),
    ];
    for (name, own_sub, foreign_sub) in cases {
        let spec = reg
            .get_for_surface(name, ds)
            .unwrap_or_else(|| panic!("{name} registered"));
        assert!(
            !spec.subcommands.is_empty(),
            "{name} expected to have a non-empty subcommand table"
        );
        let class = spec
            .object_class
            .unwrap_or_else(|| panic!("{name} has a self-referential object class"));
        assert_eq!(
            class.class_name, name,
            "{name}'s object class is self-referential"
        );
        // This is a pointer-identity check, and it is only meaningful because
        // each widget's table is a `static SUBCOMMANDS: [SubCommand; N]` that
        // both use sites reach via `&SUBCOMMANDS` — one static, one address.
        // A `const` table has NO stable address (every use site may
        // materialise its own copy), which would make this assertion pass at
        // opt-level 0 purely by accident of codegen and fail once the crate is
        // built at opt-level 2.  If you turn these tables back into
        // `const`s, this assertion becomes a coin flip again — keep them
        // `static`.
        assert!(
            std::ptr::eq(class.instance_methods, spec.subcommands),
            "{name}'s instance_methods must be the literal same slice as its \
             own subcommands — any other pairing risks the two drifting apart"
        );
        assert!(
            reg.instance_method(name, own_sub).is_some(),
            "{name} instance dispatch resolves its own subcommand `{own_sub}`"
        );
        assert!(
            reg.instance_method(name, foreign_sub).is_none(),
            "{name} instance dispatch must not accept `{foreign_sub}` from an unrelated widget"
        );
    }
}

#[test]
fn instance_methods_follow_the_owning_package_lifecycle() {
    // `current` is a Tk 9.1 addition to ttk::treeview. The class itself is
    // older, so this exercises method-row lifecycle filtering rather than
    // command-level availability. The registry must make the same answer
    // available to every downstream object-method consumer.
    for (dialect, current_is_available) in [("tcl9.0", false), ("tcl9.1", true)] {
        let reg = static_context_for(dialect).commands();
        assert_eq!(
            reg.instance_method("ttk::treeview", "current").is_some(),
            current_is_available,
            "ttk::treeview current under {dialect}"
        );
        assert_eq!(
            reg.instance_methods("ttk::treeview")
                .iter()
                .any(|method| method.name == "current"),
            current_is_available,
            "instance-method enumeration under {dialect}"
        );
        assert!(
            reg.instance_method("ttk::treeview", "instate").is_some(),
            "an older ttk::treeview method remains available under {dialect}"
        );
    }
}

#[test]
fn object_method_prefix_policy_is_strict_by_default_and_enabled_for_tk() {
    let (reg, ds) = reg_and_set("tcl8.6");

    let entry = reg.get_for_surface("entry", ds).unwrap();
    let entry_class = entry.object_class.unwrap();
    assert_eq!(
        entry_class.method_prefix_matching,
        tcl_registry::abbrev::PrefixMatching::Enabled
    );
    assert!(
        reg.instance_method("entry", "get").is_some(),
        "exact Tk method"
    );
    assert!(
        reg.instance_method("entry", "conf").is_some(),
        "unique Tk method prefix"
    );
    assert!(
        reg.instance_method("entry", "c").is_none(),
        "ambiguous Tk method prefix must abstain"
    );

    let report = reg.get_for_surface("report::report", ds).unwrap();
    let report_class = report.object_class.unwrap();
    assert_eq!(
        report_class.method_prefix_matching,
        tcl_registry::abbrev::PrefixMatching::Strict
    );
    assert!(reg.instance_method("report::report", "destroy").is_some());
    assert!(
        reg.instance_method("report::report", "des").is_none(),
        "non-Tk object methods are exact by default"
    );
}

/// Every `::report::*` command has a dedicated, hover-bearing spec.
#[test]
fn report_namespace_commands_have_specs() {
    let (reg, ds) = reg_and_set("tcl8.6");
    for cmd in [
        "report::report",
        "report::defstyle",
        "report::rmstyle",
        "report::stylearguments",
        "report::stylebody",
        "report::styles",
    ] {
        let spec = reg
            .get_for_surface(cmd, ds)
            .unwrap_or_else(|| panic!("{cmd} present"));
        assert!(spec.hover.is_some(), "{cmd} carries hover");
        assert_eq!(
            spec.required_package,
            Some("report"),
            "{cmd} gated on report"
        );
    }
}

// Object-handle bindings — which argument of a call becomes a
// handle, and of what class, is registry data rather than a walker's
// command-name match.

/// `set` declares the handle-binding layout its value word implies, and the
/// query resolves it through the same `get` path every other registry query
/// uses — so `::set` answers identically to the bare spelling.
#[test]
fn set_declares_its_handle_binding_for_every_spelling() {
    use tcl_registry::{BoundHandle, HandleClassSource};

    let reg = CommandRegistry::build_default();
    for spelling in ["set", "::set"] {
        let layout = reg
            .handle_binding(spelling)
            .unwrap_or_else(|| panic!("`{spelling}` must declare a handle binding"));
        assert_eq!(
            layout.resolve(&["axis", "[verticalAxis $win.a]"]),
            Some(BoundHandle {
                name: "axis",
                class_word: "[verticalAxis $win.a]",
                class_source: HandleClassSource::ConstructionValue(1),
            }),
            "`{spelling}` must bind argument 0 from its value word"
        );
        // The one-argument read form binds nothing.
        assert_eq!(layout.resolve(&["axis"]), None);
    }
    // A command with no such layout answers `None` — no guessing.
    assert!(reg.handle_binding("puts").is_none());
    assert!(reg.handle_binding("nosuchcommand").is_none());
}

/// snit's `install` is a **member-body** command, not a global one: it must
/// carry its layout on the definition-body grammar and must NOT be registered
/// as a global spec (which would make a user's own `proc install` look like a
/// built-in everywhere).
#[test]
fn snit_install_is_a_member_body_binding_not_a_global_command() {
    use tcl_registry::{BoundHandle, HandleClassSource};

    let reg = CommandRegistry::build_default();
    assert!(
        reg.get("install").is_none(),
        "`install` must not become a global command spec"
    );

    let bindings = reg.member_body_handle_bindings();
    let (_, layout) = bindings
        .iter()
        .find(|(name, _)| *name == "install")
        .unwrap_or_else(|| panic!("snit must declare an `install` handle binding: {bindings:?}"));
    assert_eq!(
        layout.resolve(&["axis", "using", "verticalAxis", "$win.a"]),
        Some(BoundHandle {
            name: "axis",
            class_word: "verticalAxis",
            class_source: HandleClassSource::Word(2),
        })
    );
    // Without the `using` keyword the layout does not fire, so a user's own
    // three-word `install` binds nothing.
    assert_eq!(layout.resolve(&["a", "b", "c"]), None);
    // The snit `type` and `widget` grammars share one command set — the query
    // deduplicates rather than reporting the same word twice.
    assert_eq!(
        bindings.iter().filter(|(n, _)| *n == "install").count(),
        1,
        "member-body bindings must be deduplicated: {bindings:?}"
    );
}

/// Whether a class family constructs from a **bare instance name** is a
/// grammar fact, not a metaclass-name prefix test.
#[test]
fn bare_word_construction_is_declared_per_definer_family() {
    use tcl_registry::definer::{ITCL_GRAMMAR, SNIT_GRAMMAR, SNIT_WIDGET_GRAMMAR, TCLOO_GRAMMAR};

    // snit(n), "The Type Command": `$type name ?args?` implies `create`;
    // tclsh 9.0.4: `oo::class create ::C {}; ::C x` -> `unknown method "x"`,
    // so only the snit family constructs by bare word.
    let families = [
        ("snit::type", &SNIT_GRAMMAR, true),
        ("snit::widget", &SNIT_WIDGET_GRAMMAR, true),
        ("TclOO", &TCLOO_GRAMMAR, false),
        ("itcl::class", &ITCL_GRAMMAR, false),
    ];
    for (name, grammar, expected) in families {
        assert_eq!(
            grammar.bare_word_construction, expected,
            "{name}: bare_word_construction"
        );
        // Only snit injects member-body commands today; the others inject none.
        assert_eq!(
            grammar.member_body_command("install").is_some(),
            expected,
            "{name}: member-body `install`"
        );
        // An unknown word is not a member-body command of any family.
        assert!(grammar.member_body_command("nosuchword").is_none());
    }
}

/// Manufacturer spelling and word layout live together in the grammar. A
/// consumer must not infer the name/body positions from `create`-shaped text:
/// `TclOO`'s three manufacturers have three distinct layouts, while snit's
/// homonymous method creates an ordinary instance and has no definition body.
#[test]
fn manufacturer_layout_is_declared_per_definer_family() {
    use tcl_registry::definer::{SNIT_GRAMMAR, TCLOO_GRAMMAR};

    let create = TCLOO_GRAMMAR.manufacturer("create").unwrap();
    assert_eq!(create.names_instance_at, Some(1));
    assert_eq!(create.definition_body_at, Some(2));
    assert_eq!(create.constructor_args_from, 2);

    let new = TCLOO_GRAMMAR.manufacturer("new").unwrap();
    assert_eq!(new.names_instance_at, None);
    assert_eq!(new.definition_body_at, Some(1));
    assert_eq!(new.constructor_args_from, 1);

    let with_ns = TCLOO_GRAMMAR.manufacturer("createWithNamespace").unwrap();
    assert_eq!(with_ns.names_instance_at, Some(1));
    assert_eq!(with_ns.definition_body_at, Some(3));
    assert_eq!(with_ns.constructor_args_from, 3);

    let snit_create = SNIT_GRAMMAR.manufacturer("create").unwrap();
    assert_eq!(snit_create.names_instance_at, Some(1));
    assert_eq!(snit_create.definition_body_at, None);
    assert_eq!(snit_create.constructor_args_from, 2);

    let registry = CommandRegistry::build_default();
    assert_eq!(
        registry.uniform_manufacturer_names_instance_at("create"),
        Some(1)
    );
    assert_eq!(registry.uniform_manufacturer_names_instance_at("new"), None);
    assert!(registry.is_possible_class_construction_word("new"));
    assert!(registry.is_possible_class_construction_word("%AUTO%"));
    assert!(registry.is_possible_class_construction_word(".widget"));
    assert!(!registry.is_possible_class_construction_word("ordinaryMethod"));
}

#[test]
fn registered_metaclass_manufacturer_surface_matches_c_tcl() {
    let registry = CommandRegistry::build_default();

    assert!(
        registry
            .manufacturer_method("oo::class", "create")
            .is_some()
    );
    assert!(
        registry.manufacturer_method("oo::class", "new").is_some(),
        "C Tcl retains private `new` on oo::class itself"
    );
    assert!(
        registry
            .manufacturer_method("oo::class", "createWithNamespace")
            .is_some(),
        "C Tcl retains private createWithNamespace on oo::class itself"
    );
    assert!(
        registry
            .exported_manufacturer_method("oo::class", "new")
            .is_none(),
        "ordinary dispatch cannot reach root `new`"
    );
    assert!(
        registry
            .exported_manufacturer_method("oo::class", "createWithNamespace")
            .is_none(),
        "ordinary dispatch cannot reach root createWithNamespace"
    );
    assert!(
        registry
            .manufacturer_method("oo::abstract", "new")
            .is_some(),
        "the Tcl 9 derived metaclass commands retain their inherited `new`"
    );
    assert!(
        registry
            .exported_manufacturer_method("oo::abstract", "new")
            .is_some(),
        "ordinary dispatch reaches an exported manufacturer"
    );
    assert!(
        registry
            .manufacturer_method("oo::abstract", "createWithNamespace")
            .is_some(),
        "the private method still exists for self-dispatch and export modelling"
    );
    assert!(
        registry
            .exported_manufacturer_method("oo::abstract", "createWithNamespace")
            .is_none(),
        "ordinary dispatch must not pretend an unexported manufacturer is callable"
    );
}

/// The built-in **object** methods each definer family supplies, and — the
/// load-bearing half — which dispatch spellings can reach each one.
///
/// oracle: tclsh 9.0.4 and 8.6.16, byte-identical on both:
///
/// ```text
/// info class methods ::oo::object -all -private
///   -> <cloned> destroy eval unknown variable varname
/// info class methods ::oo::class -all -private
///   -> <cloned> create createWithNamespace destroy eval new unknown variable varname
/// ```
///
/// and, from inside a method of a class declaring only `probe`:
///
/// ```text
/// my varname v      -> ::oo::Obj22::v
/// [self] varname v  -> unknown method "varname": must be destroy or probe
/// $obj varname v    -> unknown method "varname": must be destroy or probe
/// ```
///
/// That last pair is the guard: `my` bypasses `TclOO`'s export
/// filter and the object's own command does not, so a consumer diagnosing an
/// unknown method must ask with the right `MethodReach` or it will either
/// false-positive on the idiomatic `my variable v` or wave through the real
/// error `$obj varname v`.
#[test]
fn builtin_object_methods_are_reach_gated() {
    use tcl_registry::definer::{
        BuiltinMethodReceiver, ITCL_GRAMMAR, MethodReach, SNIT_GRAMMAR, TCLOO_GRAMMAR,
    };

    // `destroy` is the one exported member every TclOO object has: both
    // reaches find it.
    for reach in [MethodReach::ObjectCommand, MethodReach::SelfDispatch] {
        assert!(
            TCLOO_GRAMMAR
                .builtin_object_method("destroy", reach)
                .is_some(),
            "`destroy` is exported, so every reach finds it"
        );
    }

    // The unexported five are reachable only through `my`.
    for name in ["eval", "unknown", "variable", "varname", "<cloned>"] {
        assert!(
            TCLOO_GRAMMAR
                .builtin_object_method(name, MethodReach::SelfDispatch)
                .is_some(),
            "`my {name}` must resolve"
        );
        assert!(
            TCLOO_GRAMMAR
                .builtin_object_method(name, MethodReach::ObjectCommand)
                .is_none(),
            "`$obj {name}` / `[self] {name}` is a real error, not a builtin"
        );
    }

    // The class-object constructors are recorded as such, so a consumer with
    // receiver-kind evidence can narrow; one without may accept both.
    for name in ["new", "create"] {
        let m = TCLOO_GRAMMAR
            .builtin_object_method(name, MethodReach::ObjectCommand)
            .unwrap_or_else(|| panic!("`{name}` is an exported class-object method"));
        assert_eq!(m.receiver, BuiltinMethodReceiver::ClassObject, "{name}");
    }
    assert!(
        TCLOO_GRAMMAR
            .builtin_object_method("createWithNamespace", MethodReach::ObjectCommand)
            .is_none(),
        "createWithNamespace is present but unexported in C Tcl"
    );

    // A word no class system supplies answers for nobody.
    for grammar in [&TCLOO_GRAMMAR, &SNIT_GRAMMAR, &ITCL_GRAMMAR] {
        assert!(
            grammar
                .builtin_object_method("nosuchmethod", MethodReach::SelfDispatch)
                .is_none()
        );
    }

    // snit and itcl dispatch through their own generated machinery, which has
    // no `my`-only tier — every builtin of theirs is reachable either way.
    for (what, grammar, names) in [
        (
            "snit",
            &SNIT_GRAMMAR,
            &["configure", "configurelist", "cget", "destroy", "info"][..],
        ),
        (
            "itcl",
            &ITCL_GRAMMAR,
            &["configure", "cget", "isa", "info"][..],
        ),
    ] {
        for name in names {
            for reach in [MethodReach::ObjectCommand, MethodReach::SelfDispatch] {
                assert!(
                    grammar.builtin_object_method(name, reach).is_some(),
                    "{what} instances have `{name}`"
                );
            }
        }
        // `new` is TclOO's constructor spelling and belongs to no other family.
        assert!(
            grammar
                .builtin_object_method("new", MethodReach::ObjectCommand)
                .is_none(),
            "{what} constructs through its own type command, never `new`"
        );
    }
}
