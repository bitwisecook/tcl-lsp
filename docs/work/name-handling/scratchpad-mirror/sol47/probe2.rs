
#[test]
fn probe_cluster_b() {
    use tcl_runtime_api::CompileService;
    let profile = tcl_registry::model::resolve_environment("tcl9.0").unit_profile();
    for src in ["set x 1\nputs $x\n", "proc f {} {return 1}\nf\n", "if {1} {\nset done 1\n}\n"] {
        let svc = crate::compile_service::BytecodeCompileService::for_profile(profile);
        let m = svc.compile(src);
        match m {
            Ok(m) => eprintln!(
                "B {src:?} top={:?} preflight={:?} prereq={}",
                m.top_level.validate_native_compilation_entry(),
                m.top_level.native_compilation_preflight,
                m.top_level.native_compiler_prerequisites.len()
            ),
            Err(e) => eprintln!("B {src:?} ERR {e}"),
        }
        let m = svc.compile_for_profile(src, profile);
        eprintln!("B4P {src:?} {:?}", m.map(|m| m.top_level.validate_native_compilation_entry()));
    }
}

#[test]
fn probe_nested_if_timing() {
    use tcl_runtime_api::CompileService;
    let profile = tcl_registry::model::resolve_environment("tcl9.0").unit_profile();
    for depth in [2usize, 4, 6, 8, 10, 12] {
        let mut src = String::new();
        for _ in 0..depth {
            src.push_str("if {1} {\n");
        }
        src.push_str("set done 1\n");
        for _ in 0..depth {
            src.push_str("}\n");
        }
        let t = std::time::Instant::now();
        let svc = crate::compile_service::BytecodeCompileService::for_profile(profile);
        let ok = svc.compile(&src).is_ok();
        eprintln!("NEST depth={depth} ok={ok} {:?}", t.elapsed());
    }
}
