"""Unit tests for the typed BIG-IP scalar values in ``core.bigip.types``."""

from __future__ import annotations

import pytest

from core.bigip.types import (
    FQDN,
    Destination,
    Folder,
    IPAddress,
    Network,
    ObjectPath,
    Partition,
    Port,
    PortRange,
    RouteDomain,
)

# ---------------------------------------------------------------------------
# IPAddress
# ---------------------------------------------------------------------------


class TestIPAddress:
    def test_parse_ipv4(self):
        ip = IPAddress.parse("10.0.0.1")
        assert ip.is_ipv4
        assert not ip.is_ipv6
        assert str(ip) == "10.0.0.1"

    def test_parse_ipv6_full(self):
        ip = IPAddress.parse("2001:db8:0:0:0:0:0:1")
        assert ip.is_ipv6
        # ipaddress canonicalises to compressed form.
        assert str(ip) == "2001:db8::1"

    def test_parse_ipv6_compressed(self):
        ip = IPAddress.parse("2001:db8::1")
        assert ip.is_ipv6
        assert str(ip) == "2001:db8::1"

    def test_parse_loopback_classification(self):
        assert IPAddress.parse("127.0.0.1").is_loopback
        assert IPAddress.parse("::1").is_loopback

    def test_parse_unspecified_is_wildcard(self):
        assert IPAddress.parse("0.0.0.0").is_unspecified
        assert IPAddress.parse("::").is_unspecified

    def test_parse_invalid_raises(self):
        with pytest.raises(ValueError):
            IPAddress.parse("not.an.ip")

    def test_try_parse_returns_none(self):
        assert IPAddress.try_parse("not.an.ip") is None
        assert IPAddress.try_parse("10.0.0.1") is not None

    def test_parse_ipv4_with_route_domain(self):
        ip = IPAddress.parse("10.0.0.1%10")
        assert ip.is_ipv4
        assert ip.route_domain == 10
        # The string form round-trips the suffix verbatim.
        assert str(ip) == "10.0.0.1%10"

    def test_parse_ipv6_with_route_domain(self):
        ip = IPAddress.parse("2001:db8::1%50")
        assert ip.is_ipv6
        assert ip.route_domain == 50
        assert str(ip) == "2001:db8::1%50"

    def test_parse_default_route_domain_is_none(self):
        ip = IPAddress.parse("10.0.0.1")
        # ``None`` distinguishes "default partition / RD-0" from an
        # explicit ``%0`` qualifier — the latter would set
        # ``route_domain == 0``.
        assert ip.route_domain is None

    def test_parse_route_domain_zero_is_preserved(self):
        ip = IPAddress.parse("10.0.0.1%0")
        assert ip.route_domain == 0
        assert str(ip) == "10.0.0.1%0"

    def test_parse_non_numeric_suffix_falls_through(self):
        # IPv6 zone-id form ``fe80::1%eth0`` — F5's ``%RD`` is
        # always numeric so the parser doesn't strip a non-numeric
        # suffix.  Python's ``ipaddress.ip_address`` accepts the
        # zone-id and preserves it on the address, so
        # ``route_domain`` stays ``None`` while the zone-id
        # round-trips on the underlying stdlib value.
        ip = IPAddress.parse("fe80::1%eth0")
        assert ip.route_domain is None
        # ``ipaddress`` stores the zone-id on the address itself.
        assert ip.addr.compressed == "fe80::1%eth0"


# ---------------------------------------------------------------------------
# FQDN
# ---------------------------------------------------------------------------


class TestFQDN:
    def test_parse_simple(self):
        f = FQDN.parse("host.example.com")
        assert str(f) == "host.example.com"

    def test_parse_subdomain(self):
        f = FQDN.parse("api.v2.host.example.com")
        assert str(f) == "api.v2.host.example.com"

    def test_parse_rejects_single_label(self):
        with pytest.raises(ValueError):
            FQDN.parse("host")

    def test_parse_rejects_empty_label(self):
        with pytest.raises(ValueError):
            FQDN.parse("host..example.com")

    def test_parse_rejects_leading_dot(self):
        with pytest.raises(ValueError):
            FQDN.parse(".example.com")

    def test_parse_rejects_leading_hyphen_in_label(self):
        with pytest.raises(ValueError):
            FQDN.parse("-foo.example.com")

    def test_parse_rejects_ipv4_lookalike(self):
        with pytest.raises(ValueError):
            FQDN.parse("10.0.0.1")


# ---------------------------------------------------------------------------
# Network
# ---------------------------------------------------------------------------


class TestNetwork:
    def test_parse_ipv4(self):
        n = Network.parse("10.0.0.0/24")
        assert n.is_ipv4
        assert n.prefix_length == 24

    def test_parse_ipv6(self):
        n = Network.parse("2001:db8::/32")
        assert n.is_ipv6
        assert n.prefix_length == 32

    def test_parse_loose_keeps_original_spelling(self):
        # Loose parse stores the canonical (host-bits-masked) form on
        # ``.network`` so membership / supernet checks flow through
        # the stdlib, but ``str()`` and ``.original`` preserve the
        # user's spelling — ``203.0.113.5/24`` round-trips intact on
        # net-self interface addresses where the host bits matter.
        n = Network.parse("10.0.0.5/24")
        assert n.network.with_prefixlen == "10.0.0.0/24"
        assert n.original == "10.0.0.5/24"
        assert str(n) == "10.0.0.5/24"

    def test_parse_strict_rejects_host_bits(self):
        with pytest.raises(ValueError):
            Network.parse("10.0.0.5/24", strict=True)

    def test_membership(self):
        n = Network.parse("10.0.0.0/8")
        assert IPAddress.parse("10.0.0.1") in n
        assert IPAddress.parse("11.0.0.1") not in n

    def test_parse_dotted_quad_netmask(self):
        # F5 ``net route`` / ``net self`` configs sometimes spell the
        # mask as a dotted quad rather than a prefix length.  Loose
        # parse keeps the original spelling on ``.original`` so the
        # rewritten config re-emits the same form, while
        # ``.network.with_prefixlen`` gives the canonical CIDR for
        # membership / supernet logic.
        n = Network.parse("10.0.0.0/255.255.255.0")
        assert n.is_ipv4
        assert n.prefix_length == 24
        assert n.network.with_prefixlen == "10.0.0.0/24"
        assert n.original == "10.0.0.0/255.255.255.0"

    def test_parse_space_separated_addr_mask(self):
        # ``net route`` style: ``ADDR MASK`` with a space separator.
        n = Network.parse("10.0.0.0 255.255.0.0")
        assert n.is_ipv4
        assert n.prefix_length == 16

    def test_parse_dotted_quad_netmask_keeps_host_bits(self):
        n = Network.parse("10.0.0.5/255.255.255.0")
        assert n.network.with_prefixlen == "10.0.0.0/24"
        assert n.original == "10.0.0.5/255.255.255.0"


# ---------------------------------------------------------------------------
# Port
# ---------------------------------------------------------------------------


class TestPort:
    def test_parse_normal_port(self):
        p = Port.parse("80")
        assert p.port == 80
        assert not p.is_any
        assert str(p) == "80"

    def test_parse_any_token(self):
        for token in ("any", "*", "0", "ANY"):
            p = Port.parse(token)
            assert p.is_any
            # Spelling round-trip preserved.
            assert str(p) == token

    def test_parse_out_of_range_raises(self):
        with pytest.raises(ValueError):
            Port.parse("70000")
        with pytest.raises(ValueError):
            Port.parse("-5")

    def test_parse_non_numeric_raises(self):
        with pytest.raises(ValueError):
            Port.parse("http")


class TestPortRange:
    def test_parse_range(self):
        r = PortRange.parse("1024-65535")
        assert r.low == 1024
        assert r.high == 65535
        assert str(r) == "1024-65535"

    def test_membership(self):
        r = PortRange.parse("80-100")
        assert Port.parse("80") in r
        assert Port.parse("100") in r
        assert Port.parse("79") not in r
        assert Port.parse("101") not in r

    def test_reverse_range_rejected(self):
        with pytest.raises(ValueError):
            PortRange.parse("100-80")

    def test_single_port_form_rejected(self):
        with pytest.raises(ValueError):
            PortRange.parse("80")


# ---------------------------------------------------------------------------
# Partition
# ---------------------------------------------------------------------------


class TestPartition:
    def test_parse_with_leading_slash(self):
        p = Partition.parse("/Common")
        assert str(p) == "/Common"
        assert p.short_name == "Common"
        assert p.is_common

    def test_parse_bare(self):
        assert Partition.parse("Tenant_A").short_name == "Tenant_A"

    def test_parse_rejects_nested_path(self):
        with pytest.raises(ValueError):
            Partition.parse("/Common/sub")

    def test_parse_rejects_empty(self):
        with pytest.raises(ValueError):
            Partition.parse("")
        with pytest.raises(ValueError):
            Partition.parse("/")


# ---------------------------------------------------------------------------
# RouteDomain
# ---------------------------------------------------------------------------


class TestRouteDomain:
    def test_parse_with_percent(self):
        rd = RouteDomain.parse("%5")
        assert rd.id == 5
        assert str(rd) == "%5"

    def test_parse_bare(self):
        assert RouteDomain.parse("5").id == 5

    def test_default_is_zero(self):
        assert RouteDomain.parse("0").is_default

    def test_negative_rejected(self):
        with pytest.raises(ValueError):
            RouteDomain.parse("-1")


# ---------------------------------------------------------------------------
# Destination — every documented F5 spelling
# ---------------------------------------------------------------------------


class TestDestination:
    def test_partition_ipv4_port(self):
        d = Destination.parse("/Common/10.0.0.1:80")
        assert isinstance(d.address, IPAddress) and d.address.is_ipv4
        assert d.port.port == 80
        # ``partition`` is a convenience property over ``folder``.
        assert d.partition is not None and d.partition.short_name == "Common"
        assert d.folder is not None and d.folder.is_root
        assert str(d) == "/Common/10.0.0.1:80"

    def test_folder_nested_ipv4_port(self):
        # ``/Common/Application_X/10.0.0.1:80`` — partition + one
        # folder segment + IPv4 + port.
        d = Destination.parse("/Common/Application_X/10.0.0.1:80")
        assert isinstance(d.address, IPAddress) and d.address.is_ipv4
        assert d.port.port == 80
        assert d.folder is not None
        assert d.folder.partition.short_name == "Common"
        assert d.folder.segments == ("Application_X",)
        assert str(d) == "/Common/Application_X/10.0.0.1:80"

    def test_folder_nested_two_levels_ipv6(self):
        d = Destination.parse("/Common/iApps/Tenant.app/[2001:db8::1].443")
        assert d.folder is not None
        assert d.folder.segments == ("iApps", "Tenant.app")
        assert isinstance(d.address, IPAddress) and d.address.is_ipv6
        assert d.port.port == 443
        assert str(d) == "/Common/iApps/Tenant.app/[2001:db8::1].443"

    def test_folder_nested_fqdn_pool_member(self):
        d = Destination.parse("/Common/iApps/Tenant.app/host.example.com:443")
        assert d.folder is not None
        assert d.folder.segments == ("iApps", "Tenant.app")
        assert isinstance(d.address, FQDN)
        assert str(d) == "/Common/iApps/Tenant.app/host.example.com:443"

    def test_bare_ipv4_port(self):
        d = Destination.parse("10.0.0.1:80")
        assert d.partition is None
        assert d.port.port == 80
        assert str(d) == "10.0.0.1:80"

    def test_bracketed_ipv6_dot_port(self):
        d = Destination.parse("/Common/[2001:db8::1].80")
        assert isinstance(d.address, IPAddress) and d.address.is_ipv6
        assert d.port.port == 80
        assert d.ipv6_brackets
        assert d.port_separator == "."
        assert str(d) == "/Common/[2001:db8::1].80"

    def test_bracketed_ipv6_colon_port(self):
        d = Destination.parse("/Common/[2001:db8::1]:80")
        assert d.ipv6_brackets
        assert d.port_separator == ":"
        assert d.port.port == 80
        assert str(d) == "/Common/[2001:db8::1]:80"

    def test_unbracketed_ipv6_dot_port(self):
        d = Destination.parse("2001:db8::1.80")
        assert isinstance(d.address, IPAddress) and d.address.is_ipv6
        assert not d.ipv6_brackets
        assert d.port_separator == "."
        assert d.port.port == 80
        assert str(d) == "2001:db8::1.80"

    def test_ipv4_route_domain_port(self):
        d = Destination.parse("10.0.0.1%5:80")
        assert d.route_domain is not None and d.route_domain.id == 5
        assert d.port.port == 80
        assert str(d) == "10.0.0.1%5:80"

    def test_ipv6_route_domain_dot_port(self):
        d = Destination.parse("2001:db8::1%5.80")
        assert d.route_domain is not None and d.route_domain.id == 5
        assert d.port.port == 80
        assert str(d) == "2001:db8::1%5.80"

    def test_wildcard_port(self):
        d = Destination.parse("/Common/0.0.0.0:any")
        assert isinstance(d.address, IPAddress)
        assert d.address.is_unspecified
        assert d.port.is_any
        assert str(d) == "/Common/0.0.0.0:any"

    def test_pool_member_with_fqdn(self):
        d = Destination.parse("/Common/host.example.com:443")
        assert isinstance(d.address, FQDN)
        assert d.address.name == "host.example.com"
        assert d.port.port == 443
        assert str(d) == "/Common/host.example.com:443"

    def test_pool_member_bare_fqdn(self):
        d = Destination.parse("host.example.com:443")
        assert isinstance(d.address, FQDN)
        assert d.partition is None
        assert str(d) == "host.example.com:443"

    def test_invalid_raises(self):
        with pytest.raises(ValueError):
            Destination.parse("")
        with pytest.raises(ValueError):
            Destination.parse("/Common/[unclosed:80")
        with pytest.raises(ValueError):
            Destination.parse("not-a-host:80")


# ---------------------------------------------------------------------------
# Folder + ObjectPath
# ---------------------------------------------------------------------------


class TestFolder:
    def test_partition_only(self):
        f = Folder.parse("/Common")
        assert f.is_root
        assert f.depth == 0
        assert str(f) == "/Common"

    def test_partition_plus_one_segment(self):
        f = Folder.parse("/Common/Application_X")
        assert not f.is_root
        assert f.depth == 1
        assert f.segments == ("Application_X",)
        assert str(f) == "/Common/Application_X"

    def test_deep_nesting(self):
        f = Folder.parse("/Common/iApps/Tenant.app/sub_a")
        assert f.depth == 3
        assert f.segments == ("iApps", "Tenant.app", "sub_a")
        assert str(f) == "/Common/iApps/Tenant.app/sub_a"

    def test_trailing_slash_tolerated(self):
        assert str(Folder.parse("/Common/")) == "/Common"

    def test_must_start_with_slash(self):
        with pytest.raises(ValueError):
            Folder.parse("Common/Application_X")

    def test_invalid_segment_char(self):
        with pytest.raises(ValueError):
            Folder.parse("/Common/bad segment")

    def test_child_appends_segment(self):
        root = Folder.parse("/Common")
        child = root.child("Application_X")
        assert child.depth == 1
        assert str(child) == "/Common/Application_X"


class TestObjectPath:
    def test_root_partition(self):
        op = ObjectPath.parse("/Common/web_pool")
        assert op.in_root_folder
        assert op.partition.short_name == "Common"
        assert op.name == "web_pool"
        assert str(op) == "/Common/web_pool"

    def test_folder_nested(self):
        op = ObjectPath.parse("/Common/Application_X/web_pool")
        assert not op.in_root_folder
        assert op.folder.segments == ("Application_X",)
        assert op.name == "web_pool"
        assert str(op) == "/Common/Application_X/web_pool"

    def test_deeply_nested(self):
        op = ObjectPath.parse("/Common/iApps/Tenant.app/pool_1")
        assert op.folder.segments == ("iApps", "Tenant.app")
        assert op.name == "pool_1"
        assert str(op) == "/Common/iApps/Tenant.app/pool_1"

    def test_must_have_partition_and_leaf(self):
        with pytest.raises(ValueError):
            ObjectPath.parse("/Common")  # leaf missing
        with pytest.raises(ValueError):
            ObjectPath.parse("web_pool")  # partition missing


# ---------------------------------------------------------------------------
# Typed accessor properties on model dataclasses
# ---------------------------------------------------------------------------


class TestPoolMemberTypedField:
    """``BigipPoolMember.address`` is now a typed :class:`Address`."""

    def test_ipv4_address_is_typed(self):
        from core.bigip.model import BigipPoolMember

        m = BigipPoolMember(
            name="/Common/10.0.0.1:80",
            address=IPAddress.parse("10.0.0.1"),
            port=80,
        )
        assert isinstance(m.address, IPAddress)
        assert m.address.is_ipv4

    def test_fqdn_address_is_typed(self):
        from core.bigip.model import BigipPoolMember

        m = BigipPoolMember(
            name="/Common/host.example.com:443",
            address=FQDN.parse("host.example.com"),
            port=443,
        )
        assert isinstance(m.address, FQDN)

    def test_empty_address_is_none(self):
        from core.bigip.model import BigipPoolMember

        m = BigipPoolMember(name="/Common/x")
        assert m.address is None

    def test_parser_populates_typed_pool_member(self):
        """End-to-end round-trip: a TMSH ``ltm pool`` member parses
        into a typed :class:`Address`, no string detour."""
        from core.bigip.parser import parse_bigip_conf

        src = (
            "ltm pool /Common/p1 {\n"
            "    members {\n"
            "        /Common/10.0.0.1:80 { address 10.0.0.1 }\n"
            "        /Common/host.example.com:443 { address host.example.com }\n"
            "    }\n"
            "}\n"
        )
        config = parse_bigip_conf(src)
        pool = config.pools["/Common/p1"]
        addrs = {m.name: m.address for m in pool.members}
        assert isinstance(addrs["/Common/10.0.0.1:80"], IPAddress)
        assert str(addrs["/Common/10.0.0.1:80"]) == "10.0.0.1"
        assert isinstance(addrs["/Common/host.example.com:443"], FQDN)
        assert str(addrs["/Common/host.example.com:443"]) == "host.example.com"


class TestVirtualServerTypedField:
    """``BigipVirtualServer.destination`` is now a typed :class:`Destination`."""

    def test_destination_is_typed(self):
        from core.bigip.model import BigipVirtualServer

        vs = BigipVirtualServer(
            name="vs1",
            full_path="/Common/vs1",
            destination=Destination.parse("/Common/10.0.0.1:443"),
        )
        assert isinstance(vs.destination, Destination)
        assert vs.destination.port.port == 443
        # Round-trip through the canonical text.
        assert str(vs.destination) == "/Common/10.0.0.1:443"
        assert Destination.parse(str(vs.destination)) == vs.destination

    def test_parser_populates_typed_destination(self):
        """End-to-end: ``ltm virtual`` parses into a typed
        :class:`Destination` directly.
        """
        from core.bigip.parser import parse_bigip_conf

        src = (
            "ltm virtual /Common/vs1 {\n"
            "    destination /Common/10.0.0.1:80\n"
            "}\n"
            "ltm virtual /Common/vs6 {\n"
            "    destination /Common/[2001:db8::1].443\n"
            "}\n"
        )
        config = parse_bigip_conf(src)
        vs = config.virtual_servers["/Common/vs1"]
        assert isinstance(vs.destination, Destination)
        assert str(vs.destination) == "/Common/10.0.0.1:80"
        vs6 = config.virtual_servers["/Common/vs6"]
        assert vs6.destination is not None
        assert vs6.destination.ipv6_brackets
        assert str(vs6.destination) == "/Common/[2001:db8::1].443"


class TestNodeTypedField:
    """``BigipNode.address`` is :class:`Address`; ``.fqdn`` is :class:`FQDN`."""

    def test_parser_populates_typed_node(self):
        from core.bigip.parser import parse_bigip_conf

        src = "ltm node /Common/n1 { address 10.0.0.1 }\n"
        config = parse_bigip_conf(src)
        node = config.nodes["/Common/n1"]
        assert isinstance(node.address, IPAddress)
        assert str(node.address) == "10.0.0.1"

    def test_parser_populates_node_fqdn(self):
        from core.bigip.parser import parse_bigip_conf

        src = "ltm node /Common/n2 { fqdn { name host.example.com } }\n"
        config = parse_bigip_conf(src)
        node = config.nodes["/Common/n2"]
        assert isinstance(node.fqdn, FQDN)
        assert str(node.fqdn) == "host.example.com"


class TestDslRendersTypedFieldsAsStrings:
    """The DSL surface keeps emitting strings via the typed=True FieldSpec."""

    def test_vs_destination(self):
        from core.bigip.query.runner import run_query

        src = "ltm virtual /Common/vs1 { destination /Common/10.0.0.1:80 }\n"
        result = run_query('.ltm.virtual["/Common/vs1"].destination', {"mem://1": src})
        assert result.values_per_file["mem://1"] == ["/Common/10.0.0.1:80"]

    def test_pool_member_address(self):
        from core.bigip.query.runner import run_query

        src = (
            "ltm pool /Common/p1 {\n"
            "    members {\n"
            "        /Common/10.0.0.1:80 { address 10.0.0.1 }\n"
            "    }\n"
            "}\n"
        )
        result = run_query('.ltm.pool["/Common/p1"].members[].address', {"mem://1": src})
        assert result.values_per_file["mem://1"] == ["10.0.0.1"]

    def test_node_address(self):
        from core.bigip.query.runner import run_query

        src = "ltm node /Common/n1 { address 10.0.0.1 }\n"
        result = run_query('.ltm.node["/Common/n1"].address', {"mem://1": src})
        assert result.values_per_file["mem://1"] == ["10.0.0.1"]


class TestNetSelfTypedField:
    def test_self_address_is_typed_network(self):
        """``BigipNetSelf.address`` is now a typed :class:`Network`
        (was a raw string).  The parser populates it directly; consumers
        no longer re-parse the CIDR.
        """
        from core.bigip.model import BigipNetSelf

        s = BigipNetSelf(
            name="self1",
            full_path="/Common/self1",
            address=Network.parse("10.0.0.1/24"),
        )
        assert isinstance(s.address, Network)
        assert s.address.is_ipv4
        assert s.address.prefix_length == 24
        # Round-trip: typed → str → parse yields the same typed value.
        assert Network.parse(str(s.address)) == s.address


class TestNetRouteTypedField:
    def test_network_and_gw_are_typed(self):
        """``BigipNetRoute.network`` / ``.gw`` are now typed
        :class:`Network` / :class:`IPAddress` (were raw strings).
        """
        from core.bigip.model import BigipNetRoute

        r = BigipNetRoute(
            name="r1",
            full_path="/Common/r1",
            network=Network.parse("10.0.0.0/8"),
            gw=IPAddress.parse("192.168.1.1"),
        )
        assert isinstance(r.network, Network)
        assert isinstance(r.gw, IPAddress)
        assert not r.is_default_route
        # Round-trip both fields.
        assert Network.parse(str(r.network)) == r.network
        assert IPAddress.parse(str(r.gw)) == r.gw

    def test_default_route_carries_flag_not_network(self):
        from core.bigip.model import BigipNetRoute

        r = BigipNetRoute(
            name="default",
            full_path="/Common/default",
            network=None,
            is_default_route=True,
        )
        assert r.network is None
        assert r.is_default_route

    def test_parser_populates_typed_fields_for_net_route(self):
        """End-to-end round-trip: a TMSH ``net route`` stanza parses
        directly into typed :class:`Network` / :class:`IPAddress`
        fields, no string detour."""
        from core.bigip.parser import parse_bigip_conf

        src = "net route /Common/r1 { network 10.0.0.0/8 gw 192.168.1.1 }\n"
        config = parse_bigip_conf(src)
        r = config.net_routes["/Common/r1"]
        assert isinstance(r.network, Network)
        assert str(r.network) == "10.0.0.0/8"
        assert isinstance(r.gw, IPAddress)
        assert str(r.gw) == "192.168.1.1"
        assert not r.is_default_route

    def test_parser_populates_default_route_flag(self):
        from core.bigip.parser import parse_bigip_conf

        src = "net route /Common/default_route { network default gw 192.168.1.1 }\n"
        config = parse_bigip_conf(src)
        r = config.net_routes["/Common/default_route"]
        # ``Network`` now understands the ``default`` keyword
        # directly and preserves the original spelling so DSL output
        # re-emits ``default`` instead of ``0.0.0.0/0``; the bool
        # ``is_default_route`` stays on the model for predicate use.
        assert isinstance(r.network, Network)
        assert r.network.is_default
        assert r.network.original == "default"
        assert str(r.network) == "default"
        assert r.is_default_route
        assert isinstance(r.gw, IPAddress)

    def test_parser_populates_typed_fields_for_net_self(self):
        from core.bigip.parser import parse_bigip_conf

        src = "net self /Common/self1 { address 10.0.0.1/24 vlan /Common/internal }\n"
        config = parse_bigip_conf(src)
        s = config.net_selves["/Common/self1"]
        assert isinstance(s.address, Network)
        # ``net self`` addresses carry an interface host plus prefix
        # (the host bits matter); ``str()`` preserves the original
        # spelling so DSL queries and round-trip emission show the
        # real interface IP, while ``.network`` still gives the
        # canonical CIDR for subnet membership / supernet checks.
        assert str(s.address) == "10.0.0.1/24"
        assert s.address.network.with_prefixlen == "10.0.0.0/24"
        assert s.vlan == "/Common/internal"  # still a string ref

    def test_dsl_renders_typed_fields_as_strings(self):
        """The query DSL still surfaces these fields as strings —
        the projection's ``typed=True`` FieldSpec converts the
        typed value back to its canonical text before exposing it.
        Existing queries that match against the string value keep
        working."""
        from core.bigip.query.runner import run_query

        src = "net route /Common/r1 { network 10.0.0.0/8 gw 192.168.1.1 }\n"
        result = run_query('.net.route["/Common/r1"].network', {"mem://1": src})
        assert result.values_per_file["mem://1"] == ["10.0.0.0/8"]
        result = run_query('.net.route["/Common/r1"].gw', {"mem://1": src})
        assert result.values_per_file["mem://1"] == ["192.168.1.1"]


class TestVirtualAddressTypedField:
    """``BigipVirtualAddress.address`` is now typed; ``.network_typed``
    derives the combined CIDR from ``address`` + ``mask``."""

    def test_host_address_plus_mask_becomes_network(self):
        from core.bigip.model import BigipVirtualAddress

        va = BigipVirtualAddress(
            name="va1",
            full_path="/Common/va1",
            address=IPAddress.parse("10.0.0.1"),
            mask="255.255.255.0",
        )
        assert isinstance(va.address, IPAddress)
        assert isinstance(va.network_typed, Network)
        assert va.network_typed.prefix_length == 24

    def test_parser_populates_typed_virtual_address(self):
        from core.bigip.parser import parse_bigip_conf

        src = "ltm virtual-address /Common/10.0.0.1 { address 10.0.0.1 mask 255.255.255.0 }\n"
        config = parse_bigip_conf(src)
        va = config.virtual_addresses["/Common/10.0.0.1"]
        assert isinstance(va.address, IPAddress)
        assert str(va.address) == "10.0.0.1"
        # ``network_typed`` combines address + mask into a CIDR.
        assert va.network_typed is not None
        assert va.network_typed.prefix_length == 24
