#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Squashed-branch cutting against throwaway repositories."""

from __future__ import annotations

import importlib.util
import io
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("cut", HERE / "cut-squashed-branch.py")
assert spec and spec.loader
cut = importlib.util.module_from_spec(spec)
sys.modules["cut"] = cut
spec.loader.exec_module(cut)


def sh(cwd: Path, *args: str) -> str:
    return subprocess.run(
        ["git", *args], cwd=cwd, check=True, capture_output=True, text=True
    ).stdout.strip()


class CutSquashedBranchTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        base = Path(self.temp.name)
        self.remote = base / "remote.git"
        self.root = base / "clone"
        sh(base, "init", "-q", "--bare", "-b", "rust", str(self.remote))
        sh(base, "init", "-q", "-b", "rust", str(self.root))
        for key, value in (("user.name", "t"), ("user.email", "t@example.com")):
            sh(self.root, "config", key, value)
        sh(self.root, "remote", "add", "origin", str(self.remote))
        (self.root / "scripts/dev").mkdir(parents=True)
        self.commit("README.md", "base\n", "base")
        sh(self.root, "checkout", "-q", "-b", "work")
        self.commit("docs/work/name-handling/handoff/dump.bin", "x" * 1000, "dump")
        sh(self.root, "rm", "-q", "-r", "docs/work/name-handling/handoff")
        self.commit("src.tcl", "puts ok\n", "work")
        sh(self.root, "push", "-q", "origin", "rust", "work")
        sh(self.root, "fetch", "-q", "origin")

    def tearDown(self) -> None:
        self.temp.cleanup()

    def commit(self, path: str, text: str, message: str) -> None:
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)
        sh(self.root, "add", "-A")
        sh(self.root, "commit", "-q", "-m", message)

    def run_main(self, *args: str) -> tuple[int, str, str]:
        out, err = io.StringIO(), io.StringIO()
        with redirect_stdout(out), redirect_stderr(err):
            code = cut.main(["--root", str(self.root), *args])
        return code, out.getvalue(), err.getvalue()

    def test_squash_has_source_tree_on_base_tip_and_pushes(self) -> None:
        code, out, err = self.run_main("cut", "--branch", "slim", "--push")
        self.assertEqual(code, 0, err)
        self.assertIn("squashed 2 commits", out)
        self.assertEqual(
            sh(self.root, "rev-parse", "slim^{tree}"),
            sh(self.root, "rev-parse", "origin/work^{tree}"),
        )
        self.assertEqual(
            sh(self.root, "rev-parse", "slim^"),
            sh(self.root, "rev-parse", "origin/rust"),
        )
        self.assertEqual(
            sh(self.remote, "rev-parse", "slim"), sh(self.root, "rev-parse", "slim")
        )
        history = sh(self.root, "log", "--format=%s", "slim", "--", "docs/work")
        self.assertEqual(history, "")

    def test_uses_merge_base_when_base_moved_on(self) -> None:
        sh(self.root, "checkout", "-q", "rust")
        self.commit("CHANGELOG.md", "release\n", "release")
        sh(self.root, "push", "-q", "origin", "rust")
        sh(self.root, "fetch", "-q", "origin")
        sh(self.root, "checkout", "-q", "work")
        code, out, err = self.run_main("cut", "--branch", "slim")
        self.assertEqual(code, 0, err)
        self.assertIn("merge-base", out)
        merge_base = sh(self.root, "merge-base", "origin/work", "origin/rust")
        self.assertEqual(sh(self.root, "rev-parse", "slim^"), merge_base)

    def test_deepens_a_shallow_clone_to_find_the_base(self) -> None:
        shallow = Path(self.temp.name) / "shallow"
        sh(
            Path(self.temp.name),
            "clone",
            "-q",
            "--depth=1",
            "--no-single-branch",
            f"file://{self.remote}",
            str(shallow),
        )
        self.root = shallow
        code, _, err = self.run_main("cut", "--branch", "slim")
        self.assertEqual(code, 1)
        self.assertIn("no merge-base", err)
        code, out, err = self.run_main("cut", "--branch", "slim", "--deepen", "2")
        self.assertEqual(code, 0, err)
        self.assertIn("base tip", out)
        self.assertEqual(
            sh(shallow, "rev-parse", "slim^"), sh(shallow, "rev-parse", "origin/rust")
        )

    def test_refuses_existing_branch_and_dirty_tree(self) -> None:
        code, _, err = self.run_main("cut", "--branch", "work")
        self.assertEqual(code, 1)
        self.assertIn("already exists", err)
        (self.root / "src.tcl").write_text("changed\n")
        code, _, err = self.run_main("cut", "--branch", "fresh")
        self.assertEqual(code, 1)
        self.assertIn("uncommitted", err)

    def test_refuses_tree_over_budget(self) -> None:
        big = self.root / "big.bin"
        with big.open("wb") as handle:
            handle.truncate(cut._size_budget().FILE_LIMIT + 1)
        sh(self.root, "add", "big.bin")
        sh(self.root, "commit", "-q", "-m", "big")
        code, _, err = self.run_main("cut", "--source", "HEAD", "--branch", "fat")
        self.assertEqual(code, 1)
        self.assertIn("big.bin", err)

    def test_audit_reports_branches_with_bloat_history(self) -> None:
        code, out, _ = self.run_main("audit", "--strict")
        self.assertEqual(code, 1)
        self.assertIn("bloat  origin/work", out)
        self.assertNotIn("origin/rust", out.replace("carry", ""))
        self.run_main("cut", "--branch", "slim", "--push")
        sh(self.root, "fetch", "-q", "origin")
        _, out, _ = self.run_main("audit")
        self.assertNotIn("origin/slim", out)


if __name__ == "__main__":
    unittest.main()
