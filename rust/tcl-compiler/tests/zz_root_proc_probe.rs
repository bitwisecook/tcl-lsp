//! TEMPORARY root probe for procedure-body resolution; delete after diagnosis.
use tcl_compiler::compilation_unit::CompilationUnit;

#[test]
fn zz_root_proc_probe() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for src in [
        "proc p {} {\n    set y 1\n    puts hi\n}\n",
        "proc p {} { set a 1; set a 2; puts $a }\np\n",
    ] {
        let cu = CompilationUnit::build_for_profile(src, registry, false, profile);
        eprintln!("ZZROOT src={src:?} procs={}", cu.procedures.len());
        eprintln!("ZZROOT top={:#?}", cu.ir_module.top_level);
        eprintln!("ZZROOT irprocs={} body_units={}", cu.ir_module.procedures.len(), cu.ir_module.body_units.len());
        for (name, proc_) in cu.ir_module.procedures.iter().chain(cu.ir_module.body_units.iter()) {
            eprintln!("ZZROOT proc {name:?} body={:#?}", proc_.body);
        }
    }
}
