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

//! The heartbeat's server-liveness probe, shared by both mocha entry points.
//!
//! Split out of `index.ts` / `multiFolder/index.ts` (which each carried a copy)
//! rather than folded into `runnerWatchdog.ts`, which must stay free of
//! `import "vscode"` so its pure decision function is testable outside the
//! extension host. This is the piece that genuinely needs the host, so it is
//! injected into the watchdog's heartbeat writer rather than imported by it.

import * as vscode from "vscode";
import { scaledTimeout } from "./signal";
import { Heartbeat, SERVER_PROBE_TIMEOUT_MS } from "./runnerWatchdog";

/**
 * Ask the language server a question that touches neither the document store
 * nor the analyser, so what it times is the client → server → client path
 * itself. Mirrors the native harness's `LatencyBudget` no-op probe.
 *
 * Bounded and non-throwing: the heartbeat must never become the thing that
 * hangs, and a probe failure is an *answer* ("not responding"), not an error.
 */
export async function probeServer(): Promise<NonNullable<Heartbeat["server"]>> {
  const started = Date.now();
  try {
    const answered = await Promise.race([
      vscode.commands.executeCommand("tcl-lsp.getEffectiveConfig", "").then(() => true),
      new Promise<false>((resolve) =>
        setTimeout(() => resolve(false), scaledTimeout(SERVER_PROBE_TIMEOUT_MS)).unref(),
      ),
    ]);
    return answered
      ? { responsive: true, roundTripMs: Date.now() - started }
      : {
          responsive: false,
          reason: "getEffectiveConfig did not answer",
          afterMs: Date.now() - started,
        };
  } catch (err) {
    return {
      responsive: false,
      reason: err instanceof Error ? err.message : String(err),
      afterMs: Date.now() - started,
    };
  }
}
