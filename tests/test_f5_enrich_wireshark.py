"""Tests for ``f5 enrich-wireshark`` and the Wireshark profile builder."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.bigip.parser import parse_bigip_conf
from core.bigip.wireshark_profile import (
    WiresharkProfile,
    _extract_vlans,
    build_wireshark_profile,
)
from explorer.f5_cli import main


def _run(args, capsys):
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


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

net vlan /Common/external {
    tag 100
    interfaces {
        1.1 { }
    }
}

net vlan /Common/internal {
    tag 200
}
"""


def test_extract_vlans_pulls_tag_per_block():
    vlans = dict(_extract_vlans(SAMPLE_CONFIG))
    assert vlans["/Common/external"] == 100
    assert vlans["/Common/internal"] == 200


def test_build_wireshark_profile_covers_every_file():
    cfg = parse_bigip_conf(SAMPLE_CONFIG)
    profile = build_wireshark_profile([(cfg, SAMPLE_CONFIG)])

    # hosts: every IP gets at least one entry.
    hosts_addrs = {addr for addr, _name in profile.hosts}
    assert "10.0.0.1" in hosts_addrs
    assert "10.0.1.10" in hosts_addrs
    assert "10.0.5.1" in hosts_addrs
    # And vs-common-vs1 ended up against 10.0.0.1.
    assert ("10.0.0.1", "vs-common-vs1") in profile.hosts

    # subnets: 10.0.5.0/24 -> net-common-external-self.
    assert ("10.0.5.0/24", "net-common-external-self") in profile.subnets

    # vlans: tag 100 -> vlan-common-external; 200 -> vlan-common-internal.
    assert (100, "vlan-common-external") in profile.vlans
    assert (200, "vlan-common-internal") in profile.vlans

    # dfilters: a button per VS and per self subnet.
    labels = {label for label, _expr in profile.dfilters}
    assert "vs-common-vs1" in labels
    assert "net-common-external-self" in labels
    vs_button = next(expr for label, expr in profile.dfilters if label == "vs-common-vs1")
    assert vs_button == "ip.addr == 10.0.0.1"


def test_wireshark_profile_write_creates_expected_files(tmp_path):
    cfg = parse_bigip_conf(SAMPLE_CONFIG)
    profile = build_wireshark_profile([(cfg, SAMPLE_CONFIG)])

    out_dir = tmp_path / "f5_profile"
    result = profile.write_to(out_dir)

    assert "hosts" in result.files_written
    assert "subnets" in result.files_written
    assert "vlans" in result.files_written
    assert "dfilters" in result.files_written
    assert "README.md" in result.files_written

    hosts_text = (out_dir / "hosts").read_text()
    assert "10.0.0.1" in hosts_text
    assert "vs-common-vs1" in hosts_text

    dfilters_text = (out_dir / "dfilters").read_text()
    assert '"vs-common-vs1" ip.addr == 10.0.0.1' in dfilters_text


def test_wireshark_profile_deduplicates_across_inputs():
    """Feeding the same config twice must not double every row."""
    cfg = parse_bigip_conf(SAMPLE_CONFIG)
    profile = build_wireshark_profile([(cfg, SAMPLE_CONFIG), (cfg, SAMPLE_CONFIG)])

    # Each unique (ip, name) pair should appear exactly once.
    assert profile.hosts == list(dict.fromkeys(profile.hosts))


def test_wireshark_profile_empty_when_no_inputs():
    profile = build_wireshark_profile([])
    assert profile == WiresharkProfile()


# CLI smoke tests


def test_enrich_wireshark_cli_writes_directory(tmp_path, capsys):
    cfg = tmp_path / "bigip.conf"
    cfg.write_text(SAMPLE_CONFIG)
    out_dir = tmp_path / "profile"

    code, _out, err = _run(
        ["enrich-wireshark", "-c", str(cfg), str(out_dir)], capsys
    )

    assert code == 0
    assert (out_dir / "hosts").is_file()
    assert (out_dir / "subnets").is_file()
    assert (out_dir / "vlans").is_file()
    assert (out_dir / "dfilters").is_file()
    assert (out_dir / "README.md").is_file()
    assert "host" in err  # summary line mentions hosts


def test_enrich_wireshark_cli_refuses_to_overwrite_without_force(tmp_path, capsys):
    cfg = tmp_path / "bigip.conf"
    cfg.write_text(SAMPLE_CONFIG)
    out_dir = tmp_path / "profile"
    out_dir.mkdir()
    (out_dir / "leftover").write_text("preexisting")

    code, _out, err = _run(
        ["enrich-wireshark", "-c", str(cfg), str(out_dir)], capsys
    )
    assert code == 2
    assert "already exists" in err
    # The pre-existing file should still be there.
    assert (out_dir / "leftover").exists()


def test_enrich_wireshark_cli_force_overwrites(tmp_path, capsys):
    cfg = tmp_path / "bigip.conf"
    cfg.write_text(SAMPLE_CONFIG)
    out_dir = tmp_path / "profile"
    out_dir.mkdir()
    (out_dir / "stale_hosts").write_text("# unrelated")

    code, _out, _err = _run(
        ["enrich-wireshark", "--force", "-c", str(cfg), str(out_dir)], capsys
    )
    assert code == 0
    assert (out_dir / "hosts").is_file()


def test_enrich_wireshark_cli_multi_config_merges_inventory(tmp_path, capsys):
    """GTM in one input, LTM (server defs) in another — both contribute labels."""
    gtm = tmp_path / "gtm.conf"
    gtm.write_text(
        "gtm pool a /Common/wip_pool {\n"
        "    members { /Common/dc1:vs_a { ratio 1 } }\n"
        "}\n"
        "gtm wideip a /Common/example.com {\n"
        "    pools { /Common/wip_pool { } }\n"
        "}\n"
    )
    ltm = tmp_path / "ltm.conf"
    ltm.write_text(
        "gtm server /Common/dc1 {\n"
        "    addresses { 10.0.0.5 { device-name dc1 } }\n"
        "    virtual-servers { vs_a { destination 10.0.0.5:443 } }\n"
        "}\n"
    )
    out_dir = tmp_path / "profile"

    code, _out, _err = _run(
        [
            "enrich-wireshark",
            "-c",
            str(gtm),
            "-c",
            str(ltm),
            str(out_dir),
        ],
        capsys,
    )
    assert code == 0
    hosts = (out_dir / "hosts").read_text()
    assert "wideip-common-example-com" in hosts
    assert "10.0.0.5" in hosts
