"""Tests for the shared ``--format scf|tmsh`` flag on f5 verbs.

Every config-emitting verb (``extract`` is covered separately because
its input is a UCS archive) accepts ``--format scf`` (the historical
output, unchanged) or ``--format tmsh`` (a `tmsh create` or
`tmsh modify` script suitable for replay).  The shared helper lives in
:mod:`explorer.verbs.f5._emit`.
"""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from tooling.f5.main import main
from tooling.f5.verbs._emit import render_config

SAMPLE = (
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
    out = capsys.readouterr()
    return code, out.out, out.err


def _write(tmp_path, name, body):
    path = tmp_path / name
    path.write_text(body)
    return path


# ── render_config helper ─────────────────────────────────────────────


def test_render_config_scf_returns_text_verbatim():
    out = render_config(SAMPLE, fmt="scf", tmsh_verb="create")
    assert out == SAMPLE


def test_render_config_tmsh_uses_create_by_default():
    out = render_config(SAMPLE, fmt="tmsh", tmsh_verb="create")
    assert "tmsh create ltm pool /Common/p1" in out
    assert "tmsh modify" not in out


def test_render_config_tmsh_modify_for_rewriters():
    out = render_config(SAMPLE, fmt="tmsh", tmsh_verb="modify")
    assert "tmsh modify ltm pool /Common/p1" in out
    assert "tmsh create" not in out


def test_render_config_falls_back_to_raw_when_unparseable(capsys):
    text = "this is not a bigip config at all\n"
    out = render_config(text, fmt="tmsh", tmsh_verb="create")
    err = capsys.readouterr().err
    assert out == text
    assert "did not parse" in err


# ── merge ────────────────────────────────────────────────────────────


def test_merge_scf_is_unchanged_default(tmp_path, capsys):
    p = _write(tmp_path, "in.conf", SAMPLE)
    code, out, _err = _run(["merge", str(p)], capsys)
    assert code == 0
    assert "ltm pool /Common/p1" in out
    assert "tmsh create" not in out


def test_merge_tmsh_emits_tmsh_create(tmp_path, capsys):
    p = _write(tmp_path, "in.conf", SAMPLE)
    code, out, _err = _run(["merge", str(p), "--format", "tmsh"], capsys)
    assert code == 0
    assert "tmsh create ltm pool /Common/p1" in out
    assert "tmsh create ltm virtual /Common/vs_app" in out


# ── split ────────────────────────────────────────────────────────────


def test_split_scf_writes_dot_conf_files(tmp_path, capsys):
    p = _write(tmp_path, "in.conf", SAMPLE)
    out_dir = tmp_path / "out"
    code, _out, _err = _run(["split", str(p), str(out_dir)], capsys)
    assert code == 0
    assert any(out_dir.glob("*.conf"))
    assert not any(out_dir.glob("*.tmsh"))


def test_split_tmsh_writes_dot_tmsh_files(tmp_path, capsys):
    p = _write(tmp_path, "in.conf", SAMPLE)
    out_dir = tmp_path / "out"
    code, _out, _err = _run(["split", str(p), str(out_dir), "--format", "tmsh"], capsys)
    assert code == 0
    tmsh_files = list(out_dir.glob("*.tmsh"))
    assert tmsh_files
    body = tmsh_files[0].read_text()
    assert "tmsh create" in body


# ── rename ───────────────────────────────────────────────────────────


def test_rename_tmsh_emits_tmsh_modify(tmp_path, capsys):
    p = _write(tmp_path, "in.conf", SAMPLE)
    code, out, _err = _run(
        [
            "rename",
            "/Common/p1",
            "/Common/p1_new",
            str(p),
            "--write",
            "--format",
            "tmsh",
        ],
        capsys,
    )
    assert code == 0
    assert "tmsh modify ltm pool /Common/p1_new" in out
    # Rewriters never emit `tmsh create` — they patch existing objects.
    assert "tmsh create" not in out
    # The rename actually propagated through the parse/re-emit step.
    assert "p1_new" in out and "/Common/p1 " not in out


def test_rename_scf_unchanged_default(tmp_path, capsys):
    # Default --format scf must keep the historical unified-diff preview.
    p = _write(tmp_path, "in.conf", SAMPLE)
    code, out, _err = _run(["rename", "/Common/p1", "/Common/p1_new", str(p)], capsys)
    assert code == 0
    assert out.startswith("---") or "@@" in out  # unified diff markers


# ── redact ───────────────────────────────────────────────────────────


_REDACT_SOURCE = (
    textwrap.dedent(
        """
    ltm node /Common/n1 { address 8.8.8.8 }
    ltm pool /Common/p1 { members { /Common/n1:80 { address 8.8.8.8 } } }
    auth user admin { password supersecret }
    """
    ).strip()
    + "\n"
)


def test_redact_tmsh_emits_tmsh_modify(tmp_path, capsys):
    p = _write(tmp_path, "in.conf", _REDACT_SOURCE)
    code, out, _err = _run(
        [
            "redact",
            str(p),
            "--keep-ips",
            "--format",
            "tmsh",
        ],
        capsys,
    )
    assert code == 0
    assert "tmsh modify" in out
    assert "supersecret" not in out


# ── unredact ─────────────────────────────────────────────────────────


def test_unredact_tmsh_emits_tmsh_modify(tmp_path, capsys):
    src = _write(tmp_path, "in.conf", _REDACT_SOURCE)
    redacted = tmp_path / "redacted.conf"
    map_path = tmp_path / "map.toml"
    code, _out, _err = _run(
        [
            "redact",
            str(src),
            "-o",
            str(redacted),
            "--map-file",
            str(map_path),
        ],
        capsys,
    )
    assert code == 0
    # Now round-trip through unredact --format tmsh.
    code, out, _err = _run(
        [
            "unredact",
            str(map_path),
            str(redacted),
            "--format",
            "tmsh",
        ],
        capsys,
    )
    assert code == 0
    assert "tmsh modify ltm pool /Common/p1" in out


# ── grep ─────────────────────────────────────────────────────────────


def test_grep_scf_default_emits_text_report(tmp_path, capsys):
    p = _write(tmp_path, "in.conf", SAMPLE)
    code, out, _err = _run(["grep", "p1", str(p)], capsys)
    assert code == 0
    assert "# tcl-lsp BIG-IP grep" in out


def test_grep_tmsh_emits_only_matched_stanzas_as_tmsh(tmp_path, capsys):
    p = _write(tmp_path, "in.conf", SAMPLE)
    code, out, _err = _run(["grep", "vs_app", str(p), "--format", "tmsh"], capsys)
    assert code == 0
    # No report scaffolding in tmsh mode.
    assert "# tcl-lsp BIG-IP grep" not in out
    assert "# Searches" not in out
    # The seed virtual and at least one related object are emitted as
    # tmsh create commands.
    assert "tmsh create ltm virtual /Common/vs_app" in out
    assert "tmsh create ltm pool /Common/p1" in out


def test_grep_tmsh_does_not_collapse_same_path_across_files(tmp_path, capsys):
    # Regression for the dedupe bug Copilot flagged on `_matched_stanzas_to_scf`:
    # an object with the same /Common path in two different input files
    # must survive in the tmsh output as two stanzas, not one.
    src_a = (
        textwrap.dedent(
            """
        ltm node /Common/n1 { address 10.0.0.1 }
        ltm pool /Common/p1 { members { /Common/n1:80 { address 10.0.0.1 } } }
        """
        ).strip()
        + "\n"
    )
    src_b = (
        textwrap.dedent(
            """
        ltm node /Common/n1 { address 10.0.0.99 }
        ltm pool /Common/p1 { members { /Common/n1:80 { address 10.0.0.99 } } }
        """
        ).strip()
        + "\n"
    )
    a = _write(tmp_path, "a.conf", src_a)
    b = _write(tmp_path, "b.conf", src_b)
    code, out, _err = _run(["grep", "/Common/p1", str(a), str(b), "--format", "tmsh"], capsys)
    assert code == 0
    # Both copies of /Common/p1 must appear — collapsing on full_path
    # alone would emit only one `tmsh create ltm pool /Common/p1` line.
    assert out.count("tmsh create ltm pool /Common/p1") == 2
