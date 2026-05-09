"""Tests for ``f5 enrich-pcapng`` and the underlying enrichment helpers."""

from __future__ import annotations

import io
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

from core.bigip.parser import parse_bigip_conf
from core.bigip.pcap_enrich import (
    NameIndex,
    _extract_gtm_pools,
    _extract_gtm_servers,
    _extract_gtm_wideips,
    _extract_self_ips,
    _slug,
    _split_destination,
    build_merged_name_index,
    build_name_index,
    collect_observed_ips,
    enrich_capture_file,
    enrich_pcapng,
)
from core.bigip.pcapng import (
    BLOCK_TYPE_DSB,
    BLOCK_TYPE_EPB,
    BLOCK_TYPE_IDB,
    BLOCK_TYPE_NRB,
    BLOCK_TYPE_SHB,
    DSB_SECRETS_TYPE_TLS,
    NRB_RECORD_END,
    NRB_RECORD_IPV4,
    NRB_RECORD_IPV6,
    build_dsb_block,
    build_nrb_block,
    read_blocks,
)
from explorer.f5_cli import main


def _run(args, capsys):
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


def _build_minimal_pcapng() -> bytes:
    """Minimal PCAPNG with one SHB, one IDB, and one EPB carrying a tiny frame.

    Little-endian throughout.  The EPB carries an IPv4 / TCP frame so we
    can confirm ``enrich_pcapng`` round-trips packet bytes byte-for-byte.
    """
    out = io.BytesIO()

    # Section Header Block.
    shb_body = struct.pack("<IHHQ", 0x1A2B3C4D, 1, 0, 0xFFFFFFFFFFFFFFFF)
    shb_total = 12 + len(shb_body)
    out.write(struct.pack("<II", BLOCK_TYPE_SHB, shb_total))
    out.write(shb_body)
    out.write(struct.pack("<I", shb_total))

    # Interface Description Block (Ethernet, snaplen 65535).
    idb_body = struct.pack("<HHI", 1, 0, 65535)
    idb_total = 12 + len(idb_body)
    out.write(struct.pack("<II", BLOCK_TYPE_IDB, idb_total))
    out.write(idb_body)
    out.write(struct.pack("<I", idb_total))

    # Enhanced Packet Block: empty Ethernet + IPv4 stub frame.
    eth = bytes.fromhex("aabbccddeeff112233445566") + b"\x08\x00"
    ipv4 = bytes(
        [0x45, 0x00, 0x00, 0x14]  # ver/IHL, dscp, total_length=20
        + [0, 1, 0x40, 0x00, 0x40, 0x06, 0, 0]  # id, flags, ttl, proto, cksum
        + [10, 0, 0, 1, 10, 0, 0, 2]  # src 10.0.0.1, dst 10.0.0.2
    )
    packet = eth + ipv4
    cap_len = len(packet)
    epb_body = struct.pack("<IIIII", 0, 0, 0, cap_len, cap_len) + packet
    pad = (-cap_len) & 3
    epb_body += b"\x00" * pad
    epb_total = 12 + len(epb_body)
    out.write(struct.pack("<II", BLOCK_TYPE_EPB, epb_total))
    out.write(epb_body)
    out.write(struct.pack("<I", epb_total))

    return out.getvalue()


SAMPLE_CONFIG = """
ltm node /Common/web1 {
    address 10.0.1.10
}

ltm rule /Common/my_irule {
    when HTTP_REQUEST { return }
}

ltm pool /Common/web_pool {
    members {
        /Common/web1:80 {
            address 10.0.1.10
        }
    }
}

ltm snatpool /Common/my_snatpool {
    members {
        10.0.2.100
        /Common/192.168.1.1
    }
}

ltm virtual /Common/vs1 {
    destination /Common/10.0.0.1:443
    pool /Common/web_pool
    rules {
        /Common/my_irule
    }
}

net self /Common/external_self {
    address 10.0.5.1%0/24
    vlan /Common/external
}
"""


def _parse_blocks(buf: bytes) -> list:
    return list(read_blocks(io.BytesIO(buf)))


# Helper unit tests


def test_slug_lowercases_and_replaces_specials():
    assert _slug("Common") == "common"
    assert _slug("My_Pool") == "my-pool"
    assert _slug("/Common/foo") == "common-foo"
    assert _slug("a..b__c") == "a-b-c"


def test_split_destination_handles_v4_and_v6_and_partition():
    assert _split_destination("/Common/10.0.0.1:443") == "10.0.0.1"
    assert _split_destination("10.0.0.1:80") == "10.0.0.1"
    assert _split_destination("/Common/2001:db8::1.443") == "2001:db8::1"
    assert _split_destination("/Common/garbage") == ""


def test_build_name_index_covers_every_object_type():
    config = parse_bigip_conf(SAMPLE_CONFIG)
    index = build_name_index(config, source=SAMPLE_CONFIG)

    # Virtual server destination + attached iRule.
    assert "vs-common-vs1" in index.v4["10.0.0.1"]
    assert "irule-common-my-irule" in index.v4["10.0.0.1"]

    # Pool member uses the member's own path (purpose=pool).
    assert "pool-common-web1-80" in index.v4["10.0.1.10"]
    # Node label still tags the same IP independently.
    assert "node-common-web1" in index.v4["10.0.1.10"]

    # SNAT-pool members get the snatpool's path (one label, applied to every member).
    assert index.v4["10.0.2.100"] == ["snat-common-my-snatpool"]
    assert index.v4["192.168.1.1"] == ["snat-common-my-snatpool"]

    # net self exact-IP and subnet labels.
    assert "self-common-external-self" in index.v4["10.0.5.1"]
    assert any(label == "net-common-external-self" for _, label in index.v4_subnets)


def test_extract_self_ips_strips_route_domain_and_parses_cidr():
    selves = _extract_self_ips(SAMPLE_CONFIG)
    paths = {s.full_path: s for s in selves}
    assert "/Common/external_self" in paths
    self_ip = paths["/Common/external_self"]
    assert self_ip.address == "10.0.5.1"
    assert self_ip.network is not None
    assert str(self_ip.network) == "10.0.5.0/24"


# Wide-IP / GTM extraction tests


GTM_LTM_SOURCES = (
    # GTM-only file: wide-IPs and pools live here.
    """
gtm pool a /Common/wip_pool {
    members {
        /Common/dc1:vs_a { ratio 1 }
        /Common/dc1:vs_b { ratio 1 }
    }
}

gtm wideip a /Common/example.com {
    pools {
        /Common/wip_pool { ratio 1 }
    }
}
""",
    # LTM-only file: server addresses live here.
    """
gtm server /Common/dc1 {
    addresses {
        10.0.0.5 { device-name dc1 }
    }
    virtual-servers {
        vs_a {
            destination 10.0.0.5:443
        }
        vs_b {
            destination 10.0.0.6:80
        }
    }
}

ltm virtual /Common/dc1_internal {
    destination /Common/10.0.0.5:443
}
""",
)


def test_extract_gtm_servers_pulls_addresses_and_vs_destinations():
    src = GTM_LTM_SOURCES[1]
    servers = _extract_gtm_servers(src)
    assert "/Common/dc1" in servers
    server = servers["/Common/dc1"]
    assert "10.0.0.5" in server.addresses
    assert server.virtual_servers == {"vs_a": "10.0.0.5", "vs_b": "10.0.0.6"}


def test_extract_gtm_pools_returns_server_vs_pairs():
    pools = _extract_gtm_pools(GTM_LTM_SOURCES[0])
    assert pools["/Common/wip_pool"] == [
        ("/Common/dc1", "vs_a"),
        ("/Common/dc1", "vs_b"),
    ]


def test_extract_gtm_wideips_returns_pool_refs():
    wideips = _extract_gtm_wideips(GTM_LTM_SOURCES[0])
    assert wideips["/Common/example.com"] == ["/Common/wip_pool"]


def test_wideip_resolution_traverses_pool_to_server_vs():
    src = "\n".join(GTM_LTM_SOURCES)
    cfg = parse_bigip_conf(src)
    index = build_name_index(cfg, source=src)
    assert "wideip-common-example-com" in index.v4["10.0.0.5"]
    assert "wideip-common-example-com" in index.v4["10.0.0.6"]


def test_build_merged_name_index_resolves_cross_file_wideip():
    """Wide-IP in file A must resolve through gtm server in file B."""
    pairs = [(parse_bigip_conf(s), s) for s in GTM_LTM_SOURCES]
    merged = build_merged_name_index(pairs)
    assert "wideip-common-example-com" in merged.v4["10.0.0.5"]
    assert "wideip-common-example-com" in merged.v4["10.0.0.6"]
    # And the LTM virtual is still present.
    assert "vs-common-dc1-internal" in merged.v4["10.0.0.5"]


def test_name_index_lookup_includes_subnet_labels():
    index = NameIndex()
    import ipaddress as _ip

    index.add_subnet(_ip.IPv4Network("10.0.5.0/24"), "net-common-external-self")
    assert index.lookup("10.0.5.42") == ["net-common-external-self"]
    assert index.lookup("10.0.6.1") == []


# NRB / DSB block builder unit tests


def test_build_nrb_block_emits_records_and_end_sentinel():
    v4 = {b"\x0a\x00\x00\x01": ["vs-common-vs1"]}
    v6 = {b"\x20\x01\x0d\xb8" + b"\x00" * 11 + b"\x01": ["v6-host"]}
    block = build_nrb_block("<", v4, v6)

    assert block.block_type == BLOCK_TYPE_NRB
    assert block.endian == "<"

    body = block.body
    pos = 0
    seen_records: list[tuple[int, bytes]] = []
    while pos < len(body):
        rec_type, rec_len = struct.unpack("<HH", body[pos : pos + 4])
        value = bytes(body[pos + 4 : pos + 4 + rec_len])
        seen_records.append((rec_type, value))
        # advance past padded value and 4-byte record header.
        padded = (rec_len + 3) & ~3
        pos += 4 + padded
        if rec_type == NRB_RECORD_END:
            break

    types_seen = [t for t, _ in seen_records]
    assert NRB_RECORD_IPV4 in types_seen
    assert NRB_RECORD_IPV6 in types_seen
    assert types_seen[-1] == NRB_RECORD_END
    # name strings end up NUL-terminated inside the value.
    v4_record = next(value for t, value in seen_records if t == NRB_RECORD_IPV4)
    assert v4_record.startswith(b"\x0a\x00\x00\x01")
    assert b"vs-common-vs1\x00" in v4_record


def test_build_dsb_block_pads_and_carries_secrets_data():
    keylog = b"CLIENT_RANDOM 0123 abcd\n"
    block = build_dsb_block("<", DSB_SECRETS_TYPE_TLS, keylog)

    assert block.block_type == BLOCK_TYPE_DSB
    secrets_type, secrets_len = struct.unpack("<II", block.body[:8])
    assert secrets_type == DSB_SECRETS_TYPE_TLS
    assert secrets_len == len(keylog)
    assert block.body[8 : 8 + secrets_len] == keylog
    # Body must be 4-byte aligned even though keylog len is 24 (already aligned).
    assert len(block.body) % 4 == 0


# enrich_pcapng integration tests


def test_enrich_pcapng_inserts_nrb_filtered_to_observed_ips():
    """By default only addresses that appear in a packet get NRB records."""
    pcapng_in = _build_minimal_pcapng()  # carries 10.0.0.1 / 10.0.0.2 only.
    config = parse_bigip_conf(SAMPLE_CONFIG)
    name_index = build_name_index(config, source=SAMPLE_CONFIG)

    out = io.BytesIO()
    result = enrich_pcapng(io.BytesIO(pcapng_in), out, name_index)

    # The capture only carries 10.0.0.1 (a known VS) and 10.0.0.2 (unknown).
    # Only 10.0.0.1 should land in the NRB.
    assert result.ipv4_records == 1
    assert result.observed_ipv4 == 2
    assert result.keylog_bytes == 0
    assert result.converted_from_libpcap is False

    blocks = _parse_blocks(out.getvalue())
    types = [b.block_type for b in blocks]
    # SHB, IDB, NRB, EPB — NRB lands between IDB and EPB.
    assert types[0] == BLOCK_TYPE_SHB
    assert types[1] == BLOCK_TYPE_IDB
    assert types[2] == BLOCK_TYPE_NRB
    assert types[3] == BLOCK_TYPE_EPB
    # EPB packet bytes round-trip exactly.
    original_blocks = _parse_blocks(pcapng_in)
    in_epb = next(b for b in original_blocks if b.block_type == BLOCK_TYPE_EPB)
    out_epb = next(b for b in blocks if b.block_type == BLOCK_TYPE_EPB)
    assert in_epb.packet_data == out_epb.packet_data
    # And the NRB body actually mentions vs-common-vs1.
    nrb = blocks[2]
    assert b"vs-common-vs1\x00" in nrb.body


def test_enrich_pcapng_include_unobserved_emits_full_inventory():
    pcapng_in = _build_minimal_pcapng()
    config = parse_bigip_conf(SAMPLE_CONFIG)
    name_index = build_name_index(config, source=SAMPLE_CONFIG)

    out = io.BytesIO()
    result = enrich_pcapng(
        io.BytesIO(pcapng_in), out, name_index, include_unobserved=True
    )

    # With --all every exact-IP entry from the inventory is emitted, even
    # ones that never appeared as a packet src/dst.
    assert result.ipv4_records == len(name_index.v4)
    assert result.observed_ipv4 == 0  # walk skipped under include_unobserved


def test_collect_observed_ips_sees_packet_src_and_dst():
    v4, v6 = collect_observed_ips(io.BytesIO(_build_minimal_pcapng()))
    assert v4 == {"10.0.0.1", "10.0.0.2"}
    assert v6 == set()


def test_enrich_pcapng_subnet_label_applied_to_observed_host():
    """A host inside a self-IP CIDR with no exact entry still gets a label."""

    def _build_pcapng_with_addresses(addrs: list[bytes]) -> bytes:
        out = io.BytesIO()
        shb_body = struct.pack("<IHHQ", 0x1A2B3C4D, 1, 0, 0xFFFFFFFFFFFFFFFF)
        out.write(struct.pack("<II", BLOCK_TYPE_SHB, 12 + len(shb_body)))
        out.write(shb_body)
        out.write(struct.pack("<I", 12 + len(shb_body)))
        idb_body = struct.pack("<HHI", 1, 0, 65535)
        out.write(struct.pack("<II", BLOCK_TYPE_IDB, 12 + len(idb_body)))
        out.write(idb_body)
        out.write(struct.pack("<I", 12 + len(idb_body)))
        for src in addrs:
            eth = bytes.fromhex("aabbccddeeff112233445566") + b"\x08\x00"
            ipv4 = bytes(
                [0x45, 0x00, 0x00, 0x14]
                + [0, 1, 0x40, 0x00, 0x40, 0x06, 0, 0]
            ) + src + bytes([10, 0, 9, 9])
            packet = eth + ipv4
            cap_len = len(packet)
            epb_body = struct.pack("<IIIII", 0, 0, 0, cap_len, cap_len) + packet
            pad = (-cap_len) & 3
            epb_body += b"\x00" * pad
            out.write(struct.pack("<II", BLOCK_TYPE_EPB, 12 + len(epb_body)))
            out.write(epb_body)
            out.write(struct.pack("<I", 12 + len(epb_body)))
        return out.getvalue()

    # 10.0.5.42 isn't in the LTM inventory but it's inside the
    # /Common/external_self subnet (10.0.5.0/24).  Expect a "net-…" label.
    pcapng_in = _build_pcapng_with_addresses([bytes([10, 0, 5, 42])])
    config = parse_bigip_conf(SAMPLE_CONFIG)
    name_index = build_name_index(config, source=SAMPLE_CONFIG)

    out = io.BytesIO()
    result = enrich_pcapng(io.BytesIO(pcapng_in), out, name_index)

    assert result.ipv4_records >= 1
    blocks = _parse_blocks(out.getvalue())
    nrb = next(b for b in blocks if b.block_type == BLOCK_TYPE_NRB)
    assert b"net-common-external-self\x00" in nrb.body


def test_enrich_pcapng_inserts_dsb_when_keylog_provided():
    pcapng_in = _build_minimal_pcapng()
    keylog = "CLIENT_RANDOM " + "0" * 64 + " " + "1" * 96 + "\n"
    name_index = NameIndex()  # empty index — only the DSB should be inserted.

    out = io.BytesIO()
    result = enrich_pcapng(io.BytesIO(pcapng_in), out, name_index, keylog_text=keylog)

    assert result.keylog_bytes == len(keylog.encode("utf-8"))

    blocks = _parse_blocks(out.getvalue())
    types = [b.block_type for b in blocks]
    assert BLOCK_TYPE_DSB in types
    # Exactly one DSB inserted, no NRB because the index was empty.
    assert types.count(BLOCK_TYPE_DSB) == 1
    assert BLOCK_TYPE_NRB not in types
    # DSB sits between IDB and EPB.
    dsb_idx = types.index(BLOCK_TYPE_DSB)
    idb_idx = types.index(BLOCK_TYPE_IDB)
    epb_idx = types.index(BLOCK_TYPE_EPB)
    assert idb_idx < dsb_idx < epb_idx


def test_enrich_pcapng_rejects_libpcap_input():
    libpcap = struct.pack("<IHHIIII", 0xA1B2C3D4, 2, 4, 0, 0, 65535, 1)
    with pytest.raises(ValueError) as exc:
        enrich_pcapng(io.BytesIO(libpcap), io.BytesIO(), NameIndex())
    assert "libpcap" in str(exc.value).lower() or "pcapng" in str(exc.value).lower()


# CLI smoke tests


def test_enrich_pcapng_cli_dry_run(tmp_path, capsys):
    cfg = tmp_path / "bigip.conf"
    cfg.write_text(SAMPLE_CONFIG)

    code, out, _err = _run(
        ["enrich-pcapng", "--dry-run", "-c", str(cfg), "/dev/null", "/dev/null"], capsys
    )
    assert code == 0
    assert "10.0.0.1\tvs-common-vs1" in out
    assert "10.0.0.1\tirule-common-my-irule" in out
    assert "10.0.5.1\tself-common-external-self" in out


def test_enrich_pcapng_cli_writes_pcapng_with_nrb(tmp_path, capsys):
    cfg = tmp_path / "bigip.conf"
    cfg.write_text(SAMPLE_CONFIG)
    pcap_in = tmp_path / "in.pcapng"
    pcap_in.write_bytes(_build_minimal_pcapng())
    pcap_out = tmp_path / "out.pcapng"

    code, _out, err = _run(
        ["enrich-pcapng", "-c", str(cfg), str(pcap_in), str(pcap_out)], capsys
    )

    assert code == 0
    assert pcap_out.exists()
    blocks = _parse_blocks(pcap_out.read_bytes())
    assert any(b.block_type == BLOCK_TYPE_NRB for b in blocks)
    assert "label" in err


def test_enrich_capture_file_overwrites_input_in_place_safely(tmp_path):
    """``f5 enrich-pcapng in.pcapng in.pcapng`` (same path) must not destroy the input.

    Earlier the output ``open(path, "wb")`` ran before ``enrich_pcapng()``
    started reading, so the input was truncated to zero before we got to
    parse it.  ``enrich_capture_file`` now stages writes in a temp file
    next to the destination and renames atomically once the enrich
    succeeds — so even when ``in_path == out_path`` the original file
    survives any failure and ends up enriched on success.
    """
    config = parse_bigip_conf(SAMPLE_CONFIG)
    name_index = build_name_index(config, source=SAMPLE_CONFIG)

    pcap = tmp_path / "shared.pcapng"
    original = _build_minimal_pcapng()
    pcap.write_bytes(original)

    result = enrich_capture_file(pcap, pcap, name_index)

    # File still exists (didn't get nuked by an early-truncated wb open).
    assert pcap.exists()
    blocks = _parse_blocks(pcap.read_bytes())
    types = {b.block_type for b in blocks}
    assert BLOCK_TYPE_NRB in types  # enrichment did happen.
    assert result.ipv4_records >= 1
    # No leftover ``.partial`` staging files in the directory.
    leftover = [p.name for p in tmp_path.iterdir() if p.name.endswith(".partial")]
    assert leftover == [], leftover


def test_enrich_pcapng_records_are_byte_stable_across_runs():
    """Two enrichments of the same input must produce byte-identical NRB blocks.

    Without sorting the observed-IP sets, Python's hash randomisation
    would shuffle NRB record order between processes; this test asserts
    determinism by running the pipeline twice in a row.
    """
    config = parse_bigip_conf(SAMPLE_CONFIG)
    name_index = build_name_index(config, source=SAMPLE_CONFIG)

    pcapng_in = _build_minimal_pcapng()
    out_a = io.BytesIO()
    out_b = io.BytesIO()
    enrich_pcapng(io.BytesIO(pcapng_in), out_a, name_index)
    enrich_pcapng(io.BytesIO(pcapng_in), out_b, name_index)
    assert out_a.getvalue() == out_b.getvalue()


def test_build_merged_name_index_does_not_duplicate_subnet_rows():
    """Merging the same ``(config, source)`` twice must not double subnet rows.

    ``build_merged_name_index`` passes the combined source to every
    per-config call; without dedup in ``add_subnet`` each ``net self``
    block would land in ``v4_subnets`` once per input.
    """
    config = parse_bigip_conf(SAMPLE_CONFIG)
    merged = build_merged_name_index(
        [(config, SAMPLE_CONFIG), (config, SAMPLE_CONFIG)]
    )
    assert merged.v4_subnets == list(dict.fromkeys(merged.v4_subnets))
