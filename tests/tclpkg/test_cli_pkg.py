"""Integration tests for tcl pkg CLI verbs."""

from __future__ import annotations

import json

from explorer.tcl_cli import main


def _run(args: list[str], capsys) -> tuple[int, str, str]:
    code = main(args)
    captured = capsys.readouterr()
    return code, captured.out, captured.err


class TestPkgInit:
    def test_creates_manifest(self, tmp_path, capsys, monkeypatch):
        monkeypatch.chdir(tmp_path)
        code, out, _ = _run(["pkg", "init", "--name", "demo", "--version", "0.1.0"], capsys)
        assert code == 0
        manifest = tmp_path / "tclpkg.tcl"
        assert manifest.is_file()
        text = manifest.read_text()
        assert "package" in text
        assert "demo" in text
        assert "0.1.0" in text

    def test_refuses_overwrite_without_force(self, tmp_path, capsys, monkeypatch):
        monkeypatch.chdir(tmp_path)
        (tmp_path / "tclpkg.tcl").write_text("package x\nversion 1.0.0\n")
        code, _, err = _run(["pkg", "init", "--name", "x"], capsys)
        assert code == 1
        assert "already exists" in err

    def test_force_overwrites(self, tmp_path, capsys, monkeypatch):
        monkeypatch.chdir(tmp_path)
        (tmp_path / "tclpkg.tcl").write_text("package old\nversion 0.0.1\n")
        code, out, _ = _run(["pkg", "init", "--name", "new", "--force"], capsys)
        assert code == 0
        assert "new" in (tmp_path / "tclpkg.tcl").read_text()


class TestPkgAdd:
    def test_appends_require(self, tmp_path, capsys, monkeypatch):
        monkeypatch.chdir(tmp_path)
        (tmp_path / "tclpkg.tcl").write_text("package demo\nversion 1.0.0\n")
        code, out, _ = _run(["pkg", "add", "json", "1.3"], capsys)
        assert code == 0
        text = (tmp_path / "tclpkg.tcl").read_text()
        assert "require json 1.3" in text

    def test_add_dev(self, tmp_path, capsys, monkeypatch):
        monkeypatch.chdir(tmp_path)
        (tmp_path / "tclpkg.tcl").write_text("package demo\nversion 1.0.0\n")
        code, _, _ = _run(["pkg", "add", "tcltest", "2.5", "--dev"], capsys)
        assert code == 0
        assert "dev-require tcltest 2.5" in (tmp_path / "tclpkg.tcl").read_text()


class TestPkgInstall:
    def test_creates_lockfile(self, tmp_path, capsys, monkeypatch):
        monkeypatch.chdir(tmp_path)
        (tmp_path / "tclpkg.tcl").write_text("package demo\nversion 1.0.0\nrequire json 1.0\n")
        code, out, _ = _run(["pkg", "install"], capsys)
        assert code == 0
        lockfile = tmp_path / "tclpkg.lock"
        assert lockfile.is_file()
        data = json.loads(lockfile.read_text())
        assert data["name"] == "demo"
        names = [p["name"] for p in data["packages"]]
        assert "json" in names

    def test_frozen_fails_without_lockfile(self, tmp_path, capsys, monkeypatch):
        monkeypatch.chdir(tmp_path)
        (tmp_path / "tclpkg.tcl").write_text("package demo\nversion 1.0.0\n")
        code, _, err = _run(["pkg", "install", "--frozen"], capsys)
        assert code == 1
        assert "--frozen" in err

    def test_frozen_succeeds_when_unchanged(self, tmp_path, capsys, monkeypatch):
        monkeypatch.chdir(tmp_path)
        (tmp_path / "tclpkg.tcl").write_text("package demo\nversion 1.0.0\nrequire json 1.0\n")
        _run(["pkg", "install"], capsys)
        code, out, _ = _run(["pkg", "install", "--frozen"], capsys)
        assert code == 0
        assert "up to date" in out

    def test_frozen_fails_when_manifest_changes(self, tmp_path, capsys, monkeypatch):
        monkeypatch.chdir(tmp_path)
        (tmp_path / "tclpkg.tcl").write_text("package demo\nversion 1.0.0\nrequire json 1.0\n")
        _run(["pkg", "install"], capsys)
        (tmp_path / "tclpkg.tcl").write_text(
            "package demo\nversion 1.0.0\nrequire json 1.0\nrequire http 2.0\n"
        )
        code, _, err = _run(["pkg", "install", "--frozen"], capsys)
        assert code == 1
        assert "would change" in err


class TestPkgListTreeFreeze:
    def _setup(self, tmp_path, capsys, monkeypatch):
        monkeypatch.chdir(tmp_path)
        (tmp_path / "tclpkg.tcl").write_text(
            "package demo\nversion 1.0.0\nrequire json 1.0\ndev-require tcltest 2.5\n"
        )
        _run(["pkg", "install"], capsys)

    def test_list(self, tmp_path, capsys, monkeypatch):
        self._setup(tmp_path, capsys, monkeypatch)
        code, out, _ = _run(["pkg", "list"], capsys)
        assert code == 0
        assert "json" in out
        assert "tcltest" in out

    def test_list_json(self, tmp_path, capsys, monkeypatch):
        self._setup(tmp_path, capsys, monkeypatch)
        code, out, _ = _run(["pkg", "list", "--json"], capsys)
        assert code == 0
        data = json.loads(out)
        assert "packages" in data

    def test_tree(self, tmp_path, capsys, monkeypatch):
        self._setup(tmp_path, capsys, monkeypatch)
        code, out, _ = _run(["pkg", "tree"], capsys)
        assert code == 0
        assert "demo" in out
        assert "json" in out

    def test_freeze(self, tmp_path, capsys, monkeypatch):
        self._setup(tmp_path, capsys, monkeypatch)
        code, out, _ = _run(["pkg", "freeze"], capsys)
        assert code == 0
        assert "require json 1.0" in out
        assert "dev-require tcltest 2.5" in out

    def test_verify(self, tmp_path, capsys, monkeypatch):
        self._setup(tmp_path, capsys, monkeypatch)
        code, out, _ = _run(["pkg", "verify"], capsys)
        # Packages have no integrity hash yet so verify warns.
        assert code == 1
        assert "no integrity hash" in out


class TestPkgRemove:
    def test_removes_require(self, tmp_path, capsys, monkeypatch):
        monkeypatch.chdir(tmp_path)
        (tmp_path / "tclpkg.tcl").write_text(
            "package demo\nversion 1.0.0\nrequire json 1.0\nrequire http 2.0\n"
        )
        code, out, _ = _run(["pkg", "remove", "json"], capsys)
        assert code == 0
        text = (tmp_path / "tclpkg.tcl").read_text()
        assert "json" not in text
        assert "http" in text

    def test_remove_nonexistent_fails(self, tmp_path, capsys, monkeypatch):
        monkeypatch.chdir(tmp_path)
        (tmp_path / "tclpkg.tcl").write_text("package demo\nversion 1.0.0\n")
        code, _, err = _run(["pkg", "remove", "nonexistent"], capsys)
        assert code == 1
        assert "not found" in err


class TestPkgInfo:
    def test_shows_package_info(self, tmp_path, capsys, monkeypatch):
        monkeypatch.chdir(tmp_path)
        (tmp_path / "tclpkg.tcl").write_text("package demo\nversion 1.0.0\nrequire json 1.0\n")
        _run(["pkg", "install"], capsys)
        code, out, _ = _run(["pkg", "info", "json"], capsys)
        assert code == 0
        assert "json" in out
        assert "1.0" in out

    def test_info_unknown_package_fails(self, tmp_path, capsys, monkeypatch):
        monkeypatch.chdir(tmp_path)
        (tmp_path / "tclpkg.tcl").write_text("package demo\nversion 1.0.0\n")
        _run(["pkg", "install"], capsys)
        code, _, err = _run(["pkg", "info", "nonexistent"], capsys)
        assert code == 1
        assert "not found" in err
