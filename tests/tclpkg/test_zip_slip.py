# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Tests for tclpkg.fetchers — zip security protections."""

from __future__ import annotations

import zipfile
from pathlib import Path

import pytest

from tooling.tclpkg.fetchers import FetchError, _safe_extract_zip


def _make_zip(path: Path, members: dict[str, str]) -> None:
    """Create a zip file with the given member names and contents."""
    with zipfile.ZipFile(path, "w") as zf:
        for name, content in members.items():
            zf.writestr(name, content)


def _make_zip_with_symlink(path: Path, link_name: str, link_target: str) -> None:
    """Create a zip file containing a symlink entry."""
    with zipfile.ZipFile(path, "w") as zf:
        info = zipfile.ZipInfo(link_name)
        # Set Unix symlink mode in external_attr (high 16 bits).
        info.external_attr = 0o120777 << 16
        # Symlink target is stored as the file content.
        zf.writestr(info, link_target)


class TestZipSlipProtection:
    """Verify that malicious zip members are rejected."""

    def test_normal_zip_extracts(self, tmp_path) -> None:
        zip_path = tmp_path / "good.zip"
        _make_zip(zip_path, {"hello.txt": "hello\n", "sub/nested.txt": "nested\n"})
        dest = tmp_path / "out"
        dest.mkdir()
        _safe_extract_zip(zip_path, dest)
        assert (dest / "hello.txt").read_text() == "hello\n"
        assert (dest / "sub" / "nested.txt").read_text() == "nested\n"

    def test_dotdot_member_rejected(self, tmp_path) -> None:
        """A zip member with ../ in the path must be rejected."""
        zip_path = tmp_path / "evil.zip"
        _make_zip(zip_path, {"../escape.txt": "pwned\n"})
        dest = tmp_path / "out"
        dest.mkdir()
        with pytest.raises(FetchError, match="escapes target directory"):
            _safe_extract_zip(zip_path, dest)

    def test_absolute_path_rejected(self, tmp_path) -> None:
        """A zip member with an absolute path must be rejected."""
        zip_path = tmp_path / "abs.zip"
        _make_zip(zip_path, {"/etc/passwd": "root:x:0:0\n"})
        dest = tmp_path / "out"
        dest.mkdir()
        with pytest.raises(FetchError, match="absolute path"):
            _safe_extract_zip(zip_path, dest)

    def test_nested_dotdot_rejected(self, tmp_path) -> None:
        """A zip member like sub/../../escape must be rejected."""
        zip_path = tmp_path / "nested_evil.zip"
        _make_zip(zip_path, {"sub/../../escape.txt": "pwned\n"})
        dest = tmp_path / "out"
        dest.mkdir()
        with pytest.raises(FetchError, match="escapes target directory"):
            _safe_extract_zip(zip_path, dest)

    def test_sibling_directory_bypass_rejected(self, tmp_path) -> None:
        """A path like ../staging2/evil must not match a prefix check.

        This is the specific bypass the PR reviewer flagged: a naive
        str.startswith('/tmp/staging') would match '/tmp/staging2/evil'.
        The relative_to fix handles this correctly.
        """
        zip_path = tmp_path / "sibling.zip"
        _make_zip(zip_path, {"../staging2/evil.txt": "pwned\n"})
        dest = tmp_path / "staging"
        dest.mkdir()
        with pytest.raises(FetchError, match="escapes target directory"):
            _safe_extract_zip(zip_path, dest)


class TestZipSymlinkProtection:
    """Verify that symlinks inside zips are skipped."""

    def test_symlink_member_skipped(self, tmp_path) -> None:
        zip_path = tmp_path / "symlink.zip"
        _make_zip_with_symlink(zip_path, "evil_link", "/etc/passwd")
        dest = tmp_path / "out"
        dest.mkdir()
        _safe_extract_zip(zip_path, dest)
        # The symlink should not have been created.
        assert not (dest / "evil_link").exists()

    def test_mixed_files_and_symlinks(self, tmp_path) -> None:
        """Regular files are extracted; symlinks are skipped."""
        zip_path = tmp_path / "mixed.zip"
        with zipfile.ZipFile(zip_path, "w") as zf:
            zf.writestr("good.txt", "hello\n")
            info = zipfile.ZipInfo("bad_link")
            info.external_attr = 0o120777 << 16
            zf.writestr(info, "../../../etc/shadow")
        dest = tmp_path / "out"
        dest.mkdir()
        _safe_extract_zip(zip_path, dest)
        assert (dest / "good.txt").read_text() == "hello\n"
        assert not (dest / "bad_link").exists()


class TestZipBombProtection:
    """Verify that oversized archives are rejected."""

    def test_small_zip_ok(self, tmp_path) -> None:
        zip_path = tmp_path / "small.zip"
        _make_zip(zip_path, {"data.txt": "x" * 1000})
        dest = tmp_path / "out"
        dest.mkdir()
        _safe_extract_zip(zip_path, dest)
        assert (dest / "data.txt").is_file()

    def test_oversized_zip_rejected(self, tmp_path, monkeypatch) -> None:
        """A zip that decompresses beyond the limit is rejected."""
        import tooling.tclpkg.fetchers as mod

        # Temporarily lower the limit to make the test fast.
        monkeypatch.setattr(mod, "_MAX_EXTRACT_BYTES", 100)
        zip_path = tmp_path / "bomb.zip"
        _make_zip(zip_path, {"big.txt": "x" * 200})
        dest = tmp_path / "out"
        dest.mkdir()
        with pytest.raises(FetchError, match="zip bomb"):
            _safe_extract_zip(zip_path, dest)
