"""Tests for ``f5 extract`` and the underlying UCS helpers."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


from tooling.cli.f5_remote.ucs import is_ucs_bytes, make_test_ucs, ucs_to_scf
from tooling.f5.main import main


def _run(args, capsys):
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


def test_is_ucs_bytes_recognises_gzip_magic():
    ucs = make_test_ucs({"config/bigip.conf": "ltm pool /Common/p { }"})
    assert is_ucs_bytes(ucs)
    assert not is_ucs_bytes(b"plain text")


def test_ucs_to_scf_concatenates_in_canonical_order():
    members = {
        "config/bigip.conf": "ltm pool /Common/p1 { }\n",
        "config/bigip_base.conf": "auth partition /Common { }\n",
        "config/bigip_gtm.conf": "gtm wideip a /Common/w { }\n",
    }
    scf = ucs_to_scf(make_test_ucs(members))
    # bigip_base must appear before bigip; bigip before bigip_gtm.
    base_idx = scf.index("auth partition")
    main_idx = scf.index("ltm pool")
    gtm_idx = scf.index("gtm wideip")
    assert base_idx < main_idx < gtm_idx


def test_ucs_to_scf_skips_extras_by_default():
    members = {
        "config/bigip.conf": "ltm pool /Common/p { }\n",
        "config/profile_base.conf": "ltm profile http /Common/h { }\n",
    }
    default = ucs_to_scf(make_test_ucs(members))
    extras = ucs_to_scf(make_test_ucs(members), include_extras=True)
    assert "profile_base" not in default
    assert "profile_base" in extras


def test_extract_verb_writes_scf(tmp_path, capsys):
    ucs_path = tmp_path / "device.ucs"
    ucs_path.write_bytes(make_test_ucs({"config/bigip.conf": "ltm pool /Common/x { }\n"}))

    out_path = tmp_path / "device.scf"
    code, _out, err = _run(["extract", str(ucs_path), "-o", str(out_path)], capsys)

    assert code == 0, err
    assert "ltm pool /Common/x" in out_path.read_text()


def test_extract_verb_to_stdout(tmp_path, capsys):
    ucs_path = tmp_path / "device.ucs"
    ucs_path.write_bytes(make_test_ucs({"config/bigip.conf": "ltm node /Common/n { }\n"}))

    code, out, _err = _run(["extract", str(ucs_path)], capsys)

    assert code == 0
    assert "ltm node /Common/n" in out


def test_extract_verb_rejects_non_ucs(tmp_path, capsys):
    plain = tmp_path / "plain.txt"
    plain.write_text("not a ucs")

    code, _out, err = _run(["extract", str(plain)], capsys)

    assert code == 2
    assert "not a UCS archive" in err


def test_extract_verb_missing_file(tmp_path, capsys):
    code, _out, err = _run(["extract", str(tmp_path / "missing.ucs")], capsys)
    assert code == 2
    assert "not a file" in err


def test_extract_aliased_as_ucs2scf(tmp_path, capsys):
    ucs_path = tmp_path / "d.ucs"
    ucs_path.write_bytes(make_test_ucs({"config/bigip.conf": "ltm pool /Common/q { }\n"}))
    code, out, _err = _run(["ucs2scf", str(ucs_path)], capsys)
    assert code == 0
    assert "ltm pool /Common/q" in out
