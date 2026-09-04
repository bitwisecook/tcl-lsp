#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Validate the Cargo target ownership declared by smoke-targets.tsv."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SUPPORTED_TARGET_KINDS = {"lib", "bin", "test", "example", "bench"}


@dataclass(frozen=True)
class Target:
    package: str
    kind: str
    name: str
    source: Path
    available_in_workspace: bool = True
    testable: bool = True
    required_features: tuple[str, ...] = ()
    resolved_features: tuple[str, ...] = ()

    @property
    def manifest_name(self) -> str:
        return self.name


def ownership_rank(source: Path, target: Target) -> int | None:
    """Return how specifically *target* owns *source*, or None if it cannot."""
    root = target.source
    if source == root:
        return 3

    # Rust's conventional sibling module tree for `foo.rs` is `foo/`.
    if root.suffix == ".rs" and source.is_relative_to(root.with_suffix("")):
        return 2

    if target.kind in {"test", "example", "bench"}:
        return None

    # A `src/bin/foo/main.rs` binary owns the rest of its directory. A
    # top-level lib.rs/main.rs can include sibling modules, but that ownership
    # is deliberately low priority because a package containing both targets
    # is ambiguous without following the Rust module graph.
    if root.name == "main.rs" and root.parent.parent.name == "bin":
        return 2 if source.is_relative_to(root.parent) else None
    if root.name in {"lib.rs", "main.rs"} and source.is_relative_to(root.parent):
        return 1
    return None


def best_owners(source: Path, targets: list[Target]) -> list[Target]:
    ranked = [
        (rank, target) for target in targets if (rank := ownership_rank(source, target))
    ]
    if not ranked:
        return []
    best = max(rank for rank, _target in ranked)
    return [target for rank, target in ranked if rank == best]


def metadata_command(host: str) -> list[str]:
    """Build the host-filtered Cargo metadata command used by the gate."""
    return [
        "cargo",
        "metadata",
        "--locked",
        "--format-version",
        "1",
        "--filter-platform",
        host,
    ]


def rustc_host() -> str:
    """Return the host triple Cargo uses for this local smoke run."""
    result = subprocess.run(
        ["rustc", "-vV"],
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    for line in result.stdout.splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ")
    raise RuntimeError("rustc -vV did not report a host triple")


def load_targets() -> tuple[dict[str, Path], list[Target]]:
    result = subprocess.run(
        metadata_command(rustc_host()),
        cwd=REPO_ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    metadata = json.loads(result.stdout)
    workspace_members = set(metadata["workspace_members"])
    resolved_features = {
        node["id"]: set(node["features"]) for node in metadata["resolve"]["nodes"]
    }
    manifests: dict[str, Path] = {}
    targets: list[Target] = []
    for package in metadata["packages"]:
        if package["id"] not in workspace_members:
            continue
        package_name = package["name"]
        enabled_features = resolved_features[package["id"]]
        manifests[package_name] = Path(package["manifest_path"]).resolve().parent
        for raw_target in package["targets"]:
            supported = SUPPORTED_TARGET_KINDS.intersection(raw_target["kind"])
            required_features = tuple(raw_target.get("required-features", []))
            for kind in supported:
                targets.append(
                    Target(
                        package=package_name,
                        kind=kind,
                        name=raw_target["name"],
                        source=Path(raw_target["src_path"]).resolve(),
                        available_in_workspace=set(required_features).issubset(
                            enabled_features
                        ),
                        testable=bool(raw_target.get("test", True)),
                        required_features=required_features,
                        resolved_features=tuple(
                            sorted(enabled_features.difference({"default"}))
                        ),
                    )
                )
    return manifests, targets


def smoke_named_targets(targets: list[Target]) -> set[tuple[str, str, str]]:
    """Cargo test targets selected wholesale by nextest's binary filter."""
    return {
        (target.package, target.kind, target.manifest_name)
        for target in targets
        if target.available_in_workspace
        and target.testable
        and (target.name == "smoke" or target.name.endswith("_smoke"))
    }


def smoke_named_target_sources(targets: list[Target]) -> set[Path]:
    """Source roots selected wholesale by nextest's smoke binary filter."""
    return {
        target.source
        for target in targets
        if target.available_in_workspace
        and target.testable
        and (target.name == "smoke" or target.name.endswith("_smoke"))
    }


def validate(manifest: Path) -> list[str]:
    package_roots, all_targets = load_targets()
    errors: list[str] = []
    declared_targets: set[tuple[str, str, str]] = set()
    for line_number, raw_line in enumerate(manifest.read_text().splitlines(), 1):
        if not raw_line or raw_line.startswith("#"):
            continue
        fields = raw_line.split("\t")
        if len(fields) != 4:
            errors.append(
                f"{manifest}:{line_number}: expected four tab-separated fields"
            )
            continue
        source_text, package, kind, target_name = fields
        declared_targets.add((package, kind, target_name))
        source = (REPO_ROOT / source_text).resolve()
        if not source.is_file():
            errors.append(f"missing smoke source: {source_text}")
            continue
        package_root = package_roots.get(package)
        if package_root is None:
            errors.append(f"unknown Cargo package '{package}' for {source_text}")
            continue
        if not source.is_relative_to(package_root):
            errors.append(
                f"smoke source {source_text} does not belong to package '{package}'"
            )
            continue
        if kind not in SUPPORTED_TARGET_KINDS:
            errors.append(f"invalid smoke target kind '{kind}' for {source_text}")
            continue
        package_targets = [
            target
            for target in all_targets
            if target.package == package and target.kind in SUPPORTED_TARGET_KINDS
        ]
        owners = best_owners(source, package_targets)
        declared = [
            owner
            for owner in owners
            if owner.kind == kind and owner.manifest_name == target_name
        ]
        if len(declared) == 1 and len(owners) == 1:
            if not declared[0].testable:
                errors.append(
                    f"smoke source {source_text} belongs to Cargo target "
                    f"{kind}:{target_name} with test = false"
                )
            continue

        if not owners:
            errors.append(f"no Cargo target owns smoke source {source_text}")
        elif len(owners) > 1:
            names = ", ".join(f"{owner.kind}:{owner.manifest_name}" for owner in owners)
            errors.append(
                f"ambiguous Cargo target ownership for {source_text}: {names}; "
                "move the smoke test to a target root or integration test"
            )
        else:
            owner = owners[0]
            errors.append(
                f"smoke source {source_text} belongs to "
                f"{owner.kind}:{owner.manifest_name}, not {kind}:{target_name}"
            )

    for package, kind, target_name in sorted(
        smoke_named_targets(all_targets) - declared_targets
    ):
        errors.append(
            f"smoke-named Cargo target {package} {kind}:{target_name} has no "
            "smoke-targets.tsv row"
        )
    return errors


def runnable_manifest_lines_for_targets(
    manifest: Path, targets: list[Target]
) -> list[str]:
    """Add the exact features needed to run each workspace-enabled row."""
    available = {
        (target.package, target.kind, target.manifest_name): target
        for target in targets
        if target.available_in_workspace and target.testable
    }
    lines: list[str] = []
    for raw_line in manifest.read_text().splitlines():
        if not raw_line or raw_line.startswith("#"):
            continue
        source, package, kind, target_name = raw_line.split("\t")
        if target := available.get((package, kind, target_name)):
            features = ",".join(target.resolved_features) or "-"
            lines.append("\t".join((source, package, kind, target_name, features)))
    return lines


def runnable_manifest_lines(manifest: Path) -> list[str]:
    """Resolve and render the Cargo invocations for a validated manifest."""
    _package_roots, targets = load_targets()
    return runnable_manifest_lines_for_targets(manifest, targets)


def self_test() -> None:
    root = Path("/repo/example")
    targets = [
        Target("example", "lib", "example", root / "src/lib.rs"),
        Target("example", "bin", "example", root / "src/main.rs"),
        Target("example", "bin", "worker", root / "src/bin/worker.rs"),
        Target("example", "test", "cli", root / "tests/cli.rs"),
        Target("example", "test", "new", root / "tests/new.rs"),
    ]

    # A source must map to its own integration target, not merely to any
    # existing target in the package.
    owners = best_owners(root / "tests/new.rs", targets)
    assert [(owner.kind, owner.name) for owner in owners] == [("test", "new")]

    # Binary roots and their conventional module directories must never be
    # accepted as library-owned smoke sources.
    owners = best_owners(root / "src/main.rs", targets)
    assert [(owner.kind, owner.name) for owner in owners] == [("bin", "example")]
    owners = best_owners(root / "src/bin/worker/helper.rs", targets)
    assert [(owner.kind, owner.name) for owner in owners] == [("bin", "worker")]

    # A sibling module shared by lib.rs and main.rs is intentionally rejected
    # as ambiguous rather than letting the fallback silently choose --lib.
    owners = best_owners(root / "src/shared.rs", targets)
    assert {(owner.kind, owner.name) for owner in owners} == {
        ("lib", "example"),
        ("bin", "example"),
    }

    # Nextest's binary selector runs every unit test in a smoke-named Cargo
    # binary, even when none of its functions has a smoke-shaped name.
    smoke_targets = targets + [
        Target("example", "bin", "smoke", root / "src/bin/smoke.rs"),
        Target("example", "bin", "worker_smoke", root / "src/bin/worker_smoke.rs"),
        Target("example", "bin", "smokeless", root / "src/bin/smokeless.rs"),
        Target("example", "test", "api_smoke", root / "integration/api.rs"),
        Target("example", "example", "demo_smoke", root / "examples/demo.rs"),
        Target("example", "bench", "parse_smoke", root / "benches/parse.rs"),
        Target(
            "example",
            "bin",
            "unified_smoke",
            root / "src/bin/unified_smoke.rs",
            required_features=("shared",),
            resolved_features=("also_unified", "shared"),
        ),
        Target(
            "example",
            "bin",
            "feature_smoke",
            root / "src/bin/feature_smoke.rs",
            available_in_workspace=False,
        ),
        Target(
            "example",
            "example",
            "disabled_smoke",
            root / "examples/disabled.rs",
            testable=False,
        ),
    ]
    assert smoke_named_targets(smoke_targets) == {
        ("example", "bin", "smoke"),
        ("example", "bin", "worker_smoke"),
        ("example", "test", "api_smoke"),
        ("example", "example", "demo_smoke"),
        ("example", "bench", "parse_smoke"),
        ("example", "bin", "unified_smoke"),
    }
    assert smoke_named_target_sources(smoke_targets) == {
        root / "src/bin/smoke.rs",
        root / "src/bin/worker_smoke.rs",
        root / "integration/api.rs",
        root / "examples/demo.rs",
        root / "benches/parse.rs",
        root / "src/bin/unified_smoke.rs",
    }

    # A name match is not runnable when Cargo's resolved workspace feature set
    # does not satisfy the target's required features.
    assert ("example", "bin", "feature_smoke") not in smoke_named_targets(smoke_targets)

    # A feature enabled through workspace unification must be passed explicitly
    # when the targeted runner invokes just this package.
    with tempfile.TemporaryDirectory() as temp_dir:
        manifest = Path(temp_dir) / "smoke-targets.tsv"
        manifest.write_text("source.rs\texample\tbin\tunified_smoke\n")
        assert runnable_manifest_lines_for_targets(manifest, smoke_targets) == [
            "source.rs\texample\tbin\tunified_smoke\talso_unified,shared"
        ]

    assert metadata_command("x86_64-example-linux")[-2:] == [
        "--filter-platform",
        "x86_64-example-linux",
    ]


def main() -> int:
    if sys.argv[1:] == ["--self-test"]:
        self_test()
        return 0
    if sys.argv[1:] == ["--smoke-target-sources"]:
        _package_roots, targets = load_targets()
        for source in sorted(smoke_named_target_sources(targets)):
            print(source.relative_to(REPO_ROOT).as_posix())
        return 0
    if len(sys.argv) == 3 and sys.argv[1] == "--runnable-manifest":
        manifest = Path(sys.argv[2])
        errors = validate(manifest)
        if errors:
            print("\n".join(errors), file=sys.stderr)
            return 1
        print("\n".join(runnable_manifest_lines(manifest)))
        return 0
    if len(sys.argv) != 2:
        print(
            f"usage: {Path(sys.argv[0]).name} MANIFEST | --self-test | "
            "--smoke-target-sources | --runnable-manifest MANIFEST",
            file=sys.stderr,
        )
        return 2
    errors = validate(Path(sys.argv[1]))
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
