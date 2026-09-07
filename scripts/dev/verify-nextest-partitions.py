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

"""Verify that three nextest listings are a disjoint, complete partition.

The listing format deliberately contains tests which are present in the
binary but excluded by the active profile, ignored-test mode, or a partition.
Only ``filter-match.status == matches`` is selected.  Test names are qualified
by their nextest ``binary-id`` because names are not workspace-unique.
"""

from __future__ import annotations

import hashlib
import json
import re
import sys
import tempfile
from pathlib import Path
from typing import Any

TestId = tuple[str, str]
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
PARTITION_COUNT = 3
PARTITIONS = tuple(
    f"hash:{index}/{PARTITION_COUNT}" for index in range(1, PARTITION_COUNT + 1)
)
LISTING_NAMES = {
    "all.json",
    *(f"{index}-{PARTITION_COUNT}.json" for index in range(1, PARTITION_COUNT + 1)),
}


def _load(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise ValueError(f"{path}: expected one nextest JSON listing: {exc}") from exc
    except OSError as exc:
        raise ValueError(f"{path}: cannot read listing: {exc}") from exc


def _selected(path: Path) -> set[TestId]:
    document = _load(path)
    suites = document.get("rust-suites") if isinstance(document, dict) else None
    if not isinstance(suites, dict):
        raise TypeError(f"{path}: missing rust-suites object")

    selected: set[TestId] = set()
    seen: set[TestId] = set()
    testcase_count = 0
    for suite_name, suite in suites.items():
        if not isinstance(suite, dict):
            raise TypeError(f"{path}: suite {suite_name!r} is not an object")
        binary_id = suite.get("binary-id")
        testcases = suite.get("testcases")
        if not isinstance(binary_id, str) or not binary_id:
            raise ValueError(f"{path}: suite {suite_name!r} has no binary-id")
        if not isinstance(testcases, dict):
            raise TypeError(f"{path}: suite {suite_name!r} has no testcases object")

        for test_name, testcase in testcases.items():
            testcase_count += 1
            if not isinstance(test_name, str) or not isinstance(testcase, dict):
                raise TypeError(f"{path}: malformed testcase in suite {suite_name!r}")
            match = testcase.get("filter-match")
            if not isinstance(match, dict) or "status" not in match:
                raise ValueError(
                    f"{path}: testcase {binary_id}:{test_name} has no filter-match.status"
                )
            # Do not infer selection from presence, ignored, or a testcase
            # status: nextest's filter decision is authoritative.  This keeps
            # repository/profile config and --run-ignored semantics intact.
            if match["status"] != "matches":
                continue
            test_id = (binary_id, test_name)
            if test_id in seen:
                raise ValueError(
                    f"{path}: duplicate selected testcase {binary_id}:{test_name}"
                )
            seen.add(test_id)
            selected.add(test_id)

    if testcase_count == 0:
        raise ValueError(f"{path}: listing contains no testcases")
    return selected


def _authoritative_selected(path: Path) -> set[TestId]:
    selected = _selected(path)
    if not selected:
        raise ValueError(f"{path}: authoritative listing selects no tests")
    return selected


def _verify_partition_sets(
    all_tests: set[TestId], partitions: list[tuple[str, set[TestId]]]
) -> None:
    names = [name for name, _ in partitions]
    if tuple(names) != PARTITIONS:
        raise ValueError(
            f"expected exactly the partitions {', '.join(PARTITIONS)}; got {', '.join(names)}"
        )
    union: set[TestId] = set()
    for name, selected in partitions:
        overlap = union & selected
        if overlap:
            raise ValueError(
                f"nextest partition proof found duplicate tests in {name}: "
                f"{sorted(overlap)[:5]}"
            )
        union.update(selected)
    missing = all_tests - union
    unexpected = union - all_tests
    if missing or unexpected:
        details = [
            "nextest partition proof failed",
            f"  missing ({len(missing)}): {sorted(missing)[:5]}",
            f"  unexpected ({len(unexpected)}): {sorted(unexpected)[:5]}",
        ]
        raise ValueError("\n".join(details))


def verify(all_path: Path, *partition_paths: Path) -> None:
    all_tests = _authoritative_selected(all_path)
    if len(partition_paths) != PARTITION_COUNT:
        raise ValueError(
            f"expected exactly {PARTITION_COUNT} partition listings, got {len(partition_paths)}"
        )
    selected = [_selected(path) for path in partition_paths]
    _verify_partition_sets(
        all_tests,
        [
            (f"hash:{index}/{PARTITION_COUNT}", tests)
            for index, tests in enumerate(selected, 1)
        ],
    )
    counts = " ".join(
        f"{index}/{PARTITION_COUNT}={len(tests)}"
        for index, tests in enumerate(selected, 1)
    )
    print(f"nextest partition proof: all={len(all_tests)} {counts}")


def _sha256(path: Path) -> str:
    try:
        digest = hashlib.sha256()
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
        return digest.hexdigest()
    except OSError as exc:
        raise ValueError(f"{path}: cannot hash file: {exc}") from exc


def _metadata(path: Path) -> dict[str, Any]:
    document = _load(path)
    if not isinstance(document, dict):
        raise TypeError(f"{path}: metadata must be an object")
    return document


def _require_common(metadata: dict[str, Any], path: Path) -> None:
    required = (
        "schema",
        "workspace_sha",
        "nextest_version",
        "package",
        "filter",
        "archive_sha256",
    )
    missing = [key for key in required if key not in metadata]
    if missing:
        raise ValueError(f"{path}: metadata missing {', '.join(missing)}")
    if metadata["schema"] != 1:
        raise ValueError(f"{path}: unsupported metadata schema {metadata['schema']!r}")
    for key in ("workspace_sha", "nextest_version", "package", "filter"):
        if not isinstance(metadata[key], str) or not metadata[key]:
            raise ValueError(f"{path}: metadata field {key} must be a non-empty string")
    archive_sha = metadata["archive_sha256"]
    if not isinstance(archive_sha, str) or not SHA256_RE.fullmatch(archive_sha):
        raise ValueError(f"{path}: metadata archive_sha256 is not a SHA-256 digest")


def _require_digest(path: Path, expected: str, label: str) -> None:
    actual = _sha256(path)
    if actual != expected:
        raise ValueError(
            f"{path}: {label} digest mismatch (expected {expected}, got {actual})"
        )


def _read_archive_digest(path: Path) -> str:
    try:
        fields = path.read_text(encoding="utf-8").split()
    except OSError as exc:
        raise ValueError(f"{path}: cannot read archive digest: {exc}") from exc
    if not fields or not SHA256_RE.fullmatch(fields[0]):
        raise ValueError(f"{path}: expected a SHA-256 digest")
    return fields[0]


def _require_listing_digest(
    metadata: dict[str, Any], path: Path, listing: Path, key: str
) -> None:
    listings = metadata.get("listings")
    if not isinstance(listings, dict) or not isinstance(listings.get(key), str):
        raise TypeError(f"{path}: metadata has no digest for {key}")
    digest = listings[key]
    if not SHA256_RE.fullmatch(digest):
        raise ValueError(f"{path}: listing digest for {key} is not SHA-256")
    _require_digest(listing, digest, f"listing {key}")


def _require_exact_listing_keys(metadata: dict[str, Any], path: Path) -> None:
    listings = metadata.get("listings")
    if not isinstance(listings, dict):
        raise TypeError(f"{path}: metadata has no listings object")
    actual = set(listings)
    if actual != LISTING_NAMES:
        missing = sorted(LISTING_NAMES - actual)
        extra = sorted(actual - LISTING_NAMES)
        raise ValueError(
            f"{path}: producer listings must be exactly {sorted(LISTING_NAMES)} "
            f"(missing={missing}, extra={extra})"
        )


def _matching_metadata(
    producer: dict[str, Any], consumer: dict[str, Any], path: Path, partition: str
) -> None:
    _require_common(consumer, path)
    for key in (
        "workspace_sha",
        "nextest_version",
        "package",
        "filter",
        "archive_sha256",
    ):
        if consumer[key] != producer[key]:
            raise ValueError(f"{path}: {key} does not match producer metadata")
    if consumer.get("kind") != "consumer" or consumer.get("partition") != partition:
        raise ValueError(f"{path}: expected consumer partition {partition}")
    if consumer.get("result") != "success":
        raise ValueError(f"{path}: consumer result is {consumer.get('result')!r}")
    if consumer.get("archive_verified") is not True:
        raise ValueError(f"{path}: consumer did not verify the downloaded archive")


def verify_results(proof_dir: Path, *partition_dirs: Path) -> None:
    """Verify the transferred proof and consumer result artifacts."""
    producer_meta_path = proof_dir / "producer-metadata.json"
    producer = _metadata(producer_meta_path)
    _require_common(producer, producer_meta_path)
    if (
        producer.get("kind") != "producer"
        or producer.get("partition") != "all"
        or producer.get("result") != "success"
    ):
        raise ValueError(f"{producer_meta_path}: invalid producer metadata")
    if len(partition_dirs) != PARTITION_COUNT:
        raise ValueError(
            f"expected exactly {PARTITION_COUNT} consumer result directories, "
            f"got {len(partition_dirs)}"
        )
    _require_exact_listing_keys(producer, producer_meta_path)

    archive_digest = _read_archive_digest(proof_dir / "archive.sha256")
    if archive_digest != producer["archive_sha256"]:
        raise ValueError(
            f"{producer_meta_path}: archive.sha256 does not match producer metadata"
        )
    try:
        version = (
            (proof_dir / "nextest-version.txt").read_text(encoding="utf-8").strip()
        )
    except OSError as exc:
        raise ValueError(
            f"{proof_dir}: cannot read nextest-version.txt: {exc}"
        ) from exc
    if version != producer["nextest_version"]:
        raise ValueError(
            f"{producer_meta_path}: nextest-version.txt does not match metadata"
        )

    all_path = proof_dir / "all.json"
    producer_listing_paths = [
        proof_dir / f"{index}-{PARTITION_COUNT}.json"
        for index in range(1, PARTITION_COUNT + 1)
    ]
    listing_items = [
        (all_path, "all.json"),
        *[
            (path, f"{index}-3.json")
            for index, path in enumerate(producer_listing_paths, 1)
        ],
    ]
    for listing, key in listing_items:
        _require_listing_digest(producer, producer_meta_path, listing, key)

    consumer_data: list[tuple[str, dict[str, Any], Path, Path]] = []
    for index, directory in enumerate(partition_dirs, 1):
        metadata_path = directory / "result-metadata.json"
        metadata = _metadata(metadata_path)
        partition = f"hash:{index}/{PARTITION_COUNT}"
        _matching_metadata(producer, metadata, metadata_path, partition)
        listing = directory / "selected.json"
        consumer_data.append((partition, metadata, metadata_path, listing))
    for _, metadata, metadata_path, listing in consumer_data:
        digest = metadata.get("selected_listing_sha256")
        if not isinstance(digest, str) or not SHA256_RE.fullmatch(digest):
            raise ValueError(f"{metadata_path}: selected listing digest is not SHA-256")
        _require_digest(listing, digest, "selected listing")

    all_tests = _authoritative_selected(all_path)
    producer_selected = [_selected(path) for path in producer_listing_paths]
    consumer_selected = [_selected(listing) for _, _, _, listing in consumer_data]
    _verify_partition_sets(
        all_tests,
        [(partition, tests) for partition, tests in zip(PARTITIONS, producer_selected)],
    )
    _verify_partition_sets(
        all_tests,
        [(partition, tests) for partition, tests in zip(PARTITIONS, consumer_selected)],
    )
    if consumer_selected != producer_selected:
        raise ValueError(
            "consumer selected listings differ from producer partition listings"
        )
    counts = " ".join(
        f"{index}/{PARTITION_COUNT}={len(tests)}"
        for index, tests in enumerate(consumer_selected, 1)
    )
    print(f"transferred nextest proof: all={len(all_tests)} {counts}")


def self_test() -> None:
    """Exercise selection against ignored and profile-filtered testcases."""
    all_fixture = {
        "rust-suites": {
            "server": {
                "binary-id": "tcl-lsp-server",
                "testcases": {
                    "ordinary": {
                        "ignored": False,
                        "filter-match": {"status": "matches"},
                    },
                    "ignored": {
                        "ignored": True,
                        "filter-match": {"status": "mismatch"},
                    },
                    "profile_filtered": {
                        "ignored": False,
                        "filter-match": {
                            "status": "mismatch",
                            "reason": "default-filter",
                        },
                    },
                },
            },
            "other-server": {
                "binary-id": "other-server",
                "testcases": {
                    "ordinary": {
                        "ignored": False,
                        "filter-match": {"status": "matches"},
                    },
                },
            },
        }
    }
    first_fixture = json.loads(json.dumps(all_fixture))
    del first_fixture["rust-suites"]["other-server"]["testcases"]["ordinary"][
        "filter-match"
    ]
    first_fixture["rust-suites"]["other-server"]["testcases"]["ordinary"][
        "filter-match"
    ] = {
        "status": "mismatch",
        "reason": "partition",
    }
    second_fixture = json.loads(json.dumps(all_fixture))
    del second_fixture["rust-suites"]["server"]["testcases"]["ordinary"]["filter-match"]
    second_fixture["rust-suites"]["server"]["testcases"]["ordinary"]["filter-match"] = {
        "status": "mismatch",
        "reason": "partition",
    }
    third_fixture = json.loads(json.dumps(all_fixture))
    for suite in third_fixture["rust-suites"].values():
        for testcase in suite["testcases"].values():
            testcase["filter-match"] = {
                "status": "mismatch",
                "reason": "partition",
            }
    fixtures = {
        "all": all_fixture,
        "first": first_fixture,
        "second": second_fixture,
        "third": third_fixture,
    }
    expected = {("tcl-lsp-server", "ordinary"), ("other-server", "ordinary")}
    path = Path("<self-test>")
    # Exercise the same parser and set assertions without creating files.
    original_load = globals()["_load"]
    globals()["_load"] = lambda current: (
        fixtures["all"] if current == path else fixtures[current.name]
    )
    try:
        assert _selected(path) == expected
        verify(Path("all"), Path("first"), Path("second"), Path("third"))

        for paths in (
            (Path("first"), Path("second")),
            (Path("first"), Path("second"), Path("third"), Path("third")),
        ):
            try:
                verify(Path("all"), *paths)
            except ValueError as exc:
                assert "exactly 3 partition listings" in str(exc)
            else:
                raise AssertionError("wrong number of partition listings passed")
        try:
            _verify_partition_sets(
                expected,
                [
                    ("hash:1/3", expected),
                    ("hash:1/3", set()),
                    ("hash:3/3", set()),
                ],
            )
        except ValueError as exc:
            assert "exactly the partitions" in str(exc)
        else:
            raise AssertionError("duplicate partition labels passed verification")

        empty_fixture = json.loads(json.dumps(all_fixture))
        for suite in empty_fixture["rust-suites"].values():
            for testcase in suite["testcases"].values():
                testcase["filter-match"] = {
                    "status": "mismatch",
                    "reason": "default-filter",
                }
        fixtures["empty"] = empty_fixture
        try:
            verify(Path("empty"), Path("empty"), Path("empty"), Path("empty"))
        except ValueError as exc:
            assert "authoritative listing selects no tests" in str(exc)
        else:
            raise AssertionError("empty authoritative selection passed verification")
    finally:
        globals()["_load"] = original_load
    print(
        "nextest partition verifier self-test: ok (ignored/config-filtered cases excluded)"
    )

    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        proof_dir = root / "proof"
        partition_dirs = [root / name for name in ("first", "second", "third")]
        proof_dir.mkdir()
        for directory in partition_dirs:
            directory.mkdir()
        archive_sha = hashlib.sha256(b"archive").hexdigest()
        (proof_dir / "archive.sha256").write_text(
            f"{archive_sha}  lsp-e2e.tar.zst\n", encoding="utf-8"
        )
        (proof_dir / "nextest-version.txt").write_text(
            "cargo-nextest 0.9.143\n", encoding="utf-8"
        )
        listing_paths = {
            "all.json": proof_dir / "all.json",
            "1-3.json": proof_dir / "1-3.json",
            "2-3.json": proof_dir / "2-3.json",
            "3-3.json": proof_dir / "3-3.json",
        }
        listing_paths["all.json"].write_text(json.dumps(all_fixture), encoding="utf-8")
        listing_paths["1-3.json"].write_text(
            json.dumps(first_fixture), encoding="utf-8"
        )
        listing_paths["2-3.json"].write_text(
            json.dumps(second_fixture), encoding="utf-8"
        )
        listing_paths["3-3.json"].write_text(
            json.dumps(third_fixture), encoding="utf-8"
        )
        common: dict[str, Any] = {
            "schema": 1,
            "workspace_sha": "abc123",
            "nextest_version": "cargo-nextest 0.9.143",
            "package": "tcl-lsp-server",
            "filter": "default",
            "archive_sha256": archive_sha,
        }
        producer_metadata = dict(common)
        producer_metadata.update(
            {
                "kind": "producer",
                "partition": "all",
                "result": "success",
                "listings": {
                    name: _sha256(path) for name, path in listing_paths.items()
                },
            }
        )
        (proof_dir / "producer-metadata.json").write_text(
            json.dumps(producer_metadata), encoding="utf-8"
        )
        for directory, partition, fixture in zip(
            partition_dirs, PARTITIONS, (first_fixture, second_fixture, third_fixture)
        ):
            listing = directory / "selected.json"
            listing.write_text(json.dumps(fixture), encoding="utf-8")
            metadata = dict(common)
            metadata.update(
                {
                    "kind": "consumer",
                    "partition": partition,
                    "result": "success",
                    "selected_listing_sha256": _sha256(listing),
                    "archive_verified": True,
                }
            )
            (directory / "result-metadata.json").write_text(
                json.dumps(metadata), encoding="utf-8"
            )
        assert not (proof_dir / "lsp-e2e.tar.zst").exists()
        verify_results(proof_dir, *partition_dirs)
        try:
            verify_results(proof_dir, *partition_dirs[:2])
        except ValueError as exc:
            assert "exactly 3 consumer result directories" in str(exc)
        else:
            raise AssertionError("wrong number of consumer results passed verification")

        extra_listing = proof_dir / "4-3.json"
        extra_listing.write_text(json.dumps(third_fixture), encoding="utf-8")
        producer_metadata["listings"]["4-3.json"] = _sha256(extra_listing)
        (proof_dir / "producer-metadata.json").write_text(
            json.dumps(producer_metadata), encoding="utf-8"
        )
        try:
            verify_results(proof_dir, *partition_dirs)
        except ValueError as exc:
            assert "producer listings must be exactly" in str(exc)
        else:
            raise AssertionError("extra producer partition passed verification")
    print("nextest transferred-artifact self-test: ok (digest/metadata/result cases)")


def main(argv: list[str]) -> int:
    if argv == ["--self-test"]:
        self_test()
        return 0
    if argv and argv[0] == "--verify-results":
        if len(argv) != PARTITION_COUNT + 2:
            print(
                f"usage: {sys.argv[0]} --verify-results "
                "PROOF_DIR FIRST_DIR SECOND_DIR THIRD_DIR",
                file=sys.stderr,
            )
            return 2
        try:
            verify_results(*(Path(arg) for arg in argv[1:]))
        except (TypeError, ValueError) as exc:
            print(exc, file=sys.stderr)
            return 1
        return 0
    if len(argv) != PARTITION_COUNT + 1:
        print(
            f"usage: {sys.argv[0]} ALL.json 1-3.json 2-3.json 3-3.json",
            file=sys.stderr,
        )
        print(f"       {sys.argv[0]} --self-test", file=sys.stderr)
        return 2
    try:
        verify(*(Path(arg) for arg in argv))
    except (TypeError, ValueError) as exc:
        print(exc, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
