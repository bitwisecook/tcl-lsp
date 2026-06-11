# Hardening the `lsp_e2e` integration suite

`tests/lsp_e2e/` is the backend-neutral JSON-RPC integration suite: it drives a
real language-server subprocess over the wire and asserts the *observable* LSP
contract, so the same battery can certify either backend.

## Running both backends

```sh
# Python reference server (default)
make test-lsp-e2e
#   == uv run --extra dev pytest tests/lsp_e2e/ -q -p no:cacheprovider

# Native Rust server
make rust-server                      # builds target/{release,debug}/tcl-lsp-server
make test-lsp-e2e-rust                # or set TCL_LSP_SERVER_BIN explicitly
#   == TCL_LSP_SERVER_KIND=rust TCL_LSP_SERVER_BIN=<bin> \
#        uv run --extra dev pytest tests/lsp_e2e/ -q -p no:cacheprovider
```

Backend selection lives in `harness.py` (`server_kind()`, `native_server_bin()`,
`server_launch_argv()`); `conftest._lsp_build` builds/locates the right artifact
and fails fast in `rust` mode when no binary is present. The native server lives
on the `rust` branch — on a `main`-based checkout `make rust-server` no-ops, and
rust-mode runs are done by pointing `TCL_LSP_SERVER_BIN` at a binary built from a
worktree of that branch.

**Rule:** every test must pass against the Python reference server. Rust-mode
failures are *recorded parity gaps* (below), never a reason to weaken an
assertion.

## What this hardening added

| Area | File | What it pins |
|------|------|--------------|
| Feature toggles / effective config | `test_config_e2e.py` | `getEffectiveConfig` exposes a resolved `features` map; each toggleable provider returns empty when disabled and recovers; optimiser switch round-trips; formatting honours request indent; no sticky state |
| Config injection | `harness.py` | `apply_configuration()` / `config_session()` drive `workspace/configuration` and settle on `getEffectiveConfig` with no sleeps; restore the full prior feature map |
| Semantic-token invariants | `_lsp_helpers.semantic_token_violations`, `test_semantic_tokens_e2e.py::TestTokenInvariants` | ordered, **non-overlapping**, in-bounds, UTF-16-correct, legend-valid — over a representative + adversarial corpus |
| Token alignment under edits | `test_edit_tracking_stress_e2e.py::TestTokenAlignmentUnderEdits` | multi-cursor rename / block-indent (one `didChange`, many edits), random storms checked at every settled checkpoint, multibyte UTF-16 tracking |
| Diagnostic canaries | `test_diagnostics_e2e.py::TestDiagnosticCanaries` | matched fire/silent pairs for W210, W307, W220 + clean negative control |
| Universal range/edit invariants | `_lsp_helpers.{iter_ranges,range_violations,workspace_edit_violations}`, `test_invariants_e2e.py` | every provider's `Range` is well-formed; rename/code-action edits are disjoint |
| Robustness / adversarial | `test_invariants_e2e.py::TestRobustnessAdversarial` | unterminated delimiters, deep nesting, multibyte/emoji, CRLF, BOM, empty, large files never hang/crash or yield a bad span |

### Bug found and fixed in the Python encoder

The new token-overlap invariant caught a real off-by-one: a string ending
immediately after an interpolation (`"$name"`) emitted the closing quote as a
**2-wide** token over a 1-char quote, over-running the line (the
"Overlapping semantic tokens detected" class). Fixed in
`server/features/_semantic_tokens/_collect.py` (an empty completing `ESC`
segment now renders as a single closing quote).

## Recorded Rust-mode parity gaps

Captured against a debug build of the `rust` branch
(`8f550a5e`). These are the value of the both-backend gate — each is a real
divergence to close in the native server, not test noise.

### `getEffectiveConfig` / feature toggles — `test_config_e2e.py`
- Native `getEffectiveConfig` returns `{}` (no `features`/`dialect`/scalars), so
  **14/15** config tests fail: it honours no feature toggle and no optimiser
  switch. This is the single biggest parity gap.

### Semantic-token alignment — `test_semantic_tokens_e2e.py::TestTokenInvariants`
- `unicode_after_var`: `token 4 overlaps previous token on line 1` — the native
  encoder emits overlapping tokens around an interpolation followed by unicode.
- dense single line (`...$b";# tail`): `token 6 overlaps previous` — the same
  closing-quote-after-interpolation +1 the Python side was fixed for, still
  present natively.

### Diagnostics — `test_diagnostics_e2e.py::TestDiagnosticCanaries`
- W210 read-before-set on a path merge and use-after-unset: **not emitted** by
  the native dataflow.
- W220 dead store: not emitted (fire case) / behaviour differs.
- W307: **fires on the opaque dispatch target** `$self` where Python suppresses
  it (a native false positive).
- A clean dict/flow proc emits a native optimiser hint (`O129`) where Python is
  silent.

`test_invariants_e2e.py` (range well-formedness + robustness) and the
token-alignment edit-storm tests otherwise **pass** in rust mode — the native
server's ranges are well-formed and it survives the adversarial corpus.

## VS Code ↔ `lsp_e2e` alignment

The two suites are now intentionally complementary:

- **`lsp_e2e`** owns the backend-neutral protocol contract (every provider, the
  invariants above, both backends). It is the spec the Rust port must meet.
- **VS Code** (`editors/vscode/src/test/`) owns the *client-integration* layer
  that `lsp_e2e` cannot reach: middleware config resolution (editor-global
  inheritance for tri-state toggles), command-palette wiring, snippet/grammar
  registration, code-lens/inlay rendering through the VS Code API, and
  multi-folder workspaces.

Cases that previously lived **only** in the VS Code suite and are now mirrored
in `lsp_e2e` so they bind both backends:

- "Configuration Settings → disabling features.X" → `test_config_e2e.py`
  (`getEffectiveConfig` shape + per-provider disable + optimiser round-trip).

### Migration status of the in-process pytest suite

`lsp_e2e` is the e2e home for protocol behaviour; the large in-process suites
remain the authoritative *precision* oracles and are surfaced on the wire via
thin canaries rather than wholesale duplication:

- `tests/test_fp_*.py` + `tests/test_ground_truth_tn_fn.py` (locked to tclsh
  9.0.3) stay in-process; `TestDiagnosticCanaries` certifies one case per family
  reaches `publishDiagnostics`.
- `tests/test_semantic_tokens.py` stays the exhaustive encoder oracle;
  `TestTokenInvariants` certifies the wire output satisfies the universal
  invariants on both backends.

### Follow-ups

- Route the full FP / ground-truth battery through the server under an env flag
  so it certifies either backend end-to-end (not just the canaries).
- A cross-backend parity meta-test that diffs Python vs Rust responses over the
  corpus in one run (needs two server processes; today parity is enforced by
  running the suite in both modes).
- Statement / selection-range / symbol `end` columns include the line
  terminator (end col = line length, +1 for CRLF `\r`); clients clamp it, so the
  universal range check leaves strict end-column off by default. Worth tightening
  in the range computation as a follow-up.
