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

"""Retain isolated BIG-IP naming inputs, applied configuration and raw evidence."""

import argparse
import datetime
import json
import re
import shlex
from pathlib import Path
import capture


def build_case(tag):
    if not re.fullmatch(r"[a-z0-9]{6,24}", tag):
        raise ValueError("Invalid run tag")
    ns = "::__tcl_lsp_probe_" + tag + "_namespace"
    tail = "__tcl_lsp_probe_" + tag + "_target"
    setup = f"""set nh_owner_n namespace
set nh_owner_scope [$nh_owner_n current]
set nh_owner_candidates [list {ns}::{tail} [string trimright $nh_owner_scope :]::{tail} ::{tail}]
if {{[$nh_owner_n exists {ns}] || [llength [info commands {ns}]]}} {{error {{namespace ownership collision}}}}
foreach nh_owner_head $nh_owner_candidates {{if {{[llength [info commands $nh_owner_head]] || [llength [info procs $nh_owner_head]]}} {{error [list command ownership collision $nh_owner_head]}}}}
list absent execution-namespace $nh_owner_scope candidates $nh_owner_candidates
"""
    operation = f"""set nh_owner_inside [namespace eval {ns} {{
proc {tail} {{}} {{return TARGET}}
namespace current
}}]
set nh_owner_rows {{}}
foreach nh_owner_head $nh_owner_candidates {{
set nh_owner_code [catch {{eval [list $nh_owner_head]}} nh_owner_value]
set nh_owner_body_code [catch {{info body $nh_owner_head}} nh_owner_body]
lappend nh_owner_rows [list head $nh_owner_head commands [info commands $nh_owner_head] procs [info procs $nh_owner_head] invocation $nh_owner_code $nh_owner_value body $nh_owner_body_code $nh_owner_body]
}}
list execution-namespace $nh_owner_scope inside-namespace $nh_owner_inside definitions $nh_owner_rows
"""
    cleanup = f"""if {{$nh_setup_code != 0}} {{return {{ownership-not-acquired}}}}
foreach nh_owner_head $nh_owner_candidates {{if {{[llength [info commands $nh_owner_head]]}} {{rename $nh_owner_head {{}}}}}}
if {{[namespace exists {ns}]}} {{namespace delete {ns}}}
set nh_owner_remaining {{}}
foreach nh_owner_head $nh_owner_candidates {{
set nh_owner_after_code [catch {{eval [list $nh_owner_head]}} nh_owner_after_value]
lappend nh_owner_remaining [list head $nh_owner_head commands [info commands $nh_owner_head] procs [info procs $nh_owner_head] invocation $nh_owner_after_code $nh_owner_after_value]
}}
list namespace [namespace exists {ns}] candidates $nh_owner_remaining
"""
    return {
        "id": "namespace-definition-owner",
        "case_id": "namespace-definition-owner",
        "question_id": "F5-N02",
        "variant_id": "unique-namespace-definition-owner-v1",
        "setup": setup,
        "operation": operation,
        "cleanup": cleanup,
        "scope": "Unique namespace/command-tail control for namespace eval definition ownership in the actual wrapper frame; requested namespace, caller namespace and global candidates checked separately. It supplies no answer to an unchanged original corpus program.",
        "adaptation": {"kind": "unique-name-definition-owner-control"},
        "exact_source": False,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", required=True)
    args = parser.parse_args()
    tag = args.run
    item = build_case(tag)
    capture.TAG = tag
    dest = capture.ROOT / "experiments/name-handling/f5/answers" / tag
    dest.mkdir(exist_ok=False)
    for source in [Path(__file__), Path(capture.__file__)]:
        (dest / source.name).write_bytes(source.read_bytes())
    remote = "/var/tmp/tcl_lsp_" + tag
    check = capture.rpc(
        "test ! -e " + shlex.quote(remote) + " && mkdir -m 700 " + shlex.quote(remote)
    )
    if check.returncode:
        raise RuntimeError("Staging collision")
    (dest / "provenance.json").write_text(
        json.dumps(
            {
                "run_id": tag,
                "remote_staging": remote,
                "configuration_saved": False,
                "timestamp_utc": datetime.datetime.now(
                    datetime.timezone.utc
                ).isoformat(),
            },
            indent=2,
        )
        + "\n"
    )
    for label, command in [
        ("version", "tmsh show sys version"),
        ("platform", "tmsh show sys hardware"),
        ("tmm-roster", "tmsh show sys tmm-info"),
    ]:
        receipt = capture.rpc(command)
        (dest / (label + ".stdout")).write_bytes(receipt.stdout)
        (dest / (label + ".stderr")).write_bytes(receipt.stderr)
        (dest / (label + ".exit")).write_text(str(receipt.returncode) + "\n")
    for index, context in enumerate(
        [
            "stock-8.4",
            "stock-8.5",
            "tmsh-cli-script",
            "iapp-implementation",
            "icall",
            "tmm-RULE_INIT",
            "tmm-procedure",
            "tmm-traffic-event",
        ]
    ):
        selected = {**item, "context": context}
        (dest / (context + "-cases.json")).write_text(
            json.dumps([selected], indent=2) + "\n"
        )
        capture.run_job(
            dest / (context + "-0"),
            remote,
            context,
            "__tcl_lsp_probe_" + tag + "_" + str(index),
            capture.body([selected], context, tag),
            wait=5,
        )
    final = capture.rpc(
        "grep -F "
        + shlex.quote("NHO|" + tag + "|")
        + " /var/tmp/scriptd.out /var/log/ltm"
    )
    (dest / "final-runtime.stdout").write_bytes(final.stdout)
    (dest / "final-runtime.stderr").write_bytes(final.stderr)
    (dest / "final-runtime.exit").write_text(str(final.returncode) + "\n")
    (dest / "capture-complete.json").write_text(
        json.dumps({"completed": True, "configuration_saved": False}) + "\n"
    )


if __name__ == "__main__":
    main()
