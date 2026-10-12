#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Remove only sealed, provenance-identified staging and inspect owned objects."""

import datetime
import hashlib
import importlib.util
import json
import shlex
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[6]
BASE = ROOT / "experiments/name-handling/patterns/f5/answers"
DEST = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("native_capture", BASE / "nh20261012native1/runner.py")
native = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(native)
native.install_private_transport()


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def command(label, operation):
    result = native.capture.rpc(operation)
    (DEST / (label + ".stdout")).write_bytes(result.stdout)
    (DEST / (label + ".stderr")).write_bytes(result.stderr)
    return {"operation": operation, "exit_code": result.returncode,
            "stdout": label + ".stdout", "stderr": label + ".stderr"}


def main():
    tags = ["repair2", "harness1", "harness2", "collision1", "nativecap1",
            "aplrepair1", "native1", "native2"]
    receipts = []
    for suffix in tags:
        tag = "nh20261012" + suffix
        run = BASE / tag
        seal = run / "MANIFEST.sha256"
        entries = seal.read_text().splitlines()
        for entry in entries:
            expected, name = entry.split("  ", 1)
            assert digest(run / name) == expected, (tag, name)
        provenance = run / "provenance.json"
        remote = json.loads(provenance.read_text())["remote_staging"]
        assert remote == "/var/tmp/tcl_lsp_" + tag
        quoted = shlex.quote(remote)
        operation = (f"if test -L {quoted}; then exit 9; fi; "
                     f"if test -d {quoted}; then find {quoted} -type f; fi; "
                     f"rm -rf -- {quoted}; test ! -e {quoted}")
        receipt = command(tag, operation)
        receipt.update({"run_id": tag, "provenance": str(provenance.relative_to(ROOT)),
                        "provenance_sha256": digest(provenance),
                        "seal": str(seal.relative_to(ROOT)), "seal_sha256": digest(seal),
                        "sealed_files_verified": len(entries), "remote_staging": remote,
                        "staging_absent_after": receipt["exit_code"] == 0})
        receipts.append(receipt)
        assert receipt["exit_code"] == 0
    inventory = []
    for kind in ["ltm rule", "ltm virtual", "ltm pool", "sys application template",
                 "sys application service", "cli script", "sys icall script",
                 "sys icall handler periodic", "sys folder", "auth partition"]:
        operation = ("nh_listing=$(tmsh list " + kind + "); nh_exit=$?; "
                     'printf "listing-exit=%s\\n" "$nh_exit"; '
                     'if [ "$nh_exit" = 0 ]; then printf "%s\\n" "$nh_listing" | '
                     "awk '/__tcl_lsp_probe_nh20261012|NH_nh20261012/ {print}'; fi; "
                     'exit "$nh_exit"')
        receipt = command(kind.replace(" ", "-"), operation)
        stdout = (DEST / receipt["stdout"]).read_bytes()
        receipt.update({"kind": kind, "owned_prefix_absent":
                        receipt["exit_code"] == 0 and stdout == b"listing-exit=0\n"})
        inventory.append(receipt)
        assert receipt["owned_prefix_absent"], kind
    value = {"captured_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
             "configuration_saved": False, "staging": receipts,
             "configuration_inventory": inventory,
             "transport_configuration": "omitted",
             "scope": "Exact sealed staging and matching prefixes in ten listings; runtime and native catalogue retirement are not inferred."}
    (DEST / "receipts.json").write_text(json.dumps(value, indent=2) + "\n")
    print(json.dumps({"staging_paths_absent": len(receipts),
                      "configuration_listing_types": len(inventory)}))


if __name__ == "__main__":
    main()
