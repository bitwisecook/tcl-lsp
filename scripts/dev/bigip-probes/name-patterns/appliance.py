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

"""Measure owned cross-rule calls, argument bytes and ordered provider mutation."""

import argparse
import datetime
import json
import shlex
from pathlib import Path

import capture


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", required=True)
    args = parser.parse_args()
    tag = args.run
    capture.TAG = tag
    dest = capture.ROOT / "experiments/name-handling/patterns/f5/answers" / tag
    dest.mkdir(parents=True, exist_ok=False)
    (dest / "appliance.py").write_bytes(Path(__file__).read_bytes())
    (dest / "capture.py").write_bytes(Path(capture.__file__).read_bytes())
    remote = "/var/tmp/tcl_lsp_" + tag
    if capture.rpc("test ! -e " + remote + " && mkdir -m 700 " + remote).returncode:
        raise RuntimeError("Staging collision")
    (dest / "provenance.json").write_text(
        json.dumps(
            {
                "run_id": tag,
                "configuration_saved": False,
                "remote_staging": remote,
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
        ("tmm-roster", "tmsh show sys tmm-info"),
    ]:
        r = capture.rpc(command)
        (dest / (label + ".stdout")).write_bytes(r.stdout)
        (dest / (label + ".stderr")).write_bytes(capture.receipt_bytes(r.stderr))
    original = capture.make_config
    for ci, context in enumerate(
        ["tmm-RULE_INIT", "tmm-procedure", "tmm-traffic-event"]
    ):
        name = "__tcl_lsp_probe_" + tag + "_" + str(ci)
        provider = name + "_provider"
        other = name + "_other"
        base = "/Common/" + provider
        static = "static::__nh_" + tag + "_" + str(ci)
        case_specs = [
            ("bare-local", "corpus-f5-rule-path", "call target {a b} {c d}"),
            (
                "relative-provider",
                "corpus-f5-rule-path",
                "call " + provider + "::target {a b} {c d}",
            ),
            (
                "absolute-provider",
                "corpus-f5-rule-path",
                "call " + base + "::target {a b} {c d}",
            ),
            (
                "absolute-other",
                "corpus-f5-rule-path",
                "call /Common/" + other + "::target {a b} {c d}",
            ),
            (
                "missing-rule",
                "corpus-f5-rule-path",
                "call /Common/" + name + "_absent::target {a b} {c d}",
            ),
            (
                "missing-proc",
                "corpus-f5-rule-path",
                "call " + base + "::absent {a b} {c d}",
            ),
            (
                "computed-target",
                "corpus-f5-computed-target",
                "set target " + base + "::target; eval [list call $target {a b} {c d}]",
            ),
            (
                "list-prefix-arguments",
                "corpus-f5-prefix-arguments",
                "set prefix [list call "
                + base
                + "::target {a b}]; eval [linsert $prefix end {{x} ::tail}]",
            ),
            (
                "space-joined-near-miss",
                "corpus-f5-prefix-arguments",
                'set prefix "call ' + base + '::target a b"; eval "$prefix c d"',
            ),
            (
                "provider-lifetime",
                "corpus-f5-rule-lifetime",
                "call " + base + "::target {a b} {c d}",
            ),
            (
                "storage-read-write",
                "corpus-f5-storage-context",
                "call " + base + "::cells " + static,
            ),
        ]
        if context == "tmm-traffic-event":
            case_specs.append(
                (
                    "connection-identity",
                    "corpus-f5-storage-context",
                    "list [IP::client_addr] [TCP::client_port] [HTTP::header value X-NH-Invocation]",
                )
            )
        selected = [
            {
                "id": key,
                "question_id": q,
                "setup": "",
                "operation": op,
                "variant_id": "owned-common-cross-rule-v1",
                "scope": "Owned Common rules; distinct provider sentinels; identical target tails; no claim about other partitions or active suspended frames.",
            }
            for key, q, op in case_specs
        ]
        for x in selected:
            x["context"] = context
        (dest / (context + "-cases.json")).write_text(
            json.dumps(selected, indent=2) + "\n"
        )

        def config_override(
            ctx, n, source, port=19912, base=base, other=other, static=static
        ):
            config, objects, invoke = original(ctx, n, source, port)
            local = "proc target {fixed value} {list LOCAL $fixed $value}\n"
            provider_config = (
                "ltm rule "
                + base
                + " {\nproc target {fixed value} {list OLD $fixed $value}\nproc cells {cell} {global $cell; incr $cell; set ns namespace; list [set $cell] [info level] [$ns current]}\n}\nltm rule /Common/"
                + other
                + " {\nproc target {fixed value} {list OTHER $fixed $value}\n}\n"
            )
            config = provider_config + config.replace(
                "ltm rule /Common/" + n + " {\n",
                "ltm rule /Common/" + n + " {\n" + local,
                1,
            )
            config = config.replace(
                "when RULE_INIT {\n", "when RULE_INIT {\nset " + static + " 0\n", 1
            )
            if ctx == "tmm-procedure":
                config = config.replace(
                    "when RULE_INIT { call measure }",
                    "when RULE_INIT {set " + static + " 0; call measure}",
                )
            if ctx == "tmm-traffic-event":
                config = config.replace(
                    "when HTTP_REQUEST {",
                    "when RULE_INIT {set " + static + " 0}\nwhen HTTP_REQUEST {",
                )
                config = config.replace(
                    "rules { /Common/" + n + " }",
                    "rules { " + base + " /Common/" + other + " /Common/" + n + " }",
                )
            return (
                config,
                [("ltm rule", base), ("ltm rule", "/Common/" + other)] + objects,
                invoke,
            )

        # Deliberate override of capture's config builder for this context.
        capture.make_config = config_override  # ty: ignore[invalid-assignment]

        def mutate(job, staging, ctx, n, index, base=base, other=other):
            if index not in [8, 12]:
                return
            phase = "replace" if index == 8 else "detach"
            if index == 8:
                text = (
                    "ltm rule "
                    + base
                    + " {\nproc target {fixed value} {list NEW $fixed $value}\nproc cells {cell} {global $cell; incr $cell; set ns namespace; list [set $cell] [info level] [$ns current]}\n}\n"
                )
                path = staging + "/replacement.conf"
                capture.rpc("cat > " + shlex.quote(path), text.encode())
                (job / "replacement.conf").write_text(text)
                command = "tmsh load sys config merge file " + shlex.quote(path)
            else:
                command = (
                    "tmsh modify ltm virtual /Common/"
                    + n
                    + "_vs rules { /Common/"
                    + other
                    + " /Common/"
                    + n
                    + " }"
                )
            r = capture.rpc(command)
            (job / (phase + ".stdout")).write_bytes(r.stdout)
            (job / (phase + ".stderr")).write_bytes(capture.receipt_bytes(r.stderr))
            (job / (phase + ".json")).write_text(
                json.dumps(
                    {
                        "argv": capture.SSH + [command],
                        "receipt_redactions": capture.RECEIPT_REDACTIONS,
                        "exit_code": r.returncode,
                        "before_request": index,
                    },
                    indent=2,
                )
                + "\n"
            )

        capture.TRAFFIC_HOOK = mutate if context == "tmm-traffic-event" else None
        capture.run_job(
            dest / (context + "-0"),
            remote,
            context,
            name,
            capture.body(selected, context, tag),
            wait=5,
        )
        capture.make_config = original
        clean = {
            "id": "static-runtime-cleanup",
            "setup": "",
            "operation": "unset -nocomplain " + static + "; info exists " + static,
        }
        capture.run_job(
            dest / (context + "-cleanup"),
            remote,
            "tmm-RULE_INIT",
            name + "_cleanup",
            capture.body([clean], "tmm-RULE_INIT", tag),
            wait=5,
        )
        print("captured appliance", context, flush=True)
    capture.make_config = original
    (dest / "capture-complete.json").write_text(json.dumps({"completed": True}) + "\n")


if __name__ == "__main__":
    main()
