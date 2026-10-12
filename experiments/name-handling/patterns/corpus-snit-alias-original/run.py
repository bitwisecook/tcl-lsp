#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Measure one fixed Snit/child-alias composition on retained native shells."""
import concurrent.futures
import datetime
import gzip
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
CORPUS = ROOT / "tmp/name-pattern-corpus/tcltk/tcllib"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def capture(provider):
    name, binary, library = provider
    dest = HERE / "results" / name
    dest.mkdir(parents=True, exist_ok=False)
    command = [str(binary), str(HERE / "probe.tcl"), str(CORPUS)]
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    before = time.monotonic()
    if not binary.is_file():
        return {"provider": name, "outcome": "shell-unavailable", "started_utc": started}
    environment = dict(os.environ, LC_ALL="C")
    if library is not None:
        environment["TCL_LIBRARY"] = str(library)
    with tempfile.TemporaryDirectory(prefix="snit-alias-oracle-") as work:
        try:
            process = subprocess.run(command, cwd=work, env=environment,
                                     capture_output=True, timeout=30, check=False)
            stdout, stderr, code, timeout = process.stdout, process.stderr, process.returncode, False
        except subprocess.TimeoutExpired as exc:
            stdout, stderr, code, timeout = exc.stdout or b"", exc.stderr or b"", None, True
    (dest / "stdout.txt").write_bytes(stdout)
    (dest / "stderr.txt").write_bytes(stderr)
    fields = {}
    for line in stdout.decode("utf-8", errors="replace").splitlines():
        if line.startswith("FIELD "):
            _, key, value = line.split(" ", 2)
            fields[key] = bytes.fromhex(value).decode("utf-8", errors="replace")
    if timeout or code != 0 or "setup_code" not in fields:
        outcome = "capture-failure"
    elif fields["setup_code"] != "0":
        outcome = "framework-setup-refusal"
    elif stderr:
        outcome = "capture-failure"
    elif (fields.get("install_code"), fields.get("prefix_equal"),
          fields.get("positive_code"), fields.get("positive_result"),
          fields.get("change_code"), fields.get("changed_prefix_equal"),
          fields.get("prefix_changed"), fields.get("changed_code")) == (
              "0", "1", "0", "fixed {captured arg} {arg tail}", "0", "1", "1", "1"):
        outcome = "fixed-positive-and-changed-prefix-observed"
    else:
        outcome = "composition-refusal-or-different-result"
    row = {"provider": name, "outcome": outcome, "command": command,
           "binary_sha256": digest(binary), "started_utc": started,
           "elapsed_seconds": time.monotonic() - before, "process_exit_code": code,
           "timed_out": timeout, "fields": fields,
           "stdout_sha256": digest(dest / "stdout.txt"),
           "stderr_sha256": digest(dest / "stderr.txt")}
    (dest / "result.json").write_text(json.dumps(row, indent=2) + "\n")
    return row


def main():
    source_paths = ["modules/interp/interp.tcl", "modules/interp/pkgIndex.tcl",
                    "modules/snit/pkgIndex.tcl", "modules/snit/snit2.tcl",
                    "modules/snit/main2.tcl", "modules/snit/validate.tcl",
                    "modules/snit/snit.tcl", "modules/snit/main1.tcl"]
    before = {path: digest(CORPUS / path) for path in source_paths}
    provenance = json.loads((HERE.parent / "corpus-provenance.json").read_text())
    repo = next(r for r in provenance["public_repositories"]
                if r["group"] == "tcltk" and r["name"] == "tcllib")
    assert repo["revision"] == "8a2e288fa02363639784c2f0a8ea6245896c2884"
    (HERE / "original-interp.tcl.gz").write_bytes(
        gzip.compress((CORPUS / "modules/interp/interp.tcl").read_bytes(), mtime=0))
    providers = []
    previous = json.loads((HERE.parent / "results/provenance.json").read_text())
    for ref in previous["references"]:
        tree = ROOT / "tmp" / ("tcl" + ref["patchlevel"])
        assert digest(tree / "unix/tclsh") == ref["binary_sha256"]
        providers.append(("tcl" + ref["patchlevel"], tree / "unix/tclsh", tree / "library"))
    jim = ROOT / "tmp/jimtcl-oracle/jimsh"
    jim_receipt = json.loads((HERE.parent / "jim-results-5bac7c9/provenance.json").read_text())
    assert digest(jim) == jim_receipt["binary_sha256"]
    providers.append(("jim-5bac7c9", jim, None))
    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as executor:
        rows = list(executor.map(capture, providers))
    after = {path: digest(CORPUS / path) for path in source_paths}
    result = {"scope": "One fixed authored composition over unmodified real packages; no exact original application/framework parity claim",
              "corpus_repository": repo, "package_sources_before": before,
              "package_sources_after": after, "package_sources_unchanged": before == after,
              "jim_revision": jim_receipt["revision"],
              "input_sha256": digest(HERE / "probe.tcl"),
              "runner_sha256": digest(Path(__file__)), "providers": rows,
              "F5": "pending; no native framework execution"}
    (HERE / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    for row in rows:
        print(row["provider"], row["outcome"], row.get("fields", {}).get("setup_result", ""))


if __name__ == "__main__":
    main()
