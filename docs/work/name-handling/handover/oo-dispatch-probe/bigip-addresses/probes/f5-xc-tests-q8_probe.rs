// Temporary Q8 probe: f5_xc::names::xc_object_name over every input.
#![allow(clippy::all, clippy::pedantic, missing_docs)]
#[test]
#[ignore = "Q8 probe"]
fn q8_xc_names() {
    let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/work/name-handling/handover/oo-dispatch-probe/bigip-addresses");
    let text = std::fs::read_to_string(d.join("inputs.tsv")).unwrap();
    let mut out = String::from("id\tinput\tfunction\tverdict\tparts\n");
    for l in text.lines().skip(1) {
        let f: Vec<&str> = l.split('\t').collect();
        out.push_str(&format!("{}\t{}\tf5_xc::names::xc_object_name\tok\t{:?}\n", f[0], f[2], f5_xc::names::xc_object_name(f[2])));
    }
    std::fs::write(d.join("matrix-f5-xc.tsv"), out).unwrap();
}
