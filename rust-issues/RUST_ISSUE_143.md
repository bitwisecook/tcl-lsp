# RUST_ISSUE_143: both npm scripts copy from the retired `../../tooling/explorer/static/` path behind an `fs.existsSync` guard, so `npm run compile` (incl. vsce `vscode:prepublish`) silently skips bundling `explorer-core.js` and the explorer WASM; only the Makefile's parallel copy from `rust/tcl-cli/gui` masks it

> Part of the origin/rust (tip 6820b3c) branch bug sweep (2026-07-07). Master index: [../RUST_ISSUES.md](../RUST_ISSUES.md).

| Field | Value |
|---|---|
| **Severity** | medium |
| **Subsystem** | Build tooling & CI |
| **Location** | `editors/vscode/package.json (copy-core-js, copy-wasm)` |
| **Status** | Fixed |
| **Verification** | Reported by review agent (confidence: high) |

## Finding

editors/vscode/package.json (`copy-core-js`, `copy-wasm`) — both npm scripts copy from the retired `../../tooling/explorer/static/` path behind an `fs.existsSync` guard, so `npm run compile` (incl. vsce `vscode:prepublish`) silently skips bundling `explorer-core.js` and the explorer WASM; only the Makefile's parallel copy from `rust/tcl-cli/gui` masks it.
Anyone building/packaging outside `make` ships a VSIX whose explorer webview assets are missing, with no error. Confidence: high
