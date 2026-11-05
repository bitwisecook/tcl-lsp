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

"""Negative and positive controls for check-tracked-size.py."""

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

_SPEC = importlib.util.spec_from_file_location(
    "check_tracked_size", Path(__file__).with_name("check-tracked-size.py")
)
cts = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = cts
_SPEC.loader.exec_module(cts)


class TrackedSizeTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)

    def tearDown(self):
        self._tmp.cleanup()

    def add(self, rel, size):
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(b"x" * size)
        subprocess.run(["git", "-C", str(self.root), "add", rel], check=True)

    def run_check(self, *extra):
        return cts.main(["--root", str(self.root), *extra])

    def test_small_tree_passes(self):
        self.add("a.txt", 10)
        self.assertEqual(self.run_check(), 0)

    def test_oversized_file_fails(self):
        self.add("big.bin", cts.FILE_LIMIT + 1)
        self.assertEqual(self.run_check(), 1)

    def test_allowlisted_file_passes_and_stale_entry_fails(self):
        self.add("big.bin", cts.FILE_LIMIT + 1)
        self.add(str(cts.ALLOWLIST), 0)
        (self.root / cts.ALLOWLIST).write_text("big.bin  # read by a test\n")
        self.assertEqual(self.run_check(), 0)
        (self.root / cts.ALLOWLIST).write_text("big.bin\ngone.bin\n")
        self.assertEqual(self.run_check(), 1)

    def test_tree_budget_fails(self):
        self.add("a.bin", 2 * cts.MIB)
        self.assertEqual(self.run_check("--tree-limit-mib", "1"), 1)

    def test_untracked_files_do_not_count(self):
        (self.root / "loose.bin").write_bytes(b"x" * (cts.FILE_LIMIT + 1))
        self.assertEqual(self.run_check(), 0)


if __name__ == "__main__":
    unittest.main()
