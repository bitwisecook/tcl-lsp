# KCS: How do I hide Tcl diagnostics in diff and compare views?

> **Audience:** User
> **Type:** How-To

## Applies to

VS Code, diagnostic

## Question

Why do Tcl errors and warnings not appear while I review a change in a Git
diff, in **Compare With…**, or in **Compare with Saved** — and how do I
control that?

## Before you start

- Use VS Code. This setting is a VS Code display choice; other editors
  do not have it.
- Know that a diff editor's right-hand side is the real working file, so
  it is analysed like any open file. The squiggles it would show are the
  file's **whole-file** diagnostics — mostly findings that predate the
  change you are reviewing — not diagnostics about the change itself.

## Answer

By default the extension hides a Tcl file's diagnostics while it is shown
**only** in a diff editor, so you can read a change without the analyser's
noise. This is controlled by `tclLsp.suppressDiagnosticsInDiffEditors`
(default: on).

If the same file is also open in a normal editor — where you might be
editing it — its diagnostics stay visible there, so analysis of files you
are working on is never dimmed.

To keep diagnostics visible in diffs instead:

1. Open **Settings** (`Ctrl+,` / `Cmd+,`).
2. Search for `suppressDiagnosticsInDiffEditors`.
3. Untick **Tcl Lsp: Suppress Diagnostics In Diff Editors**.

Or add it to `settings.json`:

```json
"tclLsp.suppressDiagnosticsInDiffEditors": false
```

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
setting on (the default), the diff opens without squiggles or entries in
the **Problems** panel for that file. Open the same file normally and the
diagnostics appear.

## Related

- [KCS index](README.md)
- [How do I turn a diagnostic, optimisation, or shimmer off?](kcs-howto-suppress-diagnostics.md)
- [Glossary](../GLOSSARY.md)
