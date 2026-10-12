#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Replay pinned original source controls; keep guest outcomes separate from transport."""

import argparse
import fcntl
import hashlib
import json
import subprocess
from contextlib import contextmanager
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "rust"
SOURCES = {
    "naming.callback.lsort-invoking-frame": (
        "tcl-registry/tests/data/native_callback_lookup_scope",
        "scope.tcl",
        None,
        "receipt.json",
    ),
    "naming.callback.after-global-frame": (
        "tcl-registry/tests/data/native_callback_lookup_scope",
        "scope.tcl",
        None,
        "receipt.json",
    ),
    "naming.callback.trace-trigger-frame": (
        "tcl-registry/tests/data/native_callback_lookup_scope",
        "scope.tcl",
        None,
        "receipt.json",
    ),
    "naming.quoted-tmm-operator-value": (
        "tcl-lexer/tests/data/native_quoted_word_geometry",
        "probe.tcl",
        "{provider}.value.txt",
        "manifest.json",
    ),
    "naming.nested-expression-syntax-failure-order": (
        "tcl-syntax/tests/data/native_nested_expression_syntax",
        "nested-syntax.tcl",
        "{provider}.txt",
        "manifest.json",
    ),
    "naming.entered-child-source-completion": (
        "tcl-vm/tests/data/native_child_completion",
        "probe.tcl",
        "{provider}.tsv",
        "manifest.json",
    ),
    "naming.expression-result-normalisation-effects": (
        "tcl-registry/tests/data/native_conditional_expression_results",
        "normalisation.tcl",
        "normalisation-{provider}.txt",
        "normalisation-manifest.json",
    ),
    "naming.expression-abs-handler-replacement": (
        "tcl-registry/tests/data/native_conditional_expression_results",
        "math_binding.tcl",
        "math-binding-{provider}.txt",
        "math-binding-manifest.json",
    ),
    "naming.return-sole-result-option-spellings": (
        "tcl-registry/tests/data/native_return_single_result",
        "source.tcl",
        None,
        "manifest.json",
    ),
    "naming.return-option-pairs-release-shape": (
        "tcl-registry/tests/data/native_return_single_result",
        "option_pair_source.tcl",
        None,
        "option_pair_manifest.json",
    ),
    "naming.tcl84-authored-math-function-surface": (
        "tcl-registry/tests/data/authored_tcl84_math_functions",
        "source.tcl",
        "native84.txt",
        "manifest.json",
    ),
    "naming.tcl84-function-child-syntax-order": (
        "tcl-registry/tests/data/authored_tcl84_math_functions",
        "child-syntax.tcl",
        "child-syntax84.txt",
        "child-syntax-manifest.json",
    ),
    "naming.tcl84-random-and-precision-controls": (
        "tcl-registry/tests/data/authored_tcl84_math_functions",
        "format-random.tcl",
        "format-random84.txt",
        "format-random-manifest.json",
    ),
    "naming.concat-malformed-list-result": (
        "tcl-cmd-core/tests/data/native_concat",
        "malformed.tcl",
        None,
        "malformed-manifest.json",
    ),
    "naming.mathop-procedure-source-controls": (
        "tcl-cmd-core/tests/data/native_mathop_compilation",
        None,
        None,
        "manifest.json",
    ),
    "naming.mathop-binding-and-written-head-controls": (
        "tcl-cmd-core/tests/data/native_mathop_identity",
        None,
        None,
        "manifest.json",
    ),
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def provider_id(value):
    value = value.lower()
    if "jim" in value or value.startswith("0.84"):
        return "jim"
    for name in ["8.4", "8.5", "8.6", "9.0", "9.1"]:
        if name in value:
            return name
    return None


@contextmanager
def slot(output):
    # Serial replay acquires a shared advisory slot as well. A busy pair fails
    # explicitly so this driver cannot exceed the unchanged global limit.
    directory = ROOT.parent / ".proofs" / "native-replay-slots"
    directory.mkdir(parents=True, exist_ok=True)
    for number in range(2):
        with (directory / str(number)).open("a") as stream:
            try:
                fcntl.flock(stream, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                continue
            yield
            return
    raise RuntimeError(
        "Both native replay slots are busy; retry after an active run finishes."
    )


def cases(question, provider):
    directory, source_file, expected_file, manifest_file = SOURCES[question]
    directory = DATA / directory
    manifest = json.loads((directory / manifest_file).read_bytes())
    if question.startswith("naming.tcl84-") and provider != "8.4":
        raise ValueError("This question has only a retained Tcl8.4 observation.")
    if source_file is None:
        captures = [
            row
            for row in manifest["captures"]
            if provider_id(row["engine"]) == provider
        ]
        for row in captures:
            # Exact wrapper from the original producer, around the retained
            # source string. No parsing or reconstruction of its operands.
            wrapper = (
                "set code [catch {"
                + row["source"]
                + "} result]; binary scan $result H* bytes; puts [list $code $bytes]\n"
            )
            yield (
                str(row["case"]),
                wrapper.encode(),
                row["stdout"].encode(),
                row["stderr"].encode(),
                row["exit"],
            )
        return
    source = (directory / source_file).read_bytes()
    if question.startswith("naming.callback."):
        row = next(
            row
            for row in manifest["captures"]
            if provider_id(row["provider"]) == provider
        )
        yield (
            source_file,
            source,
            row["stdout"].encode(),
            row["stderr"].encode(),
            row["returncode"],
        )
        return
    if expected_file:
        expected = (directory / expected_file.format(provider=provider)).read_bytes()
        error_file = directory / (provider + ".stderr")
        error = error_file.read_bytes() if error_file.is_file() else b""
        yield source_file, source, expected, error, 0
        return
    rows = manifest if isinstance(manifest, list) else manifest["rows"]
    matching = [row for row in rows if provider_id(row["version"]) == provider]
    if len(matching) != 1:
        raise ValueError("Exactly one original source/output row is required.")
    row = matching[0]
    yield (
        source_file,
        source,
        row.get("stdout", row.get("output")).encode(),
        row.get("stderr", row.get("error", "")).encode(),
        row["exit"],
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--proof", choices=sorted(SOURCES), required=True)
    parser.add_argument(
        "--provider", choices=["8.4", "8.5", "8.6", "9.0", "9.1", "jim"], required=True
    )
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    records = json.loads(
        (
            ROOT
            / "docs/design/analysis/name-resolution-proofs/grammar-native-records.json"
        ).read_bytes()
    )
    records.extend(
        json.loads(
            (
                ROOT
                / "docs/design/analysis/name-resolution-proofs/grammar-callback-scope-records.json"
            ).read_bytes()
        )
    )
    record = next(row for row in records if row["id"] == args.proof)
    for evidence in record["evidence"]:
        if digest((ROOT / evidence["path"]).read_bytes()) != evidence["sha256"]:
            parser.error("Retained evidence bytes changed: " + evidence["path"])
    executable = args.executable.resolve(strict=True)
    output = args.output.resolve()
    if output == ROOT or ROOT in output.parents:
        parser.error(
            "Replay output must be outside the repository; retained captures are immutable."
        )
    output.mkdir(parents=True, exist_ok=True)
    with slot(output):
        version = subprocess.run(
            [str(executable)],
            input=b"puts [info patchlevel]\n",
            capture_output=True,
            timeout=60,
            check=False,
        )
    if (
        version.returncode
        or version.stderr
        or provider_id(version.stdout.decode()) != args.provider
    ):
        parser.error(
            "The independently launched provider reports another version or fails startup."
        )
    rows = []
    for case, source, expected, expected_error, expected_exit in cases(
        args.proof, args.provider
    ):
        with slot(output):
            result = subprocess.run(
                [str(executable)],
                input=source,
                capture_output=True,
                timeout=60,
                check=False,
            )
        stem = str(len(rows))
        (output / (stem + ".input")).write_bytes(source)
        (output / (stem + ".stdout")).write_bytes(result.stdout)
        (output / (stem + ".stderr")).write_bytes(result.stderr)
        rows.append(
            dict(
                case=case,
                source_sha256=digest(source),
                exit=result.returncode,
                stdout_sha256=digest(result.stdout),
                stderr_sha256=digest(result.stderr),
                matches=(
                    result.returncode == expected_exit
                    and result.stdout == expected
                    and result.stderr == expected_error
                ),
            )
        )
    if not rows:
        parser.error("No retained observations exist for this exact provider/question.")
    receipt = dict(
        proof=args.proof,
        provider=args.provider,
        reported_patchlevel=version.stdout.decode().strip(),
        executable=str(executable),
        executable_sha256=digest(executable.read_bytes()),
        timeout_seconds=60,
        global_slots=2,
        rows=rows,
    )
    (output / "receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
    print(
        json.dumps(dict(cases=len(rows), matches=sum(row["matches"] for row in rows)))
    )
    return int(any(not row["matches"] for row in rows))


if __name__ == "__main__":
    raise SystemExit(main())
