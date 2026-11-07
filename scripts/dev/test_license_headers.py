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

"""Negative and positive controls for check-license-headers.py."""

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

_SPEC = importlib.util.spec_from_file_location(
    "check_license_headers", Path(__file__).with_name("check-license-headers.py")
)
clh = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = clh
_SPEC.loader.exec_module(clh)

FULL = clh.NOTICE.format(year="2026")


class FixTests(unittest.TestCase):
    def test_full_notice_passes_and_is_unchanged(self):
        text = FULL + "\n//! Doc.\nfn f() {}\n"
        self.assertTrue(clh.check(text))
        self.assertEqual(clh.fix(text), text)

    def test_truncated_notice_is_completed(self):
        text = (
            "// tcl-lsp — a language server and toolchain for Tcl\n"
            "// Copyright (C) 2026 James Deucker (bitwisecook)\n"
            "// SPDX-License-Identifier: AGPL-3.0-or-later\n\n//! Doc.\n"
        )
        self.assertFalse(clh.check(text))
        self.assertEqual(clh.fix(text), FULL + "\n//! Doc.\n")

    def test_spdx_only_notice_is_replaced(self):
        text = "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! Doc.\n"
        self.assertEqual(clh.fix(text), FULL + "\n//! Doc.\n")

    def test_missing_notice_is_prepended_keeping_a_plain_comment(self):
        text = "// Compile every grammar.\nfn main() {}\n"
        self.assertEqual(clh.fix(text), FULL + "\n" + text)

    def test_year_is_kept(self):
        text = "// Copyright (C) 2024 James Deucker (bitwisecook)\nfn f() {}\n"
        fixed = clh.fix(text)
        self.assertTrue(fixed.startswith(clh.NOTICE.format(year="2024")))
        self.assertTrue(clh.check(fixed))

    def test_generated_and_foreign_licences_are_exempt(self):
        self.assertTrue(
            clh.exempt_by_content("// @generated — do not edit.\nfn f() {}\n")
        )
        self.assertTrue(
            clh.exempt_by_content("// SPDX-License-Identifier: MIT\nfn f() {}\n")
        )
        self.assertFalse(
            clh.exempt_by_content("// SPDX-License-Identifier: AGPL-3.0-or-later\n")
        )


class RepoTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)

    def tearDown(self):
        self._tmp.cleanup()

    def add(self, rel, text):
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        subprocess.run(["git", "add", rel], cwd=self.root, check=True)

    def run_gate(self, *extra):
        return clh.main(["--root", str(self.root), *extra])

    def test_gate_fails_then_fix_makes_it_pass(self):
        self.add("rust/a/src/lib.rs", "fn f() {}\n")
        self.add("rust/a/vendor/x.rs", "fn v() {}\n")
        self.add("rust/a/tests/data/case.rs", "const C: u8 = 0;\n")
        self.add("runtime/rust/src/ok.rs", FULL + "\nfn ok() {}\n")
        self.assertEqual(self.run_gate(), 1)
        self.assertEqual(self.run_gate("--fix"), 0)
        self.assertEqual(self.run_gate(), 0)
        self.assertEqual((self.root / "rust/a/vendor/x.rs").read_text(), "fn v() {}\n")

    def test_stale_exempt_entry_fails(self):
        self.add("rust/a/src/lib.rs", FULL)
        self.add(str(clh.EXEMPT_LIST), "rust/gone.rs  # removed\n")
        self.assertEqual(self.run_gate(), 1)


if __name__ == "__main__":
    unittest.main()
