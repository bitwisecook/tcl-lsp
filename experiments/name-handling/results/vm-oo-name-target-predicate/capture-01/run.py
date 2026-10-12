#!/usr/bin/env python3
"""Run the two-owner OO observation only in the coordinated source/cache window."""

import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import time


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def save(path, value):
    path.write_text(json.dumps(value, sort_keys=True, indent=2) + "\n")


def snapshot(root):
    names = subprocess.check_output(
        ["rg", "--files", "rust", "runtime/rust", "-g", "*.rs", "-g", "Cargo.toml", "-g", "Cargo.lock"],
        cwd=root, text=True,
    ).splitlines() + ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"]
    return {name: digest(root / name) for name in sorted(set(names)) if (root / name).is_file()}


def differences(before, after):
    return [name for name in sorted(before.keys() | after.keys()) if before.get(name) != after.get(name)]


def terminate_group(process):
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        pass
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    process.wait()


def run(command, root, environment, stdout, stderr, timeout):
    process = subprocess.Popen(command, cwd=root, env=environment, stdout=stdout,
                               stderr=stderr, start_new_session=True)
    try:
        return process.wait(timeout=timeout)
    except BaseException:
        terminate_group(process)
        raise


def interrupted(signum, _frame):
    raise RuntimeError(f"probe interrupted by signal {signum}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", action="store_true", help="execute only after the root's source/cache grant")
    args = parser.parse_args()
    bundle = Path(__file__).resolve().parent
    plan = json.loads((bundle / "plan.json").read_text())
    root = Path(plan["root"])
    out = Path(plan["result_dir"])
    for name, expected in (plan["inspected_source_hashes"] | plan["caller_fixture_hashes"]).items():
        assert digest(root / name) == expected, f"original owner/caller changed: {name}"
    for owner in plan["owners"]:
        assert digest(Path(owner["instrumented_path"])) == owner["instrumented_sha256"]
    if not args.run:
        print("Ready/unexecuted: three owner hashes, two original caller hashes and two proposal copies match.")
        return 0
    assert not (out / "execution-record.json").exists(), "use a fresh capture destination"
    assert os.environ.get("CARGO_HOME") == str(root / "tmp/name-tools/cargo"), "source /tmp/name-env.sh first"
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    environment = os.environ.copy()
    environment.update(CARGO_TARGET_DIR=plan["cache"], TCL_LSP_VM_OO_TARGET_DIAGNOSTIC="1")
    original = {owner["path"]: (root / owner["path"]).read_bytes() for owner in plan["owners"]}
    before = snapshot(root)
    save(out / "source-before.json", before)
    for name, data in original.items():
        (out / (Path(name).parent.name + "--" + Path(name).name + ".before")).write_bytes(data)
    record = {"started_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
              "control": plan["control"], "cache": plan["cache"],
              "environment_recipe": plan["environment_recipe"], "source_scope_count": len(before),
              "temporary_owners": list(original), "proposal_sha256": plan["patch_sha256"],
              "outcome": "not_completed"}
    start = time.monotonic()
    try:
        for owner in plan["owners"]:
            (root / owner["path"]).write_bytes(Path(owner["instrumented_path"]).read_bytes())
        instrumented = snapshot(root)
        save(out / "source-instrumented.json", instrumented)
        assert differences(before, instrumented) == sorted(original), "unexpected source change before build"
        command = ["cargo", "test", "--offline", "-p", "tcl-vm", "--lib", "--no-run", "-j", "2",
                   "--message-format=json-render-diagnostics"]
        record["build_command"] = command
        print("Two exact owners instrumented; truthful offline lib-test rebuild started.", flush=True)
        with (out / "build.jsonl").open("w") as stdout, (out / "build.stderr.txt").open("w") as stderr:
            record["build_exit"] = run(command, root, environment, stdout, stderr, 600)
        record["changed_during_build"] = differences(instrumented, snapshot(root))
        if record["build_exit"] or record["changed_during_build"]:
            record["outcome"] = "build_failed_or_source_changed"
            return 1
        artifacts = []
        for line in (out / "build.jsonl").read_text().splitlines():
            try:
                item = json.loads(line)
            except ValueError:
                continue
            if item.get("reason") == "compiler-artifact" and item.get("target", {}).get("name") == "tcl_vm" and item.get("executable"):
                artifacts.append(item["executable"])
        assert len(artifacts) == 1
        binary = Path(artifacts[0])
        record.update(binary=str(binary), binary_sha256_before=digest(binary))
        with (out / "control-list.log").open("w") as log:
            listed = run([str(binary), plan["control"], "--exact", "--list"], root, environment, log, subprocess.STDOUT, 30)
        assert listed == 0 and (out / "control-list.log").read_text().count(plan["control"] + ": test") == 1
        command = [str(binary), plan["control"], "--exact", "--nocapture", "--test-threads=1"]
        record["control_command"] = command
        with (out / "control.log").open("w") as log:
            record["control_exit"] = run(command, root, environment, log, subprocess.STDOUT, 60)
        text = (out / "control.log").read_text()
        record["exactly_one_test_executed"] = bool(re.search(r"^running 1 test$", text, re.MULTILINE))
        record["first_failed_target_predicate"] = next((line for line in re.findall(r"OO_TARGET [^\r\n]*", text) if "current=false" in line), None)
        record["namespace_failure_terms"] = re.findall(r"OO_NAMESPACE [^\r\n]*", text)
        record["changed_during_control"] = differences(instrumented, snapshot(root))
        record["binary_sha256_after"] = digest(binary)
        record["outcome"] = "measured_control" if record["exactly_one_test_executed"] else "invalid_test_count"
        print(json.dumps({key: record.get(key) for key in ["control_exit", "exactly_one_test_executed", "first_failed_target_predicate", "namespace_failure_terms"]}), flush=True)
        return int(not record["exactly_one_test_executed"] or bool(record["changed_during_control"]))
    except BaseException as error:
        record["exception"] = repr(error)
        record["outcome"] = "interrupted_or_harness_failed"
        raise
    finally:
        for name, data in original.items():
            (root / name).write_bytes(data)
        restored = snapshot(root)
        save(out / "source-restored.json", restored)
        record["restored_owners_byte_exact"] = all((root / name).read_bytes() == data for name, data in original.items())
        record["changed_after_restoration"] = differences(before, restored)
        record["diagnostic_binary_current_for_restored_source"] = False
        record["elapsed_seconds"] = round(time.monotonic() - start, 3)
        record["finished_utc"] = datetime.datetime.now(datetime.timezone.utc).isoformat()
        save(out / "execution-record.json", record)
        print("Temporary owners restored byte-exact: " + str(record["restored_owners_byte_exact"]) + "; other changes: " + str(record["changed_after_restoration"]), flush=True)


if __name__ == "__main__":
    raise SystemExit(main())
