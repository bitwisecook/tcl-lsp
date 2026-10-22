// TEMP probe (compiler-codegen), remove before hand-back.
use std::rc::Rc;
use tcl_registry::pack_hooks;
use tcl_spec_hooks::tclvm_host;
use tcl_spectcl::discovery::{Origin, PackFile, Tier};
use tcl_spectcl::hooks;

#[test]
fn zz_ccg_fold_probe() {
    let source = "\nspeclib mylib 1 {\n    command mylib::strlen {\n        arity 1\n        arg 0 -role Value\n        const_fold -inputs {words} {words ctx} {\n            fold [string length [lindex $words 0]]\n        }\n    }\n}\n";
    let dir = std::env::temp_dir().join(format!("zz-ccg-fold-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mylib.tclspec");
    std::fs::write(&path, source).unwrap();
    let packs = tcl_spectcl::pack::load(&[PackFile { tier: Tier::Workspace, path, origin: Origin::DotDir, dependency_tier: None }]);
    eprintln!("ZZP notices {:?}", packs.notices);
    let plan = hooks::plan_for(&packs);
    let host = Rc::new(tclvm_host());
    for programs in plan.packs() {
        for installed in host.install_pack_hooks(programs.clone()) {
            eprintln!("ZZP installed slot={:?} declined={:?}", installed.slot, installed.declined);
        }
    }
    pack_hooks::install_host(host);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &packs);
    eprintln!("ZZP profile {:?}", registry.profile().map(|p| p.name));
    match registry.get("mylib::strlen") {
        Some(spec) => eprintln!(
            "ZZP spec const_fold={} versioned={} run={:?} run86={:?}",
            spec.const_fold.is_some(),
            spec.const_fold_versioned.is_some(),
            spec.run_const_fold(&["abcde"], None),
            spec.run_const_fold(&["abcde"], Some(tcl_dialect::TclVersion::V8_6)),
        ),
        None => eprintln!("ZZP spec missing"),
    }
    eprintln!("ZZP resolve own={:?} none={:?}",
        registry.resolve_call("mylib::strlen", &["abcde"], registry.own_surface_query()).map(|r| r.spec.name),
        registry.resolve_call("mylib::strlen", &["abcde"], None).map(|r| r.spec.name));
    let pack_spec = registry.get("mylib::strlen").unwrap().clone();
    let dbg = format!("{pack_spec:?}");
    eprintln!("ZZP specdbg {}", &dbg[..dbg.len().min(6000)]);
    let mut b = tcl_registry::CommandRegistry::build_default();
    b.insert(pack_spec.clone());
    let mut c = tcl_registry::CommandRegistry::build_default();
    c.insert(tcl_registry::spec::CommandSpec {
        name: "mylib::strlen",
        arity: tcl_registry::arity::Arity::exact(1),
        const_fold: pack_spec.const_fold,
        ..tcl_registry::spec::CommandSpec::DEFAULT
    });
    let mut d = tcl_registry::CommandRegistry::build_default();
    let mut pure_spec = pack_spec.clone();
    pure_spec.world_effects = Some(tcl_registry::world_effect::WorldEffectDescriptor::EMPTY);
    d.insert(pure_spec);
    for (label, reg) in [("B-default+packspec", &b), ("C-default+minimal", &c), ("D-default+packspec+pure", &d)] {
        for dialect in [None, Some("tcl8.6")] {
            let found: Vec<(String, String)> = tcl_compiler::optimiser::optimise_raw("proc ::f {} {\n    set y [mylib::strlen abcde]\n}\n", reg, dialect)
                .into_iter().map(|o| (o.code.as_str().to_owned(), o.replacement)).collect();
            eprintln!("ZZP {label} {dialect:?}: {found:?}");
        }
    }
    for dialect in [None, Some("tcl8.6")] {
        for src in ["proc ::f {} {\n    set y [mylib::strlen abcde]\n}\n", "set y [mylib::strlen abcde]\n", "set y [string length abcde]\n"] {
            let found: Vec<(String, String)> = tcl_compiler::optimiser::optimise_raw(src, &registry, dialect)
                .into_iter().map(|o| (o.code.as_str().to_owned(), o.replacement)).collect();
            eprintln!("ZZP opt {dialect:?} {src:?}: {found:?}");
        }
    }
    pack_hooks::clear_host();
    let _ = std::fs::remove_dir_all(&dir);
    let source = "\nspeclib mylib 1 {\n    command mylib::strlen {\n        arity 1\n        arg 0 -role Value\n        world_effects none\n        const_fold -inputs {words} {words ctx} {\n            fold [string length [lindex $words 0]]\n        }\n    }\n}\n";
    let dir = std::env::temp_dir().join(format!("zz-ccg-fold2-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("mylib.tclspec");
    std::fs::write(&path, source).unwrap();
    let packs = tcl_spectcl::pack::load(&[PackFile { tier: Tier::Workspace, path, origin: Origin::DotDir, dependency_tier: None }]);
    eprintln!("ZZP2 notices {:?}", packs.notices);
    let plan = hooks::plan_for(&packs);
    let host = Rc::new(tclvm_host());
    for programs in plan.packs() {
        for installed in host.install_pack_hooks(programs.clone()) {
            eprintln!("ZZP2 installed declined={:?}", installed.declined);
        }
    }
    pack_hooks::install_host(host);
    let registry = tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &packs);
    for dialect in [None, Some("tcl8.6")] {
        let found: Vec<(String, String)> = tcl_compiler::optimiser::optimise_raw("proc ::f {} {\n    set y [mylib::strlen abcde]\n}\n", &registry, dialect)
            .into_iter().map(|o| (o.code.as_str().to_owned(), o.replacement)).collect();
        eprintln!("ZZP2 world_effects-none {dialect:?}: {found:?}");
    }
    pack_hooks::clear_host();
    let _ = std::fs::remove_dir_all(&dir);
}
