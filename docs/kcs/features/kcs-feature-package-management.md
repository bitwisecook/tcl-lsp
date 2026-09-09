# KCS: feature — Package Management

> **Audience:** User
> **Type:** Functionality

## Summary

Insert `package require` statements with symbol-aware suggestions and duplicate prevention.

## Applies to

VS Code

## Question

How do I quickly add a `package require` for a package I need?

## How to use

Run **Tcl: Insert package require** from the Command Palette (`Ctrl+Shift+P` or `Cmd+Shift+P`).

A picker appears with package suggestions. If the cursor is on a command from a known package (for example, `json::json2dict`), the picker lists only the packages that provide it (`json`). Otherwise, it lists every package the server knows about.

The command:

1. Checks whether a `package require` for the chosen package already exists in the file. If so, it skips the insertion and tells you.
2. Inserts the `package require` line near the top of the file, after any existing `package require` statements.

## Example

With the cursor on `http::geturl`, running the command shows:

```
Insert package require
Select package to require for 'http::geturl'
  http
```

Selecting `http` inserts `package require http` after the last existing `package require` line, and reports `Inserted 'package require http'.`

## Related

- [KCS feature index](README.md)
- [Completions](kcs-feature-completions.md) — auto-completes commands from required packages
- [Unknown Command Resolution](kcs-feature-unknown-command-resolution.md) — W123 warnings for commands without a `package require`
