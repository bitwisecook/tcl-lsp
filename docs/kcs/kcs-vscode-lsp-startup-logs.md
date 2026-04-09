# KCS: Viewing VS Code LSP startup logs

## Description

You want to confirm that the Tcl Language Server started correctly inside
VS Code. When the LSP fails to start you still get the basic TextMate
syntax highlighting that ships with the extension — keywords, strings,
and comments are still coloured — but every feature that needs the
running server is silently missing. Typical symptoms:

- Your `.tcl`, `.tm`, `.itcl`, `.irul`, or `.iapp` file shows only plain
  TextMate colours; variable references, proc names, and commands are
  not given the richer semantic-token colours you see in the extension
  screenshots.
- No red or yellow squiggles appear even on obviously broken code
  (unclosed braces, unknown commands, typos).
- Hovering a command or variable does not show a documentation tooltip.
- Typing does not offer auto-completion for commands, subcommands, or
  iRule events.
- "Go to Definition", "Find References", and rename do nothing or
  report "No definition found".
- The `tcl-lsp v<version>` badge and the dialect indicator are missing
  from the status bar in the bottom right of the window.
- The `Tcl: ...` entries in the Command Palette
  (`Tcl: Restart Language Server`, `Tcl: Optimise Document`,
  `Tcl: Open Compiler Explorer`, etc.) are missing, greyed out, or
  error as soon as you run them.

The extension writes its startup log to an output channel you can open
from inside VS Code, and that log tells you which of the startup steps
failed.

## Audience

User

## Resolution

1. Open the Output panel: **View > Output**, or press `Ctrl+Shift+U`
   (Linux/Windows) or `Cmd+Shift+U` (macOS).
2. In the dropdown on the right-hand side of the Output panel, select
   **Tcl Language Server**.
3. Open any `.tcl`, `.tm`, `.itcl`, `.irul`, or `.iapp` file so the
   extension activates, if it has not already.
4. Look for a successful startup sequence in the channel:
   - If you installed the extension from the Marketplace or a VSIX:
     lines beginning with `Python discovery:`, one line per candidate
     interpreter (e.g. `  /usr/bin/python3.12  3.12.3  (PATH)`),
     followed by `Selected: <path> (<version>)` and
     `[timing] Python discovery: Nms`.
   - If you are running from a git checkout: a single
     `Dev mode: using uv in <serverDir>` line.
   - In both cases, followed by language-client lifecycle messages ending
     in `Server initialized` on success. A repeated
     `Connection to server got closed. Server will restart.` line means
     the server crashed during startup.
5. Check the status bar (bottom right of the window). A healthy
   activation shows `tcl-lsp v<version>` and a dialect indicator. If you
   do not see either, the extension failed to activate before it could
   write to the channel — jump to step 7.
6. To see the LSP protocol traffic as well, open
   **File > Preferences > Settings**, search for `tcl-lsp.trace.server`,
   and set it to `messages` (request/response summaries) or `verbose`
   (full bodies). Then run **Tcl: Restart Language Server** from the
   Command Palette (`Ctrl+Shift+P` / `Cmd+Shift+P`) to replay the
   startup sequence with tracing enabled.
7. If the output channel is empty or does not exist, open
   **Help > Toggle Developer Tools** and look at the **Console** tab,
   or run **Developer: Show Logs... > Extension Host** from the Command
   Palette. Any stack trace there is the reason the extension failed to
   activate.
8. If you need to share the log on an issue, copy the contents of the
   **Tcl Language Server** output channel (and, if relevant, the
   Extension Host log) into the issue body.

## Related content

- [KCS index](README.md)
- [VS Code extension contracts](kcs-vscode-extension-contracts.md)
- [LSP diagnostics publication model](kcs-lsp-diagnostics-publication.md)
