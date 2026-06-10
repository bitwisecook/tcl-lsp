"""Integration tests for tcl venv CLI verbs."""

from __future__ import annotations

import json
import shutil

import pytest

from tooling.tcl.main import main


def _run(args: list[str], capsys) -> tuple[int, str, str]:
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


_has_tclsh = shutil.which("tclsh") is not None or shutil.which("tclsh8.6") is not None


class TestVenvCreate:
    @pytest.mark.skipif(not _has_tclsh, reason="tclsh not available")
    def test_creates_venv_directory(self, tmp_path, capsys):
        venv_dir = tmp_path / ".venv"
        code, out, _ = _run(["venv", "create", str(venv_dir)], capsys)
        assert code == 0
        assert venv_dir.is_dir()
        assert (venv_dir / "tclvenv.cfg").is_file()
        assert (venv_dir / "bin" / "tclsh").is_file()
        assert (venv_dir / "bin" / "activate").is_file()
        assert (venv_dir / "bin" / "activate.fish").is_file()
        assert (venv_dir / "lib" / "pkgIndex.tcl").is_file()

    @pytest.mark.skipif(not _has_tclsh, reason="tclsh not available")
    def test_tclvenv_cfg_has_version(self, tmp_path, capsys):
        venv_dir = tmp_path / ".venv"
        _run(["venv", "create", str(venv_dir)], capsys)
        cfg = (venv_dir / "tclvenv.cfg").read_text()
        assert "tcl_version" in cfg
        assert "tcl_executable" in cfg
        assert "prompt" in cfg

    def test_create_refuses_existing_dir(self, tmp_path, capsys):
        venv_dir = tmp_path / ".venv"
        venv_dir.mkdir()
        code, _, err = _run(["venv", "create", str(venv_dir)], capsys)
        assert code == 1
        assert "already exists" in err

    @pytest.mark.skipif(not _has_tclsh, reason="tclsh not available")
    def test_create_force_overwrites(self, tmp_path, capsys):
        venv_dir = tmp_path / ".venv"
        venv_dir.mkdir()
        (venv_dir / "marker.txt").write_text("old")
        code, out, _ = _run(["venv", "create", str(venv_dir), "--force"], capsys)
        assert code == 0
        assert not (venv_dir / "marker.txt").exists()
        assert (venv_dir / "tclvenv.cfg").is_file()

    @pytest.mark.skipif(not _has_tclsh, reason="tclsh not available")
    def test_create_json_output(self, tmp_path, capsys):
        venv_dir = tmp_path / ".venv"
        code, out, _ = _run(["venv", "create", str(venv_dir), "--json"], capsys)
        assert code == 0
        data = json.loads(out)
        assert "path" in data


class TestVenvInfo:
    @pytest.mark.skipif(not _has_tclsh, reason="tclsh not available")
    def test_shows_venv_info(self, tmp_path, capsys):
        venv_dir = tmp_path / ".venv"
        _run(["venv", "create", str(venv_dir)], capsys)
        code, out, _ = _run(["venv", "info", str(venv_dir)], capsys)
        assert code == 0
        assert "tcl_version" in out

    @pytest.mark.skipif(not _has_tclsh, reason="tclsh not available")
    def test_info_json(self, tmp_path, capsys):
        venv_dir = tmp_path / ".venv"
        _run(["venv", "create", str(venv_dir)], capsys)
        code, out, _ = _run(["venv", "info", str(venv_dir), "--json"], capsys)
        assert code == 0
        data = json.loads(out)
        assert "tcl_version" in data

    def test_info_nonexistent_fails(self, tmp_path, capsys):
        code, _, err = _run(["venv", "info", str(tmp_path / "nope")], capsys)
        assert code == 1
        assert "not a tclpkg venv" in err


class TestVenvDelete:
    @pytest.mark.skipif(not _has_tclsh, reason="tclsh not available")
    def test_deletes_venv(self, tmp_path, capsys):
        venv_dir = tmp_path / ".venv"
        _run(["venv", "create", str(venv_dir)], capsys)
        assert venv_dir.is_dir()
        code, out, _ = _run(["venv", "delete", str(venv_dir)], capsys)
        assert code == 0
        assert not venv_dir.exists()

    def test_delete_nonexistent_fails(self, tmp_path, capsys):
        code, _, err = _run(["venv", "delete", str(tmp_path / "nope")], capsys)
        assert code == 1

    def test_delete_non_venv_refuses(self, tmp_path, capsys):
        plain_dir = tmp_path / "notavenv"
        plain_dir.mkdir()
        code, _, err = _run(["venv", "delete", str(plain_dir)], capsys)
        assert code == 1
        assert "not a tclpkg venv" in err


class TestVenvActivate:
    @pytest.mark.skipif(not _has_tclsh, reason="tclsh not available")
    def test_prints_activation_script(self, tmp_path, capsys):
        venv_dir = tmp_path / ".venv"
        _run(["venv", "create", str(venv_dir)], capsys)
        code, out, _ = _run(["venv", "activate", str(venv_dir)], capsys)
        assert code == 0
        assert "TCLLIBPATH" in out
        assert "TCL_VENV" in out
        assert "deactivate" in out

    @pytest.mark.skipif(not _has_tclsh, reason="tclsh not available")
    def test_activate_fish(self, tmp_path, capsys):
        venv_dir = tmp_path / ".venv"
        _run(["venv", "create", str(venv_dir)], capsys)
        code, out, _ = _run(["venv", "activate", str(venv_dir), "--shell", "fish"], capsys)
        assert code == 0
        assert "functions -e deactivate" in out
