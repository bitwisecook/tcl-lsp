# KCS: LSP features are missing in VS Code

> **Audience:** User
> **Type:** Issue

## Applies to

VS Code

## Question

Squiggles, hovers, and completions are missing from my Tcl files in
VS Code — how do I tell whether the Tcl Language Server started, and
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
  Language Server**, **Tcl: Apply All Optimisations**, **Tcl: Open in Tcl
  Compiler Explorer**) are missing, greyed out, or error as soon as you
  run them.
- An error notification saying no native `tcl-lsp-server` binary was found
  for your operating system and processor architecture.

## Answer

1. Open the Output panel: **View > Output**, or press `Ctrl+Shift+U` on
   Linux and Windows, or `Cmd+Shift+U` on macOS.
2. In the dropdown on the right of the Output panel, select **Tcl
   Language Server**.
3. Open any `.tcl`, `.tm`, `.itcl`, `.irul`, or `.iapp` file so the
   extension activates, if it has not already.
4. Look for a healthy startup sequence in the channel. The extension runs
   one native `tcl-lsp-server` binary, so a healthy start is short:
   - `Using native tcl-lsp-server: <path>` names the binary that was
     picked. A packaged install reads it from the extension folder; a git
     checkout reads it from `target/release/` or `target/debug/`.
   - `[timing] client.start: Nms` and `[timing] extension activation: Nms`
     close the sequence.
   - A repeated `Connection to server got closed. Server will restart.`
     line means the server crashed during startup.
   - No `Using native tcl-lsp-server` line at all means no binary was
     found. In a checkout, build one with `cargo build -p tcl-lsp-server`
     (or `make rust-server`). Otherwise point **Tcl LSP: Rust Server
     Path** (`tclLsp.rustServerPath`) at a native binary.
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
