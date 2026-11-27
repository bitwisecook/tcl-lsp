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

"""Unit tests for worktree-cost.py. They run no cargo and use no network."""

import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

_SPEC = importlib.util.spec_from_file_location(
    "worktree_cost", Path(__file__).with_name("worktree-cost.py")
)
assert _SPEC is not None and _SPEC.loader is not None
wc = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = wc
_SPEC.loader.exec_module(wc)

WS = "path+file:///w/rust/{0}#0.1.0"
REG = "registry+https://github.com/rust-lang/crates.io-index#{0}@1.0.0"


def artifact(pkg_id, fresh, opt=1, test=False, files=("x.rlib",), features=()):
    return json.dumps(
        {
            "reason": "compiler-artifact",
            "package_id": pkg_id,
            "fresh": fresh,
            "profile": {"opt_level": str(opt), "test": test},
            "filenames": list(files),
            "features": list(features),
        }
    )


class TempDir(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)

    def tearDown(self):
        self._tmp.cleanup()


class ParseTests(unittest.TestCase):
    def test_counts_split_by_package_profile_and_features(self):
        lines = [
            artifact(WS.format("tcl-a"), False, test=True, features=["x"]),
            artifact(WS.format("tcl-a"), True, features=[]),
            artifact(WS.format("tcl-b"), False, opt=0, files=["libb.rmeta"]),
            artifact(REG.format("serde"), False, opt=0),
            artifact(REG.format("libc"), True),
            json.dumps(
                {"reason": "build-script-executed", "package_id": REG.format("libc")}
            ),
            "running 3 tests",
            "{not json",
        ]
        r = wc.parse_messages(lines)
        self.assertEqual(
            r["units_compiled"], {"workspace": 2, "third_party": 1, "total": 3}
        )
        self.assertEqual(r["workspace_compiled"], ["tcl-a", "tcl-b"])
        self.assertEqual(
            r["units_by_profile"], {"opt0": 1, "opt0-check": 1, "opt1-test": 1}
        )
        self.assertEqual(r["build_scripts_run"], {})
        self.assertEqual(r["build_scripts_unknown"], {"libc": 1})
        self.assertEqual(wc.feature_variants([r]), {"tcl-a": 2, "tcl-b": 1})

    def test_replayed_build_scripts_are_not_counted_as_runs(self):
        with tempfile.TemporaryDirectory() as tmp:
            build = Path(tmp)
            messages = []
            for pkg, age in (("ran", 0), ("fresh", -3600), ("legacy", 0)):
                unit = build / pkg / "0123456789abcdef"
                (unit / "out").mkdir(parents=True)
                record = unit / ("output" if pkg == "legacy" else "run/stdout")
                record.parent.mkdir(parents=True, exist_ok=True)
                record.write_text("")
                stamp = 2_000_000_000 + age
                os.utime(record, (stamp, stamp))
                messages.append(
                    json.dumps(
                        {
                            "reason": "build-script-executed",
                            "package_id": REG.format(pkg),
                            "out_dir": str(unit / "out"),
                        }
                    )
                )
            messages.append(
                json.dumps(
                    {
                        "reason": "build-script-executed",
                        "package_id": REG.format("norecord"),
                        "out_dir": str(build / "missing" / "out"),
                    }
                )
            )
            r = wc.parse_messages(messages, started=2_000_000_000 - 60)
        self.assertEqual(r["build_scripts_run"], {"legacy": 1, "ran": 1})
        self.assertEqual(r["build_scripts_replayed"], {"fresh": 1})
        self.assertEqual(r["build_scripts_unknown"], {"norecord": 1})

    def test_package_names(self):
        self.assertEqual(wc.package_name(WS.format("tcl-compiler")), "tcl-compiler")
        self.assertEqual(wc.package_name(REG.format("serde")), "serde")
        self.assertEqual(
            wc.package_name("path+file:///w/rust/tcl-vm#tcl-vm@0.1.0"), "tcl-vm"
        )


class GitRepo(TempDir):
    def setUp(self):
        super().setUp()
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        for k, v in (("user.email", "t@example.invalid"), ("user.name", "t")):
            subprocess.run(["git", "-C", str(self.root), "config", k, v], check=True)
        (self.root / "rust").mkdir()
        (self.root / "rust/lib.rs").write_text("pub fn f() {}\n")
        script = self.root / "scripts/dev/agent-build-env.sh"
        script.parent.mkdir(parents=True)
        script.write_text(
            "#!/bin/bash\n"
            f'echo "export CARGO_TARGET_DIR={self.root}/target"\n'
            "echo 'unset CARGO_INCREMENTAL'\n"
            "echo 'export CARGO_PROFILE_TEST_OPT_LEVEL=1'\n"
        )
        subprocess.run(["git", "-C", str(self.root), "add", "."], check=True)
        subprocess.run(
            ["git", "-C", str(self.root), "commit", "-qm", "init"], check=True
        )
        self.edit = wc.Edit("k", "rust/lib.rs", "c", "HEAD")

    def good_env(self):
        return {
            "CARGO_TARGET_DIR": f"{self.root}/target",
            "CARGO_PROFILE_TEST_OPT_LEVEL": "1",
        }

    def preflight(self, environ, edits=None):
        wc.preflight(self.root, edits or [self.edit], environ, 0, check_busy=False)


class GuardTests(GitRepo):
    def test_good_environment_passes(self):
        self.preflight(self.good_env())

    def test_missing_opt_level_refuses(self):
        env = self.good_env()
        del env["CARGO_PROFILE_TEST_OPT_LEVEL"]
        with self.assertRaisesRegex(wc.CostError, "CARGO_PROFILE_TEST_OPT_LEVEL"):
            self.preflight(env)

    def test_different_opt_level_refuses(self):
        env = {**self.good_env(), "CARGO_PROFILE_TEST_OPT_LEVEL": "0"}
        with self.assertRaisesRegex(wc.CostError, "CARGO_PROFILE_TEST_OPT_LEVEL"):
            self.preflight(env)

    def test_printf_quoted_values_are_unescaped(self):
        env = wc.parse_env_print(
            "export CARGO_ALIAS_XTASK=run\\ --profile\\ test\\ --package\\ xtask\\ --\n"
            "export CARGO_TARGET_DIR='/a b/target'\n"
        )
        self.assertEqual(
            env["CARGO_ALIAS_XTASK"], "run --profile test --package xtask --"
        )
        self.assertEqual(env["CARGO_TARGET_DIR"], "/a b/target")

    def test_shared_target_refuses(self):
        env = {**self.good_env(), "CARGO_TARGET_DIR": "/shared/target"}
        with self.assertRaisesRegex(wc.CostError, "worktree's own"):
            self.preflight(env)

    def test_set_incremental_refuses(self):
        env = {**self.good_env(), "CARGO_INCREMENTAL": "0"}
        with self.assertRaisesRegex(wc.CostError, "CARGO_INCREMENTAL"):
            self.preflight(env)

    def test_dirty_build_input_refuses(self):
        (self.root / "rust/new.rs").write_text("\n")
        with self.assertRaisesRegex(wc.CostError, "not clean"):
            self.preflight(self.good_env())

    def test_documentation_edit_does_not_refuse(self):
        (self.root / "notes.md").write_text("\n")
        self.preflight(self.good_env())

    def test_missing_edit_file_fails_before_any_build(self):
        missing = wc.Edit("gone", "rust/gone.rs", "c", "HEAD")
        with mock.patch.object(wc, "run_measured") as build:
            with self.assertRaisesRegex(wc.CostError, "rust/gone.rs does not exist"):
                self.preflight(self.good_env(), [missing])
            build.assert_not_called()

    def test_every_table_row_exists_in_this_checkout(self):
        for edit in wc.EDITS:
            self.assertTrue((wc.REPO / edit.file).is_file(), edit.file)

    def test_main_reports_the_reason_with_a_nonzero_exit(self):
        with mock.patch.object(wc, "preflight", side_effect=wc.CostError("tree dirty")):
            self.assertEqual(wc.main(["run", "--edits", "lexer", "--in-place"]), 2)


class EditTests(GitRepo):
    def status(self):
        return subprocess.run(
            ["git", "-C", str(self.root), "status", "--porcelain"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout

    def test_edit_appends_one_line_and_restore_is_clean(self):
        before = (self.root / "rust/lib.rs").read_text().splitlines()
        wc.apply_edit(self.root, self.edit, 7)
        after = (self.root / "rust/lib.rs").read_text().splitlines()
        self.assertEqual(after, [*before, "const _: u32 = 7;"])
        wc.restore_edit(self.root, self.edit)
        self.assertEqual(self.status(), "")

    def test_restore_after_exception_mid_run(self):
        args = mock.Mock(no_test_run=True, jobs=None, commands=None, phases=None)
        with mock.patch.object(
            wc, "run_measured", side_effect=[{}, {}, RuntimeError("boom")]
        ):
            with self.assertRaises(RuntimeError):
                wc.measure_edit(self.root, self.edit, 1, args, {}, self.root / "target")
        self.assertEqual(self.status(), "")


class SizerTests(TempDir):
    def test_hard_links_count_once(self):
        a = self.root / "debug/deps/liba.rlib"
        a.parent.mkdir(parents=True)
        a.write_bytes(b"x" * 8192)
        os.link(a, self.root / "debug/deps/liba-copy.rlib")
        size = a.stat().st_blocks * 512
        split = wc.allocated_bytes(self.root)
        self.assertEqual(split["total"], size)
        self.assertEqual(split["deps"], size)

    def test_variants_on_disk(self):
        for pkg, hashes in (("tcl-a", 3), ("tcl-b", 1)):
            for i in range(hashes):
                (self.root / f"debug/build/{pkg}/{i:016x}").mkdir(parents=True)
        (self.root / "debug/build/tcl-a/stray-file").write_text("")
        self.assertEqual(
            wc.variants_on_disk(self.root, ["tcl-a", "tcl-b", "tcl-c"]),
            {"tcl-a": 3, "tcl-b": 1},
        )


class SamplerTests(TempDir):
    def proc(self, pid, rss, children=(), exe=""):
        d = self.root / str(pid)
        (d / "task" / str(pid)).mkdir(parents=True, exist_ok=True)
        (d / "status").write_text(f"Name:\tx\nVmRSS:\t{rss} kB\n")
        (d / "task" / str(pid) / "children").write_text(" ".join(map(str, children)))
        (d / "cmdline").write_bytes(exe.encode() + b"\0")

    def test_summed_peak_over_three_samples(self):
        peaks = []
        for parent, c1, c2 in ((100, 200, 300), (100, 900, 50), (100, 400, 400)):
            self.proc(1, parent, children=(2, 3))
            self.proc(2, c1)
            self.proc(3, c2, exe="/t/debug/deps/tcl_a-123")
            peaks.append(wc.sample_tree(1, self.root))
        self.assertEqual(max(peaks), 1050)
        harness = wc.harness_matcher(Path("/t"), "tcl-a")
        self.assertEqual(wc.sample_tree(1, self.root, harness), 400)

    def test_process_exiting_mid_sample_is_skipped(self):
        self.proc(1, 100, children=(2, 3))
        self.proc(2, 50)
        self.assertEqual(wc.sample_tree(1, self.root), 150)

    def test_scan_by_parent_id_without_children_file(self):
        for pid, ppid, rss in ((1, 0, 10), (2, 1, 20), (3, 2, 30), (4, 9, 99)):
            d = self.root / str(pid)
            d.mkdir()
            (d / "status").write_text(f"VmRSS:\t{rss} kB\n")
            (d / "stat").write_text(f"{pid} (a b) c) S {ppid} 0 0\n")
        self.assertEqual(wc.sample_tree(1, self.root), 60)


def doc(wall, units):
    cmd = {
        "units_compiled": {"workspace": units, "third_party": 0, "total": units},
        "wall_s": wall,
        "peak_rss_tree_mb": 1000,
        "peak_rss_single_mb": 500,
        "target_growth_bytes": 2 << 20,
    }
    return {"edits": {"lexer": {"crate": "tcl-syntax", "after": {"test_build": cmd}}}}


class ReportTests(unittest.TestCase):
    def test_table_rows(self):
        rows = wc.table(doc(12.5, 3)).splitlines()
        self.assertEqual(len(rows), 3)
        self.assertEqual(
            rows[2],
            "| lexer | `tcl-syntax` | after | test_build | 3 | 3 | 12.5 | 1000 | 500 | 2 | – |",
        )

    def test_compare_signed_deltas(self):
        rows = wc.compare(doc(12.5, 9), doc(10.0, 1)).splitlines()
        self.assertEqual(
            rows[2],
            "| lexer | `tcl-syntax` | after | test_build | -8 | -8 | -2.5 | +0 | +0 | +0 | – |",
        )


class RustAnalyzerTests(unittest.TestCase):
    def test_absent_binary_records_null_and_reason(self):
        with mock.patch.object(wc.shutil, "which", return_value=None):
            r = wc.measure_rust_analyzer(Path("."))
        self.assertIsNone(r["ra_rss_mb"])
        self.assertIn("not on PATH", r["reason"])

    def test_binary_that_does_not_run_records_null(self):
        r = wc.measure_rust_analyzer(Path("."), binary="false")
        self.assertIsNone(r["ra_rss_mb"])
        self.assertIn("does not run", r["reason"])


if __name__ == "__main__":
    unittest.main()
