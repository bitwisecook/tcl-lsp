# VS Code extension contracts (client-side integration)

What the VS Code extension owns as the primary client integration surface for
LSP capabilities and user-visible workflows. Read it when diagnostics,
commands, or semantic tokens regress in the editor even though the server's
own tests pass.

## Decision rules / contracts

1. The extension renders what the server advertises; it never assumes a
   capability or a command the server did not declare at `initialize`.
2. A client-side filter or renderer never drops a diagnostic the server
   published — masking one here hides a server bug.
3. An extension change goes through `make lint-ts`, `make typecheck-ts`, and
   `make test-ext` (plus `npm run test:web` in `editors/vscode` when it
   touches the browser host).
4. Settings and command contributions in `package.json` are **generated**
   (`cargo xtask gen-vscode-package` / `gen-editor-settings` /
   `gen-editor-catalogs`) from the server-side declarations, so a new setting
   is one edit on the Rust side, not two that can drift.

## File-path anchors

- `editors/vscode/src/extension.ts` — desktop activation;
  `extensionBrowser.ts` + `webLspTransport.ts` — the browser host.
- `editors/vscode/package.json` — contributions (generated sections).
- `editors/vscode/src/generated/` — the xtask-generated catalogues.
- `editors/vscode/src/test/` — desktop suites; `src/test/web/` — browser.

## Failure modes

- Command registrations drifting from server feature set.
- Client filtering/rendering masking server diagnostics.
- Packaging/build changes breaking activation paths.

## Test anchors

- `editors/vscode/src/test/` — the extension test suite, run against the
  packaged extension.
- `rust/tcl-lsp-server/tests/e2e/` — the server-side behaviour the extension
  renders.
- `.github/workflows/ci.yml` (`test-ext` for the desktop host, `test-ext-web`
  for the browser host).

## Discoverability

- [Design doc index](../README.md)
- [LSP diagnostics publication model](lsp-diagnostics-publication.md)
- [LSP feature providers](lsp-feature-providers.md)
- [release and publish](release-and-publish.md)
