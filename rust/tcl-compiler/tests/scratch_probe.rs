use tcl_compiler::optimiser::manager::optimise_source_multipass;
use tcl_registry::registry_for_dialect;
#[test] fn p5() {
    let r = registry_for_dialect("tcl8.6");
    let (out, opts) = optimise_source_multipass("set a 1\nset a 5\nset b [expr {$a + 2}]", r, Some("tcl8.6"), 10);
    println!("[MP] reassign: OUT={:?} CODES={:?}", out, opts.iter().map(|o|o.code.as_str().to_owned()).collect::<Vec<_>>());
    let (out2, _) = optimise_source_multipass("set a 1\nset a 5\nset b [expr {$a + 2}]\nputs $b", r, Some("tcl8.6"), 10);
    println!("[MP] reassign_puts: OUT={:?}", out2);
}
