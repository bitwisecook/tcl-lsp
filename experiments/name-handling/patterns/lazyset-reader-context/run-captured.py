"""Capture discrete lazyset callers and separate trace controls on stock shells."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tcl(value):
    return "[binary format H* " + value.encode().hex() + "]"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dest", type=Path, required=True)
    args = parser.parse_args()
    dest = args.dest.resolve()
    dest.mkdir(exist_ok=False)
    providers = [("tcl" + v, ROOT / "tmp" / ("tcl" + v), "unix/tclsh", v != "8.4.20")
                 for v in ("8.4.20", "8.5.19", "8.6.18", "9.0.4", "9.1.0")]
    providers.append(("jim-5bac7c9", ROOT / "tmp/jimtcl-oracle", "jimsh", True))
    jim_pin = json.loads((HERE.parent / "jim-results-5bac7c9/provenance.json").read_text())
    records = []
    receipts = []
    for provider, tree, binary_path, options in providers:
        binary = tree / binary_path
        work = dest / provider
        work.mkdir()
        shutil.copytree(HERE / "upstream", work / "upstream")
        shutil.copytree(HERE / "probes", work / "probes")
        env = dict(os.environ, LC_ALL="C")
        for key in ("TCL_LIBRARY", "TCLLIBPATH"):
            env.pop(key, None)
        selectors = {"LC_ALL": "C"}
        if provider.startswith("tcl"):
            selectors["TCL_LIBRARY"] = str(tree / "library")
            env.update(selectors)
        elif sha(binary) != jim_pin["binary_sha256"]:
            raise RuntimeError("Jim binary does not match retained upstream capture")
        startup = work / "startup.tcl"
        startup.write_text("puts [list PATCHLEVEL [info patchlevel]]\nputs [list LIBRARY [info library]]\n")
        r = subprocess.run([str(binary), str(startup)], cwd=work, env=env, capture_output=True, timeout=15)
        (work / "startup.stdout.txt").write_bytes(r.stdout)
        (work / "startup.stderr.txt").write_bytes(r.stderr)
        receipt = {"provider": provider, "binary": str(binary), "binary_sha256": sha(binary),
                   "environment_selectors": selectors, "startup_exit_code": r.returncode,
                   "startup_stdout": r.stdout.decode(), "startup_stderr": r.stderr.decode(),
                   "startup_sha256": sha(startup)}
        if provider.startswith("tcl"):
            for name in ("library/init.tcl", "generic/tclNamesp.c", "generic/tclTrace.c"):
                receipt[name + "_sha256"] = sha(tree / name) if (tree / name).exists() else "absent in this release tree"
            if r.returncode or r.stderr or not r.stdout.decode().startswith("PATCHLEVEL " + provider[3:] + "\n"):
                raise RuntimeError("Startup does not report requested C patchlevel")
        else:
            receipt.update(revision=jim_pin["revision"], describe=jim_pin["describe"],
                           revision_scope="Binary matches retained pinned-Jim provenance; no new remote HEAD assertion")
            for name in ("initjimsh.tcl", "_initjimsh.c", "jim-config.h", "jimautoconf.h", "Makefile"):
                receipt[name + "_sha256"] = sha(tree / name) if (tree / name).exists() else "absent in this release tree"
        receipts.append(receipt)
        for source in sorted((work / "probes").glob("*.tcl")):
            case = source.stem
            capture = work / case
            capture.mkdir()
            body = source.read_text().rstrip()
            setup, question = body.rsplit("\n", 1)
            (capture / "setup.tcl").write_text(setup + "\n")
            (capture / "question.tcl").write_text(question + "\n")
            # Driver lives beside the input to retain its source-relative upstream path.
            driver = work / "probes" / (case + ".driver")
            catcher = "set cc catch\n"
            catcher += "set setup_options {}\nset question_options {}\n"
            catcher += "set setup_code [$cc " + tcl(setup) + " setup_result" + (" setup_options" if options else "") + "]\n"
            catcher += "set setup_error_code {}\nset setup_error_info {}\n"
            catcher += "if {$setup_code && [info exists ::errorCode]} {set setup_error_code $::errorCode}\n"
            catcher += "if {$setup_code && [info exists ::errorInfo]} {set setup_error_info $::errorInfo}\n"
            catcher += "set question_code unreached\nset question_result {}\nset question_error_code {}\nset question_error_info {}\n"
            catcher += "if {!$setup_code} {\nset question_code [$cc " + tcl(question) + " question_result" + (" question_options" if options else "") + "]\n"
            catcher += "if {$question_code && [info exists ::errorCode]} {set question_error_code $::errorCode}\n"
            catcher += "if {$question_code && [info exists ::errorInfo]} {set question_error_info $::errorInfo}\n}\n"
            fields = "setup_code setup_result setup_options setup_error_code setup_error_info question_code question_result question_options question_error_code question_error_info"
            catcher += "foreach key {" + fields + "} {binary scan [set $key] H* hex; puts [list OBS $key $hex]}\n"
            driver.write_text(catcher)
            r = subprocess.run([str(binary), str(driver)], cwd=work, env=env, capture_output=True, timeout=15)
            (capture / "stdout.txt").write_bytes(r.stdout)
            (capture / "stderr.txt").write_bytes(r.stderr)
            row = {"provider": provider, "case_id": case, "source_sha256": sha(source),
                   "source": str(source.relative_to(ROOT)), "driver": str(driver.relative_to(ROOT)),
                   "driver_sha256": sha(driver), "process_exit_code": r.returncode,
                   "stdout_sha256": sha(capture / "stdout.txt"), "stderr_sha256": sha(capture / "stderr.txt"),
                   "command": [str(binary), str(driver)], "environment_selectors": selectors,
                   "capture": str(capture.relative_to(ROOT)), "setup_sha256": sha(capture / "setup.tcl"),
                   "question_sha256": sha(capture / "question.tcl"),
                   "channel": "authored caller around unchanged pinned package" if case.startswith("package") else "separately authored core trace control; no package adaptation"}
            for line in r.stdout.decode().splitlines():
                if line.startswith("OBS "):
                    _, key, value = line.split(" ", 2)
                    row[key] = "" if value == "{}" else bytes.fromhex(value).decode()
            row["status"] = ("driver-failed" if r.returncode or r.stderr or "setup_code" not in row else
                             "setup-refused-question-unreached" if row["setup_code"] != "0" else
                             "returned" if row["question_code"] == "0" else "question-error")
            (capture / "record.json").write_text(json.dumps(row, indent=2) + "\n")
            records.append(row)
            print(provider, case, row["status"], repr(row.get("setup_result")), repr(row.get("question_result")))
    (dest / "records.json").write_text(json.dumps(records, indent=2) + "\n")
    (dest / "provenance.json").write_text(json.dumps({
        "captured_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "runner_sha256": sha(Path(__file__)), "references": receipts, "case_count": len(records),
        "scope": "Fresh native processes; exact preceding lines as setup and final line as question. No Tcltest wrapper, subject-engine capture, static-consumer result or canonical proof registration.",
        "F5": "not run; all native context questions unanswered"}, indent=2) + "\n")


if __name__ == "__main__":
    main()
