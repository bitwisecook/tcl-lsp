# KCS: Problems disappear from files after I close them

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors

## Question

Why do the problems (errors and warnings) on a Tcl file vanish after I close
its editor tab or open enough other files, and how are they kept?

## Symptoms

- A file that showed squiggles and a File Explorer badge loses them after its
  editor tab is closed.
- Over a working session the **Problems** panel and the File Explorer stop
  showing files you are no longer looking at, even though those files still have
  real issues.
- It feels like a step backwards from earlier builds, which kept the problems
  visible.

## Answer

This is fixed from tcl-lsp v2.1.6 onward — no action is needed beyond updating.

1. Update the tcl-lsp extension to the latest version.
2. Open a Tcl file that has a problem and confirm the squiggle and File Explorer
   badge appear.
3. Close its editor tab (or open several other files so the editor cycles it
   closed).
4. Confirm the file keeps its **Problems** entry and File Explorer badge — the
   success signal. Deleting the file, or removing its workspace folder, clears
   the badge as expected.

The server now recomputes a closed file's diagnostics from its **on-disk**
contents and republishes them, so the badge reflects what is saved on disk (not
a discarded unsaved buffer). A file you never opened does not gain a badge on
its own; open it once to have its problems tracked.

If the problems still disappear after updating, collect the **Tcl LSP** output
channel log and open an issue.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [Suppressing diagnostics](kcs-howto-suppress-diagnostics.md)
- [Duplicate diagnostics](kcs-issue-duplicate-diagnostics.md)
