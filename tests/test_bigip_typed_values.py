"""Unit tests for the typed BIG-IP scalar values in ``core.bigip.types``."""

from __future__ import annotations

import pytest

from core.bigip.types import (
    FQDN,
    Destination,
    IPAddress,
    Network,
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

    def test_parse_loose_normalises_host_bits(self):
        # ``10.0.0.5/24`` has host bits set; loose parse normalises.
        n = Network.parse("10.0.0.5/24")
        assert str(n) == "10.0.0.0/24"

    def test_parse_strict_rejects_host_bits(self):
        with pytest.raises(ValueError):
            Network.parse("10.0.0.5/24", strict=True)

    def test_membership(self):
        n = Network.parse("10.0.0.0/8")
        assert IPAddress.parse("10.0.0.1") in n
        assert IPAddress.parse("11.0.0.1") not in n

    def test_parse_dotted_quad_netmask(self):
        # F5 ``net route`` / ``net self`` configs sometimes spell the
        # mask as a dotted quad rather than a prefix length.  Both
        # spellings must round-trip to the canonical prefix form.
        n = Network.parse("10.0.0.0/255.255.255.0")
        assert n.is_ipv4
        assert n.prefix_length == 24
        assert str(n) == "10.0.0.0/24"

    def test_parse_space_separated_addr_mask(self):
        # ``net route`` style: ``ADDR MASK`` with a space separator.
        n = Network.parse("10.0.0.0 255.255.0.0")
        assert n.is_ipv4
        assert n.prefix_length == 16

    def test_parse_dotted_quad_netmask_normalises_host_bits(self):
        n = Network.parse("10.0.0.5/255.255.255.0")
        assert str(n) == "10.0.0.0/24"


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
        assert d.partition is not None and d.partition.short_name == "Common"
        assert str(d) == "/Common/10.0.0.1:80"

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
