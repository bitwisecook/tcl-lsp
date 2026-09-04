#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Validate the Cargo target ownership declared by smoke-targets.tsv."""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
SUPPORTED_TARGET_KINDS = {"lib", "bin", "test", "example", "bench"}
LIBRARY_TARGET_KINDS = {
    "lib",
    "rlib",
    "dylib",
    "cdylib",
    "staticlib",
    "proc-macro",
}
SMOKE_TEST_RE = re.compile(r"(?:^|::)smoke")


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


def workspace_feature_command(host: str) -> list[str]:
    """Build a resolver-v2 target/test-context feature query."""
    return [
        "cargo",
        "tree",
        "--workspace",
        "--locked",
        "--target",
        host,
        "--edges",
        "normal,dev,no-proc-macro",
        "--depth",
        "0",
        "--prefix",
        "none",
        "--format",
        "{p}\t{f}",
    ]


def rustc_host(repo_root: Path = REPO_ROOT) -> str:
    """Return the host triple Cargo uses for this local smoke run."""
    result = subprocess.run(
        ["rustc", "-vV"],
        cwd=repo_root,
        check=True,
        capture_output=True,
        text=True,
    )
    for line in result.stdout.splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ")
    raise RuntimeError("rustc -vV did not report a host triple")


def workspace_target_features(
    metadata: dict[str, Any], host: str, repo_root: Path
) -> dict[str, set[str]]:
    """Resolve features in Cargo's normal/dev target context, not build context."""
    packages = metadata["packages"]
    workspace_members = set(metadata["workspace_members"])
    display_to_name = {
        f"{package['name']} v{package['version']} "
        f"({Path(package['manifest_path']).resolve().parent})": package["name"]
        for package in packages
        if package["id"] in workspace_members
    }
    features = {name: set() for name in display_to_name.values()}
    result = subprocess.run(
        workspace_feature_command(host),
        cwd=repo_root,
        check=True,
        capture_output=True,
        text=True,
    )
    for line in result.stdout.splitlines():
        if not line:
            continue
        display, separator, raw_features = line.partition("\t")
        if not separator or (package_name := display_to_name.get(display)) is None:
            continue
        # Cargo appends this outside the custom format for a de-duplicated or
        # cyclic tree entry; it is not a feature name.
        raw_features = raw_features.removesuffix(" (*)").strip()
        features[package_name].update(
            feature for feature in raw_features.split(",") if feature
        )
    return features


def canonical_target_kinds(raw_kinds: list[str]) -> set[str]:
    """Map every Cargo library crate type to the `cargo test --lib` selector."""
    kinds = SUPPORTED_TARGET_KINDS.intersection(raw_kinds)
    if LIBRARY_TARGET_KINDS.intersection(raw_kinds):
        kinds.add("lib")
    return kinds


def load_targets(repo_root: Path = REPO_ROOT) -> tuple[dict[str, Path], list[Target]]:
    host = rustc_host(repo_root)
    result = subprocess.run(
        metadata_command(host),
        cwd=repo_root,
        check=True,
        capture_output=True,
        text=True,
    )
    metadata = json.loads(result.stdout)
    workspace_members = set(metadata["workspace_members"])
    resolved_features = workspace_target_features(metadata, host, repo_root)
    manifests: dict[str, Path] = {}
    targets: list[Target] = []
    for package in metadata["packages"]:
        if package["id"] not in workspace_members:
            continue
        package_name = package["name"]
        enabled_features = resolved_features[package_name]
        manifests[package_name] = Path(package["manifest_path"]).resolve().parent
        for raw_target in package["targets"]:
            supported = canonical_target_kinds(raw_target["kind"])
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
    """Return rows whose targets are testable in the workspace feature graph."""
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
        if available.get((package, kind, target_name)):
            lines.append("\t".join((source, package, kind, target_name)))
    return lines


def runnable_manifest_lines(manifest: Path) -> list[str]:
    """Resolve and render the Cargo invocations for a validated manifest."""
    _package_roots, targets = load_targets()
    return runnable_manifest_lines_for_targets(manifest, targets)


def cargo_target_command(kind: str, target_name: str) -> list[str]:
    """Select one Cargo target shape without narrowing the workspace graph."""
    command = ["cargo", "test", "--workspace", "--locked"]
    if kind == "lib":
        command.append("--lib")
    else:
        command.extend((f"--{kind}", target_name))
    return command


def cargo_test_entries(output: str) -> list[tuple[str, str]]:
    """Parse stable libtest `--list` output into (name, kind) pairs."""
    entries: list[tuple[str, str]] = []
    for line in output.splitlines():
        name, separator, entry_kind = line.rpartition(": ")
        if separator and entry_kind in {"test", "benchmark"}:
            entries.append((name, entry_kind))
    return entries


def selected_smoke_entries(
    entries: list[tuple[str, str]], target_name: str
) -> list[tuple[str, str]]:
    """Apply the nextest smoke profile's test/binary union exactly."""
    if target_name == "smoke" or target_name.endswith("_smoke"):
        return entries
    return [(name, kind) for name, kind in entries if SMOKE_TEST_RE.search(name)]


def cargo_substring_skips(
    entries: list[tuple[str, str]], selected: list[tuple[str, str]]
) -> list[str] | None:
    """Exclude Cargo substring false positives, or report exact runs are needed."""
    selected_names = {name for name, _kind in selected}
    candidates = {name for name, _kind in entries if "smoke" in name}
    skips = sorted(candidates.difference(selected_names))
    effective = {
        name for name in candidates if not any(skip_name in name for skip_name in skips)
    }
    return skips if effective == selected_names else None


def manifest_selectors(lines: list[str]) -> list[tuple[str, str]]:
    """Deduplicate manifest rows into workspace Cargo target selectors."""
    selectors: list[tuple[str, str]] = []
    seen: set[tuple[str, str]] = set()
    for line in lines:
        _source, _package, kind, target_name = line.split("\t")
        selector = (kind, "") if kind == "lib" else (kind, target_name)
        if selector not in seen:
            seen.add(selector)
            selectors.append(selector)
    return selectors


def run_cargo_target(
    kind: str,
    target_name: str,
    *,
    list_only: bool,
    repo_root: Path = REPO_ROOT,
    env: dict[str, str] | None = None,
    quiet: bool = False,
) -> list[tuple[str, str]]:
    """List or run the exact smoke selection in one workspace feature graph."""
    command = cargo_target_command(kind, target_name)
    listing = subprocess.run(
        [*command, "--", "--list"],
        cwd=repo_root,
        env=env,
        check=True,
        capture_output=True,
        text=True,
    )
    selected = selected_smoke_entries(cargo_test_entries(listing.stdout), target_name)
    if list_only:
        for name, entry_kind in selected:
            print(f"{name}: {entry_kind}")
        return selected

    if target_name == "smoke" or target_name.endswith("_smoke"):
        subprocess.run(
            command,
            cwd=repo_root,
            env=env,
            check=True,
            capture_output=quiet,
            text=quiet,
        )
        return selected

    skips = cargo_substring_skips(cargo_test_entries(listing.stdout), selected)
    if skips is not None:
        arguments = [*command, "smoke"]
        if skips:
            arguments.append("--")
            for name in skips:
                arguments.extend(("--skip", name))
        subprocess.run(
            arguments,
            cwd=repo_root,
            env=env,
            check=True,
            capture_output=quiet,
            text=quiet,
        )
        return selected

    # A pathological non-smoke name can itself be a substring of a selected
    # name, making libtest's substring-only --skip unsafe. Preserve coverage
    # with exact positive runs in that rare case.
    for name, _entry_kind in selected:
        result = subprocess.run(
            [*command, name, "--", "--exact"],
            cwd=repo_root,
            env=env,
            capture_output=True,
            text=True,
        )
        if result.returncode:
            sys.stdout.write(result.stdout)
            sys.stderr.write(result.stderr)
            result.check_returncode()
        if not quiet:
            print(f"PASS {name}")
    return selected


def run_manifest(manifest: Path, *, list_only: bool) -> None:
    """Run each distinct manifest target with workspace feature unification."""
    errors = validate(manifest)
    if errors:
        raise RuntimeError("\n".join(errors))
    for kind, target_name in manifest_selectors(runnable_manifest_lines(manifest)):
        selector = "--lib" if kind == "lib" else f"--{kind} {target_name}"
        print(f"==> cargo test --workspace {selector}")
        run_cargo_target(kind, target_name, list_only=list_only)


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
            "source.rs\texample\tbin\tunified_smoke"
        ]

    entries = [
        ("smoke_fast", "test"),
        ("nested::smoke_nested", "test"),
        ("long_smoke_corpus", "test"),
        ("deep", "test"),
    ]
    assert selected_smoke_entries(entries, "example") == entries[:2]
    assert selected_smoke_entries(entries, "api_smoke") == entries
    assert cargo_substring_skips(entries, entries[:2]) == ["long_smoke_corpus"]
    collision_entries = [
        ("outer::smoke_long_smoke", "test"),
        ("long_smoke", "test"),
    ]
    assert cargo_substring_skips(collision_entries, collision_entries[:1]) is None
    assert (
        cargo_test_entries(
            "smoke_fast: test\nlong_smoke_corpus: test\n2 tests, 0 benchmarks\n"
        )
        == entries[:1] + entries[2:3]
    )
    assert cargo_target_command("lib", "example")[-1] == "--lib"
    assert cargo_target_command("test", "api")[-2:] == ["--test", "api"]
    assert manifest_selectors(
        [
            "a.rs\ta\tlib\ta",
            "b.rs\tb\tlib\tb",
            "one.rs\ta\ttest\tapi",
            "two.rs\tb\ttest\tapi",
        ]
    ) == [("lib", ""), ("test", "api")]

    assert metadata_command("x86_64-example-linux")[-2:] == [
        "--filter-platform",
        "x86_64-example-linux",
    ]
    feature_command = workspace_feature_command("x86_64-example-linux")
    edges = feature_command.index("--edges")
    assert feature_command[edges + 1] == "normal,dev,no-proc-macro"
    depth = feature_command.index("--depth")
    assert feature_command[depth + 1] == "0"
    assert canonical_target_kinds(["cdylib", "rlib"]) == {"lib"}
    assert canonical_target_kinds(["proc-macro"]) == {"lib"}
    assert canonical_target_kinds(["bin"]) == {"bin"}

    # Resolver v2 keeps build/proc-macro feature contexts separate from the
    # normal target and test context. Cargo metadata reports their union, so
    # prove the cargo-tree query retains a normal dependency feature but does
    # not leak a build-dependency-only feature into target eligibility.
    with tempfile.TemporaryDirectory() as temp_dir:
        fixture = Path(temp_dir)
        files = {
            "Cargo.toml": """\
[workspace]
members = ["a", "b", "c", "d", "e", "f", "g"]
resolver = "2"
""",
            "a/Cargo.toml": """\
[package]
name = "a"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["rlib"]

[features]
build_only = []
normal = []

[[bin]]
name = "build_smoke"
path = "src/build.rs"
required-features = ["build_only"]

[[bin]]
name = "normal_smoke"
path = "src/normal.rs"
required-features = ["normal"]
""",
            "a/src/lib.rs": """\
pub fn value() -> bool { true }
pub fn normal_is_enabled() -> bool { cfg!(feature = "normal") }
""",
            "a/src/build.rs": "fn main() {}\n",
            "a/src/normal.rs": "fn main() {}\n",
            "b/Cargo.toml": """\
[package]
name = "b"
version = "0.1.0"
edition = "2024"

[build-dependencies]
a = { path = "../a", features = ["build_only"] }
""",
            "b/src/lib.rs": "pub fn value() -> bool { true }\n",
            "b/build.rs": "fn main() { assert!(a::value()); }\n",
            "c/Cargo.toml": """\
[package]
name = "c"
version = "0.1.0"
edition = "2024"

[dependencies]
a = { path = "../a", features = ["normal"] }
""",
            "c/src/lib.rs": "pub fn value() -> bool { a::value() }\n",
            "d/Cargo.toml": """\
[package]
name = "d"
version = "0.1.0"
edition = "2024"

[lib]
proc-macro = true
""",
            "d/src/lib.rs": "",
            "e/Cargo.toml": """\
[package]
name = "e"
version = "0.1.0"
edition = "2024"

[dependencies]
f = { path = "../f" }
""",
            "e/src/lib.rs": """\
#[test]
fn smoke_dependency() { assert!(f::normal_is_enabled()); }

#[test]
fn long_smoke_dependency() { panic!("deep test must not run"); }

#[test]
fn deep() { panic!("deep test must not run"); }
""",
            "f/Cargo.toml": """\
[package]
name = "f"
version = "0.1.0"
edition = "2024"

[dependencies]
a = { path = "../a" }
""",
            "f/src/lib.rs": "pub fn normal_is_enabled() -> bool { a::normal_is_enabled() }\n",
            "g/Cargo.toml": """\
[package]
name = "g"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]
""",
            "g/src/lib.rs": "pub fn value() -> bool { true }\n",
        }
        for relative_path, contents in files.items():
            path = fixture / relative_path
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(contents)
        subprocess.run(
            ["cargo", "generate-lockfile", "--offline"],
            cwd=fixture,
            check=True,
            capture_output=True,
            text=True,
        )
        _fixture_roots, fixture_targets = load_targets(fixture)
        fixture_by_name = {target.name: target for target in fixture_targets}
        fixture_by_package = {
            (target.package, target.name): target for target in fixture_targets
        }
        assert fixture_by_package[("a", "a")].kind == "lib"
        assert fixture_by_package[("d", "d")].kind == "lib"
        assert fixture_by_package[("g", "g")].kind == "lib"
        assert not fixture_by_name["build_smoke"].available_in_workspace
        assert fixture_by_name["normal_smoke"].available_in_workspace
        assert fixture_by_name["normal_smoke"].resolved_features == ("normal",)

        # `-p e` loses the `normal` feature enabled for dependency `a` by
        # workspace member `c`. The fallback must keep the workspace graph,
        # while its exact test selection must exclude the substring-only
        # `long_smoke_dependency` and the unrelated failing test.
        fixture_env = os.environ.copy()
        fixture_env["CARGO_TARGET_DIR"] = str(fixture / "target")
        targeted = subprocess.run(
            [
                "cargo",
                "test",
                "--locked",
                "-p",
                "e",
                "--lib",
                "smoke_dependency",
                "--",
                "--exact",
            ],
            cwd=fixture,
            env=fixture_env,
            capture_output=True,
            text=True,
        )
        assert targeted.returncode != 0
        selected = run_cargo_target(
            "lib",
            "e",
            list_only=False,
            repo_root=fixture,
            env=fixture_env,
            quiet=True,
        )
        assert ("smoke_dependency", "test") in selected
        assert ("long_smoke_dependency", "test") not in selected


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
    if len(sys.argv) == 3 and sys.argv[1] in {"--run-manifest", "--list-manifest"}:
        try:
            run_manifest(Path(sys.argv[2]), list_only=sys.argv[1] == "--list-manifest")
        except (RuntimeError, subprocess.CalledProcessError) as error:
            print(error, file=sys.stderr)
            return 1
        return 0
    if len(sys.argv) != 2:
        print(
            f"usage: {Path(sys.argv[0]).name} MANIFEST | --self-test | "
            "--smoke-target-sources | --runnable-manifest MANIFEST | "
            "--run-manifest MANIFEST | --list-manifest MANIFEST",
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
