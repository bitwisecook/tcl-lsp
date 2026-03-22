# KCS: feature — APL (iApp Presentation Language) support

## Summary

Semantic highlighting for F5 iApp APL (Application Presentation Language)
files used to define the presentation layer of iApp templates.

## Surface

lsp, vscode

## How to use

- **Editor**: Open a `.apl` file or a file named `presentation`. The language
  is auto-detected and semantic tokens are applied.
- **VS Code language ID**: `tcl-apl` (alias: "iApp APL", "apl", "presentation").
- **Settings**: Semantic tokens toggle with `tclLsp.features.semanticTokens`.

## Operational context

APL describes the user-facing form elements of an iApp template: sections,
field types (`string`, `choice`, `password`, `yesno`, `editchoice`, etc.),
attributes (`default`, `display`, `required`, `validator`), reusable `define`
blocks, `optional` conditionals, `text` label blocks, `table`/`row` structures,
and `#include`/`#inline` directives.

### Semantic token types

| Token type | Description |
|---|---|
| `aplSection` | `section`, `text`, `table`, `row` keywords |
| `aplFieldType` | Field type keywords (`string`, `choice`, `password`, …) |
| `aplAttribute` | `default`, `display`, `required`, `validator` |
| `aplSectionName` | Name following a section/table/row keyword |
| `aplFieldName` | Name following a field-type keyword |
| `aplDefine` | `define` keyword |
| `aplDefineName` | Name after `define` |
| `aplDirective` | `#include`, `#inline` |
| `aplOptional` | `optional` keyword |
| `aplValidator` | Validator value (`IpAddress`, `PortNumber`, `FQDN`, …) |

Standard Tcl token types (`variable`, `string`, `number`, `operator`, `escape`,
`comment`) are also emitted for the corresponding APL constructs.

## File-path anchors

- `core/bigip/apl_parser.py` — APL tokeniser
- `lsp/features/semantic_tokens.py` — `_collect_apl_tokens()`, token type legend
- `lsp/server.py` — `_is_apl_source()`, semantic token dispatch
- `lsp/workspace/scanner.py` — `.apl` extension and `presentation` basename detection
- `editors/vscode/package.json` — `tcl-apl` language registration and colours
- `editors/vscode/apl-language-configuration.json` — editor behaviour

## Failure modes

- APL-specific tokens not emitted when `is_apl=False` (language detection miss).
- New APL keywords not recognised after spec changes.

## Test anchors

- `tests/test_apl_parser.py` — APL tokeniser unit tests
- `tests/test_semantic_tokens.py::TestAplSemanticTokens` — end-to-end semantic token tests

## Discoverability

- [KCS feature index](README.md)
- [Semantic tokens feature](kcs-feature-semantic-tokens.md)
