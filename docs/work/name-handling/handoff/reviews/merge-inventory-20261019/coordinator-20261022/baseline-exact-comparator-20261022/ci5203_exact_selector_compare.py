#!/usr/bin/env python3
"""Fail-closed comparison of nextest selector outcomes by (binary_id, selector).

Input JSON: an object with a `records` array and optional provenance fields.
Baseline rows must be PASS rows with package, binary_label_as_rendered,
binary_id, selector, and status. Current rows must include binary_id, selector,
and status; all other receipt/record fields are preserved in output.
"""
from __future__ import annotations

import argparse
import copy
import json
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any

BASELINE_SCHEMA = "nextest-human-reporter-observed-identities-v1"
FAILURE_STATUSES = {"FAIL", "FAILED", "ABORT", "ABORTED", "TIMEOUT", "TIMED_OUT", "TIMEDOUT"}
PASS_STATUSES = {"PASS", "PASSED"}
SKIP_STATUSES = {"SKIP", "SKIPPED", "IGNORE", "IGNORED", "FILTERED"}


class ReceiptError(ValueError):
    pass


def identity(record: dict[str, Any], label: str) -> tuple[str, str]:
    binary_id = record.get("binary_id")
    selector = record.get("selector")
    if not isinstance(binary_id, str) or not binary_id:
        raise ReceiptError(f"{label}: binary_id must be a nonempty string")
    if not isinstance(selector, str) or not selector:
        raise ReceiptError(f"{label}: selector must be a nonempty string")
    return binary_id, selector


def status(record: dict[str, Any], label: str) -> str:
    value = record.get("status")
    if not isinstance(value, str) or not value:
        raise ReceiptError(f"{label}: status must be a nonempty string")
    return value.strip().upper().replace("-", "_").replace(" ", "_")


def load_receipt(path: Path, kind: str) -> dict[str, Any]:
    try:
        receipt = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise ReceiptError(f"{kind}: cannot read JSON receipt: {exc}") from exc
    if not isinstance(receipt, dict) or not isinstance(receipt.get("records"), list):
        raise ReceiptError(f"{kind}: expected a JSON object with a records array")
    return receipt


def validate_baseline(receipt: dict[str, Any]) -> None:
    if receipt.get("schema") != BASELINE_SCHEMA:
        raise ReceiptError(f"baseline: expected schema {BASELINE_SCHEMA!r}")
    if not receipt["records"]:
        raise ReceiptError("baseline: records must not be empty")
    for index, row in enumerate(receipt["records"]):
        label = f"baseline record {index}"
        if not isinstance(row, dict):
            raise ReceiptError(f"{label}: expected object")
        for field in ("package", "binary_label_as_rendered"):
            if field not in row:
                raise ReceiptError(f"{label}: missing {field}")
        package = row["package"]
        rendered = row["binary_label_as_rendered"]
        if not isinstance(package, str) or not package:
            raise ReceiptError(f"{label}: package must be a nonempty string")
        if rendered is not None and (not isinstance(rendered, str) or not rendered):
            raise ReceiptError(f"{label}: rendered binary label must be null or nonempty string")
        expected = package if rendered is None else f"{package}::{rendered}"
        if row.get("binary_id") != expected:
            raise ReceiptError(f"{label}: binary_id disagrees with recovered package/label")
        identity(row, label)
        if status(row, label) not in PASS_STATUSES:
            raise ReceiptError(f"{label}: baseline must contain PASS rows only")


def duplicate_groups(receipt: dict[str, Any], kind: str) -> list[dict[str, Any]]:
    groups: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
    for index, row in enumerate(receipt["records"]):
        if not isinstance(row, dict):
            raise ReceiptError(f"{kind} record {index}: expected object")
        groups[identity(row, f"{kind} record {index}")].append(row)
    return [
        {"source": kind, "key": {"binary_id": key[0], "selector": key[1]}, "records": copy.deepcopy(rows)}
        for key, rows in sorted(groups.items()) if len(rows) > 1
    ]


def provenance(receipt: dict[str, Any]) -> dict[str, Any]:
    # Keep the complete receipt envelope (run/head/workflow/profile/features/config,
    # counts and source references) while excluding only the repeated row payload.
    return {key: copy.deepcopy(value) for key, value in receipt.items() if key != "records"}


def compare(baseline: dict[str, Any], current: dict[str, Any]) -> dict[str, Any]:
    validate_baseline(baseline)
    baseline_dupes = duplicate_groups(baseline, "baseline")
    current_dupes = duplicate_groups(current, "current")
    duplicates = baseline_dupes + current_dupes
    result: dict[str, Any] = {
        "schema": "exact-nextest-selector-comparison-v1",
        "comparison_status": "fail_closed_duplicate_identity" if duplicates else "compared",
        "identity": ["binary_id", "selector"],
        "baseline_provenance": provenance(baseline),
        "current_provenance": provenance(current),
        "input_counts": {"baseline_records": len(baseline["records"]), "current_records": len(current["records"])},
        "duplicates": duplicates,
        "matched_passes": [],
        "regressions": [],
        "unknown": [],
        "extra_current_passes": [],
        "unmatched_current_nonpass": [],
        "classification_counts": None,
        "limitations": [
            "Only exact named (binary_id, selector) records are compared.",
            "Missing or skipped current selectors are unknown, never passes or failures.",
            "Rows absent from baseline are kept separate from baseline regressions.",
            "A duplicate identity anywhere fails closed: no outcome classifications are emitted.",
            "This helper does not establish suite/listing completeness or a CI verdict.",
        ],
    }
    if duplicates:
        return result

    baseline_rows = {identity(row, "baseline"): row for row in baseline["records"]}
    current_rows = {identity(row, "current"): row for row in current["records"]}

    def pair(key: tuple[str, str], reason: str, current_row: dict[str, Any] | None) -> dict[str, Any]:
        return {
            "key": {"binary_id": key[0], "selector": key[1]},
            "reason": reason,
            "baseline_record": copy.deepcopy(baseline_rows[key]),
            "current_record": copy.deepcopy(current_row) if current_row is not None else None,
        }

    for key, baseline_row in baseline_rows.items():
        current_row = current_rows.get(key)
        if current_row is None:
            result["unknown"].append(pair(key, "missing_current_record", None))
            continue
        current_status = status(current_row, "current")
        if current_status in PASS_STATUSES:
            result["matched_passes"].append(pair(key, "current_pass", current_row))
        elif current_status in FAILURE_STATUSES:
            result["regressions"].append(pair(key, f"current_{current_status.lower()}", current_row))
        else:
            reason = "current_skip" if current_status in SKIP_STATUSES else "unrecognized_current_status"
            result["unknown"].append(pair(key, reason, current_row))

    for key, current_row in current_rows.items():
        if key in baseline_rows:
            continue
        current_status = status(current_row, "current")
        wrapper = {"key": {"binary_id": key[0], "selector": key[1]}, "current_record": copy.deepcopy(current_row)}
        if current_status in PASS_STATUSES:
            wrapper["reason"] = "new_current_pass"
            result["extra_current_passes"].append(wrapper)
        else:
            wrapper["reason"] = "current_nonpass_without_baseline"
            result["unmatched_current_nonpass"].append(wrapper)

    for key in ("matched_passes", "regressions", "unknown", "extra_current_passes", "unmatched_current_nonpass"):
        result[key].sort(key=lambda row: (row["key"]["binary_id"], row["key"]["selector"]))
    result["classification_counts"] = {
        key: len(result[key])
        for key in ("matched_passes", "regressions", "unknown", "extra_current_passes", "unmatched_current_nonpass")
    }
    return result


def self_test() -> dict[str, Any]:
    baseline = {
        "schema": BASELINE_SCHEMA,
        "run": {"number": 5203, "head_sha": "baseline-head"},
        "profile": "default",
        "features": ["native", "browser"],
        "records": [
            {"package": "p", "binary_label_as_rendered": None, "binary_id": "p", "selector": "pass", "status": "PASS", "feature": "native", "reporter_ordinal": 1},
            {"package": "p", "binary_label_as_rendered": "e2e", "binary_id": "p::e2e", "selector": "fail", "status": "PASS", "feature": "native", "reporter_ordinal": 2},
            {"package": "p", "binary_label_as_rendered": None, "binary_id": "p", "selector": "abort", "status": "PASS", "feature": "native", "reporter_ordinal": 3},
            {"package": "p", "binary_label_as_rendered": None, "binary_id": "p", "selector": "timeout", "status": "PASS", "feature": "browser", "reporter_ordinal": 4},
            {"package": "p", "binary_label_as_rendered": None, "binary_id": "p", "selector": "skip", "status": "PASS", "feature": "browser", "reporter_ordinal": 5},
            {"package": "p", "binary_label_as_rendered": None, "binary_id": "p", "selector": "missing", "status": "PASS", "feature": "browser", "reporter_ordinal": 6},
        ],
    }
    current = {
        "run": {"number": 9999, "head_sha": "synthetic-current"},
        "profile": "default",
        "features": ["native", "browser"],
        "records": [
            {"package": "p", "binary_id": "p", "selector": "pass", "status": "PASS", "feature": "native", "profile": "default", "raw": "preserved"},
            {"package": "p", "binary_id": "p::e2e", "selector": "fail", "status": "FAIL", "feature": "native", "profile": "default"},
            {"package": "p", "binary_id": "p", "selector": "abort", "status": "ABORT", "feature": "native"},
            {"package": "p", "binary_id": "p", "selector": "timeout", "status": "TIMEOUT", "feature": "browser"},
            {"package": "p", "binary_id": "p", "selector": "skip", "status": "SKIP", "feature": "browser"},
            {"package": "new", "binary_id": "new", "selector": "fresh", "status": "PASS", "feature": "native", "profile": "default"},
        ],
    }
    got = compare(baseline, current)
    assert got["classification_counts"] == {
        "matched_passes": 1,
        "regressions": 3,
        "unknown": 2,
        "extra_current_passes": 1,
        "unmatched_current_nonpass": 0,
    }
    assert {x["reason"] for x in got["regressions"]} == {"current_fail", "current_abort", "current_timeout"}
    assert {x["reason"] for x in got["unknown"]} == {"current_skip", "missing_current_record"}
    assert got["extra_current_passes"][0]["current_record"]["feature"] == "native"
    assert got["matched_passes"][0]["current_record"]["raw"] == "preserved"

    duplicate_current = copy.deepcopy(current)
    duplicate_current["records"].append(copy.deepcopy(duplicate_current["records"][0]))
    duplicate_result = compare(baseline, duplicate_current)
    assert duplicate_result["comparison_status"] == "fail_closed_duplicate_identity"
    assert duplicate_result["classification_counts"] is None
    assert duplicate_result["matched_passes"] == []

    duplicate_baseline = copy.deepcopy(baseline)
    duplicate_baseline["records"].append(copy.deepcopy(duplicate_baseline["records"][0]))
    duplicate_baseline_result = compare(duplicate_baseline, current)
    assert duplicate_baseline_result["comparison_status"] == "fail_closed_duplicate_identity"
    assert duplicate_baseline_result["classification_counts"] is None

    return {
        "synthetic_cases": {
            "baseline_to_current_pass": "matched_passes",
            "baseline_pass_to_fail_abort_timeout": "regressions",
            "baseline_pass_to_skip_or_missing": "unknown",
            "duplicate_baseline_or_current_key": "fail_closed_no_classifications",
            "new_current_pass": "extra_current_passes",
        },
        "synthetic_expected_counts": got["classification_counts"],
        "synthetic_regression_reasons": sorted(x["reason"] for x in got["regressions"]),
        "synthetic_unknown_reasons": sorted(x["reason"] for x in got["unknown"]),
        "duplicate_tests": {
            "current_duplicate_status": duplicate_result["comparison_status"],
            "baseline_duplicate_status": duplicate_baseline_result["comparison_status"],
            "both_emit_zero_classifications": True,
        },
        "provenance_and_rows_preserved": {
            "baseline_run": got["baseline_provenance"]["run"],
            "current_run": got["current_provenance"]["run"],
            "current_profile": got["current_provenance"]["profile"],
            "current_features": got["current_provenance"]["features"],
            "row_feature_profile_and_custom_fields": "preserved verbatim in each original record wrapper",
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", nargs="?", type=Path)
    parser.add_argument("current", nargs="?", type=Path)
    parser.add_argument("--self-test", action="store_true", help="run temporary synthetic validation")
    parser.add_argument("--output", type=Path, help="write comparison JSON; stdout if omitted")
    args = parser.parse_args()
    try:
        if args.self_test:
            result = self_test()
        else:
            if args.baseline is None or args.current is None:
                parser.error("baseline and current receipt paths are required unless --self-test is used")
            baseline = load_receipt(args.baseline, "baseline")
            current = load_receipt(args.current, "current")
            result = compare(baseline, current)
    except ReceiptError as exc:
        print(json.dumps({"schema": "exact-nextest-selector-comparison-v1", "comparison_status": "invalid_input", "error": str(exc)}, indent=2), file=sys.stderr)
        return 2
    encoded = json.dumps(result, indent=2, ensure_ascii=False) + "\n"
    if args.output:
        args.output.write_text(encoded, encoding="utf-8")
    else:
        sys.stdout.write(encoded)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
