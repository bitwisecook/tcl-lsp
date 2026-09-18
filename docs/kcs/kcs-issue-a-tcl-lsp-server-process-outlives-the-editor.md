# KCS: a `tcl-lsp-server` process outlives the editor and keeps every core busy

> **Audience:** User
> **Type:** Issue

## Applies to

all-editors

## Question

The editor window is closed but a `tcl-lsp-server` process is still running,
using several cores and gigabytes of memory — is it safe to kill, and how do
you stop it happening again?

## Symptoms

- `ps` or a task manager shows `tcl-lsp-server` (under the editor's extension
  directory, for example
  `~/.vscode/extensions/bitwisecook.tcl-lsp-<version>-<platform>/server/`)
  after every window that used it has closed. On Linux its parent is the
  per-user `systemd` or `init`, not the editor.
- CPU stays near one full core per available worker (400 % on a four-core
  machine) and resident memory keeps growing, for hours.
- The workspace is large and contains generated Tcl — Intel Quartus
  `ip/altera` trees are the reported case — and the server's log never
  printed `[timing] workspace_folders_scan` before the window closed.

## Answer

Kill the process. It belongs to a session that no longer exists and holds
nothing you need: a server that has lost its editor cannot deliver anything
it computes, and the next window starts a fresh one.

1. Find it: `pgrep -fa tcl-lsp-server` (Linux and macOS) or the Task Manager
   details pane on Windows.
2. Kill it: `kill <pid>`; if it is still there after a few seconds,
   `kill -9 <pid>`.
3. Update the Tcl extension. From the release after 2.2.4 the server
   exits within three seconds of the editor closing its connection, whatever
   it was doing, and the analysis that kept it busy (a read-before-set check
   that grew exponentially on long chains of conditional writes) runs in
   linear time. Builds up to and including 2.2.4 can leave an orphan whenever
   a window closes while the workspace scan is still running.
4. Confirm: close the window and check `pgrep -fa tcl-lsp-server` again a
   few seconds later; nothing should be listed for that window.

If a current build still leaves a process behind, collect the server's
output (in VS Code the **Tcl Language Server** output channel, saved before
closing) and the `ps` line, and file an issue.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [kcs-issue-lsp-features-are-missing.md](kcs-issue-lsp-features-are-missing.md)
