# KCS: feature — Runtime Validation

> **Audience:** User
> **Type:** Functionality

## Summary

Check the open document with a real `tclsh`: a completeness check for Tcl, a stubbed evaluation for iRules.

## Applies to

VS Code

## Availability

| Context | How |
|---------|-----|
| VS Code | `Tcl: Run Runtime Validation` |

## How to use

- **VS Code**: Run `Tcl: Run Runtime Validation` from the command palette.
  It needs a `tclsh` on your PATH. The result arrives as a notification —
  `Runtime validation passed (…)`, or `Runtime validation failed (…)` with
  the interpreter's own message.
- **On save**: turn on `tclLsp.runtimeValidation.enabled` to run the same
  check after every save. Failures then go to the status bar.

## Settings

- `tclLsp.runtimeValidation.tclshPath` — interpreter to run. Default `tclsh`.
- `tclLsp.runtimeValidation.timeoutMs` — give up after this long. Default 5000.
- `tclLsp.runtimeValidation.adapter` — `auto`, `tcl-syntax`, or `irules-stub`.
- `tclLsp.runtimeValidation.enabled` — also validate on save. Default off.

## Operational context

There are two adapters. On `auto`, the dialect picks one.

- **Tcl syntax adapter** — reads the file and runs `info complete`. It never
  evaluates your code, so a script with side effects is safe to check. It
  catches unbalanced braces, brackets, and quotes, and nothing else.
- **iRules stub adapter** — chosen for the iRules dialect. It stubs `when` and
  `unknown`, then evaluates the script at the global level, so a malformed
  `when` header or an unparsable event body is reported.

## Failure modes

- `tclsh` is not on PATH, or the script outruns `timeoutMs`.
- The iRules adapter evaluates the script, so anything written outside a
  `when` body really runs.

## Test anchors

- `editors/vscode/src/test/runtimeValidation.test.ts`

## Example

Running **Tcl: Run Runtime Validation** on this iRule:

```tcl
when HTTP_REQUEST priority high {
    pool web
}
```

VS Code shows:

```
Runtime validation failed (iRules stub adapter): Invalid priority 'high' for event 'HTTP_REQUEST'
```

A `priority` must be an integer, and the stub `when` says so before the rule
ever reaches a BIG-IP.

## Discoverability

- [KCS feature index](README.md)
- [VS Code extension contracts](../../../docs/design/contracts/vscode-extension.md)
