# KCS: Which files keep their problems, and what the badge reflects

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors

## Question

Which Tcl files keep their problems (errors and warnings) in the
**Problems** panel and the File Explorer once I am no longer looking at
them, and do those problems describe my unsaved edits or what is on disk?

## Answer

A file keeps its problems after you close its editor tab. Closing a tab
does not retract a file's diagnostics, so the **Problems** panel and the
File Explorer badge still list files you are not currently looking at.

What changes on close is *which text* the problems describe. While a file
is open, its diagnostics track the buffer you are editing, unsaved changes
included. Once it closes, the server recomputes them from the file's
**on-disk** contents and republishes, so the badge reflects what is saved
— not a buffer you closed without saving.

Two boundaries follow from that:

- **A file you have never opened does not gain a badge on its own.** Open
  it once to have its problems tracked.
- **Deleting the file, or removing its workspace folder, clears the
  badge** — as you would expect.

If a file's problems do disappear after you close its tab, that is a
fault: collect the **Tcl LSP** output channel log and open an issue.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [Why diagnostics appear progressively](kcs-qa-why-diagnostics-appear-progressively.md)
- [Suppressing diagnostics](kcs-howto-suppress-diagnostics.md)
