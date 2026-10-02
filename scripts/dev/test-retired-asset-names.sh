#!/usr/bin/env bash
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Zero-reference gate for retired RELEASE ASSET names.
#
# A release asset is resolved by exact name: Package Control fetches the asset
# the channel entry names, and the shared attestation action resolves one glob
# to exactly one file. So a renamed asset leaves every other mention of the old
# name silently wrong — a test fixture that passes against itself, an operator
# runbook that tells a human to look for a file no release carries.
#
# Nothing else reads these names programmatically, so this gate is the only
# thing standing between a rename and that drift. It is textual on purpose: it
# fails on the retired spelling anywhere in the tracked tree, including prose,
# so the name cannot come back in a fixture, a skill, or a doc.
#
# Adding a retirement is one row in RETIRED_ASSET_NAMES.
#
# Waiver: put `retired-asset-ok:` on the line to allow a deliberate mention,
# such as a changelog entry that has to name what the asset used to be called.

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/../.." && pwd)
# This file names the retired spellings in its own table and fixtures, so it
# must exclude itself. Derived rather than written out, so moving the script
# cannot leave it flagging itself.
SELF_REL=${SCRIPT_DIR#"$REPO_ROOT"/}/$(basename "${BASH_SOURCE[0]}")

WAIVER='retired-asset-ok:'

# Each row: <extended regex>|<what to use instead>|<why the name is resolved exactly>
#
# The regex is anchored to the asset suffix because the retired stem is also a
# live identifier prefix: the JetBrains plugin has TclLspSettings, TclLspActions
# and friends, and none of those are release assets.
RETIRED_ASSET_NAMES=(
    'TclLsp[A-Za-z0-9._*-]*\.sublime-package|LSP-Tcl.sublime-package (stable) or LSP-Tcl-prerelease.sublime-package (odd-minor pre-release)|Package Control resolves the channel entry asset by exact name'
)

# Report every unwaived hit for one pattern over the NUL-separated files on
# stdin. Prints `path:line:text` per hit; the caller supplies the context.
scan_pattern() {
    local regex=$1
    xargs -0 grep -HInE -- "$regex" 2>/dev/null || true
}

# Files this gate reads: everything git tracks, minus itself. Using the index
# rather than a directory walk keeps build output, vendored trees and the
# fetched Tcl sources out without a hand-kept exclude list.
tracked_files() {
    git -C "$REPO_ROOT" ls-files -z | grep -zZv "^${SELF_REL}$"
}

run_gate() {
    local -i failures=0
    local entry regex replacement why hit text

    for entry in "${RETIRED_ASSET_NAMES[@]}"; do
        IFS='|' read -r regex replacement why <<<"$entry"
        while IFS= read -r hit; do
            [[ -z $hit ]] && continue
            text=${hit#*:}
            text=${text#*:}
            [[ $text == *"$WAIVER"* ]] && continue
            if (( failures == 0 )); then
                echo "retired release asset name found:" >&2
            fi
            printf '  %s\n' "$hit" >&2
            printf '    use instead: %s\n' "$replacement" >&2
            printf '    why: %s\n' "$why" >&2
            failures+=1
        done < <(tracked_files | (cd "$REPO_ROOT" && scan_pattern "$regex"))
    done

    if (( failures > 0 )); then
        echo >&2
        echo "A release asset is resolved by exact name, so every mention must" >&2
        echo "match what CI publishes. Fix the name, or mark a deliberate" >&2
        echo "historical mention with '$WAIVER' on the same line." >&2
        return 1
    fi
    return 0
}

# Prove the gate fires and that it leaves the live identifiers alone. Without
# this the gate could silently match nothing and still report success.
self_test() {
    local fixture
    fixture=$(mktemp -d)
    # shellcheck disable=SC2064
    trap "rm -rf '$fixture'" RETURN

    printf 'artefact-glob: "build/TclLsp*.sublime-package"\n' >"$fixture/stale.yml"
    printf 'asset: TclLsp.sublime-package\n' >"$fixture/stale.md"
    printf 'class TclLspSettings { }\nval x = "LSP-Tcl.sublime-package"\n' >"$fixture/live.kt"
    printf 'the old TclLsp.sublime-package name  # %s renamed in v2.2.2\n' "$WAIVER" >"$fixture/waived.md"

    local regex=${RETIRED_ASSET_NAMES[0]%%|*}
    local hits
    hits=$(printf '%s\0' "$fixture/stale.yml" "$fixture/stale.md" | scan_pattern "$regex" | wc -l)
    [[ $hits -eq 2 ]] || {
        echo "self-test: expected 2 hits on retired names, got $hits" >&2
        return 1
    }
    echo "ok: flags the retired glob and the retired bare name"

    hits=$(printf '%s\0' "$fixture/live.kt" | scan_pattern "$regex" | wc -l)
    [[ $hits -eq 0 ]] || {
        echo "self-test: TclLsp* identifiers and the live asset must not match" >&2
        return 1
    }
    echo "ok: leaves TclLsp identifiers and the current asset name alone"

    local line
    line=$(printf '%s\0' "$fixture/waived.md" | scan_pattern "$regex")
    [[ $line == *"$WAIVER"* ]] || {
        echo "self-test: waiver fixture did not produce a waivable hit" >&2
        return 1
    }
    echo "ok: a waived mention is still matched, so the waiver is what excuses it"
}

if [[ ${1:-} == --self-test ]]; then
    self_test
    exit 0
fi

self_test >/dev/null
run_gate
echo "retired release asset name gate: ok"
