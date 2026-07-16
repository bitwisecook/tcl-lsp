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
- Know that a diff editor's right-hand side is the real working file, so
  it is analysed like any open file — that is why its squiggles appear in
  the diff.

## Answer

Turn on `tclLsp.diagnostics.suppressInDiffEditors`.

1. Open **Settings** (`Ctrl+,` / `Cmd+,`).
2. Search for `suppressInDiffEditors`.
3. Tick **Tcl Lsp › Diagnostics: Suppress In Diff Editors**.

Or add it to `settings.json`:

```json
"tclLsp.diagnostics.suppressInDiffEditors": true
```

With the setting on, a Tcl file's diagnostics are hidden while it is shown
**only** in a diff editor. If the same file is also open in a normal editor
— where you might be editing it — its diagnostics stay visible there, so
analysis of files you are working on is never dimmed.

The change takes effect immediately: no window reload, and no edit to the
file. Turning the setting back off restores the diagnostics straight away.

This never changes what the [Tcl Language Server](../GLOSSARY.md#lsp)
computes. The server keeps analysing every open file; the setting only
decides whether VS Code paints the result while the file is being viewed
as a diff, so no report is lost.

## How to tell it worked

Open a modified `.tcl` file from the **Source Control** view. With the
setting on, the diff opens without squiggles or entries in the
**Problems** panel for that file. Open the same file normally and the
diagnostics reappear.

## Related

- [KCS index](README.md)
- [How do I turn a diagnostic, optimisation, or shimmer off?](kcs-howto-suppress-diagnostics.md)
- [Glossary](../GLOSSARY.md)
