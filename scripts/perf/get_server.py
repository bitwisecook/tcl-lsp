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

"""Resolve a released version tag to a runnable `tcl-lsp-server` binary.

Two sources, in order:

1. **The published release asset** (`tcl-lsp-server-<target>`). This is the
   binary users actually ran, which is what a release-notes graph should
   measure — a locally rebuilt binary can differ in optimisation flags,
   toolchain version and feature set from what shipped.
2. **A build from the tag**, for versions predating the native server
   assets (v2.1.0–v2.1.4). Uses a detached `git worktree` so the working
   tree is never disturbed, and caches the result.

Binaries are cached under `scripts/perf/servers/<tag>/tcl-lsp-server` and
reused, so re-running the suite costs nothing after the first pass.

Usage:
    get_server.py --tag v2.1.14              # prints the binary path
    get_server.py --all                      # resolve every manifest tag
"""

from __future__ import annotations

import argparse
import platform
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parent.parent
CACHE = HERE / "servers"


def host_target() -> str:
    """The Rust target triple for this host, matching release asset names."""
    machine = platform.machine()
    arch = {
        "arm64": "aarch64",
        "aarch64": "aarch64",
        "x86_64": "x86_64",
        "AMD64": "x86_64",
    }.get(machine, machine)
    system = platform.system()
    if system == "Darwin":
        return f"{arch}-apple-darwin"
    if system == "Linux":
        return f"{arch}-unknown-linux-gnu"
    if system == "Windows":
        return f"{arch}-pc-windows-msvc.exe"
    raise SystemExit(f"unsupported platform {system}")


def version_key(tag: str) -> tuple[int, ...]:
    """Semantic ordering, so v2.1.9 sorts before v2.1.10."""
    return tuple(int(p) for p in tag.lstrip("v").split("."))


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, capture_output=True, text=True, check=False, **kw)


def from_release_asset(tag: str, target: str, dest: Path) -> bool:
    """Download the published server asset for `tag`, if there is one."""
    asset = f"tcl-lsp-server-{target}"
    dest.parent.mkdir(parents=True, exist_ok=True)
    r = run(
        [
            "gh",
            "release",
            "download",
            tag,
            "--repo",
            "bitwisecook/tcl-lsp",
            "-p",
            asset,
            "-D",
            str(dest.parent),
            "--clobber",
        ]
    )
    if r.returncode != 0:
        return False
    downloaded = dest.parent / asset
    if not downloaded.exists():
        return False
    downloaded.replace(dest)
    dest.chmod(0o755)
    # macOS quarantines downloaded executables; without this the first exec
    # fails with a Gatekeeper kill that looks like a server crash.
    if platform.system() == "Darwin":
        run(["xattr", "-d", "com.apple.quarantine", str(dest)])
    return True


def from_source(tag: str, dest: Path) -> bool:
    """Build the server from `tag` in a detached worktree.

    A worktree rather than a checkout so the caller's working tree — very
    possibly dirty, very possibly the thing being compared against — is
    untouched. The worktree is removed on success or failure.
    """
    work = HERE / ".build" / tag
    if work.exists():
        run(["git", "worktree", "remove", "--force", str(work)], cwd=REPO_ROOT)
        shutil.rmtree(work, ignore_errors=True)
    work.parent.mkdir(parents=True, exist_ok=True)

    r = run(["git", "worktree", "add", "--detach", str(work), tag], cwd=REPO_ROOT)
    if r.returncode != 0:
        print(f"  worktree add {tag} failed: {r.stderr.strip()[:200]}", file=sys.stderr)
        return False
    try:
        # A separate target dir per tag: sharing one with the main tree would
        # thrash the cache on every version switch and can pick up stale
        # artefacts from an incompatible toolchain.
        r = run(
            [
                "cargo",
                "build",
                "--release",
                "--bin",
                "tcl-lsp-server",
                "--target-dir",
                str(work / "target"),
            ],
            cwd=work,
        )
        if r.returncode != 0:
            print(f"  cargo build {tag} failed:\n{r.stderr[-2000:]}", file=sys.stderr)
            return False
        built = work / "target" / "release" / "tcl-lsp-server"
        if not built.exists():
            return False
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(built, dest)
        dest.chmod(0o755)
        return True
    finally:
        run(["git", "worktree", "remove", "--force", str(work)], cwd=REPO_ROOT)
        shutil.rmtree(work, ignore_errors=True)


def resolve(tag: str, *, target: str, force: bool = False) -> Path | None:
    """Return a runnable binary for `tag`, downloading or building as needed."""
    dest = CACHE / tag / "tcl-lsp-server"
    if dest.exists() and not force:
        return dest
    if from_release_asset(tag, target, dest):
        print(f"  {tag}: release asset", file=sys.stderr)
        return dest
    print(f"  {tag}: no {target} asset, building from tag", file=sys.stderr)
    if from_source(tag, dest):
        return dest
    return None


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--manifest", type=Path, default=HERE / "MANIFEST.toml")
    ap.add_argument("--tag")
    ap.add_argument("--all", action="store_true")
    ap.add_argument("--force", action="store_true", help="ignore the cache")
    ap.add_argument("--target", default=None, help="override the host target triple")
    args = ap.parse_args()

    target = args.target or host_target()
    with args.manifest.open("rb") as fh:
        manifest = tomllib.load(fh)

    if args.all:
        tags = sorted(manifest["versions"]["tags"], key=version_key)
    elif args.tag:
        tags = [args.tag]
    else:
        ap.error("pass --tag or --all")

    missing = []
    for tag in tags:
        path = resolve(tag, target=target, force=args.force)
        if path is None:
            missing.append(tag)
            print(f"{tag}\tUNAVAILABLE")
        else:
            print(f"{tag}\t{path}")

    if missing:
        print(f"\n{len(missing)} unavailable: {', '.join(missing)}", file=sys.stderr)
    return 1 if missing else 0


if __name__ == "__main__":
    raise SystemExit(main())
