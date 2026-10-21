// Temporary vm-runtime probe; deleted after diagnosis.
use std::rc::Rc;
use tcl_registry::pack_hooks;
use tcl_spec_hooks::tclvm_host;
use tcl_spectcl::discovery::{Origin, PackFile, Tier};
use tcl_spectcl::hooks;

const SRC: &str = "\nspeclib mylib 1 {\n    command mylib::strlen {\n        arity 1\n        arg 0 -role Value\n        const_fold -inputs {words} {words ctx} {\n            set subject [lindex $words 0]\n            if {![string is ascii $subject]} { return }\n            fold [string length $subject]\n        }\n    }\n}\n";

#[test]
fn zz_probe() {
    let dir = std::env::temp_dir().join(format!("zz-vmrt-ps-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mylib.tclspec");
    std::fs::write(&path, SRC).unwrap();
    let packs = tcl_spectcl::pack::load(&[PackFile {
        tier: Tier::Workspace,
        path,
        origin: Origin::DotDir,
        dependency_tier: None,
    }]);
    eprintln!("notices {:?}", packs.notices);
    let plan = hooks::plan_for(&packs);
    let host = Rc::new(tclvm_host());
    for programs in plan.packs() {
        for p in &programs.programs {
            eprintln!("program {p:?}");
        }
        let installed = host.install_pack_hooks(programs.clone());
        eprintln!("installed {installed:?}");
    }
    pack_hooks::install_host(host.clone());
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &packs);
    let spec = registry.get("mylib::strlen");
    eprintln!("spec const_fold {:?}", spec.map(|s| format!("{:?}", s.const_fold)));
    let opts = tcl_compiler::optimiser::manager::optimise_raw(
        "proc ::f {} {\n    set y [mylib::strlen abcde]\n}\n",
        &registry,
        None,
    );
    for o in &opts {
        eprintln!("opt {} {:?}", o.code.as_str(), o.replacement);
    }
    eprintln!("errors {:?}", host.error_log());
    eprintln!("crashes {:?}", host.crash_records());
    pack_hooks::clear_host();

    use tcl_engine_api::Engine;
    for body in [
        "set r [string cat a - b]\nreturn $r",
        "return [string length abc]",
        "set s abc\nif {![string is ascii $s]} { return }\nreturn [string length $s]",
        "return [lindex {a b} 0]",
        "return [expr {1 + 2}]",
        "return ok",
    ] {
        let mut engine = tcl_engine_tclvm::TclVmEngine::new();
        let unit = tcl_engine_api::CompileUnit { name: "probe", parameters: &[], body };
        let r = engine
            .compile(unit)
            .and_then(|h| engine.invoke(&h, &[]));
        eprintln!("engine {body:?} => {r:?}");
    }
    use tcl_runtime_api::CompileService;
    for profile in ["tcl8.6", "tcl9.0"] {
        let profile = tcl_dialect::DialectProfile::find(profile).unwrap();
        for body in ["string cat a - b", "string length abc", "lindex {a b} 0", "set x 1"] {
            let params: Vec<String> = Vec::new();
            let r = tcl_compiler::compile_service::BytecodeCompileService::default()
                .compile_procedure_for_profile(
                    tcl_runtime_api::ProcedureCompileTarget { source: body, parameters: &params, namespace: "" },
                    profile,
                    tcl_runtime_api::ProcedureDispatch::Optimised,
                );
            match r {
                Ok(m) => eprintln!(
                    "proc {} {body:?}: preflight={:?} prereqs={:?} validate={:?}",
                    profile.name,
                    m.top_level.native_compilation_preflight,
                    m.top_level.native_compiler_prerequisites,
                    m.top_level.validate_native_compilation_entry()
                ),
                Err(e) => eprintln!("proc {} {body:?}: ERR {e:?}", profile.name),
            }
        }
    }
}

// Writes every shipped pack's `golden::render` beside its golden name, exactly
// as `cargo xtask pack-goldens` renders it.
#[test]
fn zz_goldens() {
    use tcl_spectcl::golden;
    let Some(out) = std::env::var_os("ZZ_OUT") else { return };
    let out = std::path::PathBuf::from(out);
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::fs::create_dir_all(&out).unwrap();
    for path in golden::shipped_packs(&root) {
        let source = std::fs::read_to_string(&path).unwrap();
        let fresh = golden::render(&tcl_spectcl::evaluate_pack(&source));
        let golden_path = golden::golden_path(&root, &path);
        std::fs::write(out.join(golden_path.file_name().unwrap()), &fresh).unwrap();
        let pack = tcl_spectcl::evaluate_pack(&source);
        let recorded = std::fs::read_to_string(&golden_path).unwrap_or_default();
        let mut renders = String::new();
        for (a, b) in recorded.lines().zip(fresh.lines()) {
            if a != b
                && let Some(cmd) = b.strip_prefix("command ").and_then(|r| r.split(' ').next())
                && let Some(r) = golden::spec_rendering(&pack, cmd)
            {
                renders.push_str(cmd);
                renders.push('\t');
                renders.push_str(&r);
                renders.push('\n');
            }
        }
        std::fs::write(out.join(golden_path.file_name().unwrap()).with_extension("renders"), renders).unwrap();
    }
}
