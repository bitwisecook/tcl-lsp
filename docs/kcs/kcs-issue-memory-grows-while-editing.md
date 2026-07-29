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

1. Check your installed version — in VS Code, run **Tcl: Show Version** from
   the command palette; elsewhere, read the version from the server's output
   channel at startup.
2. If it is **v2.1.14 or earlier**, upgrade to the next release. Versions
   v2.1.9 through v2.1.14 leaked roughly half a megabyte per keystroke.
3. Restart the language server (**Tcl: Restart Server**, or reload the
   editor window). The leaked memory is only reclaimed by restarting the
   process.
4. Success signal: with the same file open, type for a minute and watch the
   server process in your OS task manager. Memory should rise to a plateau
   and stay there, rather than climbing without limit.

If memory still grows without a plateau on a current release, collect the
output channel log and open an issue with the file that reproduces it.

### What was actually wrong

Two of the generated command tables — the `tcl::mathop` operator ensemble and
the `tcl::mathfunc` math-function ensemble — build their command specifications
by leaking small strings, on the assumption that a `CommandRegistry` is
constructed once per process. It is not: the CFG builder constructed a fresh
default registry on *every* control-flow-graph build, which is several times
per keystroke, so every edit permanently leaked the whole ensemble. The
specifications are now built once and shared, and the hot paths take a cached
registry instead of constructing one.

Nothing about your configuration, workspace size, or dialect affects this —
the leak is per-edit and unconditional, which is why it looks like a slow,
steady climb rather than a spike on a particular file.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [kcs-issue-stale-compiler-cache.md](kcs-issue-stale-compiler-cache.md) —
  a different cache problem, where results rather than memory go wrong.
- [kcs-qa-when-to-restart-server.md](kcs-qa-when-to-restart-server.md) —
  when restarting the server is the right remedy.
- [Incremental analysis design](../design/rust/incremental-analysis.md) —
  the per-keystroke analysis path and its fallback telemetry.
