"""Tests for the M4 transformation verbs: emit, rename, redact, split, merge, convert."""

from __future__ import annotations

import json
import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

from core.bigip.emit import emit_merged, emit_split_by_partition, partition_of
from core.bigip.parser import parse_bigip_conf
from core.bigip.rewrite import redact_secrets, rename_object
from explorer.f5_cli import main
from explorer.f5_remote.ucs import make_test_ucs

SMALL = (
    textwrap.dedent(
        """
    ltm node /Common/n1 { address 10.0.0.1 }
    ltm pool /Common/p1 {
        members { /Common/n1:80 { address 10.0.0.1 } }
        load-balancing-mode round-robin
    }
    ltm rule /Common/r1 {
        when HTTP_REQUEST {
            pool /Common/p1
        }
    }
    ltm virtual /Common/vs_app {
        destination /Common/10.0.0.5:80
        pool /Common/p1
        rules { /Common/r1 }
    }
    """
    ).strip()
    + "\n"
)


def _run(args, capsys):
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


def _write(tmp_path, name, body):
    path = tmp_path / name
    path.write_text(body)
    return path


# ── emit core ────────────────────────────────────────────────────────


def test_partition_of_extracts_common():
    assert partition_of("/Common/foo") == "/Common/"
    assert partition_of("/Tenant1/foo") == "/Tenant1/"
    assert partition_of("") == ""
    assert partition_of("admin") == ""


def test_split_by_partition_separates_objects():
    body = SMALL + "ltm pool /Common2/p2 { }\n"
    parts = emit_split_by_partition(body)
    assert "Common" in parts
    assert "Common2" in parts
    assert "/Common/p1" in parts["Common"]
    assert "/Common2/p2" in parts["Common2"]


def test_split_then_merge_round_trip_keeps_objects():
    parts = emit_split_by_partition(SMALL)
    merged = emit_merged(parts)
    cfg_orig = parse_bigip_conf(SMALL)
    cfg_round = parse_bigip_conf(merged)
    assert set(cfg_orig.pools) == set(cfg_round.pools)
    assert set(cfg_orig.virtual_servers) == set(cfg_round.virtual_servers)
    assert set(cfg_orig.rules) == set(cfg_round.rules)


# ── rename core ──────────────────────────────────────────────────────


def test_rename_pool_updates_header_and_references():
    report = rename_object(SMALL, "/Common/p1", "/Common/p1_new")
    # 4 references: pool header, virtual.pool, irule pool command, plus the
    # "p1" short token inside the rule body. Exact count varies; require
    # the header + at least one reference.
    assert report.occurrences >= 2
    assert "/Common/p1_new" in report.new_source
    assert "/Common/p1 " not in report.new_source.replace("/Common/p1_new", "")


def test_rename_zero_matches_returns_zero():
    report = rename_object(SMALL, "/Common/no_such_thing", "/Common/x")
    assert report.occurrences == 0


def test_rename_does_not_match_substring():
    body = "ltm pool /Common/p1 { }\nltm pool /Common/p1_extra { }\n"
    report = rename_object(body, "/Common/p1", "/Common/q1")
    # Only /Common/p1 must rename, /Common/p1_extra must remain
    assert "/Common/q1" in report.new_source
    assert "/Common/p1_extra" in report.new_source
    # Old name must not appear anywhere as a standalone token
    assert "/Common/p1 " not in report.new_source.replace("/Common/p1_extra", "")


def test_rename_invalid_new_name_fails():
    with pytest.raises(ValueError):
        rename_object(SMALL, "/Common/p1", "")


def test_rename_does_not_touch_short_name_tokens():
    """Renaming /Common/foo must NOT rewrite bare 'foo' tokens elsewhere —
    they may appear in comments, unrelated property values, or arbitrary
    string literals.  See PR #392 review."""
    body = (
        "# foo is a great pool, btw\n"
        'ltm rule /Common/r { when HTTP_REQUEST { log local0. "foo bar" } }\n'
        "ltm pool /Common/foo { }\n"
        "ltm virtual /Common/v { description foo }\n"
    )
    report = rename_object(body, "/Common/foo", "/Common/baz")
    # /Common/foo header must rename
    assert "/Common/baz" in report.new_source
    # Bare 'foo' tokens elsewhere must survive
    assert "foo is a great pool" in report.new_source
    assert '"foo bar"' in report.new_source
    assert "description foo" in report.new_source


# ── rename verb ──────────────────────────────────────────────────────


def test_rename_verb_dry_run_emits_diff(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, out, _err = _run(["rename", "/Common/p1", "/Common/p1_new", str(p)], capsys)
    assert code == 0
    assert "+++" in out
    assert "/Common/p1_new" in out
    # Original file untouched
    assert p.read_text() == SMALL


def test_rename_verb_in_place(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, _out, _err = _run(["rename", "/Common/p1", "/Common/q1", str(p), "--in-place"], capsys)
    assert code == 0
    assert "/Common/q1" in p.read_text()
    assert "/Common/p1 " not in p.read_text()


def test_rename_verb_warns_when_no_match(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, _out, err = _run(["rename", "/Common/nope", "/Common/x", str(p)], capsys)
    assert code == 1
    assert "no occurrences" in err


# ── redact core ──────────────────────────────────────────────────────


def test_redact_replaces_password_field():
    body = (
        textwrap.dedent(
            """
        auth user /Common/admin {
            encrypted-password $6$abcdef$realhashvalue
        }
        sys snmp {
            community public
        }
        """
        ).strip()
        + "\n"
    )
    report = redact_secrets(body, remap_ips=False)
    assert "<REDACTED>" in report.new_source
    assert "realhashvalue" not in report.new_source
    assert "public" not in report.new_source.split("community")[1].split("\n")[0]
    assert report.secrets_replaced >= 2


def test_redact_replaces_pem_block():
    body = (
        "sys file ssl-cert /Common/c {\n"
        '    cert-text "-----BEGIN CERTIFICATE-----\\n'
        "MIIDxxxxxxxxxxxxxxxxxxxxxxx\\n"
        '-----END CERTIFICATE-----"\n'
        "}\n"
    )
    report = redact_secrets(body, remap_ips=False)
    assert "MIIDxxx" not in report.new_source
    assert "<REDACTED>" in report.new_source


def test_redact_remaps_public_ips_consistently():
    body = "ltm node /Common/n1 { address 8.8.8.8 }\nltm virtual /Common/v { destination /Common/8.8.8.8:443 }\n"
    report = redact_secrets(body, remap_ips=True)
    # Public IP gone
    assert "8.8.8.8" not in report.new_source
    # Both occurrences must have been remapped to the same private IP.
    import re

    seen = re.findall(r"\b10\.0\.\d+\.\d+\b", report.new_source)
    assert len(seen) == 2
    assert seen[0] == seen[1]


def test_redact_preserves_cidr_relationships():
    """Two real IPs in the same /24 must land in the same target /24."""
    body = (
        "ltm node /Common/a { address 1.2.3.5 }\n"
        "ltm node /Common/b { address 1.2.3.6 }\n"
        "ltm node /Common/c { address 8.8.8.10 }\n"
    )
    report = redact_secrets(body, remap_ips=True)
    import ipaddress
    import re

    addrs = [
        ipaddress.IPv4Address(t) for t in re.findall(r"\b\d+\.\d+\.\d+\.\d+\b", report.new_source)
    ]
    assert len(addrs) == 3
    a24 = ipaddress.ip_network(f"{addrs[0]}/24", strict=False)
    b24 = ipaddress.ip_network(f"{addrs[1]}/24", strict=False)
    c24 = ipaddress.ip_network(f"{addrs[2]}/24", strict=False)
    assert a24 == b24, "a and b were in the same /24 — they must remain so after redaction"
    assert c24 != a24, "c was in a different /24 — it must remain in a different /24"


def test_redact_round_trip_via_map():
    """redact -> apply_map(reverse=True) must recover the original IPs."""
    from core.bigip.redact_map import apply_map

    body = "ltm node /Common/n1 { address 8.8.8.8 }\nltm virtual /Common/v { destination 1.1.1.1:443 }\n"
    report = redact_secrets(body, remap_ips=True)
    rm = report.redaction_map
    assert rm is not None
    recovered, _ = apply_map(rm, report.new_source, reverse=True)
    assert "8.8.8.8" in recovered
    assert "1.1.1.1" in recovered


def test_redact_shuffle_round_trip():
    from core.bigip.redact_map import apply_map

    body = "host 8.8.8.4 host 8.8.8.5 host 8.8.8.6\n"
    report = redact_secrets(body, remap_ips=True, mode="shuffle", seed="test-seed")
    rm = report.redaction_map
    assert rm is not None
    recovered, _ = apply_map(rm, report.new_source, reverse=True)
    assert recovered == body


def test_redact_keeps_private_ips_unchanged():
    body = "ltm node /Common/n1 { address 10.0.0.5 }\n"
    report = redact_secrets(body, remap_ips=True)
    assert "10.0.0.5" in report.new_source
    assert report.ips_remapped == 0


# ── redact verb ──────────────────────────────────────────────────────


def test_redact_verb(tmp_path, capsys):
    body = "auth user /Common/admin { password sekret123 }\n"
    p = _write(tmp_path, "c.conf", body)
    code, out, err = _run(["redact", str(p)], capsys)
    assert code == 0
    assert "sekret123" not in out
    assert "<REDACTED>" in out
    assert "redacted:" in err


def test_redact_verb_writes_map_sidecar(tmp_path, capsys):
    body = "ltm node /Common/n { address 8.8.8.8 }\n"
    p = _write(tmp_path, "c.conf", body)
    out_path = tmp_path / "redacted.conf"
    code, _out, _err = _run(["redact", str(p), "-o", str(out_path)], capsys)
    assert code == 0
    map_path = tmp_path / "redacted.conf.redact.toml"
    assert map_path.is_file()
    map_text = map_path.read_text()
    assert "schema" in map_text
    assert "8.8.8.8" in map_text


def test_redact_verb_target_cidr_flag(tmp_path, capsys):
    body = "ltm node /Common/n { address 8.8.8.8 }\n"
    p = _write(tmp_path, "c.conf", body)
    out_path = tmp_path / "redacted.conf"
    code, _out, _err = _run(
        ["redact", str(p), "-o", str(out_path), "--target-cidr", "192.0.2.0/24"], capsys
    )
    assert code == 0
    redacted = out_path.read_text()
    assert "8.8.8.8" not in redacted
    # IP must fall in the user-specified pool
    import re

    addrs = re.findall(r"\b\d+\.\d+\.\d+\.\d+\b", redacted)
    import ipaddress

    pool = ipaddress.IPv4Network("192.0.2.0/24")
    for addr in addrs:
        assert ipaddress.IPv4Address(addr) in pool


def test_unredact_verb_recovers_originals(tmp_path, capsys):
    body = "ltm node /Common/n1 { address 8.8.8.8 }\nlog: contact 8.8.8.8 for help\n"
    p = _write(tmp_path, "c.conf", body)
    out_path = tmp_path / "redacted.conf"
    code, _out, _err = _run(["redact", str(p), "-o", str(out_path)], capsys)
    assert code == 0
    map_path = tmp_path / "redacted.conf.redact.toml"

    code, recovered, err = _run(["unredact", str(map_path), str(out_path)], capsys)
    assert code == 0, err
    assert "8.8.8.8" in recovered
    assert "unredacted:" in err


def test_unredact_works_on_external_text(tmp_path, capsys):
    """Unredacting a support email containing only redacted IPs must work."""
    body = "ltm node /Common/n1 { address 1.2.3.42 }\n"
    p = _write(tmp_path, "c.conf", body)
    out_path = tmp_path / "redacted.conf"
    _run(["redact", str(p), "-o", str(out_path)], capsys)
    redacted_addr = None
    import re

    for tok in re.findall(r"\b\d+\.\d+\.\d+\.\d+\b", out_path.read_text()):
        if tok != "1.2.3.42":
            redacted_addr = tok
            break
    assert redacted_addr is not None

    support_text = f"We see traffic to {redacted_addr} failing handshake.\n"
    snippet = _write(tmp_path, "support.txt", support_text)
    map_path = tmp_path / "redacted.conf.redact.toml"
    code, recovered, _err = _run(["unredact", str(map_path), str(snippet)], capsys)
    assert code == 0
    assert "1.2.3.42" in recovered


# ── split / merge verbs ──────────────────────────────────────────────


def test_split_verb_creates_per_partition_files(tmp_path, capsys):
    body = SMALL + "ltm pool /Common2/p2 { }\n"
    p = _write(tmp_path, "c.conf", body)
    out_dir = tmp_path / "split"
    code, _out, _err = _run(["split", str(p), str(out_dir)], capsys)
    assert code == 0
    files = sorted(f.name for f in out_dir.iterdir())
    assert "Common.conf" in files
    assert "Common2.conf" in files


def test_merge_verb_concatenates_dir(tmp_path, capsys):
    body = SMALL + "ltm pool /Common2/p2 { }\n"
    p = _write(tmp_path, "c.conf", body)
    out_dir = tmp_path / "split"
    _run(["split", str(p), str(out_dir)], capsys)

    code, out, _err = _run(["merge", str(out_dir)], capsys)
    assert code == 0
    cfg = parse_bigip_conf(out)
    assert "/Common/p1" in cfg.pools
    assert "/Common2/p2" in cfg.pools


def test_split_merge_round_trip(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    out_dir = tmp_path / "split"
    _run(["split", str(p), str(out_dir)], capsys)

    code, out, _err = _run(["merge", str(out_dir)], capsys)
    assert code == 0
    original = parse_bigip_conf(SMALL)
    round = parse_bigip_conf(out)
    assert set(original.pools) == set(round.pools)
    assert set(original.virtual_servers) == set(round.virtual_servers)
    assert set(original.rules) == set(round.rules)


# ── convert verb ─────────────────────────────────────────────────────


def test_convert_ucs2scf(tmp_path, capsys):
    ucs = tmp_path / "d.ucs"
    ucs.write_bytes(make_test_ucs({"config/bigip.conf": "ltm pool /Common/u { }\n"}))
    code, out, _err = _run(["convert", "ucs2scf", str(ucs)], capsys)
    assert code == 0
    assert "/Common/u" in out


def test_convert_ucs2scf_rejects_non_ucs(tmp_path, capsys):
    plain = tmp_path / "p.scf"
    plain.write_text("ltm pool /Common/x { }\n")
    code, _out, err = _run(["convert", "ucs2scf", str(plain)], capsys)
    assert code == 2
    assert "not a UCS" in err


def test_convert_scf2as3_basic(tmp_path, capsys):
    p = _write(tmp_path, "c.conf", SMALL)
    code, out, _err = _run(["convert", "scf2as3", str(p)], capsys)
    assert code == 0
    payload = json.loads(out)
    assert payload["class"] == "AS3"
    common = payload["declaration"]["Common"]
    app = common["app"]
    assert "p1" in app
    assert app["p1"]["class"] == "Pool"
    assert "vs_app" in app
    assert app["vs_app"]["class"] in {
        "Service_HTTP",
        "Service_TCP",
        "Service_L4",
        "Service_HTTPS",
        "Service_UDP",
    }


def test_convert_scf2as3_report_lists_unmapped(tmp_path, capsys):
    body = SMALL + "ltm profile http /Common/h_profile { }\n"
    p = _write(tmp_path, "c.conf", body)
    code, _out, err = _run(["convert", "scf2as3", str(p), "--report"], capsys)
    assert code == 0
    assert "unmapped" in err
    assert "profile:/Common/h_profile" in err
