// Temporary Q8 probe. Verbatim copies (marked) of private helpers at b3f2f32c
// plus the full collect_model pipeline over addresses.conf.
#![allow(clippy::all, clippy::pedantic, missing_docs, dead_code)]
use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../docs/work/name-handling/handover/oo-dispatch-probe/bigip-addresses")
}

// ---- verbatim: src/graph.rs split_rd, mask_to_prefix, is_any, normalise_cidr
fn split_rd(addr: &str) -> (String, i64) {
    if let Some((base, rd)) = addr.split_once('%') {
        // rd = re.split(r"[:./]", rd)[0]
        let rd_head: &str = rd.split([':', '.', '/']).next().unwrap_or("");
        let n = rd_head.parse::<i64>().unwrap_or(0);
        (base.to_string(), n)
    } else {
        (addr.to_string(), 0)
    }
}
fn mask_to_prefix(mask: &str) -> Option<i64> {
    let mask = mask.trim();
    if mask.is_empty() {
        return None;
    }
    if mask.chars().all(|c| c.is_ascii_digit()) {
        return mask.parse::<i64>().ok();
    }
    if mask.contains(':') {
        // IPv6 mask is unusual; fall back to counting bits.
        let mut bits = 0u32;
        for b in mask.split(':') {
            if b.is_empty() {
                continue;
            }
            let v = u32::from_str_radix(b, 16).ok()?;
            bits += v.count_ones();
        }
        return Some(i64::from(bits));
    }
    // IPv4 dotted netmask -> prefix length (bit popcount, matching a valid
    // contiguous mask's prefixlen).
    let v4 = Ipv4Addr::from_str(mask).ok()?;
    Some(i64::from(u32::from(v4).count_ones()))
}
fn is_any(addr: &str) -> bool {
    matches!(addr, "" | "any" | "0.0.0.0" | "::" | "0.0.0.0/0" | "::/0")
}
fn normalise_cidr(s: &str) -> String {
    let s = s.split('%').next().unwrap_or("");
    if s.contains('/') {
        s.to_string()
    } else if s.contains(':') {
        format!("{s}/128")
    } else {
        format!("{s}/32")
    }
}
// ---- verbatim: src/enrich.rs to_ip
fn to_ip(addr: &str) -> Option<IpAddr> {
    let a = addr.trim();
    if a.is_empty() {
        return None;
    }
    let a = a.split('%').next().unwrap_or(a);
    let a = a.split('/').next().unwrap_or(a);
    if let Ok(ip) = IpAddr::from_str(a) {
        return Some(ip);
    }
    // IPv4 `host:port`.
    if let Some((host, _)) = a.rsplit_once(':')
        && !host.contains(':')
        && let Ok(ip) = IpAddr::from_str(host)
    {
        return Some(ip);
    }
    None
}
// ---- verbatim: src/model.rs split_dest
fn split_dest(dest: &str) -> (String, String) {
    if dest.is_empty() {
        return (String::new(), String::new());
    }
    let leaf = dest.rsplit('/').next().unwrap_or(dest);
    let colons = leaf.matches(':').count();
    if colons >= 2 {
        // IPv6: address holds the colons, port after a dot.
        if let Some(idx) = leaf.rfind('.') {
            return (leaf[..idx].to_string(), leaf[idx + 1..].to_string());
        }
        return (leaf.to_string(), String::new());
    }
    if let Some(idx) = leaf.rfind(':') {
        return (leaf[..idx].to_string(), leaf[idx + 1..].to_string());
    }
    (leaf.to_string(), String::new())
}

#[test]
#[ignore = "Q8 probe"]
fn q8_matrix_report_gen() {
    let text = std::fs::read_to_string(dir().join("inputs.tsv")).unwrap();
    let mut out = String::from("id\tinput\tfunction\tverdict\tparts\n");
    for l in text.lines().skip(1) {
        let f: Vec<&str> = l.split('\t').collect();
        let (id, s) = (f[0], f[2]);
        let (a, rd) = split_rd(s);
        out.push_str(&format!("{id}\t{s}\tbigip-report-gen::graph::split_rd\tok\taddr={a:?} rd={rd}\n"));
        out.push_str(&format!("{id}\t{s}\tbigip-report-gen::graph::mask_to_prefix\tok\t{:?}\n", mask_to_prefix(s)));
        out.push_str(&format!("{id}\t{s}\tbigip-report-gen::graph::is_any\tok\t{}\n", is_any(s)));
        out.push_str(&format!("{id}\t{s}\tbigip-report-gen::graph::normalise_cidr\tok\t{:?}\n", normalise_cidr(s)));
        out.push_str(&format!("{id}\t{s}\tbigip-report-gen::enrich::to_ip\tok\t{:?}\n", to_ip(s)));
        let (a, p) = split_dest(s);
        out.push_str(&format!("{id}\t{s}\tbigip-report-gen::model::split_dest\tok\taddr={a:?} port={p:?}\n"));
    }
    std::fs::write(dir().join("matrix-report-gen.tsv"), out).unwrap();
}

#[test]
#[ignore = "Q8 probe"]
fn q8_config_report_gen() {
    let src = std::fs::read_to_string(dir().join("addresses.conf")).unwrap();
    let m = bigip_report_gen_rust::collect_model(&[("addresses.conf".to_owned(), src)], "Q8");
    std::fs::write(dir().join("config-report-gen.json"), serde_json::to_string_pretty(&m).unwrap()).unwrap();
}
