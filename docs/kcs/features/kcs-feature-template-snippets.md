# KCS: feature — Template Snippets

## Summary

16 built-in Tcl and iRules code templates insertable from the command palette.

## Surface

vscode-command, all-editors

## Availability

| Context | How |
|---------|-----|
| VS Code | `Tcl: Insert Tcl Template Snippet` |
| Zed | Built-in snippets via completion |
| Sublime Text | Bundled snippets via tab completion |

## How to use

- **VS Code**: Run `Tcl: Insert Tcl Template Snippet` and pick from the list.
- **Other editors**: Type the snippet prefix (e.g. `tcl-proc`, `irule-http-request`) and use tab completion.

## Snippet list

| Prefix | Description |
|--------|------------|
| `tcl-proc` | Tcl procedure |
| `tcl-namespace` | Namespace eval block |
| `tcl-package` | Package provide/require boilerplate |
| `tcl-class` | oo::class definition |
| `tcl-if` | If/else block |
| `tcl-foreach` | Foreach loop |
| `tcl-for` | For loop with braced expressions |
| `tcl-switch` | Switch with `--` option terminator |
| `tcl-catch` | Catch with result/options preservation |
| `tcl-try` | Try/trap block |
| `tcl-dict-for` | Dict iteration |
| `irule-rule-init` | RULE_INIT handler |
| `irule-http-request` | HTTP_REQUEST skeleton |
| `irule-redirect-https` | HTTP to HTTPS redirect |
| `irule-collect-release` | HTTP collect/release pair |
| `irule-class-lookup` | Data-group lookup and routing |

## File-path anchors

- `editors/vscode/src/templateSnippets.ts`
- `editors/zed/snippets/`
- `editors/sublime-text/Snippets/`

## Failure modes

- Snippet produces invalid code for a specific dialect.

## Test anchors

- `editors/vscode/src/test/templateSnippets.test.ts`
- `editors/vscode/src/test/snippets.test.ts`

## Screenshots

- `24-template-snippets` — template picker showing available snippets

![template picker showing available snippets](../screenshots/24-template-snippets.png)

## Discoverability

- [KCS feature index](README.md)
