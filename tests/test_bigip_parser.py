"""Tests for the BIG-IP configuration parser."""

from __future__ import annotations

from core.bigip.model import DataGroupType, ProfileType
from core.bigip.parser import parse_bigip_conf

# Basic parsing


def test_parse_empty():
    config = parse_bigip_conf("")
    assert not config.data_groups
    assert not config.pools
    assert not config.virtual_servers


def test_parse_comments_only():
    config = parse_bigip_conf("# just a comment\n# another comment\n")
    assert not config.pools


# Data groups


def test_parse_data_group_internal():
    source = """\
ltm data-group internal /Common/my_dg {
    records {
        foo { }
        bar { }
    }
    type string
}
"""
    config = parse_bigip_conf(source)
    assert "/Common/my_dg" in config.data_groups
    dg = config.data_groups["/Common/my_dg"]
    assert dg.name == "my_dg"
    assert dg.kind == DataGroupType.INTERNAL
    assert dg.value_type == "string"
    assert "foo" in dg.records
    assert "bar" in dg.records


def test_parse_data_group_external():
    source = """\
ltm data-group external /Common/ext_dg {
    external-file-name /config/filestore/ext_dg
    type ip
}
"""
    config = parse_bigip_conf(source)
    assert "/Common/ext_dg" in config.data_groups
    dg = config.data_groups["/Common/ext_dg"]
    assert dg.kind == DataGroupType.EXTERNAL


def test_parse_data_group_with_values():
    source = """\
ltm data-group internal /Common/redirect_map {
    records {
        /old-path {
            data "/new-path"
        }
        /legacy {
            data "/modern"
        }
    }
    type string
}
"""
    config = parse_bigip_conf(source)
    dg = config.data_groups["/Common/redirect_map"]
    assert "/old-path" in dg.records
    assert "/legacy" in dg.records


# Pools


def test_parse_pool():
    source = """\
ltm pool /Common/web_pool {
    members {
        /Common/web1:80 {
            address 10.0.1.10
        }
        /Common/web2:80 {
            address 10.0.1.11
        }
    }
    monitor /Common/http
    load-balancing-mode round-robin
}
"""
    config = parse_bigip_conf(source)
    assert "/Common/web_pool" in config.pools
    pool = config.pools["/Common/web_pool"]
    assert pool.name == "web_pool"
    assert pool.module == "ltm"
    assert len(pool.members) == 2
    assert pool.monitor == "/Common/http"
    assert pool.load_balancing_mode == "round-robin"


def test_parse_pool_no_members():
    source = """\
ltm pool /Common/empty_pool {
    monitor /Common/http
}
"""
    config = parse_bigip_conf(source)
    pool = config.pools["/Common/empty_pool"]
    assert len(pool.members) == 0


# Virtual servers


def test_parse_virtual_server():
    source = """\
ltm virtual /Common/my_vs {
    destination /Common/10.0.0.1:443
    pool /Common/web_pool
    profiles {
        /Common/http { }
        /Common/clientssl {
            context clientside
        }
    }
    rules {
        /Common/my_irule
        /Common/my_other_irule
    }
    persist {
        /Common/cookie {
            default yes
        }
    }
}
"""
    config = parse_bigip_conf(source)
    assert "/Common/my_vs" in config.virtual_servers
    vs = config.virtual_servers["/Common/my_vs"]
    assert vs.name == "my_vs"
    assert vs.pool == "/Common/web_pool"
    assert vs.destination == "/Common/10.0.0.1:443"
    assert "/Common/my_irule" in vs.rules
    assert "/Common/my_other_irule" in vs.rules
    assert "/Common/http" in vs.profiles
    assert "/Common/clientssl" in vs.profiles
    assert "/Common/cookie" in vs.persist


def test_parse_virtual_server_source_addr_translation():
    source = """\
ltm virtual /Common/snat_vs {
    destination /Common/10.0.0.1:80
    source-address-translation {
        type snat
        pool /Common/my_snatpool
    }
}
"""
    config = parse_bigip_conf(source)
    vs = config.virtual_servers["/Common/snat_vs"]
    assert vs.snatpool == "/Common/my_snatpool"
    assert vs.source_address_translation == "snat"


# Profiles


def test_parse_profile_http():
    source = """\
ltm profile http /Common/my_http {
    defaults-from /Common/http
    insert-xforwarded-for enabled
}
"""
    config = parse_bigip_conf(source)
    assert "/Common/my_http" in config.profiles
    p = config.profiles["/Common/my_http"]
    assert p.profile_type == ProfileType.HTTP


def test_parse_profile_client_ssl():
    source = """\
ltm profile client-ssl /Common/my_clientssl {
    defaults-from /Common/clientssl
}
"""
    config = parse_bigip_conf(source)
    p = config.profiles["/Common/my_clientssl"]
    assert p.profile_type == ProfileType.CLIENT_SSL


def test_parse_profile_tcp():
    source = """\
ltm profile tcp /Common/my_tcp {
    defaults-from /Common/tcp
}
"""
    config = parse_bigip_conf(source)
    assert config.profiles["/Common/my_tcp"].profile_type == ProfileType.TCP


# Nodes, monitors, snatpools, persistence


def test_parse_node():
    source = """\
ltm node /Common/web1 {
    address 10.0.1.10
}
"""
    config = parse_bigip_conf(source)
    assert "/Common/web1" in config.nodes
    assert config.nodes["/Common/web1"].address == "10.0.1.10"


def test_parse_monitor():
    source = """\
ltm monitor http /Common/my_monitor {
    defaults-from /Common/http
    interval 10
}
"""
    config = parse_bigip_conf(source)
    assert "/Common/my_monitor" in config.monitors
    assert config.monitors["/Common/my_monitor"].monitor_type == "http"


def test_parse_snatpool():
    source = """\
ltm snatpool /Common/my_snatpool {
    members {
        10.0.2.100
        10.0.2.101
    }
}
"""
    config = parse_bigip_conf(source)
    sp = config.snat_pools["/Common/my_snatpool"]
    assert len(sp.members) == 2


def test_parse_persistence():
    source = """\
ltm persistence cookie /Common/my_cookie {
    defaults-from /Common/cookie
    cookie-name "SERVERID"
}
"""
    config = parse_bigip_conf(source)
    assert "/Common/my_cookie" in config.persistence
    assert config.persistence["/Common/my_cookie"].persistence_type == "cookie"


# iRules


def test_parse_rule():
    source = """\
ltm rule /Common/my_irule {
    when HTTP_REQUEST {
        pool /Common/web_pool
    }
}
"""
    config = parse_bigip_conf(source)
    assert "/Common/my_irule" in config.rules
    rule = config.rules["/Common/my_irule"]
    assert rule.name == "my_irule"
    assert "HTTP_REQUEST" in rule.source
    assert "pool /Common/web_pool" in rule.source


def test_parse_gtm_rule():
    source = """\
gtm rule /Common/my_gtm_rule {
    when DNS_REQUEST {
        log local0. "DNS query received"
    }
}
"""
    config = parse_bigip_conf(source)
    assert "/Common/my_gtm_rule" in config.rules


def test_parse_gtm_pool_tracks_module():
    source = """\
gtm pool /Common/dns_pool {
    members { /Common/dc1_vs1:53 { } }
}
"""
    config = parse_bigip_conf(source)
    assert "/Common/dns_pool" not in config.pools
    assert (
        config.resolve_generic_object(
            "/Common/dns_pool",
            module="gtm",
            object_types=("pool",),
        )
        is not None
    )


# Name resolution


def test_resolve_pool_full_path():
    source = """\
ltm pool /Common/web_pool {
    monitor /Common/http
}
"""
    config = parse_bigip_conf(source)
    assert config.resolve_pool("/Common/web_pool") == "/Common/web_pool"


def test_resolve_pool_short_name():
    source = """\
ltm pool /Common/web_pool {
    monitor /Common/http
}
"""
    config = parse_bigip_conf(source)
    assert config.resolve_pool("web_pool") == "/Common/web_pool"


def test_resolve_pool_not_found():
    config = parse_bigip_conf("")
    assert config.resolve_pool("nonexistent") is None


# Multi-object config


def test_parse_full_config():
    """Parse the example bigip.conf and verify object counts."""
    import os

    conf_path = os.path.join(os.path.dirname(__file__), "..", "examples", "bigip", "bigip.conf")
    with open(conf_path) as f:
        source = f.read()
    config = parse_bigip_conf(source)
    assert len(config.data_groups) == 4
    assert len(config.pools) == 3
    assert len(config.virtual_servers) == 4
    assert len(config.nodes) == 3
    assert len(config.profiles) == 5
    assert len(config.monitors) == 1
    assert len(config.snat_pools) == 1
    assert len(config.persistence) == 2
    assert len(config.rules) == 5


# Ranges


def test_parsed_objects_have_ranges():
    source = """\
ltm pool /Common/p1 {
    monitor /Common/http
}
ltm rule /Common/r1 {
    when HTTP_REQUEST { pool p1 }
}
"""
    config = parse_bigip_conf(source)
    assert config.pools["/Common/p1"].range is not None
    assert config.rules["/Common/r1"].range is not None


def test_virtual_pool_reference_has_range():
    source = """\
ltm pool /Common/web_pool {
    monitor /Common/http
}
ltm virtual /Common/www_vs {
    destination /Common/10.0.0.1:443
    pool /Common/web_pool
}
"""
    config = parse_bigip_conf(source)
    vs = config.virtual_servers["/Common/www_vs"]
    assert vs.pool_range is not None
    line = source.splitlines()[vs.pool_range.start.line]
    token = line[vs.pool_range.start.character : vs.pool_range.end.character + 1]
    assert token == "/Common/web_pool"


def test_parse_example_bigip_base_conf():
    """Parse the example bigip_base.conf and verify key object extraction."""
    import os

    conf_path = os.path.join(
        os.path.dirname(__file__), "..", "examples", "bigip", "bigip_base.conf"
    )
    with open(conf_path) as f:
        source = f.read()

    config = parse_bigip_conf(source)
    assert "/Common/web_pool" in config.pools
    assert "/Common/app_https_vs" in config.virtual_servers


def test_parse_generic_objects_from_non_ltm_modules():
    source = """\
auth partition foo { }
auth user admin { }
net route-domain /Common/0 {
    id 0
}
cm traffic-group /Common/traffic-group-1 { }
ltm virtual-address /Common/192.0.2.10 {
    traffic-group /Common/traffic-group-1
}
"""
    config = parse_bigip_conf(source)
    assert (
        config.resolve_generic_object("foo", module="auth", object_types=("partition",)) is not None
    )
    assert config.resolve_generic_object("admin", module="auth", object_types=("user",)) is not None
    assert (
        config.resolve_generic_object(
            "/Common/0",
            module="net",
            object_types=("route-domain",),
        )
        is not None
    )
    assert (
        config.resolve_generic_object(
            "/Common/traffic-group-1",
            module="cm",
            object_types=("traffic-group",),
        )
        is not None
    )


def test_parse_generic_object_with_multiword_type():
    source = """\
security firewall policy /Common/fw_pol_1 {
    rules { }
}
"""
    config = parse_bigip_conf(source)
    resolved = config.resolve_generic_object(
        "/Common/fw_pol_1",
        module="security",
        object_types=("firewall policy",),
    )
    assert resolved is not None
