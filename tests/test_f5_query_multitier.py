"""Sample-backed multi-device stress tests for the ``f5 query`` DSL."""

from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.query import run_query

SAMPLE_DIR = Path(__file__).resolve().parent.parent / "samples" / "for_f5_query" / "multitier"
T2_COUNT = 12
T2_IDS = tuple(f"c{i:02d}" for i in range(1, T2_COUNT + 1))
SHARED_T2_VIP = "10.2.0.10"
SHARED_T2_PORT = 443


def _read(name: str) -> str:
    return (SAMPLE_DIR / name).read_text(encoding="utf-8")


def _source(name: str) -> dict[str, str]:
    return {f"sample://{name}": _read(name)}


def _t2_sources() -> dict[str, str]:
    return {
        f"sample://tier2-{cluster}.conf": _read(f"tier2-{cluster}-ltm-ha.conf")
        for cluster in T2_IDS
    }


def _all_sources() -> dict[str, str]:
    sources = {
        "sample://gtm.conf": _read("gtm-dns.conf"),
        "sample://tier1.conf": _read("tier1-ltm-ha.conf"),
        "sample://tier3.conf": _read("tier3-reaggregator.conf"),
    }
    sources.update(_t2_sources())
    return sources


def test_multitier_sample_files_exist_for_physically_separate_devices():
    expected = {
        "border-nat.json",
        "generate.py",
        "generate_logs.py",
        "gtm-dns.conf",
        "tier1-ltm-ha.conf",
        "tier3-reaggregator.conf",
        *(f"tier2-{cluster}-ltm-ha.conf" for cluster in T2_IDS),
    }
    assert {p.name for p in SAMPLE_DIR.iterdir() if p.is_file()} == expected


def test_gtm_public_answers_and_border_nat_side_input_agree_on_service():
    result = run_query(
        ".gtm.server[].virtual-servers",
        _source("gtm-dns.conf"),
    )
    [answers] = result.values_per_file["sample://gtm-dns.conf"]
    assert sorted(answers) == ["203.0.113.10:443", "203.0.113.11:443"]

    nat_path = SAMPLE_DIR / "border-nat.json"
    result = run_query(
        f'[json_load("{nat_path}")[] | select(.service == "www.example.com") | .internal] | unique',
        _source("gtm-dns.conf"),
    )
    [internal_vips] = result.values_per_file["sample://gtm-dns.conf"]
    assert internal_vips == ["10.1.0.10:443"]


def test_gtm_wideip_deep_links_to_pool_members():
    query = (
        ".gtm.wideip[] as $wide "
        "| $wide.pools[] as $pool "
        '| tsv($wide.name, $pool.name, join($pool.members, ","), source_file($pool))'
    )
    result = run_query(query, _source("gtm-dns.conf"))
    assert result.values_per_file["sample://gtm-dns.conf"] == [
        "www.example.com\twww_pool\t/Common/border-fw:public_a,/Common/border-fw:public_b\tsample://gtm-dns.conf"
    ]


def test_tier1_fanout_pool_has_one_route_domain_member_per_tier2_device():
    result = run_query(
        '.ltm.pool["/Common/t2_fanout_pool"].members[] | .address',
        _source("tier1-ltm-ha.conf"),
    )
    addresses = result.values_per_file["sample://tier1-ltm-ha.conf"]
    assert len(addresses) == T2_COUNT
    assert {addr.split("%", 1)[0] for addr in addresses} == {SHARED_T2_VIP}
    assert sorted(addr.split("%", 1)[1] for addr in addresses) == [
        str(101 + index) for index in range(T2_COUNT)
    ]


def test_tier1_and_tier2_are_ha_but_tier3_is_standalone():
    result = run_query("[.cm.device-group[]] | count", _all_sources())
    counts = {uri: values[0] for uri, values in result.values_per_file.items()}
    assert counts["sample://gtm.conf"] == 0
    assert counts["sample://tier1.conf"] == 1
    assert counts["sample://tier3.conf"] == 0
    for cluster in T2_IDS:
        assert counts[f"sample://tier2-{cluster}.conf"] == 1


def test_every_tier2_device_uses_same_vip_and_port_without_translation():
    query = (
        ".ltm.virtual[] "
        "| tsv(source_file(.), (.destination | host), str(.destination | port), "
        '."translate-address", ."translate-port")'
    )
    result = run_query(query, _t2_sources())
    rows = [row for values in result.values_per_file.values() for row in values]
    assert len(rows) == T2_COUNT
    for row in rows:
        source, host, port, translate_address, translate_port = row.split("\t")
        assert source.startswith("sample://tier2-c")
        assert host == SHARED_T2_VIP
        assert int(port) == SHARED_T2_PORT
        assert translate_address == "disabled"
        assert translate_port == "disabled"


def test_every_tier2_device_has_distinct_vlan_for_the_same_vip():
    query = ".ltm.virtual[].vlans[] | tsv(source_file(.), .name, str(.tag))"
    result = run_query(query, _t2_sources())
    rows = [row for values in result.values_per_file.values() for row in values]
    assert len(rows) == T2_COUNT
    tags = sorted(row.split("\t")[2] for row in rows)
    assert tags == [str(201 + index) for index in range(T2_COUNT)]
    assert {row.split("\t")[1] for row in rows} == {"client_vlan"}


def test_tier2_tls_http_and_policy_routing_deep_links_to_backend_pool():
    query = (
        ".ltm.virtual[] as $vs "
        "| $vs.policies[].rules[] as $rule "
        '| select($rule.name == "api") '
        '| tsv(source_file($vs), join(([$vs.profiles[].name] | sort), ","), '
        "$rule.conditions[].operand, $rule.conditions[].operator, "
        "$rule.actions[].pool.full-path)"
    )
    result = run_query(query, _t2_sources())
    rows = [row for values in result.values_per_file.values() for row in values]
    assert len(rows) == T2_COUNT
    for row in rows:
        source, profiles, operand, operator, pool_path = row.split("\t")
        assert source.startswith("sample://tier2-c")
        assert profiles == "clientssl,http,serverssl,tcp"
        assert operand == "http-uri"
        assert operator == "starts-with"
        assert pool_path == "/Common/backend_pool"


def test_tier2_security_policy_rule_lists_deep_link():
    query = (
        ".security.firewall-policy[] as $policy "
        "| $policy.rule-lists[] as $rule_list "
        '| tsv(source_file($policy), $policy.name, $rule_list.name, join($rule_list.rules, ","))'
    )
    result = run_query(query, _t2_sources())
    rows = [row for values in result.values_per_file.values() for row in values]
    assert len(rows) == T2_COUNT
    assert all(row.endswith("\tedge_fw\tallow_web\tallow_tls") for row in rows)


def test_tier3_is_the_only_device_with_snatpool():
    result = run_query(".ltm.snatpool[].name", _all_sources())
    owners = {uri: values for uri, values in result.values_per_file.items() if values}
    assert owners == {"sample://tier3.conf": ["internet_snat"]}


def test_tier3_snatpool_members_are_translation_refs():
    result = run_query(
        '.ltm.snatpool["/Common/internet_snat"].members',
        _source("tier3-reaggregator.conf"),
    )
    [members] = result.values_per_file["sample://tier3-reaggregator.conf"]
    assert sorted(str(member) for member in members) == [
        "/Common/snat_xlat_1",
        "/Common/snat_xlat_2",
        "/Common/snat_xlat_3",
    ]


def test_topology_census_keeps_devices_separate_instead_of_merge_mode():
    result = run_query(
        "{virtuals: ([.ltm.virtual[]] | count), pools: ([.ltm.pool[]] | count), "
        "device_groups: ([.cm.device-group[]] | count), "
        "snat_pools: ([.ltm.snatpool[]] | count)}",
        _all_sources(),
    )
    census = {uri: rows[0] for uri, rows in result.values_per_file.items()}
    assert census["sample://gtm.conf"] == {
        "device_groups": 0,
        "pools": 0,
        "snat_pools": 0,
        "virtuals": 0,
    }
    assert census["sample://tier1.conf"]["virtuals"] == 1
    assert census["sample://tier1.conf"]["device_groups"] == 1
    assert census["sample://tier3.conf"]["virtuals"] == 1
    assert census["sample://tier3.conf"]["snat_pools"] == 1
    for cluster in T2_IDS:
        assert census[f"sample://tier2-{cluster}.conf"] == {
            "device_groups": 1,
            "pools": 1,
            "snat_pools": 0,
            "virtuals": 1,
        }


def test_tier1_member_addresses_match_each_tier2_device_vip():
    t1 = run_query(
        '.ltm.pool["/Common/t2_fanout_pool"].members[].address',
        _source("tier1-ltm-ha.conf"),
    )
    t1_hosts = {
        str(address).split("%", 1)[0]
        for address in t1.values_per_file["sample://tier1-ltm-ha.conf"]
    }
    t2 = run_query("[.ltm.virtual[].destination | host] | unique", _t2_sources())
    t2_hosts = {
        host for values in t2.values_per_file.values() for group in values for host in group
    }
    assert t1_hosts == t2_hosts == {SHARED_T2_VIP}


def test_border_nat_sample_is_valid_json_and_names_separate_firewall_devices():
    rows = json.loads((SAMPLE_DIR / "border-nat.json").read_text(encoding="utf-8"))
    assert {row["device"] for row in rows} == {"border-fw-a", "border-fw-b"}
    assert {row["public"] for row in rows} == {"203.0.113.10:443", "203.0.113.11:443"}
    assert {row["internal"] for row in rows} == {"10.1.0.10:443"}
