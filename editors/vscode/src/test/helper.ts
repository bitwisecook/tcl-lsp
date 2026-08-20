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

import {
  awaitSignal,
  bounded,
  scaledTimeout,
  WAIT_TIMEOUT_MARKER,
  type TimeoutDiagnostic,
} from "./signal";

export {
  awaitSignal,
  beginTestDeadline,
  bounded,
  DIAGNOSTIC_BUDGET_MS,
  loadFactor,
  MAX_LOAD_FACTOR,
  scaledTimeout,
  sleep,
} from "./signal";

/**
 * Resolve a fixture file name to a URI.
 * e.g. getDocUri("simple.tcl") → file:///…/testFixture/simple.tcl
 */
export function getDocUri(fileName: string): vscode.Uri {
  return vscode.Uri.file(path.resolve(__dirname, "../../testFixture", fileName));
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

// Waiters registered by `waitForServerLog`. The `window/logMessage`
// notification *is* the signal a log-line wait is for, so an arriving line
// wakes every waiter immediately rather than leaving them to notice it on
// their next poll tick.
const _serverLogWaiters = new Set<() => void>();

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
    for (const notify of _serverLogWaiters) notify();
  });
  _serverLogSubscribed = true;
}

/**
 * Resolve on the next ``onDidChangeDiagnostics`` event for *docUri*,
 * returning the diagnostics that publish left in place.
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
 *
 * **Rejects** on timeout.  The whole point of this helper is the barrier —
 * "a publish happened, so what you read next is not the previous test's
 * set".  Resolving with the current diagnostics when the publish never
 * arrived hands the caller exactly the stale set it exists to exclude, and
 * every caller then asserts against it as if the barrier had held.
 */
export function nextDiagnosticsPublish(
  docUri: vscode.Uri,
  opts?: { timeout?: number },
): Promise<vscode.Diagnostic[]> {
  const base = opts?.timeout ?? 5_000;
  const timeout = scaledTimeout(base);
  return new Promise<vscode.Diagnostic[]>((resolve, reject) => {
    const disposable = vscode.languages.onDidChangeDiagnostics((e) => {
      if (e.uris.some((u) => u.toString() === docUri.toString())) {
        disposable.dispose();
        clearTimeout(timer);
        resolve(vscode.languages.getDiagnostics(docUri));
      }
    });
    const timer = setTimeout(() => {
      disposable.dispose();
      reject(
        new Error(
          `${WAIT_TIMEOUT_MARKER}: timed out awaiting the next diagnostics publish for ` +
            `${docUri.toString()}\n` +
            `  waited ${timeout}ms (base ${base}ms, load-scaled)\n` +
            `  no onDidChangeDiagnostics event named this URI, so anything read now is ` +
            `the pre-action set this barrier exists to exclude.\n` +
            `  currently published: ${vscode.languages.getDiagnostics(docUri).length} ` +
            `diagnostic(s)`,
        ),
      );
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
 * *predicate*.  Returns the matching line; **rejects** on timeout.
 *
 * Signal-driven: the ``window/logMessage`` notification the server sends *is*
 * the thing being waited on, so an arriving line wakes the wait immediately.
 * The interval re-scan inside [`awaitSignal`] is only a missed-notification
 * backstop.
 *
 * The default ``since`` is ``0``, which keeps the legacy behaviour of
 * searching the entire captured buffer.  Tests waiting for an event
 * produced by a specific action should snapshot
 * ``getServerLogSize()`` first.
 */
export async function waitForServerLog(
  predicate: (line: string) => boolean,
  opts?: { timeout?: number; since?: number; label?: string },
): Promise<string> {
  const since = opts?.since ?? 0;
  const scan = (): string | null => {
    for (let i = _sinceToIndex(since); i < _serverLog.length; i++) {
      if (predicate(_serverLog[i])) return _serverLog[i];
    }
    return null;
  };
  const hit = await awaitSignal<string | null>({
    label: opts?.label ?? "a matching server log line",
    subscribe: (notify) => {
      _serverLogWaiters.add(notify);
      return { dispose: () => _serverLogWaiters.delete(notify) };
    },
    probe: scan,
    predicate: (line) => line !== null,
    timeout: opts?.timeout ?? 5_000,
    describe: () =>
      `no match among the ${_serverLog.length - _sinceToIndex(since)} line(s) logged ` +
      `since seq ${since} (${_serverLogTotalPushed} logged in total)`,
  });
  // `predicate` above admits only non-null, so this cannot be null.
  return hit as string;
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
  await waitForServerLog(
    (line) => line.includes("[timing] deep diagnostics") && line.includes(`uri=${uri}`),
    {
      since: opts?.since,
      timeout: opts?.timeout ?? 20_000,
      label: `the server's deep-diagnostics marker for ${uri}`,
    },
  );
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
  await waitForServerLog(
    (line) => line.includes("[timing] diagnostics master-off") && line.includes(`uri=${uri}`),
    {
      since: opts?.since,
      timeout: opts?.timeout ?? 10_000,
      label: `the server's master-off diagnostics marker for ${uri}`,
    },
  );
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
  spec_packs: string[];
  spec_packs_loaded: Array<{
    name: string;
    tier: string;
    commands: number;
    files: string[];
  }>;
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
 * Poll ``tcl-lsp.getEffectiveConfig`` until ``predicate`` returns true.
 * Use this instead of ``sleep(N)`` after any ``tclLsp.*`` config change so
 * tests wait on the server's resolved state rather than the debounce timer
 * or the ``workspace/configuration`` round-trip.  Throws on timeout with the
 * last-seen config snapshot in the message.
 *
 * **Deliberately polled** — see [`pollUntil`].  Configuration settling emits
 * no client-visible event: ``onDidChangeConfiguration`` fires when *VS Code*
 * accepts the new value, which is strictly before the server has pulled it
 * via ``workspace/configuration`` and re-resolved the document, so waiting on
 * it would answer a different question than the one asked here.  The server
 * publishes no settle marker for a config pull either, so the only available
 * signal is the resolved config itself.
 */
export async function waitForEffectiveConfig(
  docUri: vscode.Uri,
  predicate: (cfg: EffectiveConfig) => boolean,
  opts?: { timeout?: number; label?: string },
): Promise<EffectiveConfig> {
  return pollUntil(
    () =>
      vscode.commands.executeCommand("tcl-lsp.getEffectiveConfig", docUri.toString()) as Thenable<
        EffectiveConfig | undefined
      >,
    (cfg): cfg is EffectiveConfig => cfg !== undefined && predicate(cfg),
    {
      timeout: opts?.timeout ?? 5_000,
      label: `effective config${opts?.label ? ` (${opts.label})` : ""}`,
    },
  );
}

/**
 * Poll ``fn`` until ``predicate(result)`` returns true.  Returns the first
 * result that satisfies the predicate.  Throws on timeout with the
 * last-seen result and a machine-load verdict in the message.
 *
 * # Use this only where there is genuinely no signal to wait on
 *
 * Polling is the fallback, not the default (see [`awaitSignal`]'s module
 * docs): it is slower than the work in the good case and says nothing useful
 * in the bad one.  It is the right answer for exactly one situation — a
 * ``vscode.commands.executeCommand`` pull whose readiness VS Code announces
 * through no event at all, such as ``tcl-lsp.getEffectiveConfig`` (config
 * settling) or the extension host's document registry.
 *
 * A provider pull whose readiness follows the server *re-analysing a
 * document* — code actions, code lenses, symbols — does have a signal:
 * ``onDidChangeDiagnostics`` is that publish. Use
 * [`waitForProviderResult`] for those, which polls only as a
 * missed-event backstop.
 *
 * The deadline is scaled by measured machine load, so a loaded container
 * cannot manufacture a timeout out of a merely slow round-trip.
 */
export async function pollUntil<T, U extends T>(
  fn: () => Thenable<T> | T,
  predicate: (value: T) => value is U,
  opts?: { timeout?: number; interval?: number; label?: string },
): Promise<U>;
export async function pollUntil<T>(
  fn: () => Thenable<T> | T,
  predicate: (value: T) => boolean,
  opts?: { timeout?: number; interval?: number; label?: string },
): Promise<T>;
export async function pollUntil<T>(
  fn: () => Thenable<T> | T,
  predicate: (value: T) => boolean,
  opts?: { timeout?: number; interval?: number; label?: string },
): Promise<T> {
  return awaitSignal<T>({
    label: opts?.label ?? "a polled condition",
    // No event exists for these callers — that is the whole reason this
    // helper is separate from `awaitSignal`'s event-driven users. The
    // subscription is a no-op and the backstop interval is the mechanism.
    subscribe: () => ({ dispose: () => {} }),
    probe: fn,
    predicate,
    timeout: opts?.timeout ?? 5_000,
    backstopInterval: opts?.interval ?? 50,
  });
}

/**
 * Whether an error is VS Code cancelling an in-flight request.
 *
 * VS Code cancels a pending provider request when the document or its
 * diagnostics change underneath it. For a *wait*, that is not a failure — it
 * means the answer would have been stale anyway, so ask again. Recognised
 * structurally as well as by class, because the rejection crosses the
 * extension-host RPC boundary and arrives as a plain ``Error`` named
 * ``Canceled`` rather than a ``CancellationError`` instance.
 */
function isCancellation(error: unknown): boolean {
  if (error instanceof vscode.CancellationError) return true;
  if (!(error instanceof Error)) return false;
  return (
    error.name === "Canceled" ||
    error.name === "CancellationError" ||
    error.message === "Canceled" ||
    error.message === "Cancelled"
  );
}

/**
 * Wait for a language-feature provider pull (``vscode.executeCodeActionProvider``
 * and friends) to return a result satisfying *predicate*.
 *
 * Provider pulls have no completion event of their own — you ask, you get
 * whatever the server knows *now*.  What they do have is a signal for the
 * thing that usually changes the answer: the server re-analysing the
 * document, which it announces by publishing diagnostics.  So this waits on
 * ``onDidChangeDiagnostics`` for *uri* and re-pulls on every publish.
 *
 * # Why the backstop here is tight, not slow
 *
 * Unlike [`waitForDiagnostics`], where the event *is* the thing being awaited
 * and the interval is a pure missed-event net, a provider's readiness only
 * *correlates* with the publish: the server can finish the work that changes
 * this answer without publishing anything new.  So the interval is load-bearing
 * here, and it stays at the 50ms cadence of the polling this replaced — the
 * conversion must only ever be able to return *sooner* (when the publish wakes
 * it), never later.  Widening it to the 500ms default would make the common
 * case — analysis already done, the provider just needs asking again — ten
 * times slower than the code it replaced.
 */
export async function waitForProviderResult<T>(
  uri: vscode.Uri,
  pull: () => Thenable<T> | T,
  predicate: (value: T) => boolean,
  opts?: {
    timeout?: number;
    label?: string;
    backstopInterval?: number;
    describe?: (value: T | undefined) => string;
  },
): Promise<T> {
  const target = uri.toString();
  return awaitSignal<T>({
    label: opts?.label ?? `a provider result for ${target}`,
    subscribe: (notify) =>
      vscode.languages.onDidChangeDiagnostics((e) => {
        if (e.uris.some((u) => u.toString() === target)) notify();
      }),
    probe: pull,
    predicate,
    timeout: opts?.timeout ?? 10_000,
    backstopInterval: opts?.backstopInterval ?? 50,
    describe: opts?.describe,
    retryProbeErrors: isCancellation,
  });
}

/**
 * [`waitForProviderResult`] specialised to ``vscode.executeCodeActionProvider``:
 * wait until the code actions offered over *range* satisfy *predicate*.
 */
export async function waitForCodeActions(
  uri: vscode.Uri,
  range: vscode.Range,
  predicate: (actions: vscode.CodeAction[]) => boolean,
  opts?: { timeout?: number; label?: string },
): Promise<vscode.CodeAction[]> {
  return waitForProviderResult<vscode.CodeAction[]>(
    uri,
    async () =>
      ((await vscode.commands.executeCommand("vscode.executeCodeActionProvider", uri, range)) as
        vscode.CodeAction[] | undefined) ?? [],
    predicate,
    {
      timeout: opts?.timeout,
      label: opts?.label ?? `code actions at ${range.start.line}:${range.start.character}`,
      describe: (actions) =>
        actions === undefined
          ? "<never pulled>"
          : actions.length === 0
            ? "no code actions offered"
            : `offered: ${actions.map((a) => a.title).join("; ")}`,
    },
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
 * The document's dialect is resolved by the *server*, per document, from the
 * ``didOpen`` the editor sends — the extension only mirrors it into the status
 * bar.  So this waits for the server to have drained that ``didOpen`` (via the
 * hover round trips below) rather than for a dialect notification.
 *
 * # Every step is bounded
 *
 * Each await here is already the right *shape* — it resolves when the work
 * completes — but none of them carried a bound, so a wedged step could only be
 * caught by mocha's 60s per-test timeout, which fires knowing nothing about
 * which step was outstanding.  That is what issue #1274 recorded: two
 * `shimmerPrecision` tests each hit the raw 60s timeout, a number no wait in
 * their body could produce (``waitForDiagnostics`` bounds at 20s and would have
 * failed the assertion long before), so the stall was inside this function and
 * the log said nothing about where.  [`bounded`] wraps each step with a
 * load-scaled deadline that names it.
 */
export async function activate(docUri: vscode.Uri): Promise<vscode.TextDocument> {
  // Ensure the extension is activated first.
  // ext.activate() awaits client.start() which resolves after the LSP
  // initialise/initialised handshake -- the server is ready at that point.
  const ext = vscode.extensions.getExtension("bitwisecook.tcl-lsp");
  if (ext && !ext.isActive) {
    // Cold start spawns the server binary and completes the LSP handshake, so
    // it gets a larger budget than the per-document steps below.
    await bounded(ext.activate(), "the tcl-lsp extension to activate (LSP handshake)", {
      timeout: 60_000,
    });
  }
  await _ensureServerLogSubscribed();

  const doc = await bounded(
    vscode.workspace.openTextDocument(docUri),
    `workspace.openTextDocument(${docUri.toString()})`,
  );
  await bounded(
    vscode.window.showTextDocument(doc),
    `window.showTextDocument(${docUri.toString()})`,
  );

  // Mirror the detected dialect into the status bar, exactly as the
  // extension's onDidChangeActiveTextEditor handler does.  Purely cosmetic
  // now — it sends the server nothing — but tests that assert on the status
  // bar still need it applied deterministically rather than on the editor
  // event.
  if (ext && ext.isActive && typeof ext.exports?.applyDialectForDocument === "function") {
    ext.exports.applyDialectForDocument(doc);
  }

  // The server resolves this document's dialect from the ``didOpen`` and
  // loads the matching command catalog (large for ``f5-irules`` /
  // ``f5-iapps``).  Send a request that will be serialised behind the
  // server's processing queue so we don't return until that ``didOpen`` has
  // been drained.  Two hovers — one to flush, one belt-and-braces —
  // eliminate the residual race we saw on macOS.
  for (const attempt of [1, 2]) {
    await bounded(
      vscode.commands.executeCommand(
        "vscode.executeHoverProvider",
        docUri,
        new vscode.Position(0, 0),
      ),
      `the server to drain didOpen for ${docUri.toString()} ` +
        `(flush hover ${attempt} of 2)\n  a timeout here means the server accepted the ` +
        `document but never answered a request queued behind it`,
      { timeout: 30_000, diagnose: serverLivenessDiagnostic(docUri) },
    );
  }

  return doc;
}

/** Fixture the liveness probe asks about — a document no test drives, so a
 *  hover on it can never be queued behind another test's edits. */
const LIVENESS_PROBE_FIXTURE = "livenessProbe.tcl";

/** Per-question ceiling inside [`serverLivenessDiagnostic`]. Short by design:
 *  the caller has already waited its whole budget, so a question that has not
 *  answered in this long has answered "no". */
const LIVENESS_PROBE_BUDGET_MS = 4_000;

/**
 * Set once a liveness probe has confirmed a **transport-level** wedge: the
 * server did not answer even a request that touches no document.
 *
 * That verdict is terminal, not transient. Every test after it pays its whole
 * wait budget to re-discover the same dead server: issue #1294's second
 * occurrence spent ~27 of its 32 minutes that way, 47 tests × ~34s, after the
 * first failure had already diagnosed the fault. The run learned nothing in
 * that time and reported it half an hour later than it could have.
 *
 * Deliberately *only* the transport verdict. The other three
 * [`classifyLiveness`] outcomes are recoverable — a dropped request, or one
 * document's queue stuck while the rest of the suite runs fine — and skipping
 * on those would hide real, attributable failures.
 */
let transportWedgeConfirmed = false;

/** Whether a probe has confirmed a terminal, server-wide wedge (see above). */
export function serverTransportWedged(): boolean {
  return transportWedgeConfirmed;
}

/** Reset the wedge latch — for tests of this module only. */
export function resetServerTransportWedged(): void {
  transportWedgeConfirmed = false;
}

/**
 * Arm [`serverTransportWedged`] if — and only if — `outcomes` is the terminal
 * transport verdict. Kept separate from the asking, for the same reason
 * [`classifyLiveness`] is: the decision is then a pure function of three
 * booleans, so every branch can be pinned without a server behind it.
 *
 * **Requires corroboration.** The flag skips every remaining test, so arming it
 * on one unanswered request is a claim the evidence has to actually support: an
 * answer to *either* of the other two questions is a reply that travelled the
 * whole client → server → client path, which contradicts "the server answers
 * nothing" outright. Issue #1600's occurrence is exactly that shape — the
 * verdict fired at 687/899, and a byte-identical re-run passed 899/899 — so a
 * single slow document-free request must read as "slow", not as "dead", and
 * the run must be allowed to continue and report what it finds.
 *
 * Latch-only: it never clears the flag, so a later probe that happens to get
 * an answer out of a dying server cannot un-diagnose an earlier wedge.
 */
export function latchFromOutcomes(outcomes: LivenessOutcomes): void {
  if (
    !outcomes.transport.answered &&
    !outcomes.otherDocument.answered &&
    !outcomes.retry.answered
  ) {
    transportWedgeConfirmed = true;
  }
}

/** How one liveness question went. */
export interface ProbeOutcome {
  answered: boolean;
  afterMs: number;
  /** Why it failed, when it failed by throwing rather than by timing out. */
  detail?: string;
}

/** The three independent questions [`serverLivenessDiagnostic`] asks. */
export interface LivenessOutcomes {
  /** A request that touches no document at all. */
  transport: ProbeOutcome;
  /** The same kind of request, on a document no test drives. */
  otherDocument: ProbeOutcome;
  /** The stalled document's own request, tried once more. */
  retry: ProbeOutcome;
}

/**
 * Turn three probe outcomes into the verdict — kept separate from the asking so
 * the classification is a pure function with no server, no timing and no VS
 * Code behind it, and can therefore be pinned by tests for every branch.
 *
 * Ordered most-general fault first: a server that answers nothing subsumes a
 * pipeline that answers nothing, which subsumes one document being stuck.
 */
export function classifyLiveness(outcomes: LivenessOutcomes): string {
  if (!outcomes.transport.answered) {
    if (outcomes.otherDocument.answered || outcomes.retry.answered) {
      return (
        "DOCUMENT-FREE REQUEST SLOW, TRANSPORT ALIVE — the request that touches no document " +
        "did not answer in its budget, but a document hover did, and that reply travelled the " +
        "same client → server → client path. So the transport is not wedged; the document-free " +
        "command was merely slower than a probe budget that is deliberately short."
      );
    }
    return (
      "SERVER WEDGED — the server did not even answer a request that touches no document, " +
      "so the fault is the server (or the connection), not this document."
    );
  }
  if (!outcomes.otherDocument.answered) {
    return (
      "DOCUMENT PIPELINE WEDGED — the server answers a document-free request but no " +
      "document hover, so intake is stuck for every document, not just this one."
    );
  }
  if (outcomes.retry.answered) {
    return (
      "REQUEST DROPPED, NOT WEDGED — a retry of the same request on the same document " +
      "answered, so the queue is draining and the original request was lost or merely " +
      "slower than its budget."
    );
  }
  return (
    "THIS DOCUMENT'S QUEUE WEDGED — another document answers but this one still does " +
    "not, so the stall is specific to this document (issue #1294's hypothesis)."
  );
}

/**
 * Ask one question with a hard, short bound, reporting how it went rather than
 * throwing. Never rejects — this runs on an already-failing path.
 */
async function probeOutcome(
  work: () => Thenable<unknown>,
  budgetMs: number,
): Promise<ProbeOutcome> {
  const started = Date.now();
  try {
    const answered = await Promise.race([
      Promise.resolve(work()).then(() => true),
      new Promise<false>((resolve) => setTimeout(() => resolve(false), budgetMs).unref()),
    ]);
    return { answered, afterMs: Date.now() - started };
  } catch (err) {
    return {
      answered: false,
      afterMs: Date.now() - started,
      detail: err instanceof Error ? err.message : String(err),
    };
  }
}

/**
 * Separate "the server is wedged" from "this document's queue is wedged" after
 * a wait on *stalledUri* has already expired.
 *
 * This is the gap issue #1294 records. Four consecutive waits on one fixture's
 * ``didOpen`` drain expired with the machine measurably healthy, and the
 * evidence could say only that the request never came back — not whether the
 * server was answering *anything*. Three cheap questions separate the cases:
 *
 * 1. ``getEffectiveConfig`` with **no URI at all** — a request that touches
 *    neither the analyser nor any document's work queue, so it answers whenever
 *    the client → server → client path itself is alive. The empty argument is
 *    load-bearing, not tidiness: see the note on the probe below.
 * 2. A hover on a **different** document, which no test drives. Answering means
 *    the document pipeline as a whole is draining, so whatever is stuck is
 *    specific to *this* document.
 * 3. A retry of the same request on the stalled document, which distinguishes a
 *    permanently wedged queue from a request that was simply dropped.
 *
 * Every question is separately bounded and none may throw: the test is already
 * failing, and a diagnostic that hangs turns an attributable failure into an
 * unattributable one.
 */
export function serverLivenessDiagnostic(stalledUri: vscode.Uri): TimeoutDiagnostic {
  return async () => {
    const budget = scaledTimeout(LIVENESS_PROBE_BUDGET_MS);
    const other = getDocUri(LIVENESS_PROBE_FIXTURE);
    // Asked concurrently: the three questions are independent, and running
    // them in series would make the whole diagnostic cost three budgets — which
    // on a loaded machine is long enough for `bounded`'s own cap to truncate
    // the answer, exactly when the answer is most wanted.
    const [transport, otherDoc, retry] = await Promise.all([
      // `""`, not `stalledUri` — the same document-free form `probeServer`
      // (the heartbeat's liveness probe) uses, and the form the extension's own
      // pack sync uses.
      //
      // Passed a document URI, this command is not the transport question the
      // verdict above reports it as. `get_effective_config_command` parses the
      // URI and then, on the `Some` branch only, calls `read_document` — whose
      // first act is `edits_settled()`, the server's *global* `EditOrder`
      // barrier that every request handler waits on — and later takes the
      // `documents` lock and the salsa `db` mutex (which `set_text` holds while
      // it waits out open reads). Every one of those can be held by exactly the
      // condition this diagnostic exists to classify, so the "server answers
      // nothing" branch could be reached by a server that was answering
      // everything else perfectly well. With no URI, none of that code runs:
      // the handler reads a dozen leaf mutexes and returns, so a failure here
      // really is the transport.
      probeOutcome(() => vscode.commands.executeCommand("tcl-lsp.getEffectiveConfig", ""), budget),
      probeOutcome(
        () =>
          vscode.commands.executeCommand(
            "vscode.executeHoverProvider",
            other,
            new vscode.Position(0, 0),
          ),
        budget,
      ),
      probeOutcome(
        () =>
          vscode.commands.executeCommand(
            "vscode.executeHoverProvider",
            stalledUri,
            new vscode.Position(0, 0),
          ),
        budget,
      ),
    ]);

    // Latch the terminal verdict so the rest of the run stops paying full wait
    // budgets to rediscover a server that is already gone (see
    // `transportWedgeConfirmed`).
    latchFromOutcomes({ transport, otherDocument: otherDoc, retry });

    const describe = (name: string, p: ProbeOutcome) =>
      `${name} ${p.answered ? "answered" : "did NOT answer"} after ${p.afterMs}ms` +
      (p.detail ? ` (${p.detail})` : "");

    return (
      `LIVENESS: ${classifyLiveness({ transport, otherDocument: otherDoc, retry })}\n` +
      `    ${describe("document-free getEffectiveConfig", transport)}\n` +
      `    ${describe(`hover on ${LIVENESS_PROBE_FIXTURE} (a different document)`, otherDoc)}\n` +
      `    ${describe("hover retried on the stalled document", retry)}`
    );
  };
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
  const base = opts?.timeout ?? 20_000;
  const timeout = scaledTimeout(base);
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
          `${WAIT_TIMEOUT_MARKER}: timed out awaiting onDidCloseTextDocument on ${uri} ` +
            `after ${timeout}ms (base ${base}ms, load-scaled) — the document is ` +
            `still open despite its editor closing, so the server never received a ` +
            `textDocument/didClose (was it opened with activateViaWorkbench?)`,
        ),
      );
    }, timeout);
  });
}

/**
 * Wait for diagnostics on the given URI to satisfy ``predicate`` (or to reach
 * ``minCount`` entries), driven by ``onDidChangeDiagnostics`` — the server's
 * own publish is the signal — with a 500ms re-read purely as a missed-event
 * backstop.  **Rejects** on a load-scaled timeout.
 *
 * The deep-diagnostic pass (IRULE1005 and friends) is async and fires
 * *after* the initial batch of basic diagnostics, so a test that wants
 * a specific deep code should pass a ``predicate`` rather than a fixed
 * ``minCount`` — ``minCount`` alone returns as soon as enough basic
 * diagnostics arrive, before the deep pass has run.
 *
 * # Why the timeout rejects
 *
 * It used to resolve with ``getDiagnostics(uri)`` — whatever happened to be
 * published when the clock ran out.  A test that timed out therefore carried
 * on and asserted against a set the server had never finished producing.  For
 * a positive assertion that surfaces as a confusing content failure; for a
 * *negative* one ("no diagnostic of kind X on line N") an unanalysed, empty
 * set satisfies it trivially and the test passes vacuously — the suite's
 * strongest tests are precisely the negative ones (issue #1274).
 *
 * Callers that genuinely want "settle, then look" and cannot name what they
 * are waiting for are asking a different question; those are the ones that
 * should pass ``minCount: 0`` explicitly, which is satisfied immediately and
 * so never reaches this timeout at all.
 */
export async function waitForDiagnostics(
  uri: vscode.Uri,
  opts?: {
    timeout?: number;
    minCount?: number;
    predicate?: (diags: vscode.Diagnostic[]) => boolean;
  },
): Promise<vscode.Diagnostic[]> {
  const minCount = opts?.minCount ?? 1;
  const predicate = opts?.predicate;
  const target = uri.toString();

  return awaitSignal<vscode.Diagnostic[]>({
    label: predicate
      ? `diagnostics on ${target} to satisfy the test's predicate`
      : `at least ${minCount} diagnostic(s) on ${target}`,
    subscribe: (notify) =>
      vscode.languages.onDidChangeDiagnostics((e) => {
        if (e.uris.some((u) => u.toString() === target)) notify();
      }),
    probe: () => vscode.languages.getDiagnostics(uri),
    predicate: (diags) => (predicate ? predicate(diags) : diags.length >= minCount),
    timeout: opts?.timeout ?? 20_000,
    describe: (diags) =>
      diags === undefined || diags.length === 0
        ? "no diagnostics published"
        : diags
            .map(
              (d) =>
                `${typeof d.code === "object" && d.code !== null ? d.code.value : d.code}@` +
                `${d.range.start.line}:${d.range.start.character}`,
            )
            .join(", "),
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
