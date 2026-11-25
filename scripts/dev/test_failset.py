# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Tests for failset.py."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent))
import failset

LOG = """\
running 3 tests
test a::passes ... ok
test a::breaks ... FAILED
test b::also_breaks ... FAILED

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
"""


def git(repo: Path, *args: str) -> None:
    subprocess.run(["git", *args], cwd=repo, check=True, capture_output=True)


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    git(tmp_path, "init", "-q")
    git(tmp_path, "config", "user.email", "t@example.invalid")
    git(tmp_path, "config", "user.name", "t")
    (tmp_path / "rust").mkdir()
    (tmp_path / "rust" / "lib.rs").write_text("fn a() {}\n")
    (tmp_path / "Cargo.toml").write_text("[workspace]\n")
    (tmp_path / "Cargo.lock").write_text("")
    (tmp_path / "README").write_text("docs\n")
    git(tmp_path, "add", "-A")
    git(tmp_path, "commit", "-qm", "init")
    return tmp_path


def test_parse_collects_failures_and_summary() -> None:
    outcome = failset.parse(LOG)
    assert outcome.failures == {"a::breaks", "b::also_breaks"}
    assert outcome.summary.startswith("test result: FAILED. 1 passed")


@pytest.mark.parametrize(
    "log",
    [
        "error[E0425]: cannot find value\n",
        LOG
        + "error: test failed, to rerun pass `--lib`\nprocess didn't exit successfully: `x` (signal: 6, SIGABRT)\n",
        "thread 'big' has overflowed its stack\n" + LOG,
    ],
)
def test_parse_refuses_builds_and_aborts(log: str) -> None:
    with pytest.raises(failset.FailsetError):
        failset.parse(log)


def test_clean_tree_key_equals_head_key(repo: Path) -> None:
    assert failset.source_key(repo) == failset.source_key(repo, "HEAD")


def test_key_tracks_source_edits_not_docs(repo: Path) -> None:
    head = failset.source_key(repo, "HEAD")
    (repo / "README").write_text("changed\n")
    assert failset.source_key(repo) == head
    (repo / "rust" / "new.rs").write_text("fn b() {}\n")
    assert failset.source_key(repo) != head
    assert not failset.working_tree_matches_head(repo)


def test_committed_after_run_becomes_next_baseline(
    repo: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setenv("CARGO_TARGET_DIR", str(repo / "target"))
    args = ["-p", "x", "--lib"]
    (repo / "rust" / "lib.rs").write_text("fn a() { b() }\n")
    after = failset.entry(repo, failset.source_key(repo), args)
    failset.store(after, failset.parse(LOG))
    git(repo, "commit", "-qam", "edit")
    assert failset.load(
        failset.entry(repo, failset.source_key(repo, "HEAD"), args)
    ) == failset.parse(LOG)


def test_report_flags_only_new_failures(capsys: pytest.CaptureFixture[str]) -> None:
    assert failset.report(frozenset({"a", "b"}), frozenset({"b"})) == 0
    assert failset.report(frozenset({"b"}), frozenset({"b", "c"})) == 1
    assert "  c\n" in capsys.readouterr().out
