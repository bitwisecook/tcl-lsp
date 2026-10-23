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
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Read-only required source-pin checks for diagnostic dispatch preflight.

The caller supplies the exact approved manifests for its selected mode. No build,
child process, environment change, input refresh or source mutation is performed.
"""

import argparse
import hashlib
import json
import re
from collections.abc import Callable, Iterable, Mapping
from pathlib import Path


def validate_required_source_pins(
    checks: Iterable[tuple[str, Mapping[str, str]]],
    read_source: Callable[[str], bytes],
) -> list[dict[str, str | None]]:
    """Return every absent, malformed, conflicting or changed required input."""
    failures: list[dict[str, str | None]] = []
    approved = {}
    count = 0
    for label, pins in checks:
        count += 1
        if not pins:
            failures.append({"manifest": label, "error": "required source pins absent"})
        for name, expected in pins.items():
            path = Path(name)
            record = {"manifest": label, "path": name, "expected": expected}
            if (
                path.is_absolute()
                or ".." in path.parts
                or not re.fullmatch(r"[0-9a-f]{64}", expected)
            ):
                failures.append(
                    {**record, "error": "invalid source path or SHA256 pin"}
                )
                continue
            if name in approved and approved[name] != expected:
                failures.append({**record, "error": "conflicting required pin"})
            approved[name] = expected
            try:
                actual = hashlib.sha256(read_source(name)).hexdigest()
            except OSError as error:
                failures.append({**record, "error": str(error)})
                continue
            if actual != expected:
                failures.append(
                    {**record, "actual": actual, "error": "required source changed"}
                )
    if not count:
        failures.append({"error": "required manifests absent"})
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--roster", type=Path, action="append", required=True)
    args = parser.parse_args()
    checks = [
        (str(path), json.loads(path.read_text())["source_pins"]) for path in args.roster
    ]
    root = args.root.resolve()

    def read_source(name: str) -> bytes:
        target = root / name
        if not target.resolve().is_relative_to(root):
            raise OSError("required source leaves repository")
        return target.read_bytes()

    failures = validate_required_source_pins(checks, read_source)
    print(json.dumps({"required_source_pin_failures": failures}, indent=2))
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
