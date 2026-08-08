# Python → Rust parity scrub

A deep audit of the Rust LSP port against the reference Python implementation,
focused on **config/settings parity**, **diagnostic/optimiser/code completeness**,
and **test-coverage gaps**. Motivated by a run of *parsed-but-not-applied* config
features that shipped because the Python tests that assert them were never ported.

Status legend: ✅ done · 🔶 intentional divergence (documented) · ⬜ large feature, out of scope.

## Completed

Config-application + the preceding config work:

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
- ✅ O111 brace-expression performance hint now paired with every W100.

Preview tickets:

- ✅ #726 — W114 false positive on `[expr]` inside a command substitution
  (`first_nested_expr` now only flags a depth-0 `[expr]`).
- ✅ #727 — parameter go-to-definition / references / rename resolve to the
  parameter name, not the proc name (proc) or method body (TclOO).
  `param_name_spans` recovers each parameter name's source span.
- ✅ #720/#721/#723/#724/#725 — verified correct in Rust with unit + (where
  applicable) LSP-e2e + vscode tests.

Tier 1:

- ✅ **T1-a** — default-off diagnostics are enableable via config. The opt-in
  codes are seeded into the resolved disabled set (`default_disabled_set`), and
  `tclLsp.diagnostics.<CODE>: true` removes a code to enable it; the hard
  publish-time `DEFAULT_OFF_CODES` filter is gone.
- ✅ **T1-b** — `tclLsp.formatting.*` settings reach the formatter
  (`formatter_config_from` maps every key; LSP options override indentation);
  config-file `[formatting]` maps the full section.
- ✅ **T1-c** — covered by existing tests (optimiser unknown-profile fallback;
  willSaveWaitUntil opt-in).

Tier 2:

- ✅ **T2-a** — `extraCommands` and `genericVariablePatterns` are per-folder
  (per-folder salsa `AnalyserConfig` handles + `resolved_*` for the uncached
  path). `libraryPaths` per-folder values are unioned into the workspace package
  resolver so they take effect (the package DB is additive). *Note:* full
  per-folder package-resolver isolation (a folder's packages visible only to its
  own docs) remains future work — today they refine W120 workspace-wide.

Tier 3:

- ✅ **T3-b** — MSYS2 / Cygwin (`MSYSTEM` set) config dir uses XDG `~/.config`
  instead of `%APPDATA%` (`config_path_for` `posix_compat_windows` flag).
- ✅ **T3-c** — `normalize_config_payload` accepts nested / flat-dotted /
  unwrapped (JetBrains) config payloads.
- ✅ **T3-d** — INI continuation lines containing `:` / `=` (e.g. `mylib::send`,
  colon-bearing regex patterns) join correctly (configparser semantics).

## Remaining — intentional divergences / out-of-scope (documented decisions)

### 🔶 T3-a · W123 shown by default (Python is opt-in) — kept on, deliberately
Python makes W123 (unknown command) opt-in because its unresolved-command check
is noisy. The Rust port has materially more precise W123 resolution — cross-file
`project_diagnostics` suppression (#723 work), `extraCommands`, and
user-`unknown`-handler detection — so it is **intentionally default-on**, and
multiple server tests assert it appears by default. It is fully controllable:
`tclLsp.diagnostics.W123: false` disables it (via the same seed/enable mechanism
as T1-a), and it is reported in `getEffectiveConfig`. Flipping the default would
hide a flagship, low-false-positive diagnostic, so this divergence is kept on
purpose rather than ported.

### ⬜ T3-e · P100 PGO (`f5 irule pgo`) — large compiler feature, out of scope
Python's profile-guided branch-reorder optimisation (`compiler/pgo`, code P100)
is a substantial codegen pass, not a config knob. Issue #1315 found the Rust
CLI verb advertised in `--help` with a working-sounding description while
always exiting 2 — a stub dressed up as a real verb, worse than an honest
gap. Resolved by removing `irule pgo` from the command surface entirely
(`f5-cli/.../irule.rs`, `cli.rs`) rather than keeping the deferral stub;
`--help` no longer lists a command that cannot run. Still tracked as a
standalone feature port, outside this config-parity sweep, should someone
build the real branch-reorder engine.

### ⬜ T3-f · W130–W134 tclpkg diagnostics — low-confidence, not emitted
Defined in both code registries but with no clear emission path in either
implementation (the Python LSP emission site is also unclear). Left unported
pending confirmation of whether the package resolver should surface
lock/sync/integrity warnings.

### 🔶 T3-g · Cosmetic config logging — behavior correct, logging not ported
Unknown diagnostic/optimiser codes and unknown optimiser profiles are silently
ignored (Rust's behavior is harmless — unknown codes never match; an unknown
profile falls back to the default, which is tested). Python additionally logs a
WARNING; that purely-cosmetic logging is intentionally not ported.

## Test-port backlog (server-lifecycle layer is the exposure)

The compiler, VM, and per-feature LSP providers are strongly tested in Rust. The
gap is the server-lifecycle layer in `rust/tcl-lsp-server/src/lib.rs`. The
config/feature-handler half has now been substantially covered by this pass
(feature-toggle enforcement, default-off enable, formatting passthrough,
per-folder config, payload normalisation, INI edge cases). Remaining
higher-value Python suites to port for breadth:

1. `test_pull_diagnostics.py` — result-id increment / unchanged-report semantics.
2. `test_async_diagnostics.py` — debounce / coalesce / cancel scheduler.
3. `test_incremental_diagnostics.py` — re-offset-equals-recompute soundness.
4. `test_lsp_server_actions_e2e.py` — apply-action → re-analyse → diagnostic-clears.
5. `test_settings_scope.py` — `editors/vscode/package.json` config-scope metadata.
