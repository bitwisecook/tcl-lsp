#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Offline contract test for the shared release attestation action (issue
# #1685). It exercises representative release shapes and guards the action pin,
# subject binding, SBOM inputs, and least-privilege workflow permissions.

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/../.." && pwd)
RESOLVER=$SCRIPT_DIR/resolve-release-artefact.sh
ACTION=$REPO_ROOT/.github/actions/sign-and-upload/action.yml
WORKFLOW=$REPO_ROOT/.github/workflows/ci.yml
fixture=$(mktemp -d)
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/build"

zip=$fixture/build/tcl-lsp-claude-skills-2.2.0.zip
vsix=$fixture/build/tcl-lsp-vscode-2.2.0-universal.vsix
sublime=$fixture/build/TclLsp.sublime-package
: > "$zip"
: > "$vsix"
: > "$sublime"

assert_resolves() {
    local pattern=$1 expected=$2 actual
    actual=$($RESOLVER "$pattern")
    if [[ $actual != "$expected" ]]; then
        printf 'resolver returned %q, expected %q\n' "$actual" "$expected" >&2
        exit 1
    fi
}

# Representative glob shapes from the shared action's real callers.
assert_resolves "$fixture/build/tcl-lsp-claude-skills-*.zip" "$zip"
assert_resolves "$fixture/build/tcl-lsp-vscode-*-universal.vsix" "$vsix"
assert_resolves "$fixture/build/TclLsp*.sublime-package" "$sublime"

if $RESOLVER "$fixture/build/missing-*.zip" >/dev/null 2>&1; then
    echo "resolver accepted a zero-match release glob" >&2
    exit 1
fi
: > "$fixture/build/tcl-lsp-claude-skills-2.2.1.zip"
if $RESOLVER "$fixture/build/tcl-lsp-claude-skills-*.zip" >/dev/null 2>&1; then
    echo "resolver silently selected one of multiple release artefacts" >&2
    exit 1
fi

action=$(cat "$ACTION")
if grep -q 'actions/attest-sbom@' "$ACTION"; then
    echo "deprecated actions/attest-sbom remains in the shared action" >&2
    exit 1
fi
case "$action" in
    *'uses: actions/attest@1e69f48acb82d1966a394da916b4c1698aa569d6 # v4.2.2'*) ;;
    *)
        echo "SBOM attestation must use the reviewed actions/attest v4.2.2 commit" >&2
        exit 1
        ;;
esac
case "$action" in
    *'Resolve artefact path'*'Attest build provenance'*'Generate SBOM'*'Attest SBOM'*) ;;
    *)
        echo "the exact subject must be resolved before either attestation" >&2
        exit 1
        ;;
esac
case "$action" in
    *'subject-path: ${{ steps.artefact_path.outputs.artefact }}'*'sbom-path: ${{ steps.artefact_path.outputs.sbom }}'*) ;;
    *)
        echo "actions/attest is not bound to the resolved subject and generated SBOM" >&2
        exit 1
        ;;
esac
case "$action" in
    *'path: ${{ steps.artefact_path.outputs.artefact }}'*) ;;
    *)
        echo "workflow upload is not bound to the single resolved subject" >&2
        exit 1
        ;;
esac
case "$action" in
    *'ARTEFACT_PATH: ${{ steps.artefact_path.outputs.artefact }}'*'gh release upload "$tag" "$ARTEFACT_PATH" "$SBOM_PATH" --clobber'*) ;;
    *)
        echo "release upload is not bound to the single resolved subject" >&2
        exit 1
        ;;
esac

# Find every job that consumes the composite and prove it retains precisely the
# token writes file attestations need. artifact-metadata is registry-only and
# must not be broadened onto these release jobs.
permission_rows=$(awk '
    function emit() {
        if (uses_action) {
            print job "|" contents "|" oidc "|" attestations "|" metadata
        }
    }
    /^  [A-Za-z0-9_-]+:$/ {
        emit()
        job = $1
        sub(/:$/, "", job)
        contents = oidc = attestations = metadata = uses_action = 0
        next
    }
    /contents: write/ { contents = 1 }
    /id-token: write/ { oidc = 1 }
    /attestations: write/ { attestations = 1 }
    /artifact-metadata: write/ { metadata = 1 }
    /uses: \.\/\.github\/actions\/sign-and-upload/ { uses_action = 1 }
    END { emit() }
' "$WORKFLOW")

if [[ -z $permission_rows ]]; then
    echo "no sign-and-upload consumers found; permission check is vacuous" >&2
    exit 1
fi
while IFS='|' read -r job contents oidc attestations metadata; do
    if [[ $contents != 1 || $oidc != 1 || $attestations != 1 || $metadata != 0 ]]; then
        echo "job $job has incorrect release-attestation permissions" >&2
        exit 1
    fi
done <<< "$permission_rows"

echo "release sign-and-upload action regression: ok"
