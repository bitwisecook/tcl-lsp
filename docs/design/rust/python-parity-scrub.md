# Python → Rust parity scrub

A deep audit of the Rust LSP port against the reference Python implementation,
focused on **config/settings parity**, **diagnostic/optimiser/code completeness**,
and **test-coverage gaps**. Motivated by a run of *parsed-but-not-applied* config
features that shipped because the Python tests that assert them were never ported.

Status legend: ✅ done this pass · ⬜ open · 🔶 judgment call / behavior change.

## Already fixed (this scrub + the preceding config work)

- ✅ `genericVariablePatterns` → IRULE4002 (was parsed, never applied).
- ✅ Feature-toggle enforcement across 13 handlers + `workspaceFileOps`
  (semanticTokens, codeActions, rename/prepareRename, documentHighlight,
  codeLens, implementation, typeDefinition, declaration, linkedEditingRange,
  callHierarchy×3, workspaceSymbols, will/did-rename).
- ✅ `diagnostics` master switch (`tclLsp.features.diagnostics=false` → empty publish).
- ✅ `style.lineLength` → W111 (separate from the formatter width); config-file
  `[style] line_length` no longer conflated into the formatter key.
- ✅ LSP-feature recursion caps lifted 20/24 → 256 (folding, declaration,
  refactor, explorer CST) to match the analyser's `MAX_BODY_DEPTH`.
- ✅ O111 brace-expression performance hint now paired with every W100
  (gated on optimiser-enabled + O111-not-disabled), matching Python.
- ✅ Ported config robustness tests (multiline lists, invalid-value handling,
  empty dialect, combined sections).

## Open — Tier 1 (low-risk, recommend doing)

### T1-a · Default-off diagnostics cannot be enabled via config
- **What:** `DEFAULT_OFF_CODES = ["W242"]` (`rust/tcl-lsp-server/src/lib.rs`) is
  hard-filtered from the published set with no enable path. Python honours
  `tclLsp.diagnostics.W242: true` (`server/settings.py` seeds
  `default_disabled_diagnostics()` then `discard`s on a `true` value).
- **Impact:** Low (W242 "loop termination" is niche); but it is the same
  *silently-ignored-config* class we just fixed elsewhere.
- **Fix recipe (faithful):** seed the resolved disabled set with the catalogue's
  default-off codes (`DiagCode::default_on() == false`), let
  `diagnostics.<CODE>: true` remove a code, and delete the publish-time
  `DEFAULT_OFF_CODES` filter — the analyser already suppresses disabled codes
  (`analyser/state.rs:754`). Care: seed every analyser-building path
  (global field init, the salsa `AnalyserConfig` default, `settings_disabled_diagnostics`),
  and **do not** seed W123 (see T3-a).
- **Python tests to port:** `test_user_config::TestGetGenericVariablePatterns`
  enable cases; `test_per_folder_config::TestAnalyserOptInDiagnostics`.

### T1-b · `[formatting]` config never reaches the formatter
- **What:** the formatter handler builds `FormatterConfig` only from LSP
  `FormattingOptions` (`formatter_config_from_options`); the resolved
  `tclLsp.formatting.{lineLength,indentSize,indentStyle,braceStyle,goalLineLength}`
  is ignored, and `config_ini` drops every `[formatting]` key except
  `max_line_length`. Python merges these via `_normalise_formatter_settings`.
- **Impact:** Medium — INI/editor-configured indent/brace style and formatter
  width have no effect; LSP `tabSize`/`insertSpaces` still work.
- **Fix:** parse `tclLsp.formatting.*` into server state (per-folder), map the
  `[formatting]` INI keys in `config_ini`, and merge resolved config under the
  LSP options (LSP overrides per contract) in the `formatting`/`range_formatting`
  handlers. **Python tests to port:** `test_user_config::test_formatting`.

### T1-c · Config behaviors implemented but still untested
Port to lock them (the bug-catching class). Mostly done for `config_ini`; still
open at the server layer: `willSaveWaitUntil` default-off; no-workspace-folders
→ fallback-only pull; optimiser unknown-profile fallback.

## Open — Tier 2 (multi-root architecture, medium effort)

### T2-a · Per-folder `extraCommands` / `libraryPaths` / `genericVariablePatterns`
- **What:** all three are **global-only** in Rust (`apply_folder_configs`
  explicitly inherits the process-global value; `FolderConfig` has no fields for
  them). Python resolves each per folder.
- **Impact:** in a multi-root workspace these leak across folders.
- **Fix:** add the three fields to `FolderConfig`, parse them in
  `parse_folder_config`, and resolve per-uri (the salsa `AnalyserConfig` per-folder
  handles already exist for disabled/non-ascii). **Tests:** `test_per_folder_config`
  (`TestPerFolderPackageResolver`, `test_apply_feature_settings_picks_up_extra_commands`).

## Open — Tier 3 (judgment calls / larger features)

### T3-a · 🔶 W123 shown by default (Python = opt-in)
W123 (unknown command) is `default_on == false` in both catalogues, but Rust
publishes it by default (it is not in `DEFAULT_OFF_CODES`) while Python hides it
until enabled. Flipping it is entangled with the cross-file arity machinery
(`unresolved_command_sites` is recorded regardless of the W123 toggle) — needs a
deliberate product decision before changing.

### T3-b · MSYS2 / Cygwin config-dir resolution
`config_path_for` keys only off `(is_windows, is_macos)`; there is no
`_is_posix_compat_windows` equivalent, so on MSYS2/Cygwin Rust uses `%APPDATA%`
where Python uses `~/.config`. Edge platform. **Tests:**
`test_user_config::test_{windows_msys2,cygwin,msys_platform}_uses_xdg`.

### T3-c · Flat-dotted / unwrapped config payload shapes
Rust handles flat `tclLsp.diagnostics.<CODE>` and `tclLsp.style.nonAscii`, but not
flat `tclLsp.optimiser.*` / `tclLsp.features.*` / `tclLsp.xcDiagnostics.enabled`
nor the fully-unwrapped (JetBrains) shape. Low risk for the VS Code pull path
(nested objects). **Tests:** `test_server_config::test_flat_keys*`,
`test_unwrapped_payload`.

### T3-d · INI continuation heuristic misreads `::` names
An indented continuation line containing `:` (e.g. `mylib::send`, a regex with a
colon) is parsed as a new `key: value` rather than a continuation, so multiline
`extraCommands` / `genericVariablePatterns` with colons break. The comma form
works. Fix = indentation-depth-based continuation (configparser semantics)
instead of the `contains(':')` heuristic.

### T3-e · P100 PGO not ported
`f5 irule pgo` returns a `deferred("pgo")` error (`f5-cli/.../irule.rs`). The
Python branch-reorder PGO (`compiler/pgo`, code P100) is unported. Explicit
opt-in CLI verb; large feature.

### T3-f · W130–W134 tclpkg diagnostics
Defined in both registries but with no emission path in Rust (and an unclear one
in Python). Low confidence; confirm whether the package resolver should surface
lock/sync/integrity warnings.

### T3-g · Cosmetic config niceties
No WARNING log on mismatched section / unknown code / unknown profile; no
registry validation of unknown diagnostic/optimiser codes; `save_settings_to_config`
export covers fewer sections than Python; per-folder pull uses two requests vs
Python's single batched request (end-state matches).

## Test-port backlog (server-lifecycle layer is the exposure)

The compiler, VM, and per-feature LSP providers are strongly tested in Rust. The
gap is the server-lifecycle layer in `rust/tcl-lsp-server/src/lib.rs`. Highest
value Python suites to port:

1. `test_settings_scope.py` — validates `editors/vscode/package.json` config
   scopes (no Rust coverage at all).
2. `test_pull_diagnostics.py` — result-id increment / unchanged-report semantics.
3. `test_async_diagnostics.py` — debounce/coalesce/cancel scheduler.
4. `test_per_folder_config_integration.py` / `_e2e.py` / `.py` — multi-folder
   routing, isolation, runtime re-analysis on toggle.
5. `test_incremental_diagnostics.py` — re-offset-equals-recompute soundness.
6. `test_lsp_server_actions_e2e.py` — apply-action → re-analyse → diagnostic-clears.
7. The `issue_*` regression pins (format-on-save default, inlay toggle, etc.) —
   no Rust counterparts.
