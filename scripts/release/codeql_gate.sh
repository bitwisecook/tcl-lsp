#!/usr/bin/env bash
# release_codeql_gate.sh — block a release until CodeQL has finished
# cleanly on a given commit and no open high/critical alerts remain on
# main.
#
# Usage:  scripts/release/codeql_gate.sh <commit-sha>
#
# The release skill calls this once, against HEAD of main after the
# RELEASE_NOTES.md PR has merged — that is the commit the tag will
# point at.  The CodeQL workflow runs automatically on push to main,
# so the script normally just locates the existing run and waits for
# it.  If no run is found within ~2 min the script falls back to
# `workflow_dispatch` so a missed run (cancelled by concurrency, infra
# hiccup) doesn't silently skip the gate.
#
# Releases are tag-only off main; the gate (and the dispatch fallback)
# is hardcoded to `refs/heads/main` to match.
#
# Exit codes:
#   0  CodeQL run succeeded and no open high/critical alerts on main
#   1  CodeQL run failed, was cancelled, or open high/critical alerts exist
#   2  gh or jq missing/unauthenticated, or could not locate/dispatch
#      a run within the timeout
set -euo pipefail

SHA="${1:?usage: $0 <commit-sha>}"
REF="refs/heads/main"
REPO="${GH_REPO:-bitwisecook/tcl-lsp}"
WORKFLOW="codeql.yml"

# Severity threshold honoured by the alert check.  "high" blocks on
# both high and critical; "critical" blocks only on critical.  Set
# CODEQL_GATE_MIN_SEVERITY=critical to relax temporarily.
MIN_SEVERITY="${CODEQL_GATE_MIN_SEVERITY:-high}"

case "$MIN_SEVERITY" in
    critical)         SEVERITY_FILTER='.rule.security_severity_level == "critical"' ;;
    high|*)           SEVERITY_FILTER='.rule.security_severity_level == "high" or .rule.security_severity_level == "critical"' ;;
esac

require_deps() {
    command -v gh >/dev/null 2>&1 || { echo "error: gh CLI required" >&2; exit 2; }
    command -v jq >/dev/null 2>&1 || { echo "error: jq required" >&2; exit 2; }
    gh auth status >/dev/null 2>&1 || { echo "error: gh CLI not authenticated" >&2; exit 2; }
}

find_run() {
    gh run list \
        --repo "$REPO" \
        --workflow "$WORKFLOW" \
        --commit "$SHA" \
        --limit 1 \
        --json databaseId \
        --jq '.[0].databaseId // empty'
}

wait_for_run_to_appear() {
    local run_id=""
    local i
    for i in $(seq 1 12); do
        run_id="$(find_run)"
        [ -n "$run_id" ] && { printf '%s' "$run_id"; return 0; }
        sleep 10
    done
    return 1
}

main() {
    require_deps

    echo "==> CodeQL gate for $SHA (ref $REF, repo $REPO)"

    local run_id
    if ! run_id="$(wait_for_run_to_appear)"; then
        echo "    no push-triggered run found after 2 min; dispatching workflow_dispatch"
        gh workflow run "$WORKFLOW" --repo "$REPO" --ref main
        if ! run_id="$(wait_for_run_to_appear)"; then
            echo "ERROR: could not locate a CodeQL run for $SHA after dispatch" >&2
            exit 2
        fi
    fi

    echo "    watching run $run_id"
    if ! gh run watch --repo "$REPO" "$run_id" --exit-status; then
        echo "ERROR: CodeQL run $run_id did not succeed; release blocked" >&2
        exit 1
    fi

    echo "==> Checking open CodeQL alerts on $REF (threshold: $MIN_SEVERITY)"
    # `gh api --paginate` emits one JSON document per page, not a single
    # combined array — `jq -s` slurps every page into one array, and
    # `.[][]` flattens to the full alert stream.  Filtering inside one
    # jq invocation guarantees the count below is a single integer; the
    # naive `gh --paginate | jq length` form produces one count per
    # page, which would crash `[ "$count" -gt 0 ]` and silently fall
    # through to the success path (CVE-grade failure mode for a gate).
    local matching
    matching="$(gh api --paginate \
        "/repos/$REPO/code-scanning/alerts?state=open&tool_name=CodeQL&per_page=100" \
        | jq -s "[.[][] | select(.most_recent_instance.ref == \"$REF\") | select($SEVERITY_FILTER)]")"

    local count
    count="$(printf '%s' "$matching" | jq 'length')"

    if [ "$count" -gt 0 ]; then
        echo "ERROR: $count open CodeQL alert(s) at $MIN_SEVERITY+ on $REF; release blocked" >&2
        printf '%s' "$matching" | jq -r '.[] |
            "  #\(.number)  \(.rule.id)  [\(.rule.security_severity_level)]  \(.html_url)"' >&2
        echo >&2
        echo "Resolve the alerts (fix or dismiss with justification) and re-run the release." >&2
        exit 1
    fi

    echo "    no open alerts at $MIN_SEVERITY+ on $REF"
    echo "==> CodeQL gate passed for $SHA"
}

main "$@"
