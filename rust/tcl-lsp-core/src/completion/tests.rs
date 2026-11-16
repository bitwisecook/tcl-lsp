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

use super::*;
use tcl_compiler::analyser::Analyser;
use tcl_dialect::model::{Family, SurfaceLayer};

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

fn analyse_for(source: &str, environment: &str) -> AnalysisResult {
    let mut analyser = Analyser::new();
    let input = analyser
        .prepare_analysis_input(environment)
        .expect("test environment has a complete analysis input");
    analyser = analyser.with_resolved_input(input);
    analyser.analyse(source, environment).clone()
}

/// [`analyse`] under Tcl 9.0 — for sources using 9.0-only class
/// machinery (`oo::configurable`, `property`), which the 8.6 analysis
/// the rest of this module uses does not model.
fn analyse9(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl9.0").clone()
}

#[test]
fn array_element_completion_does_not_eat_trailing_text_when_unclosed() {
    // `$arr(k more stuff` with no `)` — accepting `$arr(key)` must replace
    // only up to the cursor, not to end-of-line, so ` more stuff` survives
    // (issue 181).
    let src = "set arr(key) 1\nset v $arr(k more stuff\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    // Line 1, cursor just after `k`: `set v $arr(k` → char col 12.
    let items = completions(
        src,
        1,
        12,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let key = items
        .iter()
        .find(|i| i.label == "$arr(key)")
        .expect("array-element completion offered");
    let edit = key.text_edit.as_ref().expect("has a replace edit");
    assert_eq!(
        edit.end_char, 12,
        "replace range must stop at the cursor, not run to EOL"
    );
}

#[test]
fn array_element_completion_swallows_existing_close_paren() {
    // With a closing `)`, the replace range extends through it so the old
    // index is fully replaced.
    let src = "set arr(key) 1\nset v $arr(k)\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        1,
        12,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let key = items
        .iter()
        .find(|i| i.label == "$arr(key)")
        .expect("array-element completion offered");
    let edit = key.text_edit.as_ref().expect("has a replace edit");
    assert_eq!(edit.end_char, 13, "replace range must cover the `)`");
}

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
    let src = "ttk::treeview .t\n.t i\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    // Cursor after `.t i` on line 1.
    let items = completions(
        src,
        1,
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

    let plain = "ttk::treeview .t\n.t c\n";
    assert!(
        !labels(plain, "tcl9.0", 1)
            .iter()
            .any(|name| name == "current")
    );
    assert!(
        labels(plain, "tcl9.1", 1)
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

/// The `set lb [listbox .l]` return-value-capture shape completes too.
#[test]
fn widget_var_captured_completion_offers_subcommands() {
    let src = "set lb [listbox .l]\n$lb cu\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        1,
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
fn dollar_trigger_at_eol_returns_partial() {
    let (trigger, partial) = variable_trigger("set x $", 0, 7).unwrap();
    assert_eq!(trigger, '$');
    assert_eq!(partial, "");
}

#[test]
fn dollar_trigger_with_partial_name() {
    let (trigger, partial) = variable_trigger("set y $foo", 0, 10).unwrap();
    assert_eq!(trigger, '$');
    assert_eq!(partial, "foo");
}

#[test]
fn brace_trigger_recognised() {
    let (trigger, partial) = variable_trigger("set y ${ba", 0, 10).unwrap();
    assert_eq!(trigger, '{');
    assert_eq!(partial, "ba");
}

#[test]
fn space_resets_trigger_context() {
    // The `$` is followed by a space + new identifier — no
    // variable trigger active at the cursor.
    assert!(variable_trigger("set $ ab", 0, 8).is_none());
}

#[test]
fn variable_completion_lists_globals_alphabetically() {
    let src = "set apple 1\nset banana 2\nset $\n";
    let analysis = analyse(src);
    // Cursor after `$` on the third line.
    let items = completions(
        src,
        2,
        5,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(!items.is_empty(), "expected variable completions");
    assert_eq!(items[0].kind, CompletionKind::Variable);
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    // Alphabetical order.
    let mut sorted = labels.clone();
    sorted.sort_unstable();
    assert_eq!(labels, sorted);
    assert!(labels.contains(&"$apple"));
    assert!(labels.contains(&"$banana"));
}

#[test]
fn variable_completion_filters_by_partial() {
    let src = "set apple 1\nset banana 2\nset $b\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        2,
        6,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["$banana"]);
}

#[test]
fn proc_completion_lists_user_defined_procs() {
    let src = "proc greet {} {}\nproc shout {} {}\ng\n";
    let analysis = analyse(src);
    // Cursor right after `g` on third line.
    let items = completions(
        src,
        2,
        1,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert_eq!(items.len(), 1, "{items:?}");
    assert_eq!(items[0].kind, CompletionKind::Function);
    assert_eq!(items[0].label, "greet");
}

// usage-bucket sort-text

#[test]
fn usage_bucket_thresholds() {
    // Exhaustive bucket-table check across every threshold.
    assert_eq!(usage_bucket(0), 5);
    assert_eq!(usage_bucket(1), 4);
    assert_eq!(usage_bucket(2), 4);
    assert_eq!(usage_bucket(3), 3);
    assert_eq!(usage_bucket(7), 3);
    assert_eq!(usage_bucket(8), 2);
    assert_eq!(usage_bucket(19), 2);
    assert_eq!(usage_bucket(20), 1);
    assert_eq!(usage_bucket(49), 1);
    assert_eq!(usage_bucket(50), 0);
    assert_eq!(usage_bucket(1000), 0);
}

#[test]
fn proc_sort_text_has_lower_bucket_for_used_procs() {
    // Three calls to `greet` and zero calls to `shout`.
    // The completion-list partial is empty (cursor on
    // line 6 col 0), so both procs surface — `greet` in
    // bucket 3 (≥3 calls) and `shout` in bucket 5 (no
    // calls).
    let src = "proc greet {} {}\nproc shout {} {}\ngreet\ngreet\ngreet\n\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        5,
        0,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let greet = items
        .iter()
        .find(|i| i.label == "greet")
        .unwrap_or_else(|| panic!("greet missing from {items:?}"));
    let greet_sort = greet.sort_text.as_deref().unwrap_or("");
    assert!(
        greet_sort.starts_with("A003_"),
        "greet sort_text {greet_sort:?} should land in bucket 3",
    );
    let shout = items
        .iter()
        .find(|i| i.label == "shout")
        .unwrap_or_else(|| panic!("shout missing from {items:?}"));
    let shout_sort = shout.sort_text.as_deref().unwrap_or("");
    assert!(
        shout_sort.starts_with("A005_"),
        "shout sort_text {shout_sort:?} should land in bucket 5",
    );
    // Lower bucket sorts first.
    assert!(greet_sort < shout_sort);
}

#[test]
fn empty_partial_lists_all_procs() {
    let src = "proc alpha {} {}\nproc beta {} {}\n\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        2,
        0,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"alpha"));
    assert!(labels.contains(&"beta"));
}

// built-in command completion
//
// Tests pin the contract that a non-`None` registry parameter
// extends proc-name completion with every command the
// registry knows about, filtered by the partial typed at the
// cursor.

#[test]
fn builtin_completion_lists_registry_commands_at_command_position() {
    // No user-defined procs, partial `pu` → `puts` (and any
    // other registered command beginning with `pu`) should
    // surface.
    let src = "pu\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        2,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"puts"),
        "expected `puts` from registry; got {labels:?}",
    );
}

#[test]
fn builtin_completion_detail_shows_provenance() {
    // A core built-in shows `built-in`; a stdlib command shows its
    // `stdlib (PKG)` provenance from the registry `required_package`.
    let src = "pu\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        2,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let puts = items.iter().find(|i| i.label == "puts").expect("puts");
    assert_eq!(puts.detail.as_deref(), Some("built-in"), "{puts:?}");

    // `http::geturl` is a stdlib command requiring the `http` package.
    let src = "http::ge\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        0,
        8,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    if let Some(geturl) = items.iter().find(|i| i.label == "http::geturl") {
        assert_eq!(
            geturl.detail.as_deref(),
            Some("stdlib (http)"),
            "{geturl:?}"
        );
    }
}

#[test]
fn command_detail_formats_each_provenance() {
    let reg = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    // built-in: no package.
    assert_eq!(command_detail(reg.get("puts").unwrap(), reg), "built-in");
    // stdlib: required_package set.
    if let Some(spec) = reg.get("http::geturl") {
        assert_eq!(command_detail(spec, reg), "stdlib (http)");
    }
    // An ambient vendor surface reads as part of the runtime, never as
    // a require-gated stdlib package (§7.1 axis C). The registry is what
    // is asked, so a pack's `ambient_package` row would read the same way.
    let ireg = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let http2 = ireg.get("HTTP2::header").expect("HTTP2::header spec");
    assert_eq!(command_detail(http2, ireg), "built-in");
}

#[test]
fn irules_subcommand_completion_uses_configured_bigip_version() {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let profile = tcl_dialect::DialectProfile::irules();
    let c3d = registry.get("SSL::c3d").expect("SSL::c3d spec");

    let mut analysis = AnalysisResult::default();
    analysis.library_versions.bigip_version = Some("21.0.0".to_owned());
    let labels_21_0: Vec<String> = subcommand_completions(c3d, &analysis, profile, "")
        .into_iter()
        .map(|item| item.label)
        .collect();
    assert!(!labels_21_0.iter().any(|label| label == "cert_lifespan"));
    assert!(!labels_21_0.iter().any(|label| label == "cert_start_date"));

    analysis.library_versions.bigip_version = Some("21.1.0".to_owned());
    let labels_21_1: Vec<String> = subcommand_completions(c3d, &analysis, profile, "")
        .into_iter()
        .map(|item| item.label)
        .collect();
    assert!(labels_21_1.iter().any(|label| label == "cert_lifespan"));
    assert!(labels_21_1.iter().any(|label| label == "cert_start_date"));
}

#[test]
fn builtin_completion_filters_by_partial() {
    // Partial `whi` should yield `while` but not unrelated
    // commands like `puts`.
    let src = "whi\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        3,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"while"),
        "expected `while`; got {labels:?}"
    );
    assert!(
        !labels.contains(&"puts"),
        "should NOT contain `puts` for partial `whi`; got {labels:?}",
    );
}

#[test]
fn builtin_completion_skips_math_operators() {
    // The registry registers `+` / `-` / `*` / etc. as
    // commands (Tcl 9 `tcl::mathop`), but they don't make
    // sense as completion items at a command position.
    let src = "+\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        1,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    for op in &["+", "-", "*", "/", ">", ">=", "<", "<=", "==", "!="] {
        assert!(
            !labels.contains(op),
            "math operator `{op}` should be filtered out; got {labels:?}",
        );
    }
}

/// `SKIP_BUILTIN_NAMES` must cover the word-form mathop commands
/// (`eq`/`ne`/`in`/`ni`, and the 9.0+ `lt`/`le`/`gt`/`ge`) as well as the
/// 10 symbolic operators; otherwise they leak through as a nonsensical
/// "did you mean `eq`?" at a command position.
#[test]
fn builtin_completion_skips_word_form_operators() {
    let src = "\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        0,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    for op in &["eq", "ne", "in", "ni", "lt", "le", "gt", "ge"] {
        assert!(
            !labels.contains(op),
            "word-form operator `{op}` should be filtered out; got {labels:?}",
        );
    }
}

#[test]
fn builtin_completion_none_registry_keeps_minimal_behaviour() {
    // Passing `None` for the registry must not regress the
    // minimal port's behaviour — proc-name completion alone.
    let src = "proc helper {} {}\nhe\n";
    let analysis = analyse(src);
    let items_no_registry = completions(
        src,
        1,
        2,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels_no_registry: Vec<&str> =
        items_no_registry.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels_no_registry.contains(&"helper"),
        "expected `helper` proc; got {labels_no_registry:?}",
    );
    // No registry commands surface without the registry.
    assert_eq!(items_no_registry.len(), 1, "{items_no_registry:?}");
}

#[test]
fn builtin_completion_merges_procs_and_registry() {
    // Both a user-defined proc and built-in commands should
    // surface when both apply.
    let src = "proc parade {} {}\npar\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        1,
        3,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"parade"),
        "expected user proc `parade`; got {labels:?}",
    );
    // `parray` is a stdlib command starting with `par`.
    assert!(
        labels.contains(&"parray"),
        "expected built-in `parray`; got {labels:?}",
    );
}

#[test]
fn builtin_completion_skipped_inside_variable_trigger() {
    // Variable trigger (`$par`) must take precedence and
    // suppress built-in command completions even when a
    // registry is supplied, via the `variable_completions`
    // short-circuit at the top of `completions`.
    let src = "set apple 1\nset $par\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        1,
        8,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    // Only variable completions allowed here.
    for it in &items {
        assert_eq!(
            it.kind,
            CompletionKind::Variable,
            "variable trigger should suppress built-ins; got {it:?}",
        );
    }
}

// subcommand completion
//
// When the cursor sits at word-index 1 of a known command
// whose spec declares non-empty `subcommands`, the
// completion surface lists subcommand names (not user procs
// or unrelated built-ins).

#[test]
fn subcommand_completion_surfaces_subcommands_at_word_index_1() {
    // `string l` — partial `l`, cursor at word-index 1 of
    // `string`.  The registry declares many `string`
    // subcommands; expect `length` and similar `l*` ones.
    let src = "string l\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        8,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"length"),
        "expected `length` subcommand; got {labels:?}",
    );
    // No proc / unrelated commands should leak through.
    assert!(
        !labels.contains(&"puts"),
        "subcommand context should not include `puts`; got {labels:?}",
    );
}

#[test]
fn sub_subcommand_completion_surfaces_at_word_index_2() {
    // After `info object ` (word-index 2), offer the
    // OBJECT INTROSPECTION operations with their descriptions.
    let src = "info object \n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        12,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"class"),
        "expected `class`; got {labels:?}"
    );
    assert!(
        labels.contains(&"methods"),
        "expected `methods`; got {labels:?}"
    );
    // Descriptions are surfaced as detail.
    assert!(
        items
            .iter()
            .find(|i| i.label == "class")
            .and_then(|i| i.detail.as_deref())
            .is_some_and(|d| !d.is_empty()),
        "expected a detail on the `class` op",
    );
    // Partial filters: `info class super` → `superclasses`.
    let src = "info class super\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        0,
        16,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(
        labels,
        vec!["superclasses"],
        "prefix should filter; got {labels:?}"
    );
}

#[test]
fn subcommand_completion_lists_all_subcommands_with_empty_partial() {
    // `string ` (cursor just past the space) — empty partial,
    // word-index 1 of `string`.  Should list every `string`
    // subcommand, alphabetically.
    let src = "string \n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        7,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    // `string` has at minimum `length`, `match`, `range`,
    // `tolower`, `toupper` across all dialects.
    assert!(labels.contains(&"length"), "{labels:?}");
    assert!(labels.contains(&"match"), "{labels:?}");
    // Should be sorted.
    let mut sorted = labels.clone();
    sorted.sort_unstable();
    assert_eq!(labels, sorted, "expected sorted subcommands");
}

#[test]
fn subcommand_completion_falls_through_when_word_index_not_1() {
    // `string length f` — cursor at word-index 2, the string operand.
    // Subcommand completion must not fire, and a data operand offers no
    // command names either.
    let src = "string length f\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        15,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.iter().any(|l| l.starts_with('f')),
        "a data operand offers no command names; got {labels:?}",
    );
    // String's `length` subcommand should NOT be in the
    // result — we're past word-index 1.
    assert!(
        !labels.contains(&"length"),
        "should not surface `length` subcommand at word-index 2; got {labels:?}",
    );
}

#[test]
fn subcommand_completion_skipped_when_command_has_no_subcommands() {
    // `puts hel` — `puts` declares no subcommands, and `hel` is the
    // string it writes: data, where no procedure name is offered.
    let src = "proc helper {} {}\nputs hel\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        1,
        8,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.contains(&"helper"),
        "a data operand offers no procedure names; got {labels:?}",
    );
}

// switch completion
//
// When the partial starts with `-` and the surrounding
// command's spec declares matching options, the completion
// surface lists option names (not subcommands or built-ins).

#[test]
fn switch_completion_surfaces_options_for_dash_partial() {
    // `lsearch -n` — partial `-n`.  The `lsearch` spec
    // declares many `-...` options; expect at least one
    // starting with `-n` (`-nocase`).
    let src = "lsearch -n\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        10,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.iter().any(|l| l.starts_with("-n")),
        "expected at least one `-n*` switch; got {labels:?}",
    );
    // No unrelated commands / procs.
    assert!(
        !labels.iter().any(|l| !l.starts_with('-')),
        "all switch-completion items must start with '-'; got {labels:?}",
    );
}

#[test]
fn switch_completion_lists_all_options_for_bare_dash() {
    // `lsearch -` — partial `-`, every option should
    // surface.
    let src = "lsearch -\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        9,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    // Every result must start with `-`.
    for l in &labels {
        assert!(l.starts_with('-'), "non-switch in result: {l}");
    }
    // Sorted.
    let mut sorted = labels.clone();
    sorted.sort_unstable();
    assert_eq!(labels, sorted);
}

#[test]
fn switch_completion_gates_options_by_package_version() {
    // `entry -placeholder` was introduced in Tk 8.7.  Completion must gate
    // it on the version resolved from `package require Tk <req>`.
    let registry = CommandRegistry::build_default();
    let older = "package require Tk 8.6\nentry .e -p\n";
    let a1 = analyse(older);
    let l1: Vec<String> = completions(
        older,
        1,
        11,
        &a1,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        !l1.iter().any(|l| l == "-placeholder"),
        "Tk 8.6 must not offer -placeholder: {l1:?}",
    );

    let newer = "package require Tk 8.7\nentry .e -p\n";
    let a2 = analyse(newer);
    let l2: Vec<String> = completions(
        newer,
        1,
        11,
        &a2,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        l2.iter().any(|l| l == "-placeholder"),
        "Tk 8.7 must offer -placeholder: {l2:?}",
    );
}

#[test]
fn switch_completion_gates_options_by_dialect() {
    // `lsearch -stride` is Tcl 9.0+ (see the W004 diagnostic for the same
    // gate).  Completion must not suggest it under an older dialect, and
    // must once the document's dialect supports it.
    let registry = CommandRegistry::build_default();
    let src = "lsearch -s {a b} x\n";
    let old_analysis = analyse_for(src, "tcl8.6");
    let old: Vec<String> = completions(
        src,
        0,
        10,
        &old_analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        !old.iter().any(|l| l == "-stride"),
        "tcl8.6 must not offer -stride: {old:?}",
    );
    let new_analysis = analyse_for(src, "tcl9.0");
    let new: Vec<String> = completions(
        src,
        0,
        10,
        &new_analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        new.iter().any(|l| l == "-stride"),
        "tcl9.0 must offer -stride: {new:?}",
    );
}

#[test]
fn switch_completion_resolves_subcommand_scoped_options() {
    // `chan configure -inputmode` is a subcommand-scoped, Tcl 9.0+
    // option that lives only on the `configure` SubCommand's own table —
    // absent from `chan`'s top-level option table entirely.  Completion
    // must resolve the typed subcommand to find it, and still gate it by
    // dialect exactly like W004 does.
    let registry = CommandRegistry::build_default();
    let src = "chan configure $chan -i\n";
    let old_analysis = analyse_for(src, "tcl8.6");
    let old: Vec<String> = completions(
        src,
        0,
        23,
        &old_analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        !old.iter().any(|l| l == "-inputmode"),
        "tcl8.6 must not offer -inputmode: {old:?}",
    );
    let new_analysis = analyse_for(src, "tcl9.0");
    let new: Vec<String> = completions(
        src,
        0,
        23,
        &new_analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        new.iter().any(|l| l == "-inputmode"),
        "tcl9.0 must offer -inputmode: {new:?}",
    );
}

#[test]
fn command_completion_gates_commands_by_package_version() {
    // `ttk::button` needs Tk 8.5. On a tcl8.4 host the shipped Tk is
    // 8.4 (the §7.1 TracksBase pin) and a require cannot raise it —
    // not offered. On a tcl8.6 host the shipped Tk is 8.6 even when
    // the file writes `package require Tk 8.4` (a minimum, not a
    // downgrade) — offered; the old require-only floor wrongly hid it.
    let registry = CommandRegistry::build_default();
    let older = "package require Tk 8.4\nttk::b\n";
    let a1 = analyse_for(older, "tcl8.4");
    let l1: Vec<String> = completions(
        older,
        1,
        6,
        &a1,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.4").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        !l1.iter().any(|l| l == "ttk::button"),
        "a tcl8.4 host (Tk 8.4) must not offer ttk::button: {l1:?}",
    );

    let a1_86 = analyse_for(older, "tcl8.6");
    let l1_86: Vec<String> = completions(
        older,
        1,
        6,
        &a1_86,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        l1_86.iter().any(|l| l == "ttk::button"),
        "a tcl8.6 host ships Tk 8.6; `require Tk 8.4` does not downgrade it: {l1_86:?}",
    );

    let newer = "package require Tk 8.5\nttk::b\n";
    let a2 = analyse_for(newer, "tcl8.6");
    let l2: Vec<String> = completions(
        newer,
        1,
        6,
        &a2,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    )
    .into_iter()
    .map(|i| i.label)
    .collect();
    assert!(
        l2.iter().any(|l| l == "ttk::button"),
        "Tk 8.5 must offer ttk::button: {l2:?}",
    );
}

// iRules event-name completion
//
// When the cursor sits at word-index 1 of an event-handler
// command (the `when` iRules keyword carries
// `Traits::IS_EVENT_HANDLER`), the completion surface lists
// event names from the shared `EventRegistry`.

fn irules_registry() -> CommandRegistry {
    let mut r = CommandRegistry::build_default();
    r.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    r
}

#[test]
fn event_completion_surfaces_when_handler_first_arg() {
    // `when HT` inside iRules — should list every event
    // starting with `HT` (HTTP_REQUEST, HTTP_RESPONSE, …).
    let src = "when HT\n";
    let analysis = analyse_for(src, "f5-irules");
    let registry = irules_registry();
    let items = completions(
        src,
        0,
        7,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("f5-irules").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"HTTP_REQUEST"),
        "expected `HTTP_REQUEST`; got {labels:?}",
    );
    assert!(
        labels.iter().all(|l| l.starts_with("HT")),
        "all results should start with `HT`; got {labels:?}",
    );
}

#[test]
fn event_completion_marks_detail_as_irules_event() {
    let src = "when CL\n";
    let analysis = analyse(src);
    let registry = irules_registry();
    let items = completions(
        src,
        0,
        7,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(
        items
            .iter()
            .all(|i| i.detail.as_deref() == Some("F5 iRules event")),
        "every event completion should be labelled as an iRules event; got {items:?}",
    );
}

#[test]
fn event_completion_does_not_fire_in_plain_tcl_dialect() {
    // The default registry doesn't carry the `when` spec —
    // completion should fall through to plain command + proc
    // completion (which won't surface `HTTP_REQUEST`).
    let src = "when HT\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        7,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.contains(&"HTTP_REQUEST"),
        "event completion should not fire outside iRules; got {labels:?}",
    );
}

#[test]
fn event_completion_skipped_at_word_index_other_than_1() {
    // `when HTTP_REQUEST f` — word-index 2 (after the event
    // name).  Event completion must not fire.
    let src = "when HTTP_REQUEST f\n";
    let analysis = analyse(src);
    let registry = irules_registry();
    let items = completions(
        src,
        0,
        19,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.iter().any(|l| l.contains("HTTP_REQUEST")),
        "event completion should not fire at word-index 2; got {labels:?}",
    );
}

// iRules `call PROC_NAME`
//
// When the cursor sits at word-index 1 of a command carrying
// `Traits::INVOKES_USER_PROC` (today only the iRules `call`
// command), the completion surface lists user-defined proc
// names and excludes built-in commands.

#[test]
fn call_completion_surfaces_user_procs_at_word_index_1() {
    // Two user procs starting with `he`, plus one that
    // doesn't.  Built-in `puts` starts with `p` so won't
    // surface against `he` either way — but we use it as
    // additional cover.
    let src = "proc helper {} {}\nproc help_inner {} {}\nproc unrelated {} {}\ncall he\n";
    let analysis = analyse_for(src, "f5-irules");
    let registry = irules_registry();
    let items = completions(
        src,
        3,
        7,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"helper") && labels.contains(&"help_inner"),
        "expected user procs `helper` and `help_inner`; got {labels:?}",
    );
    assert!(
        !labels.contains(&"unrelated"),
        "should filter on partial `he`; got {labels:?}",
    );
    // Every result must be a user proc — built-in commands
    // are excluded from the `call` context.
    for it in &items {
        assert_eq!(
            it.kind,
            CompletionKind::Function,
            "every call-completion item should be a Function; got {it:?}",
        );
    }
}

#[test]
fn call_completion_does_not_fire_in_plain_tcl_dialect() {
    // Plain Tcl registry has no `call` spec, so `help` is an argument of
    // an unknown command: data, where no command name is offered.
    let src = "proc helper {} {}\ncall help\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        1,
        9,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.contains(&"helper"),
        "the iRules `call` completion must not fire; got {labels:?}",
    );
}

#[test]
fn call_completion_excludes_builtin_commands() {
    // Built-in `puts` starts with `p`.  In a `call p`
    // context, the completion list should contain user
    // procs starting with `p` but never `puts`.
    let src = "proc parade {} {}\ncall p\n";
    let analysis = analyse_for(src, "f5-irules");
    let registry = irules_registry();
    let items = completions(
        src,
        1,
        6,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"parade"), "{labels:?}");
    assert!(
        !labels.contains(&"puts"),
        "`puts` is a built-in; should not surface in `call` context: {labels:?}",
    );
}

#[test]
fn tk_commands_only_offered_when_tk_is_loaded() {
    let registry = CommandRegistry::build_default();

    // Plain `.tcl` (tcl8.6) with no `package require Tk`: Tk widget
    // commands must NOT be offered, even though the registry knows them.
    let src = "butt\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        0,
        4,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.contains(&"button"),
        "Tk `button` must not surface without `package require Tk`: {labels:?}",
    );

    // Once `package require Tk` is declared, `button` becomes available.
    let src = "package require Tk\nbutt\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        1,
        4,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"button"),
        "Tk `button` should surface after `package require Tk`: {labels:?}",
    );
}

#[test]
fn tk_commands_offered_in_wish_tk_dialect() {
    // A `wish`-labelled document (`tk` dialect) is implicitly Tk-loaded.
    let registry = CommandRegistry::build_default();
    let src = "butt\n";
    let analysis = analyse_for(src, "tk");
    let items = completions(
        src,
        0,
        4,
        &analysis,
        Some(&registry),
        None,
        crate::profile_for_dialect("tk"),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"button"),
        "Tk `button` should surface in the `tk` dialect: {labels:?}",
    );
}

/// The completion gate reads two placement queries: Tk names are offered where
/// Tk is in the world (ambient, or required here) **and** the
/// environment declares a Tk placement at all. The `wish` alias must
/// answer identically to `tk` — it is the same environment row — and a
/// vendor shell must refuse even with an explicit require.
#[test]
fn tk_completion_follows_the_placement_not_the_environment_name() {
    let registry = CommandRegistry::build_default();
    let offers = |src: &str, line: u32, character: u32, dialect: &str| {
        let analysis = analyse_for(src, dialect);
        completions(
            src,
            line,
            character,
            &analysis,
            Some(&registry),
            None,
            crate::profile_for_dialect(dialect),
        )
        .iter()
        .any(|item| item.label == "button")
    };
    // Ambient: `wish` under either spelling of its identity.
    assert!(offers("butt\n", 0, 4, "tk"), "tk");
    assert!(offers("butt\n", 0, 4, "wish"), "wish");
    // Hosted: offered only once this document requires it.
    assert!(!offers("butt\n", 0, 4, "tcl8.6"), "tcl8.6 bare");
    assert!(!offers("butt\n", 0, 4, "tcl"), "lenient sink bare");
    assert!(
        offers("package require Tk\nbutt\n", 1, 4, "tcl8.6"),
        "tcl8.6 with a require"
    );
    // Unhostable: a vendor shell declares no Tk placement, so a stray
    // require changes nothing.
    for shell in ["f5-iapps", "xilinx-eda-tcl", "bpf"] {
        assert!(
            !offers("package require Tk\nbutt\n", 1, 4, shell),
            "{shell}"
        );
    }
}

#[test]
fn tk_commands_never_offered_in_irules() {
    // Even a stray `package require Tk` cannot make Tk valid in iRules —
    // the command is dialect-gated out.
    let registry = CommandRegistry::build_default();
    let src = "package require Tk\nbutt\n";
    let analysis = analyse_for(src, "f5-irules");
    let items = completions(
        src,
        1,
        4,
        &analysis,
        Some(&registry),
        None,
        tcl_dialect::DialectProfile::irules(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.contains(&"button"),
        "Tk `button` must never surface in iRules: {labels:?}",
    );
}

#[test]
fn call_completion_skipped_at_word_index_other_than_1() {
    // `call helper extra` — word-index 2 is an argument of the called
    // procedure: data, where neither procedures nor built-ins are offered.
    let src = "proc helper {} {}\ncall helper e\n";
    let analysis = analyse_for(src, "f5-irules");
    let registry = irules_registry();
    let items = completions(
        src,
        1,
        14,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.is_empty(),
        "no command name belongs at word-index 2; got {labels:?}",
    );
}

// -- subcommand argument-value completion ----

#[test]
fn string_is_completes_character_classes() {
    // `string is a` — cursor at the class arg (word 2);
    // expect the `a*` classes (alnum, alpha, ascii).
    let src = "string is a\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        11,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"alnum"), "{labels:?}");
    assert!(labels.contains(&"alpha"), "{labels:?}");
    assert!(labels.contains(&"ascii"), "{labels:?}");
    // Every result is an enum value, not a proc / command.
    for it in &items {
        assert_eq!(it.kind, CompletionKind::EnumValue, "{it:?}");
    }
    // Non-`a` classes filtered out.
    assert!(!labels.contains(&"digit"), "{labels:?}");
}

#[test]
fn string_is_lists_all_classes_with_empty_partial() {
    // `string is ` — cursor just past the space, empty
    // partial → all 22 character classes.
    let src = "string is \n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        10,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"boolean"), "{labels:?}");
    assert!(labels.contains(&"wordchar"), "{labels:?}");
    assert!(
        labels.len() >= 20,
        "expected the full class set; got {labels:?}"
    );
    // Sorted.
    let mut sorted = labels.clone();
    sorted.sort_unstable();
    assert_eq!(labels, sorted);
}

#[test]
fn string_is_class_detail_is_surfaced() {
    let src = "string is alnum\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        15,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let alnum = items.iter().find(|i| i.label == "alnum").expect("alnum");
    assert!(
        alnum
            .detail
            .as_deref()
            .unwrap_or("")
            .contains("alphabet or digit"),
        "{alnum:?}",
    );
}

#[test]
fn subcommand_without_arg_values_falls_through() {
    // `string length x` — `length` has no arg_values, so
    // word-index 2 falls through to plain completion (no
    // enum-value items).
    let src = "string length x\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        15,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(
        items.iter().all(|i| i.kind != CompletionKind::EnumValue),
        "no enum-value completions expected; got {items:?}",
    );
}

// workspace-index: cross-document proc completion

#[test]
fn workspace_procs_surface_in_completion() {
    use crate::workspace_index::WorkspaceIndex;
    // Current doc defines `local_proc`; a sibling doc
    // defines `shared_helper`.  Completing `s` should
    // surface the cross-document proc.
    let cur_src = "proc local_proc {} {}\ns\n";
    let cur = analyse(cur_src);
    let other = analyse("proc shared_helper {} {}\n");
    let index =
        WorkspaceIndex::from_documents([("file:///cur.tcl", &cur), ("file:///other.tcl", &other)]);
    let items = completions(
        cur_src,
        1,
        1,
        &cur,
        None,
        Some(&index),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"shared_helper"),
        "expected cross-doc proc; got {labels:?}",
    );
    // It's tagged as a workspace proc in the detail.
    let helper = items.iter().find(|i| i.label == "shared_helper").unwrap();
    assert!(
        helper.detail.as_deref().unwrap_or("").contains("workspace"),
        "{helper:?}",
    );
    assert!(helper.sort_text.as_deref().unwrap_or("").starts_with("C0_"));
}

#[test]
fn workspace_procs_do_not_duplicate_local_procs() {
    use crate::workspace_index::WorkspaceIndex;
    // Both the current doc and the index contain `greet`.
    // The result must list `greet` once (the local entry).
    let cur_src = "proc greet {} {}\ngr\n";
    let cur = analyse(cur_src);
    let index = WorkspaceIndex::from_documents([("file:///cur.tcl", &cur)]);
    let items = completions(
        cur_src,
        1,
        2,
        &cur,
        None,
        Some(&index),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let count = items.iter().filter(|i| i.label == "greet").count();
    assert_eq!(count, 1, "{items:?}");
}

#[test]
fn option_enum_value_completion_offers_members() {
    // `button .b -relief ra` — cursor on the value word after a closed-set
    // option offers the relief members, filtered by the partial.
    let src = "button .b -relief ra";
    let cur = analyse(src);
    let registry = CommandRegistry::build_default();
    let col = u32::try_from(src.len()).unwrap();
    let items = completions(
        src,
        0,
        col,
        &cur,
        Some(&registry),
        None,
        crate::profile_for_dialect("tk"),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"raised"),
        "expected `raised` among relief completions; got {labels:?}"
    );
}

#[test]
fn workspace_none_keeps_single_doc_behaviour() {
    let cur_src = "proc greet {} {}\ngr\n";
    let cur = analyse(cur_src);
    let items = completions(
        cur_src,
        1,
        2,
        &cur,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["greet"], "{labels:?}");
}

/// iRule event snippet labels surfaced at a cursor.
fn irule_snippet_labels(
    src: &str,
    line: u32,
    character: u32,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<String> {
    let analysis = tcl_compiler::analyser::Analyser::new().analyse(src, dialect.name);
    let registry = irules_registry();
    completions(
        src,
        line,
        character,
        &analysis,
        Some(&registry),
        None,
        dialect,
    )
    .into_iter()
    .filter(|i| i.label.starts_with("iRule"))
    .map(|i| i.label)
    .collect()
}

#[test]
fn irules_event_snippets_surface_at_top_level() {
    // `irule` partial at a top-level command position in the iRules
    // dialect, including canonicalised legacy aliases, offers every
    // event template.
    for dialect in ["f5-irules", "irules", "tcl-irule"] {
        let labels = irule_snippet_labels(
            "irule",
            0,
            5,
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        );
        assert!(
            labels.iter().any(|l| l == "iRule RULE_INIT"),
            "{dialect}: expected RULE_INIT; got {labels:?}",
        );
        assert!(
            labels.iter().any(|l| l == "iRule HTTP_REQUEST"),
            "{dialect}: expected HTTP_REQUEST; got {labels:?}",
        );
    }
}

#[test]
fn completion_builds_resolved_event_facts_once_per_request() {
    let before = crate::irules_context::expensive_build_count();
    let labels = irule_snippet_labels("irule", 0, 5, tcl_dialect::DialectProfile::irules());
    assert!(labels.iter().any(|label| label == "iRule RULE_INIT"));
    assert_eq!(
        crate::irules_context::expensive_build_count(),
        before + 1,
        "completion must reuse one precomputed identity/boundary inventory"
    );
}

#[test]
fn irules_snippets_decline_for_already_declared_event() {
    // `when RULE_INIT { }` already declared → the RULE_INIT template
    // drops out, but other event templates remain.
    let src = "when RULE_INIT {\n}\nirule";
    let labels = irule_snippet_labels(src, 2, 5, tcl_dialect::DialectProfile::irules());
    assert!(
        !labels.iter().any(|l| l == "iRule RULE_INIT"),
        "RULE_INIT already declared, should decline; got {labels:?}",
    );
    assert!(
        labels.iter().any(|l| l == "iRule HTTP_REQUEST"),
        "HTTP_REQUEST not declared, should remain; got {labels:?}",
    );
}

#[test]
fn irules_event_snippets_suppressed_inside_when_body() {
    // Cursor inside a `when HTTP_REQUEST { }` body → the top-level
    // guard suppresses every event template.
    let src = "when HTTP_REQUEST {\n    irule\n}\n";
    let labels = irule_snippet_labels(src, 1, 9, tcl_dialect::DialectProfile::irules());
    assert!(
        labels.is_empty(),
        "event templates must not offer inside a when block; got {labels:?}",
    );
}

#[test]
fn irules_snippets_hidden_in_plain_tcl_dialect() {
    // Same source, but the `tcl8.6` dialect carries no iRule
    // templates at all.
    let labels = irule_snippet_labels(
        "irule",
        0,
        5,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(labels.is_empty(), "got {labels:?}");
}

// The shared candidate filter (prefix path + fuzzy fallback).

#[test]
fn filter_candidates_prefix_path_keeps_input_order() {
    let FilteredCandidates { candidates, fuzzy } =
        filter_candidates("alpha", vec!["delta", "alpha", "beta", "alphabet"], |n| *n);
    assert!(!fuzzy);
    assert_eq!(candidates, vec!["alpha", "alphabet"]);
}

#[test]
fn filter_candidates_empty_partial_keeps_everything() {
    let FilteredCandidates { candidates, fuzzy } = filter_candidates("", vec!["b", "a"], |n| *n);
    assert!(!fuzzy);
    assert_eq!(candidates, vec!["b", "a"]);
}

#[test]
fn filter_candidates_no_fallback_for_one_char_fragment() {
    let FilteredCandidates { candidates, fuzzy } =
        filter_candidates("q", vec!["set", "puts"], |n| *n);
    assert!(!fuzzy);
    assert_eq!(candidates, [] as [&str; 0]);
}

#[test]
fn filter_candidates_fuzzy_ranks_by_distance_then_name() {
    // No candidate prefix-matches `abcdef`; `zbcdef` is one edit
    // away, `abcdxy` two — distance outranks the name order.
    let FilteredCandidates { candidates, fuzzy } =
        filter_candidates("abcdef", vec!["abcdxy", "zbcdef", "qqqqqq"], |n| *n);
    assert!(fuzzy);
    assert_eq!(candidates, vec!["zbcdef", "abcdxy"]);
}

#[test]
fn filter_candidates_fuzzy_cap_respected() {
    // Nine candidates all one edit from `az` — the fallback caps
    // at MAX_FUZZY_SUGGESTIONS, keeping the name-order head.
    let names = vec!["aa", "ab", "ac", "ad", "ae", "af", "ag", "ah", "ai"];
    let FilteredCandidates { candidates, fuzzy } = filter_candidates("az", names, |n| *n);
    assert!(fuzzy);
    assert_eq!(candidates.len(), MAX_FUZZY_SUGGESTIONS);
    assert_eq!(candidates[0], "aa");
    assert!(!candidates.contains(&"ai"), "cap should drop the tail");
}

#[test]
fn filter_candidates_fuzzy_respects_distance_budget() {
    // `xyzzy` is beyond the scaled edit budget of every candidate.
    let FilteredCandidates { candidates, fuzzy } =
        filter_candidates("xyzzy", vec!["set", "puts"], |n| *n);
    assert!(!fuzzy);
    assert_eq!(candidates, [] as [&str; 0]);
}

// Pinned prefix lists — the byte-identical contract for fragments
// that match something.

#[test]
fn prefix_proc_list_pinned() {
    let src = "proc alpha {} {}\nproc beta {} {}\nal\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        2,
        2,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["alpha"]);
    assert_eq!(items[0].insert_text, "::alpha");
    assert_eq!(
        items[0].filter_text, None,
        "prefix matches stay undecorated"
    );
}

#[test]
fn prefix_subcommand_list_pinned() {
    let src = "string to\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        9,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["tolower", "totitle", "toupper"]);
    assert!(items.iter().all(|i| i.filter_text.is_none()));
}

// Fuzzy fallback behaviour through the public entry point.

#[test]
fn fuzzy_fallback_offers_lsearch_for_lsaerch() {
    let src = "lsaerch\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        7,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let lsearch = items
        .iter()
        .find(|i| i.label == "lsearch")
        .unwrap_or_else(|| panic!("expected fuzzy lsearch, got {items:?}"));
    // Decorated so editors neither hide nor re-order the item.
    assert_eq!(lsearch.filter_text.as_deref(), Some("lsaerch"));
    assert_eq!(lsearch.sort_text.as_deref(), Some("F00_lsearch"));
}

#[test]
fn fuzzy_fallback_skips_one_char_fragment() {
    // `q` prefix-matches no command and is below the fragment
    // minimum — the response stays empty rather than fuzzing.
    let src = "q\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        1,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(items.is_empty(), "got {items:?}");
}

#[test]
fn fuzzy_subcommand_offers_length_for_lenght() {
    let src = "string lenght\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        13,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["length"], "typo'd subcommand fragment");
    assert_eq!(items[0].filter_text.as_deref(), Some("lenght"));
}

#[test]
fn fuzzy_switch_offers_nocase_for_ncoase() {
    let src = "lsort -ncoase\n";
    let analysis = analyse(src);
    let registry = CommandRegistry::build_default();
    let items = completions(
        src,
        0,
        13,
        &analysis,
        Some(&registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let nocase = items
        .iter()
        .find(|i| i.label == "-nocase")
        .unwrap_or_else(|| panic!("expected fuzzy -nocase, got {items:?}"));
    // The filter text carries the dash, matching the replaced range.
    assert_eq!(nocase.filter_text.as_deref(), Some("-ncoase"));
    let edit = nocase.text_edit.as_ref().expect("switch replace edit");
    assert_eq!(edit.new_text, "-nocase");
}

#[test]
fn fuzzy_variable_offers_banana_for_bnaana() {
    let src = "set apple 1\nset banana 2\nputs $bnaana\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        2,
        12,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["$banana"], "typo'd variable fragment");
    // Sigil included, matching the replaced `$…` range.
    assert_eq!(items[0].filter_text.as_deref(), Some("$bnaana"));
    let edit = items[0].text_edit.as_ref().expect("variable replace edit");
    assert_eq!(edit.new_text, "$banana");
}

#[test]
fn fuzzy_array_element_offers_key_for_kye() {
    let src = "set arr(key) 1\nset v $arr(kye\n";
    let analysis = analyse(src);
    let items = completions(
        src,
        1,
        14,
        &analysis,
        None,
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["$arr(key)"], "typo'd array index");
    assert_eq!(items[0].filter_text.as_deref(), Some("$arr(kye"));
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

#[test]
fn proc_completion_quotes_retained_callable_source() {
    let source = "proc {odd name} {} {}";
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(source, engine);
        let items = proc_completions(&analysis, "odd", &FxHashMap::default());
        assert_eq!(items.len(), 1, "{engine}: {items:?}");
        assert_eq!(items[0].insert_text, "\"::odd name\"", "{engine}");
    }
}

#[test]
fn proc_completion_keeps_independent_original_inventory_when_ui_geometry_is_removed() {
    let mut analysis = analyse("proc greet {} {}");
    analysis.all_procs.get_mut("::greet").unwrap().source_name = None;
    assert_eq!(
        proc_completions(&analysis, "gre", &FxHashMap::default()).len(),
        1
    );
    analysis.original_procedure_metadata.clear();
    assert_eq!(
        proc_completions(&analysis, "gre", &FxHashMap::default()),
        [] as [crate::completion::CompletionItem; 0]
    );
}

#[test]
fn proc_completion_withdraws_unaddressable_colon_publication() {
    let source = "namespace eval N {proc :f {} {return LOCAL}}";
    for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(source, engine);
        assert!(analysis.all_procs.contains_key("::N:::f"), "{engine}");
        assert!(
            proc_completions(&analysis, ":f", &FxHashMap::default()).is_empty(),
            "{engine}"
        );
    }
}

#[test]
fn invoked_proc_completions_fuzzy_falls_back() {
    let src = "proc greet {} {}\nproc shout {} {}\n";
    let analysis = analyse(src);
    // Prefix path unchanged…
    let items = invoked_proc_completions(&analysis, "gre");
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["greet"]);
    assert!(items[0].filter_text.is_none());
    // …and the typo'd fragment falls back to the closest proc.
    let items = invoked_proc_completions(&analysis, "gret");
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(labels, vec!["greet"]);
    assert_eq!(items[0].filter_text.as_deref(), Some("gret"));
}

// Dialect-profile availability (dialect-profile-model.md): the §5.1
// available_subcommands gap and the §5.2 option semantics in
// completion.

#[test]
fn subcommand_completion_is_version_gated_by_the_profile() {
    let src = "dict \n";
    let analysis = analyse_for(src, "tcl8.6");
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    // 8.6 buffer: 9.0-only subcommands are not offered.
    let items = completions(
        src,
        0,
        5,
        &analysis,
        Some(registry),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(labels.contains(&"get"), "{labels:?}");
    assert!(
        !labels.contains(&"getwithdefault"),
        "dict getwithdefault is 9.0+ and must not be offered under tcl8.6: {labels:?}"
    );
    // 9.0 buffer: it is offered.
    let registry90 = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let analysis90 = analyse_for(src, "tcl9.0");
    let items90 = completions(
        src,
        0,
        5,
        &analysis90,
        Some(registry90),
        None,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    let labels90: Vec<&str> = items90.iter().map(|i| i.label.as_str()).collect();
    assert!(labels90.contains(&"getwithdefault"), "{labels90:?}");
}

#[test]
fn command_completion_uses_profile_availability() {
    let src = "dic\n";
    let analysis = analyse_for(src, "f5-iapps");
    // iApps ride the `f5-tcl` trunk — a fork of Tcl at 8.4.6
    // (measured, bigip-irule-parser-measurements.md §4a: `dict` fails
    // in `IAppImplementation`) — so `dict` is NOT offered there…
    let registry = tcl_registry::model::ingress::static_context_for("f5-iapps").commands();
    let items = completions(
        src,
        0,
        3,
        &analysis,
        Some(registry),
        None,
        tcl_registry::model::ingress::resolve_environment("f5-iapps").analyser_profile(),
    );
    assert!(
        !items.iter().any(|i| i.label == "dict"),
        "dict must not be offered under f5-iapps (8.4.6 trunk, measured)"
    );
    // …while the banned iRules commands never surface there.
    let irules_reg = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    let exec_src = "exe\n";
    let exec_analysis = analyse_for(exec_src, "f5-irules");
    let irules_items = completions(
        exec_src,
        0,
        3,
        &exec_analysis,
        Some(irules_reg),
        None,
        tcl_dialect::DialectProfile::irules(),
    );
    assert!(
        !irules_items.iter().any(|i| i.label == "exec"),
        "exec is banned in iRules and must not be completed"
    );
}

#[test]
fn switch_option_completion_respects_profile_gating() {
    // Option completion under f5-iapps follows the measured fork:
    // F5 reclassification (measurements §4/§4a,
    // `docs/design/f5/bigip-irule-parser-measurements.md`) — the iApps
    // host is the 8.4.6 `f5-tcl` fork, not the falsified 8.5.13
    // hypothesis, so the 8.5+ `switch -nocase` is not offered
    // there, and the 9.0-only `regsub
    // -command` stays out. The 8.4-real `-exact` is offered, so
    // the composed vendor mask keeps working.
    let src = "switch -\n";
    let analysis = analyse_for(src, "f5-iapps");
    let registry = tcl_registry::model::ingress::static_context_for("f5-iapps").commands();
    let items = completions(
        src,
        0,
        8,
        &analysis,
        Some(registry),
        None,
        tcl_registry::model::ingress::resolve_environment("f5-iapps").analyser_profile(),
    );
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"-exact"),
        "switch -exact is 8.4 core, offered under f5-iapps: {labels:?}"
    );
    assert!(
        !labels.contains(&"-nocase"),
        "switch -nocase is 8.5+ and the iApps host is a measured 8.4.6 fork: {labels:?}"
    );

    let src9 = "regsub -\n";
    let analysis9 = analyse_for(src9, "f5-iapps");
    let items9 = completions(
        src9,
        0,
        8,
        &analysis9,
        Some(registry),
        None,
        tcl_registry::model::ingress::resolve_environment("f5-iapps").analyser_profile(),
    );
    let labels9: Vec<&str> = items9.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels9.contains(&"-command"),
        "regsub -command is 9.0-only, hidden under f5-iapps: {labels9:?}"
    );
}
