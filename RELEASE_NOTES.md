# v1.10.1

## New Features

- **f5 query DSL — jq 1.7 standard-library parity.** The jq-flavoured
  `f5 query` verb now implements the full jq 1.7 stdlib surface, so
  idioms that work in `jq` against arbitrary JSON now transfer
  verbatim to BIG-IP configuration queries.
- **f5 query — import-friendly Python API and renderer plugins.** The
  query engine is exposed as a callable Python API with pluggable
  renderers, so callers can embed query execution and produce custom
  output formats without shelling out to the CLI.
- **Per-workspace-folder config.** Multi-folder workspaces now resolve
  dialect, `extraCommands`, `libraryPaths`, and `style.nonAscii` per
  folder, with longest-prefix matching and a fixed race in
  `_apply_settings_to_target` that previously caused settings from one
  folder to leak into another (issue #407).
- **BIG-IP sysadmin queries cookbook.** New cookbook documenting
  real-world `f5 query` recipes against captured outputs from
  production-shaped configs.

## Improvements

- **Tcl 9 tcltest WASM baseline refresh.** The Tcl 9 tcltest sweep
  baseline (`tests/baselines/tcl9-tcltest-wasm/`) is regenerated
  against the current runtime, capturing additional categories
  (`subst`, several `*-old` legacy suites) and refreshed per-category
  pass counts.
- **Documentation.** The installation guide now covers VS Code forks
  and other LSP-capable editors.
- **JetBrains plugin compatibility (2024.1+).** The plugin descriptor
  now declares compatibility with IntelliJ Platform 2024.1 and newer,
  unblocking installs on current JetBrains IDEs (issue #416).
- **CI supply-chain hardening.** Pinned GitHub Actions to commit SHAs
  and tightened token scopes in response to recent third-party action
  compromises (#419).

## Bug Fixes

- **WASM `lreplace` multi-value form (lreplace-4.7.1 / 4.11).** The
  dedicated `tcl_cmd_lreplace_list` runtime helper now ships in the
  runtime WASM artefact so multi-value `lreplace LIST FIRST LAST v1
  v2 ...` resolves `end-N` against the original list rather than
  drifting slot-by-slot through a chain of inserts.
- **eglot painter accumulation tolerance.** The `test_issue333`
  threshold is loosened so the test no longer trips on minor upstream
  eglot delta-painter variation while still catching the underlying
  regression (#415).
- **Per-folder config race condition.** Fixes a race where folder
  configuration could be applied against stale state when settings
  arrived during workspace push (#414, #407).
- **Pull-diagnostics test fixture leak.** `_install_diag_capture` now
  also restores `_dp._server`, so suites running downstream of
  `test_per_folder_config` see the real server and
  `text_document_publish_diagnostics` monkeypatches intercept as
  intended.
- **Call-graph completeness.** Scan script bodies inside conditions
  and stub `BODY` args so the call graph picks up procedures invoked
  from `if {[…]} { proc … }`–style patterns (#410).
