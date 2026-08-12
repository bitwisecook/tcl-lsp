# VS Code extension contracts (client-side integration)

## Symptom

VS Code features (diagnostics, commands, semantic tokens) regress even when the server's own tests pass.

## Operational context

The VS Code extension is the primary client integration surface for LSP capabilities and user-visible workflows.

## Decision rules / contracts

1. Extension behaviour must track LSP server capabilities and command metadata.
2. Client-side UX changes should preserve stable diagnostics/command expectations.
3. Extension integration changes require lint + compile + extension test coverage.
4. Settings and command contributions in `package.json` are **generated**
   (`cargo xtask gen-vscode-package` / `gen-editor-settings` /
   `gen-editor-catalogs`) from the server-side declarations, so a new setting
   is one edit on the Rust side, not two that can drift.

## File-path anchors

- `editors/vscode/src/extension.ts`
- `editors/vscode/package.json`
- `editors/vscode/src/test/`

## Failure modes

- Command registrations drifting from server feature set.
- Client filtering/rendering masking server diagnostics.
- Packaging/build changes breaking activation paths.

## Test anchors

- `editors/vscode/src/test/` — the extension test suite, run against the
  packaged extension.
- `rust/tcl-lsp-server/tests/e2e/` — the server-side behaviour the extension
  renders.
- `.github/workflows/ci.yml` (`test-ext` job).

## Discoverability

- [Design doc index](../README.md)
- [LSP diagnostics publication model](lsp-diagnostics-publication.md)
- [LSP feature providers](lsp-feature-providers.md)
- [release and publish](release-and-publish.md)
