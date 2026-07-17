# KCS: How do I hide Tcl diagnostics in diff and compare views?

> **Audience:** User
> **Type:** How-To

## Applies to

VS Code, diagnostic

## Question

How do I stop Tcl errors and warnings from appearing while I review a
change in a Git diff, in **Compare With…**, or in **Compare with Saved**?

## Before you start

- Use VS Code. This setting is a VS Code display choice; other editors
  do not have it.
- Know what the squiggles in a diff are. The analyser never runs on diff
  content — a diff editor is two real documents shown side by side, and
  the right-hand side of a Git diff is the real working file, analysed
  like any open file. The diagnostics you see are that file's own
  correct, whole-file diagnostics — mostly findings that predate the
  change you are reviewing, which is why hiding them can help.

## Answer

Diagnostics are shown in diffs by default. To hide them while a file is
shown **only** in a diff editor, turn on
`tclLsp.suppressDiagnosticsInDiffEditors`:

1. Open **Settings** (`Ctrl+,` / `Cmd+,`).
2. Search for `suppressDiagnosticsInDiffEditors`.
3. Tick **Tcl Lsp: Suppress Diagnostics In Diff Editors**.

Or add it to `settings.json`:

```json
"tclLsp.suppressDiagnosticsInDiffEditors": true
```

If the same file is also open in a normal editor — where you might be
editing it — its diagnostics stay visible there, so analysis of files you
are working on is never dimmed.

Changes take effect immediately: no window reload, and no edit to the
file. This never changes what the [Tcl Language Server](../GLOSSARY.md#lsp)
computes. The server keeps analysing every open file; the setting only
decides whether VS Code paints the result while the file is being viewed
as a diff, so no report is lost.

The setting is deliberately **not** under `tclLsp.diagnostics.*` — that
section is reserved for per-code on/off toggles (for example
`tclLsp.diagnostics.W100`), and the server reads every boolean key in it
as a diagnostic code.

## How to tell it worked

Open a modified `.tcl` file from the **Source Control** view. With the
setting on, the diff opens without squiggles or entries in the
**Problems** panel for that file. Open the same file normally and the
diagnostics reappear.

## Related

- [KCS index](README.md)
- [How do I turn a diagnostic, optimisation, or shimmer off?](kcs-howto-suppress-diagnostics.md)
- [Glossary](../GLOSSARY.md)
