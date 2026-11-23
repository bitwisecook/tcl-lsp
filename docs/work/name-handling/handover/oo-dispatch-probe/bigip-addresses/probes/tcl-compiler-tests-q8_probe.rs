// Temporary Q8 probe: iRules address snippets through the analyser.
#![allow(clippy::all, clippy::pedantic, missing_docs)]
use tcl_compiler::analyser::Analyser;

#[test]
#[ignore = "Q8 probe"]
fn q8_irules_addresses() {
    let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/work/name-handling/handover/oo-dispatch-probe/bigip-addresses");
    let text = std::fs::read_to_string(d.join("irules-snippets.tsv")).unwrap();
    let mut out = String::from("id\tbody\tcode\tmessage\n");
    for l in text.lines().skip(1) {
        let (id, body) = l.split_once('\t').unwrap();
        let src = format!("when HTTP_REQUEST {{\n    {body}\n}}\n");
        let diags = Analyser::new().analyse(&src, "f5-irules").diagnostics;
        if diags.is_empty() {
            out.push_str(&format!("{id}\t{body}\t-\t-\n"));
        }
        for dg in diags {
            out.push_str(&format!("{id}\t{body}\t{:?}\t{}\n", dg.code, dg.message.replace(['\t', '\n'], " ")));
        }
    }
    std::fs::write(d.join("analyser-irules.tsv"), out).unwrap();
}
