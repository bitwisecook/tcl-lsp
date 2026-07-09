# RUST_ISSUE_141: the weekly sweep deletes every Actions cache whose ref is not `refs/heads/main`, wiping the active `rust` branch's rust-cache entries that the blocking rust-gate/rust-lsp-e2e workflows depend on

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Build tooling & CI |
| **Location** | `.github/workflows/cache-cleanup.yml:62-79` |
| **Status** | Resolved — the sweep now protects `refs/heads/rust`. |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

.github/workflows/cache-cleanup.yml:62-79 — the weekly sweep deletes every Actions cache whose ref is not `refs/heads/main`, wiping the active `rust` branch's rust-cache entries that the blocking rust-gate/rust-lsp-e2e workflows depend on.
`--jq '.[] | select(.ref != "refs/heads/main") | .id'` — rust-gate.yml/rust-lsp-e2e.yml run on `push: branches: [rust]`, so their caches live on `refs/heads/rust` and get purged every Monday, forcing cold rebuilds. Confidence: high

## Fix (staged — apply with a `workflow`-scoped token)

The fix is a one-line change to the sweep's `--jq` filter so it also protects
`refs/heads/rust`. It could not be pushed from the automated session because
modifying `.github/workflows/` requires the `workflow` OAuth scope, which that
token lacks.

Apply from the repo root with `git apply --recount --ignore-whitespace` (the
`--recount` tolerates hunk-header offset drift):

```diff
diff --git a/.github/workflows/cache-cleanup.yml b/.github/workflows/cache-cleanup.yml
--- a/.github/workflows/cache-cleanup.yml
+++ b/.github/workflows/cache-cleanup.yml
@@ -62,11 +62,17 @@
-      - name: Weekly sweep of non-default-branch caches
+      - name: Weekly sweep of unprotected-branch caches
         if: github.event_name != 'pull_request'
         env:
           GH_TOKEN: ${{ github.token }}
           REPO: ${{ github.repository }}
         run: |
-          echo "Sweeping caches not on refs/heads/main"
+          # Protect the caches of long-lived branches that have push-triggered
+          # BLOCKING workflows: `main`, and `rust` (rust-gate.yml /
+          # rust-lsp-e2e.yml run on `push: branches: [rust]`, so their
+          # rust-cache entries live on refs/heads/rust). Purging those every
+          # week forced cold rebuilds on the gate (RUST_ISSUE_141). Keep this
+          # list in sync with the branches named in those workflows' triggers.
+          echo "Sweeping caches not on a protected ref (main, rust)"
           # Single pass (limit 100); a residual >100 is caught next week.
           ids=$(gh cache list --repo "$REPO" --limit 100 --json id,ref \
-            --jq '.[] | select(.ref != "refs/heads/main") | .id')
+            --jq '.[] | select(.ref != "refs/heads/main" and .ref != "refs/heads/rust") | .id')
           if [ -z "$ids" ]; then
             echo "  nothing to sweep"
```

The only functional change is the `--jq` filter gaining
`and .ref != "refs/heads/rust"`; the `name:` and comment edits are cosmetic.
After applying, set this file's **Status** to `Fixed` and tick `141` in
[`../RUST_ISSUES.md`](../RUST_ISSUES.md).
