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

"""Deterministically reconstitute the benchmark corpus from `MANIFEST.toml`.

Idempotent: an already-correct checkout is left alone. The important
property is not speed but *pinning* — a cross-version graph is only
meaningful if every version measured the same bytes, so a checkout that
does not match its pinned commit is a hard error, never a silent
"benchmark whatever is on disk".

Usage:
    fetch_corpus.py [--scope small|medium|full] [--dest DIR] [--verify-only]
"""

from __future__ import annotations

import argparse
import subprocess
import sys
import tomllib
from pathlib import Path

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
    args = ap.parse_args()

    manifest = load_manifest(args.manifest)
    groups = set(scope_groups(manifest, args.scope))
    # Sorted so the log is stable run to run, like everything else here.
    entries = sorted(
        (e for e in manifest["repo"] if e["group"] in groups),
        key=lambda e: (e["group"], e["name"]),
    )

    failures = 0
    for entry in entries:
        ok, msg = ensure_repo(entry, args.dest, verify_only=args.verify_only)
        print(msg)
        failures += not ok

    tcl_files = sum(1 for _ in args.dest.rglob("*.tcl")) if args.dest.exists() else 0
    print(
        f"\nscope={args.scope} repos={len(entries)} "
        f"corpus_revision={manifest['corpus']['revision']} .tcl files={tcl_files}"
    )
    if failures:
        print(f"{failures} repo(s) not at their pin", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
