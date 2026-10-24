# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook)
# SPDX-License-Identifier: AGPL-3.0-or-later

"""Fail-closed checks for mixed and all-ignored Nextest reporter suites."""

import json
import sys
from pathlib import Path


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(f"check-ignored-report: {message}")


def check(
    path: str,
    package: str,
    binary: str,
    selected: int,
    ignored: int,
    emit_nextest: bool = True,
) -> None:
    rows = []
    for number, line in enumerate(Path(path).read_text().splitlines(), start=1):
        if not line.strip():
            continue
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError as exc:
            raise SystemExit(
                f"check-ignored-report: invalid JSON at {path}:{number}: {exc}"
            ) from exc
    suites = [row for row in rows if row.get("type") == "suite"]
    require(
        len(suites) == 2,
        f"{package}: expected one suite start and finish, got {len(suites)} suite events",
    )
    opened = [row for row in suites if row.get("event") == "started"]
    closed = [row for row in suites if row.get("event") == "ok"]
    require(
        len(opened) == 1 and len(closed) == 1, f"{package}: unexpected suite lifecycle"
    )
    require(
        opened[0].get("test_count") == selected + ignored,
        f"{package}: unexpected suite test_count",
    )
    for row in suites:
        if emit_nextest:
            identity = row.get("nextest") or {}
            require(
                identity.get("crate") == package,
                f"{package}: suite crate identity mismatch",
            )
            require(
                identity.get("test_binary") == binary,
                f"{binary}: suite binary identity mismatch",
            )
        else:
            require(
                "nextest" not in row,
                f"{package}: non-plus suite unexpectedly contains nextest metadata",
            )
    actual_summary = {
        key: closed[0].get(key)
        for key in ("passed", "failed", "ignored", "measured", "filtered_out")
    }
    expected_summary = {
        "passed": selected,
        "failed": 0,
        "ignored": ignored,
        "measured": 0,
        "filtered_out": 0,
    }
    require(
        actual_summary == expected_summary,
        f"{package}: suite summary mismatch: {actual_summary!r}",
    )
    tests = [row for row in rows if row.get("type") == "test"]
    terminal = [
        (row.get("name"), row.get("event"))
        for row in tests
        if row.get("event") in {"ok", "ignored", "failed"}
    ]
    expected = [
        (f"{package}::{binary}$tests::selected_{index}", "ok")
        for index in range(1, selected + 1)
    ]
    expected += [
        (f"{package}::{binary}$tests::ignored_{index}", "ignored")
        for index in range(1, ignored + 1)
    ]
    require(
        sorted(terminal) == sorted(expected),
        f"{package}: terminal test rows mismatch: {terminal!r}",
    )


if len(sys.argv) != 5:
    raise SystemExit(
        "check-ignored-report: expected plus mixed/all-ignored and non-plus mixed/all-ignored report paths"
    )
for offset, emit_nextest in ((1, True), (3, False)):
    check(
        sys.argv[offset],
        "nextest-ignored-regression",
        "nextest_ignored_regression",
        2,
        1,
        emit_nextest,
    )
    check(
        sys.argv[offset + 1],
        "nextest-all-ignored-regression",
        "nextest_all_ignored_regression",
        0,
        2,
        emit_nextest,
    )
print(
    "nextest ignored reporter regression: plus and non-plus mixed 2+1/all-ignored 0+2; one lifecycle each"
)
