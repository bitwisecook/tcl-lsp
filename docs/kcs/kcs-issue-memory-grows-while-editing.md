# KCS: memory use climbs steadily while you edit

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors

## Question

The Tcl Language Server's memory use keeps rising the longer I edit a file,
and never comes back down — how do I tell whether I am hitting the known
leak, and what fixes it?

## Symptoms

- The language-server process grows by hundreds of megabytes over a single
  editing session, and keeps growing the longer you type.
- Memory never falls back after you stop typing, close the file, or close
  the whole workspace — only restarting the server recovers it.
- Growth tracks *keystrokes*, not file size: a small file edited for a few
  minutes grows as much as a large one opened and left alone.
- Reported in [issue #1035](https://github.com/bitwisecook/tcl-lsp/issues/1035).

## Answer

Upgrade to a release containing the fix, then restart the language server.

1. Check your installed version. In VS Code it is the `tcl-lsp v<version>`
   badge in the bottom-right status bar; elsewhere, read the version from
   the server's output channel at startup.
2. If it is **v2.1.14 or earlier**, upgrade. Versions v2.1.9 through
   v2.1.14 leak roughly half a megabyte per keystroke.
3. Restart the language server (**Tcl: Restart Language Server**, or reload
   the editor window). The leaked memory is only reclaimed by restarting the
   process — upgrading alone does not release what a running server already
   holds.
4. Success signal: with the same file open, type for a minute and watch the
   server process in your OS task manager. Memory should rise to a plateau
   and stay there, rather than climbing without limit.

If memory still grows without a plateau on a current release, collect the
output channel log and open an issue with the file that reproduces it.

Nothing about your configuration, workspace size, or dialect affects the
affected versions — the leak there is per-edit and unconditional, which is
why it looks like a slow, steady climb rather than a spike on a particular
file. If your climb only appears on one file or one project, it is not this.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [kcs-qa-when-to-restart-server.md](kcs-qa-when-to-restart-server.md) —
  when restarting the server is the right remedy.
- [Incremental analysis design](../design/rust/incremental-analysis.md) —
  the per-keystroke analysis path and its fallback telemetry.
