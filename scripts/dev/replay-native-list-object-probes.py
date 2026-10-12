#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Rebuild exact retained list-object probes and compare retained complete stdout.

This runner does not execute Rust tests or infer native authority from byte equality.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

VERSIONS = ("8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0")
ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def require_digest(path, expected):
    actual = digest(path)
    if actual != expected:
        raise ValueError(
            f"Adapter refusal: {path}: actual SHA {actual}, expected {expected}"
        )


def read_json(path):
    return json.loads(path.read_text())


def tasks(corpus):
    result = []
    if corpus in ("operations", "assignment"):
        base = (
            ROOT
            / "rust/tcl-registry/tests/data"
            / (
                "native_list_operations"
                if corpus == "operations"
                else "native_list_assignment_order"
            )
        )
        manifest = read_json(base / "manifest.json")
        for row in manifest["rows"]:
            result.append(
                dict(
                    version=row["version"],
                    name=row["version"],
                    source=base / "probe.c",
                    source_sha=manifest["source_sha256"],
                    library_sha=row["library_sha256"],
                    expected=base / (row["version"] + ".txt"),
                    output_sha=row["stdout_sha256"],
                    arguments=[],
                    private=True,
                )
            )
        jim = read_json(base / "jim-manifest.json")
        hashes = jim["sha256"]
        result.append(
            dict(
                version="jim",
                name="jim",
                source=base / "jim-probe.c",
                source_sha=next(
                    v for k, v in hashes.items() if k.endswith("/jim-probe.c")
                ),
                library_sha=next(
                    v for k, v in hashes.items() if k.endswith("/libjim.a")
                ),
                expected=base / "jim.txt",
                output_sha=next(v for k, v in hashes.items() if k.endswith("/jim.txt")),
                arguments=[],
                private=False,
            )
        )
        if corpus == "assignment":
            trace = read_json(base / "trace-manifest.json")
            for row in trace["rows"]:
                args = (
                    ["trace"] if row["mode"] == "generic" else ["trace", "allow-inline"]
                )
                result.append(
                    dict(
                        version=row["version"],
                        name=row["version"] + "-" + row["mode"],
                        source=base / "trace-probe.c",
                        source_sha=trace["source_sha256"],
                        library_sha=row["library_sha256"],
                        expected=base / Path(row["stdout_path"]).name,
                        output_sha=row["stdout_sha256"],
                        arguments=args,
                        private=True,
                    )
                )
    else:
        base = (
            ROOT
            / "rust/tcl-syntax/tests/data/native_list_methods"
            / ("stock_length" if corpus == "stock-length" else "string_callbacks")
        )
        manifest = read_json(base / "manifest.json")
        for n, row in enumerate(manifest):
            jim = row["release"].startswith("Jim")
            version = "jim" if jim else row["release"]
            if corpus == "stock-length":
                empty = row["observations"] == 2
                source = (
                    "empty-result-probe.c"
                    if empty
                    else ("jim-probe.c" if jim else "probe.c")
                )
                output = (
                    "jim.txt" if jim else ("empty-" if empty else "") + version + ".txt"
                )
            else:
                source, output = row["source"], row["output"]
            arguments = [] if jim or corpus == "stock-length" else ["{library}"]
            result.append(
                dict(
                    version=version,
                    name=str(n) + "-" + output.removesuffix(".txt"),
                    source=base / source,
                    source_sha=row["source_sha256"],
                    library_sha=row["library_sha256"],
                    expected=base / output,
                    output_sha=row["output_sha256"],
                    arguments=arguments,
                    private=False,
                )
            )
    return result


def run(argv, destination, environment):
    try:
        completed = subprocess.run(
            argv, capture_output=True, timeout=60, env=environment, check=False
        )
        status, stdout, stderr = (
            completed.returncode,
            completed.stdout,
            completed.stderr,
        )
    except subprocess.TimeoutExpired as error:
        status, stdout, stderr = "timeout", error.stdout or b"", error.stderr or b""
    destination.with_suffix(".stdout").write_bytes(stdout)
    destination.with_suffix(".stderr").write_bytes(stderr)
    return (
        dict(
            argv=argv,
            status=status,
            stdout_sha256=hashlib.sha256(stdout).hexdigest(),
            stderr_sha256=hashlib.sha256(stderr).hexdigest(),
        ),
        stdout,
        stderr,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--corpus",
        choices=("operations", "assignment", "stock-length", "callbacks"),
        required=True,
    )
    parser.add_argument("--tcl-source-root", type=Path, required=True)
    parser.add_argument("--jim-source-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cc", default="cc")
    parser.add_argument("--provider", choices=(*VERSIONS, "jim"))
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    results = []
    environment = dict(os.environ)
    for task in tasks(args.corpus):
        version = task["version"]
        if args.provider and args.provider != version:
            continue
        directory = args.output / task["name"]
        directory.mkdir()
        record = dict(
            version=version,
            source=str(task["source"]),
            input_channel="Actual native API factory and original ASCII source/object-vector protocol in the retained probe.",
            expected_stream=str(task["expected"]),
        )
        try:
            require_digest(task["source"], task["source_sha"])
            require_digest(task["expected"], task["output_sha"])
            if version == "jim":
                tree = args.jim_source_root.resolve()
                library = tree / "libjim.a"
                includes = ["-I" + str(tree)]
                headers = [tree / "jim.h"]
                abi = []
                links = ["-lm", "-ldl", "-lssl", "-lcrypto", "-lz"]
            else:
                tree = (args.tcl_source_root / ("tcl" + version)).resolve()
                library = (
                    tree / "unix" / ("libtcl" + ".".join(version.split(".")[:2]) + ".a")
                )
                includes = ["-I" + str(tree / "generic"), "-I" + str(tree / "unix")]
                headers = [tree / "generic/tcl.h"]
                if task["private"]:
                    headers += [
                        tree / "generic/tclInt.h",
                        tree / "generic/tclCompile.h",
                    ]
                abi = ["-DSTDC_HEADERS=1", "-DHAVE_UNISTD_H=1"]
                links = ["-lm", "-ldl", "-lpthread", "-lz"]
            require_digest(library, task["library_sha"])
            binary = directory / "probe"
            argv = [
                args.cc,
                *abi,
                *includes,
                str(task["source"]),
                str(library),
                *links,
                "-o",
                str(binary),
            ]
            record.update(
                source_sha256=digest(task["source"]),
                library_sha256=digest(library),
                headers={str(p): digest(p) for p in headers},
            )
            compilation, _, _ = run(argv, directory / "compile", environment)
            record["compile"] = compilation
            if compilation["status"] != 0:
                record.update(
                    status="harness-failure",
                    comparison="No native answer: compiler failed or timed out.",
                )
            else:
                record["executed_probe_sha256"] = digest(binary)
                arguments = [
                    str(tree / "library") if a == "{library}" else a
                    for a in task["arguments"]
                ]
                execution, stdout, stderr = run(
                    [str(binary), *arguments], directory / "run", environment
                )
                record["run"] = execution
                matched = (
                    execution["status"] == 0
                    and stdout == task["expected"].read_bytes()
                    and stderr == b""
                )
                record.update(
                    status="matched" if matched else "comparison-failure",
                    comparison="Current process0 and exact retained original stdout; current stderr must be empty. Original stderr is not recorded by these manifests. Guest Error and unavailable operation rows remain part of stdout.",
                )
        except (OSError, ValueError) as error:
            record.update(status="adapter-refusal", comparison=str(error))
        (directory / "receipt.json").write_text(json.dumps(record, indent=2) + "\n")
        results.append(record)
    receipt = dict(
        corpus=args.corpus,
        native_process_budget_seconds=60,
        serial=True,
        results=results,
        passed=bool(results) and all(r["status"] == "matched" for r in results),
        limitations="This is an independent current rebuild. Original executed executable hashes and labels remain in the unchanged original receipts. No Rust pass, native token, body/frame entry or unrestricted class/effect grant is inferred.",
    )
    (args.output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(
        json.dumps(
            dict(
                passed=receipt["passed"],
                comparisons=len(results),
                receipt=str(args.output / "receipt.json"),
            )
        )
    )
    return 0 if receipt["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
