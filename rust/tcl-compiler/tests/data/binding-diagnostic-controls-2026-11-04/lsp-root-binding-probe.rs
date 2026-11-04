use std::io::Read;
use tcl_compiler::analyser::{Analyser, ResolvedAnalysisInput};
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_compiler::ssa::SsaSourceView;
use tcl_lexer::LexerConfig;
use tcl_registry::model::ingress::resolve_environment;
fn main() {
    let dialect = std::env::args().nth(1).expect("dialect");
    let mut source = String::new();
    std::io::stdin().read_to_string(&mut source).unwrap();
    let env = resolve_environment(&dialect);
    let profile = env.analyser_profile();
    let input = ResolvedAnalysisInput::new(profile, profile, env.default_context_registry(), LexerConfig::for_file_grammar(profile.grammar));
    let mut analyser = Analyser::new().with_resolved_input(input.clone());
    let result = analyser.analyse(&source, &dialect);
    println!("INPUT resolved={} unavailable={}", result.resolved_input.is_some(), result.analysis_context_unavailable.is_some());
    for d in result.diagnostics { println!("A {} {} {} {:?}", d.code, d.span.start(), d.span.end(), d.message); }
    let settings = analyser.source_compilation_settings();
    let registry = input.borrowed_context_registry().commands();
    let unit = CompilationUnit::build_with_analysis_input_and_source_settings(&source, UnitBuildOptions {
        registry, defer_top_level: false, config: input.lexer_config(), dialect: Some(input.unit_profile()), external_call_sites: None, declared_commands: None,
    }, None, &input, &settings);
    println!("UNIT procedures={}", unit.procedures.len());
    for d in tcl_compiler::compiler_checks::run_all_checks(&unit, registry, Some(input.analyser_profile())) {println!("C {} {} {} {:?}",d.code,d.span.start(),d.span.end(),d.message);}
    for (i,fu) in unit.analysable_body_function_units().enumerate() {
        println!("FU {i} blocks={}",fu.ssa.blocks.len());
        for (&block, b) in &fu.ssa.blocks {
            for at in (0..b.statements.len()).chain(std::iter::once(usize::MAX)) {
                let view = SsaSourceView::at_statement(&fu.ssa, block, at);
                let Some(tokens) = view.source_tokens() else {continue;};
                for access in &tokens.variable_accesses {println!("READ {:?} {:?} {:?}", access.original_spelling,fu.abs_span(access.source.span),view.read_reference(&access.source,&access.original_spelling));}
            }
        }
    }
}
