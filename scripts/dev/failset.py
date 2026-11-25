#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Failing-test sets for `cargo test`, cached by source state.

Some suites carry known failures, so "did my change break anything" is a
set comparison: the failures after the change minus those at HEAD. A run is
cached under a key made from the rust/ tree, Cargo.toml and Cargo.lock as
they stand in the working tree (uncommitted edits included) plus the cargo
arguments. An unchanged source state is never run twice, and the after-run
of one change is the next change's baseline once it is committed.

    failset.py baseline -- -p tcl-compiler --lib analyser::   # before editing
    failset.py check    -- -p tcl-compiler --lib analyser::   # after editing
    failset.py run      -- <args>     # print the failing set for the tree
    failset.py compare BEFORE AFTER   # diff two saved sets
"""

from __future__ import annotations

import argparse
import hashlib
import os
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

SOURCE_PATHS = ("rust", "Cargo.toml", "Cargo.lock")
FAILED = re.compile(r"^test (\S.*?) \.\.\. FAILED$", re.MULTILINE)
RESULT = re.compile(r"^test result: .*$", re.MULTILINE)
ABORTED = re.compile(
    r"process didn't exit successfully: .*\(signal: |has overflowed its stack"
)


class FailsetError(RuntimeError):
    pass


@dataclass(frozen=True)
class Outcome:
    failures: frozenset[str]
    summary: str


def git(repo: Path, *args: str, env: dict[str, str] | None = None) -> str:
    return subprocess.run(
        ["git", *args], cwd=repo, env=env, check=True, capture_output=True, text=True
    ).stdout.strip()


def source_key(repo: Path, rev: str | None = None) -> str:
    """Hash of the source paths at `rev`, or in the working tree when None."""
    if rev is not None:
        ids = [git(repo, "rev-parse", f"{rev}:{p}") for p in SOURCE_PATHS]
    else:
        with tempfile.TemporaryDirectory() as tmp:
            index = Path(tmp) / "index"
            shutil.copyfile(
                git(repo, "rev-parse", "--path-format=absolute", "--git-path", "index"),
                index,
            )
            env = {**os.environ, "GIT_INDEX_FILE": str(index)}
            git(repo, "add", "-A", "--", *SOURCE_PATHS, env=env)
            tree = git(repo, "write-tree", env=env)
            ids = [git(repo, "rev-parse", f"{tree}:{p}") for p in SOURCE_PATHS]
    return hashlib.sha1(" ".join(ids).encode()).hexdigest()[:16]


def args_key(cargo_args: list[str]) -> str:
    return hashlib.sha1("\0".join(cargo_args).encode()).hexdigest()[:12]


def parse(log: str) -> Outcome:
    results = RESULT.findall(log)
    if ABORTED.search(log) or not results:
        raise FailsetError("the run aborted or never reached a test summary")
    return Outcome(frozenset(FAILED.findall(log)), results[-1])


def cache_dir(repo: Path) -> Path:
    target = Path(os.environ.get("CARGO_TARGET_DIR") or repo / "target")
    return target / "failsets"


def entry(repo: Path, key: str, cargo_args: list[str]) -> Path:
    return cache_dir(repo) / f"{key}-{args_key(cargo_args)}"


def load(path: Path) -> Outcome | None:
    fail, summary = path.with_suffix(".fail"), path.with_suffix(".summary")
    if not fail.exists() or not summary.exists():
        return None
    names = frozenset(line for line in fail.read_text().splitlines() if line)
    return Outcome(names, summary.read_text().strip())


def store(path: Path, outcome: Outcome) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.with_suffix(".fail").write_text(
        "".join(f"{n}\n" for n in sorted(outcome.failures))
    )
    path.with_suffix(".summary").write_text(outcome.summary + "\n")


def run_cargo(repo: Path, cargo_args: list[str], log: Path) -> Outcome:
    log.parent.mkdir(parents=True, exist_ok=True)
    with log.open("w") as out:
        subprocess.run(
            ["cargo", "test", *cargo_args],
            cwd=repo,
            check=False,
            stdout=out,
            stderr=subprocess.STDOUT,
        )
    try:
        return parse(log.read_text(errors="replace"))
    except FailsetError as err:
        raise FailsetError(f"{err}; see {log}") from None


def failing_set(repo: Path, cargo_args: list[str]) -> tuple[Outcome, bool]:
    path = entry(repo, source_key(repo), cargo_args)
    cached = load(path)
    if cached is not None:
        return cached, True
    outcome = run_cargo(repo, cargo_args, path.with_suffix(".log"))
    store(path, outcome)
    return outcome, False


def working_tree_matches_head(repo: Path) -> bool:
    return not git(repo, "status", "--porcelain", "--", *SOURCE_PATHS)


def describe(outcome: Outcome, cached: bool) -> str:
    return f"{'cached' if cached else 'ran'}: {outcome.summary}"


def report(before: frozenset[str], after: frozenset[str]) -> int:
    new, fixed = sorted(after - before), sorted(before - after)
    print(f"NEW failures ({len(new)}):")
    print("".join(f"  {n}\n" for n in new), end="")
    print(f"FIXED ({len(fixed)}):")
    print("".join(f"  {n}\n" for n in fixed), end="")
    return 1 if new else 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    sub = parser.add_subparsers(dest="cmd", required=True)
    for name in ("run", "baseline", "check"):
        p = sub.add_parser(name)
        p.add_argument("cargo_args", nargs=argparse.REMAINDER)
    cmp = sub.add_parser("compare")
    cmp.add_argument("before", type=Path)
    cmp.add_argument("after", type=Path)
    opts = parser.parse_args(argv)
    repo = Path(git(Path.cwd(), "rev-parse", "--show-toplevel"))

    if opts.cmd == "compare":
        read = lambda p: frozenset(l for l in p.read_text().splitlines() if l)
        return report(read(opts.before), read(opts.after))

    cargo_args = (
        opts.cargo_args[1:] if opts.cargo_args[:1] == ["--"] else opts.cargo_args
    )
    if not cargo_args:
        parser.error("give the cargo test arguments after --")
    try:
        if opts.cmd == "baseline" and not working_tree_matches_head(repo):
            raise FailsetError(
                "baseline needs rust/ and the Cargo files to match HEAD; take it before editing"
            )
        if opts.cmd == "check":
            base = load(entry(repo, source_key(repo, "HEAD"), cargo_args))
            if base is None:
                raise FailsetError(
                    "no cached baseline for HEAD with these arguments; run `baseline` on a clean tree first"
                )
            outcome, cached = failing_set(repo, cargo_args)
            print(f"baseline: {base.summary}\nafter:    {describe(outcome, cached)}")
            return report(base.failures, outcome.failures)
        outcome, cached = failing_set(repo, cargo_args)
    except FailsetError as err:
        print(f"failset: {err}", file=sys.stderr)
        return 2
    print(describe(outcome, cached), file=sys.stderr)
    print("".join(f"{n}\n" for n in sorted(outcome.failures)), end="")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
