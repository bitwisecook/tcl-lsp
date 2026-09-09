#!/bin/sh
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
CLASSIFIER=$SCRIPT_DIR/spectcl-compat-path.sh
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)

expect_relevant() {
    if ! "$CLASSIFIER" "$1"; then
        echo "expected SpecTcl-relevant path: $1" >&2
        exit 1
    fi
}

expect_unrelated() {
    if "$CLASSIFIER" "$1"; then
        echo "expected unrelated path: $1" >&2
        exit 1
    else
        status=$?
        if [ "$status" -ne 1 ]; then
            echo "classifier failed for unrelated path $1 with status $status" >&2
            exit 1
        fi
    fi
}

expect_relevant specs/eda_cadence.tclspec
expect_relevant docs/design/spec-dsl-examples/foreach.tclspec
expect_relevant docs/design/spec-dsl-examples/external/tcllib.tclspec
expect_relevant rust/tcl-compiler/src/lib.rs
expect_relevant runtime/rust/src/lib.rs
expect_relevant rust/tcl-spectcl/data/shipped-pack-dirs.txt
expect_unrelated README.md
expect_unrelated editors/vscode/src/extension.ts

bad_manifest=$(mktemp)
trap 'rm -f "$bad_manifest"' EXIT HUP INT TERM
printf '%s\n' 'specs/' > "$bad_manifest"
if TCL_LSP_SPECTCL_PACK_DIRS_MANIFEST=$bad_manifest "$CLASSIFIER" README.md; then
    echo "malformed shipped-pack manifest unexpectedly passed" >&2
    exit 1
else
    status=$?
    if [ "$status" -ne 2 ]; then
        echo "malformed shipped-pack manifest returned $status, expected 2" >&2
        exit 1
    fi
fi

echo "SpecTcl compatibility path classifier tests passed"

# The owner target must include the shipped-pack corpus lane directly. The
# general workspace test job also reaches it today, but relying on that would
# let a future test partition silently make the focused merge-blocking lane
# vacuous for production hook installation and invocation.
spectcl_target=$(awk '
    /^test-spectcl-compat:/ { in_target = 1 }
    in_target && /^[A-Za-z0-9_.-]+:/ && $1 != "test-spectcl-compat:" { exit }
    in_target { print }
' "$REPO_ROOT/Makefile")

case "$spectcl_target" in
    *'--test eval_loader'*'--test golden_packs'*'--test pack_source_e2e'*'--test spec_corpus'*'--test pack_is_real_tcl'*) ;;
    *)
        echo "test-spectcl-compat must own loader, upgrade, live-hook, shipped-corpus, and real-Tcl cases" >&2
        exit 1
        ;;
esac

echo "SpecTcl compatibility target contract tests passed"

# The focused job's caches are accelerators, never alternate correctness
# paths. It must derive Tcl identity from the manifest owner, restore before
# the unchanged Make target, save only after success, and enable sccache only
# after its optional setup succeeds.
spectcl_job=$(awk '
    /^  spectcl-compat:/ { in_job = 1 }
    in_job && /^  [A-Za-z0-9_-]+:/ && $1 != "spectcl-compat:" { exit }
    in_job { print }
' "$REPO_ROOT/.github/workflows/ci.yml")

spectcl_job_header=$(printf '%s\n' "$spectcl_job" | awk '
    /^    steps:/ { exit }
    { print }
')

case "$spectcl_job_header" in
    *'env:'*'SCCACHE_GHA_ENABLED: "true"'*) ;;
    *)
        echo "spectcl-compat must enable the GitHub Actions sccache backend at job scope" >&2
        exit 1
        ;;
esac

spectcl_step() {
    step_name=$1
    step_count=$(printf '%s\n' "$spectcl_job" | awk -v header="      - name: $step_name" '
        $0 == header { count += 1 }
        END { print count + 0 }
    ')
    if [ "$step_count" -ne 1 ]; then
        echo "spectcl-compat must contain exactly one step named: $step_name" >&2
        exit 1
    fi
    printf '%s\n' "$spectcl_job" | awk -v header="      - name: $step_name" '
        $0 == header { in_step = 1 }
        in_step && /^      - / && $0 != header { exit }
        in_step { print }
    '
}

require_in_spectcl_step() {
    required_step_name=$1
    required_step_text=$2
    required_step_block=$(spectcl_step "$required_step_name")
    case "$required_step_block" in
        *"$required_step_text"*) ;;
        *)
            echo "SpecTcl CI cache step '$required_step_name' is missing: $required_step_text" >&2
            exit 1
            ;;
    esac
}

require_in_spectcl_step 'Resolve the exact Tcl 9.0 cache identity' 'id: tcl-oracle'
require_in_spectcl_step 'Resolve the exact Tcl 9.0 cache identity' 'tcl_reference_patchlevel 9.0'
require_in_spectcl_step 'Resolve the exact Tcl 9.0 cache identity' "hashFiles('rust/tcl-dialect/data/reference-toolchains.tsv', '.claude/skills/fetch-tcl-source/fetch_tcl_source.sh', 'scripts/dev/ensure-test-deps.sh', 'scripts/dev/tcl-reference-toolchains.sh')"

require_in_spectcl_step 'Restore the exact Tcl 9.0 source and build tree' 'id: tcl-cache'
require_in_spectcl_step 'Restore the exact Tcl 9.0 source and build tree' 'actions/cache/restore@'
require_in_spectcl_step 'Restore the exact Tcl 9.0 source and build tree' 'path: ${{ steps.tcl-oracle.outputs.path }}'
require_in_spectcl_step 'Restore the exact Tcl 9.0 source and build tree' 'key: ${{ steps.tcl-oracle.outputs.key }}'

require_in_spectcl_step 'Set up sccache' 'id: sccache'
require_in_spectcl_step 'Set up sccache' 'continue-on-error: true'
require_in_spectcl_step 'Set up sccache' 'mozilla-actions/sccache-action@'

require_in_spectcl_step 'Enable sccache when available' 'SCCACHE_SETUP_OUTCOME: ${{ steps.sccache.outcome }}'
require_in_spectcl_step 'Enable sccache when available' 'RUSTC_WRAPPER=sccache'
require_in_spectcl_step 'Enable sccache when available' 'continuing with uncached compilation'

require_in_spectcl_step 'Run exact SpecTcl 1.x + 2.0 + real-Tcl compatibility gate' 'run: make test-spectcl-compat'

require_in_spectcl_step 'Save the validated Tcl 9.0 source and build tree' "success() && steps.tcl-cache.outputs.cache-hit != 'true'"
require_in_spectcl_step 'Save the validated Tcl 9.0 source and build tree' 'actions/cache/save@'
require_in_spectcl_step 'Save the validated Tcl 9.0 source and build tree' 'path: ${{ steps.tcl-oracle.outputs.path }}'
require_in_spectcl_step 'Save the validated Tcl 9.0 source and build tree' 'key: ${{ steps.tcl-oracle.outputs.key }}'

require_in_spectcl_step 'Report sccache statistics' 'if: always()'
require_in_spectcl_step 'Report sccache statistics' 'continue-on-error: true'
require_in_spectcl_step 'Report sccache statistics' 'sccache --show-stats'

previous_step_line=0
while IFS= read -r ordered_step_name; do
    ordered_step_line=$(printf '%s\n' "$spectcl_job" | awk -v header="      - name: $ordered_step_name" '
        $0 == header { print NR }
    ')
    if [ "$ordered_step_line" -le "$previous_step_line" ]; then
        echo "SpecTcl CI cache step is out of order: $ordered_step_name" >&2
        exit 1
    fi
    previous_step_line=$ordered_step_line
done <<'EOF'
Resolve the exact Tcl 9.0 cache identity
Restore the exact Tcl 9.0 source and build tree
Set up sccache
Enable sccache when available
Run exact SpecTcl 1.x + 2.0 + real-Tcl compatibility gate
Save the validated Tcl 9.0 source and build tree
Report sccache statistics
EOF

echo "SpecTcl compatibility cache contract tests passed"

# The repository's existing required check is `pr-gate`, so a job that is not
# itself required must feed a real failure into that check. A plain `needs`
# edge is insufficient: Actions skips dependent jobs after a failed need.
# Two jobs ride this lane — `spectcl-compat` and `web-frontends` — and both
# are only merge-blocking for as long as the wiring below survives. Keep this
# small textual contract beside the classifier contract so changing any part
# of the merge-blocking lane is caught by `make rust-check`.
pr_gate_block=$(awk '
    /^  pr-gate:/ { in_pr_gate = 1 }
    in_pr_gate && /^  [A-Za-z0-9_-]+:/ && $1 != "pr-gate:" { exit }
    in_pr_gate { print }
' "$REPO_ROOT/.github/workflows/ci.yml")

case "$pr_gate_block" in
    *'if: ${{ always() }}'*) ;;
    *)
        echo "pr-gate must run under always() so failed prerequisites are propagated" >&2
        exit 1
        ;;
esac
case "$pr_gate_block" in
    *'needs: [channel, spectcl-compat, web-frontends]'*) ;;
    *)
        echo "pr-gate must depend on channel, spectcl-compat and web-frontends" >&2
        exit 1
        ;;
esac

# Checked piecewise rather than as one contiguous string: the condition spans
# a line continuation, and pinning the exact joining would break on a purely
# cosmetic rewrap while saying nothing about whether the gate still fails.
for required in \
    'CHANNEL_RESULT: ${{ needs.channel.result }}' \
    'SPECTCL_COMPAT_RESULT: ${{ needs.spectcl-compat.result }}' \
    'WEB_FRONTENDS_RESULT: ${{ needs.web-frontends.result }}' \
    '[ "$CHANNEL_RESULT" != success ]' \
    '[ "$SPECTCL_COMPAT_RESULT" != success ]' \
    '[ "$WEB_FRONTENDS_RESULT" != success ]' \
    'exit 1'
do
    case "$pr_gate_block" in
        *"$required"*) ;;
        *)
            echo "pr-gate must explicitly fail when a prerequisite gate fails (missing: $required)" >&2
            exit 1
            ;;
    esac
done

echo "Merge-blocking gate contract tests passed"
