#!/usr/bin/env python3
"""Golden fixtures for the Rust query *net/IP/CIDR-category builtins*.

For a list of `query` strings (each over a `null` JSON root, mode ``json``)
runs the Python query DSL end to end — parse → evaluate → ``output.render``
— and records the rendered string (or the ``error:`` message). The Rust
``net_parity`` test mirrors the same pipeline and asserts byte-equality, so
the net builtins stay byte-faithful to the Python source. Self-contained:
the fixtures are captured once here and checked in.

Run from the repo root:

    PYTHONPATH=. python3 scripts/codegen/gen_f5_query_net_fixtures.py
"""

from __future__ import annotations

import json
from pathlib import Path

from dialects.f5.bigip.model import BigipConfig
from dialects.f5.query.errors import QueryError
from dialects.f5.query.evaluator import EvalContext, evaluate
from dialects.f5.query.output import render
from dialects.f5.query.parser import parse_query
from dialects.f5.query.source_map import SourceMap
from dialects.f5.query.values import Root


def run(query: str, data: object, mode: str) -> dict:
    root = Root(
        uri="data.json",
        source="",
        config=BigipConfig(),
        source_map=SourceMap.build(""),
        json_value=data,
    )
    ctx = EvalContext(root=root)
    try:
        prog = parse_query(query)
        values = evaluate(prog, ctx)
        out = render(values, mode=mode)
        return {"query": query, "input": data, "mode": mode, "kind": "ok", "output": out}
    except (QueryError, ValueError) as exc:
        return {"query": query, "input": data, "mode": mode, "kind": "err", "output": str(exc)}


# Every net builtin ported into `builtins/net.rs`, with representative inputs:
# IPv4 + IPv6, with/without partition, with/without `%route-domain`,
# with/without `:port`, CIDR ranges, predicates (true & false), the merge /
# subset / overlap helpers, port-sets, IP-ranges, path/folder edits, and the
# error cases. Root is always `null`; each query stands alone. Mode is "json".
QUERIES: list[str] = [
    # ip (1-arg normalisation)
    'ip("10.0.0.1")',
    'ip("/Common/192.168.1.1:80")',
    'ip("/Common/10.10.0.5%5:443")',
    'ip("2001:db8::1")',
    'ip("/Common/[2001:db8::1]:80")',
    'ip("[2001:db8::1%5].8080")',
    # ip (2-arg rebase)
    'ip("192.168.9.0/24", "10.10.0.5%5:443")',
    'ip("192.168.9.0/24", "/Common/10.10.0.5%5:443")',
    'ip("2001:db8:9::/64", "2001:db8::1:203")',
    'ip("192.168.9.0/24", "10.10.0.5")',
    # ip errors
    'ip("notanip")',
    'ip("garbage", "10.0.0.5")',
    'ip("192.168.9.0/24", "notanip")',
    'ip("10.0.0.0/24", "2001:db8::1")',
    'ip("2001:db8::/64", "10.0.0.1")',
    'ip(42)',
    # ip_translate
    'ip_translate("10.0.0.0/8", "2001:db8::/32", "10.1.2.3")',
    'ip_translate("192.168.50.0/24", "2001:db8:50::/64", "192.168.50.10")',
    'ip_translate("10.0.0.0/8", "192.168.0.0/16", "10.0.0.5")',
    'ip_translate("10.0.0.0/24", "10.1.0.0/16", "10.0.0.5")',
    'ip_translate("10.0.0.0/24", "10.1.0.0/26", "10.0.0.5")',
    'ip_translate("10.0.0.0/24", "10.1.0.0/16", "192.168.1.1")',
    'ip_translate("bad", "10.0.0.0/8", "10.0.0.1")',
    'ip_translate("10.0.0.0/8", "bad", "10.0.0.1")',
    'ip_translate("10.0.0.0/8", "10.1.0.0/16", "notanip")',
    # net
    'net("192.168.9.0/24")',
    'net("192.168.9.42/24")',
    'net("2001:db8::42/64")',
    'net("10.0.0.0/255.255.255.0")',
    'net("notanet")',
    # host / port / route_domain
    'host("/Common/192.168.1.1:80")',
    'host("/Common/10.0.0.1%5:443")',
    'host("2001:db8::1.80")',
    'host("plainstring")',
    'port("192.168.1.1:80")',
    'port("/Common/10.0.0.1%5:443")',
    'port("/Common/10.0.0.1")',
    'port("0.0.0.0:any")',
    'route_domain("10.0.0.1%5:80")',
    'route_domain("/Common/10.0.0.1:80")',
    'route_domain("2001:db8::1%7.443")',
    # with_route_domain
    'with_route_domain("/Common/10.0.0.1:80", 7)',
    'with_route_domain("/Common/10.0.0.1%5:80", "")',
    'with_route_domain("10.0.0.1:80", "9")',
    "with_route_domain(\"10.0.0.1\", 3)",
    'with_route_domain("10.0.0.1", true)',
    # in_cidr
    'in_cidr("10.0.0.5", "10.0.0.0/8")',
    'in_cidr("/Common/10.0.0.5:80", "10.0.0.0/8")',
    'in_cidr("10.0.0.5%5", "10.0.0.0/8")',
    'in_cidr("192.168.1.1", "10.0.0.0/8")',
    'in_cidr("2001:db8::1", "2001:db8::/32")',
    'in_cidr("10.0.0.5", "2001:db8::/32")',
    'in_cidr("notanip", "10.0.0.0/8")',
    'in_cidr("10.0.0.5", "notanet")',
    # predicates: is_ipv4 / is_ipv6 / is_fqdn
    'is_ipv4("10.0.0.1")',
    'is_ipv4("2001:db8::1")',
    'is_ipv4("host.example.com")',
    'is_ipv4("/Common/10.0.0.1:80")',
    'is_ipv6("2001:db8::1")',
    'is_ipv6("[2001:db8::1]:80")',
    'is_ipv6("10.0.0.1")',
    'is_fqdn("host.example.com")',
    'is_fqdn("/Common/host.example.com:443")',
    'is_fqdn("10.0.0.1")',
    "is_ipv4(null)",
    # is_private / is_loopback / is_unspecified
    'is_private("10.0.0.1")',
    'is_private("172.16.5.5")',
    'is_private("192.168.1.1")',
    'is_private("8.8.8.8")',
    'is_private("fc00::1")',
    'is_private("2001:db8::1")',
    'is_loopback("127.0.0.1")',
    'is_loopback("::1")',
    'is_loopback("10.0.0.1")',
    'is_unspecified("0.0.0.0")',
    'is_unspecified("::")',
    'is_unspecified("10.0.0.1")',
    # is_multicast / is_link_local / is_reserved
    'is_multicast("224.0.0.1")',
    'is_multicast("ff02::1")',
    'is_multicast("10.0.0.1")',
    'is_link_local("169.254.1.1")',
    'is_link_local("fe80::1")',
    'is_link_local("10.0.0.1")',
    'is_reserved("240.0.0.1")',
    'is_reserved("10.0.0.1")',
    # is_public / is_documentation
    'is_public("8.8.8.8")',
    'is_public("10.0.0.1")',
    'is_public("127.0.0.1")',
    'is_public("224.0.0.1")',
    'is_public("2606:4700:4700::1111")',
    'is_documentation("192.0.2.1")',
    'is_documentation("198.51.100.1")',
    'is_documentation("203.0.113.1")',
    'is_documentation("2001:db8::1")',
    'is_documentation("10.0.0.1")',
    # is_wildcard_port
    'is_wildcard_port("/Common/0.0.0.0:any")',
    'is_wildcard_port("/Common/10.0.0.1:0")',
    'is_wildcard_port("/Common/10.0.0.1:80")',
    "is_wildcard_port(null)",
    # prefix_length / network_address / broadcast_address
    'prefix_length("10.0.0.0/24")',
    'prefix_length("10.0.0.0/255.255.255.0")',
    'prefix_length("2001:db8::/64")',
    'prefix_length("notanet")',
    'network_address("10.0.0.5/24")',
    'network_address("2001:db8::5/64")',
    'network_address("notanet")',
    'broadcast_address("10.0.0.0/24")',
    'broadcast_address("2001:db8::/120")',
    # first_host / last_host / host_count
    'first_host("10.0.0.0/24")',
    'first_host("10.0.0.0/31")',
    'first_host("10.0.0.5/32")',
    'first_host("2001:db8::/120")',
    'last_host("10.0.0.0/24")',
    'last_host("10.0.0.0/31")',
    'last_host("2001:db8::/120")',
    'host_count("10.0.0.0/24")',
    'host_count("10.0.0.0/31")',
    'host_count("10.0.0.0/32")',
    'host_count("2001:db8::/120")',
    'host_count("notanet")',
    # collapse_cidrs
    'collapse_cidrs(["10.0.0.0/24", "10.0.1.0/24"])',
    'collapse_cidrs(["10.0.0.0/9", "10.128.0.0/9"])',
    'collapse_cidrs(["10.0.0.0/8", "10.1.0.0/16"])',
    'collapse_cidrs(["10.0.0.0/24", "10.0.2.0/24"])',
    'collapse_cidrs(["10.0.0.0/24", "2001:db8::/64", "10.0.1.0/24"])',
    'collapse_cidrs(["10.0.0.0/24", "notanet"])',
    "collapse_cidrs([])",
    # supernet_of
    'supernet_of(["10.0.0.1", "10.0.1.1"])',
    'supernet_of(["10.0.0.0/24", "10.0.1.0/24"])',
    'supernet_of(["10.0.0.5"])',
    'supernet_of(["2001:db8::1", "2001:db8::1:0"])',
    'supernet_of(["10.0.0.1", "2001:db8::1"])',
    "supernet_of([])",
    # subnet_of / overlaps
    'subnet_of("10.1.0.0/16", "10.0.0.0/8")',
    'subnet_of("10.0.0.0/8", "10.1.0.0/16")',
    'subnet_of("10.0.0.0/8", "10.0.0.0/8")',
    'subnet_of("10.0.0.0/8", "2001:db8::/32")',
    'subnet_of("notanet", "10.0.0.0/8")',
    'overlaps("10.0.0.0/24", "10.0.0.0/16")',
    'overlaps("10.0.0.0/24", "10.1.0.0/24")',
    'overlaps("10.0.0.0/8", "2001:db8::/32")',
    # with_port
    'with_port("/Common/10.0.0.1:80", 8443)',
    'with_port("/Common/10.0.0.1:80", "any")',
    'with_port("/Common/10.0.0.1:80", "")',
    'with_port("/Common/[2001:db8::1]:80", 443)',
    'with_port("2001:db8::1.80", 443)',
    'with_port("/Common/10.0.0.1:80", 70000)',
    'with_port("/Common/10.0.0.1:80", true)',
    'with_port("notadest", 80)',
    'with_port("/Common/10.0.0.1:80", "notaport")',
    # with_host
    'with_host("/Common/10.0.0.1:80", "10.0.0.2")',
    'with_host("/Common/10.0.0.1:80", "host.example.com")',
    'with_host("/Common/10.0.0.1:80", "2001:db8::1")',
    'with_host("/Common/[2001:db8::1]:80", "10.0.0.5")',
    'with_host("notadest", "10.0.0.1")',
    # folder / with_folder
    'folder("/Common/web_pool")',
    'folder("/Common/iApps/Tenant.app/pool_1")',
    'folder("relativename")',
    'with_folder("/Common/web_pool", "/Tenant_A")',
    'with_folder("/Common/iApps/old.app/pool_1", "/Common/iApps/new.app")',
    'with_folder("notapath", "/Common")',
    'with_folder("/Common/web_pool", "badfolder")',
    # with_name
    'with_name("/Common/old_pool", "new_pool")',
    'with_name("/Common/iApps/T.app/p1", "p2")',
    'with_name("barename", "renamed")',
    'with_name("/Common/pool", "  ")',
    'with_name("/badpath/", "x")',
    # in_partition / in_folder
    'in_partition("/Common/web_pool", "Common")',
    'in_partition("/Common/web_pool", "/Common")',
    'in_partition("/Tenant_A/p", "Common")',
    'in_partition("notapath", "Common")',
    'in_folder("/Common/iApps/Tenant.app/pool_1", "/Common/iApps")',
    'in_folder("/Common/web_pool", "/Common/iApps")',
    'in_folder("/Common/iApps_bak/p", "/Common/iApps")',
    'in_folder("/Common/web_pool", "/Common")',
    # can_see
    'can_see("/Tenant_A/vs1", "/Common/web_pool")',
    'can_see("/Common/vs1", "/Tenant_A/web_pool")',
    'can_see("/Tenant_A/vs1", "/Tenant_B/web_pool")',
    'can_see("/Common/a", "/Common/b")',
    'can_see("notapath", "/Common/b")',
    # port_set_contains / count / overlaps
    'port_set_contains("80-82,8081", 81)',
    'port_set_contains("80,443", 8080)',
    'port_set_contains("any", 12345)',
    'port_set_contains("notaset", 80)',
    'port_set_count("80-82,8081")',
    'port_set_count("any")',
    'port_set_count("80,80,80")',
    'port_set_count("notaset")',
    'port_set_overlaps("80-82,443", "82-100")',
    'port_set_overlaps("80-82,443", "100-200")',
    'port_set_overlaps("notaset", "80")',
    # ip_range_to_cidrs / supernet / count / contains
    'ip_range_to_cidrs("192.168.9.77-192.168.9.83")',
    'ip_range_to_cidrs("10.0.0.1-10.0.0.255")',
    'ip_range_to_cidrs("10.0.0.5")',
    'ip_range_to_cidrs("notarange")',
    'ip_range_supernet("192.168.9.77-192.168.9.83")',
    'ip_range_supernet("10.0.0.0-10.0.0.255")',
    'ip_range_count("192.168.9.77-192.168.9.83")',
    'ip_range_count("10.0.0.1")',
    'ip_range_count("badrange")',
    'ip_range_contains("192.168.9.77-192.168.9.83", "192.168.9.80")',
    'ip_range_contains("10.0.0.0-10.0.0.255", "10.0.1.1")',
    'ip_range_contains("10.0.0.0-10.0.0.255", "2001:db8::1")',
    'ip_range_contains("badrange", "10.0.0.1")',
    # shared coercion errors
    "host(42)",
    'in_cidr(42, "10.0.0.0/8")',
    "port_set_count(true)",
]


def main() -> None:
    repo_root = Path(__file__).resolve().parents[2]
    out_path = repo_root / "rust" / "tcl-bigip-query" / "tests" / "fixtures" / "net.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    cases = [run(q, None, "json") for q in QUERIES]
    out_path.write_text(json.dumps(cases, indent=2, ensure_ascii=True) + "\n", encoding="utf-8")
    ok = sum(1 for c in cases if c["kind"] == "ok")
    print(f"wrote {out_path} ({len(cases)} cases, {ok} ok, {len(cases) - ok} err)")


if __name__ == "__main__":
    main()
