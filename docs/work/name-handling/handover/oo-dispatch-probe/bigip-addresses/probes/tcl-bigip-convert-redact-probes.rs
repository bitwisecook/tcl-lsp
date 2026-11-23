// Appended temporarily to the `mod tests` of rust/tcl-bigip/src/convert.rs and
// rust/tcl-bigip/src/redact.rs (reverted after the run), each with these helpers.
    fn q8_inputs() -> Vec<(String, String)> {
        let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/work/name-handling/handover/oo-dispatch-probe/bigip-addresses");
        std::fs::read_to_string(d.join("inputs.tsv")).unwrap().lines().skip(1)
            .map(|l| { let f: Vec<&str> = l.split('\t').collect(); (f[0].to_owned(), f[2].to_owned()) })
            .collect()
    }
    fn q8_write(name: &str, out: &str) {
        let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/work/name-handling/handover/oo-dispatch-probe/bigip-addresses");
        std::fs::write(d.join(name), out).unwrap();
    }
    // convert.rs
    #[test]
    #[ignore = "Q8 probe"]
    fn q8_probe_convert() {
        let mut out = String::from("id\tinput\tfunction\tverdict\tparts\n");
        for (id, s) in q8_inputs() {
            let (a, p) = split_destination(&s);
            out.push_str(&format!("{id}\t{s}\ttcl_bigip::convert::split_destination\tok\taddr={a:?} port={p:?}\n"));
            out.push_str(&format!("{id}\t{s}\ttcl_bigip::convert::short_name\tok\tname={:?}\n", short_name(&s)));
        }
        q8_write("matrix-tcl-bigip-convert.tsv", &out);
    }
    // redact.rs
    #[test]
    #[ignore = "Q8 probe"]
    fn q8_probe_redact() {
        let mut out = String::from("id\tinput\tfunction\tverdict\tparts\n");
        for (id, s) in q8_inputs() {
            let (v4, v6) = collect_addresses(&s, true);
            out.push_str(&format!("{id}\t{s}\ttcl_bigip::redact::collect_addresses\tok\tv4={v4:?} v6={v6:?}\n"));
        }
        q8_write("matrix-tcl-bigip-redact.tsv", &out);
    }
