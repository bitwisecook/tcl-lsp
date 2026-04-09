# KCS: The Tcl Language Server does not seem to be running in VS Code

> **Audience:** User
> **Type:** Issue

## Question

How do I tell whether the Tcl Language Server started inside VS Code, and
where do I read its startup log if it did not?

## Symptoms

- Your `.tcl`, `.tm`, `.itcl`, `.irul`, or `.iapp` file shows only plain
  keyword colours. Variables, proc names, and commands do not get the
  richer colours from the extension screenshots.
- No red or yellow squiggles appear, even on obviously broken code such
  as unclosed braces or unknown commands.
- Hovering a command or variable does not show a tooltip.
- Typing does not offer auto-completion for commands, subcommands, or
  iRule events.
- **Go to Definition**, **Find References**, and rename do nothing or
  say "No definition found".
- The `tcl-lsp v<version>` badge and the dialect indicator are missing
  from the bottom-right status bar.
- `Tcl: ...` entries in the Command Palette (for example **Tcl: Restart
  Language Server**, **Tcl: Optimise Document**, **Tcl: Open Compiler
  Explorer**) are missing, greyed out, or error as soon as you run them.

## Answer

1. Open the Output panel: **View > Output**, or press `Ctrl+Shift+U` on
   Linux and Windows, or `Cmd+Shift+U` on macOS.
2. In the dropdown on the right of the Output panel, select **Tcl
   Language Server**.
3. Open any `.tcl`, `.tm`, `.itcl`, `.irul`, or `.iapp` file so the
   extension activates, if it has not already.
4. Look for a healthy startup sequence in the channel:
   - If you installed the extension from the Marketplace or a VSIX, you
     should see lines beginning with `Python discovery:`, one line per
     candidate interpreter, followed by `Selected: <path> (<version>)`
     and `[timing] Python discovery: Nms`.
   - If you are running from a git checkout, you should see a single
     `Dev mode: using uv in <serverDir>` line.
   - In both cases, the startup ends with `Server initialized`. A
     repeated `Connection to server got closed. Server will restart.`
     line means the server crashed during startup.
5. Check the status bar in the bottom right. A healthy activation shows
   `tcl-lsp v<version>` and a dialect indicator. If you do not see
   either, the extension failed to activate before it could write to
   the channel — jump to step 7.
6. To see the protocol traffic as well, open **File > Preferences >
   Settings**, search for `tcl-lsp.trace.server`, and set it to
   `messages` or `verbose`. Then run **Tcl: Restart Language Server**
   from the Command Palette (`Ctrl+Shift+P` or `Cmd+Shift+P`) to replay
   the startup with tracing enabled.
7. If the output channel is empty, or does not exist, open **Help >
   Toggle Developer Tools** and look at the **Console** tab, or run
   **Developer: Show Logs... > Extension Host** from the Command
   Palette. Any stack trace there is the reason the extension failed to
   activate.
8. If you need to share the log on an issue, copy the contents of the
   **Tcl Language Server** output channel, and the Extension Host log
   if it is relevant, into the issue body.

## Related

- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
- [VS Code extension contracts](../design/contracts/vscode-extension.md)
- [LSP diagnostics publication](../design/contracts/lsp-diagnostics-publication.md)
