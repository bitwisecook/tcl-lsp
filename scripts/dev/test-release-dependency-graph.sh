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
    *"    if: startsWith(github.ref, 'refs/tags/v')"*) ;;
    *)
        echo "build-server-matrix must remain tag-gated" >&2
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

echo "release dependency graph contract passed"
