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

import * as assert from "assert";
import * as vscode from "vscode";
import {
  activate,
  awaitSignal,
  bounded,
  getDocUri,
  loadFactor,
  scaledTimeout,
  waitForDiagnostics,
} from "./helper";

// Properties of the wait discipline itself (issue #1274).
//
// The suite's other tests assert what the server does; these assert what
// happens when it *doesn't*. A stalled wait must fail at its bound with a
// message naming what was awaited — never hang to mocha's per-test timeout,
// and never resolve with a partial answer the test then asserts against.
//
// The bad-case tests are what make this a fix rather than a hope: they are the
// deliberate stall, run on every CI run.

/** Ceiling a wait with base deadline `base` may not exceed, whatever the load
 *  factor claims. `signal.ts` caps the factor at 8x; the slack covers the
 *  ~100ms scheduling probes taken on each of the (at most 3) extensions. */
function boundCeiling(base: number): number {
  return base * 8 + 4_000;
}

suite("Wait discipline (issue #1274)", () => {
  const docUri = getDocUri("diagnostics.tcl");

  test("a load-scaled deadline never tightens the deadline it is given", () => {
    // Mirrors the native harness's `load_factor_never_tightens_a_barrier`.
    // A busy machine may only ever *stretch* a hang backstop.
    assert.ok(loadFactor() >= 1, `load factor must be >= 1, got ${loadFactor()}`);
    for (const base of [1, 250, 5_000, 60_000]) {
      assert.ok(
        scaledTimeout(base) >= base,
        `scaledTimeout(${base}) = ${scaledTimeout(base)} must not be below ${base}`,
      );
    }
  });

  test("awaitSignal resolves as soon as the signal fires, not on the next backstop tick", async () => {
    // The good-case property: with a 10s backstop interval, a wait that only
    // polled could not possibly return in under 10s. Waiting on the signal
    // returns as soon as the work does.
    let fire: (() => void) | undefined;
    let ready = false;
    const started = Date.now();
    const waiter = awaitSignal<boolean>({
      label: "a test signal",
      subscribe: (notify) => {
        fire = notify;
        return { dispose: () => {} };
      },
      probe: () => ready,
      predicate: (v) => v,
      timeout: 30_000,
      backstopInterval: 10_000,
    });
    setTimeout(() => {
      ready = true;
      fire?.();
    }, 100);
    assert.strictEqual(await waiter, true);
    const elapsed = Date.now() - started;
    assert.ok(elapsed < 5_000, `signal-driven wait took ${elapsed}ms; a poll would take 10000ms`);
  });

  test("awaitSignal rejects at its bound when the predicate can never hold", async () => {
    const base = 1_000;
    const started = Date.now();
    let error: Error | undefined;
    try {
      await awaitSignal<number>({
        label: "a condition that can never hold",
        subscribe: () => ({ dispose: () => {} }),
        probe: () => 41,
        predicate: (v) => v === 42,
        timeout: base,
        backstopInterval: 50,
        describe: (v) => `probe returned ${v}`,
      });
      assert.fail("awaitSignal resolved for a predicate that can never hold");
    } catch (err) {
      error = err as Error;
    }
    const elapsed = Date.now() - started;
    assert.ok(error, "expected a rejection");
    assert.ok(
      error.message.includes("VSCODE-WAIT-TIMEOUT"),
      `failure must carry the greppable marker: ${error.message}`,
    );
    assert.ok(
      error.message.includes("a condition that can never hold"),
      `failure must name what was awaited: ${error.message}`,
    );
    assert.ok(
      error.message.includes("probe returned 41"),
      `failure must report what was last seen: ${error.message}`,
    );
    assert.ok(
      /PROBE: (CPU STARVATION CONFIRMED|could not confirm starvation)/.test(error.message),
      `failure must carry a machine-load verdict: ${error.message}`,
    );
    assert.ok(
      elapsed < boundCeiling(base),
      `stall must fail at its bound; took ${elapsed}ms for a ${base}ms base`,
    );
  });

  test("waitForDiagnostics rejects on timeout rather than resolving with a partial set", async () => {
    // The defect this replaces: on timeout it used to resolve with
    // `getDiagnostics(uri)`. The fixture below really does publish
    // diagnostics, so a lenient timeout would return a *non-empty*, entirely
    // plausible set — and a negative assertion over it ("no diagnostic of kind
    // ZZZ999") would pass without the awaited analysis ever having happened.
    await activate(docUri);
    const base = 1_000;
    const started = Date.now();
    let error: Error | undefined;
    try {
      const diags = await waitForDiagnostics(docUri, {
        timeout: base,
        predicate: (ds) => ds.some((d) => String(d.code) === "ZZZ999-never-emitted"),
      });
      assert.fail(
        `waitForDiagnostics resolved instead of rejecting, with ${diags.length} diagnostic(s) ` +
          `the predicate never accepted`,
      );
    } catch (err) {
      error = err as Error;
    }
    const elapsed = Date.now() - started;
    assert.ok(error, "expected a rejection");
    assert.ok(
      error.message.includes("VSCODE-WAIT-TIMEOUT"),
      `failure must carry the greppable marker: ${error.message}`,
    );
    assert.ok(
      error.message.includes(docUri.toString()),
      `failure must name the document awaited: ${error.message}`,
    );
    assert.ok(
      elapsed < boundCeiling(base),
      `stall must fail at its bound; took ${elapsed}ms for a ${base}ms base`,
    );
  });

  test("awaitSignal retries a probe error the caller marks recoverable", async () => {
    // VS Code cancels an in-flight provider request when the document or its
    // diagnostics change underneath it — which is exactly when a signal-driven
    // wait probes hardest. A cancellation means "that answer would have been
    // stale, ask again", so it must not fail the test.
    let calls = 0;
    const value = await awaitSignal<string>({
      label: "a provider that cancels before answering",
      subscribe: () => ({ dispose: () => {} }),
      probe: () => {
        calls++;
        if (calls < 3) {
          const err = new Error("Canceled");
          err.name = "Canceled";
          throw err;
        }
        return "answered";
      },
      predicate: (v) => v === "answered",
      timeout: 10_000,
      backstopInterval: 20,
      retryProbeErrors: (err) => err instanceof Error && err.name === "Canceled",
    });
    assert.strictEqual(value, "answered");
    assert.strictEqual(calls, 3, "the cancelled probes must have been retried, not swallowed");
  });

  test("awaitSignal rethrows a probe error the caller does not mark recoverable", async () => {
    await assert.rejects(
      awaitSignal<string>({
        label: "a provider that genuinely faults",
        subscribe: () => ({ dispose: () => {} }),
        probe: () => {
          throw new Error("provider exploded");
        },
        predicate: () => true,
        timeout: 10_000,
        retryProbeErrors: (err) => err instanceof Error && err.name === "Canceled",
      }),
      /provider exploded/,
      "a real provider fault must fail the test immediately, not wait out the deadline",
    );
  });

  test("bounded rejects when the work it wraps never settles", async () => {
    const base = 500;
    const started = Date.now();
    let error: Error | undefined;
    try {
      await bounded(new Promise<void>(() => {}), "a request that never comes back", {
        timeout: base,
      });
      assert.fail("bounded resolved for a promise that never settles");
    } catch (err) {
      error = err as Error;
    }
    const elapsed = Date.now() - started;
    assert.ok(error, "expected a rejection");
    assert.ok(
      error.message.includes("VSCODE-WAIT-TIMEOUT") &&
        error.message.includes("a request that never comes back"),
      `failure must name the bounded await: ${error.message}`,
    );
    assert.ok(
      elapsed < boundCeiling(base),
      `stall must fail at its bound; took ${elapsed}ms for a ${base}ms base`,
    );
  });

  test("bounded lets its work win and adds no delay of its own", async () => {
    const started = Date.now();
    const value = await bounded(Promise.resolve("done"), "an already-settled promise", {
      timeout: 30_000,
    });
    assert.strictEqual(value, "done");
    assert.ok(
      Date.now() - started < 1_000,
      "bounded must not delay a promise that already settled",
    );
  });

  test("a diagnostics wait returns on the publish event without waiting out a backstop", async () => {
    // End-to-end version of the good-case property, against the real server:
    // the document's diagnostics are already published by `activate`, so this
    // must return on the immediate probe rather than on any interval.
    await activate(docUri);
    const started = Date.now();
    const diags = await waitForDiagnostics(docUri, { minCount: 1, timeout: 20_000 });
    assert.ok(diags.length >= 1, "the fixture publishes diagnostics");
    assert.ok(
      Date.now() - started < scaledTimeout(2_000),
      `an already-satisfied wait must return immediately, took ${Date.now() - started}ms`,
    );
    assert.ok(
      vscode.languages.getDiagnostics(docUri).length >= 1,
      "sanity: the diagnostics really are published",
    );
  });
});
