use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::types::TclType;
use tcl_registry::model::ingress::static_context_for;

fn inferred_result(profile_name: &str, function: &str) -> TclType {
    let registry = static_context_for(profile_name).commands();
    let profile = registry.profile().expect("profile-backed registry");
    let source = format!("set x 1.0\nset z [expr {{{function}($x)}}]");
    let unit = CompilationUnit::build_for(&source, registry, false);
    eprintln!("source={source:?} top types={:?}", unit.top_level.types.iter().map(|((sym, ver), ty)| (unit.top_level.ssa.var_name(*sym), ver, ty)).collect::<Vec<_>>());
    let results = unit.top_level.types.iter().filter_map(|((symbol, version), value)| {
        (unit.top_level.ssa.var_name(*symbol) == "z")
            .then(|| (*version, value.tcl_type()))
    });
    results.max_by_key(|(version, _)| *version).and_then(|(_, kind)| kind)
        .unwrap_or_else(|| panic!("no Tcl type for {profile_name} {function}"))
}

#[test]
fn public_compilation_type_inference_uses_shared_math_result_classes() {
    for (profile, name, expected) in [
        ("tcl8.4", "ceil", TclType::Double),
        ("tcl8.4", "int", TclType::Int),
        ("tcl9.0", "isnan", TclType::Boolean),
        ("tcl9.1", "signbit", TclType::Boolean),
        ("tcl9.1", "trunc", TclType::Double),
        ("jim", "ceil", TclType::Double),
    ] {
        let got = inferred_result(profile, name);
        assert_eq!(got, expected, "{profile} {name}");
    }
}
