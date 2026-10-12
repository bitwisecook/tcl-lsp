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

"""Retain APL callback results from a real browser form render."""

import argparse
import datetime
import json
import re
import shlex
from pathlib import Path
import capture
import cases
import broader_cases


def receipt(job, label, command):
    result = capture.rpc(command)
    (job / (label + ".stdout")).write_bytes(result.stdout)
    (job / (label + ".stderr")).write_bytes(result.stderr)
    (job / (label + ".exit")).write_text(str(result.returncode) + "\n")
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--run", required=True)
    parser.add_argument(
        "--kind",
        choices=["capabilities", "patterns", "broader", "wiki"],
        default="capabilities",
    )
    parser.add_argument("--phase", choices=["prepare", "finish"], required=True)
    args = parser.parse_args()
    if not re.fullmatch("[a-z0-9]{6,24}", args.run):
        raise ValueError("Invalid tag")
    tag = args.run
    capture.TAG = tag
    root = capture.ROOT / "experiments/name-handling"
    dest = (
        root / ("f5/answers" if args.kind == "broader" else "patterns/f5/answers") / tag
    )
    job = dest / "iapp-apl-0"
    name = "/Common/__tcl_lsp_probe_" + tag
    staging = "/var/tmp/tcl_lsp_" + tag
    if args.phase == "finish":
        ownership = json.loads((dest / "ownership.json").read_text())
        if ownership != {"object": name, "absent_before": True}:
            raise RuntimeError("Ownership not proven")
        receipt(job, "delete-0", "tmsh delete sys application template " + name)
        after = receipt(job, "after-0", "tmsh list sys application template " + name)
        if b"was not found" not in after.stderr:
            raise RuntimeError("Cleanup not proven")
        (dest / "capture-complete.json").write_text(
            json.dumps({"completed": True, "browser_render_required": True}) + "\n"
        )
        return
    dest.mkdir(parents=True, exist_ok=False)
    job.mkdir()
    for module in [
        Path(__file__),
        Path(capture.__file__),
        Path(cases.__file__),
        Path(broader_cases.__file__),
    ]:
        (dest / module.name).write_bytes(module.read_bytes())
    provenance = {
        "run_id": tag,
        "configuration_saved": False,
        "remote_staging": staging,
        "timestamp_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "invoking_action": "Chrome Application Services Create form; template selection invokes APL callbacks",
    }
    (dest / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    for label, command in [
        ("version", "tmsh show sys version"),
        ("platform", "tmsh show sys hardware"),
        ("tmm-roster", "tmsh show sys tmm-info"),
    ]:
        receipt(dest, label, command)
    before = receipt(job, "before-0", "tmsh list sys application template " + name)
    if b"was not found" not in before.stderr:
        raise RuntimeError("Object already exists or absence unproven")
    if args.kind == "capabilities":
        selected = capture.capabilities(tag)
    elif args.kind == "patterns":
        selected = cases.build_cases(tag)
    elif args.kind == "wiki":
        selected = cases.wiki_cases()
        for item in selected:
            item["exact_source"] = False
            item["scope"] = (
                "Complete real suite in APL callback; fresh-child premise unproven; setup failure leaves suite unreached."
            )
            item["variant_id"] = "apl-real-tcltest-capability-guard-v1"
    else:
        selected = broader_cases.build_cases(tag)
    presentation = "section probe {\n"
    sources = []
    for index, item in enumerate(selected):
        item["context"] = "iapp-apl"
        source = capture.body([item], "iapp-apl", tag)
        source = "set nh_apl_rows {}\n" + source.replace(
            'puts "NHO|', 'lappend nh_apl_rows "NHO|'
        )
        source += "\nset nh_apl_choices {}; foreach nh_apl_row $nh_apl_rows {lappend nh_apl_choices [list $nh_apl_row $nh_apl_row]}; return $nh_apl_choices\n"
        sources.append(source)
        presentation += "choice result" + str(index) + " tcl {\n" + source + "\n}\n"
    presentation += "}\n"
    config = (
        "sys application template "
        + name
        + " {\n actions {\n definition {\n implementation {}\n presentation {\n"
        + presentation
        + "}\n }\n }\n}\n"
    )
    (dest / "iapp-apl-cases.json").write_text(json.dumps(selected, indent=2) + "\n")
    (job / "input.tcl").write_text("\n".join(sources))
    (job / "applied.conf").write_text(config)
    result = capture.rpc(
        "test ! -e "
        + staging
        + " && mkdir -m 700 "
        + staging
        + " && cat > "
        + staging
        + "/template.conf",
        config.encode(),
    )
    if result.returncode:
        raise RuntimeError(result.stderr.decode(errors="replace"))
    (dest / "ownership.json").write_text(
        json.dumps({"object": name, "absent_before": True}) + "\n"
    )
    load = receipt(
        job,
        "load",
        "tmsh load sys config merge file " + shlex.quote(staging + "/template.conf"),
    )
    receipt(job, "applied-0", "tmsh list sys application template " + name)
    print(
        json.dumps(
            {
                "template": name,
                "load_exit": load.returncode,
                "cases": len(selected),
                "next": "Render template in Chrome; retain DOM option strings as invoke.stdout, browser action metadata and screenshot; then finish.",
            }
        )
    )


if __name__ == "__main__":
    main()
