# KCS: Troubleshooting VS Code LSP startup logs

## Symptom

A user is unsure whether the Tcl language server started correctly inside
VS Code. Features like diagnostics, hover, or semantic tokens are missing,
slow to appear, or the status bar does not show `tcl-lsp v<version>`, and
there is no obvious error dialog to explain why.

## Operational context

The VS Code extension writes structured startup information to a dedicated
output channel while it locates the server bundle, discovers a suitable
Python interpreter, and launches the language client. The
`vscode-languageclient` library then takes over and records its own
lifecycle and (optionally) protocol trace messages into the same channel.
Both streams are the primary diagnostic surface for activation and startup
problems.

The extension also exposes a trace setting for LSP protocol traffic and a
restart command that replays the startup sequence without reloading the
window.

## Decision rules / contracts

1. All extension-side startup logging goes through the shared
   `Tcl Language Server` output channel returned by `getOutputChannel()`
   in `editors/vscode/src/extension.ts`.
2. The channel must be inspected before filing a startup bug; dialogs only
   fire for fatal conditions (missing Python, missing bundle).
3. Protocol-level traffic is opt-in via `tcl-lsp.trace.server` and must not
   be enabled by default.
4. Pre-activation failures (syntax errors in the extension, unhandled
   promise rejections before `activate()` runs) are not written to the
   channel — those live in the VS Code Extension Host log.

## File-path anchors

- `editors/vscode/src/extension.ts` — `getOutputChannel()`, `resolvePython()`,
  `activate()`, `restartServer()` (channel name `Tcl Language Server`, client
  id `tcl-lsp`).
- `editors/vscode/package.json` — `tcl-lsp.trace.server` setting and the
  `tclLsp.restartServer` command contribution.
- `lsp/__main__.py` — Python entry point; its `logging` calls surface in the
  same output channel via `window/logMessage` once the client is connected.

## Failure modes

- **Empty output channel, no status bar item**: extension activation aborted
  before `getOutputChannel()` ran. Check the Extension Host log for a stack
  trace; usually a bundling or dependency regression.
- **`Tcl LSP: bundled server (tcl-lsp-server.pyz) not found`**: VSIX is
  corrupted or the user set `tclLsp.serverPath` to an invalid directory.
  Reinstall or clear the setting.
- **`Tcl LSP: Python 3.10+ is required but was not found`**: no discovered
  interpreter met the minimum version. The channel shows every candidate
  probed under `Python discovery:`; use it to decide whether to install
  Python, set `tclLsp.pythonPath`, or add the interpreter to `PATH`.
- **`Dev mode: using uv in <dir>` then silence**: `uv` is missing or failed
  to sync the project. Run `uv run --directory <dir> python -m lsp` manually
  to see the underlying error.
- **`Server initialized` appears but features still missing**: activation
  succeeded but a later feature failed. Enable `tcl-lsp.trace.server` and
  re-run the failing action to capture the request/response pair, then file
  an issue with the trace excerpt.
- **Two `Tcl Language Server` entries in the Output dropdown**: the
  extension and the language client each create a channel with the same
  name. This is cosmetic — inspect both; extension-side startup text is in
  the one written first, LSP lifecycle and trace messages are in the one
  owned by `vscode-languageclient`.

## Triage checklist

1. Open the Output panel: **View > Output**, or `Ctrl+Shift+U` on
   Linux/Windows, `Cmd+Shift+U` on macOS.
2. In the dropdown on the right, select **Tcl Language Server**.
3. Open any `.tcl`, `.tm`, `.itcl`, `.irul`, or `.iapp` file to trigger
   activation if it has not already happened.
4. Confirm the channel shows a successful startup sequence. Expected lines
   depend on mode:
   - Dev mode (git checkout with `tclLsp.serverPath` set or a detected repo
     layout): `Dev mode: using uv in <serverDir>`.
   - VSIX mode (bundled `tcl-lsp-server.pyz`): `Python discovery:` followed
     by one line per candidate interpreter (e.g.
     `  /usr/bin/python3.12  3.12.3  (PATH)`), then
     `Selected: <path> (<version>)` and
     `[timing] Python discovery: Nms`.
   - Both modes: `vscode-languageclient` lifecycle messages ending in a
     clean `Server initialized` on success, or
     `Connection to server got closed. Server will restart.` on failure.
5. Check the status bar (bottom right): a healthy activation shows
   `tcl-lsp v<version>` and a dialect indicator. If neither appears, the
   extension failed to activate at all — jump to the Extension Host log
   step below.
6. For protocol-level detail, set `tcl-lsp.trace.server` to `messages`
   (request/response summaries) or `verbose` (full bodies) in
   **File > Preferences > Settings**, then run
   **Tcl: Restart Language Server** from the Command Palette
   (`tclLsp.restartServer`) to replay startup with tracing enabled.
7. If the channel is empty or missing entirely, open
   **Help > Toggle Developer Tools > Console** or run
   **Developer: Show Logs... > Extension Host** from the Command Palette
   to see extension-host-level errors that prevent the channel from ever
   being created.

## Test anchors

- `editors/vscode/src/test/extensionActivation.test.ts` — asserts the
  extension activates and exposes the expected API.
- `editors/vscode/src/test/serverHealth.test.ts` — checks the language
  client reaches a running state after activation.
- `editors/vscode/src/test/configSettings.test.ts` — covers the
  `tcl-lsp.trace.server` default and round-trip behaviour.

## Discoverability

- [KCS index](README.md)
- [VS Code extension contracts](kcs-vscode-extension-contracts.md)
- [LSP diagnostics publication model](kcs-lsp-diagnostics-publication.md)
