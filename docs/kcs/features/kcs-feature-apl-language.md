# KCS: feature — APL (iApp Presentation Language) support

> **Audience:** User
> **Type:** Functionality

## Summary

Semantic highlighting, cross-file diagnostics, and embedded Tcl support for
F5 iApp APL (Application Presentation Language) files.

## Applies to

all-editors

## How to use

- **Editor**: Open a `.apl` file or a file named `presentation`. The language
  is auto-detected and semantic tokens are applied.
- **VS Code language ID**: `tcl-apl` (alias: "iApp APL", "apl", "presentation").
- **Settings**: Semantic tokens toggle with `tclLsp.features.semanticTokens`.
- **Cross-file**: Place `presentation` and `implementation` (or `.apl` and
  `.iapp`) files in the same directory for cross-validation diagnostics.

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

### Embedded Tcl

`[...]` bracket expressions inside APL source receive full Tcl semantic
tokenisation.  This covers `[tmsh::create ...]`, `[iapp::conf ...]`, and
other command substitutions.

### Cross-file diagnostics

When a presentation file and implementation file coexist in the same directory:

| Code | Severity | Description |
|---|---|---|
| IAPP7001 | WARNING | Implementation `$::section__field` references a field not defined in presentation |
| IAPP7002 | HINT | Presentation field never referenced in implementation |
| IAPP7003 | WARNING | `#include` file not found |

### Variable naming convention

APL qualified names (`section.field`) map to Tcl globals with double
underscores: `$::section__field`.

### #include resolution

`#include "file"` directives are resolved relative to the APL file's
directory.  Resolution is recursive with circular-include protection.

### tmsh:: commands

30+ `tmsh::` namespace commands and 4 `script::` commands are registered in
the `f5-iapps` and `f5-tmsh` dialects with hover documentation and arity
validation.

## Failure modes

- APL-specific tokens not emitted when `is_apl=False` (language detection miss).
- Cross-file diagnostics not triggered when files are in different directories.
- `#include` resolution fails if the included file uses a different encoding.
- New APL keywords not recognised after spec changes.

## Example

A small APL presentation file:

```
section "Basic"
string web_server_ip {
    default ""
    required "true"
    validator "IpAddress"
}
choice protocol {
    display "medium"
    default "HTTPS"
    value "HTTP" { display "HTTP" }
    value "HTTPS" { display "HTTPS" }
}
```

Opened in the editor, `section` is coloured as an `aplSection`,
`string` and `choice` as `aplFieldType`, `validator` as
`aplAttribute`, `IpAddress` as `aplValidator`, and the field names
`web_server_ip` and `protocol` as `aplFieldName`. If a sibling
implementation file references `$::Basic__web_server_ip_typo`, the
implementation file shows an **IAPP7001** warning squiggle on the
typo.

## Discoverability

- [KCS feature index](README.md)
- [Semantic tokens feature](kcs-feature-semantic-tokens.md)
