# RUST_ISSUE_206: stale self-descriptions: report-pyz.yml still says "this file lives here (not under `.github/workflows/`)"; github-pages.yml cites `scripts/verify-explorer-wasm.mjs`, which doesn't exist (actual: `scripts/verify-wasm-externref.mjs`)

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | low |
| **Subsystem** | Build tooling & CI |
| **Location** | `.github/workflows/report-pyz.yml:16-21 and github-pages.yml:98` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

.github/workflows/report-pyz.yml:16-21 and github-pages.yml:98 — stale self-descriptions: report-pyz.yml still says "this file lives here (not under `.github/workflows/`)"; github-pages.yml cites `scripts/verify-explorer-wasm.mjs`, which doesn't exist (actual: `scripts/verify-wasm-externref.mjs`). Confidence: high

## Fix

The wrong script name also appeared in the script's own usage banner
(`scripts/verify-wasm-externref.mjs`) and in the canonical deploy sources under
`rust/bigip-report-gen/python/deploy/`. The deploy sources and the script were
fixed and committed directly.

The two **installed** copies under `.github/workflows/` are byte-identical
downstream artifacts of the deploy sources (installed via the documented
`cp rust/bigip-report-gen/python/deploy/<file> .github/workflows/` step). They
could not be committed from the automation branch because updating a file under
`.github/workflows/` requires the GitHub `workflow` OAuth scope, which the
automation token lacks. Apply the patch below from a clone that has the
`workflow` scope (or simply re-run the `cp` step from the now-fixed deploy
sources) and push:

> **Follow-up.** "Byte-identical" was an invariant nothing enforced, and both
> pairs drifted again afterwards — in *both* directions, each side holding a
> fix the other lacked, so either direction of the documented `cp` would have
> silently reverted real work. They are reconciled, and
> `cargo xtask workflow-sync --check` (in `make check-all`) now gates them.
> See `rust/xtask/src/workflow_sync.rs`.

```diff
diff --git a/.github/workflows/github-pages.yml b/.github/workflows/github-pages.yml
index 228a6ca..987a1c8 100644
--- a/.github/workflows/github-pages.yml
+++ b/.github/workflows/github-pages.yml
@@ -95,7 +95,7 @@ jobs:
       # No wasm-opt/binaryen: it is intentionally disabled for this module
       # (see rust/tcl-explorer-wasm/Cargo.toml + the explorer-wasm Make target).
       # `make explorer-build` verifies the externref table is growable via
-      # scripts/verify-explorer-wasm.mjs (node is preinstalled on the runner).
+      # scripts/verify-wasm-externref.mjs (node is preinstalled on the runner).
       - name: Build the compiler-explorer GUI bundle (WASM + Mermaid)
         # Emits the wasm/js bundle + vendored Mermaid into rust/tcl-cli/gui/.
         run: make explorer-build
diff --git a/.github/workflows/report-pyz.yml b/.github/workflows/report-pyz.yml
index f7e8851..843e562 100644
--- a/.github/workflows/report-pyz.yml
+++ b/.github/workflows/report-pyz.yml
@@ -13,9 +13,10 @@
 # even when it is later run outside any git checkout — the whole reason this
 # lives in CI rather than only in `make build-report-pyz`.
 #
-# Like `github-pages.yml`, this file lives here (not under `.github/workflows/`)
-# because installing a workflow requires the `workflow` scope; copy it into
-# place from a clone that has it:
+# The canonical copy of this workflow lives at
+# `rust/bigip-report-gen/python/deploy/report-pyz.yml`; the copy under
+# `.github/workflows/` is installed from it.  Installing a workflow requires the
+# `workflow` scope, so copy it into place from a clone that has it:
 #
 #     cp rust/bigip-report-gen/python/deploy/report-pyz.yml .github/workflows/
 #     git add .github/workflows/report-pyz.yml && git commit && git push
```
