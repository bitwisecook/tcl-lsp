#[path="/workspace/tcl-lsp/rust/tcl-registry/src/simulator_mock_name.rs"]
mod simulator_mock_name;
fn main() {
    let bytes = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let labels: std::collections::BTreeSet<&str> = std::str::from_utf8(&bytes).unwrap().split('\0').filter(|s| !s.is_empty()).collect();
    println!("# Authored simulator roster projection; regenerate with cargo xtask gen-irule-test-data.\nnamespace eval ::itest::cmd {{\n    variable _mock_names\n    array set _mock_names {{");
    for label in labels {
        if let Some(name) = simulator_mock_name::SimulatorMockName::for_label(label) {
            println!("        {} {}", tcl_syntax::list::list_element(label), tcl_syntax::list::list_element(&format!("::itest::cmd::{}",name.suffix())));
        }
    }
    println!("    }}\n}}");
}
