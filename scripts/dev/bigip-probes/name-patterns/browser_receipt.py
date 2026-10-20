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

"""Project retained live APL DOM option values into strict result rows."""

import argparse
import hashlib
import json
import tkinter
from pathlib import Path

import parse_capture
import report


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=Path)
    args = parser.parse_args()
    run = args.run.resolve()
    if (run / "MANIFEST.sha256").exists():
        raise RuntimeError("Sealed run is immutable")
    data = json.loads((run / "browser-options.json").read_text())
    snapshot = (run / "browser-dom.txt").read_text()
    selected_template = '- option "__tcl_lsp_probe_' + run.name + '" [selected]'
    if not any(line.strip() == selected_template for line in snapshot.splitlines()):
        raise RuntimeError("Selected template not proven by DOM")
    data = [select for select in data if select["name"].startswith("var__probe__")]
    snapshot = (
        "\n".join(
            line
            for line in snapshot.splitlines()
            if line.strip() == selected_template
            or "var__probe__" in line
            or "NHO|" in line
        )
        + "\n"
    )
    tcl = tkinter.Tcl()
    rows = []
    projections = set()
    for select in data:
        if not select["name"].startswith("var__probe__"):
            continue
        for option in select["options"]:
            value = option["value"]
            if value.startswith("NHO|"):
                rows.append(value)
                projections.add("one DOM option.value per NHO row")
            elif value:
                for pair in tcl.splitlist(value):
                    parts = tcl.splitlist(pair)
                    if (
                        len(parts) != 2
                        or parts[0] != parts[1]
                        or not parts[0].startswith("NHO|")
                    ):
                        raise RuntimeError("Unexpected choice output")
                    rows.append(parts[0])
                projections.add(
                    "Tcl list of equal label/value pairs in DOM option.value"
                )
    if not rows:
        raise RuntimeError("No callback rows observed")
    (run / "browser-options.json").write_text(json.dumps(data, indent=2) + "\n")
    (run / "browser-dom.txt").write_text(snapshot)
    (run / "browser.png").unlink(missing_ok=True)
    job = run / "iapp-apl-0"
    (job / "invoke.stdout").write_text("\n".join(rows) + "\n")
    (job / "invocation.json").write_text(
        json.dumps(
            {
                "action": "Chrome Application Services Create form template selection; callback values retained from live DOM",
                "template": "/Common/__tcl_lsp_probe_" + run.name,
                "source": "browser-options.json",
                "projection": sorted(projections),
                "projection_validation": "passed",
                "browser_process_exit_code": None,
                "rows": len(rows),
                "retained_browser_scope": "Selected owned template and probe option rows; device chrome and screenshot omitted",
            },
            indent=2,
        )
        + "\n"
    )
    capture = parse_capture.parse_capture(
        [job / "invoke.stdout"], context="iapp-apl", run_id=run.name
    )
    versions = [
        f
        for f in capture["fields"]
        if f["case_id"] == "version"
        and f["stage"] == "lookup"
        and f["field"] == "result"
        and f["complete"]
        and f["completion_code"] == 0
    ]
    if versions:
        field = versions[0]
        parts = field["text"].split()
        if len(parts) != 3:
            raise RuntimeError("Unexpected provider version result")
        facts = {
            "interpreter_patchlevel": parts[0],
            "execution_namespace": parts[2],
            "value_hex": field["value_hex"],
            "source": str(job / "invoke.stdout"),
            "source_sha256": hashlib.sha256(
                (job / "invoke.stdout").read_bytes()
            ).hexdigest(),
            "case_id": "version",
            "context": "iapp-apl",
        }
        (run / "interpreter-facts.json").write_text(json.dumps(facts, indent=2) + "\n")
    records = report.write_report(run)
    print(
        json.dumps(
            {
                "rows": len(rows),
                "records": len(records),
                "observed": sum(r["status"] == "observed" for r in records),
            }
        )
    )


if __name__ == "__main__":
    main()
