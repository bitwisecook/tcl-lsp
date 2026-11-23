#!/usr/bin/env python3
"""Fold the matrix-*.tsv files into summary-endpoints.tsv: for each endpoint-
shaped input, the (address, route domain, port) every destination/member
splitter returns, so disagreements are visible on one row.
Run: python3 -I summarize.py
"""
import glob
import re

SPLITTERS = {
    "tcl_bigip::value::Destination::parse": "bigip.Destination",
    "tcl_bigip_query::typed_values::Destination::try_parse": "query.Destination",
    "tcl_bigip_query::typed_values::split_destination": "query.split_destination",
    "tcl_bigip::pcap_enrich::split_destination": "pcap.split_destination",
    "tcl_bigip::convert::split_destination": "as3.split_destination",
    "bigip-report-gen::model::split_dest": "report.split_dest",
    "f5-cli::explain_flow::parse_destination": "explain_flow",
    "frontend topology.ts parseIpQuery": "frontend.parseIpQuery",
    "tcl_bigip_query::architecture::bare_ip": "arch.bare_ip",
    "bigip-report-gen::enrich::to_ip": "report.to_ip",
    "tcl_bigip::object_names::reference_without_numeric_service(Retain)": "member->node",
}


def norm(func, verdict, parts):
    if verdict == "err":
        return "REJECT"
    g = lambda k: (re.search(k + r'=("?)([^" ]*)\1', parts) or [None, None, ""])[2]
    if func.endswith("Destination::parse") or func.endswith("Destination::try_parse"):
        a = g("addr") or ("fqdn:" + g("fqdn"))
        rd = g("destrd")
        rd = re.sub(r"Some\((\d+)\)", r"\1", rd)
        rd = "" if rd in ("-", "None") else rd
        port = g("port")
        sp = g("spelling")
        return f"{a}|rd={rd}|port={port if sp else 'none'}"
    if func.endswith("query::typed_values::split_destination"):
        return f"{g('addr')}|rd={g('rd')}|port={g('port') or 'none'}"
    if "pcap" in func:
        return f"{g('addr') or 'EMPTY'}"
    if "convert" in func:
        m = re.search(r'port=Some\((\d+)\)', parts)
        return f"{g('addr')}|port={m.group(1) if m else 'none'}"
    if "split_dest" in func:
        return f"{g('addr')}|port={g('port') or 'none'}"
    if "explain_flow" in func:
        m = re.search(r'Some\(\("([^"]*)", (\d+)\)\)', parts)
        return f"{m.group(1)}|port={m.group(2)}" if m else "NONE"
    if "parseIpQuery" in func:
        return parts.replace(",", ";")
    if "bare_ip" in func or "to_ip" in func:
        return parts.replace("addr=", "")
    if "member->node" in SPLITTERS.get(func, ""):
        return g("node")
    return parts


def main():
    rows = {}
    order = []
    for f in sorted(glob.glob("matrix-*.tsv")):
        for line in open(f, encoding="utf-8").read().splitlines()[1:]:
            p = line.split("\t")
            if len(p) < 5 or p[2] not in SPLITTERS:
                continue
            key = (p[0], p[1])
            if key not in rows:
                rows[key] = {}
                order.append(key)
            rows[key][SPLITTERS[p[2]]] = norm(p[2], p[3], p[4])
    cols = list(SPLITTERS.values())
    docs = {}
    for line in open("inputs.tsv", encoding="utf-8").read().splitlines()[1:]:
        p = line.split("\t")
        docs[p[0]] = (p[1], p[3])
    with open("summary-endpoints.tsv", "w", encoding="utf-8") as out:
        out.write("id\tgroup\tinput\tdoc\t" + "\t".join(cols) + "\tdistinct_answers\n")
        for key in sorted(order):
            r = rows[key]
            g, d = docs.get(key[0], ("", ""))
            vals = [r.get(c, "") for c in cols]
            addr_only = {v.split("|")[0] for v in vals[:7] if v and v not in ("REJECT", "NONE", "EMPTY")}
            out.write(f"{key[0]}\t{g}\t{key[1]}\t{d}\t" + "\t".join(vals) + f"\t{len(addr_only)}\n")


if __name__ == "__main__":
    main()
