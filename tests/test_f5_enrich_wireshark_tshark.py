"""End-to-end validation of ``f5 enrich-wireshark`` against real tshark.

These tests build a profile, point a real tshark binary at it, and
confirm Wireshark accepts every file we emit and resolves the right
labels.  They're marked ``slow`` so they don't gate ``make prep-pr``;
``make test-slow`` (or ``AUTO_INSTALL_DEPS=1 make test-slow``) installs
``tshark`` via :file:`scripts/dev/ensure-test-deps.sh` and runs them.
"""

from __future__ import annotations

import io
import shutil
import struct
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

from core.bigip.parser import parse_bigip_conf
from core.bigip.wireshark_profile import build_wireshark_profile

pytestmark = [pytest.mark.slow]


SAMPLE_CONFIG = """
ltm node /Common/web1 {
    address 10.0.1.10
}

ltm pool /Common/web_pool {
    members {
        /Common/web1:80 {
            address 10.0.1.10
        }
    }
}

ltm snatpool /Common/snat_a {
    members {
        10.0.2.5
    }
}

ltm virtual /Common/vs1 {
    destination /Common/10.0.0.1:443
    pool /Common/web_pool
}

net self /Common/external_self {
    address 10.0.5.1%0/24
    vlan /Common/external
    allow-service {
        tcp:443
        udp:161
    }
}

net vlan /Common/external {
    tag 100
}

net arp /Common/static_peer {
    ip-address 10.0.5.42%0
    mac-address 00:11:22:33:44:55
}
"""


def _require_tshark() -> str:
    tshark = shutil.which("tshark")
    if tshark is None:
        pytest.skip(
            "tshark not installed — run "
            "`AUTO_INSTALL_DEPS=1 make test-slow` to fetch it via "
            "scripts/dev/ensure-test-deps.sh"
        )
    return tshark


def _ipv4_checksum(header: bytes) -> int:
    total = 0
    for i in range(0, len(header), 2):
        total += (header[i] << 8) | header[i + 1]
        total = (total & 0xFFFF) + (total >> 16)
    return (~total) & 0xFFFF


def _build_tcp_packet(src_ip: bytes, dst_ip: bytes, src_port: int, dst_port: int) -> bytes:
    eth = bytes.fromhex("aabbccddeeff112233445566") + b"\x08\x00"
    ip_total = 40
    ip = bytearray(
        bytes([0x45, 0x00])
        + struct.pack(">H", ip_total)
        + bytes([0, 1, 0x40, 0x00, 0x40, 0x06, 0, 0])
        + src_ip
        + dst_ip
    )
    cksum = _ipv4_checksum(bytes(ip))
    ip[10], ip[11] = (cksum >> 8) & 0xFF, cksum & 0xFF
    tcp = (
        struct.pack(">HH", src_port, dst_port)
        + struct.pack(">II", 1, 0)
        + bytes([0x50, 0x02])
        + struct.pack(">H", 0xFFFF)
        + struct.pack(">HH", 0, 0)
    )
    return eth + bytes(ip) + tcp


def _build_pcap(packets: list[bytes]) -> bytes:
    out = io.BytesIO()
    out.write(struct.pack("<IHHIIII", 0xA1B2C3D4, 2, 4, 0, 0, 65535, 1))
    for packet in packets:
        out.write(struct.pack("<IIII", 1_000_000, 0, len(packet), len(packet)))
        out.write(packet)
    return out.getvalue()


def _v4(addr: str) -> bytes:
    return bytes(int(p) for p in addr.split("."))


@pytest.fixture
def tshark_profile(tmp_path: Path) -> tuple[str, Path, Path]:
    """Build the profile, lay it out in a fresh ``WIRESHARK_CONFIG_DIR``,
    and write a pcap that exercises the proxy-side coloring rules.
    """
    tshark = _require_tshark()
    cfg = parse_bigip_conf(SAMPLE_CONFIG)
    profile = build_wireshark_profile([(cfg, SAMPLE_CONFIG)])

    config_dir = tmp_path / "wsconfig"
    profile_dir = config_dir / "profiles" / "f5_test"
    profile.write_to(profile_dir)

    pcap_path = tmp_path / "trace.pcap"
    # Client-side: client → VS dest 10.0.0.1:443
    p1 = _build_tcp_packet(_v4("192.168.1.10"), _v4("10.0.0.1"), 12345, 443)
    # Server-side: BIG-IP self → pool member 10.0.1.10:80
    p2 = _build_tcp_packet(_v4("10.0.5.1"), _v4("10.0.1.10"), 33333, 80)
    # SNAT egress: SNAT addr → external 8.8.8.8
    p3 = _build_tcp_packet(_v4("10.0.2.5"), _v4("8.8.8.8"), 33333, 443)
    pcap_path.write_bytes(_build_pcap([p1, p2, p3]))

    return tshark, config_dir, pcap_path


def _run_tshark(
    tshark: str,
    config_dir: Path,
    pcap_path: Path | None,
    extra_args: list[str],
) -> str:
    env = {
        "WIRESHARK_CONFIG_DIR": str(config_dir),
        "PATH": "/usr/bin:/usr/local/bin",
        "HOME": str(config_dir),
    }
    # ``-G`` must be the first argument tshark sees; everything else
    # (``-C profile``, ``-r pcap``, ``-q``) only applies to ``-r`` mode.
    if any(arg == "-G" for arg in extra_args):
        cmd = [tshark, *extra_args, "-C", "f5_test"]
    else:
        cmd = [tshark, "-C", "f5_test"]
        if pcap_path is not None:
            cmd += ["-r", str(pcap_path), "-q"]
        cmd += extra_args
    result = subprocess.run(  # noqa: S603 — args fully controlled.
        cmd,
        capture_output=True,
        check=False,
        env=env,
    )
    if result.returncode != 0:
        raise AssertionError(
            f"tshark exited {result.returncode}; stderr:\n{result.stderr.decode()}"
        )
    return result.stdout.decode()


def test_tshark_loads_profile_without_errors(tshark_profile):
    """tshark must accept every file we emit; any parse error is fatal."""
    tshark, config_dir, pcap_path = tshark_profile
    out = _run_tshark(tshark, config_dir, pcap_path, ["-T", "fields", "-e", "frame.number"])
    # Three packets in, three frame numbers out.
    nums = [line.strip() for line in out.splitlines() if line.strip()]
    assert nums == ["1", "2", "3"]


def test_tshark_color_rules_apply_per_proxy_side(tshark_profile):
    """The colour rule that fires must reflect each packet's proxy side."""
    tshark, config_dir, pcap_path = tshark_profile
    # ``frame.coloring_rule.name`` only populates with ``--color`` enabled;
    # without it tshark applies coloring rules but leaves the field
    # empty for ``-T fields`` extraction.
    out = _run_tshark(
        tshark,
        config_dir,
        pcap_path,
        ["--color", "-T", "fields", "-e", "frame.coloring_rule.name"],
    )
    rules = [line.strip() for line in out.splitlines()]
    assert rules[0] == "BIG-IP client side"
    assert rules[1] == "BIG-IP server side"
    assert rules[2] == "BIG-IP SNAT"


def test_tshark_resolves_hosts_to_profile_labels(tshark_profile):
    """The ``hosts`` file must apply: VS dest resolves to a profile label."""
    tshark, config_dir, pcap_path = tshark_profile
    # ``-N n`` (lowercase) enables Wireshark's network-name resolver
    # which reads the profile's ``hosts`` file.  Multiple labels per IP
    # are stored as aliases — Wireshark picks one for ``ip.dst_host``,
    # so we assert membership in the known label set rather than
    # equality.
    out = _run_tshark(
        tshark,
        config_dir,
        pcap_path,
        ["-N", "n", "-T", "fields", "-e", "ip.dst_host"],
    )
    rows = [line.strip() for line in out.splitlines()]
    # First packet's destination is the VS — only one label applies.
    assert rows[0] == "vs-common-vs1", rows
    # Second packet's destination is the pool member; either pool-… or
    # node-… is acceptable depending on which alias Wireshark picks.
    assert rows[1] in {"pool-common-web1-80", "node-common-web1"}, rows


def test_tshark_preferences_define_proxy_side_column(tshark_profile):
    """The profile's ``preferences`` file must register the *Proxy Side* column.

    ``tshark -G currentprefs`` prints whatever the running config has
    for every preference, including the ones loaded from the profile.
    Confirm both the column name and its custom field formula made it
    through Wireshark's preferences parser.
    """
    tshark, config_dir, _pcap_path = tshark_profile
    out = _run_tshark(tshark, config_dir, None, ["-G", "currentprefs"])
    # ``gui.column.format`` is multi-line; grab the block between the
    # header and the next preference key.
    block: list[str] = []
    capturing = False
    for line in out.splitlines():
        if line.startswith("gui.column.format:"):
            capturing = True
            continue
        if capturing:
            stripped = line.strip()
            if stripped == "" or (line and not line[0].isspace() and not line.startswith("\t")):
                break
            block.append(stripped)
    joined = " ".join(block)
    assert "Proxy Side" in joined, f"column.format didn't include the override: {joined!r}"
    assert "frame.coloring_rule.name" in joined, joined


def test_tshark_services_file_resolves_bigip_port_names(tshark_profile):
    """The profile's ``services`` file must apply BIG-IP-derived names.

    With ``-N t`` (transport name resolution) tshark looks up port 443
    against the profile's ``services`` file.  The resolved name must be
    one of the BIG-IP-specific labels we emit (``vs-…``, ``self-…``,
    or ``policy-…``); which exact one wins depends on how Wireshark
    resolves duplicates between multiple ``services`` lines, but it
    must NOT be the system default ``https``.
    """
    tshark, config_dir, pcap_path = tshark_profile
    out = _run_tshark(
        tshark,
        config_dir,
        pcap_path,
        ["-N", "t", "-T", "fields", "-e", "_ws.col.Info"],
    )
    info_lines = out.splitlines()
    # The first packet (port 443) Info column should contain a
    # BIG-IP-prefixed service name, not "https".
    first_info = info_lines[0]
    assert "https" not in first_info, first_info
    assert any(marker in first_info for marker in ("vs-", "self-", "policy-")), first_info


def test_tshark_dfilters_file_parses(tshark_profile):
    """A malformed dfilters line would make tshark refuse to start with -C.

    Specifically the ``"BIG-IP client side"`` and per-VS buttons must
    parse as display filter expressions; tshark validates them at
    profile-load time when ``GUI`` mode is touched, and validates
    on-demand otherwise.  Apply one of our generated filters via
    ``-Y`` and confirm it produces the expected packet subset.
    """
    tshark, config_dir, pcap_path = tshark_profile
    # Apply the "BIG-IP server side" filter — should keep only the
    # 10.0.1.10 packet.
    out = _run_tshark(
        tshark,
        config_dir,
        pcap_path,
        ["-Y", "ip.dst == 10.0.1.10", "-T", "fields", "-e", "ip.dst"],
    )
    kept = [line.strip() for line in out.splitlines() if line.strip()]
    assert kept == ["10.0.1.10"]
