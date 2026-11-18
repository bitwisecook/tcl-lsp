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

//! Method candidates for objects, class commands and Tk widgets.

use super::*;

#[test]
fn obj_method_completion_includes_inherited() {
    let src = "oo::class create Animal {\n    method eat {} {}\n}\noo::class create Dog {\n    superclass Animal\n    method bark {} {}\n}\nset d [Dog new]\n$d \n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    // Cursor after `$d ` on line 8.
    let items = completions(
        src,
        8,
        3,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"bark"), "own method missing: {labels:?}");
    assert!(
        labels.contains(&"eat"),
        "inherited method missing: {labels:?}"
    );
    assert!(labels.contains(&"destroy"), "builtin missing: {labels:?}");
    // The inherited one is labelled as such.
    let eat = items.iter().find(|i| i.label == "eat").unwrap();
    assert!(
        eat.detail
            .as_deref()
            .unwrap_or("")
            .contains("inherited from ::Animal"),
        "{:?}",
        eat.detail
    );
}

#[test]
fn obj_method_completion_offers_the_builtin_destroy_as_the_class_exports_it() {
    // tclsh 8.6.18, 9.0.4 and 9.1.0:
    //   oo::class create A {method bark {} {}; unexport destroy}
    //   oo::class create P {unexport destroy}
    //   oo::class create C {superclass P; method bark {} {}}
    //   oo::class create D {superclass A; export destroy}
    //   info object methods [X new] -all  ->  A: bark  C: bark  D: bark destroy
    //   and `$o destroy` fails on A and C and runs on D.
    let decls = "oo::class create A {method bark {} {}; unexport destroy}\n\
                 oo::class create P {unexport destroy}\n\
                 oo::class create C {superclass P; method bark {} {}}\n\
                 oo::class create D {superclass A; export destroy}\n";
    for (class, offered) in [("A", false), ("C", false), ("D", true)] {
        let src = format!("{decls}set o [{class} new]\n$o \n");
        let analysis = analyse(&src);
        let registry = CommandRegistry::build_default();
        let items = completions(
            &src,
            5,
            3,
            &analysis,
            Some(&registry),
            None,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        );
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"bark"), "{class}: {labels:?}");
        assert_eq!(labels.contains(&"destroy"), offered, "{class}: {labels:?}");
    }
}

#[test]
fn malformed_braced_instance_head_does_not_offer_method_completion() {
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset x [Dog new]\n${x ba\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        4,
        6,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(
        !items.iter().any(|item| item.label == "bark"),
        "malformed `${{x` must not dispatch through real `x`: {items:?}"
    );
    assert_eq!(
        strip_instance_var("${x}", tcl_lexer::LexerConfig::default()),
        Some("x".to_owned())
    );
    assert_eq!(
        strip_instance_var("$x", tcl_lexer::LexerConfig::default()),
        Some("x".to_owned())
    );
    // Completion intentionally only dispatches simple scalar handles;
    // the shared codec still parses the array decoration, but this
    // feature's documented name validation rejects it.
    assert_eq!(
        strip_instance_var("$arr(idx)", tcl_lexer::LexerConfig::default()),
        None
    );
    assert_eq!(
        strip_instance_var("$::a:::b", tcl_lexer::LexerConfig::default()),
        Some("::a:::b".to_owned())
    );
    assert_eq!(
        strip_instance_var("$foo:::", tcl_lexer::LexerConfig::default()).as_deref(),
        Some("foo:::")
    );
    assert_eq!(
        strip_instance_var("${x", tcl_lexer::LexerConfig::default()).as_deref(),
        None
    );
}

#[test]
fn obj_method_completion_includes_generated_property_accessors() {
    // W308 and the semantic-token
    // classifier accept the `configure` an `oo::configurable`
    // class generates for its `property` members — completion must
    // offer the same surface, or the editor colours a method it will
    // not complete.  `cget` is not generated (`configurable.n`) and
    // must not be offered.
    let src = "oo::configurable create Point {\n    property x\n    method shift {} {}\n}\nset p [Point new]\n$p \n";
    let analysis = analyse9(src);
    let registry = CommandRegistry::build_default();
    // Cursor after `$p ` on line 5.
    let items = completions(
        src,
        5,
        3,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"shift"), "declared method: {labels:?}");
    assert!(
        labels.contains(&"configure"),
        "generated accessor missing: {labels:?}"
    );
    assert!(
        !labels.contains(&"cget"),
        "cget is not generated and must not be offered: {labels:?}"
    );
    let configure = items.iter().find(|i| i.label == "configure").unwrap();
    assert!(
        configure
            .detail
            .as_deref()
            .unwrap_or("")
            .contains("generated"),
        "{:?}",
        configure.detail
    );
}

#[test]
fn obj_method_completion_inherits_generated_accessors_through_plain_subclass() {
    // A plain `oo::class` subclass of a configurable class inherits the
    // generated `configure` as a real method — same MRO fold as W308.
    let src = "oo::configurable create Point {\n    property x\n}\noo::class create Point3 {\n    superclass Point\n    property z\n}\nset p [Point3 new]\n$p \n";
    let analysis = analyse9(src);
    let registry = CommandRegistry::build_default();
    // Cursor after `$p ` on line 8.
    let items = completions(
        src,
        8,
        3,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"configure"),
        "inherited generated accessor missing: {labels:?}"
    );
}

#[test]
fn obj_method_completion_offers_no_accessor_on_a_plain_class() {
    // The other direction: a plain `oo::class` with no configurable
    // ancestor has no `configure` — offering one would complete a
    // method that fails at dispatch (the W308 true positive's twin).
    let src = "oo::class create Plain {\n    method only {} {}\n}\nset p [Plain new]\n$p \n";
    let analysis = analyse9(src);
    let registry = CommandRegistry::build_default();
    // Cursor after `$p ` on line 4.
    let items = completions(
        src,
        4,
        3,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.contains(&"configure"),
        "plain class must not offer configure: {labels:?}"
    );
}

#[test]
fn obj_method_completion_includes_a_literal_foreach_installed_method() {
    // The loop-installed names must be offered exactly
    // like an ordinary written method — completion, like hover /
    // definition / outline, reads the same `ClassDef::methods` table.
    let src = concat!(
        "oo::class create Widget {\n",
        "    foreach m {alpha beta gamma} {\n",
        "        method $m {args} { return $args }\n",
        "    }\n",
        "    method fetch {item} { return $item }\n",
        "}\n",
        "set d [Widget new]\n",
        "$d \n",
    );
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    // Cursor after `$d ` on line 7.
    let items = completions(
        src,
        7,
        3,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    for name in ["alpha", "beta", "gamma", "fetch"] {
        assert!(labels.contains(&name), "{name} missing: {labels:?}");
    }
}

#[test]
fn obj_method_completion_excludes_class_side_methods() {
    // `classmethod build` is callable on the *class* command, not on an
    // instance.  `$obj ` completion must offer the instance method
    // (`bark`) but not the class-side `build`.
    let src = "oo::class create Dog {\n    method bark {} {}\n    classmethod build {} {}\n}\nset d [Dog new]\n$d \n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    // Cursor after `$d ` on line 5.
    let items = completions(
        src,
        5,
        3,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"bark"),
        "instance method missing: {labels:?}"
    );
    assert!(
        !labels.contains(&"build"),
        "class-side method must not appear on an instance: {labels:?}",
    );
}

#[test]
fn obj_method_completion_survives_a_self_scoped_unexport_of_the_same_name() {
    // The visibility flip is sided: a class-object-side `unexport bark`
    // must not un-export the instance method of the same name, or
    // instance completion (which keeps only `visibility == "public"`)
    // drops `bark` entirely.
    // Oracle, byte-identical on tclsh 9.0.4 and 8.6.14:
    //   oo::class create C { method m {} {return inst-m}
    //                        self { method m {} {return class-m}
    //                               unexport m } }
    //   info class methods ::C  -> m        (instance side still exported)
    //   [::C new] m             -> inst-m   (still dispatches)
    let src = "oo::class create Dog {\n    method bark {} {}\n    self {\n        method bark {} {}\n        unexport bark\n    }\n}\nset d [Dog new]\n$d \n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    // Cursor after `$d ` on line 8.
    let items = completions(
        src,
        8,
        3,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"bark"),
        "a `self`-scoped unexport must not hide the instance method: {labels:?}",
    );
}

#[test]
fn class_command_completion_honours_a_self_scoped_unexport() {
    // The class-side half. Ignoring a `self`-scoped `unexport` leaves the
    // class command offering a member `::Counter m` rejects at runtime;
    // routing the flip to the class-object side makes it honour it.
    let hidden = "oo::class create Counter {\n    self {\n        method m {} {}\n        unexport m\n    }\n}\nCounter \n";
    let shown =
        "oo::class create Counter {\n    self {\n        method m {} {}\n    }\n}\nCounter \n";
    let registry = CommandRegistry::build_default();
    for (src, line, want) in [(hidden, 6, false), (shown, 5, true)] {
        let analysis = analyse(src);
        let items = completions(
            src,
            line,
            8,
            &analysis,
            Some(&registry),
            None,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        );
        let offered = items
            .iter()
            .any(|i| i.label == "m" && i.detail.as_deref().unwrap_or("").contains("::Counter"));
        assert_eq!(offered, want, "{src}");
    }
}

#[test]
fn class_command_completion_ignores_an_unwrapped_unexport() {
    // The mirror direction, and the false positive an un-sided setter
    // causes: an *unwrapped* `unexport` acts on the
    // instance side, so a class-object-side member of that name keeps its
    // exported state. Oracle: `oo::class create E2 { self { method
    // onlyclass {} {…} } }; oo::define E2 { unexport onlyclass }` leaves
    // `info object methods ::E2` -> onlyclass and `::E2 onlyclass` -> oc.
    let src = "oo::class create Counter {\n    self {\n        method onlyclass {} {}\n    }\n}\noo::define Counter {\n    unexport onlyclass\n}\nCounter \n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        8,
        8,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    assert!(
        items.iter().any(|i| i.label == "onlyclass"),
        "an instance-side unexport must not hide a class-side method: {:?}",
        items.iter().map(|i| &i.label).collect::<Vec<_>>(),
    );
}

#[test]
fn obj_method_completion_layers_per_object_members_and_flips() {
    // Per-object member state reaches completion: a
    // `oo::objdefine`d method is offered ahead of the class chain, a
    // per-object `unexport` masks a class-provided member (oracle:
    // `$d bark` → `unknown method "bark"`, tclsh 9.0.4 / 8.6.14), and a
    // per-object `export` revives one the name rule left unexported.
    let src = "oo::class create Dog {\n    method bark {} {}\n    method Fetch {} {}\n}\n\
                   set d [Dog new]\n\
                   oo::objdefine $d {\n    method roll {} {}\n    unexport bark\n    export Fetch\n}\n\
                   $d \n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    // Cursor after `$d ` on line 10.
    let items = completions(
        src,
        10,
        3,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"roll"),
        "the per-object member must be offered: {labels:?}",
    );
    assert!(
        !labels.contains(&"bark"),
        "a per-object unexport masks the class member: {labels:?}",
    );
    assert!(
        labels.contains(&"Fetch"),
        "a per-object export revives an unexported class member: {labels:?}",
    );
    let roll = items.iter().find(|i| i.label == "roll").unwrap();
    assert!(
        roll.detail.as_deref().unwrap_or("").contains("per-object"),
        "{:?}",
        roll.detail
    );
}

#[test]
fn obj_method_completion_leaves_sibling_objects_untouched() {
    // TN (CRITICAL FP guard) for the per-object layering: another instance of
    // the same class carries none of the first object's per-object state.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\n\
                   set d [Dog new]\n\
                   set e [Dog new]\n\
                   oo::objdefine $d {\n    method roll {} {}\n    unexport bark\n}\n\
                   $e \n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        9,
        3,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"bark"),
        "the sibling's class member survives: {labels:?}",
    );
    assert!(
        !labels.contains(&"roll"),
        "another object's per-object member must not be offered: {labels:?}",
    );
}

#[test]
fn obj_method_completion_omits_an_unwrapped_deleted_method() {
    // The same provider, destructive half. An unwrapped
    // `deletemethod` removes the instance method in real Tcl, so offering
    // it would complete a name the interpreter does not have.
    let src = "oo::class create Dog {\n    method bark {} {}\n    method sit {} {}\n    deletemethod bark\n}\nset d [Dog new]\n$d \n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        6,
        3,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"sit"), "sibling missing: {labels:?}");
    assert!(
        !labels.contains(&"bark"),
        "deleted method must not be offered: {labels:?}",
    );
}

#[test]
fn obj_method_completion_filters_by_partial() {
    let src = "oo::class create Dog {\n    method bark {} {}\n    method beg {} {}\n    method sit {} {}\n}\nset d [Dog new]\n$d b\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        6,
        4,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"bark") && labels.contains(&"beg"),
        "{labels:?}"
    );
    assert!(
        !labels.contains(&"sit"),
        "partial `b` should exclude sit: {labels:?}"
    );
}

/// A Tk widget's bareword instance path completes its own subcommands
/// (a self-referential registry `object_class`, not a user class), same
/// as `$var`-receiver completion above.
#[test]
fn widget_bareword_completion_offers_subcommands() {
    let src = "package require Tk\nttk::treeview .t\n.t i\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    // Cursor after `.t i` on line 2.
    let items = completions(
        src,
        2,
        4,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"instate") && labels.contains(&"identify") && labels.contains(&"insert"),
        "{labels:?}"
    );
    assert!(
        !labels.contains(&"curselection"),
        "must not offer an unrelated widget's subcommand: {labels:?}"
    );
}

#[test]
fn widget_method_completion_follows_the_document_tk_floor() {
    fn labels(source: &str, dialect: &'static str, line: u32) -> Vec<String> {
        let analysis = analyse_for(source, dialect);
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        completions(
            source,
            line,
            5,
            &analysis,
            Some(registry),
            None,
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        )
        .into_iter()
        .map(|item| item.label)
        .collect()
    }

    let plain = "package require Tk\nttk::treeview .t\n.t c\n";
    assert!(
        !labels(plain, "tcl9.0", 2)
            .iter()
            .any(|name| name == "current")
    );
    assert!(
        labels(plain, "tcl9.1", 2)
            .iter()
            .any(|name| name == "current")
    );

    let raised = "package require Tk 9.1\nttk::treeview .t\n.t c\n";
    assert!(
        labels(raised, "tcl9.0", 2)
            .iter()
            .any(|name| name == "current"),
        "an unconditional package require must raise the completion floor"
    );
}

/// Without Tk loaded the widget command does not exist (tclsh 8.6.18:
/// `invalid command name "ttk::treeview"`), so its path offers no methods.
#[test]
fn widget_methods_need_the_widget_package() {
    let src = "ttk::treeview .t\n.t i\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        1,
        4,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(
        !items.iter().any(|item| item.label == "instate"),
        "{:?}",
        items.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
}

/// The `set lb [listbox .l]` return-value-capture shape completes too.
#[test]
fn widget_var_captured_completion_offers_subcommands() {
    let src = "package require Tk\nset lb [listbox .l]\n$lb cu\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        2,
        6,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"curselection"), "{labels:?}");
}

/// A bareword that merely shares a name with an unrelated tracked
/// variable must not offer method completion — only a name genuinely
/// bound by a create call qualifies (mirrors
/// `logical_receiver_candidates_keep_instance_and_class_buckets_together` in
/// definition.rs).
#[test]
fn bareword_completion_does_not_leak_unrelated_variable_class() {
    let src = "oo::class create Bar {\n    method get {} {}\n}\nset b [Bar new]\nb \n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        4,
        2,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.contains(&"get"),
        "bareword `b` was never created — must not complete as `Bar`: {labels:?}"
    );
}

#[test]
fn fuzzy_method_surfaces_when_response_would_be_empty() {
    // `$d brk` — no method prefix-matches and no command comes
    // within the edit budget, so the response-level fallback offers
    // the instance's `bark`.
    let src = "oo::class create Dog {\n    method bark {} {}\n    method beg {} {}\n}\nset d [Dog new]\n$d brk\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        5,
        6,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let bark = items
        .iter()
        .find(|i| i.label == "bark")
        .unwrap_or_else(|| panic!("expected fuzzy bark, got {items:?}"));
    assert!(
        bark.detail.as_deref().unwrap_or("").starts_with("method"),
        "method item expected, got {:?}",
        bark.detail,
    );
}

#[test]
fn method_typo_does_not_hijack_command_prefix_matches() {
    // `$d xy` — no method prefix-matches, but the proc `xyz` does;
    // the fall-through command/proc list must stay exactly as
    // before (no fuzzy methods creep in alongside it).
    let src =
        "oo::class create Dog {\n    method xx {} {}\n}\nproc xyz {} {}\nset d [Dog new]\n$d xy\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        5,
        5,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"xyz"), "prefix proc expected: {labels:?}");
    assert!(
        !labels.contains(&"xx"),
        "fuzzy method must not pad a prefix-matching response: {labels:?}",
    );
    assert!(items.iter().all(|i| i.filter_text.is_none()));
}

/// Per-object member masks reach method completion on the object.
/// tclsh 8.6.18/9.1.0: `$o prior` runs the object's own method, and after
/// `unexport "nul\0mask"` the class's `run` is still callable (RUN).
#[test]
fn object_member_completion_keeps_own_methods_and_ignores_a_nul_mask() {
    let labels = |src: &str| {
        let analysis = analyse_for(src, "tcl");
        completions(
            src,
            1,
            4,
            &analysis,
            None,
            None,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        )
        .into_iter()
        .map(|item| item.label)
        .collect::<Vec<_>>()
    };
    assert_eq!(
        labels("oo::class create C {}; set o [C new]; oo::objdefine $o {method prior {} {}}\n$o p"),
        ["prior"]
    );
    assert_eq!(
        labels(
            "oo::class create C {method run {} {}}; set o [C new]; oo::objdefine $o {unexport {nul\0mask}}\n$o r"
        ),
        ["run"]
    );
}
