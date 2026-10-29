//! TEMPORARY root probe for procedure-body resolution; delete after diagnosis.
use tcl_compiler::optimiser::manager::optimise_with_dialect;

#[test]
fn zz_root_proc_probe() {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for src in [std::env::var("ZZSRC").unwrap_or_else(|_| "proc p {s} { set a 1; regsub {x} $s y a; puts $a }
".into()).as_str()] {
        let codes: Vec<_> = optimise_with_dialect(src, registry, Some(profile))
            .iter()
            .map(|o| o.code.as_str().to_owned())
            .collect();
        eprintln!("ZZROOT src={src:?} codes={codes:?}");
    }
}
