#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Check or independently replay exact native_naming_corners CLI controls."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

PROVIDERS = {
    "tcl8.4": "8.4.20",
    "tcl8.5": "8.5.19",
    "tcl8.6": "8.6.18",
    "tcl9.0": "9.0.4",
    "tcl9.1": "9.1.0",
    "jim": "Jim",
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def assignments(values: list[str]) -> dict[str, Path]:
    selected: dict[str, Path] = {}
    for value in values:
        provider, separator, path = value.partition("=")
        if not separator or provider not in PROVIDERS or not path:
            raise ValueError(f"expected provider-id=path, got {value!r}")
        if provider in selected:
            raise ValueError(f"duplicate provider {provider}")
        selected[provider] = Path(path).resolve(strict=True)
    return selected


def original_file(corpus: Path, row: dict, field: str) -> bytes:
    relative = Path(row[field])
    path = (corpus / relative).resolve(strict=True)
    if relative.is_absolute() or not path.is_relative_to(corpus.resolve()):
        raise ValueError(f"fixture outside corpus: {relative}")
    data = path.read_bytes()
    if sha256(data) != row[field + "_sha256"]:
        raise ValueError(f"retained {field} hash mismatch: {path}")
    return data


def execute(binary: Path, corpus: Path, row: dict, environment: dict[str, str]) -> dict:
    source = original_file(corpus, row, "source")
    expected_stdout = original_file(corpus, row, "stdout")
    expected_stderr = original_file(corpus, row, "stderr")
    command = [str(binary), str(corpus / row["source"])]
    started = time.monotonic()
    timed_out = False
    try:
        process = subprocess.run(
            command,
            env=environment,
            capture_output=True,
            timeout=row["timeout_seconds"],
            check=False,
        )
        status, stdout, stderr = process.returncode, process.stdout, process.stderr
    except subprocess.TimeoutExpired as error:
        timed_out = True
        status, stdout, stderr = None, error.stdout or b"", error.stderr or b""
    return {
        "case": row["case"],
        "source": row["source"],
        "source_sha256": sha256(source),
        "command": command,
        "exit_status": status,
        "timeout": timed_out,
        "seconds": time.monotonic() - started,
        "stdout_hex": stdout.hex(),
        "stdout_sha256": sha256(stdout),
        "stderr_hex": stderr.hex(),
        "stderr_sha256": sha256(stderr),
        "matches": status == row["exit_status"]
        and timed_out == row["timeout"]
        and stdout == expected_stdout
        and stderr == expected_stderr,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--provider", action="append", default=[], metavar="ID=EXECUTABLE"
    )
    parser.add_argument(
        "--library", action="append", default=[], metavar="ID=DIRECTORY"
    )
    parser.add_argument(
        "--case", help="one retained fixed control; omitted means all controls"
    )
    parser.add_argument(
        "--corpus",
        type=Path,
        default=Path(__file__).resolve().parents[2]
        / "rust/tcl-syntax/tests/data/native_naming_corners",
    )
    parser.add_argument(
        "--output", type=Path, required=True, help="new independent JSON receipt"
    )
    parser.add_argument(
        "--verify-only",
        action="store_true",
        help="check retained files without native execution",
    )
    args = parser.parse_args()
    try:
        providers, libraries = assignments(args.provider), assignments(args.library)
        corpus = args.corpus.resolve(strict=True)
        manifest_bytes = (corpus / "manifest.json").read_bytes()
        manifest = json.loads(manifest_bytes)
        cases = {row["case"] for row in manifest["rows"]}
        if args.case is not None and args.case not in cases:
            raise ValueError(f"unknown retained case {args.case!r}")
        if not args.verify_only and not providers:
            raise ValueError("native replay requires at least one explicit --provider")
        if set(libraries) - set(providers):
            raise ValueError("--library requires a matching explicit --provider")
        rows = manifest["engines"] + manifest["rows"]
        for row in rows:
            for field in ("source", "stdout", "stderr"):
                original_file(corpus, row, field)
        receipt = {
            "schema_version": 1,
            "purpose": "Exact native CLI process and result-stream comparison; no Rust execution or physical-object grant",
            "manifest_sha256": sha256(manifest_bytes),
            "verified_original_rows": len(rows),
            "verify_only": args.verify_only,
            "providers": [],
        }
        failed = False
        for provider, binary in providers.items():
            engine = PROVIDERS[provider]
            identity = next(
                row for row in manifest["engines"] if row["engine"] == engine
            )
            selected_rows = [
                row
                for row in manifest["rows"]
                if row["engine"] == engine
                and (args.case is None or row["case"] == args.case)
            ]
            actual_binary_sha = sha256(binary.read_bytes())
            if actual_binary_sha != identity["binary_sha256"]:
                raise ValueError(
                    f"{provider} executable SHA256 differs from the retained build"
                )
            if any(row["binary_sha256"] != actual_binary_sha for row in selected_rows):
                raise ValueError(
                    f"{provider} controls retain a different executable build"
                )
            if not os.access(binary, os.X_OK):
                raise ValueError(f"{provider} executable is not executable")
            if provider != "jim" and provider not in libraries:
                raise ValueError(
                    f"{provider} needs its explicit matching native --library directory"
                )
            environment = os.environ.copy()
            for name in ("TCL_LIBRARY", "TCLLIBPATH", "JIMLIB"):
                environment.pop(name, None)
            startup = {}
            if provider in libraries:
                library = libraries[provider]
                if not library.is_dir():
                    raise ValueError(f"{provider} library is not a directory")
                variable = "JIMLIB" if provider == "jim" else "TCL_LIBRARY"
                environment[variable] = str(library)
                startup[variable] = str(library)
                init = library / "init.tcl"
                if init.is_file():
                    startup["init_tcl_sha256"] = sha256(init.read_bytes())
            record = {
                "id": provider,
                "binary": str(binary),
                "binary_sha256": actual_binary_sha,
                "startup": startup,
                "startup_correspondence": "Explicit selected library; original library tree hashes were not captured. Exact identity and output comparison remain required.",
                "rows": [],
            }
            receipt["providers"].append(record)
            if args.verify_only:
                continue
            observed_identity = execute(binary, corpus, identity, environment)
            record["identity"] = observed_identity
            if not observed_identity["matches"]:
                failed = True
                continue
            for row in selected_rows:
                observed = execute(binary, corpus, row, environment)
                record["rows"].append(observed)
                failed |= not observed["matches"]
        receipt["passed"] = not failed
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with args.output.open("x", encoding="utf-8") as destination:
            json.dump(receipt, destination, indent=2)
            destination.write("\n")
        print(
            f"verified {len(rows)} retained rows; native comparison {'failed' if failed else 'passed' if providers and not args.verify_only else 'not executed'}"
        )
        return 1 if failed else 0
    except (OSError, ValueError, KeyError, StopIteration) as error:
        parser.error(str(error))
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
