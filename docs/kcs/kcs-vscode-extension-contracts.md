# KCS: VS Code extension contracts (client-side integration)

## Symptom

VS Code features (diagnostics, commands, semantic tokens) regress even when Python server tests pass.

## Operational context

The VS Code extension is the primary client integration surface for LSP capabilities and user-visible workflows.

## Decision rules / contracts

1. Extension behaviour must track LSP server capabilities and command metadata.
2. Client-side UX changes should preserve stable diagnostics/command expectations.
3. Extension integration changes require lint + compile + extension test coverage.

## File-path anchors

- `editors/vscode/src/extension.ts`
- `editors/vscode/package.json`
- `editors/vscode/src/test/`

## Failure modes

- Command registrations drifting from server feature set.
- Client filtering/rendering masking server diagnostics.
- Packaging/build changes breaking activation paths.

## Test anchors

- `editors/vscode/src/test/`
- `tests/test_vscode_tcl_issues.py`
- `.github/workflows/ci.yml` (`test-ext` job)

## Discoverability

- [KCS index](README.md)
- [LSP diagnostics publication model](kcs-lsp-diagnostics-publication.md)
- [pipeline LSP-first note](kcs-pipeline-lsp-first.md)
