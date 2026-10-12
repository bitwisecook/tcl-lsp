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

"""Compare literal loader acceptance with separately retained dynamic controls."""

import argparse
import datetime
import json
from pathlib import Path
import capture


def main():
    p = argparse.ArgumentParser()
    p.add_argument("--run", required=True)
    a = p.parse_args()
    tag = a.run
    capture.TAG = tag
    dest = capture.ROOT / "experiments/name-handling/f5/answers" / tag
    dest.mkdir(parents=True, exist_ok=False)
    remote = "/var/tmp/tcl_lsp_" + tag
    if capture.rpc("test ! -e " + remote + " && mkdir -m 700 " + remote).returncode:
        raise RuntimeError("Staging collision")
    for module in [Path(__file__), Path(capture.__file__)]:
        (dest / module.name).write_bytes(module.read_bytes())
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
    for label, cmd in [
        ("version", "tmsh show sys version"),
        ("platform", "tmsh show sys hardware"),
        ("tmm-roster", "tmsh show sys tmm-info"),
    ]:
        r = capture.rpc(cmd)
        (dest / (label + ".stdout")).write_bytes(r.stdout)
        (dest / (label + ".stderr")).write_bytes(r.stderr)
    for ci, context in enumerate(
        [
            "tmsh-cli-script",
            "iapp-implementation",
            "icall",
            "tmm-RULE_INIT",
            "tmm-procedure",
        ]
    ):
        n = "::__nh_" + tag + str(ci)
        specs = [
            ("namespace-path", "namespace path"),
            (
                "namespace-ensemble",
                "namespace ensemble create -command " + n + "_facade -map {}",
            ),
            ("namespace-upvar", "namespace upvar :: __nh_absent nh_link"),
            ("interp-create", "interp create " + tag + str(ci) + "_child"),
            ("interp-alias", "interp alias {} " + n + "_alias {} list"),
            ("tcltest", "package require tcltest"),
            ("tcloo", "oo::class create " + n + "_class {}"),
            ("catch-options", "catch {list VALUE} result options"),
            ("encoding", "encoding convertto utf-8 VALUE"),
            ("proc-in-body", "proc " + n + "_target {} {return TARGET}"),
            ("missing-command", "__nh_" + tag + "_absent"),
        ]
        records = []
        for index, (cid, op) in enumerate(specs):
            case = dict(
                id="literal-" + cid,
                question_id="F5-N01",
                setup="",
                operation=op,
                variant_id="literal-caught-loader-control-v1",
                scope="Literal source inside catch; load rejection leaves operation unreached. Compare separate dynamic capability probe.",
                context=context,
            )
            records.append(case)
            unit = (
                "[TMM::cmp_group]:[TMM::cmp_unit]"
                if context.startswith("tmm-")
                else "none"
            )
            source = (
                'set nh_options_available unavailable\nset nh_return_options {}\nset nh_invocation once\nset nh_unit "'
                + unit
                + '"\nset nh_setup_code 0\nset nh_setup_result {}\n'
            )
            source += capture.emit(
                context,
                tag,
                case["id"],
                "setup",
                "$nh_setup_code",
                "$nh_setup_result",
                "$nh_unit",
            )
            source += "set nh_code [catch { " + op + " } nh_result]\n"
            source += capture.emit(
                context, tag, case["id"], "lookup", "$nh_code", "$nh_result", "$nh_unit"
            )
            cleanup = (
                "foreach child [interp slaves] {if {[string match "
                + tag
                + str(ci)
                + "_* $child]} {interp delete $child}}; foreach command [info commands "
                + n
                + "_*] {rename $command {}}; info commands "
                + n
                + "_*"
            )
            source += (
                "set nh_code [catch { "
                + cleanup
                + " } nh_result]\n"
                + capture.emit(
                    context,
                    tag,
                    case["id"],
                    "runtime-cleanup",
                    "$nh_code",
                    "$nh_result",
                    "$nh_unit",
                )
            )
            name = "__tcl_lsp_probe_" + tag + "_" + str(ci) + "_" + str(index)
            capture.run_job(
                dest / (context + "-" + str(index)),
                remote,
                context,
                name,
                source,
                wait=5,
            )
            print("captured literal", context, cid, flush=True)
        (dest / (context + "-cases.json")).write_text(
            json.dumps(records, indent=2) + "\n"
        )
    (dest / "capture-complete.json").write_text('{"completed":true}\n')


if __name__ == "__main__":
    main()
