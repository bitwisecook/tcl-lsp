use std::collections::HashSet;
use tcl_compiler::analyser::{Analyser, diagnostics::BodyFrame, types::DiagCode};
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};

fn spans(analyser: &Analyser) -> Vec<(u32, u32)> {
    let mut rows = analyser.result.diagnostics.iter().filter(|row| row.code == DiagCode::W210).map(|row| (row.span.start(), row.span.end())).collect::<Vec<_>>();
    rows.sort_unstable();
    rows.dedup();
    rows
}

fn main() {
    for (label, source) in [
        ("exact", "proc p {} {\n    foreach x {} {}\n    if {[info exists x]} { puts yes }\n    puts $x\n}\nforeach y {} {}\nputs $y\n"),
        ("guarded-only", "proc p {} {foreach x {} {}; if {[info exists x]} {puts $x}; return quiet}; error fail"),
        ("guarded-later-read", "proc p {} {foreach x {} {}; if {[info exists x]} {puts $x}; puts $x}; error fail"),
    ] {
        let mut analyser = Analyser::new();
        let result = analyser.analyse(source, "tcl8.6");
        analyser.result = result;
        println!("BASELINE {label} {:?}", spans(&analyser));
        let input = analyser.resolved_analysis_input();
        let settings = analyser.source_compilation_settings();
        let registry = input.borrowed_context_registry().commands();
        let unit = CompilationUnit::build_with_analysis_input_and_source_settings(source, UnitBuildOptions {
            registry, defer_top_level: false, config: input.lexer_config(), dialect: Some(input.unit_profile()), external_call_sites: None, declared_commands: None,
        }, None, &input, &settings);
        for (id, fu) in &unit.procedures {
            let procedure = unit.ir_module.procedures.get(id).unwrap();
            analyser.emit_cfg_ssa_diagnostics_for_function_with_extra(fu, BodyFrame::Procedure(procedure), &HashSet::new());
        }
        println!("CONTROL_WITHOUT_OPAQUE_NAME_ADVICE {label} {:?}", spans(&analyser));
    }
}
