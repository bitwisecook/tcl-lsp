#!/usr/bin/env python3
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

"""Verify a binary-aware partition of cargo-nextest test listings.

The manifest is deliberately independent of nextest's hash implementation:
it records the binary target assigned to each shard. Library and binary
targets are built and listed on every shard to retain workspace-wide feature
unification, while an integration-test target is built on its assigned shard
only. Tests from every target are selected only on their assigned shard.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


class VerificationError(ValueError):
    """An input does not satisfy the shard proof contract."""


@dataclass(frozen=True, order=True)
class Target:
    binary_id: str
    kind: str
    package: str
    target: str


@dataclass(frozen=True)
class Manifest:
    partitions: int
    excluded: frozenset[str]
    assignments: dict[str, tuple[Target, int]]


def _read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        raise VerificationError(f"{path}: cannot read JSON: {exc}") from exc
    except (UnicodeError, json.JSONDecodeError) as exc:
        raise VerificationError(f"{path}: malformed JSON: {exc}") from exc


def _nonempty_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise VerificationError(f"{label} must be a non-empty string")
    return value


def _binary_id(package: str, kind: str, target: str) -> str:
    if kind == "lib":
        return package
    if kind == "bin":
        return f"{package}::bin/{target}"
    return f"{package}::{target}"


def _metadata_targets(path: Path, excluded: frozenset[str]) -> dict[str, Target]:
    document = _read_json(path)
    if not isinstance(document, dict):
        raise VerificationError(f"{path}: cargo metadata must be an object")
    packages = document.get("packages")
    members = document.get("workspace_members")
    if not isinstance(packages, list) or not packages:
        raise VerificationError(f"{path}: cargo metadata has no packages")
    if not isinstance(members, list) or not members:
        raise VerificationError(f"{path}: cargo metadata has no workspace_members")
    member_ids: list[str] = []
    for index, member in enumerate(members):
        member_ids.append(
            _nonempty_string(member, f"{path}: workspace_members[{index}]")
        )
    if len(set(member_ids)) != len(member_ids):
        raise VerificationError(f"{path}: duplicate workspace member IDs")
    by_id: dict[str, dict[str, Any]] = {}
    for index, package in enumerate(packages):
        if not isinstance(package, dict):
            raise VerificationError(f"{path}: packages[{index}] is not an object")
        package_id = _nonempty_string(package.get("id"), f"{path}: package id")
        if package_id in by_id:
            raise VerificationError(f"{path}: duplicate package id {package_id!r}")
        by_id[package_id] = package

    targets: dict[str, Target] = {}
    seen_workspace_names: set[str] = set()
    for member_id in member_ids:
        package = by_id.get(member_id)
        if package is None:
            raise VerificationError(
                f"{path}: workspace member {member_id!r} is absent from packages"
            )
        package_name = _nonempty_string(
            package.get("name"), f"{path}: package {member_id} name"
        )
        if package_name in seen_workspace_names:
            raise VerificationError(
                f"{path}: duplicate workspace package name {package_name!r}"
            )
        seen_workspace_names.add(package_name)
        if package_name in excluded:
            continue
        package_targets = package.get("targets")
        if not isinstance(package_targets, list):
            raise VerificationError(
                f"{path}: package {package_name!r} has no targets list"
            )
        has_common_harness = False
        for target_index, target_data in enumerate(package_targets):
            label = f"{path}: package {package_name!r} target {target_index}"
            if not isinstance(target_data, dict):
                raise VerificationError(f"{label} is not an object")
            if target_data.get("test") is not True:
                continue
            kinds = target_data.get("kind")
            if not isinstance(kinds, list) or len(kinds) != 1:
                raise VerificationError(f"{label} has malformed kind")
            kind = kinds[0]
            if kind not in {"lib", "bin", "test"}:
                continue
            if kind in {"lib", "bin"}:
                has_common_harness = True
            target_name = _nonempty_string(target_data.get("name"), f"{label} name")
            binary_id = _binary_id(package_name, kind, target_name)
            target = Target(binary_id, kind, package_name, target_name)
            if binary_id in targets:
                raise VerificationError(
                    f"{path}: duplicate eligible binary ID {binary_id!r}"
                )
            targets[binary_id] = target
        if not has_common_harness:
            raise VerificationError(
                f"{path}: included workspace package {package_name!r} has no testable lib/bin harness; "
                "the binary-aware runner cannot preserve its all-features test graph on every shard"
            )

    unknown_exclusions = excluded - seen_workspace_names
    if unknown_exclusions:
        names = ", ".join(sorted(unknown_exclusions))
        raise VerificationError(
            f"{path}: exclusions name non-workspace packages: {names}"
        )
    if not targets:
        raise VerificationError(f"{path}: eligible binary universe is empty")
    return targets


def _manifest(path: Path) -> Manifest:
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as exc:
        raise VerificationError(f"{path}: cannot read manifest: {exc}") from exc
    except UnicodeError as exc:
        raise VerificationError(f"{path}: manifest is not UTF-8: {exc}") from exc

    partitions: int | None = None
    excluded: set[str] = set()
    assignments: dict[str, tuple[Target, int]] = {}
    rows_started = False
    for line_number, raw_line in enumerate(lines, 1):
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        fields = raw_line.split("\t")
        if fields[0].startswith("@"):
            if rows_started:
                raise VerificationError(
                    f"{path}:{line_number}: directives must precede target rows"
                )
            if fields[0] == "@partitions":
                if len(fields) != 2 or partitions is not None or excluded:
                    raise VerificationError(
                        f"{path}:{line_number}: malformed or duplicate @partitions"
                    )
                try:
                    partitions = int(fields[1])
                except ValueError as exc:
                    raise VerificationError(
                        f"{path}:{line_number}: @partitions must be an integer"
                    ) from exc
                if partitions < 1:
                    raise VerificationError(
                        f"{path}:{line_number}: @partitions must be positive"
                    )
            elif fields[0] == "@exclude":
                if partitions is None or len(fields) != 2 or not fields[1]:
                    raise VerificationError(f"{path}:{line_number}: malformed @exclude")
                if fields[1] in excluded:
                    raise VerificationError(
                        f"{path}:{line_number}: duplicate exclusion {fields[1]!r}"
                    )
                excluded.add(fields[1])
            else:
                raise VerificationError(
                    f"{path}:{line_number}: unknown directive {fields[0]!r}"
                )
            continue
        if partitions is None:
            raise VerificationError(
                f"{path}:{line_number}: target row precedes @partitions"
            )
        if len(fields) != 5 or any(not field for field in fields):
            raise VerificationError(
                f"{path}:{line_number}: expected SHARD, BINARY_ID, KIND, PACKAGE, TARGET"
            )
        shard_text, binary_id, kind, package, target_name = fields
        try:
            shard = int(shard_text)
        except ValueError as exc:
            raise VerificationError(
                f"{path}:{line_number}: shard must be an integer"
            ) from exc
        if shard < 1 or shard > partitions:
            raise VerificationError(
                f"{path}:{line_number}: shard {shard} is outside 1..{partitions}"
            )
        if kind not in {"lib", "bin", "test"}:
            raise VerificationError(
                f"{path}:{line_number}: unsupported target kind {kind!r}"
            )
        if binary_id in assignments:
            raise VerificationError(
                f"{path}:{line_number}: duplicate binary assignment {binary_id!r}"
            )
        rows_started = True
        assignments[binary_id] = (Target(binary_id, kind, package, target_name), shard)
    if partitions is None:
        raise VerificationError(f"{path}: missing @partitions directive")
    if not assignments:
        raise VerificationError(f"{path}: manifest has no target rows")
    populated = {shard for _, shard in assignments.values()}
    empty = sorted(set(range(1, partitions + 1)) - populated)
    if empty:
        raise VerificationError(f"{path}: partitions contain no target rows: {empty}")
    return Manifest(partitions, frozenset(excluded), assignments)


def _suite_target(
    path: Path, suite_name: Any, suite: Any, targets: dict[str, Target]
) -> tuple[Target, dict[str, Any]]:
    if not isinstance(suite_name, str) or not suite_name:
        raise VerificationError(f"{path}: suite key must be a non-empty string")
    if not isinstance(suite, dict):
        raise VerificationError(f"{path}: suite {suite_name!r} is not an object")
    binary_id = _nonempty_string(
        suite.get("binary-id"), f"{path}: suite {suite_name!r} binary-id"
    )
    if suite_name != binary_id:
        raise VerificationError(
            f"{path}: suite key {suite_name!r} does not equal binary-id {binary_id!r}"
        )
    target = targets.get(binary_id)
    if target is None:
        raise VerificationError(
            f"{path}: suite {binary_id!r} is not an eligible cargo target"
        )
    expected = {
        "package-name": target.package,
        "binary-id": target.binary_id,
        "kind": target.kind,
        "binary-name": target.target,
    }
    for field, wanted in expected.items():
        actual = suite.get(field)
        if actual != wanted:
            raise VerificationError(
                f"{path}: suite {binary_id!r} metadata {field}={actual!r}, expected {wanted!r}"
            )
    testcases = suite.get("testcases")
    if not isinstance(testcases, dict):
        raise VerificationError(f"{path}: suite {binary_id!r} has no testcases object")
    return target, testcases


def _verify_listing(
    path: Path,
    shard: int,
    manifest: Manifest,
    targets: dict[str, Target],
) -> set[tuple[str, str]]:
    document = _read_json(path)
    if not isinstance(document, dict):
        raise VerificationError(f"{path}: nextest listing must be an object")
    suites = document.get("rust-suites")
    if not isinstance(suites, dict) or not suites:
        raise VerificationError(f"{path}: listing has no rust-suites")
    declared_count = document.get("test-count")
    if type(declared_count) is not int or declared_count < 1:
        raise VerificationError(f"{path}: listing has malformed test-count")
    seen_binary_ids: set[str] = set()
    selected: set[tuple[str, str]] = set()
    testcase_count = 0
    for suite_name, suite in suites.items():
        target, testcases = _suite_target(path, suite_name, suite, targets)
        if target.binary_id in seen_binary_ids:
            raise VerificationError(
                f"{path}: duplicate suite for binary {target.binary_id!r}"
            )
        seen_binary_ids.add(target.binary_id)
        assigned = manifest.assignments[target.binary_id][1]
        expected_here = assigned == shard
        for test_name, testcase in testcases.items():
            testcase_count += 1
            if not isinstance(test_name, str) or not test_name:
                raise VerificationError(
                    f"{path}: suite {target.binary_id!r} has malformed testcase name"
                )
            if not isinstance(testcase, dict):
                raise VerificationError(
                    f"{path}: testcase {target.binary_id}:{test_name} is not an object"
                )
            ignored = testcase.get("ignored")
            if type(ignored) is not bool:
                raise VerificationError(
                    f"{path}: testcase {target.binary_id}:{test_name} has malformed ignored flag"
                )
            match = testcase.get("filter-match")
            if not isinstance(match, dict) or type(match.get("status")) is not str:
                raise VerificationError(
                    f"{path}: testcase {target.binary_id}:{test_name} has malformed filter-match.status"
                )
            status = match["status"]
            if status not in {"matches", "mismatch"}:
                raise VerificationError(
                    f"{path}: testcase {target.binary_id}:{test_name} has unknown filter status {status!r}"
                )
            if ignored:
                if status == "matches":
                    raise VerificationError(
                        f"{path}: ignored testcase selected: {target.binary_id}:{test_name}"
                    )
            elif expected_here:
                if status != "matches":
                    raise VerificationError(
                        f"{path}: assigned testcase is not selected: {target.binary_id}:{test_name}"
                    )
                selected.add((target.binary_id, test_name))
            elif status == "matches":
                raise VerificationError(
                    f"{path}: testcase selected in wrong shard: {target.binary_id}:{test_name}"
                )

    for binary_id, (_, assigned) in manifest.assignments.items():
        target = targets[binary_id]
        if target.kind in {"lib", "bin"} or assigned == shard:
            if binary_id not in seen_binary_ids:
                raise VerificationError(f"{path}: assigned suite missing: {binary_id}")
    if testcase_count == 0:
        raise VerificationError(f"{path}: listing contains no testcases")
    if declared_count != testcase_count:
        raise VerificationError(
            f"{path}: test-count is {declared_count}, but listing contains {testcase_count} testcases"
        )
    return selected


def verify(
    metadata_path: Path,
    manifest_path: Path,
    listing_paths: list[Path],
    partition_count: int | None = None,
    metadata_only: bool = False,
) -> None:
    manifest = _manifest(manifest_path)
    if partition_count is not None:
        if partition_count < 1:
            raise VerificationError("--partition-count must be positive")
        if partition_count != manifest.partitions:
            raise VerificationError(
                f"--partition-count is {partition_count}, manifest declares {manifest.partitions}"
            )
    targets = _metadata_targets(metadata_path, manifest.excluded)
    manifest_ids = set(manifest.assignments)
    metadata_ids = set(targets)
    missing = metadata_ids - manifest_ids
    unexpected = manifest_ids - metadata_ids
    if missing or unexpected:
        details = ["manifest does not exactly cover eligible cargo binaries"]
        if missing:
            details.append(f"missing ({len(missing)}): {sorted(missing)}")
        if unexpected:
            details.append(f"unexpected ({len(unexpected)}): {sorted(unexpected)}")
        raise VerificationError("\n".join(details))
    for binary_id, (manifest_target, _) in manifest.assignments.items():
        if manifest_target != targets[binary_id]:
            raise VerificationError(
                f"manifest metadata for {binary_id!r} does not match cargo metadata: "
                f"{manifest_target} != {targets[binary_id]}"
            )

    if metadata_only:
        if listing_paths:
            raise VerificationError("--metadata-only does not accept listing paths")
        print(
            f"nextest binary manifest proof: targets={len(targets)} "
            f"partitions={manifest.partitions}"
        )
        return
    if len(listing_paths) != manifest.partitions:
        raise VerificationError(
            f"expected exactly {manifest.partitions} listings, got {len(listing_paths)}"
        )

    selected_by_shard: list[set[tuple[str, str]]] = []
    for shard, path in enumerate(listing_paths, 1):
        selected_by_shard.append(_verify_listing(path, shard, manifest, targets))
    union: set[tuple[str, str]] = set()
    for shard, selected in enumerate(selected_by_shard, 1):
        overlap = union & selected
        if overlap:
            raise VerificationError(
                f"selected testcase IDs overlap between shards at {shard}: {sorted(overlap)[:5]}"
            )
        union.update(selected)
    counts = " ".join(
        f"{shard}/{manifest.partitions}={len(selected)}"
        for shard, selected in enumerate(selected_by_shard, 1)
    )
    print(f"nextest binary shard proof: selected={len(union)} {counts}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="verify a cargo metadata/TSV manifest and binary-aware nextest listings"
    )
    parser.add_argument("metadata", type=Path, help="cargo metadata JSON")
    parser.add_argument("manifest", type=Path, help="binary shard TSV manifest")
    parser.add_argument("listings", type=Path, nargs="*", help="one listing per shard")
    parser.add_argument(
        "--partition-count",
        type=int,
        help="optional expected partition count (must match @partitions)",
    )
    parser.add_argument(
        "--metadata-only",
        action="store_true",
        help="check only that the manifest exactly covers locked Cargo metadata",
    )
    args = parser.parse_args(argv)
    try:
        verify(
            args.metadata,
            args.manifest,
            args.listings,
            args.partition_count,
            args.metadata_only,
        )
    except VerificationError as exc:
        print(f"nextest binary shard proof failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
