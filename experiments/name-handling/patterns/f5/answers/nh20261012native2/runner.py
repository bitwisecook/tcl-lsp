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

"""Prepare singleton RULE_INIT completion controls; only --execute deploys."""

import argparse
import datetime
import importlib.util
import json
import shlex
import shutil
from pathlib import Path

PREVIOUS = Path(__file__).resolve().parent.parent / "nh20261012native1/runner.py"
SPEC = importlib.util.spec_from_file_location("native1_preparation", PREVIOUS)
native = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(native)
capture = native.capture


def build_plan(tag):
    base = native.build_plan(tag)
    seed = next(
        case
        for case in base["metadata"]["tmm-RULE_INIT"]
        if case["id"] == "owned-state-seed"
    )
    seed = dict(seed)
    seed["id"] = "native2-state-seed"
    old = base["replacement"].replace("list owner NEW argc", "list owner OLD argc")
    if old == base["replacement"]:
        raise ValueError("Provider sentinel replacement failed")
    caller = "/Common/" + base["stem"] + "_caller"
    virtual = caller + "_vs"
    initial = capture.body([seed], "tmm-RULE_INIT", tag)
    config = (
        old
        + "ltm rule "
        + caller
        + " {\nwhen RULE_INIT {\n"
        + initial
        + "}\nwhen HTTP_REQUEST {\nHTTP::respond 200 content {"
        + base["stem"]
        + "} Connection close\n}\n}\nltm virtual "
        + virtual
        + " { destination 192.0.2.10:19912 mask 255.255.255.255 ip-protocol tcp profiles { tcp {} http {} } rules { "
        + base["provider"]
        + " "
        + caller
        + " } }\n"
    )
    cases = [dict(base["preflight"]), seed, dict(base["cleanup"])]
    groups = []
    for phase in ["old", "new", "rejected"]:
        for source in base["cases"]:
            if source["id"] not in {
                "provider-lifetime",
                "provider-local-upvar-write",
                "provider-shared-write-readback",
            }:
                continue
            case = dict(source)
            case["id"] = phase + "-" + source["id"]
            case["setup"] = "set nh_phase " + phase + "; set nh_action provider-write"
            case["variant_id"] = "native-owned-singleton-init-completion-v1"
            case["scope"] = (
                "New uniquely named native2 control, identical native operation family to native1; singleton RULE_INIT load on actual measured units. Explicit provider writes; OLD/NEW/failed replacement phases. No native-proc HTTP substitution or suspended-frame claim."
            )
            body = capture.body([case], "tmm-RULE_INIT", tag).replace(
                "set nh_invocation once\n",
                "set nh_invocation " + phase + "-" + source["id"] + "\n",
                1,
            )
            cases.append(case)
            groups.append(
                {
                    "phase": phase,
                    "case": case,
                    "source": body,
                    "name": base["stem"] + "_" + str(len(groups)),
                }
            )
    return {
        "tag": tag,
        "stem": base["stem"],
        "provider": base["provider"],
        "config": config,
        "objects": [
            ("ltm rule", base["provider"]),
            ("ltm rule", caller),
            ("ltm virtual", virtual),
        ],
        "cases": cases,
        "groups": groups,
        "preflight": base["preflight"],
        "cleanup": base["cleanup"],
        "replacement": base["replacement"],
        "rejected_replacement": base["rejected_replacement"],
        "limits": {
            "scope": "RULE_INIT singleton gap controls only",
            "suspended_frames": "unmeasured",
            "native_catalog_retirement": "not inferred",
            "configuration_saved": False,
        },
    }


def execute(plan, dest):
    native.install_private_transport()
    capture.TAG = plan["tag"]
    remote = "/var/tmp/tcl_lsp_" + plan["tag"]
    if native.save_rpc(
        dest,
        "staging-create",
        "test ! -e " + shlex.quote(remote) + " && mkdir -m 700 " + shlex.quote(remote),
    ).returncode:
        raise RuntimeError("Remote staging collision")
    for label, command in [
        ("version", "tmsh show sys version"),
        ("platform", "tmsh show sys hardware"),
        ("tmm-roster", "tmsh show sys tmm-info"),
    ]:
        native.save_rpc(dest, label, command)
    preflight = dest / "tmm-RULE_INIT-0"
    capture.run_job(
        preflight,
        remote,
        "tmm-RULE_INIT",
        plan["stem"] + "_preflight",
        capture.body([plan["preflight"]], "tmm-RULE_INIT", plan["tag"]),
        wait=5,
    )
    fields = native.parse_capture(
        [preflight / "runtime-logs.stdout"], "tmm-RULE_INIT", plan["tag"]
    )["fields"]
    rows = [
        field
        for field in fields
        if field.get("case_id") == plan["preflight"]["id"]
        and field.get("stage") == "lookup"
        and field.get("field") == "result"
        and field.get("complete")
    ]
    if len(
        {row.get("tmm_unit") for row in rows if row.get("completion_code") == 0}
    ) != 4 or any(row.get("completion_code") != 0 for row in rows):
        raise RuntimeError("Four-unit preabsence not proved")
    original_config = capture.make_config
    original_hook = capture.TRAFFIC_HOOK

    def remote_diagnostic(label, command, data=None):
        # Remote tmsh stderr joins remote stdout; SSH notices remain outside it.
        return native.save_rpc(
            dest / "tmm-traffic-event-0",
            label,
            "bash -c " + shlex.quote(command + " 2>&1"),
            data,
        )

    def hook(job, staging, _context, _name, index):
        phases = {0: "old", 3: "new", 6: "rejected"}
        if index not in phases:
            return
        phase = phases[index]
        if phase != "old":
            key = "replacement" if phase == "new" else "rejected_replacement"
            text = plan[key]
            (job / (phase + ".conf")).write_text(text)
            path = staging + "/" + phase + ".conf"
            if native.save_rpc(
                job, phase + "-stage", "cat > " + shlex.quote(path), text.encode()
            ).returncode:
                raise RuntimeError("Provider staging failed")
            result = remote_diagnostic(
                phase + "-apply", "tmsh load sys config merge file " + shlex.quote(path)
            )
            if (phase == "rejected") == (result.returncode == 0):
                raise RuntimeError("Unexpected provider replacement acceptance")
        remote_diagnostic(phase + "-provider", "tmsh list ltm rule " + plan["provider"])
        for number, group in enumerate(plan["groups"]):
            if group["phase"] != phase:
                continue
            capture.make_config = original_config
            try:
                capture.run_job(
                    dest / ("tmm-RULE_INIT-" + str(number + 2)),
                    remote,
                    "tmm-RULE_INIT",
                    group["name"],
                    group["source"],
                    wait=5,
                )
            finally:
                capture.make_config = lambda _c, _n, _s, port=19912: (
                    plan["config"],
                    plan["objects"],
                    None,
                )

    job = dest / "tmm-traffic-event-0"
    try:
        capture.make_config = lambda _c, _n, _s, port=19912: (
            plan["config"],
            plan["objects"],
            None,
        )
        capture.TRAFFIC_HOOK = hook
        capture.run_job(
            job, remote, "tmm-traffic-event", plan["stem"], plan["config"], wait=5
        )
    finally:
        capture.make_config = original_config
        capture.TRAFFIC_HOOK = original_hook
        capture.run_job(
            dest / "tmm-RULE_INIT-11",
            remote,
            "tmm-RULE_INIT",
            plan["stem"] + "_cleanup",
            capture.body([plan["cleanup"]], "tmm-RULE_INIT", plan["tag"]),
            wait=5,
        )
    alias = dest / "tmm-RULE_INIT-1"
    shutil.copytree(job, alias)
    (alias / "evidence-alias.json").write_text(
        json.dumps(
            {
                "source_job": job.name,
                "context_filter": "tmm-RULE_INIT",
                "independent_execution": False,
            },
            indent=2,
        )
        + "\n"
    )
    native.save_rpc(
        dest,
        "final-runtime",
        "grep -F " + shlex.quote("NHO|" + plan["tag"] + "|") + " /var/log/ltm",
    )
    (dest / "capture-complete.json").write_text(
        json.dumps({"completed": True, "configuration_saved": False}) + "\n"
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", default="nh20261012native2")
    parser.add_argument("--destination", type=Path)
    parser.add_argument("--execute", action="store_true")
    args = parser.parse_args()
    plan = build_plan(args.run)
    dest = args.destination or Path(__file__).resolve().parent
    dest.mkdir(parents=True, exist_ok=True)
    if (dest / "plan.json").exists():
        raise RuntimeError("Existing preparation plan")
    if dest.resolve() != Path(__file__).resolve().parent:
        (dest / "runner.py").write_bytes(Path(__file__).read_bytes())
    (dest / "plan.json").write_text(json.dumps(plan, indent=2) + "\n")
    (dest / "prepared.conf").write_text(plan["config"])
    (dest / "tmm-RULE_INIT-cases.json").write_text(
        json.dumps(plan["cases"], indent=2) + "\n"
    )
    (dest / "provenance.json").write_text(
        json.dumps(
            {
                "run_id": args.run,
                "remote_staging": "/var/tmp/tcl_lsp_" + args.run,
                "timestamp_utc": datetime.datetime.now(
                    datetime.timezone.utc
                ).isoformat(),
                "configuration_saved": False,
                "execution_requested": args.execute,
                "source_helper": native.capture.ROOT.joinpath(PREVIOUS)
                .relative_to(native.capture.ROOT)
                .as_posix(),
                "source_helper_sha256": capture.digest(PREVIOUS.read_bytes()),
                "capture_helper_sha256": capture.digest(
                    Path(capture.__file__).read_bytes()
                ),
                "limits": plan["limits"],
            },
            indent=2,
        )
        + "\n"
    )
    if args.execute:
        execute(plan, dest)
    print(dest)


if __name__ == "__main__":
    main()
