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

"""Deterministically reconstitute the corpus from `MANIFEST.toml`.

Idempotent: an already-correct checkout is left alone. The important
property is not speed but *pinning* — a cross-version graph is only
meaningful if every version measured the same bytes, so a checkout that
does not match its pinned commit is a hard error, never a silent
"benchmark whatever is on disk".

The same pinning makes this the general corpus fetcher, not just the
benchmark's: `--scope irules` reconstitutes the iRules corpus, and
`--scope everything` both dialects. To grow the set, add a `[[repo]]`
block with an exact `commit` and put its `group` in a scope. Advancing an
existing pin invalidates stored benchmark results — see the header of
`MANIFEST.toml`.

Usage:
    fetch_corpus.py [--scope NAME] [--dest DIR] [--verify-only]
    fetch_corpus.py --list
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

import tomllib

HERE = Path(__file__).resolve().parent
DEFAULT_DEST = HERE / "corpus"


def load_manifest(path: Path) -> dict:
    with path.open("rb") as fh:
        return tomllib.load(fh)


def git(*args: str, cwd: Path | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["git", *args], cwd=cwd, capture_output=True, text=True, check=False
    )


def head_of(repo: Path) -> str | None:
    """The checked-out commit, or None if `repo` is not a git checkout."""
    if not (repo / ".git").exists():
        return None
    r = git("rev-parse", "HEAD", cwd=repo)
    return r.stdout.strip() if r.returncode == 0 else None


def ensure_repo(entry: dict, dest: Path, *, verify_only: bool) -> tuple[bool, str]:
    """Bring one manifest entry to its pinned commit.

    Returns `(ok, message)`. Cloning shallow then fetching the exact SHA
    keeps the download small while still landing on the pin — a plain
    `--depth 1` clone of the default branch usually will *not* contain a
    pinned historical commit, so the explicit fetch is not redundant.
    """
    target = dest / entry["group"] / entry["name"]
    want = entry["commit"]
    have = head_of(target)

    if have == want:
        return True, f"ok       {entry['group']}/{entry['name']} @ {want[:12]}"
    if verify_only:
        state = "missing" if have is None else f"at {have[:12]}"
        return (
            False,
            f"MISMATCH {entry['group']}/{entry['name']} {state}, want {want[:12]}",
        )

    target.parent.mkdir(parents=True, exist_ok=True)
    if have is None:
        if target.exists():
            return False, f"FAIL     {target} exists but is not a git checkout"
        r = git(
            "clone",
            "--quiet",
            "--filter=blob:none",
            "--no-checkout",
            entry["url"],
            str(target),
        )
        if r.returncode != 0:
            return False, f"FAIL     clone {entry['url']}: {r.stderr.strip()[:200]}"

    # Fetch the exact pin. A partial-clone (`--filter=blob:none`) checkout
    # fetches blobs on demand, so this stays cheap even for tcllib.
    r = git("fetch", "--quiet", "origin", want, cwd=target)
    if r.returncode != 0:
        # Some servers refuse to serve an arbitrary SHA; fall back to a full
        # fetch, which always can.
        r = git("fetch", "--quiet", "--tags", "origin", cwd=target)
        if r.returncode != 0:
            return False, f"FAIL     fetch {entry['name']}: {r.stderr.strip()[:200]}"

    r = git("checkout", "--quiet", "--force", want, cwd=target)
    if r.returncode != 0:
        return (
            False,
            f"FAIL     checkout {entry['name']}@{want[:12]}: {r.stderr.strip()[:200]}",
        )

    got = head_of(target)
    if got != want:
        return False, f"FAIL     {entry['name']} landed on {got}, want {want}"
    return True, f"fetched  {entry['group']}/{entry['name']} @ {want[:12]}"


def scope_groups(manifest: dict, scope: str) -> list[str]:
    scopes = manifest.get("scope", {})
    if scope not in scopes:
        raise SystemExit(f"unknown scope {scope!r}; have {sorted(scopes)}")
    return list(scopes[scope]["groups"])


# iRule sources are published as often in `.txt` (DevCentral) or `.irule` as
# in `.tcl`, so counting only `*.tcl` under-reports the iRules corpus by
# roughly three quarters and makes a half-fetched tree look complete.
SOURCE_SUFFIXES = ("*.tcl", "*.irule", "*.tmsh")


def describe(manifest: dict) -> None:
    """Print the groups and scopes on offer, so `--scope` is discoverable."""
    counts: dict[str, int] = {}
    for entry in manifest["repo"]:
        counts[entry["group"]] = counts.get(entry["group"], 0) + 1
    print("groups:")
    for group in sorted(counts):
        print(f"  {group:<24} {counts[group]:>3} repo(s)")
    print("\nscopes:")
    for scope in sorted(manifest.get("scope", {})):
        groups = manifest["scope"][scope]["groups"]
        repos = sum(counts.get(g, 0) for g in groups)
        print(f"  {scope:<24} {repos:>3} repo(s)  ({', '.join(groups)})")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--manifest", type=Path, default=HERE / "MANIFEST.toml")
    ap.add_argument("--dest", type=Path, default=DEFAULT_DEST)
    ap.add_argument("--scope", default="full")
    ap.add_argument(
        "--verify-only",
        action="store_true",
        help="check pins without network access; exit non-zero on drift",
    )
    ap.add_argument(
        "--include-private",
        action="store_true",
        help='also fetch entries marked `visibility = "private"`, which need credentials',
    )
    ap.add_argument(
        "--list",
        action="store_true",
        help="print the available groups and scopes, then exit",
    )
    args = ap.parse_args()

    manifest = load_manifest(args.manifest)
    if args.list:
        describe(manifest)
        return 0

    groups = set(scope_groups(manifest, args.scope))
    # Sorted so the log is stable run to run, like everything else here.
    entries = sorted(
        (e for e in manifest["repo"] if e["group"] in groups),
        key=lambda e: (e["group"], e["name"]),
    )
    # Skipping is reported, never silent: a private entry quietly dropped from
    # a corpus is indistinguishable from one that was never there.
    skipped = [e for e in entries if e.get("visibility") == "private"]
    if not args.include_private:
        entries = [e for e in entries if e.get("visibility") != "private"]
    else:
        skipped = []

    failures = 0
    for entry in entries:
        ok, msg = ensure_repo(entry, args.dest, verify_only=args.verify_only)
        print(msg)
        failures += not ok
    for entry in skipped:
        print(
            f"skipped  {entry['group']}/{entry['name']} (private; --include-private to fetch)"
        )

    files = 0
    if args.dest.exists():
        files = sum(len(list(args.dest.rglob(pat))) for pat in SOURCE_SUFFIXES)
    print(
        f"\nscope={args.scope} repos={len(entries)} "
        f"corpus_revision={manifest['corpus']['revision']} "
        f"source files={files} ({', '.join(SOURCE_SUFFIXES)})"
    )
    if failures:
        print(f"{failures} repo(s) not at their pin", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
