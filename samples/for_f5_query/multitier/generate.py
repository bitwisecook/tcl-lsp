"""Generate the multi-device F5 query sample topology.

The generated files model physically separate devices:

- one GTM device;
- one tier-1 LTM HA pair config;
- twelve tier-2 LTM HA pair configs, each on a separate device;
- one standalone tier-3 reaggregation LTM; and
- one JSON file for the assumed border-firewall NAT.

Run from the repository root with:

    python samples/for_f5_query/multitier/generate.py
"""

from __future__ import annotations

import json
from pathlib import Path

T2_COUNT = 12
T2_IDS = tuple(f"c{i:02d}" for i in range(1, T2_COUNT + 1))
SHARED_T2_VIP = "10.2.0.10"
SHARED_T2_PORT = "443"


def _write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def _gtm() -> str:
    return """\
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


def _tier1() -> str:
    route_domains: list[str] = []
    nodes: list[str] = []
    members: list[str] = []
    for index, cluster in enumerate(T2_IDS, start=1):
        rd = 100 + index
        route_domains.append(
            f"""net route-domain /Common/to_t2_{cluster} {{
    id {rd}
}}
"""
        )
        nodes.append(
            f"""ltm node /Common/t2_{cluster}_vip {{
    address {SHARED_T2_VIP}%{rd}
}}
"""
        )
        members.append(
            f"""        /Common/t2_{cluster}_vip:{SHARED_T2_PORT} {{
            address {SHARED_T2_VIP}%{rd}
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
    auto-sync enabled
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
        /Common/clientssl {{ context clientside }}
        /Common/serverssl {{ context serverside }}
    }}
    ip-protocol tcp
}}
"""
    )


def _tier2(cluster: str) -> str:
    index = int(cluster[1:])
    rd = 100 + index
    vlan_tag = 200 + index
    backend_octet = 20 + index
    return f"""cm device /Common/t2-{cluster}-a {{
    hostname t2-{cluster}-a.example.test
    management-ip 192.0.2.{backend_octet}
}}
cm device /Common/t2-{cluster}-b {{
    hostname t2-{cluster}-b.example.test
    management-ip 192.0.2.{backend_octet + 100}
}}
cm device-group /Common/t2-{cluster}-failover {{
    type sync-failover
    devices {{
        /Common/t2-{cluster}-a {{ }}
        /Common/t2-{cluster}-b {{ }}
    }}
}}
net vlan /Common/client_vlan {{
    interfaces {{
        1.{index} {{ }}
    }}
    tag {vlan_tag}
}}
net route-domain /Common/client_rd {{
    id {rd}
    vlans {{
        /Common/client_vlan
    }}
}}
security firewall rule-list /Common/allow_web {{
    rules {{
        allow_tls {{
            action accept
            ip-protocol tcp
        }}
    }}
}}
security firewall policy /Common/edge_fw {{
    rules {{
        allow_web_binding {{ rule-list /Common/allow_web }}
    }}
}}
ltm pool /Common/backend_pool {{
    members {{
        /Common/backend_1:8443 {{
            address 10.3.{backend_octet}.10
        }}
        /Common/backend_2:8443 {{
            address 10.3.{backend_octet}.11
        }}
    }}
    monitor /Common/tcp
}}
ltm policy /Common/http_router {{
    controls {{ forwarding }}
    requires {{ http }}
    rules {{
        api {{
            actions {{
                0 {{
                    forward
                    select
                    pool /Common/backend_pool
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
                    pool /Common/backend_pool
                }}
            }}
            ordinal 20
        }}
    }}
}}
ltm virtual /Common/app_vs {{
    destination /Common/{SHARED_T2_VIP}:{SHARED_T2_PORT}
    pool /Common/backend_pool
    profiles {{
        /Common/tcp {{ }}
        /Common/http {{ }}
        /Common/clientssl {{ context clientside }}
        /Common/serverssl {{ context serverside }}
    }}
    policies {{
        /Common/http_router {{ }}
    }}
    fw-enforced-policy /Common/edge_fw
    vlans {{
        /Common/client_vlan
    }}
    vlans-enabled
    translate-address disabled
    translate-port disabled
    ip-protocol tcp
}}
net route /Common/default_to_t3 {{
    gw 10.3.{backend_octet}.254
    network default
}}
"""


def _tier3() -> str:
    return """\
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


def main() -> None:
    root = Path(__file__).resolve().parent
    _write(root / "gtm-dns.conf", _gtm())
    _write(root / "tier1-ltm-ha.conf", _tier1())
    _write(root / "tier3-reaggregator.conf", _tier3())
    for cluster in T2_IDS:
        _write(root / f"tier2-{cluster}-ltm-ha.conf", _tier2(cluster))
    nat_rows = [
        {
            "service": "www.example.com",
            "public": "203.0.113.10:443",
            "internal": "10.1.0.10:443",
            "device": "border-fw-a",
        },
        {
            "service": "www.example.com",
            "public": "203.0.113.11:443",
            "internal": "10.1.0.10:443",
            "device": "border-fw-b",
        },
    ]
    _write(root / "border-nat.json", json.dumps(nat_rows, indent=2) + "\n")


if __name__ == "__main__":
    main()
