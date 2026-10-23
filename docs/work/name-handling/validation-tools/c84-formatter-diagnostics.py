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

"""WORK-only unchanged original C84 archive and independently loaded image probe."""

import argparse
import hashlib
import importlib.util
import json
import os
import re
import resource
import shutil
import sys
import time
from pathlib import Path

CAP = 20 * 1024**2
FLOOR = 2 * 1024**3
EXECUTOR_SHA = "88125bd5267ac897dda4b5f665872c5ae0ab9984b0f42a41edeadf718873d811"
SNAPSHOT_SHA = "e6613bbd7cef5f002fceafa950a516905ece370e9242139ad3ad3eae3d8ed300"
PROBE_SHA = "2c1e656d0a1bdc6a86f84a08a8601cb414eef16ce7f53ec9b1d882c99f91ff6e"
OBSERVATIONS_SHA = "cc19ad501bb96674935c4dd855ec055b3095b97673f1dca64f5cdc8a06a6929c"
HEADER_SHA = "824fdc7632335682f70066a013b655b176b5539b194e85f8b5ccddb1815bcccf"
BASE = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise RuntimeError(message)


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def provider_files(root):
    return {
        str(path.relative_to(root)): {
            "sha256": sha(path),
            "symlink": os.readlink(path) if path.is_symlink() else None,
        }
        for path in sorted(root.rglob("*"))
        if path.is_file()
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sha", required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    require(re.fullmatch("[0-9a-f]{40}", args.sha), "full exact SHA required")
    require(
        os.environ.get("GITHUB_SHA") == args.sha
        and os.environ.get("GITHUB_REF") == "refs/heads/work",
        "actual WORK source required",
    )
    root = Path.cwd().resolve()
    out = args.out.resolve()
    require(
        not out.exists() and not out.is_relative_to(root),
        "fresh external output required",
    )
    out.mkdir(parents=True)
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    deadline = float(os.environ["C84_FORMATTER_DEADLINE"])
    require(
        time.time() < deadline <= time.time() + 2100,
        "bounded actual job deadline required",
    )
    executor = BASE / "runtime-diagnostics-exec.py"
    require(sha(executor) == EXECUTOR_SHA, "reviewed process owner changed")
    spec = importlib.util.spec_from_file_location("c84_formatter_owner", executor)
    if spec is None or spec.loader is None:
        raise RuntimeError("process owner loader unavailable")
    owner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(owner)
    helper = root / "scripts/dev/tracked-source-snapshot.py"
    require(sha(helper) == SNAPSHOT_SHA, "strict source owner changed")
    probe = root / "rust/tcl-host-c-abi/testdata/native_integer_formatter/probe.c"
    fixture = probe.with_name("observations.tsv")
    require(
        sha(probe) == PROBE_SHA and sha(fixture) == OBSERVATIONS_SHA,
        "unchanged original probe and all812 observations required",
    )
    require(
        len(fixture.read_bytes().splitlines()) == 812,
        "whole original812 fixture required",
    )
    tree = root / "tmp/tcl8.4.20"
    require(not tree.exists(), "fresh uncached C84 CI bootstrap required")
    tools = {
        name: Path(shutil.which(name) or "/absent").resolve()
        for name in ["cc", "gcc", "make", "ar", "ld", "git"]
    }
    tools["python"] = Path(sys.executable).resolve()
    require(
        all(path.is_file() for path in tools.values()),
        "all explicit native tools required",
    )
    before_tools = {
        name: {"path": str(path), "sha256": sha(path)} for name, path in tools.items()
    }
    save(out / "tools-before.json", before_tools)
    environment = os.environ.copy()
    for name in [
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
    ]:
        require(not environment.get(name), "unexpected provider override: " + name)
    environment["TCL_LSP_TCL_BIN_DIR"] = str(out / "reference-bin")
    used = 0
    processes = []

    def execute(label, argv, seconds):
        nonlocal used
        require(
            CAP - used > 0 and deadline - time.time() > 0,
            "aggregate log/deadline exhausted",
        )
        receipt = owner.run(
            argv,
            cwd=root,
            env=environment,
            stdout_path=out / (label + ".stdout"),
            stderr_path=out / (label + ".stderr"),
            receipt_path=out / (label + ".process.json"),
            artifact_path=argv[0],
            deadline_seconds=min(seconds, deadline - time.time()),
            cap_bytes=CAP - used,
            disk_floor_bytes=FLOOR,
            disk_paths=(root, out, Path(os.environ["RUNNER_TEMP"])),
        )
        used += receipt["retained_bytes"]
        processes.append({"label": label, "receipt": receipt})
        require(
            receipt["complete"] and receipt["exit_code"] == 0,
            "process failed/incomplete: " + label,
        )
        return receipt

    native_before = None
    error = None
    final_errors = []
    outcomes = {}
    try:
        execute(
            "git-before",
            [str(tools["git"]), "status", "--porcelain=v1", "--untracked-files=all"],
            30,
        )
        require(
            (out / "git-before.stdout").stat().st_size == 0,
            "clean actual source required",
        )
        execute(
            "source-before",
            [
                str(tools["python"]),
                str(helper),
                "record",
                str(out / "tracked-source-before.json"),
            ],
            180,
        )
        require(
            json.loads((out / "tracked-source-before.json").read_text())[
                "source_commit"
            ]
            == args.sha,
            "actual tracked source commit differs",
        )
        for name in ["cc", "gcc", "make", "ar", "ld"]:
            execute(name + "-version", [str(tools[name]), "--version"], 30)
        execute(
            "bootstrap",
            [str(tools["make"]), "TCL_LSP_TCL_RELEASES=8.4", "ensure-tcl-deps"],
            900,
        )
        archive = tree / "unix/libtcl8.4.a"
        header = tree / "generic/tcl.h"
        require(sha(header) == HEADER_SHA, "exact original C84 header required")
        require(
            "-std=gnu17" in (tree / "unix/Makefile").read_text(),
            "existing CI GNU17 C84 build required",
        )
        for path, expected in [
            (
                tree / "generic/tclObj.c",
                "6e20f047f13e19527f746934b7a9472405d1871e0936687004506f966bb8bd9c",
            ),
            (
                tree / "generic/tclInt.h",
                "f51c8e66c562de424b84a35a04b6b86d9a135ef7d4ca891b35c3b0586bc0ce1a",
            ),
            (
                tree / "library/init.tcl",
                "a7f4388d1989f005872a47ca5d698aa06680aca21407462301972c5a6048d92b",
            ),
        ]:
            require(sha(path) == expected, "original native C84 source input changed")
        native_before = provider_files(tree)
        save(out / "provider-before.json", native_before)
        native = out / "native"
        native.mkdir()
        image = native / "libtcl8.4-observed.so"
        executable = native / "probe"
        for path in [
            archive,
            header,
            tree / "generic/tclObj.c",
            tree / "generic/tclInt.h",
            tree / "unix/Makefile",
            tree / "unix/tclConfig.sh",
            tree / "unix/config.log",
            tree / "library/init.tcl",
            probe,
            fixture,
        ]:
            name = (
                "provider-" + path.name
                if path not in [probe, fixture]
                else "original-" + path.name
            )
            shutil.copyfile(path, native / name)
            require(
                sha(native / name) == sha(path), "retained exact native input differs"
            )
        save(
            out / "native-inputs.json",
            {
                "archive_sha256": sha(archive),
                "header_sha256": sha(header),
                "config_sha256": sha(tree / "unix/tclConfig.sh"),
                "source": args.sha,
            },
        )
        execute(
            "link",
            [
                str(tools["cc"]),
                "-shared",
                "-Wl,--whole-archive",
                str(archive),
                "-Wl,--no-whole-archive",
                "-lm",
                "-ldl",
                "-lpthread",
                "-lz",
                "-o",
                str(image),
            ],
            60,
        )
        execute(
            "compile",
            [
                str(tools["cc"]),
                "-I" + str(tree / "generic"),
                "-I" + str(tree / "unix"),
                str(probe),
                str(archive),
                "-lm",
                "-ldl",
                "-lpthread",
                "-lz",
                "-o",
                str(executable),
            ],
            60,
        )
        before_image = sha(image)
        before_probe = sha(executable)
        for label, argv in [
            ("archive", [str(executable)]),
            ("loaded", [str(executable), str(image)]),
        ]:
            execute(label, argv, 30)
            raw = (out / (label + ".stdout")).read_bytes()
            require(
                (out / (label + ".stderr")).stat().st_size == 0,
                "original native stderr must be empty",
            )
            require(
                len(raw.splitlines()) == 812
                and hashlib.sha256(raw).hexdigest() == OBSERVATIONS_SHA,
                "whole original812 differs: " + label,
            )
            require(raw == fixture.read_bytes(), "original full counted bytes differ")
            require(
                sha(image) == before_image and sha(executable) == before_probe,
                "loaded image/probe changed",
            )
            outcomes[label] = {
                "rows": 812,
                "sha256": OBSERVATIONS_SHA,
                "exit": 0,
                "stderr_bytes": 0,
            }
        require(
            all(
                (out / (label + ".stderr")).stat().st_size == 0
                for label in ["link", "compile"]
            ),
            "original link/compile stderr must be empty",
        )
        save(
            out / "native-build.json",
            {
                "archive_sha256": sha(archive),
                "header_sha256": sha(header),
                "image_sha256": before_image,
                "probe_executable_sha256": before_probe,
                "probe_source_sha256": PROBE_SHA,
                "original_observations_sha256": OBSERVATIONS_SHA,
                "config_sha256": sha(tree / "unix/tclConfig.sh"),
                "scope": "Exact existing original C84 archive and independently dlopened image observations only; no Rust loader/test/admission-table pass or authority for another build.",
            },
        )
    except (RuntimeError, OSError, ValueError, KeyError, TypeError) as exception:
        error = repr(exception)
    finally:
        if native_before is not None:
            try:
                after = provider_files(tree)
                save(out / "provider-after.json", after)
                require(after == native_before, "native source/build inputs changed")
            except (RuntimeError, OSError, ValueError) as exception:
                final_errors.append(repr(exception))
        try:
            execute(
                "source-after",
                [
                    str(tools["python"]),
                    str(helper),
                    "verify",
                    str(out / "tracked-source-before.json"),
                ],
                180,
            )
            source_after = json.loads((out / "source-after.stdout").read_text())
            require(
                source_after["status"] == "PASS"
                and source_after["source_commit"] == args.sha,
                "full source verification refused",
            )
            execute(
                "git-after",
                [
                    str(tools["git"]),
                    "status",
                    "--porcelain=v1",
                    "--untracked-files=all",
                ],
                30,
            )
            require(
                (out / "git-after.stdout").stat().st_size == 0, "final source not clean"
            )
        except (RuntimeError, OSError, ValueError, KeyError) as exception:
            final_errors.append(repr(exception))
        after_tools = {
            name: {"path": str(path), "sha256": sha(path)}
            for name, path in tools.items()
        }
        save(out / "tools-after.json", after_tools)
        if after_tools != before_tools:
            final_errors.append("actual native tool ELF changed")
        save(
            out / "summary.json",
            {
                "source": args.sha,
                "complete": error is None
                and not final_errors
                and set(outcomes) == {"archive", "loaded"},
                "error": error,
                "final_errors": final_errors,
                "outcomes": outcomes,
                "processes": processes,
                "retained_log_bytes": used,
                "log_cap_bytes": CAP,
                "scope": "Unchanged original812 native experiment only; prospective build-pair evidence, no admission relaxation, Rust assertion outcome or runtime performance claim.",
            },
        )
    return (
        0
        if error is None and not final_errors and set(outcomes) == {"archive", "loaded"}
        else 125
    )


if __name__ == "__main__":
    raise SystemExit(main())
