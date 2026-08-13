# KCS: When do I need to restart the Tcl Language Server?

> **Audience:** User
> **Type:** Q&A

## Applies to

all-editors

## Question

When do I need to restart the Tcl Language Server?

## Answer

In normal editing, you should never need to restart the server. The
server watches your open files, picks up every edit as you type, and
re-runs its analysis in the background. Saving a file, renaming a symbol,
switching between files, and hot-swapping between dialects all work
without a restart.

There are three situations where a restart is the right thing to do:

1. **The server has crashed or become unresponsive.** You see a
   `Connection to server got closed. Server will restart.` line in the
   **Tcl Language Server** output channel, or features like **Go to
   Definition** stop responding. See
   [LSP features are missing in VS Code](kcs-issue-lsp-features-are-missing.md)
   for how to read the startup log.
2. **You changed a tcl-lsp setting that is read only at startup.** A few
   settings — the native server binary path and the log level — are read
   once when the extension activates. After you change any of these in
   **Settings**, restart the server to pick them up.
3. **You installed a new version of the extension, or rebuilt the server
   binary in a development checkout.** VS Code will usually prompt you to
   reload the window in this case, but if you updated the server path by
   hand you may need to restart manually.

To restart the server in VS Code, run **Tcl: Restart Language Server**
from the Command Palette (`Ctrl+Shift+P` on Linux and Windows,
`Cmd+Shift+P` on macOS). Other editors expose the same action under a
similar name; see the editor's `README.md` for the exact label.

If you find yourself reaching for the restart command to fix an analysis
problem (missing diagnostics, stale hover text, a reference that should
exist but does not), that is usually a bug — please file an issue with
the output channel log attached rather than restarting silently.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [LSP features are missing in VS Code](kcs-issue-lsp-features-are-missing.md)
- [VS Code extension contracts](../design/contracts/vscode-extension.md)
