// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import * as vscode from "vscode";
import * as path from "path";

/**
 * Resolve a fixture file name to a URI.
 * e.g. getDocUri("simple.tcl") → file:///…/testFixture/simple.tcl
 */
export function getDocUri(fileName: string): vscode.Uri {
  return vscode.Uri.file(path.resolve(__dirname, "../../testFixture", fileName));
}

/** Promisified setTimeout. */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

// ---------------------------------------------------------------------------
// Server log capture
//
// The LSP server emits ``window/logMessage`` notifications which the
// vscode-languageclient routes into an OutputChannel.  Output channels are
// write-only from a test's perspective, so we snoop the messages directly
// off the client with ``onNotification`` once on first request and let
// tests query the ring buffer when they want to see what the server was
// doing (e.g. dialect resolution diagnostics).
// ---------------------------------------------------------------------------

const _serverLog: string[] = [];
const _SERVER_LOG_MAX = 2000;
let _serverLogSubscribed = false;
// Total lines ever pushed, monotonic and never trimmed -- unlike
// `_serverLog.length`, which pins at `_SERVER_LOG_MAX` once the ring buffer
// fills (every push beyond the cap is immediately followed by a splice that
// drops the same number of old entries off the front). A `since` value
// captured via `getServerLogSize()` after that point equals `_serverLog.length`
// exactly, so `for (i = since; i < _serverLog.length; i++)` never runs even
// as fresh lines keep arriving -- every `since`-anchored wait taken out this
// deep into a long suite run would silently never match anything and burn
// its full timeout every time.
let _serverLogTotalPushed = 0;

async function _ensureServerLogSubscribed(): Promise<void> {
  if (_serverLogSubscribed) return;
  const ext = vscode.extensions.getExtension("bitwisecook.tcl-lsp");
  if (!ext?.isActive) return;
  const client = ext.exports?.getClient?.();
  if (!client || typeof client.onNotification !== "function") return;
  client.onNotification("window/logMessage", (params: { message?: string }) => {
    if (typeof params?.message !== "string") return;
    _serverLog.push(params.message);
    _serverLogTotalPushed++;
    if (_serverLog.length > _SERVER_LOG_MAX) {
      _serverLog.splice(0, _serverLog.length - _SERVER_LOG_MAX);
    }
  });
  _serverLogSubscribed = true;
}

/**
 * Resolve on the next ``onDidChangeDiagnostics`` event for *docUri*,
 * or on timeout.  Returns the current diagnostics for *docUri* either
 * way.
 *
 * Use this when a test needs to observe a single server publish for a
 * URI — typically to assert that an empty publish arrived (no signal
 * shows up via ``[timing]`` logs when the server skips analysis), or
 * to read a *fresh* set of diagnostics after a config change rather
 * than the stale pre-change set.
 *
 * Register the listener **before** the action that triggers the
 * publish (e.g. ``activate(docUri)`` or an edit) so the event is not
 * missed.
 */
export function nextDiagnosticsPublish(
  docUri: vscode.Uri,
  opts?: { timeout?: number },
): Promise<vscode.Diagnostic[]> {
  const timeout = opts?.timeout ?? 5_000;
  return new Promise<vscode.Diagnostic[]>((resolve) => {
    const disposable = vscode.languages.onDidChangeDiagnostics((e) => {
      if (e.uris.some((u) => u.toString() === docUri.toString())) {
        disposable.dispose();
        clearTimeout(timer);
        resolve(vscode.languages.getDiagnostics(docUri));
      }
    });
    const timer = setTimeout(() => {
      disposable.dispose();
      resolve(vscode.languages.getDiagnostics(docUri));
    }, timeout);
  });
}

/**
 * Snapshot of LSP server log lines captured since the extension started.
 * Tests can use ``getServerLog().filter(...)`` to assert on server-side
 * behaviour (dialect switches, diagnostic-pipeline activity, …) without
 * scraping the output channel.
 */
export function getServerLog(): string[] {
  return [..._serverLog];
}

/**
 * Total number of server log lines captured so far (monotonic -- never
 * decreases, even once the ring buffer starts trimming old entries).
 * Capture this **before** triggering an action and pass the value as
 * ``waitForServerLog``'s ``since`` option so the wait only matches lines
 * emitted by the action, not stale lines from earlier tests.
 */
export function getServerLogSize(): number {
  return _serverLogTotalPushed;
}

/**
 * Translate an absolute ``since`` sequence number (from
 * ``getServerLogSize()``) into a start index into the current
 * ``_serverLog`` array, which only holds the most recent
 * ``_SERVER_LOG_MAX`` lines. A ``since`` older than every retained line
 * (already trimmed off the front) clamps to the oldest retained line --
 * everything still in the buffer is at or after that point anyway.
 */
function _sinceToIndex(since: number): number {
  const oldestKept = _serverLogTotalPushed - _serverLog.length;
  return Math.max(0, since - oldestKept);
}

/**
 * Wait until at least one server log line at or after ``opts.since``
 * (an absolute sequence number from ``getServerLogSize()``) matches
 * *predicate*, or the timeout expires.  Returns the matching line, or
 * ``null`` on timeout.
 *
 * The default ``since`` is ``0``, which keeps the legacy behaviour of
 * searching the entire captured buffer.  Tests waiting for an event
 * produced by a specific action should snapshot
 * ``getServerLogSize()`` first.
 */
export async function waitForServerLog(
  predicate: (line: string) => boolean,
  opts?: { timeout?: number; since?: number },
): Promise<string | null> {
  const timeout = opts?.timeout ?? 5_000;
  const since = opts?.since ?? 0;
  const start = Date.now();
  while (Date.now() - start < timeout) {
    for (let i = _sinceToIndex(since); i < _serverLog.length; i++) {
      if (predicate(_serverLog[i])) return _serverLog[i];
    }
    await sleep(50);
  }
  for (let i = _sinceToIndex(since); i < _serverLog.length; i++) {
    if (predicate(_serverLog[i])) return _serverLog[i];
  }
  return null;
}

/**
 * Wait for the LSP server to emit its deep-diagnostics-complete log
 * line for *docUri*.  The server logs ``[timing] deep diagnostics
 * <Nms> (uri=<docUri>, diags=<N>)`` at INFO level after every deep
 * pass, so this is a direct signal that codes like O1xx, IRULE1005,
 * and the shimmer / taint diagnostics have been computed and
 * published.
 *
 * Pass ``opts.since = getServerLogSize()`` captured **before** the
 * triggering action (didOpen, edit, restart) so the wait only matches
 * the run you care about, not a deep pass from an earlier test.
 *
 * Defaults to a 20s deadline (matching ``waitForDiagnostics``) rather
 * than ``waitForServerLog``'s 5s: the deep pass runs *after* the basic
 * pass and is at least as slow, so under parallel ``test-slow`` load it
 * routinely needs more than 5s to land — a tighter default made callers
 * like the ``optimiser.enabled`` toggle test flake.  Callers may still
 * pass an explicit ``timeout`` to override.
 */
export async function waitForDeepDiagnostics(
  docUri: vscode.Uri,
  opts?: { timeout?: number; since?: number },
): Promise<void> {
  const uri = docUri.toString();
  const hit = await waitForServerLog(
    (line) => line.includes("[timing] deep diagnostics") && line.includes(`uri=${uri}`),
    { since: opts?.since, timeout: opts?.timeout ?? 20_000 },
  );
  if (hit === null) {
    throw new Error(
      `Timeout waiting for deep diagnostics on ${uri} ` +
        `(since=${opts?.since ?? 0}, logSize=${_serverLog.length})`,
    );
  }
}

/**
 * Wait for the LSP server to emit its **master-off** diagnostics marker for
 * *docUri* — ``[timing] diagnostics master-off 0ms (uri=<docUri>, diags=0)``,
 * logged by ``run_diagnostics_master_off`` *after* it publishes the empty set
 * when ``features.diagnostics`` is off.
 *
 * This is the reliable signal that the master-switch-off pass actually ran and
 * cleared this URI.  It exists because the alternative — waiting on
 * ``onDidChangeDiagnostics`` — is unreliable on this path: publishing an empty
 * set onto an already-empty collection often fires no change event, so a test
 * that waited on the publish event would fall through to its timeout and
 * (with ``nextDiagnosticsPublish``) resolve with the current (empty)
 * diagnostics — passing *vacuously* whether or not the server ran at all.
 * Keying on the server's own marker, and throwing on timeout, makes the wait a
 * true signal with the timeout only as a failure backstop.
 *
 * Pass ``opts.since = getServerLogSize()`` captured **before** the triggering
 * action so the wait matches only the run you care about.
 */
export async function waitForMasterOffDiagnostics(
  docUri: vscode.Uri,
  opts?: { timeout?: number; since?: number },
): Promise<void> {
  const uri = docUri.toString();
  const hit = await waitForServerLog(
    (line) => line.includes("[timing] diagnostics master-off") && line.includes(`uri=${uri}`),
    { since: opts?.since, timeout: opts?.timeout ?? 10_000 },
  );
  if (hit === null) {
    throw new Error(
      `Timeout waiting for master-off diagnostics on ${uri} ` +
        `(since=${opts?.since ?? 0}, logSize=${_serverLog.length})`,
    );
  }
}

/**
 * Shape of the value returned by ``tcl-lsp.getEffectiveConfig``.
 *
 * Mirrors ``on_get_effective_config`` in ``lsp/commands.py``; covers the
 * fields tests poll on.  Any field the server adds in future is
 * harmlessly ignored by predicates that do not name it.
 */
export interface EffectiveConfig {
  uri: string;
  folder_uri: string | null;
  dialect: string;
  extra_commands: string[];
  non_ascii_mode: string | null;
  library_paths: string[];
  line_length: number;
  dialect_explicitly_set: boolean;
  features: Record<string, boolean>;
  optimiser_enabled: boolean;
  shimmer_enabled: boolean;
  xc_diagnostics_enabled: boolean;
  disabled_diagnostics: string[];
  disabled_optimisations: string[];
  known_folder_uris: string[];
}

/**
 * Poll ``tcl-lsp.getEffectiveConfig`` until ``predicate`` returns true,
 * or until ``opts.timeout`` elapses.  Use this instead of ``sleep(N)``
 * after any ``tclLsp.*`` config change so tests wait on the server's
 * resolved state rather than the debounce timer or the
 * ``workspace/configuration`` round-trip.  Throws on timeout with the
 * last-seen config snapshot in the message.
 */
export async function waitForEffectiveConfig(
  docUri: vscode.Uri,
  predicate: (cfg: EffectiveConfig) => boolean,
  opts?: { timeout?: number; label?: string },
): Promise<EffectiveConfig> {
  const timeout = opts?.timeout ?? 5_000;
  const deadline = Date.now() + timeout;
  let last: EffectiveConfig | undefined;
  while (Date.now() < deadline) {
    last = (await vscode.commands.executeCommand(
      "tcl-lsp.getEffectiveConfig",
      docUri.toString(),
    )) as EffectiveConfig | undefined;
    if (last && predicate(last)) return last;
    await sleep(50);
  }
  throw new Error(
    `Timeout waiting for effective config${opts?.label ? ` (${opts.label})` : ""} ` +
      `(last seen: ${JSON.stringify(last)})`,
  );
}

/**
 * Poll ``fn`` every 50ms until ``predicate(result)`` returns true, or
 * until ``opts.timeout`` elapses.  Returns the first result that
 * satisfies the predicate.  Throws on timeout with the last-seen
 * result included in the message.
 *
 * Useful for waiting on a VS Code command's response shape without
 * sleeping on wall-clock time — for example, polling
 * ``vscode.executeCodeLensProvider`` until the language server has
 * published its first batch of lenses.
 */
export async function pollUntil<T>(
  fn: () => Thenable<T> | T,
  predicate: (value: T) => boolean,
  opts?: { timeout?: number; interval?: number; label?: string },
): Promise<T> {
  const timeout = opts?.timeout ?? 5_000;
  const interval = opts?.interval ?? 50;
  const deadline = Date.now() + timeout;
  let last: T | undefined;
  while (Date.now() < deadline) {
    last = await fn();
    if (predicate(last)) return last;
    await sleep(interval);
  }
  throw new Error(
    `Timeout polling${opts?.label ? ` (${opts.label})` : ""} ` +
      `(last seen: ${JSON.stringify(last)})`,
  );
}

/**
 * Convenience wrapper around ``waitForEffectiveConfig`` for the common
 * case of waiting on a single ``tclLsp.features.X`` toggle.
 */
export async function waitForFeatureToggle(
  docUri: vscode.Uri,
  key: string,
  expected: boolean,
  opts?: { timeout?: number },
): Promise<void> {
  await waitForEffectiveConfig(docUri, (cfg) => cfg.features?.[key] === expected, {
    timeout: opts?.timeout,
    label: `tclLsp.features.${key} = ${expected}`,
  });
}

/**
 * Open a document and wait for the language server to finish its initial
 * analysis. Returns the opened TextDocument.
 *
 * The auto-detected dialect is applied *and awaited* before this returns,
 * so tests that depend on dialect-specific completions or diagnostics
 * don't race the fire-and-forget ``onDidChangeActiveTextEditor`` path
 * the extension uses in normal interactive use.
 */
export async function activate(docUri: vscode.Uri): Promise<vscode.TextDocument> {
  // Ensure the extension is activated first.
  // ext.activate() awaits client.start() which resolves after the LSP
  // initialise/initialised handshake -- the server is ready at that point.
  const ext = vscode.extensions.getExtension("bitwisecook.tcl-lsp");
  if (ext && !ext.isActive) {
    await ext.activate();
  }
  await _ensureServerLogSubscribed();

  const doc = await vscode.workspace.openTextDocument(docUri);
  await vscode.window.showTextDocument(doc);

  // Synchronously apply the auto-detected dialect.  In normal interactive
  // use the extension's onDidChangeActiveTextEditor handler kicks off
  // ``applyDialectForDocument`` as fire-and-forget, which races test
  // assertions that follow immediately.  Tests need to know the server
  // has the right dialect's command catalog loaded before they query
  // completions/diagnostics.
  if (ext && ext.isActive && typeof ext.exports?.applyDialectForDocument === "function") {
    await ext.exports.applyDialectForDocument(doc);
  }

  // After a dialect change the server reloads its command catalog (large
  // for ``f5-irules`` / ``f5-iapps``).  Send a request that will be
  // serialised behind the server's processing queue so we don't return
  // until both the ``didOpen`` notification AND any dialect-change
  // notification have been drained.  Two hovers — one to flush, one
  // belt-and-braces — eliminates the residual race we saw on macOS where
  // a single hover sometimes raced the config-change apply.
  await vscode.commands.executeCommand(
    "vscode.executeHoverProvider",
    docUri,
    new vscode.Position(0, 0),
  );
  await vscode.commands.executeCommand(
    "vscode.executeHoverProvider",
    docUri,
    new vscode.Position(0, 0),
  );

  return doc;
}

/**
 * Open *docUri* through the **workbench** (the ``vscode.open`` command) rather
 * than ``workspace.openTextDocument``, then hand off to [`activate`].
 *
 * Use this — and only this — when a test needs the document to actually *close*
 * later.  ``workspace.openTextDocument`` (which ``activate`` calls, and which
 * ``window.showTextDocument(uri)`` calls internally) makes VS Code's main
 * thread take a model reference on the extension host's behalf and park it in a
 * ``BoundModelReferenceCollection``.  That collection releases a reference only
 * after **three minutes**, or once 60 of them have piled up (it then drops the
 * oldest ten), or on a delete/rename of the file — *never* when the editor tab
 * closes.  While the reference is held the underlying text model cannot be
 * disposed, so ``workspace.onDidCloseTextDocument`` does not fire, so
 * vscode-languageclient never sends ``textDocument/didClose``: a test that
 * closes the tab and waits for the server to react waits for something that
 * (within any usable budget) never happens.
 *
 * Opening through the workbench takes no such reference — the editor is the
 * model's only holder, exactly as when a user clicks the file in the Explorer.
 * The subsequent ``openTextDocument`` inside ``activate`` then resolves against
 * the already-registered document instead of asking the main thread to open
 * (and pin) it, so the document is still unpinned when the tab closes.
 */
export async function activateViaWorkbench(docUri: vscode.Uri): Promise<vscode.TextDocument> {
  await vscode.commands.executeCommand("vscode.open", docUri);
  // The main thread registers the new model with the extension host over a
  // separate channel from the command's own reply, so the document is not
  // guaranteed to be visible here the instant `vscode.open` resolves.  Wait
  // until it is: `activate`'s `openTextDocument` must find it already present,
  // or it will fall back to the pinning path this function exists to avoid.
  await pollUntil(
    () => vscode.workspace.textDocuments.some((d) => d.uri.toString() === docUri.toString()),
    (registered) => registered,
    { label: `${docUri.toString()} registered with the extension host` },
  );
  return activate(docUri);
}

/**
 * Resolve once VS Code closes the *document* for *docUri* — the event that
 * drives vscode-languageclient's ``textDocument/didClose``.
 *
 * Closing an editor tab is **not** the same thing: the document survives its
 * last editor whenever something still holds a reference to the underlying
 * model (see [`activateViaWorkbench`]).  A test that closes tabs and expects
 * the server to see a close must wait on this, not on the close command
 * resolving — and must have opened the document via [`activateViaWorkbench`],
 * or the wait will (correctly) time out.
 *
 * Register **before** issuing the close so the event cannot be missed.
 */
export function documentClosed(docUri: vscode.Uri, opts?: { timeout?: number }): Promise<void> {
  const timeout = opts?.timeout ?? 20_000;
  const uri = docUri.toString();
  return new Promise<void>((resolve, reject) => {
    const disposable = vscode.workspace.onDidCloseTextDocument((doc) => {
      if (doc.uri.toString() !== uri) return;
      disposable.dispose();
      clearTimeout(timer);
      resolve();
    });
    const timer = setTimeout(() => {
      disposable.dispose();
      reject(
        new Error(
          `Timeout waiting for onDidCloseTextDocument on ${uri} — the document is ` +
            `still open despite its editor closing, so the server never received a ` +
            `textDocument/didClose (was it opened with activateViaWorkbench?)`,
        ),
      );
    }, timeout);
  });
}

/**
 * Poll for diagnostics on the given URI until ``minCount`` are available
 * or ``predicate`` returns truthy, whichever comes first.  Combines event
 * listening with periodic polling for robustness.
 *
 * The deep-diagnostic pass (IRULE1005 and friends) is async and fires
 * *after* the initial batch of basic diagnostics, so a test that wants
 * a specific deep code should pass a ``predicate`` rather than a fixed
 * ``minCount`` — ``minCount`` alone returns as soon as enough basic
 * diagnostics arrive, before the deep pass has run.
 */
export async function waitForDiagnostics(
  uri: vscode.Uri,
  opts?: {
    timeout?: number;
    minCount?: number;
    predicate?: (diags: vscode.Diagnostic[]) => boolean;
  },
): Promise<vscode.Diagnostic[]> {
  const timeout = opts?.timeout ?? 20_000;
  const minCount = opts?.minCount ?? 1;
  const predicate = opts?.predicate;

  const isReady = (diags: vscode.Diagnostic[]): boolean => {
    if (predicate) {
      return predicate(diags);
    }
    return diags.length >= minCount;
  };

  // Check immediately
  const immediate = vscode.languages.getDiagnostics(uri);
  if (isReady(immediate)) {
    return immediate;
  }

  return new Promise<vscode.Diagnostic[]>((resolve) => {
    let resolved = false;

    const finish = (diags: vscode.Diagnostic[]) => {
      if (resolved) return;
      resolved = true;
      clearTimeout(timer);
      clearInterval(poller);
      disposable.dispose();
      resolve(diags);
    };

    // Timeout -- return whatever we have
    const timer = setTimeout(() => {
      finish(vscode.languages.getDiagnostics(uri));
    }, timeout);

    // Event-driven: listen for diagnostic changes
    const disposable = vscode.languages.onDidChangeDiagnostics((e) => {
      const changed = e.uris.some((u) => u.toString() === uri.toString());
      if (changed) {
        const diags = vscode.languages.getDiagnostics(uri);
        if (isReady(diags)) {
          finish(diags);
        }
      }
    });

    // Polling fallback: check every 500ms in case we missed an event
    const poller = setInterval(() => {
      const diags = vscode.languages.getDiagnostics(uri);
      if (isReady(diags)) {
        finish(diags);
      }
    }, 500);
  });
}

/**
 * Replace the entire document content in the given editor.
 */
export async function setTestContent(editor: vscode.TextEditor, content: string): Promise<boolean> {
  const doc = editor.document;
  const fullRange = new vscode.Range(doc.positionAt(0), doc.positionAt(doc.getText().length));
  return editor.edit((editBuilder) => {
    editBuilder.replace(fullRange, content);
  });
}
