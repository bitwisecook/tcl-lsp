// Temporary Q8 probe: verbatim copy of f5-cli/src/commands/explain_flow.rs
// parse_destination + canonicalise_ip + dest_re (b3f2f32c, :143-166, :813-815),
// and the public f5_xc::names::xc_object_name.
#![allow(clippy::all, clippy::pedantic, missing_docs)]
use regex::Regex;
fn parse_destination(dest: &str, re: &Regex) -> Option<(String, u32)> {
    let caps = re.captures(dest.trim())?;
    let addr = caps
        .name("addr")?
        .as_str()
        .trim_matches(|c| c == '[' || c == ']');
    let port_raw = caps.name("port")?.as_str();
    let canonical = canonicalise_ip(addr)?;
    let port = if port_raw == "any" {
        0
    } else {
        port_raw.parse::<u32>().ok()?
    };
    Some((canonical, port))
}
fn canonicalise_ip(addr: &str) -> Option<String> {
    addr.parse::<std::net::IpAddr>()
        .ok()
        .map(|ip| ip.to_string())
}

#[test]
#[ignore = "Q8 probe"]
fn q8_f5cli() {
    let d = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/work/name-handling/handover/oo-dispatch-probe/bigip-addresses");
    let re = Regex::new(
        r"^(?P<path>/[^/\s]+/)?(?P<addr>\[[^\]]+\]|[0-9a-fA-F\.:]+)(?:%\d+)?:(?P<port>\d+|any)$",
    ).unwrap();
    let text = std::fs::read_to_string(d.join("inputs.tsv")).unwrap();
    let mut out = String::from("id\tinput\tfunction\tverdict\tparts\n");
    for l in text.lines().skip(1) {
        let f: Vec<&str> = l.split('\t').collect();
        let r = parse_destination(f[2], &re);
        out.push_str(&format!("{}\t{}\tf5-cli::explain_flow::parse_destination\t{}\t{:?}\n", f[0], f[2], if r.is_some() {"ok"} else {"err"}, r));
    }
    std::fs::write(d.join("matrix-f5-cli.tsv"), out).unwrap();
}
