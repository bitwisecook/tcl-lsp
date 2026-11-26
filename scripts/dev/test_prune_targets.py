# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Tests for prune-targets.py on a fake target tree."""

from __future__ import annotations

import fcntl
import importlib.util
import io
import json
import os
import sys
import tempfile
import time
import unittest
from contextlib import redirect_stdout
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "prune_targets", Path(__file__).with_name("prune-targets.py")
)
prune = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = prune
SPEC.loader.exec_module(prune)

DAY = 86400


def make_unit(target: Path, package: str, digest: str, *, age_days: float = 0.0, rustc: int = 1,
              features: str = "[]", kind: str = "lib-x.json", payload: bool = True) -> Path:
    unit = target / "debug" / "build" / package / digest
    (unit / "fingerprint").mkdir(parents=True)
    (unit / "out").mkdir()
    fingerprint = {"rustc": rustc, "features": features, "target": 7, "profile": 9,
                   "path": 3, "config": 0, "compile_kind": 0}
    files = [unit / "fingerprint" / kind]
    files[0].write_text(json.dumps(fingerprint), encoding="utf-8")
    if payload:
        files.append(unit / "out" / f"lib{package}-{digest}.rlib")
        files[-1].write_bytes(b"x" * 1024)
    stamp = time.time() - age_days * DAY
    for path in files:
        os.utime(path, (stamp, stamp))
    return unit


class PruneTargetsTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.target = Path(self.tmp.name) / "target"

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def run_prune(self, *args: str) -> str:
        out = io.StringIO()
        with redirect_stdout(out):
            self.assertEqual(prune.main(["--target", str(self.target), *args]), 0)
        return out.getvalue()

    def test_keeps_the_newest_units_per_key_and_deletes_superseded_ones(self) -> None:
        units = [make_unit(self.target, "a", f"{n:016x}", age_days=n * 0.1) for n in range(5)]
        self.run_prune("--keep", "3")
        self.assertEqual([u.exists() for u in units], [True, True, True, False, False])

    def test_feature_variants_are_separate_keys(self) -> None:
        plain = make_unit(self.target, "a", "1" * 16, age_days=0.5)
        featured = make_unit(self.target, "a", "2" * 16, features='["test-support"]', age_days=0.4)
        newer = make_unit(self.target, "a", "3" * 16, age_days=0.1)
        self.run_prune("--keep", "1")
        self.assertFalse(plain.exists())
        self.assertTrue(featured.exists())
        self.assertTrue(newer.exists())

    def test_units_of_an_older_rustc_old_units_and_empty_units_go(self) -> None:
        current = make_unit(self.target, "a", "1" * 16, rustc=2)
        old_rustc = make_unit(self.target, "b", "2" * 16, rustc=1, age_days=0.5)
        old = make_unit(self.target, "c", "3" * 16, rustc=2, age_days=4)
        empty = make_unit(self.target, "d", "4" * 16, rustc=2, payload=False)
        for path in (empty / "fingerprint").iterdir():
            path.unlink()
        self.run_prune()
        self.assertTrue(current.exists())
        self.assertFalse(old_rustc.exists())
        self.assertFalse(old.exists())
        self.assertFalse(empty.exists())

    def test_old_incremental_directories_go_and_recent_ones_stay(self) -> None:
        make_unit(self.target, "a", "1" * 16)
        incremental = self.target / "debug" / "incremental"
        for name, age in (("a-old", 5), ("a-new", 0)):
            (incremental / name).mkdir(parents=True)
            file = incremental / name / "dep-graph.bin"
            file.write_bytes(b"x")
            stamp = time.time() - age * DAY
            os.utime(file, (stamp, stamp))
        self.run_prune()
        self.assertFalse((incremental / "a-old").exists())
        self.assertTrue((incremental / "a-new").exists())

    def test_dry_run_deletes_nothing(self) -> None:
        old = make_unit(self.target, "a", "1" * 16, age_days=10)
        self.assertIn("would delete", self.run_prune("--dry-run"))
        self.assertTrue(old.exists())

    def test_a_held_cargo_lock_skips_the_profile(self) -> None:
        old = make_unit(self.target, "a", "1" * 16, age_days=10)
        lock = self.target / "debug" / ".cargo-lock"
        lock.touch()
        with open(lock, "a+b") as handle:
            fcntl.flock(handle, fcntl.LOCK_EX)
            self.run_prune()
        self.assertTrue(old.exists())
        self.run_prune()
        self.assertFalse(old.exists())

    def test_removed_worktree_loses_its_whole_target(self) -> None:
        removed = Path(self.tmp.name) / "gone"
        make_unit(removed / "target", "a", "1" * 16)
        with redirect_stdout(io.StringIO()):
            self.assertEqual(prune.main(["--removed", str(removed)]), 0)
        self.assertFalse((removed / "target").exists())

    def test_a_live_worktree_is_refused(self) -> None:
        live = Path(self.tmp.name) / "live"
        (live / "target").mkdir(parents=True)
        (live / ".git").write_text("gitdir: elsewhere\n", encoding="utf-8")
        with self.assertRaises(SystemExit), redirect_stdout(io.StringIO()):
            prune.main(["--removed", str(live)])
        self.assertTrue((live / "target").exists())


if __name__ == "__main__":
    unittest.main()
