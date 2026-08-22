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
  beginTestDeadline,
  bounded,
  classifyLiveness,
  DIAGNOSTIC_BUDGET_MS,
  getDocUri,
  latchFromOutcomes,
  loadFactor,
  MAX_LOAD_FACTOR,
  type ProbeOutcome,
  resetServerTransportWedged,
  scaledTimeout,
  serverStateEvidence,
  serverTransportWedged,
  waitForDiagnostics,
  waitForProviderResult,
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

  test("waitForProviderResult rejects loudly when the provider's answer never changes (issue #1295 shape)", async () => {
    // The shape #1295 exists to catch: a feature-toggle "after" sample whose
    // provider keeps answering with its pre-toggle result. Prove the failure
    // is loud (rejects, names what was awaited and what was last seen)
    // rather than silent (resolving with the stale value and letting the
    // caller's assertion pass or fail on its own, or worse, pass vacuously).
    await activate(docUri);
    const base = 500;
    const started = Date.now();
    let probes = 0;
    let error: Error | undefined;
    try {
      const result = await waitForProviderResult<number>(
        docUri,
        () => {
          probes++;
          return 7; // a "depth" that never drops, as if a toggle never took effect
        },
        (depth) => depth < 7,
        { timeout: base, backstopInterval: 20, label: "depth to drop below 7" },
      );
      assert.fail(`waitForProviderResult resolved with ${result} instead of rejecting`);
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
      error.message.includes("depth to drop below 7"),
      `failure must name what was awaited: ${error.message}`,
    );
    assert.ok(
      elapsed < boundCeiling(base),
      `stall must fail at its bound; took ${elapsed}ms for a ${base}ms base`,
    );
    assert.ok(
      probes > 1,
      `the provider must have been re-probed, not sampled once; got ${probes} probe(s)`,
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

  // Attributability of a stalled document (issue #1294).
  //
  // A timeout says the answer never came; on its own it cannot tell a wedged
  // server apart from one document's queue being stuck, which is what left
  // #1294's four consecutive `didOpen`-drain timeouts unexplainable. These pin
  // both halves: that the classification is right for every combination of
  // outcomes, and that the plumbing carries the verdict into the message
  // without ever being able to displace the failure it explains.

  const outcome = (answered: boolean): ProbeOutcome => ({ answered, afterMs: 1 });

  test("the liveness verdict names the most general fault the probes support", () => {
    // Nothing answers: the server, not this document.
    assert.match(
      classifyLiveness({
        transport: outcome(false),
        otherDocument: outcome(false),
        retry: outcome(false),
      }),
      /SERVER WEDGED/,
    );
    // The server answers a document-free request but no document at all.
    assert.match(
      classifyLiveness({
        transport: outcome(true),
        otherDocument: outcome(false),
        retry: outcome(false),
      }),
      /DOCUMENT PIPELINE WEDGED/,
    );
    // Another document answers, this one still does not — #1294's hypothesis,
    // and the distinction the report could not previously make.
    assert.match(
      classifyLiveness({
        transport: outcome(true),
        otherDocument: outcome(true),
        retry: outcome(false),
      }),
      /THIS DOCUMENT'S QUEUE WEDGED/,
    );
    // Everything answers on retry: the request was lost, not the queue.
    assert.match(
      classifyLiveness({
        transport: outcome(true),
        otherDocument: outcome(true),
        retry: outcome(true),
      }),
      /REQUEST DROPPED, NOT WEDGED/,
    );
    // A transport probe that did not answer while a *document* hover did is
    // self-contradictory evidence, not the most general fault: the hover's
    // reply travelled the whole client → server → client path, which is the
    // only thing "SERVER WEDGED" claims is broken. Issue #1600's occurrence
    // read this way and skipped 212 tests on it; a byte-identical re-run then
    // passed 899/899.
    for (const contradicted of [
      { transport: outcome(false), otherDocument: outcome(true), retry: outcome(true) },
      { transport: outcome(false), otherDocument: outcome(true), retry: outcome(false) },
      { transport: outcome(false), otherDocument: outcome(false), retry: outcome(true) },
    ]) {
      const verdict = classifyLiveness(contradicted);
      assert.match(verdict, /DOCUMENT-FREE REQUEST SLOW, TRANSPORT ALIVE/);
      assert.doesNotMatch(
        verdict,
        /SERVER WEDGED/,
        `a reply that crossed the transport contradicts a wedge: ${verdict}`,
      );
    }
  });

  // Issue #1294: once the server answers nothing, every later test can only
  // re-pay its wait budget to learn the same thing. The latch is what lets the
  // runner skip them, so it must arm on exactly the terminal verdict and no
  // other.
  test("only a transport-level wedge latches the skip-the-rest flag", () => {
    resetServerTransportWedged();
    assert.strictEqual(serverTransportWedged(), false, "latch must start clear");

    // Recoverable verdicts must NOT latch: a stuck document queue, or a
    // request that was merely dropped, still leaves a suite that can run.
    //
    // The last three are the #1600 shape — the document-free request did not
    // answer in its short budget, but something else did. One answer anywhere
    // is a reply that crossed the transport, so the run must continue: the
    // latch skips every remaining test, and 212 skipped tests is far too
    // expensive a conclusion to draw from contradicted evidence.
    for (const recoverable of [
      { transport: outcome(true), otherDocument: outcome(false), retry: outcome(false) },
      { transport: outcome(true), otherDocument: outcome(true), retry: outcome(true) },
      { transport: outcome(true), otherDocument: outcome(true), retry: outcome(false) },
      { transport: outcome(false), otherDocument: outcome(true), retry: outcome(true) },
      { transport: outcome(false), otherDocument: outcome(true), retry: outcome(false) },
      { transport: outcome(false), otherDocument: outcome(false), retry: outcome(true) },
    ]) {
      latchFromOutcomes(recoverable);
      assert.strictEqual(
        serverTransportWedged(),
        false,
        `a recoverable verdict must not latch: ${classifyLiveness(recoverable)}`,
      );
    }

    // The terminal verdict — nothing answered at all — arms it.
    latchFromOutcomes({
      transport: outcome(false),
      otherDocument: outcome(false),
      retry: outcome(false),
    });
    assert.strictEqual(serverTransportWedged(), true, "a transport wedge must latch");

    resetServerTransportWedged();
  });

  // Issue #1600: three unanswered probes say "nothing answered"; they do not
  // say whether the server was spinning, parked, or had stopped reading stdin,
  // and a wedge that cannot be told apart from those is only ever re-runnable.
  // This pins the capture that closes that gap.
  test("the liveness diagnostic captures what the server process was doing", async () => {
    // Needs a running server to have a process to read: `activate` is what
    // guarantees one, and every other test in this file that touches the
    // server does the same.
    await activate(getDocUri("simple.tcl"));
    const evidence = await serverStateEvidence();

    assert.match(
      evidence,
      /extension host: a \d+ms timer woke [\d.]+x late/,
      `the host's own event-loop stall must be reported: ${evidence}`,
    );
    // Not merely "some string": the pid comes from a private field on
    // `LanguageClient`, which has no public accessor. If a vscode-languageclient
    // upgrade renames it the capture degrades silently to nothing, which is
    // exactly the rot this assertion exists to catch.
    assert.doesNotMatch(
      evidence,
      /pid unavailable/,
      `the server pid must still be reachable from the language client: ${evidence}`,
    );
    if (process.platform === "linux") {
      assert.match(
        evidence,
        /server process: pid \d+, state \S+, \d+ thread\(s\)/,
        `the process reading must name state and thread count: ${evidence}`,
      );
      assert.match(
        evidence,
        /CPU tick\(s\) and moved \d+ byte\(s\) in \/ \d+ byte\(s\) out/,
        `the CPU and byte deltas are what separate spinning from stopped: ${evidence}`,
      );
      // The byte counters are process-wide (`/proc/<pid>/io` aggregates every
      // descriptor), so the reading must say so rather than let a reader take
      // movement as proof the LSP pipes are draining — a pack-discovery walk
      // moves them too.
      assert.match(
        evidence,
        /aggregate process I\/O, not the LSP pipes alone/,
        `the byte counters must be labelled as process-wide: ${evidence}`,
      );
    } else {
      assert.match(
        evidence,
        /server process: pid \d+, but \/proc is unreadable \(already exited, or not Linux\)/,
        `non-Linux hosts must retain the pid and explain the missing process sample: ${evidence}`,
      );
    }
    assert.match(evidence, /server log:/, `the server's last words must be quoted: ${evidence}`);
  });

  test("a timeout carries its follow-up probe's verdict", async () => {
    let error: Error | undefined;
    try {
      await bounded(new Promise<void>(() => {}), "a stalled document request", {
        timeout: 300,
        diagnose: () => "LIVENESS: the follow-up probe ran",
      });
      assert.fail("bounded resolved for a promise that never settles");
    } catch (err) {
      error = err as Error;
    }
    assert.ok(error, "expected a rejection");
    assert.ok(
      error.message.includes("LIVENESS: the follow-up probe ran"),
      `the verdict must reach the failure message: ${error.message}`,
    );
    assert.ok(
      error.message.includes("a stalled document request"),
      "the diagnostic must not displace what was awaited",
    );
  });

  test("a follow-up probe that throws or hangs cannot displace the timeout", async () => {
    for (const [label, diagnose] of [
      [
        "throws",
        () => {
          throw new Error("probe blew up");
        },
      ],
      ["hangs", () => new Promise<string>(() => {})],
    ] as const) {
      const base = 300;
      const started = Date.now();
      let error: Error | undefined;
      try {
        await bounded(new Promise<void>(() => {}), `a stalled request (${label})`, {
          timeout: base,
          diagnose,
        });
        assert.fail(`bounded resolved for a promise that never settles (${label})`);
      } catch (err) {
        error = err as Error;
      }
      assert.ok(error, `expected a rejection (${label})`);
      assert.ok(
        error.message.includes("VSCODE-WAIT-TIMEOUT") &&
          error.message.includes(`a stalled request (${label})`),
        `the original timeout must survive a ${label} probe: ${error.message}`,
      );
      // The hanging probe is capped by signal.ts's own diagnostic budget, so
      // the failure still arrives — it just arrives later than the bound.
      //
      // That budget is *load-scaled* (`scaledTimeout(DIAGNOSTIC_BUDGET_MS)`),
      // so this ceiling has to scale with it — but it must use the
      // *guaranteed worst case* (`MAX_LOAD_FACTOR`), not a fresh `loadFactor()`
      // reading taken here, after the diagnose has already run. Load
      // fluctuates: a `scaledTimeout(DIAGNOSTIC_BUDGET_MS)` computed at this
      // point can read a *lower* factor than was in effect while the actual
      // diagnose ran (the 1s `LOAD_FACTOR_TTL_MS` cache elapses within a
      // multi-second diagnose), asserting a smaller ceiling than the
      // documented contract actually allows. Observed on CI: a diagnose that
      // took ~32.7s (a real, legitimate ~2.2x scaling) failing against a
      // ceiling computed moments later from a ~1.6x reading — the diagnose
      // itself was within contract, only this ceiling's own measurement was
      // stale. `MAX_LOAD_FACTOR` is the fixed upper bound `diagnose()` itself
      // is capped by (see `signal.ts`), so asserting against it is racy in
      // one direction only: it can under-fail (miss a real regression that
      // stays within the max) but never flake on legitimate load.
      //
      // That worst case is 8 * 15_000 + 2_000 = 122s, and this test's own
      // mocha backstop is 60s at load factor 1 — a ceiling above the bound
      // that would kill the test before it could be checked. It is reachable
      // only because `diagnose()` clamps a probe to the enclosing test's own
      // remaining time (`diagnosticBudget()` in signal.ts): the probe now
      // finishes inside the test by construction, whatever the load factor
      // says at the moment it runs. Without that clamp this assertion was
      // unreachable under load and CI failed here — not on the ceiling, but
      // at mocha's `Timeout of 60000ms exceeded`.
      const ceiling =
        boundCeiling(base) +
        (label === "hangs" ? MAX_LOAD_FACTOR * DIAGNOSTIC_BUDGET_MS + 2_000 : 0);
      assert.ok(
        Date.now() - started < ceiling,
        `a ${label} probe must not make the failure unbounded; took ${Date.now() - started}ms`,
      );
    }
  });

  test("a follow-up probe cannot outlive the test that is waiting for it", async () => {
    // The property the test above can only assert under load, asserted here
    // without needing any: whatever `scaledTimeout(DIAGNOSTIC_BUDGET_MS)` says,
    // a follow-up probe may not run past the deadline of the test awaiting it.
    // Declaring a short deadline makes the clamp observable in a second — with
    // the clamp removed, the hanging probe below runs for the full 15s budget
    // (120s under load) and this fails. The root `beforeEach` puts the real
    // deadline back for every following test.
    const enclosing = 4_000;
    beginTestDeadline(enclosing);
    const started = Date.now();
    let error: Error | undefined;
    try {
      await bounded(new Promise<void>(() => {}), "a stalled request (clamped)", {
        timeout: 200,
        diagnose: () => new Promise<string>(() => {}),
      });
      assert.fail("bounded resolved for a promise that never settles");
    } catch (err) {
      error = err as Error;
    }
    const elapsed = Date.now() - started;
    assert.ok(error, "expected a rejection");
    assert.ok(
      error.message.includes("VSCODE-WAIT-TIMEOUT") &&
        error.message.includes("a stalled request (clamped)"),
      `the original timeout must still survive the probe: ${error.message}`,
    );
    assert.ok(
      elapsed < enclosing,
      `a hanging probe must stay inside the enclosing test's deadline; took ${elapsed}ms ` +
        `against a ${enclosing}ms deadline`,
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
