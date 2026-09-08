#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Contract test for the tag release graph. The native matrix is a producer of
# short-lived workflow artefacts, so it may overlap the validation and release
# creation gates. Only the read-only producer may take that shortcut: every
# job that consumes its bytes for a GitHub Release must retain the release,
# test, and Linux portability dependencies.

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
WORKFLOW=${WORKFLOW:-$REPO_ROOT/.github/workflows/ci.yml}

job_block() {
    awk -v wanted="$1" '
        $0 == "  " wanted ":" { found = 1; print; next }
        found && /^  [A-Za-z0-9_-]+:/ { exit }
        found { print }
    ' "$WORKFLOW"
}

needs_block() {
    printf '%s\n' "$1" | awk '
        /^    needs:/ { reading = 1; print; next }
        reading && /^    [A-Za-z0-9_-]+:/ { exit }
        reading { print }
    '
}

job_needs() {
    # Job identifiers are the only words in a `needs` list. Keep this parser
    # deliberately small: ci.yml authors the list in one line today, while
    # the block extraction above also leaves room for a wrapped list later.
    needs_block "$(require_job "$1")" |
        sed -E 's/^[[:space:]]*needs:[[:space:]]*//' |
        tr '[],' '   ' |
        awk '{ for (i = 1; i <= NF; i++) if ($i ~ /^[A-Za-z0-9_-]+$/) print $i }'
}

depends_transitively_on() {
    current=$1
    target=$2
    visited=${3:-}

    case " $visited " in
        *" $current "*) return 1 ;;
    esac
    visited="$visited $current"

    for dependency in $(job_needs "$current"); do
        if [ "$dependency" = "$target" ] ||
            (depends_transitively_on "$dependency" "$target" "$visited"); then
            return 0
        fi
    done
    return 1
}

require_job() {
    block=$(job_block "$1")
    if [ -z "$block" ]; then
        echo "ci.yml must define the $1 job" >&2
        exit 1
    fi
    printf '%s\n' "$block"
}

require_needs() {
    job=$1
    shift
    block=$(require_job "$job")
    needs=$(needs_block "$block")
    if [ -z "$needs" ]; then
        echo "$job must define a needs list" >&2
        exit 1
    fi
    for dependency in "$@"; do
        if ! printf '%s\n' "$needs" | grep -Eq "(^|[^[:alnum:]_-])${dependency}([^[:alnum:]_-]|$)"; then
            echo "$job must depend on $dependency" >&2
            printf '%s\n' "$needs" >&2
            exit 1
        fi
    done
}

matrix=$(require_job build-server-matrix)
matrix_without_comments=$(printf '%s\n' "$matrix" | sed -E 's/[[:space:]]+#.*$//')

case "$matrix_without_comments" in
    *"    if: startsWith(github.ref, 'refs/tags/v') || (github.event_name == 'workflow_dispatch' && inputs.native_release_build_proof)"*) ;;
    *)
        echo "build-server-matrix must remain tag-gated except for explicit proof dispatches" >&2
        exit 1
        ;;
esac
case "$matrix_without_comments" in
    *"    needs: [channel]"*) ;;
    *)
        echo "build-server-matrix must start after channel, not create-release" >&2
        exit 1
        ;;
esac

# A workflow-artifact producer has no authority to mutate a Release, mint OIDC
# credentials, read marketplace secrets, or pause on a deployment environment.
permissions=$(printf '%s\n' "$matrix_without_comments" | awk '
    /^    permissions:/ { reading = 1; print; next }
    reading && /^    [A-Za-z0-9_-]+:/ { exit }
    reading { print }
')
if [ -z "$permissions" ] || ! printf '%s\n' "$permissions" | grep -Fqx '      contents: read'; then
    echo "build-server-matrix must declare read-only contents permission" >&2
    exit 1
fi
if printf '%s\n' "$permissions" | grep -Eq 'write|id-token'; then
    echo "build-server-matrix permissions must not grant write or id-token access" >&2
    printf '%s\n' "$permissions" >&2
    exit 1
fi
if printf '%s\n' "$matrix_without_comments" | grep -Eq '^[[:space:]]+environment:|secrets\.'; then
    echo "build-server-matrix must not use an environment or secrets" >&2
    exit 1
fi
if ! printf '%s\n' "$matrix_without_comments" | grep -Eq '^[[:space:]]+retention-days: 1$'; then
    echo "build-server-matrix artifacts must keep the one-day retention bound" >&2
    exit 1
fi

# The setup action caches target directories by default. Those contain host
# build scripts, which cannot cross safely into the UBI 8 build environment or
# between platform legs. Registry caching remains useful and safe.
rust_setup=$(printf '%s\n' "$matrix_without_comments" | awk '
    $0 == "      - name: Set up Rust" { reading = 1 }
    reading && $0 != "      - name: Set up Rust" && /^      - / { exit }
    reading { print }
')
if [ "$(printf '%s\n' "$rust_setup" | grep -cF '          cache-targets: false')" -ne 1 ]; then
    echo "build-server-matrix must disable compiled Cargo target caching" >&2
    exit 1
fi

platform_count=$(printf '%s\n' "$matrix_without_comments" | grep -c '^          - os:')
if [ "$platform_count" -ne 7 ]; then
    echo "build-server-matrix must retain exactly seven platform rows" >&2
    exit 1
fi
for mapping in \
    'macos-latest|darwin-arm64|aarch64-apple-darwin|none|' \
    'macos-latest|darwin-x64|x86_64-apple-darwin|none|' \
    'ubuntu-24.04|linux-x64|x86_64-unknown-linux-gnu|ubi8|2.28' \
    'ubuntu-24.04-arm|linux-arm64|aarch64-unknown-linux-gnu|ubi8|2.28' \
    'ubuntu-24.04|linux-riscv64|riscv64gc-unknown-linux-gnu|riscv_glibc|2.35' \
    'windows-latest|win32-x64|x86_64-pc-windows-msvc|none|' \
    'windows-latest|win32-arm64|aarch64-pc-windows-msvc|none|'; do
    os=${mapping%%|*}
    rest=${mapping#*|}
    id=${rest%%|*}
    rest=${rest#*|}
    target=${rest%%|*}
    rest=${rest#*|}
    mode=${rest%%|*}
    ceiling=${rest#*|}
    platform_block=$(printf '%s\n' "$matrix_without_comments" | awk -v id="$id" '
        $0 == "            id: " id { found = 1; print previous; print; next }
        found && /^          - os:/ { exit }
        found { print }
        { previous = $0 }
    ')
    if [ -z "$platform_block" ] ||
        ! printf '%s\n' "$platform_block" | grep -Fqx "          - os: $os" ||
        ! printf '%s\n' "$platform_block" | grep -Fqx "            id: $id" ||
        ! printf '%s\n' "$platform_block" | grep -Fqx "            target: $target"; then
        echo "build-server-matrix is missing platform mapping $mapping" >&2
        exit 1
    fi
    if [ "$mode" = none ]; then
        if printf '%s\n' "$platform_block" | grep -Eq '^[[:space:]]+(ubi8|riscv_glibc|glibc_max):'; then
            echo "non-Linux platform $id must not declare a glibc build mode" >&2
            exit 1
        fi
    elif ! printf '%s\n' "$platform_block" | grep -Fqx "            $mode: true" ||
        ! printf '%s\n' "$platform_block" | grep -Fqx "            glibc_max: \"$ceiling\""; then
        echo "Linux platform $id has the wrong build mode or glibc ceiling" >&2
        exit 1
    fi
    if ! grep -Fq "${target}:${id}" "$REPO_ROOT/Makefile"; then
        echo "build-server-matrix platform $target:$id is absent from SERVER_TARGET_MAP" >&2
        exit 1
    fi
done

program_count=$(printf '%s\n' "$matrix_without_comments" | grep -c '^          - package:')
if [ "$program_count" -ne 4 ]; then
    echo "build-server-matrix must define exactly four independent program rows" >&2
    exit 1
fi
for mapping in \
    'tcl-lsp-server|tcl-lsp-server|server' \
    'tcl-mcp|tcl-mcp|mcp' \
    'tcl-cli|tcl|tcl' \
    'f5-cli|f5-query|f5'; do
    package=${mapping%%|*}
    rest=${mapping#*|}
    binary=${rest%%|*}
    id=${rest##*|}
    if ! printf '%s\n' "$matrix_without_comments" | awk \
        -v package="$package" -v binary="$binary" -v id="$id" '
            $0 == "          - package: " package { package_seen = 1; next }
            package_seen && $0 == "            binary: " binary { binary_seen = 1; next }
            package_seen && binary_seen && $0 == "            id: " id { found = 1 }
            package_seen && /^          - package:/ { exit }
            END { exit !found }
        '; then
        echo "build-server-matrix is missing program mapping $mapping" >&2
        exit 1
    fi
done

# A multi-root Cargo invocation unions dependency features and changes the
# shipping graph. Each matrix leg must select only its one program root.
if printf '%s\n' "$matrix_without_comments" |
    grep -Eq 'cargo build -p (tcl-lsp-server|tcl-mcp|tcl-cli|f5-cli)'; then
    echo "build-server-matrix must select the matrix package, not hard-code or combine roots" >&2
    exit 1
fi
package_builds=$(printf '%s\n' "$matrix_without_comments" |
    grep -cF 'cargo build -p "$PACKAGE" --release --target "$TARGET"')
matrix_builds=$(printf '%s\n' "$matrix_without_comments" |
    grep -cF 'cargo build -p "${{ matrix.program.package }}" --release --target "$t"')
if [ "$package_builds" -ne 2 ] || [ "$matrix_builds" -ne 1 ]; then
    echo "each native build path must invoke Cargo once for its matrix program" >&2
    exit 1
fi
if ! printf '%s\n' "$matrix_without_comments" |
    grep -Fq 'name: server-bins-${{ matrix.platform.id }}-${{ matrix.program.id }}'; then
    echo "native artifacts must be unique per platform and program" >&2
    exit 1
fi
concurrency_group=$(grep '^  group:' "$WORKFLOW")
if ! printf '%s\n' "$concurrency_group" | grep -Fqx \
    "  group: \${{ github.workflow }}-\${{ github.event_name == 'workflow_dispatch' && !startsWith(github.ref, 'refs/tags/v') && (inputs.native_release_sccache_proof && 'native-release-sccache-proof' || inputs.native_release_build_proof && 'native-release-proof') || 'ordinary' }}-\${{ github.ref }}"; then
    echo "proof and ordinary runs need fixed, unambiguous concurrency discriminators before the ref" >&2
    exit 1
fi
if ! printf '%s\n' "$matrix_without_comments" |
    grep -Fq "if: matrix.program.id == 'server' && matrix.platform.riscv_glibc != true"; then
    echo "only server legs may run the cross-server smoke test" >&2
    exit 1
fi

# `create-release` remains the root of every release-producing path and keeps
# the complete validation surface. The matrix itself must not become a hidden
# prerequisite of that gate, or the overlap would disappear.
require_needs create-release channel pr-gate rust-tests rust-tests-heavy spectcl-compat \
    lsp-server-wasm lsp-e2e cargo-deny python web-frontends
if printf '%s\n' "$(needs_block "$matrix")" | grep -Eq 'create-release|pr-gate|rust-tests|spectcl-compat|lsp-server-wasm|lsp-e2e|cargo-deny|python|web-frontends'; then
    echo "build-server-matrix must not wait for release validation jobs" >&2
    exit 1
fi
# The direct check above is not enough: an innocent-looking intermediary can
# make either job an ancestor of the other and erase the intended overlap.
# Walk the complete `needs` graph so any such path fails this contract.
if depends_transitively_on build-server-matrix create-release; then
    echo "build-server-matrix must not transitively wait for create-release" >&2
    exit 1
fi
if depends_transitively_on create-release build-server-matrix; then
    echo "create-release must not transitively wait for build-server-matrix" >&2
    exit 1
fi

# These jobs consume the matrix bytes or their derived digests. Keep the
# release object, extension tests, and portability fan-in in their direct
# needs lists so a future refactor cannot accidentally publish an unvalidated
# native artefact through a transitive or skipped dependency.
require_needs linux-release-portability create-release build-server-matrix
require_needs publish-native-binaries create-release build-server-matrix linux-release-portability
require_needs build-vsix create-release test-ext test-ext-web build-server-matrix \
    linux-release-portability lsp-server-wasi
require_needs build-jetbrains create-release build-server-matrix linux-release-portability
require_needs build-sublime create-release build-server-matrix linux-release-portability

# Checksum publication waits for both release-producing branches. Marketplace
# jobs then consume only the verified packages and signed checksum manifest.
require_needs build-claude-skills create-release
require_needs build-zed create-release
require_needs publish-checksums create-release build-vsix build-claude-skills \
    build-jetbrains build-sublime build-zed publish-native-binaries
require_needs publish-vsix-marketplace build-vsix publish-checksums
require_needs publish-vsix-openvsx build-vsix publish-checksums
require_needs publish-jetbrains-marketplace build-jetbrains publish-checksums
require_needs cleanup-old-artifacts build-vsix build-claude-skills build-jetbrains \
    build-sublime build-zed publish-checksums
require_needs cleanup-untagged-artifacts build-vsix build-claude-skills \
    build-jetbrains build-sublime build-zed publish-checksums

# The proof dispatch may build read-only workflow artifacts from a branch, but
# no release or publishing consumer may become reachable on that ref.
for job in create-release linux-release-portability publish-native-binaries \
    build-vsix build-jetbrains build-sublime build-claude-skills build-zed \
    publish-checksums publish-vsix-marketplace publish-vsix-openvsx \
    publish-jetbrains-marketplace cleanup-old-artifacts \
    cleanup-untagged-artifacts; do
    if ! require_job "$job" | grep -Fq "startsWith(github.ref, 'refs/tags/v')"; then
        echo "$job must remain unreachable from a branch proof dispatch" >&2
        exit 1
    fi
done

verify_versions=$REPO_ROOT/scripts/verify-native-versions.sh
if output=$(TCL_LSP_RELEASE_BINARIES= "$verify_versions" 0 /nonexistent 2>&1); then
    echo "verify-native-versions must reject an empty matrix selection" >&2
    exit 1
fi
if ! printf '%s\n' "$output" | grep -Fq 'selected no binaries'; then
    echo "verify-native-versions did not diagnose an empty matrix selection" >&2
    exit 1
fi
if output=$(TCL_LSP_RELEASE_BINARIES=not-a-binary "$verify_versions" 0 /nonexistent 2>&1); then
    echo "verify-native-versions must reject an unknown matrix binary" >&2
    exit 1
fi
if ! printf '%s\n' "$output" | grep -Fq 'unsupported native release binary'; then
    echo "verify-native-versions did not diagnose an unknown matrix binary" >&2
    exit 1
fi

# The Darwin ARM64 sccache experiment is deliberately opt-in and cannot enter
# the release graph. Keep its proof surface explicit so a future edit cannot
# accidentally turn it into a producer or weaken the byte/runtime checks.
if ! grep -Fq '      native_release_sccache_proof:' "$WORKFLOW"; then
    echo "workflow_dispatch must expose the native sccache proof input" >&2
    exit 1
fi
proof=$(require_job native-release-sccache-proof)
if ! printf '%s\n' "$proof" | grep -Fqx "    if: github.event_name == 'workflow_dispatch' && inputs.native_release_sccache_proof == true && github.ref_type == 'branch'"; then
    echo "native sccache proof must be restricted to explicit branch dispatches" >&2
    exit 1
fi
if ! printf '%s\n' "$proof" | grep -Fqx '    runs-on: macos-latest'; then
    echo "native sccache proof must use the production Darwin runner image" >&2
    exit 1
fi
darwin_arm64=$(printf '%s\n' "$matrix_without_comments" | awk '
    /^          - os: macos-latest$/ { platform = $0; next }
    platform && /^            id: darwin-arm64$/ { found = 1; print platform; print; next }
    found && /^          - os:/ { exit }
    found { print }
')
if ! printf '%s\n' "$darwin_arm64" | grep -Fqx '          - os: macos-latest' ||
    ! printf '%s\n' "$darwin_arm64" | grep -Fqx '            id: darwin-arm64' ||
    ! printf '%s\n' "$darwin_arm64" | grep -Fqx '            target: aarch64-apple-darwin'; then
    echo "production native matrix must retain the Darwin ARM64 proof mapping" >&2
    exit 1
fi
if printf '%s\n' "$proof" | grep -Eq '^    needs:|actions/(upload|download)-artifact|gh (release|api)|sign-and-upload|attest|GH_TOKEN|id-token:|secrets\.|environment:'; then
    echo "native sccache proof must have no dependencies, uploads, signing, or release authority" >&2
    exit 1
fi
if ! printf '%s\n' "$proof" | grep -Fq 'TCL_LSP_VERSION: v0.0.0-proof'; then
    echo "native sccache proof must pin its non-release version" >&2
    exit 1
fi
if ! printf '%s\n' "$proof" | grep -Fq 'SCCACHE_GHA_CACHE_PREFIX: native-release-sccache-darwin-arm64-server-${{ github.run_id }}-${{ github.run_attempt }}-'; then
    echo "native sccache proof must use a unique run/attempt cache namespace" >&2
    exit 1
fi
if [ "$(printf '%s\n' "$proof" | grep -cF '          cache-targets: false')" -ne 1 ]; then
    echo "native sccache proof must disable compiled Cargo target caching" >&2
    exit 1
fi
for required in \
    'test "${RUNNER_OS}" = macOS' \
    'test "${RUNNER_ARCH}" = ARM64' \
    '--target aarch64-apple-darwin' \
    'env -u RUSTC_WRAPPER' \
    'RUSTC_WRAPPER=sccache' \
    'sccache --zero-stats' \
    'sccache --show-stats' \
    'cold_misses=' \
    'test "${cold_misses:-0}" -gt 0' \
    'warm_hits=' \
    'test "${warm_hits:-0}" -gt 0' \
    'baseline_target="$GITHUB_WORKSPACE/proof-target-baseline"' \
    'cold_target="$GITHUB_WORKSPACE/proof-target-sccache-cold"' \
    'warm_target="$GITHUB_WORKSPACE/proof-target-sccache-warm"' \
    'baseline-cargo-time.txt' \
    'sccache-cold-cargo-time.txt' \
    'sccache-warm-cargo-time.txt' \
    'test "$baseline_sha" = "$cold_sha"' \
    'test "$baseline_sha" = "$warm_sha"' \
    'cmp "$baseline" "$cold"' \
    'cmp "$baseline" "$warm"' \
    'verify-native-versions.sh' \
    'file "$baseline" "$cold" "$warm"' \
    'otool -hv "$binary"' \
    'make server-cross-test'; do
    if ! printf '%s\n' "$proof" | grep -Fq -- "$required"; then
        echo "native sccache proof is missing required check: $required" >&2
        exit 1
    fi
done

# The experiment must remain an isolated leaf. No production or publishing
# job may consume it, directly or through a future needs-list refactor.
for job in $(awk '/^  [A-Za-z0-9_-]+:$/ { sub(/^  /, ""); sub(/:$/, ""); print }' "$WORKFLOW"); do
    if [ "$job" = native-release-sccache-proof ]; then
        continue
    fi
    if job_needs "$job" | grep -Fqx native-release-sccache-proof; then
        echo "$job must not depend on the experimental native sccache proof" >&2
        exit 1
    fi
done

echo "release dependency graph contract passed"
