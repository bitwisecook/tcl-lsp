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

//! Members lifted out of class definitions: `TclOO`, snit and itcl method units, declared variables and the evidence left by unreadable bodies.

use super::*;

// The `extract_oo_methods_pass` post-pass
// lifts TclOO method bodies into `module.methods`.

#[test]
fn extracts_oo_class_methods() {
    let src = "oo::class create Counter {\n\
                   \x20   variable n\n\
                   \x20   constructor {} { set n 0 }\n\
                   \x20   method bump {} { incr n }\n\
                   \x20   method get {} { return $n }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    let mut keys: Vec<String> = m
        .methods
        .values()
        .map(crate::ir::MethodDef::reporting_label)
        .collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            "::Counter::<constructor>".to_string(),
            "::Counter::bump".to_string(),
            "::Counter::get".to_string(),
        ],
        "methods: {keys:?}",
    );
    let get = fixture_method(&m.methods, "::Counter::get").expect("unique retained fixture method");
    assert_eq!(get.method_name, "get");
    assert_eq!(get.kind, MethodKind::Method);
    assert_eq!(get.class_name, "::Counter");
    assert_eq!(
        get.execution_namespace,
        crate::ir::ExecutionNamespace::RuntimeSelected,
        "TclOO resolves relative commands in the runtime receiver namespace"
    );
    // `variable n` in the class body is an in-scope instance var
    // for every method (auto-linked).
    assert!(
        get.instance_vars.contains("n"),
        "get ivars: {:?}",
        get.instance_vars
    );
    // The method body is lowered for analysis (the `incr n` /
    // `return $n` statements are present, not an empty barrier).
    let bump =
        fixture_method(&m.methods, "::Counter::bump").expect("unique retained fixture method");
    assert!(
        !bump.body.statements.is_empty(),
        "bump body should be lowered"
    );
    let ctor = fixture_method(&m.methods, "::Counter::<constructor>")
        .expect("unique retained fixture method");
    assert_eq!(ctor.kind, MethodKind::Constructor);
    assert_eq!(m.redefined_methods.len(), 0);
    assert!(
        !m.oo_evidence.unretained_executable_roots,
        "every executable body in a fully static class was retained"
    );
}

#[test]
fn lowers_oo_configurable_class_body() {
    // `oo::configurable create` (and `oo::abstract` / `oo::singleton`)
    // share the `METACLASS create NAME { body }` shape, so their bodies
    // must be lowered and their methods lifted like `oo::class`'s — not left
    // as an unanalysed barrier, which would skip every method body of a
    // class defined with `oo::configurable`.
    let src = "oo::configurable create Pin {\n\
                   \x20   property node\n\
                   \x20   method describe {} { return [my configure -node] }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::Pin::describe"),
        "oo::configurable method must be lifted; methods: {:?}",
        m.methods.keys().collect::<Vec<_>>()
    );
    assert!(
        !fixture_method(&m.methods, "::Pin::describe")
            .expect("unique retained fixture method")
            .body
            .statements
            .is_empty(),
        "the method body should be lowered, not barriered"
    );
}

#[test]
fn extracts_oo_define_methods_and_namespaced_class() {
    // `oo::define` adds methods to an existing class, and a class
    // created inside `namespace eval` qualifies under that ns.
    let src = "namespace eval ::app {\n\
                   \x20   oo::class create Widget {\n\
                   \x20       method draw {} { return ok }\n\
                   \x20   }\n\
                   }\n\
                   oo::define ::app::Widget {\n\
                   \x20   method hide {} { return done }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::app::Widget::draw"),
        "methods: {:?}",
        m.methods.keys().collect::<Vec<_>>()
    );
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::app::Widget::hide"),
        "methods: {:?}",
        m.methods.keys().collect::<Vec<_>>()
    );
}

// Extraction is driven by the registry definer grammars, so
// oo::objdefine, snit, and itcl bodies produce method units too.

// TP: an `oo::objdefine $obj { … }` body produces per-object method
// units, so the body is visible to analysis.
#[test]
fn objdefine_bodies_produce_per_object_method_units() {
    let src = "oo::class create C {}\n\
                   set k [C new]\n\
                   oo::objdefine $k {\n\
                   \x20   variable z\n\
                   \x20   method probe {} { return $z }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    let unit = m
        .methods
        .unique_display("::@objdefine@::k::probe")
        .map_or_else(
            || panic!("methods: {:?}", m.methods.keys().collect::<Vec<_>>()),
            |(_, method)| method,
        );
    assert_eq!(unit.method_name, "probe");
    assert!(!unit.body.statements.is_empty(), "body must be lowered");
    // The block's `variable z` is per-object state in scope for the
    // per-object method.
    assert!(
        unit.instance_vars.contains("z"),
        "ivars: {:?}",
        unit.instance_vars
    );
}

// TN: objdefine `variable` declarations are
// per-object — they must NOT pollute the real class's cross-block
// instance-variable union.
#[test]
fn objdefine_variables_stay_out_of_the_class_union() {
    let src = "oo::class create C {\n\
                   \x20   method m {} { info exists z }\n\
                   }\n\
                   set k [C new]\n\
                   oo::objdefine $k { variable z }\n";
    let m = lower_to_ir(src, &reg());
    let class_m = fixture_method(&m.methods, "::C::m").expect("unique retained fixture method");
    assert!(
        !class_m.instance_vars.contains("z"),
        "per-object `z` leaked into the class union: {:?}",
        class_m.instance_vars
    );
}

// TP (item 2): snit method / typemethod / constructor bodies become
// method units, with the grammar's implicit member-body variables
// (self / selfns / type / options) and declared type variables in scope.
#[test]
fn snit_type_bodies_produce_method_units() {
    let src = "snit::type Dog {\n\
                   \x20   variable name\n\
                   \x20   constructor {args} { set name fido }\n\
                   \x20   method bark {} { return \"$name barks\" }\n\
                   \x20   typemethod count {} { return 0 }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    let bark = m.methods.unique_display("::Dog::bark").map_or_else(
        || panic!("methods: {:?}", m.methods.keys().collect::<Vec<_>>()),
        |(_, method)| method,
    );
    assert_eq!(bark.kind, MethodKind::Method);
    assert_eq!(
        bark.execution_namespace,
        crate::ir::ExecutionNamespace::exact("::Dog")
    );
    assert_ne!(bark.body.statements, [] as [crate::ir::Statement; 0]);
    for implicit in ["self", "selfns", "type", "options", "name"] {
        assert!(
            bark.instance_vars.contains(implicit),
            "missing {implicit}: {:?}",
            bark.instance_vars
        );
    }
    assert_eq!(
        fixture_method(&m.methods, "::Dog::count")
            .expect("unique retained fixture method")
            .kind,
        MethodKind::ClassMethod,
        "typemethod dispatches on the type command"
    );
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::Dog::<constructor>")
    );
}

// TP: the widget grammar's extra implicit vars (win / hull) reach
// widget method units — registry data, not a name-suffix check.
#[test]
fn snit_widget_bodies_see_win_and_hull() {
    let src = "snit::widget MyBar {\n\
                   \x20   method redraw {} { return $win }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    let redraw = m.methods.unique_display("::MyBar::redraw").map_or_else(
        || panic!("methods: {:?}", m.methods.keys().collect::<Vec<_>>()),
        |(_, method)| method,
    );
    assert!(redraw.instance_vars.contains("win"));
    assert!(redraw.instance_vars.contains("hull"));
}

// TP (item 2): itcl method bodies — including the access-modifier
// wrapped spellings — become method units with `this`, instance
// variables, and commons in scope.
#[test]
fn itcl_class_bodies_produce_method_units() {
    let src = "itcl::class Toaster {\n\
                   \x20   variable crumbs 0\n\
                   \x20   common heat 3\n\
                   \x20   public method toast {n} { incr crumbs $n }\n\
                   \x20   method clean {} { set crumbs 0 }\n\
                   \x20   constructor {} { set crumbs 0 }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    let toast = m.methods.unique_display("::Toaster::toast").map_or_else(
        || panic!("methods: {:?}", m.methods.keys().collect::<Vec<_>>()),
        |(_, method)| method,
    );
    assert_eq!(toast.params, vec!["n".to_string()]);
    assert_eq!(
        toast.execution_namespace,
        crate::ir::ExecutionNamespace::exact("::Toaster")
    );
    for name in ["this", "crumbs", "heat"] {
        assert!(
            toast.instance_vars.contains(name),
            "missing {name}: {:?}",
            toast.instance_vars
        );
    }
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::Toaster::clean")
    );
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::Toaster::<constructor>")
    );
}

#[test]
fn definition_member_words_use_positioned_source_and_retained_grammar() {
    // Software source projection only: no runtime member or frame grant.
    use crate::registry_invocation::EffectiveInvocationWord as Word;
    let source = "method {a\\\n  b} \"$name\" {*}$args";
    let config = tcl_lexer::LexerConfig::for_dialect("tcl8.6");
    for (config, expected) in [
        (config, "a b"),
        (
            tcl_lexer::LexerConfig {
                brace_backslash_newline: tcl_dialect::BraceBackslashNewline::Literal,
                ..config
            },
            "a\\\n  b",
        ),
    ] {
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .expect("one original member invocation");
        let tokens =
            CommandTokens::from_segmented(&tcl_lexer::SourceMap::new(source), config, &segment);
        assert_eq!(
            statement_words(&tokens, config),
            vec![
                Word::Literal("method".to_owned()),
                Word::Literal(expected.to_owned()),
                Word::Dynamic,
                Word::Expanded,
            ]
        );
    }
}

// TP: TclOO wrapper members — `self { method … }` lifts a
// classmethod-kind unit, `private { method … }` an instance one; the
// prefix spellings match.
#[test]
fn tcloo_wrapper_members_produce_units() {
    let src = "oo::class create W {\n\
                   \x20   self { method make {} { return [my new] } }\n\
                   \x20   private { method hidden {} { return 1 } }\n\
                   \x20   self method direct {} { return 2 }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    assert_eq!(
        fixture_method(&m.methods, "::W::make").map_or_else(
            || panic!("methods: {:?}", m.methods.keys().collect::<Vec<_>>()),
            |d| d.kind
        ),
        MethodKind::ClassMethod
    );
    assert_eq!(
        fixture_method(&m.methods, "::W::direct")
            .expect("unique retained fixture method")
            .kind,
        MethodKind::ClassMethod
    );
    assert_eq!(
        fixture_method(&m.methods, "::W::hidden")
            .expect("unique retained fixture method")
            .kind,
        MethodKind::Method
    );
    assert!(!m.oo_evidence.unretained_executable_roots);
}

// TN: an `initialise` body is a *definition script* evaluated in the
// class object's namespace, not a method frame — no unit.
#[test]
fn tcloo_initialise_body_is_not_a_method_unit() {
    let src = "oo::class create I {\n\
                   \x20   initialise { variable cache {} }\n\
                   \x20   method read {} { return 1 }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        !m.methods
            .values()
            .any(|m| m.reporting_label().contains("initialise")),
        "methods: {:?}",
        m.methods.keys().collect::<Vec<_>>()
    );
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::I::read")
    );
    assert!(
        m.oo_evidence.unretained_executable_roots,
        "the unrepresented initialise script must remain explicit evidence"
    );
}

// TN: `self constructor` / `self destructor` name no member (tclsh
// 8.6.18, 9.0.4 and 9.1: `invalid command name "constructor"`), so
// neither lifts a unit —
// while the instance-side spellings beside them do.
#[test]
fn tcloo_self_constructor_and_destructor_lift_no_unit() {
    let src = "oo::class create S {\n\
                   \x20   self constructor {} { set a 1 }\n\
                   \x20   self destructor { set b 2 }\n\
                   \x20   method m {} { return 3 }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        m.methods
            .values()
            .all(|d| !matches!(d.kind, MethodKind::Constructor | MethodKind::Destructor)),
        "methods: {:?}",
        m.methods.keys().collect::<Vec<_>>()
    );
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::S::m")
    );
    let src = "oo::class create T {\n\
                   \x20   constructor {} { set a 1 }\n\
                   \x20   destructor { set b 2 }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    assert_eq!(
        fixture_method(&m.methods, "::T::<constructor>")
            .expect("unique retained fixture method")
            .kind,
        MethodKind::Constructor
    );
    assert!(
        m.methods.values().any(|d| d.kind == MethodKind::Destructor),
        "methods: {:?}",
        m.methods.keys().collect::<Vec<_>>()
    );
}

// TN: a dynamic objdefine receiver that is not a simple variable
// reference (a command substitution) stays un-extracted.
#[test]
fn objdefine_command_substitution_receiver_abstains() {
    let src = "oo::class create C {}\n\
                   oo::objdefine [C new] { method m {} { return 1 } }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        !m.methods
            .values()
            .any(|m| m.reporting_label().starts_with("::@objdefine@")),
        "methods: {:?}",
        m.methods.keys().collect::<Vec<_>>()
    );
    assert!(m.oo_evidence.unretained_executable_roots);
}

#[test]
fn colliding_method_reports_retain_distinct_bodies_and_constructor_purposes() {
    let source = "namespace eval A {}\noo::class create A::B {method c {} {return LEFT}}\noo::class create A {method B::c {} {return RIGHT}; method <constructor> {} {return NAMED}; constructor {} {set local ACTUAL}; export B::c <constructor>}\n";
    let module = lower_to_ir(source, &reg());
    assert_eq!(module.methods.len(), 4);
    assert_eq!(
        module
            .methods
            .keys()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        4
    );
    assert!(module.methods.unique_display("::A::B::c").is_none());
    assert!(
        module
            .methods
            .unique_display("::A::<constructor>")
            .is_none()
    );
    assert_eq!(
        module
            .methods
            .values()
            .filter(|method| method.kind == MethodKind::Constructor)
            .count(),
        1
    );
    assert_eq!(
        module
            .methods
            .values()
            .filter(
                |method| method.kind == MethodKind::Method && method.method_name == "<constructor>"
            )
            .count(),
        1
    );
    assert!(module.redefined_methods.is_empty());
    for id in module.methods.keys() {
        assert_eq!(module.method_definition_bodies(*id).count(), 1);
    }
}

#[test]
fn redefined_oo_method_retains_the_replacement_body() {
    // A method redefined by a later `oo::define` keeps the first
    // body in `methods`, and RETAINS the replacement in
    // `redefined_methods` so analysis consumers can
    // scan every body a dispatch may run instead of abstaining.
    let src = "oo::class create C {\n\
                   \x20   method m {} { return 1 }\n\
                   }\n\
                   oo::define C {\n\
                   \x20   method m {} { return 2 }\n\
                   }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::C::m")
    );
    let (primary, replacements) = m
        .redefined_methods
        .iter()
        .next()
        .unwrap_or_else(|| panic!("redefined: {:?}", m.redefined_methods));
    assert_eq!(m.methods.get(primary).unwrap().method_name, "m");
    assert_eq!(replacements.len(), 1, "one replacement body retained");
    let retained = m.methods.get(&replacements[0]).unwrap();
    assert_eq!(retained.class_name, "::C");
    assert!(
        !retained.body.statements.is_empty(),
        "the replacement body is lowered, not discarded"
    );
    assert_eq!(m.oo_unanalysed_classes.len(), 0);
    assert!(!m.oo_evidence.dynamic_target);
    assert!(!m.oo_evidence.unretained_executable_roots);
}

#[test]
fn unreadable_oo_members_flag_the_class_or_module() {
    // A dynamic member NAME may redefine any method of the class —
    // the class is flagged unanalysable.
    let m = lower_to_ir(
        "oo::class create C {\n    method m {} { return 1 }\n    method $n {} { return 2 }\n}\n",
        &reg(),
    );
    assert!(m.oo_unanalysed_classes.contains("::C"), "{m:?}");
    assert!(m.oo_evidence.unretained_executable_roots);
    // A dynamic member BODY is a method whose code no scan can read.
    let m = lower_to_ir("oo::class create C {\n    method m {} $body\n}\n", &reg());
    assert!(m.oo_unanalysed_classes.contains("::C"));
    assert!(m.oo_evidence.unretained_executable_roots);
    // A dynamic CLASS word may touch any class — module-wide flag.
    let m = lower_to_ir("oo::define $cls { method m {} { return 1 } }\n", &reg());
    assert!(m.oo_evidence.dynamic_target);
    assert!(m.oo_evidence.unretained_executable_roots);
    // A fully-static module sets neither.
    let m = lower_to_ir("oo::class create C { method m {} { return 1 } }\n", &reg());
    assert_eq!(m.oo_unanalysed_classes.len(), 0);
    assert!(!m.oo_evidence.dynamic_target);
    assert!(!m.oo_evidence.unretained_executable_roots);
}

#[test]
fn every_unextracted_oo_body_shape_leaves_module_evidence() {
    for source in [
        // The outer definition script is computed, so the definer post-pass
        // cannot segment it.
        "oo::class create C [list method mutate {} { return 1 }]",
        // Inline oo::define is body-bearing but is not a braced definition
        // block accepted by classify_definer_call.
        "oo::class create C {}; oo::define C method mutate {} { return 1 }",
        // Accessor and configuration scripts are deliberately not MethodDef
        // frames, but remain runtime-callable executable bodies.
        "oo::configurable create C { property p -get { return 1 } }",
        "itcl::class C { variable value 0 { return 1 } }",
        // A definition created when an already-extracted method runs is not
        // visited by this source-root post-pass.
        "oo::class create Outer { method install {} { oo::class create Inner { method mutate {} { return 1 } } } }",
        // Definition scripts may invoke ordinary commands; such execution
        // is not represented by a lifted method body.
        "oo::class create C { ::rename ::set ::saved_set }",
    ] {
        let module = lower_to_ir(source, &reg());
        assert!(
            module.oo_evidence.unretained_executable_roots,
            "unretained executable definition body was not recorded: {source}"
        );
    }
}

#[test]
fn oo_method_nested_proc_not_registered_globally() {
    // A `proc` defined inside a method body is created at
    // method-call time, not script load — it must not leak into
    // `module.procedures` (codegen safety).
    let src = "oo::class create C {\n\
                   \x20   method m {} { proc helper {} { return 1 }; return 2 }\n\
                   }\n\
                   proc toplevel {} { return 3 }\n";
    let m = lower_to_ir(src, &reg());
    assert!(
        fixture_id_if_present(&m.procedures, "::toplevel").is_some(),
        "procs: {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
    assert!(
        fixture_id_if_present(&m.procedures, "::helper").is_none(),
        "nested method proc must not be registered: {:?}",
        m.procedures.keys().collect::<Vec<_>>()
    );
    // The method body still lowered the `proc helper ...` call.
    assert!(
        m.methods
            .values()
            .any(|method| method.reporting_label() == "::C::m")
    );
}

#[test]
fn no_oo_methods_for_plain_script() {
    let m = lower_to_ir("proc greet {} { puts hi }", &reg());
    assert_eq!(m.methods.len(), 0);
    assert_eq!(m.redefined_methods.len(), 0);
}

#[test]
fn oo_method_params_use_the_formal_parameter_grammar() {
    let m = lower_to_ir(
        "oo::class create C {\n    method m {a\\ b {c 1}} { return 1 }\n}\n",
        &reg(),
    );
    let method = fixture_method(&m.methods, "::C::m").unwrap_or_else(|| {
        panic!(
            "::C::m lowered, got {:?}",
            m.methods.keys().collect::<Vec<_>>()
        )
    });
    assert_eq!(method.params, vec!["a", "c"]);
    // A parameter list Tcl rejects makes the whole class unanalysable.
    let m = lower_to_ir(
        "oo::class create C {\n    method m {{a b c}} { return 1 }\n}\n",
        &reg(),
    );
    assert!(m.oo_unanalysed_classes.contains("::C"), "{m:?}");
    assert!(
        !m.methods
            .values()
            .any(|method| method.reporting_label() == "::C::m")
    );
}

#[test]
fn unavailable_lowering_metadata_keeps_generic_dispatch_and_conservative_method_names() {
    let registry = reg();
    let source = "unresolved local $dynamic";
    let mut lowerer = Lowerer::new(&registry);
    lowerer.source_entry_origin = SourceEntryOrigin::Driver;
    lowerer.metadata_origin = LoweringMetadataOrigin::Supplied;
    lowerer.word_space = WordSpace::new(source, 0, lowerer.source_channel);
    let commands =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, lowerer.config);
    let statement = lowerer.lower_default(&commands[0], "::");
    let Statement::Call {
        canonical_command,
        defs,
        reads,
        ..
    } = &statement
    else {
        panic!("missing original/context ownership must retain a generic call");
    };
    assert!(canonical_command.is_none());
    assert_eq!(defs.len(), 0);
    assert_eq!(reads.len(), 0);
    assert_eq!(
        lowerer.advisory_method_instance_names(&statement),
        vec!["local", "${dynamic}"]
    );
}

#[test]
fn method_variable_advice_preserves_literal_names_and_captured_prefix_values() {
    let registry = reg();
    let mut lowerer = Lowerer::new(&registry).trace_visible();
    lowerer.lower("variable {}; variable {$counter}; variable {brace{name}}; variable {-dash}");
    let names: Vec<_> = lowerer
        .module
        .top_level
        .statements
        .iter()
        .flat_map(|statement| lowerer.advisory_method_instance_names(statement))
        .collect();
    assert_eq!(names, vec!["", "$counter", "brace{name}", "-dash"]);

    let mut captured = Lowerer::new(&registry).trace_visible();
    captured.lower("interp alias {} ivar {} variable {$counter}; ivar");
    let statement = captured.module.top_level.statements.last().unwrap();
    assert_eq!(
        captured.advisory_method_instance_names(statement),
        vec!["$counter"]
    );
}

#[test]
fn unresolved_method_name_protection_keeps_original_source_text_without_reparsing() {
    let registry = reg();
    let source = "unresolved {} {$counter} {brace{name}} {a(b)} {-dash} $dynamic";
    let mut lowerer = Lowerer::new(&registry);
    lowerer.source_entry_origin = SourceEntryOrigin::Driver;
    lowerer.metadata_origin = LoweringMetadataOrigin::Supplied;
    lowerer.word_space = WordSpace::new(source, 0, lowerer.source_channel);
    let commands =
        crate::segmenter::segment_commands_with_offset_and_config(source, 0, lowerer.config);
    let statement = lowerer.lower_default(&commands[0], "::");
    let Statement::Call { args, .. } = &statement else {
        panic!("generic source fallback");
    };
    assert_eq!(lowerer.advisory_method_instance_names(&statement), *args);
    assert_eq!(
        args,
        &vec!["", "$counter", "brace{name}", "a(b)", "-dash", "${dynamic}"]
    );
}

#[test]
fn class_variable_source_union_keeps_decoded_names_and_slot_operation_boundaries() {
    let registry = reg();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let source = r"variable -set {} {$counter} {brace{name}} {-dash}; variable \$counter";
    let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let names = declared_member_vars(
        &tcl_registry::definer::TCLOO_GRAMMAR,
        &segments,
        Some(profile.surface_query()),
        config,
    );
    assert_eq!(
        names,
        HashSet::from([
            String::new(),
            "$counter".to_owned(),
            "brace{name}".to_owned(),
            "-dash".to_owned()
        ])
    );
    assert!(!names.contains("counter"));
    assert!(!names.contains("-set"));
    let escaped_segments =
        crate::segmenter::segment_commands_with_offset_and_config(r"variable \$counter", 0, config);
    assert_eq!(
        declared_member_vars(
            &tcl_registry::definer::TCLOO_GRAMMAR,
            &escaped_segments,
            Some(profile.surface_query()),
            config,
        ),
        HashSet::from(["$counter".to_owned()])
    );
}

#[test]
fn class_variable_unknown_operands_keep_known_literal_protection_without_reference_parsing() {
    let registry = reg();
    let profile = registry.profile().unwrap();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let segments = crate::segmenter::segment_commands_with_offset_and_config(
        "variable $dynamic {} {$counter} {a(b)}",
        0,
        config,
    );
    let names = declared_member_vars(
        &tcl_registry::definer::TCLOO_GRAMMAR,
        &segments,
        Some(profile.surface_query()),
        config,
    );
    assert_eq!(
        names,
        HashSet::from([String::new(), "$counter".to_owned(), "a(b)".to_owned()])
    );
    assert!(!names.contains("counter"));
    assert!(!names.contains("a"));
    assert!(!names.contains("dynamic"));
}

/// A proved call of a document procedure or class runs a body the module
/// retains; only a call whose execution is unbounded may supply another root.
#[test]
fn only_an_unbounded_call_leaves_unretained_executable_roots() {
    for (source, expected) in [
        ("proc helper {x} {return $x}\nhelper 1\n", false),
        (
            "oo::class create C {method m {} {return 1}}\nC create c\n",
            false,
        ),
        (
            "oo::class create C {method m {} {return 1}}\nset o [C new]\n",
            false,
        ),
        ("helper 1\n", true),
        ("$cmd 1\n", true),
        (
            "proc helper {x} {return $x}\nrename helper h2\nhelper 1\n",
            true,
        ),
        (
            "oo::class create C {}\nset b {return 1}\noo::define C method m {} $b\n",
            true,
        ),
    ] {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let input = analysis.resolved_input.as_ref().unwrap();
        let module = lower_to_ir_with(
            Lowerer::with_config(
                input.borrowed_context_registry().commands(),
                input.lexer_config(),
            )
            .with_resolved_analysis_input(input.clone()),
            source,
        );
        assert_eq!(
            module.oo_evidence.unretained_executable_roots, expected,
            "{source:?}"
        );
    }
}
