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

//! What a build produces: the top level, procedures and method units, fixture lookup and the complexity guards.

use super::*;

fn fixture_method<'a>(unit: &'a CompilationUnit, label: &str) -> Option<&'a FunctionUnit> {
    let (id, _) = unit.ir_module.methods.unique_display(label)?;
    unit.methods.get(&id)
}

#[test]
fn authored_fixture_lookup_keeps_actual_body_owner_without_publication() {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for("proc ::p {} {set x 1}", &registry, false);
    assert!(unit.ir_module.procedures.is_empty());
    let function = unit.function("::p").expect("original analysis body");
    let id = function.body_id.expect("actual issued body");
    assert!(std::ptr::eq(
        function,
        unit.function_for_body(Some(id))
            .expect("same stamped store")
    ));
    let procedure = unit.original_procedure_for_body(id).expect("original card");
    assert!(procedure.publication.is_none());
    assert!(procedure.body.native_compilation_admission.is_none());
    assert!(
        unit.ir_module
            .original_native_declaration_body_units
            .is_empty()
    );
    assert!(unit.function("::missing").is_none());
}

#[test]
fn authored_fixture_lookup_refuses_duplicate_reporting_labels() {
    let registry = CommandRegistry::build_default();
    let unit = CompilationUnit::build_for(
        "proc ::p {} {return FIRST}; proc ::p {} {return SECOND}",
        &registry,
        false,
    );
    assert_eq!(
        unit.ir_module
            .original_authored_declaration_body_units
            .len(),
        2
    );
    assert!(unit.function("::p").is_none());
}

#[test]
fn prepare_cfg_context_retains_same_tailed_body_parameters() {
    let reg = registry();
    let src = "namespace eval ::a { proc x {p1} { set q $p1 } }
namespace eval ::b { proc x {p2 extra} { set q $p2 } }
";
    let m = crate::lowering::lower_to_ir_with_config(src, &reg, tcl_lexer::LexerConfig::default());
    let (_, proc_params, _, _, _) = crate::cfg_builder::prepare_cfg_context(&m);
    let a = fixture_id(&m.procedures, "::a::x");
    let b = fixture_id(&m.procedures, "::b::x");
    assert_ne!(a, b);
    assert_eq!(proc_params.len(), 2);
    assert_eq!(proc_params.get(&a), Some(&vec!["p1".to_string()]));
    assert_eq!(
        proc_params.get(&b),
        Some(&vec!["p2".to_string(), "extra".to_string()])
    );
}

#[test]
fn build_for_empty_source() {
    let cu = CompilationUnit::build_for("", &registry(), false);
    assert_eq!(cu.source, "");
    assert_eq!(cu.top_level.name, "::top");
    assert!(cu.procedures.is_empty());
}

#[test]
fn prepared_binding_mutation_projection_matches_compatibility_scan() {
    let reg = registry();
    let cu = CompilationUnit::build_for(
        "rename string saved-string\nrename saved-string string\n",
        &reg,
        false,
    );
    assert_eq!(
        cu.command_mutations,
        crate::command_binding::scan_module_command_mutations(&cu.ir_module, &reg),
    );
}

/// Every method unit carries exactly one instance-state carrier
/// (`method_facts`), and its content matches the typed IR — the
/// invariant that keeps the existence fold (I230/O100/O101), the
/// W210/W211/W220 `known_bound` set, and the optimiser's method-constants
/// escaping set reading the same struct.
#[test]
fn method_units_carry_the_single_method_facts_carrier() {
    let src = "oo::class create C {\n\
                       variable state\n\
                       method m {arg} { ::return $arg }\n\
                   }\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let fu = fixture_method(&cu, "::C::m").expect("method unit built");
    let facts = fu
        .method_facts
        .as_deref()
        .expect("a method unit must carry method_facts");
    assert_eq!(facts.params, vec!["arg".to_string()]);
    assert!(facts.instance_vars.contains("state"));
    let known = facts.known_bound_at_entry();
    assert!(known.contains("arg") && known.contains("state"));
    // The IR the carrier was built from agrees (same construction site).
    let ir = cu
        .ir_module
        .methods
        .unique_display("::C::m")
        .map(|(_, method)| method)
        .expect("method IR");
    assert_eq!(facts.instance_vars, ir.instance_vars);
    assert_eq!(facts.params, ir.params);
}

/// TN: bodies with no object state — the top level and plain
/// procedures — carry no `method_facts`, so no consumer can mistake a
/// proc frame for a method frame.
#[test]
fn non_method_units_carry_no_method_facts() {
    let src = "proc p {a} { return $a }\nset x 1\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    assert!(cu.top_level.method_facts.is_none());
    let fu = fixture_id_if_present(&cu.procedures, "::p")
        .and_then(|id| cu.procedures.get(&id))
        .expect("proc unit built");
    assert!(fu.method_facts.is_none());
}

#[test]
fn build_for_single_statement() {
    let cu = CompilationUnit::build_for("set x 1", &registry(), false);
    assert!(!cu.top_level.cfg.blocks.is_empty());
    // SCCP should mark entry executable for a non-empty CFG.
    assert!(
        cu.top_level
            .sccp
            .executable_blocks
            .contains(&cu.top_level.ssa.entry)
    );
}

#[test]
fn complexity_guard_flags_byte_huge_proc() {
    // A normal proc is analysed (not guarded), with real SSA + SCCP.
    let normal = CompilationUnit::build_for("proc small {} { set x 1 }", &registry(), false);
    let small = normal.function("::small").expect("::small built");
    assert!(!small.complexity_guarded);
    assert!(!small.ssa.blocks.is_empty());

    // A block-light but byte-huge body (one statement with a ~270 KB
    // literal) trips the body-byte half of the complexity guard: the unit
    // is flagged and carries a trivial SSA, so the O(blocks·vars) walk and
    // the dataflow passes never run on it.
    let big_literal = "A".repeat(270_000);
    let src = format!("proc big {{}} {{ set x \"{big_literal}\" }}");
    let cu = CompilationUnit::build_for(&src, &registry(), false);
    let big = cu.function("::big").expect("::big built");
    assert!(big.complexity_guarded, "byte-huge body must be guarded");
    assert!(big.ssa.blocks.is_empty(), "guarded unit has trivial SSA");
    assert!(big.sccp.values.is_empty());
    // Guarded functions are excluded from the analysable-function view the
    // per-proc diagnostic and optimiser passes iterate.
    assert!(
        cu.analysable_functions().all(|fu| fu.name != "::big"),
        "guarded ::big must be skipped by analysable_functions"
    );
}

#[test]
fn build_for_captures_procedures() {
    let cu = CompilationUnit::build_for("proc greet {name} {puts $name}", &registry(), false);
    assert!(!cu.procedures.is_empty());
    assert!(cu.function("::greet").is_some());
    assert!(cu.function("::top").is_some());
    // No OO methods in a plain proc source.
    assert!(cu.methods.is_empty());
}

#[test]
fn colliding_method_reports_keep_independent_analysis_units() {
    let source = "namespace eval A {}\noo::class create A::B {method c {} {return LEFT}}\noo::class create A {method B::c {} {return RIGHT}; method <constructor> {} {return NAMED}; constructor {} {set local ACTUAL}; export B::c <constructor>}\n";
    let unit = CompilationUnit::build_for(source, &registry(), false);
    assert_eq!(unit.ir_module.methods.len(), 4);
    assert_eq!(unit.methods.len(), 4);
    for (id, method) in &unit.ir_module.methods {
        let function = unit
            .methods
            .get(id)
            .expect("every retained method has its own analysis unit");
        assert_eq!(function.body_id, Some(*id));
        assert_eq!(function.name, method.reporting_label());
    }
}

#[test]
fn build_for_lowers_oo_methods_to_function_units() {
    // TclOO method bodies get their own
    // FunctionUnit (CFG → SSA → analysis) in `cu.methods`, kept
    // separate from `procedures` so the optimiser's O126 gate can
    // iterate them. `procedures` stays free of method qnames.
    let src = "oo::class create Counter {\n\
                   \x20   variable n\n\
                   \x20   method bump {} { ::incr n }\n\
                   \x20   method get {} { ::return $n }\n\
                   }\n";
    let cu = CompilationUnit::build_for(src, &registry(), false);
    let mut keys: Vec<&String> = cu.methods.values().map(|method| &method.name).collect();
    keys.sort();
    assert_eq!(
        keys,
        vec![
            &"::Counter::bump".to_string(),
            &"::Counter::get".to_string()
        ],
        "method units: {keys:?}",
    );
    // Each method unit carries a real analysis pipeline (its CFG
    // has the entry block at minimum).
    let get = fixture_method(&cu, "::Counter::get").expect("unique retained fixture method");
    assert_eq!(get.name, "::Counter::get");
    assert!(!get.cfg.blocks.is_empty());
    // Methods are NOT mixed into the per-proc map.
    assert!(
        !cu.procedures
            .values()
            .any(|procedure| procedure.name == "::Counter::get")
    );
}

#[test]
fn runtime_selected_method_commands_guard_deep_cfg_analysis() {
    let cu = CompilationUnit::build_for(
        "interp alias {} e {} expr\n\
             oo::class create C {\n\
                 method relative {} {set x 1}\n\
                 method absolute {} {::set x 1}\n\
                 method absolute_dispatch {} {::set x [my absolute]}\n\
                 method absolute_expr {p} {::return [::expr {$p + 1}]}\n\
                 method absolute_info {p} {::if {[::info exists p]} {::puts yes}}\n\
                 method absolute_regex {s} {::set re {a+}; ::regexp $re $s}\n\
                 method nested_alias {p} {::set x [e {$p + 1}]}\n\
             }",
        &registry(),
        false,
    );
    assert!(
        fixture_method(&cu, "::C::relative")
            .expect("unique retained fixture method")
            .complexity_guarded,
        "a receiver-local command may shadow the lexical builtin"
    );
    assert!(
        !fixture_method(&cu, "::C::absolute")
            .expect("unique retained fixture method")
            .complexity_guarded,
        "an absolute-only method is independent of the receiver namespace"
    );
    assert!(
        fixture_method(&cu, "::C::absolute_dispatch")
            .expect("unique retained fixture method")
            .complexity_guarded,
        "the receiver namespace can shadow TclOO's injected dispatch commands"
    );
    for name in [
        "::C::absolute_expr",
        "::C::absolute_info",
        "::C::absolute_regex",
    ] {
        assert!(
            !fixture_method(&cu, name)
                .expect("unique retained fixture method")
                .complexity_guarded,
            "absolute-only method {name} must retain deep analysis: {:#?}",
            fixture_method(&cu, name).expect("unique retained fixture method")
        );
    }
    assert!(
        fixture_method(&cu, "::C::nested_alias")
            .expect("unique retained fixture method")
            .complexity_guarded,
        "a relative nested binding consumed by typed lowering remains runtime-selected"
    );
}

#[test]
fn functions_iterator_yields_top_plus_procs() {
    let cu = CompilationUnit::build_for(
        "proc foo {} {return 1}\nproc bar {} {return 2}",
        &registry(),
        false,
    );
    let count = cu.functions().count();
    assert_eq!(count, cu.procedures.len() + 1);
}
