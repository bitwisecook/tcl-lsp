"""Tests for the BIG-IP port-name → port-number resolver."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.convert.as3 import _split_destination as _as3_split_destination
from core.bigip.pcap_enrich import _split_destination as _pcap_split_destination
from core.bigip.port_names import (
    NAME_TO_PORT,
    PORT_TO_NAME,
    port_name_to_number,
    port_number_to_name,
    resolve_port,
)
from core.bigip.wireshark_profile import _vs_port_proto


def test_table_loads_with_expected_size():
    # The vendored mcpd table has ~4.5k rows; assert a lower bound so a
    # silently-truncated table trips the test rather than the consumers.
    assert len(NAME_TO_PORT) > 4000


def test_reverse_table_is_complete():
    # The CSV has no duplicate ports, so the reverse dict must be a
    # full inverse of the forward dict.
    assert len(PORT_TO_NAME) == len(NAME_TO_PORT)


def test_well_known_names_resolve():
    assert port_name_to_number("http") == 80
    assert port_name_to_number("https") == 443
    assert port_name_to_number("ssh") == 22
    assert port_name_to_number("domain") == 53
    assert port_name_to_number("smtp") == 25


def test_bigip_specific_names_resolve():
    # ``any`` is BIG-IP's wildcard, not IANA.
    assert port_name_to_number("any") == 0
    assert port_name_to_number("f5-iquery") == 4353
    assert port_name_to_number("f5-globalsite") == 2792


def test_lookup_is_case_insensitive():
    assert port_name_to_number("HTTPS") == 443
    assert port_name_to_number("Https") == 443


def test_unknown_name_returns_none():
    assert port_name_to_number("definitely-not-a-service") is None
    assert port_name_to_number("") is None


def test_reverse_lookup_returns_canonical_name():
    assert port_number_to_name(80) == "http"
    assert port_number_to_name(443) == "https"
    assert port_number_to_name(4353) == "f5-iquery"


def test_resolve_port_accepts_numeric_string():
    assert resolve_port("443") == 443
    assert resolve_port("0") == 0
    assert resolve_port("65535") == 65535


def test_resolve_port_rejects_out_of_range_number():
    assert resolve_port("65536") is None
    assert resolve_port("99999") is None


def test_resolve_port_accepts_name():
    assert resolve_port("https") == 443


def test_resolve_port_rejects_garbage():
    assert resolve_port("") is None
    assert resolve_port("not-a-port") is None


def test_as3_split_destination_accepts_named_port():
    assert _as3_split_destination("/Common/10.0.0.5:https") == ("10.0.0.5", 443)
    assert _as3_split_destination("/Common/10.0.0.5:80") == ("10.0.0.5", 80)


def test_as3_split_destination_handles_any():
    # ``any`` resolves to port 0 — the AS3 emitter currently treats
    # ``None`` as "no virtualPort"; 0 keeps the wildcard explicit.
    assert _as3_split_destination("/Common/10.0.0.5:any") == ("10.0.0.5", 0)


def test_as3_split_destination_handles_ipv6_numeric_port():
    assert _as3_split_destination("/Common/2001:db8::1.443") == ("2001:db8::1", 443)


def test_as3_split_destination_handles_ipv6_named_port():
    assert _as3_split_destination("/Common/2001:db8::1.https") == ("2001:db8::1", 443)


def test_as3_split_destination_preserves_wildcard_host():
    # Hostname / wildcard destinations don't have to parse as an IP —
    # the ``:`` fallback splits them so the port still gets extracted.
    assert _as3_split_destination("/Common/*:443") == ("*", 443)


def test_pcap_split_destination_accepts_named_port_ipv4():
    assert _pcap_split_destination("/Common/10.0.0.5:https") == "10.0.0.5"


def test_pcap_split_destination_accepts_named_port_ipv6():
    assert _pcap_split_destination("/Common/2001:db8::1.https") == "2001:db8::1"


def test_vs_port_proto_accepts_named_port():
    assert _vs_port_proto("/Common/10.0.0.5:https") == (443, "tcp")
    assert _vs_port_proto("/Common/10.0.0.5:80") == (80, "tcp")


def test_vs_port_proto_skips_wildcard_any():
    # ``any`` -> 0, which is BIG-IP's any-port wildcard. Wireshark
    # services entries need a real port, so port 0 must be filtered.
    assert _vs_port_proto("/Common/10.0.0.5:any") is None
