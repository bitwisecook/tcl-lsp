#!/usr/bin/env python3
"""Verify that two nextest listings are a disjoint, complete partition.

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
        raise ValueError(f"{path}: missing rust-suites object")

    selected: set[TestId] = set()
    seen: set[TestId] = set()
    testcase_count = 0
    for suite_name, suite in suites.items():
        if not isinstance(suite, dict):
            raise ValueError(f"{path}: suite {suite_name!r} is not an object")
        binary_id = suite.get("binary-id")
        testcases = suite.get("testcases")
        if not isinstance(binary_id, str) or not binary_id:
            raise ValueError(f"{path}: suite {suite_name!r} has no binary-id")
        if not isinstance(testcases, dict):
            raise ValueError(f"{path}: suite {suite_name!r} has no testcases object")

        for test_name, testcase in testcases.items():
            testcase_count += 1
            if not isinstance(test_name, str) or not isinstance(testcase, dict):
                raise ValueError(f"{path}: malformed testcase in suite {suite_name!r}")
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


def verify(all_path: Path, first_path: Path, second_path: Path) -> None:
    all_tests = _selected(all_path)
    first = _selected(first_path)
    second = _selected(second_path)
    overlap = first & second
    missing = all_tests - (first | second)
    unexpected = (first | second) - all_tests
    if overlap or missing or unexpected:
        details = [
            "nextest partition proof failed",
            f"  overlap ({len(overlap)}): {sorted(overlap)[:5]}",
            f"  missing ({len(missing)}): {sorted(missing)[:5]}",
            f"  unexpected ({len(unexpected)}): {sorted(unexpected)[:5]}",
        ]
        raise ValueError("\n".join(details))
    print(
        f"nextest partition proof: all={len(all_tests)} 1/2={len(first)} 2/2={len(second)}"
    )


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
        raise ValueError(f"{path}: metadata must be an object")
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
        raise ValueError(f"{path}: metadata has no digest for {key}")
    digest = listings[key]
    if not SHA256_RE.fullmatch(digest):
        raise ValueError(f"{path}: listing digest for {key} is not SHA-256")
    _require_digest(listing, digest, f"listing {key}")


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


def verify_results(proof_dir: Path, first_dir: Path, second_dir: Path) -> None:
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
    producer_first_path = proof_dir / "1-2.json"
    producer_second_path = proof_dir / "2-2.json"
    for listing, key in (
        (all_path, "all.json"),
        (producer_first_path, "1-2.json"),
        (producer_second_path, "2-2.json"),
    ):
        _require_listing_digest(producer, producer_meta_path, listing, key)

    first_meta_path = first_dir / "result-metadata.json"
    second_meta_path = second_dir / "result-metadata.json"
    first_meta = _metadata(first_meta_path)
    second_meta = _metadata(second_meta_path)
    _matching_metadata(producer, first_meta, first_meta_path, "hash:1/2")
    _matching_metadata(producer, second_meta, second_meta_path, "hash:2/2")
    first_listing = first_dir / "selected.json"
    second_listing = second_dir / "selected.json"
    for metadata, metadata_path, listing in (
        (first_meta, first_meta_path, first_listing),
        (second_meta, second_meta_path, second_listing),
    ):
        digest = metadata.get("selected_listing_sha256")
        if not isinstance(digest, str) or not SHA256_RE.fullmatch(digest):
            raise ValueError(f"{metadata_path}: selected listing digest is not SHA-256")
        _require_digest(listing, digest, "selected listing")

    all_tests = _selected(all_path)
    producer_first = _selected(producer_first_path)
    producer_second = _selected(producer_second_path)
    first = _selected(first_listing)
    second = _selected(second_listing)
    if (
        producer_first & producer_second
        or (producer_first | producer_second) != all_tests
    ):
        raise ValueError(
            "producer hash-partition listings are not a disjoint complete union"
        )
    if first & second or (first | second) != all_tests:
        raise ValueError(
            "consumer hash-partition listings are not a disjoint complete union"
        )
    if first != producer_first or second != producer_second:
        raise ValueError(
            "consumer selected listings differ from producer partition listings"
        )
    print(
        f"transferred nextest proof: all={len(all_tests)} 1/2={len(first)} 2/2={len(second)}"
    )


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
    fixtures = {"all": all_fixture, "first": first_fixture, "second": second_fixture}
    expected = {("tcl-lsp-server", "ordinary"), ("other-server", "ordinary")}
    path = Path("<self-test>")
    # Exercise the same parser and set assertions without creating files.
    original_load = globals()["_load"]
    globals()["_load"] = lambda current: (
        fixtures["all"] if current == path else fixtures[current.name]
    )
    try:
        assert _selected(path) == expected
        verify(Path("all"), Path("first"), Path("second"))
    finally:
        globals()["_load"] = original_load
    print(
        "nextest partition verifier self-test: ok (ignored/config-filtered cases excluded)"
    )

    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        proof_dir = root / "proof"
        first_dir = root / "first"
        second_dir = root / "second"
        proof_dir.mkdir()
        first_dir.mkdir()
        second_dir.mkdir()
        archive_sha = hashlib.sha256(b"archive").hexdigest()
        (proof_dir / "archive.sha256").write_text(
            f"{archive_sha}  lsp-e2e.tar.zst\n", encoding="utf-8"
        )
        (proof_dir / "nextest-version.txt").write_text(
            "cargo-nextest 0.9.143\n", encoding="utf-8"
        )
        listing_paths = {
            "all.json": proof_dir / "all.json",
            "1-2.json": proof_dir / "1-2.json",
            "2-2.json": proof_dir / "2-2.json",
        }
        listing_paths["all.json"].write_text(json.dumps(all_fixture), encoding="utf-8")
        listing_paths["1-2.json"].write_text(
            json.dumps(first_fixture), encoding="utf-8"
        )
        listing_paths["2-2.json"].write_text(
            json.dumps(second_fixture), encoding="utf-8"
        )
        common = {
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
        for directory, partition, fixture in (
            (first_dir, "hash:1/2", first_fixture),
            (second_dir, "hash:2/2", second_fixture),
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
        verify_results(proof_dir, first_dir, second_dir)
    print("nextest transferred-artifact self-test: ok (digest/metadata/result cases)")


def main(argv: list[str]) -> int:
    if argv == ["--self-test"]:
        self_test()
        return 0
    if argv and argv[0] == "--verify-results":
        if len(argv) != 4:
            print(
                f"usage: {sys.argv[0]} --verify-results PROOF_DIR FIRST_DIR SECOND_DIR",
                file=sys.stderr,
            )
            return 2
        try:
            verify_results(*(Path(arg) for arg in argv[1:]))
        except ValueError as exc:
            print(exc, file=sys.stderr)
            return 1
        return 0
    if len(argv) != 3:
        print(f"usage: {sys.argv[0]} ALL.json FIRST.json SECOND.json", file=sys.stderr)
        print(f"       {sys.argv[0]} --self-test", file=sys.stderr)
        return 2
    try:
        verify(*(Path(arg) for arg in argv))
    except ValueError as exc:
        print(exc, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
