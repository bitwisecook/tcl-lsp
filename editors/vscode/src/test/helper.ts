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

async function _ensureServerLogSubscribed(): Promise<void> {
  if (_serverLogSubscribed) return;
  const ext = vscode.extensions.getExtension("bitwisecook.tcl-lsp");
  if (!ext?.isActive) return;
  const client = ext.exports?.getClient?.();
  if (!client || typeof client.onNotification !== "function") return;
  client.onNotification("window/logMessage", (params: { message?: string }) => {
    if (typeof params?.message !== "string") return;
    _serverLog.push(params.message);
    if (_serverLog.length > _SERVER_LOG_MAX) {
      _serverLog.splice(0, _serverLog.length - _SERVER_LOG_MAX);
    }
  });
  _serverLogSubscribed = true;
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
 * Wait until at least one server log line matches *predicate*, or the
 * timeout expires.  Returns the matching line, or ``null`` on timeout.
 */
export async function waitForServerLog(
  predicate: (line: string) => boolean,
  opts?: { timeout?: number },
): Promise<string | null> {
  const timeout = opts?.timeout ?? 5_000;
  const start = Date.now();
  while (Date.now() - start < timeout) {
    const hit = _serverLog.find(predicate);
    if (hit !== undefined) return hit;
    await sleep(50);
  }
  return _serverLog.find(predicate) ?? null;
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
