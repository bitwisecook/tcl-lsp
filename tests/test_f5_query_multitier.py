"""Multi-tier topology stress tests for the ``f5 query`` DSL.

The synthetic topology models a common enterprise traffic path:

* GTM resolves a public service name to public border addresses.
* An assumed border firewall NATs those public addresses to an
  internal tier-1 LTM VIP.
* Tier 1 is HA and load-balances to twelve tier-2 LTM clusters.
* Every tier-2 cluster exposes the same application VIP IP and port,
  but in a different partition, route-domain, and VLAN, with address
  and port translation disabled.
* Tier 2 carries TLS, HTTP profiles, HTTP policy routing, and a
  firewall policy reference.
* Tier 2 default-routes to a standalone tier-3 reaggregator.
* Tier 3 is not HA and owns the SNAT pool for egress.

The tests deliberately use large joins, aggregations, PathRef walks,
merge mode, and per-file queries so the query engine has to exercise
the same shapes a real multi-layer review would ask for.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.query import run_query

_T2_COUNT = 12
_T2_IDS = tuple(f"c{i:02d}" for i in range(1, _T2_COUNT + 1))
_SHARED_T2_VIP = "10.2.0.10"
_SHARED_T2_PORT = "443"


def _partition(cluster: str) -> str:
    return f"T2_{cluster.upper()}"


GTM_CONF = """\
gtm datacenter /Common/dc-east {
    location east
}
gtm server /Common/border-fw {
    datacenter /Common/dc-east
    product bigip
    virtual-servers {
        public_a {
            destination 203.0.113.10:443
        }
        public_b {
            destination 203.0.113.11:443
        }
    }
}
gtm pool a /Common/www_pool {
    members {
        /Common/border-fw:public_a { }
        /Common/border-fw:public_b { }
    }
    monitor /Common/gateway_icmp
}
gtm wideip a /Common/www.example.com {
    pools {
        /Common/www_pool { }
    }
    pool-lb-mode round-robin
}
"""


def _render_t1() -> str:
    nodes = []
    members = []
    route_domains = []
    for index, cluster in enumerate(_T2_IDS, start=1):
        rd = 100 + index
        nodes.append(
            f"""ltm node /Common/t2_{cluster}_vip {{
    address {_SHARED_T2_VIP}%{rd}
}}
"""
        )
        members.append(
            f"""        /Common/t2_{cluster}_vip:{_SHARED_T2_PORT} {{
            address {_SHARED_T2_VIP}%{rd}
        }}
"""
        )
        route_domains.append(
            f"""net route-domain /Common/t2_{cluster}_rd {{
    id {rd}
}}
"""
        )
    return (
        """cm device /Common/t1-a {
    hostname t1-a.example.test
    management-ip 192.0.2.11
}
cm device /Common/t1-b {
    hostname t1-b.example.test
    management-ip 192.0.2.12
}
cm device-group /Common/t1-failover {
    type sync-failover
    devices {
        /Common/t1-a { }
        /Common/t1-b { }
    }
}
"""
        + "".join(route_domains)
        + "".join(nodes)
        + f"""ltm pool /Common/t2_fanout_pool {{
    members {{
{"".join(members)}    }}
    monitor /Common/tcp
    load-balancing-mode least-connections-member
}}
ltm virtual /Common/t1_ingress_vs {{
    destination /Common/10.1.0.10:443
    pool /Common/t2_fanout_pool
    profiles {{
        /Common/tcp {{ }}
        /Common/http {{ }}
        /Common/clientssl {{ }}
    }}
    ip-protocol tcp
}}
"""
    )


T1_CONF = _render_t1()


def _render_t2(cluster: str) -> str:
    index = int(cluster[1:])
    partition = _partition(cluster)
    vlan_tag = 200 + index
    rd = 100 + index
    backend_octet = index + 20
    return f"""auth partition {partition} {{ }}
cm device /{partition}/t2-{cluster}-a {{
    hostname t2-{cluster}-a.example.test
    management-ip 192.0.2.{backend_octet}
}}
cm device /{partition}/t2-{cluster}-b {{
    hostname t2-{cluster}-b.example.test
    management-ip 192.0.2.{backend_octet + 100}
}}
cm device-group /{partition}/t2-{cluster}-failover {{
    type sync-failover
    devices {{
        /{partition}/t2-{cluster}-a {{ }}
        /{partition}/t2-{cluster}-b {{ }}
    }}
}}
net vlan /{partition}/client_vlan {{
    interfaces {{
        1.{index} {{ }}
    }}
    tag {vlan_tag}
}}
net route-domain /{partition}/client_rd {{
    id {rd}
    vlans {{
        /{partition}/client_vlan
    }}
}}
security firewall rule-list /{partition}/allow_web {{
    rules {{
        allow_tls {{
            action accept
            ip-protocol tcp
        }}
    }}
}}
security firewall policy /{partition}/edge_fw {{
    rules {{
        allow_web_binding {{ rule-list /{partition}/allow_web }}
    }}
}}
ltm pool /{partition}/backend_pool {{
    members {{
        /{partition}/backend_1:8443 {{
            address 10.3.{backend_octet}.10
        }}
        /{partition}/backend_2:8443 {{
            address 10.3.{backend_octet}.11
        }}
    }}
    monitor /Common/tcp
}}
ltm policy /{partition}/http_router {{
    controls {{ forwarding }}
    requires {{ http }}
    rules {{
        api {{
            actions {{
                0 {{
                    forward
                    select
                    pool /{partition}/backend_pool
                }}
            }}
            conditions {{
                0 {{
                    http-uri
                    path
                    starts-with
                    values {{ /api }}
                }}
            }}
            ordinal 10
        }}
        default {{
            actions {{
                0 {{
                    forward
                    select
                    pool /{partition}/backend_pool
                }}
            }}
            ordinal 20
        }}
    }}
}}
ltm virtual /{partition}/app_vs {{
    destination /{partition}/{_SHARED_T2_VIP}:{_SHARED_T2_PORT}
    pool /{partition}/backend_pool
    profiles {{
        /Common/tcp {{ }}
        /Common/http {{ }}
        /Common/clientssl {{ context clientside }}
        /Common/serverssl {{ context serverside }}
    }}
    policies {{
        /{partition}/http_router {{ }}
    }}
    fw-enforced-policy /{partition}/edge_fw
    vlans {{
        /{partition}/client_vlan
    }}
    vlans-enabled
    translate-address disabled
    translate-port disabled
    ip-protocol tcp
}}
net route /{partition}/default_to_t3 {{
    gw 10.3.{backend_octet}.254
    network default
}}
"""


T2_CONFS = {cluster: _render_t2(cluster) for cluster in _T2_IDS}

T3_CONF = """\
ltm snat-translation /Common/snat_xlat_1 {
    address 198.51.100.101
}
ltm snat-translation /Common/snat_xlat_2 {
    address 198.51.100.102
}
ltm snat-translation /Common/snat_xlat_3 {
    address 198.51.100.103
}
ltm snatpool /Common/internet_snat {
    members {
        /Common/snat_xlat_1
        /Common/snat_xlat_2
        /Common/snat_xlat_3
    }
}
ltm node /Common/upstream_gw {
    address 198.51.100.1
}
ltm pool /Common/internet_uplink_pool {
    members {
        /Common/upstream_gw:0 {
            address 198.51.100.1
        }
    }
    monitor /Common/gateway_icmp
}
ltm virtual /Common/internet_egress_vs {
    destination /Common/0.0.0.0:0
    pool /Common/internet_uplink_pool
    profiles {
        /Common/fastL4 { }
    }
    source-address-translation {
        type snat
        pool /Common/internet_snat
    }
    ip-forward
    ip-protocol any
}
"""


def _topology_sources() -> dict[str, str]:
    sources = {
        "mem://gtm.conf": GTM_CONF,
        "mem://t1.conf": T1_CONF,
        "mem://t3.conf": T3_CONF,
    }
    for cluster, conf in T2_CONFS.items():
        sources[f"mem://t2-{cluster}.conf"] = conf
    return sources


def _run_merge(query: str):
    return run_query(query, _topology_sources(), merge=True)


def test_gtm_public_answers_model_border_nat_entrypoints():
    result = run_query(
        ".gtm.server[].virtual-servers",
        {"mem://gtm.conf": GTM_CONF},
    )
    [answers] = result.values_per_file["mem://gtm.conf"]
    assert sorted(answers) == ["203.0.113.10:443", "203.0.113.11:443"]


def test_gtm_wideip_deep_links_to_public_pool_members():
    query = (
        ".gtm.wideip[] as $wide "
        "| $wide.pools[] as $pool "
        '| tsv($wide.name, $pool.name, join($pool.members, ","), source_file($pool))'
    )
    result = run_query(query, {"mem://gtm.conf": GTM_CONF})
    assert result.values_per_file["mem://gtm.conf"] == [
        "www.example.com\twww_pool\t/Common/border-fw:public_a,/Common/border-fw:public_b\tmem://gtm.conf"
    ]


def test_t1_fanout_pool_has_one_member_per_t2_cluster():
    result = run_query(
        '.ltm.pool["/Common/t2_fanout_pool"].members',
        {"mem://t1.conf": T1_CONF},
    )
    [members] = result.values_per_file["mem://t1.conf"]
    assert len(members) == _T2_COUNT
    names = sorted(member.fields["name"] for member in members)
    assert names[0] == "/Common/t2_c01_vip:443"
    assert names[-1] == "/Common/t2_c12_vip:443"


def test_t1_is_ha_and_t3_is_not_ha():
    result = run_query(
        "[.cm.device-group[]] | count",
        {
            "mem://t1.conf": T1_CONF,
            "mem://t3.conf": T3_CONF,
        },
    )
    assert result.values_per_file == {
        "mem://t1.conf": [1],
        "mem://t3.conf": [0],
    }


def test_all_t2_clusters_are_ha():
    result = run_query(
        ".cm.device-group[].type",
        {f"mem://t2-{cluster}.conf": conf for cluster, conf in T2_CONFS.items()},
    )
    assert set(result.values_per_file) == {f"mem://t2-{cluster}.conf" for cluster in _T2_IDS}
    assert all(types == ["sync-failover"] for types in result.values_per_file.values())


def test_twelve_t2_virtuals_share_host_and_port_but_have_distinct_vlans():
    query = (
        ".ltm.virtual[] "
        f'| select((.destination | host) == "{_SHARED_T2_VIP}") '
        "| . as $vs "
        "| $vs.vlans[] as $vlan "
        "| tsv(partition(path($vs)), ($vs.destination | host), "
        "str($vs.destination | port), str($vlan.tag), $vlan.name, source_file($vs))"
    )
    result = _run_merge(query)
    rows = next(iter(result.values_per_file.values()))
    assert len(rows) == _T2_COUNT

    partitions = set()
    vlan_tags = set()
    for row in rows:
        partition, host, port, tag, vlan_name, source = row.split("\t")
        partitions.add(partition)
        vlan_tags.add(tag)
        assert host == _SHARED_T2_VIP
        assert port == _SHARED_T2_PORT
        assert vlan_name == "client_vlan"
        assert source.startswith("mem://t2-c")
    assert len(partitions) == _T2_COUNT
    assert sorted(vlan_tags) == [str(201 + i) for i in range(_T2_COUNT)]


def test_t2_virtuals_disable_address_and_port_translation():
    query = (
        ".ltm.virtual[] "
        f'| select((.destination | host) == "{_SHARED_T2_VIP}") '
        '| tsv(partition(path(.)), ."translate-address", ."translate-port")'
    )
    result = _run_merge(query)
    rows = next(iter(result.values_per_file.values()))
    assert len(rows) == _T2_COUNT
    assert all(row.endswith("\tdisabled\tdisabled") for row in rows)


def test_t2_tls_http_profiles_and_http_policy_routing_are_visible():
    query = (
        ".ltm.virtual[] "
        f'| select((.destination | host) == "{_SHARED_T2_VIP}") '
        "| . as $vs "
        "| $vs.policies[].rules[] as $rule "
        '| select($rule.name == "api") '
        "| tsv(partition(path($vs)), "
        'join(([$vs.profiles[].name] | sort), ","), '
        "$rule.conditions[].operand, $rule.conditions[].operator, "
        "$rule.actions[].pool.full-path)"
    )
    result = _run_merge(query)
    rows = next(iter(result.values_per_file.values()))
    assert len(rows) == _T2_COUNT
    for row in rows:
        partition, profiles, operand, operator, pool_path = row.split("\t")
        assert profiles == "clientssl,http,serverssl,tcp"
        assert operand == "http-uri"
        assert operator == "starts-with"
        assert pool_path == f"/{partition}/backend_pool"


def test_t2_security_policy_rule_lists_deep_link():
    query = (
        ".security.firewall-policy[] as $policy "
        "| $policy.rule-lists[] as $rule_list "
        "| tsv(partition(path($policy)), $policy.name, $rule_list.name, "
        'join($rule_list.rules, ","))'
    )
    result = _run_merge(query)
    rows = next(iter(result.values_per_file.values()))
    assert len(rows) == _T2_COUNT
    assert all(row.endswith("\tedge_fw\tallow_web\tallow_tls") for row in rows)


def test_t3_is_the_only_snat_pool_owner():
    result = run_query(".ltm.snatpool[].name", _topology_sources())
    owners = {uri: names for uri, names in result.values_per_file.items() if names}
    assert owners == {"mem://t3.conf": ["internet_snat"]}


def test_t3_snatpool_members_are_translation_refs():
    result = run_query(
        '.ltm.snatpool["/Common/internet_snat"].members',
        {"mem://t3.conf": T3_CONF},
    )
    [members] = result.values_per_file["mem://t3.conf"]
    assert sorted(str(member) for member in members) == [
        "/Common/snat_xlat_1",
        "/Common/snat_xlat_2",
        "/Common/snat_xlat_3",
    ]


def test_topology_census_across_all_layers():
    """Roll up per-kind counts across every loaded source.  Merge
    mode evaluates per-source (not as one aggregate), so the
    census is computed in Python by summing each source's count
    rather than asking the engine for a single object literal —
    matching how a real audit would walk a file tree."""
    sources = _topology_sources()
    counts = {
        "virtuals": 0,
        "t2_virtuals": 0,
        "pools": 0,
        "device_groups": 0,
        "snat_pools": 0,
        "firewall_policies": 0,
        "http_policies": 0,
    }
    for uri, source in sources.items():
        per_file = {
            "virtuals": ".ltm.virtual[].name",
            "t2_virtuals": (
                f'.ltm.virtual[] | select((.destination | host) == "{_SHARED_T2_VIP}") | .name'
            ),
            "pools": ".ltm.pool[].name",
            "device_groups": ".cm.device-group[].name",
            "snat_pools": ".ltm.snatpool[].name",
            "firewall_policies": ".security.firewall-policy[].name",
            "http_policies": ".ltm.policy[].name",
        }
        for key, query in per_file.items():
            result = run_query(query, {uri: source})
            counts[key] += len(result.values_per_file.get(uri, []))
    assert counts == {
        "device_groups": 1 + _T2_COUNT,
        "firewall_policies": _T2_COUNT,
        "http_policies": _T2_COUNT,
        "pools": 1 + _T2_COUNT + 1,
        "snat_pools": 1,
        "t2_virtuals": _T2_COUNT,
        "virtuals": 1 + _T2_COUNT + 1,
    }


def test_t1_fanout_targets_one_node_per_t2_cluster():
    """The "T1 fans out to every T2 cluster" invariant.  T1's
    pool has one member per T2 cluster, and each member's *name*
    encodes the cluster id (``t2_c01_vip``..``t2_c12_vip``).
    Per-cluster, the T2 VS destination is the same shared VIP
    (``10.2.0.10:443``) — proven by separate per-file queries
    because the engine's merge refuses cross-file name
    collisions inside the per-cluster partitions."""
    t1 = run_query(
        '.ltm.pool["/Common/t2_fanout_pool"].members[].name',
        {"mem://t1.conf": T1_CONF},
    )
    member_names = sorted(str(n) for n in t1.values_per_file["mem://t1.conf"])
    # Twelve members, one per T2 cluster.
    assert len(member_names) == _T2_COUNT
    # Cluster id is encoded in each member name.
    cluster_tags = sorted(name.split("/")[-1].split(":")[0] for name in member_names)
    assert cluster_tags == sorted(f"t2_{c}_vip" for c in _T2_IDS)

    # Every T2 cluster's VS sits on the shared VIP.
    for cluster in _T2_IDS:
        partition = _partition(cluster)
        result = run_query(
            ".ltm.virtual[].destination",
            {f"mem://t2-{cluster}.conf": T2_CONFS[cluster]},
        )
        [dest] = result.values_per_file[f"mem://t2-{cluster}.conf"]
        # /<partition>/10.2.0.10:443
        assert str(dest) == f"/{partition}/{_SHARED_T2_VIP}:{_SHARED_T2_PORT}"


def test_t1_pool_member_addresses_preserve_route_domain_qualifier():
    """T1's fanout pool uses ``%RD`` to distinguish per-cluster
    routes to a shared backend VIP.  The address parser must
    round-trip the suffix so a query on ``.members[].address``
    returns the canonical ``<ip>%<rd>`` form."""
    result = run_query(
        '.ltm.pool["/Common/t2_fanout_pool"].members[].address',
        {"mem://t1.conf": T1_CONF},
    )
    addrs = result.values_per_file["mem://t1.conf"]
    # Twelve addresses, every one ``10.2.0.10%<rd>`` where rd
    # ranges 101..112 (one per T2 cluster).
    assert len(addrs) == _T2_COUNT
    bases = {addr.split("%", 1)[0] for addr in addrs}
    assert bases == {_SHARED_T2_VIP}
    rds = sorted(int(addr.split("%", 1)[1]) for addr in addrs)
    assert rds == list(range(101, 101 + _T2_COUNT))


def test_t1_nodes_carry_route_domain_metadata():
    """The standalone ``ltm node`` stanzas that the fanout pool
    references also preserve their ``%RD`` qualifier through the
    parser → DSL projection round-trip."""
    result = run_query(
        ".ltm.node[].address",
        {"mem://t1.conf": T1_CONF},
    )
    addrs = result.values_per_file["mem://t1.conf"]
    assert len(addrs) == _T2_COUNT
    assert all("%" in a for a in addrs)
    bases = {a.split("%", 1)[0] for a in addrs}
    assert bases == {_SHARED_T2_VIP}
