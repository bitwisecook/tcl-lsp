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
import { activate, getDocUri, pollUntil } from "./helper";

suite("Code Lens", () => {
  const docUri = getDocUri("procs.tcl");

  test("returns resolved lenses for each proc", async () => {
    await activate(docUri);
    // Code lenses are populated asynchronously — poll until the server
    // has published and resolved its first batch rather than racing on a
    // fixed sleep.
    // executeCodeLensProvider's second argument is the number of lenses
    // to resolve; without it VS Code returns unresolved lenses (command=null).
    const lenses = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeCodeLensProvider", docUri, 100),
      (r) => {
        const ls = r as vscode.CodeLens[] | undefined;
        return !!ls && ls.filter((l) => l.command !== undefined).length >= 2;
      },
      { timeout: 10_000, label: "resolved code lenses" },
    )) as vscode.CodeLens[] | undefined;

    assert.ok(lenses, "codeLens result should not be null");
    assert.ok(
      lenses.length >= 2,
      `Expected at least 2 code lenses (fib + factorial), got ${lenses.length}`,
    );
    const resolved = lenses.filter((l) => l.command !== undefined);
    assert.ok(resolved.length >= 2, `Expected at least 2 resolved lenses, got ${resolved.length}`);
    for (const lens of resolved) {
      assert.ok(
        lens.command &&
          typeof lens.command.title === "string" &&
          /\d+\s+reference/i.test(lens.command.title),
        `Expected reference-count title, got "${lens.command?.title}"`,
      );
    }
  });

  // Regression for issue #637 / PR #644: the reference-count title must match
  // the actual references, including a call written before its definition
  // (which resolves to null at analysis time), and a bare call must be
  // attributed only to the same-named proc in its own namespace.
  test("reference count matches resolution for forward and namespaced calls", async () => {
    const refsUri = getDocUri("codeLensRefs.tcl");
    await activate(refsUri);
    // Poll until the lenses for the proc-definition lines under test
    // (1, 5, 8) have all resolved their reference-count titles.
    const lenses = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeCodeLensProvider", refsUri, 100),
      (r) => {
        const ls = r as vscode.CodeLens[] | undefined;
        if (!ls) return false;
        const resolvedLines = new Set(
          ls
            .filter((l) => l.command && typeof l.command.title === "string")
            .map((l) => l.range.start.line),
        );
        return [1, 5, 8].every((line) => resolvedLines.has(line));
      },
      { timeout: 10_000, label: "resolved code lenses for forward/namespaced calls" },
    )) as vscode.CodeLens[] | undefined;
    assert.ok(lenses, "codeLens result should not be null");

    // Map each resolved lens to the line its proc name sits on.
    const titleByLine = new Map<number, string>();
    for (const lens of lenses) {
      if (lens.command && typeof lens.command.title === "string") {
        titleByLine.set(lens.range.start.line, lens.command.title);
      }
    }

    // Line 1: `proc greet637` — called once before its definition (forward
    // reference). The old count path reported "0 references" here.
    assert.strictEqual(
      titleByLine.get(1),
      "1 reference",
      `forward-referenced proc: got "${titleByLine.get(1)}"`,
    );
    // Line 5: `proc dup644` in nsa644 — the bare `dup644` call inside nsa644
    // resolves here.
    assert.strictEqual(
      titleByLine.get(5),
      "1 reference",
      `::nsa644::dup644: got "${titleByLine.get(5)}"`,
    );
    // Line 8: `proc dup644` in nsb644 — must NOT be credited the nsa644 call.
    assert.strictEqual(
      titleByLine.get(8),
      "0 references",
      `::nsb644::dup644 should have no phantom reference: got "${titleByLine.get(8)}"`,
    );
  });

  // Regression for issue #864: the reference-count lens above a TclOO method
  // must count external `$obj method` dispatch, not just intra-class calls.
  // `puts [$b get foo]` (with `set b [Bar new]`) is one reference to `get`.
  test("method lens counts external \\$obj method dispatch", async () => {
    const refsUri = getDocUri("codeLensMethodRefs.tcl");
    await activate(refsUri);
    // Method / member lenses are informational: the server attaches their
    // count eagerly as `command.title` (no separate resolve round-trip), so
    // poll until the lenses on the member lines under test carry a title.
    const lenses = (await pollUntil(
      () => vscode.commands.executeCommand("vscode.executeCodeLensProvider", refsUri, 100),
      (r) => {
        const ls = r as vscode.CodeLens[] | undefined;
        if (!ls) return false;
        const titledLines = new Set(
          ls
            .filter((l) => l.command && typeof l.command.title === "string")
            .map((l) => l.range.start.line),
        );
        // `method get` on line 6, `method unused` on line 10.
        return [6, 10].every((line) => titledLines.has(line));
      },
      { timeout: 10_000, label: "resolved method code lenses" },
    )) as vscode.CodeLens[] | undefined;
    assert.ok(lenses, "codeLens result should not be null");

    const titleByLine = new Map<number, string>();
    for (const lens of lenses) {
      if (lens.command && typeof lens.command.title === "string") {
        titleByLine.set(lens.range.start.line, lens.command.title);
      }
    }

    // Line 6: `method get` — dispatched once via `puts [$b get foo]`.
    assert.strictEqual(
      titleByLine.get(6),
      "1 reference",
      `TclOO method with an external \$obj dispatch: got "${titleByLine.get(6)}"`,
    );
    // Line 10: `method unused` — never dispatched.
    assert.strictEqual(
      titleByLine.get(10),
      "0 references",
      `uncalled TclOO method: got "${titleByLine.get(10)}"`,
    );
  });

  // Regression for issue #724: the reference-count lens must be *clickable* —
  // its resolved command must invoke `tcl-lsp.showReferences` with the URI,
  // anchor position, and reference locations. A bare title with no command is
  // rendered but inert ("reference is not active").
  test("resolved lens invokes the showReferences command with locations", async () => {
    const refsUri = getDocUri("codeLensRefs.tcl");
    await activate(refsUri);
    // Poll the provider (message passing) until the server has published its
    // first batch of lenses, rather than sleeping on a fixed delay.
    const lenses = await pollUntil(
      () =>
        vscode.commands.executeCommand("vscode.executeCodeLensProvider", refsUri, 100) as Thenable<
          vscode.CodeLens[] | undefined
        >,
      (ls) => Array.isArray(ls) && ls.length > 0,
      { label: "codeLens published" },
    );
    assert.ok(lenses, "codeLens result should not be null");

    const withCommand = lenses.filter((l) => l.command && l.command.command);
    assert.ok(
      withCommand.length >= 1,
      `Expected at least one lens with a non-empty command, got ${withCommand.length}`,
    );
    for (const lens of withCommand) {
      assert.strictEqual(
        lens.command?.command,
        "tcl-lsp.showReferences",
        `lens should invoke the showReferences wrapper, got "${lens.command?.command}"`,
      );
      const args = lens.command?.arguments;
      assert.ok(
        Array.isArray(args) && args.length === 3,
        `showReferences needs [uri, position, locations], got ${JSON.stringify(args)}`,
      );
      assert.strictEqual(typeof args[0], "string", "first arg is the URI string");
    }

    // The forward-referenced proc on line 1 has exactly one call site, so its
    // lens must carry exactly one location for the peek.
    const line1 = withCommand.find((l) => l.range.start.line === 1);
    assert.ok(line1, "expected a clickable lens on line 1");
    const locations = line1.command?.arguments?.[2] as unknown[];
    assert.ok(
      Array.isArray(locations) && locations.length === 1,
      `line 1 proc has one reference → one peek location, got ${JSON.stringify(locations)}`,
    );
  });
});
