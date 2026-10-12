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

"""Retain exact authored caller-frame/capture inputs on native C Tcl and pinned Jim."""

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def literal(value):
    return "[binary format H* " + value.encode().hex() + "]" if value else "{}"


def driver(source, support, modern):
    setup, question = source.rstrip().rsplit("\n", 1)
    return support + """
set background_errors {}
proc bgerror {message} {
    set code {}
    set detail {}
    if {[info exists ::errorCode]} {set code $::errorCode}
    if {[info exists ::errorInfo]} {set detail $::errorInfo}
    lappend ::background_errors [list $message $code $detail]
}
set setup __SETUP__
set question __QUESTION__
set modern __MODERN__
set setup_options {}
set question_options {}
set setup_error_code {}
set setup_error_info {}
set question_error_code {}
set question_error_info {}
if {$modern} {
    set setup_code [catch $setup setup_result setup_options]
} else {
    set setup_code [catch $setup setup_result]
}
if {$setup_code && [info exists ::errorCode]} {set setup_error_code $::errorCode}
if {$setup_code && [info exists ::errorInfo]} {set setup_error_info $::errorInfo}
set question_code not-run
set question_result {}
if {!$setup_code} {
    if {$modern} {
        set question_code [catch $question question_result question_options]
    } else {
        set question_code [catch $question question_result]
    }
    if {$question_code && [info exists ::errorCode]} {set question_error_code $::errorCode}
    if {$question_code && [info exists ::errorInfo]} {set question_error_info $::errorInfo}
}
foreach field {setup_code setup_result setup_options setup_error_code setup_error_info question_code question_result question_options question_error_code question_error_info background_errors} {
    binary scan [set $field] H* bytes
    puts [list FIELD $field $bytes]
}
""".replace("__SETUP__", literal(setup)).replace("__QUESTION__", literal(question)).replace("__MODERN__", "1" if modern else "0")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dest", type=Path, required=True)
    parser.add_argument("--provider", choices=["all", "jim"], default="all")
    args = parser.parse_args()
    output = args.dest.resolve()
    output.mkdir(exist_ok=False)
    references = []
    retained_jim = json.loads((HERE.parent / "jim-results-5bac7c9/provenance.json").read_text())
    providers = []
    for line in (ROOT / "rust/tcl-dialect/data/reference-toolchains.tsv").read_text().splitlines():
        if not line or line.startswith("#"):
            continue
        release, patchlevel, tag = line.split("\t")
        tree = ROOT / "tmp" / ("tcl" + patchlevel)
        providers.append(("tcl" + patchlevel, tree / "unix/tclsh", tree, {"TCL_LIBRARY": str(tree / "library")}, ""))
    tree = ROOT / "tmp/jimtcl-oracle"
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=tree, text=True).strip()
    if revision != retained_jim["revision"] or digest(tree / "jimsh") != retained_jim["binary_sha256"]:
        raise RuntimeError("Jim does not match retained upstream receipt")
    providers.append(("jim-5bac7c9", tree / "jimsh", tree, {}, "source " + literal(str(tree / "tests/testing.tcl")) + "\n"))
    rows = []
    for provider, binary, tree, selectors, support in providers:
        if args.provider == "jim" and not provider.startswith("jim"):
            continue
        env = dict(os.environ, LC_ALL="C")
        for key in ("TCL_LIBRARY", "TCLLIBPATH"):
            env.pop(key, None)
        env.update(selectors)
        startup = support + "puts [list PATCHLEVEL [info patchlevel]]\n"
        with tempfile.TemporaryDirectory(prefix="composed-callback-startup-") as work:
            startup_file = Path(work) / "startup.tcl"
            startup_file.write_text(startup)
            preflight = subprocess.run([str(binary), str(startup_file)], cwd=work, env=env, capture_output=True, timeout=15)
        dest = output / provider
        dest.mkdir()
        (dest / "startup.tcl").write_text(startup)
        (dest / "startup.stdout.txt").write_bytes(preflight.stdout)
        (dest / "startup.stderr.txt").write_bytes(preflight.stderr)
        reference = {"provider": provider, "binary": str(binary), "binary_sha256": digest(binary), "selectors": dict(selectors, LC_ALL="C"), "startup_exit_code": preflight.returncode, "startup_stdout": preflight.stdout.decode(), "startup_stderr": preflight.stderr.decode(), "startup_sha256": digest(dest / "startup.tcl")}
        if provider.startswith("tcl"):
            for name in ["library/init.tcl", "generic/tclNamesp.c"]:
                reference[name + "_sha256"] = digest(tree / name)
            expected = provider[3:]
            if preflight.returncode or preflight.stdout.decode().strip() != "PATCHLEVEL " + expected:
                raise RuntimeError("C startup mismatch: " + provider)
        else:
            reference.update(revision=revision, describe=retained_jim["describe"], upstream_head_check_scope="reuses prior retained check; no new remote HEAD assertion")
            for name in ["initjimsh.tcl", "_initjimsh.c", "tests/testing.tcl", "tcltest.tcl", "jim-config.h", "jimautoconf.h", "Makefile"]:
                reference[name + "_sha256"] = digest(tree / name)
            if preflight.returncode or preflight.stderr:
                raise RuntimeError("Jim native startup failed")
        references.append(reference)
        for probe in sorted(HERE.glob("*.tcl")):
            capture = dest / probe.stem
            capture.mkdir()
            source = probe.read_text()
            script = driver(source, support, provider not in {"tcl8.4.20"})
            (capture / "source.tcl").write_text(source)
            (capture / "input.tcl").write_text(script)
            with tempfile.TemporaryDirectory(prefix="frame-capture-prefix-") as work:
                try:
                    result = subprocess.run([str(binary), str(capture / "input.tcl")], cwd=work, env=env, capture_output=True, timeout=15)
                    stdout, stderr, exit_code, timed_out = result.stdout, result.stderr, result.returncode, False
                except subprocess.TimeoutExpired as exc:
                    stdout, stderr, exit_code, timed_out = exc.stdout or b"", exc.stderr or b"", None, True
            (capture / "stdout.txt").write_bytes(stdout)
            (capture / "stderr.txt").write_bytes(stderr)
            row = {"provider": provider, "case_id": probe.stem, "question": "After exact successful setup, which caller-local value and namespace-cell-existence result does ::writer::caller produce?", "source": str(probe.relative_to(ROOT)), "source_sha256": digest(probe), "input_sha256": digest(capture / "input.tcl"), "stdout_sha256": digest(capture / "stdout.txt"), "stderr_sha256": digest(capture / "stderr.txt"), "process_exit_code": exit_code, "timed_out": timed_out, "command": [str(binary), str(capture / "input.tcl")], "environment": dict(selectors, LC_ALL="C"), "capture": str(capture.relative_to(ROOT)), "adaptation": "none; exact preceding lines are setup and exact final line is question; driver intercepts bgerror and preserves message/errorCode/errorInfo separately"}
            for line in stdout.decode().splitlines():
                if line.startswith("FIELD "):
                    _, field, data = line.split(" ", 2)
                    row[field] = "" if data == "{}" else bytes.fromhex(data).decode()
            if timed_out:
                row["status"] = "process-timed-out"
            elif exit_code or stderr or "setup_code" not in row:
                row["status"] = "driver-failed"
            elif row["setup_code"] != "0":
                row["status"] = "setup-failed-question-and-callback-unreached"
            elif row["background_errors"]:
                row["status"] = "callback-failed"
            elif row["question_code"] == "0":
                row["status"] = "returned-value"
            else:
                row["status"] = "question-raised-error"
            (capture / "record.json").write_text(json.dumps(row, indent=2) + "\n")
            rows.append(row)
            print(provider, probe.stem, row["status"], row.get("setup_result"), row.get("question_result"))
    (output / "records.json").write_text(json.dumps(rows, indent=2) + "\n")
    provenance = {"captured_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(), "subject_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(), "runner_sha256": digest(Path(__file__)), "references": references, "case_count": len(rows), "F5": "unanswered; no device access; see F5-REQUEST.md", "semantic_scope": "authored caller-frame/capture reductions in fresh native processes; final question and asynchronous callback failures retained separately; no static-consumer execution or canonical proof registration; driver selects native catch-options capability by provider rather than C package vcompare"}
    (output / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")


if __name__ == "__main__":
    main()
