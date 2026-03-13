"""Tests for the IP address utility module and hover/code-action integration."""

from __future__ import annotations

import ipaddress
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types as lsp_types

from core.common.ip_utils import (
    IPClass,
    format_ip_hover,
    ipv4_to_ipv6_mapped,
    ipv6_mapped_to_ipv4,
    parse_ip,
    parse_ipv4,
    parse_ipv6,
    validate_ipv4_octets,
)
from lsp.features.hover import get_hover


class TestParseIPv4:
    def test_private_10(self):
        info = parse_ipv4("10.0.0.1")
        assert info is not None
        assert info.classification == IPClass.PRIVATE
        assert "1918" in info.rfc

    def test_private_172(self):
        info = parse_ipv4("172.16.0.1")
        assert info is not None
        assert info.classification == IPClass.PRIVATE

    def test_private_192_168(self):
        info = parse_ipv4("192.168.1.1")
        assert info is not None
        assert info.classification == IPClass.PRIVATE

    def test_loopback(self):
        info = parse_ipv4("127.0.0.1")
        assert info is not None
        assert info.classification == IPClass.LOOPBACK

    def test_multicast(self):
        info = parse_ipv4("224.0.0.1")
        assert info is not None
        assert info.classification == IPClass.MULTICAST

    def test_link_local(self):
        info = parse_ipv4("169.254.1.1")
        assert info is not None
        assert info.classification == IPClass.LINK_LOCAL

    def test_public(self):
        info = parse_ipv4("8.8.8.8")
        assert info is not None
        assert info.classification == IPClass.PUBLIC

    def test_broadcast(self):
        info = parse_ipv4("255.255.255.255")
        assert info is not None
        assert info.classification == IPClass.BROADCAST

    def test_documentation_test_net_1(self):
        info = parse_ipv4("192.0.2.1")
        assert info is not None
        assert info.classification == IPClass.DOCUMENTATION

    def test_documentation_test_net_2(self):
        info = parse_ipv4("198.51.100.1")
        assert info is not None
        assert info.classification == IPClass.DOCUMENTATION

    def test_documentation_test_net_3(self):
        info = parse_ipv4("203.0.113.1")
        assert info is not None
        assert info.classification == IPClass.DOCUMENTATION

    def test_carrier_grade_nat(self):
        info = parse_ipv4("100.64.0.1")
        assert info is not None
        assert info.classification == IPClass.CARRIER_GRADE_NAT

    def test_subnet_mask(self):
        info = parse_ipv4("255.255.255.0")
        assert info is not None
        assert info.is_subnet_mask is True
        assert info.cidr_prefix == 24

    def test_subnet_mask_16(self):
        info = parse_ipv4("255.255.0.0")
        assert info is not None
        assert info.is_subnet_mask is True
        assert info.cidr_prefix == 16

    def test_not_subnet_mask(self):
        info = parse_ipv4("192.168.1.1")
        assert info is not None
        assert info.is_subnet_mask is False

    def test_ipv6_mapped_generated(self):
        info = parse_ipv4("10.0.0.1")
        assert info is not None
        assert info.ipv6_mapped is not None
        assert "ffff" in str(info.ipv6_mapped).lower()

    def test_invalid_returns_none(self):
        assert parse_ipv4("999.999.999.999") is None
        assert parse_ipv4("not-an-ip") is None


class TestParseIPv6:
    def test_loopback(self):
        info = parse_ipv6("::1")
        assert info is not None
        assert info.classification == IPClass.LOOPBACK

    def test_link_local(self):
        info = parse_ipv6("fe80::1")
        assert info is not None
        assert info.classification == IPClass.LINK_LOCAL

    def test_multicast(self):
        info = parse_ipv6("ff02::1")
        assert info is not None
        assert info.classification == IPClass.MULTICAST

    def test_ula(self):
        info = parse_ipv6("fd00::1")
        assert info is not None
        assert info.classification == IPClass.PRIVATE
        assert "4193" in info.rfc

    def test_documentation(self):
        info = parse_ipv6("2001:db8::1")
        assert info is not None
        assert info.classification == IPClass.DOCUMENTATION

    def test_ipv4_mapped(self):
        info = parse_ipv6("::ffff:192.168.1.1")
        assert info is not None
        assert info.ipv4_mapped is not None
        assert str(info.ipv4_mapped) == "192.168.1.1"

    def test_public(self):
        info = parse_ipv6("2001:4860:4860::8888")
        assert info is not None
        assert info.classification == IPClass.PUBLIC


class TestParseIP:
    def test_ipv4(self):
        info = parse_ip("10.0.0.1")
        assert info is not None
        assert info.version == 4

    def test_ipv6(self):
        info = parse_ip("::1")
        assert info is not None
        assert info.version == 6

    def test_cidr_prefix(self):
        info = parse_ip("10.0.0.0/8")
        assert info is not None
        assert info.cidr_prefix == 8
        assert info.network is not None

    def test_invalid(self):
        assert parse_ip("not-an-ip") is None


class TestIPConversion:
    def test_ipv4_to_ipv6_mapped(self):
        result = ipv4_to_ipv6_mapped(ipaddress.IPv4Address("192.168.1.1"))
        assert "ffff" in result.lower()
        assert "192.168.1.1" in result or "c0a8:101" in result.lower()

    def test_ipv6_mapped_to_ipv4(self):
        result = ipv6_mapped_to_ipv4(ipaddress.IPv6Address("::ffff:192.168.1.1"))
        assert result == "192.168.1.1"

    def test_ipv6_not_mapped_returns_none(self):
        result = ipv6_mapped_to_ipv4(ipaddress.IPv6Address("2001:db8::1"))
        assert result is None


class TestValidateIPv4Octets:
    def test_valid(self):
        assert validate_ipv4_octets("192.168.1.1") is None

    def test_octet_over_255(self):
        result = validate_ipv4_octets("192.168.1.256")
        assert result is not None
        assert "exceeds 255" in result

    def test_leading_zero(self):
        result = validate_ipv4_octets("192.168.01.1")
        assert result is not None
        assert "leading zero" in result

    def test_leading_zero_non_octal_digit(self):
        """09 contains '9' which is not valid octal — no warning."""
        assert validate_ipv4_octets("192.168.09.1") is None

    def test_leading_zero_08_not_octal(self):
        assert validate_ipv4_octets("10.0.08.1") is None

    def test_leading_zero_077_octal(self):
        result = validate_ipv4_octets("192.168.077.1")
        assert result is not None
        assert "leading zero" in result


class TestFormatIPHover:
    def test_ipv4_hover_contains_class(self):
        info = parse_ipv4("10.0.0.1")
        assert info is not None
        text = format_ip_hover(info)
        assert "Private" in text
        assert "RFC 1918" in text

    def test_ipv4_hover_contains_binary(self):
        info = parse_ipv4("255.255.255.0")
        assert info is not None
        text = format_ip_hover(info)
        assert "Binary" in text
        assert "11111111" in text

    def test_ipv6_hover_contains_class(self):
        info = parse_ipv6("::1")
        assert info is not None
        text = format_ip_hover(info)
        assert "Loopback" in text

    def test_mask_hover_shows_prefix(self):
        info = parse_ipv4("255.255.255.0")
        assert info is not None
        text = format_ip_hover(info)
        assert "/24" in text

    def test_ipv4_mapped_hover_shows_equivalent(self):
        info = parse_ipv6("::ffff:10.0.0.1")
        assert info is not None
        text = format_ip_hover(info)
        assert "10.0.0.1" in text


def _hover_text(result: lsp_types.Hover) -> str:
    contents = result.contents
    if isinstance(contents, lsp_types.MarkupContent):
        return contents.value
    if isinstance(contents, list):
        return "\n".join(
            item.value if isinstance(item, lsp_types.MarkedStringWithLanguage) else str(item)
            for item in contents
        )
    if isinstance(contents, lsp_types.MarkedStringWithLanguage):
        return contents.value
    return str(contents)


class TestIPHover:
    def test_ipv4_hover(self):
        source = "set addr 10.0.0.1"
        hover = get_hover(source, 0, 10)
        assert hover is not None
        assert "Private" in _hover_text(hover)

    def test_ipv6_loopback_hover(self):
        source = "set addr ::1"
        hover = get_hover(source, 0, 10)
        assert hover is not None
        assert "Loopback" in _hover_text(hover)

    def test_subnet_mask_hover(self):
        source = "set mask 255.255.255.0"
        hover = get_hover(source, 0, 10)
        assert hover is not None
        assert "/24" in _hover_text(hover)

    def test_non_ip_no_hover(self):
        """A regular word should not trigger IP hover."""
        source = "set name hello"
        hover = get_hover(source, 0, 10)
        # Should not return an IP hover (might return command hover or None)
        if hover is not None:
            assert "IPv4" not in _hover_text(hover)
