#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Prune a worktree's cargo target directory (plan item 260).

A worktree's `target/` keeps every unit cargo ever built for it: one directory
per unit hash under `target/<profile>/build/<package>/<hash>/` and one
incremental directory per crate hash under `target/<profile>/incremental/`.
Old hashes stay after a rustc upgrade, a feature or dependency change, or a
branch switch, so the directory grows without bound. The rules
(docs/design/build-time/260-shared-cache.md, *Pruning policy*):

- a unit directory built by another rustc than the newest unit's: delete;
- a unit directory superseded by newer ones with the same fingerprint key
  (unit kind, rustc, features, target, profile, path, config): keep the
  newest `--keep` (3) per key, which leaves room for the variants a
  dependency's features give one key, and delete the rest;
- a unit directory that holds no file, or whose newest file is older than
  `--max-age-days`: delete;
- an incremental directory older than `--max-age-days`: delete;
- `--removed PATH`: delete that removed worktree's whole `target/`.

A deleted live unit only costs a rebuild. The script never touches a target
directory while cargo holds its `.cargo-lock`: it takes the lock itself.

    prune-targets.py [--target DIR] [--keep N] [--max-age-days D] [--dry-run]
    prune-targets.py --removed PATH [--dry-run]
"""

from __future__ import annotations

import argparse
import fcntl
import json
import os
import shutil
import subprocess
import sys
import time
from collections import defaultdict
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path

KEY_FIELDS = ("rustc", "features", "target", "profile", "path", "config", "compile_kind")
DAY_S = 86400


@dataclass(frozen=True)
class Unit:
    path: Path
    key: tuple
    rustc: object
    newest: float
    size: int


def tree_stats(path: Path) -> tuple[float, int]:
    newest = 0.0
    size = 0
    for directory, _, files in os.walk(path):
        for name in files:
            try:
                stat = os.lstat(os.path.join(directory, name))
            except FileNotFoundError:
                continue
            newest = max(newest, stat.st_mtime)
            size += stat.st_size
    return newest, size


def read_unit(path: Path) -> Unit:
    newest, size = tree_stats(path)
    fingerprint = path / "fingerprint"
    names = sorted(p.name for p in fingerprint.glob("*.json")) if fingerprint.is_dir() else []
    fields: tuple = ()
    rustc = None
    for name in names:
        try:
            data = json.loads((fingerprint / name).read_text(encoding="utf-8"))
        except (OSError, ValueError):
            continue
        rustc = data.get("rustc", rustc)
        fields += (name, *(json.dumps(data.get(field)) for field in KEY_FIELDS))
    key = (path.parent.name, fields) if fields else (path.parent.name, path.name)
    return Unit(path, key, rustc, newest, size)


def units(profile_dir: Path) -> list[Unit]:
    build = profile_dir / "build"
    if not build.is_dir():
        return []
    return [
        read_unit(unit)
        for package in sorted(build.iterdir())
        if package.is_dir()
        for unit in sorted(package.iterdir())
        if unit.is_dir()
    ]


def stale_units(found: list[Unit], keep: int, max_age_s: float, now: float) -> list[tuple[Unit, str]]:
    stale: dict[Path, tuple[Unit, str]] = {}
    with_rustc = [unit for unit in found if unit.rustc is not None]
    if with_rustc:
        current = max(with_rustc, key=lambda unit: unit.newest).rustc
        for unit in with_rustc:
            if unit.rustc != current:
                stale[unit.path] = (unit, "older rustc")
    by_key: dict[tuple, list[Unit]] = defaultdict(list)
    for unit in found:
        if unit.path not in stale:
            by_key[unit.key].append(unit)
    for group in by_key.values():
        group.sort(key=lambda unit: unit.newest, reverse=True)
        for unit in group[keep:]:
            stale[unit.path] = (unit, f"superseded (keep {keep})")
    for unit in found:
        if unit.path in stale:
            continue
        if unit.newest == 0.0:
            stale[unit.path] = (unit, "holds no file")
        elif now - unit.newest > max_age_s:
            stale[unit.path] = (unit, "older than the age limit")
    return sorted(stale.values(), key=lambda item: str(item[0].path))


def stale_incremental(profile_dir: Path, max_age_s: float, now: float) -> list[tuple[Path, int, str]]:
    incremental = profile_dir / "incremental"
    if not incremental.is_dir():
        return []
    stale = []
    for crate in sorted(incremental.iterdir()):
        newest, size = tree_stats(crate)
        if now - newest > max_age_s:
            stale.append((crate, size, "older than the age limit"))
    return stale


@contextmanager
def cargo_lock(profile_dir: Path):
    """Hold cargo's own lock on a profile directory, or yield False."""
    lock = profile_dir / ".cargo-lock"
    if not lock.exists():
        yield True
        return
    with open(lock, "a+b") as handle:
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            yield False
            return
        try:
            yield True
        finally:
            fcntl.flock(handle, fcntl.LOCK_UN)


def remove(path: Path, dry_run: bool) -> None:
    if not dry_run:
        shutil.rmtree(path, ignore_errors=True)


def prune_target(target: Path, keep: int, max_age_days: float, dry_run: bool) -> int:
    now = time.time()
    max_age_s = max_age_days * DAY_S
    freed = 0
    for profile_dir in sorted(p for p in target.iterdir() if p.is_dir() and (p / "build").is_dir()):
        with cargo_lock(profile_dir) as held:
            if not held:
                print(f"skip {profile_dir}: cargo holds its lock", file=sys.stderr)
                continue
            for unit, reason in stale_units(units(profile_dir), keep, max_age_s, now):
                print(f"{'would delete' if dry_run else 'delete'} {unit.path} ({unit.size >> 20} MB, {reason})")
                remove(unit.path, dry_run)
                freed += unit.size
            for path, size, reason in stale_incremental(profile_dir, max_age_s, now):
                print(f"{'would delete' if dry_run else 'delete'} {path} ({size >> 20} MB, {reason})")
                remove(path, dry_run)
                freed += size
    return freed


def default_target() -> Path:
    root = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], check=True, capture_output=True, text=True
    ).stdout.strip()
    return Path(os.environ.get("CARGO_TARGET_DIR") or Path(root) / "target")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--target", type=Path)
    parser.add_argument("--keep", type=int, default=3)
    parser.add_argument("--max-age-days", type=float, default=3.0)
    parser.add_argument("--removed", type=Path, help="a removed worktree whose target/ goes whole")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv)
    if args.keep < 1:
        parser.error("--keep must be at least 1")
    if args.removed is not None:
        target = args.removed / "target"
        if (args.removed / ".git").exists():
            parser.error(f"{args.removed} is still a worktree")
        if target.is_dir():
            freed = tree_stats(target)[1]
            print(f"{'would delete' if args.dry_run else 'delete'} {target} ({freed >> 20} MB)")
            remove(target, args.dry_run)
        return 0
    target = args.target or default_target()
    if not target.is_dir():
        print(f"no target directory at {target}")
        return 0
    freed = prune_target(target, args.keep, args.max_age_days, args.dry_run)
    print(f"{'would free' if args.dry_run else 'freed'} {freed >> 20} MB under {target}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
