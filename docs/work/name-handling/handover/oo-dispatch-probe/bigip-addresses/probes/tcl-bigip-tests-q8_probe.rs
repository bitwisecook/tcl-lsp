// Temporary Q8 probe (docs/work/name-handling/bigip-addresses-design.md).
// Copy kept in docs/work/name-handling/handover/oo-dispatch-probe/bigip-addresses/.
#![allow(clippy::all, clippy::pedantic, missing_docs)]

use std::fmt::Write as _;
use std::path::PathBuf;

use tcl_bigip::model::ModelObject;
use tcl_bigip::object_names::{
    EmptyServiceSuffix, object_path_partition, owner_partition_reference_paths,
    reference_without_numeric_service,
};
use tcl_bigip::value::{
    Address, Destination, FQDN, Folder, IPAddress, IPRange, Network, ObjectPath, Partition, Port,
    PortSet, RouteDomain, parse_address,
};

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/work/name-handling/handover/oo-dispatch-probe/bigip-addresses")
}

fn inputs() -> Vec<(String, String)> {
    let text = std::fs::read_to_string(dir().join("inputs.tsv")).unwrap();
    text.lines()
        .skip(1)
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f[0].to_owned(), f[2].to_owned())
        })
        .collect()
}

fn fam(a: &std::net::IpAddr) -> &'static str {
    if a.is_ipv4() { "v4" } else { "v6" }
}

fn addr_parts(a: &Address) -> String {
    match a {
        Address::Ip(ip) => format!(
            "addr={} fam={} iprd={}",
            ip.addr,
            fam(&ip.addr),
            ip.route_domain.map_or("-".into(), |r| r.to_string())
        ),
        Address::Fqdn(f) => format!("fqdn={}", f.name),
    }
}

fn row(out: &mut String, id: &str, input: &str, func: &str, res: Result<String, String>) {
    let (v, p) = match res {
        Ok(p) => ("ok", p),
        Err(e) => ("err", e),
    };
    let _ = writeln!(out, "{id}\t{input}\t{func}\t{v}\t{}", p.replace('\t', " "));
}

#[test]
#[ignore = "Q8 probe"]
fn q8_matrix() {
    let mut out = String::from("id\tinput\tfunction\tverdict\tparts\n");
    for (id, s) in inputs() {
        let s = s.as_str();
        row(&mut out, &id, s, "tcl_bigip::value::IPAddress::parse", IPAddress::parse(s)
            .map(|ip| format!("addr={} fam={} rd={} show={}", ip.addr, fam(&ip.addr),
                ip.route_domain.map_or("-".into(), |r| r.to_string()), ip))
            .map_err(|e| e.0));
        row(&mut out, &id, s, "tcl_bigip::value::parse_address", parse_address(s)
            .map(|a| format!("{} show={a}", addr_parts(&a))).map_err(|e| e.0));
        row(&mut out, &id, s, "tcl_bigip::value::FQDN::parse", FQDN::parse(s)
            .map(|f| f.name).map_err(|e| e.0));
        row(&mut out, &id, s, "tcl_bigip::value::Destination::parse", Destination::parse(s)
            .map(|d| format!(
                "folder={} {} destrd={} port={} spelling={:?} sep={:?} brackets={} show={}",
                d.folder.as_ref().map_or("-".into(), |f| f.to_string()),
                addr_parts(&d.address),
                d.route_domain.as_ref().map_or("-".into(), |r| r.id.to_string()),
                d.port.port, d.port.spelling, d.port_separator, d.ipv6_brackets, d))
            .map_err(|e| e.0));
        row(&mut out, &id, s, "tcl_bigip::value::Network::parse", Network::parse(s)
            .map(|n| format!("net={} default={} show={}", format!("{:?}", n.network), n.is_default(), n))
            .map_err(|e| e.0));
        row(&mut out, &id, s, "tcl_bigip::value::Port::parse", Port::parse(s)
            .map(|p| format!("port={} show={p}", p.port)).map_err(|e| e.0));
        row(&mut out, &id, s, "tcl_bigip::value::RouteDomain::parse", RouteDomain::parse(s)
            .map(|r| format!("rd={}", r.id)).map_err(|e| e.0));
        row(&mut out, &id, s, "tcl_bigip::value::Partition::parse", Partition::parse(s)
            .map(|p| p.name).map_err(|e| e.0));
        row(&mut out, &id, s, "tcl_bigip::value::Folder::parse", Folder::parse(s)
            .map(|f| format!("partition={} folders={} show={f}", f.partition, f.segments.join("/")))
            .map_err(|e| e.0));
        row(&mut out, &id, s, "tcl_bigip::value::ObjectPath::parse", ObjectPath::parse(s)
            .map(|o| format!("partition={} folders={} name={} show={o}", o.partition(),
                o.folder.segments.join("/"), o.name)).map_err(|e| e.0));
        row(&mut out, &id, s, "tcl_bigip::value::IPRange::parse", IPRange::parse(s)
            .map(|r| format!("{r:?}")).map_err(|e| e.0));
        row(&mut out, &id, s, "tcl_bigip::value::PortSet::parse", PortSet::parse(s)
            .map(|r| format!("{r:?}")).map_err(|e| e.0));
        row(&mut out, &id, s, "tcl_bigip::object_names::object_path_partition",
            object_path_partition(s).map(|p| format!("partition={p:?}")).ok_or_else(|| "none".into()));
        row(&mut out, &id, s, "tcl_bigip::object_names::reference_without_numeric_service(Retain)",
            Ok(format!("node={}", reference_without_numeric_service(s, EmptyServiceSuffix::Retain))));
        row(&mut out, &id, s, "tcl_bigip::object_names::owner_partition_reference_paths(owner=/T1/vs)",
            Ok(owner_partition_reference_paths(s, "/T1/vs", "Common").join(" ")));
        row(&mut out, &id, s, "tcl_bigip::pcap_enrich::split_destination",
            Ok(format!("addr={:?}", tcl_bigip::pcap_enrich::split_destination(s))));
        row(&mut out, &id, s, "tcl_bigip::pcap_enrich::resolve_port",
            tcl_bigip::pcap_enrich::resolve_port(s).map(|p| format!("port={p}")).ok_or_else(|| "none".into()));
    }
    std::fs::write(dir().join("matrix-tcl-bigip.tsv"), out).unwrap();
}

#[test]
#[ignore = "Q8 probe"]
fn q8_config() {
    let src = std::fs::read_to_string(dir().join("addresses.conf")).unwrap();
    let cfg = tcl_bigip::parser::parse_bigip_conf(&src, "Common");
    let mut out = String::from("# parse_bigip_conf(addresses.conf): typed fields per object\n");
    for p in &cfg.objects {
        let line = match &p.object {
            ModelObject::VirtualServer(v) => format!(
                "destination={} parsed={:?} pool={} mask={} source={}",
                v.destination.as_ref().map_or("NONE".into(), |d| d.to_string()),
                v.destination.as_ref().map(|d| (addr_parts(&d.address), d.route_domain.as_ref().map(|r| r.id), d.port.port, d.port.spelling.clone())),
                v.pool, v.mask, v.source),
            ModelObject::Node(n) => format!("address={} fqdn={:?}",
                n.address.as_ref().map_or("NONE".into(), addr_parts), n.fqdn.as_ref().map(|f| f.name.clone())),
            ModelObject::VirtualAddress(a) => format!("address={} mask={}",
                a.address.as_ref().map_or("NONE".into(), |ip| format!("{} rd={:?}", ip.addr, ip.route_domain)), a.mask),
            ModelObject::NetRoute(r) => format!("network={} default={} gw={}",
                r.network.as_ref().map_or("NONE".into(), |n| format!("{} show={n}", format!("{:?}", n.network))),
                r.is_default_route,
                r.gw.as_ref().map_or("NONE".into(), |ip| ip.to_string())),
            ModelObject::NetSelf(s) => format!("address={}",
                s.address.as_ref().map_or("NONE".into(), |n| format!("{} show={n}", format!("{:?}", n.network)))),
            ModelObject::Pool(pl) => {
                let mut m = String::new();
                for it in &pl.members.items {
                    if let tcl_bigip::value::ListItemValue::PoolMember(pm) = &it.value {
                        let _ = write!(m, "[name={} address={} port={}] ", pm.name,
                            pm.address.as_ref().map_or("NONE".into(), addr_parts), pm.port);
                    } else {
                        let _ = write!(m, "[{:?}] ", it.value);
                    }
                }
                format!("members={m} monitor={}", pl.monitor)
            }
            ModelObject::LtmSnatTranslation(t) => format!("address={}", t.address),
            _ => continue,
        };
        let _ = writeln!(out, "{}\t{}\t{}", p.table_name, p.full_path, line);
    }
    let _ = writeln!(out, "\n# object paths (all tables)");
    for p in &cfg.objects {
        let _ = writeln!(out, "{}\t{}", p.table_name, p.full_path);
    }
    let configs = vec![("file:///addresses.conf".to_owned(), &cfg)];
    let _ = writeln!(out, "\n# run_lint");
    for f in tcl_bigip::lint::run_lint(&configs, &[], None, None) {
        let _ = writeln!(out, "{}\t{}\t{}\t{}", f.rule_id, f.severity, f.full_path, f.message);
    }
    let _ = writeln!(out, "\n# validate_bigip_source");
    for d in tcl_bigip::validator::validate_bigip_source(&src, "Common") {
        let _ = writeln!(out, "{d:?}");
    }
    let ctx = tcl_bigip::graph::GraphContext::new();
    let sources = vec![("file:///addresses.conf".to_owned(), src.clone())];
    let graph = tcl_bigip::graph::build_bigip_object_graph(&sources, &configs, &ctx);
    let _ = writeln!(out, "\n# graph edges");
    for e in &graph.edges {
        let _ = writeln!(out, "{}\t-> {}\tvia {} ({})", e.source_id, e.target_id, e.via_property, e.via_kind);
    }
    let stats = tcl_bigip::stats::compute_stats(&graph, &configs, 50);
    let _ = writeln!(out, "\n# stats\n{}", tcl_bigip::stats::report_to_json(&stats));
    let idx = tcl_bigip::pcap_enrich::build_name_index(&cfg, Some(&src));
    let _ = writeln!(out, "\n# pcap name index\nv4={:?}\nv6={:?}", idx.v4, idx.v6);
    let _ = writeln!(out, "v4_subnets={:?}\nv6_subnets={:?}",
        idx.v4_subnets.iter().map(|(n, l)| format!("{}={l}", n.text_for_probe())).collect::<Vec<_>>(),
        idx.v6_subnets.len());
    let ws = tcl_bigip::wireshark_profile::build_wireshark_profile(&[(cfg.clone(), src.clone())]);
    let _ = writeln!(out, "\n# wireshark hosts={:?}\nsubnets={:?}", ws.hosts, ws.subnets);
    let as3 = tcl_bigip::convert::scf_to_as3(&cfg, "T", "A");
    let _ = writeln!(out, "\n# scf_to_as3\n{:?}", as3.declaration);
    std::fs::write(dir().join("config-tcl-bigip.txt"), out).unwrap();
}

trait ProbeText {
    fn text_for_probe(&self) -> String;
}
impl ProbeText for tcl_bigip::pcap_enrich::IpNetwork {
    fn text_for_probe(&self) -> String {
        format!("{self:?}")
    }
}
