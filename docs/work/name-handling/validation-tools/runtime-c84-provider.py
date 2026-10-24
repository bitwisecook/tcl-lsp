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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Seal the exact Tcl 8.4 provider required by the runtime integer-cache tests."""

import argparse
import hashlib
import json
import os
import shutil
import stat
import sys
from pathlib import Path

ARCHIVE_SHA = "dc67eba353eeb83cbfb9ce5e0f6ff52972297374799156c09cef1ac06cc5cfff"
PINNED = {
    "unix/libtcl8.4.a": ARCHIVE_SHA,
    "generic/tcl.h": "824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf",
    "generic/tclObj.c": "6e20f047f13e19527f746934b7a9472405d1871e0936687004506f966bb8bd9c",
    "generic/tclInt.h": "f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a",
    "library/init.tcl": "a7f4388d1989f005872a47ca5d698aa06680aca21407462301972c5a6048d92b",
    "unix/tclConfig.sh": "33d929abbd1102f01624ca3aa217af1bcdc7e6023be2ea094e82e5228f8880e9",
}
TOOLS = (
    "cc",
    "gcc",
    "make",
    "ar",
    "ld",
    "as",
    "ranlib",
    "bash",
    "sh",
    "curl",
    "tar",
    "gzip",
    "git",
    "install",
    "sed",
    "awk",
    "grep",
    "getconf",
)
INPUTS = (
    "Makefile",
    "scripts/dev/ensure-test-deps.sh",
    "scripts/dev/tcl-reference-toolchains.sh",
    ".claude/skills/fetch-tcl-source/fetch_tcl_source.sh",
    "rust/tcl-dialect/data/reference-toolchains.tsv",
    "docs/work/name-handling/validation-tools/runtime-c84-provider.py",
)
OVERRIDES = (
    "CC",
    "CFLAGS",
    "CPPFLAGS",
    "LDFLAGS",
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "TCL_LIBRARY",
    "TCLLIBPATH",
    "TCL_LSP_TCL_SOURCE_PARENT",
    "TCL_LSP_TCLSH84",
    "TCL_LSP_TCL_REFERENCE_MANIFEST",
)


def require(value, message):
    if not value:
        raise RuntimeError(message)


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def save(path, value):
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n"
    )


def entry(path):
    metadata = path.lstat()
    result: dict[str, object] = {"mode": stat.S_IMODE(metadata.st_mode)}
    if path.is_symlink():
        result["kind"] = "symlink"
        result["target"] = os.readlink(path)
        resolved = path.resolve(strict=True)
        require(resolved.is_file(), "non-file symbolic link: " + str(path))
        result["resolved_path"] = str(resolved)
        result["sha256"] = sha(resolved)
    elif path.is_file():
        result["kind"] = "file"
        result["sha256"] = sha(path)
    elif path.is_dir():
        result["kind"] = "directory"
    else:
        raise RuntimeError("unsupported provider entry: " + str(path))
    return result


def tree(root):
    require(
        root.is_dir() and not root.is_symlink(),
        "real provider directory required: " + str(root),
    )
    return {
        str(path.relative_to(root)): entry(path) for path in sorted(root.rglob("*"))
    }


def tool_seals():
    paths = {name: Path(shutil.which(name) or "/absent") for name in TOOLS}
    paths["python"] = Path(sys.executable)
    return {
        name: {"path": str(path), "entry": entry(path)} for name, path in paths.items()
    }


def input_seals(root):
    return {name: entry(root / name) for name in INPUTS}


def preflight(root, out):
    require(
        not out.exists() and not out.is_relative_to(root),
        "fresh external output required",
    )
    require(
        not (root / "tmp/tcl8.4.20").exists(),
        "fresh uncached Tcl 8.4 provider required",
    )
    require(
        os.environ.get("TCL_LSP_TCL_BIN_DIR") == str(out / "reference-bin"),
        "private reference-bin required",
    )
    for name in OVERRIDES:
        require(not os.environ.get(name), "unexpected Tcl provider override: " + name)
    require(
        all(shutil.disk_usage(path).free >= 2 * 1024**3 for path in (root, out.parent)),
        "disk below 2GiB floor",
    )
    out.mkdir()
    save(
        out / "preflight.json",
        {
            "tools": tool_seals(),
            "inputs": input_seals(root),
            "root": str(root),
            "source": os.environ.get("GITHUB_SHA"),
            "archive_sha256": ARCHIVE_SHA,
        },
    )


def bootstrap_seals(out):
    names = (
        "bootstrap.log",
        "bootstrap-log.json",
        "bootstrap-exits.txt",
        "bootstrap.stdout",
        "bootstrap.stderr",
        "bootstrap.process.json",
    )
    result = {name: entry(out / name) for name in names if (out / name).exists()}
    require(
        "bootstrap.log" in result
        or {"bootstrap.stdout", "bootstrap.stderr"}.issubset(result),
        "retained bootstrap streams required",
    )
    return result


def ready(root, out):
    require(
        not (out / "provider-before.json").exists(),
        "exclusive provider snapshot required",
    )
    require(
        all(shutil.disk_usage(path).free >= 2 * 1024**3 for path in (root, out)),
        "disk below 2GiB floor",
    )
    before = json.loads((out / "preflight.json").read_text())
    require(
        before["root"] == str(root)
        and before["source"] == os.environ.get("GITHUB_SHA"),
        "source location changed",
    )
    require(
        before["tools"] == tool_seals() and before["inputs"] == input_seals(root),
        "bootstrap input or tool changed",
    )
    provider = root / "tmp/tcl8.4.20"
    for name, expected in PINNED.items():
        require(
            sha(provider / name) == expected, "verified Tcl 8.4 input differs: " + name
        )
    require(
        "-std=gnu17" in (provider / "unix/Makefile").read_text(),
        "GNU17 Tcl 8.4 build required",
    )
    require(
        (out / "reference-bin/tclsh8.4").is_file(),
        "private reference executable required",
    )
    save(
        out / "provider-before.json",
        {
            "source": before["source"],
            "provider": tree(provider),
            "reference_bin": tree(out / "reference-bin"),
            "bootstrap": bootstrap_seals(out),
            "tools": tool_seals(),
            "inputs": input_seals(root),
            "scope": "Exact verified archive/header only; provider setup does not prove the Rust updater tests passed. Declared executable seals do not represent a trace of every operating-system library or child executable.",
        },
    )


def verify(root, out):
    before = json.loads((out / "provider-before.json").read_text())
    current = {
        "source": os.environ.get("GITHUB_SHA"),
        "provider": tree(root / "tmp/tcl8.4.20"),
        "reference_bin": tree(out / "reference-bin"),
        "bootstrap": bootstrap_seals(out),
        "tools": tool_seals(),
        "inputs": input_seals(root),
    }
    changes = [name for name, value in current.items() if before[name] != value]
    save(
        out / "provider-after.json",
        {"complete": not changes, "changed_components": changes, "current": current},
    )
    require(not changes, "provider or tool inputs changed: " + ", ".join(changes))


def drain(_root, out):
    digest = hashlib.sha256()
    count = retained = 0
    with (out / "bootstrap.log").open("xb") as target:
        while data := sys.stdin.buffer.read(65536):
            digest.update(data)
            count += len(data)
            kept = data[: max(0, 512 * 1024 - retained)]
            target.write(kept)
            retained += len(kept)
    save(
        out / "bootstrap-log.json",
        {
            "raw_bytes": count,
            "raw_sha256": digest.hexdigest(),
            "retained_bytes": retained,
            "complete": retained == count,
        },
    )
    require(retained == count, "bootstrap log exceeded 512KiB retention cap")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=("preflight", "ready", "verify", "drain"))
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    root = Path.cwd().resolve()
    out = args.out.resolve()
    try:
        {"preflight": preflight, "ready": ready, "verify": verify, "drain": drain}[
            args.phase
        ](root, out)
    except (OSError, RuntimeError, KeyError, ValueError) as error:
        if out.is_dir():
            save(
                out / (args.phase + "-incomplete.json"),
                {"complete": False, "error": str(error)},
            )
        raise SystemExit(str(error)) from error


if __name__ == "__main__":
    main()
