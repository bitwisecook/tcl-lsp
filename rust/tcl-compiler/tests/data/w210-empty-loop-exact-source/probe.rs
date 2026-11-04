use tcl_compiler::analyser::Analyser;
use tcl_compiler::analyser::types::DiagCode;
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_compiler::ssa::SsaSourceView;

fn main() {
    for (name, source) in [
        ("exact", "proc p {} {\n    foreach x {} {}\n    if {[info exists x]} { puts yes }\n    puts $x\n}\nforeach y {} {}\nputs $y\n"),
        ("proc_only", "proc p {} {foreach x {} {}; if {[info exists x]} {puts yes}; puts $x}"),
        ("top_error_without_guard", "proc p {} {foreach x {} {}; puts $x}; foreach y {} {}; puts $y"),
        ("top_safe", "proc p {} {foreach x {} {}; if {[info exists x]} {puts yes}; puts $x}; foreach y {} {}; puts safe"),
        ("top_other_error", "proc p {} {foreach x {} {}; if {[info exists x]} {puts yes}; puts $x}; error fail"),
        ("called_before_y", "proc p {} {foreach x {} {}; if {[info exists x]} {puts yes}; puts $x}; catch p; foreach y {} {}; puts $y"),
    ] {
        eprintln!("CASE {name}");
        let mut analyser = Analyser::new();
        let result = analyser.analyse(source, "tcl8.6");
        let warnings: Vec<_> = result.diagnostics.iter().filter(|row| row.code == DiagCode::W210).map(|row| (row.span.start(), row.span.end())).collect();
        println!("CASE {name} W210 {warnings:?}");
        let input = analyser.resolved_analysis_input();
        let settings = analyser.source_compilation_settings();
        let registry = input.borrowed_context_registry().commands();
        let unit = CompilationUnit::build_with_analysis_input_and_source_settings(source, UnitBuildOptions {
            registry, defer_top_level: false, config: input.lexer_config(), dialect: Some(input.unit_profile()), external_call_sites: None, declared_commands: None,
        }, None, &input, &settings);
        for (number, fu) in unit.analysable_body_function_units().enumerate() {
            println!("FU {number} point_contexts={} sccp_executable={:?}", fu.ssa.point_contexts.is_some(), fu.sccp.executable_blocks);
            let opaque_names = tcl_compiler::interprocedural::collect_positioned_opaque_callee_name_args(&fu.cfg, &|statement| statement.tokens().and_then(|tokens| tokens.source_binding.as_ref()).and_then(|binding| binding.proved_execution_target()).is_some());
            println!("OPAQUE_NAMES {opaque_names:?}");
            for (&id, block) in &fu.ssa.blocks {
                println!("BLOCK {id:?} idom={:?} semantic_reaches={:?}", fu.ssa.idom.get(&id), fu.semantic_values().map(|facts| facts.reaches(id)));
                for index in (0..block.statements.len()).chain(std::iter::once(usize::MAX)) {
                    let view = SsaSourceView::at_statement(&fu.ssa, id, index);
                    let Some(tokens) = view.source_tokens() else { continue; };
                    let binding = tokens.source_binding.as_ref();
                    println!("STMT {index} words={:?} purpose={:?} target={:?}", tokens.argv, binding.map(|v| v.runtime_reachability()), binding.and_then(|v| v.proved_execution_target()).map(|v| v.command.as_str()));
                    for access in &tokens.variable_accesses {
                        println!("READ {:?} span={:?} ref={:?} presence={:?}", access.original_spelling, fu.abs_span(access.source.span), view.read_reference(&access.source, &access.original_spelling), view.read_contents_presence_alternatives_at(&access.source, &access.original_spelling, registry));
                    }
                }
            }
        }
    }
}
