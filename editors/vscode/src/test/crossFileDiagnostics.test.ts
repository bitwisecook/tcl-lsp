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

// Issues #1331 and #1332 — the user-visible outcome in a multi-file workspace.
//
// Both were reported by @nico-robert on #1181 against v2.1.16 / VS Code, and
// both are things a user sees in the Problems panel:
//
//   * #1331 — every call to a proc defined in another file carried a
//     "Unknown command" hint, and a wrong argument count to such a proc was
//     never reported at all. W123 is severity `hint`, which is why this was
//     quiet rather than loud, but on a real project it fires on a large
//     fraction of all call sites.
//   * #1332 — a file that `source`s a file doing `package require Tk` still
//     got `"winfo" requires package require Tk`.
//
// The point of testing this *here* rather than only in the Rust suite is that
// the squiggle is the deliverable: these assert on `vscode.languages
// .getDiagnostics`, i.e. what the Problems panel actually shows.

import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate, pollUntil } from "./helper";

/** The diagnostic codes present on `uri`, as plain strings. */
function codesFor(uri: vscode.Uri): string[] {
  return vscode.languages.getDiagnostics(uri).map((d) => {
    const code = d.code;
    if (typeof code === "string") return code;
    if (typeof code === "number") return String(code);
    return String((code as { value: string | number } | undefined)?.value ?? "");
  });
}

/** The message of the first diagnostic on `uri` carrying `code`. */
function messageFor(uri: vscode.Uri, code: string): string | undefined {
  const codes = codesFor(uri);
  const idx = codes.indexOf(code);
  return idx < 0 ? undefined : vscode.languages.getDiagnostics(uri)[idx].message;
}

/**
 * Open `uri` and poll until it carries `code`.
 *
 * The barrier is a **positive** marker, deliberately. W120, W123 and the
 * cross-file arity check are all computed in the server's deep pass; the
 * progressive fast tier publishes none of them, so an assertion that one of
 * those codes is *absent* is satisfied by a fast-tier publish before the deep
 * pass has looked, and would pass whatever the deep pass later concluded.
 * Waiting for a code that must be present proves the deep pass has landed, and
 * the absence assertions are then read off that same converged state.
 *
 * A log-marker barrier (`waitForDeepDiagnostics`) cannot serve here: these
 * tests open the same documents repeatedly, and re-activating an already-open,
 * unedited document triggers no new analysis and therefore no new marker.
 */
async function openUntil(uri: vscode.Uri, code: string): Promise<void> {
  await activate(uri);
  await pollUntil(
    () => codesFor(uri),
    (codes) => codes.includes(code),
    {
      timeout: 30_000,
      label: `${code} on ${uri.fsPath}`,
    },
  );
}

suite("Cross-file diagnostics in a multi-file workspace (#1331, #1332)", () => {
  const defLib = getDocUri("crossFileDefLib.tcl");
  const caller = getDocUri("crossFileCaller.tcl");
  const sourceTk = getDocUri("crossFileSourceTk.tcl");
  const sourceMain = getDocUri("crossFileSourceMain.tcl");
  const noTk = getDocUri("crossFileNoTk.tcl");

  test("a call to a proc defined in another file is not flagged unknown", async () => {
    await activate(defLib);
    // E002 present is the deep-pass marker; the W123 absence below is read off
    // that same converged state, so it cannot pass against the fast tier.
    await openUntil(caller, "E002");

    const codes = codesFor(caller);
    assert.ok(
      !codes.includes("W123"),
      `the workspace defines \`libtest\`; the Problems panel must not call it ` +
        `an unknown command, got: ${JSON.stringify(codes)}`,
    );
  });

  test("go-to-definition resolves the same call — the control", async () => {
    // The issue's own control: if the fixture were mis-scoped, every
    // cross-file lookup would fail for an unrelated reason and look exactly
    // like the bug. Definition resolving is what rules that out.
    await activate(defLib);
    await openUntil(caller, "E002");

    const locations = (await vscode.commands.executeCommand(
      "vscode.executeDefinitionProvider",
      caller,
      new vscode.Position(7, 2),
    )) as vscode.Location[];

    assert.ok(locations && locations.length > 0, "definition must resolve the cross-file call");
    assert.strictEqual(
      locations[0].uri.fsPath,
      defLib.fsPath,
      "definition must land in the defining document",
    );
  });

  test("a wrong argument count to a cross-file proc is reported", async () => {
    await activate(defLib);
    await openUntil(caller, "E002");

    const codes = codesFor(caller);
    assert.ok(
      codes.includes("E002"),
      `\`libtest 1 2\` supplies two of three required arguments — tclsh 9.0.4: ` +
        `\`wrong # args: should be "libtest a b c"\`; got: ${JSON.stringify(codes)}`,
    );
    const message = messageFor(caller, "E002") ?? "";
    assert.ok(
      message.includes("libtest") && message.includes("expected at least 3"),
      `the arity message must name the proc and its minimum, got: ${message}`,
    );
  });

  test("the arity error is a real Error in the Problems panel", async () => {
    // Severity is what decides whether the user sees a red squiggle or a
    // barely-visible hint. A cross-file arity problem is the same defect as a
    // same-file one and must be classified the same way.
    await activate(defLib);
    await openUntil(caller, "E002");

    const arity = vscode.languages
      .getDiagnostics(caller)
      .find((d) => String((d.code as { value?: unknown })?.value ?? d.code) === "E002");
    assert.ok(arity, "E002 must be present");
    assert.strictEqual(
      arity.severity,
      vscode.DiagnosticSeverity.Error,
      "a wrong argument count is an error, cross-file or not",
    );
  });

  test("a sourced file's `package require Tk` suppresses W120", async () => {
    await activate(sourceTk);
    // The fixture's unknown command is the deep-pass marker — and a true
    // positive in its own right: following `source` must not go quiet.
    await openUntil(sourceMain, "W123");

    const codes = codesFor(sourceMain);
    assert.ok(
      !codes.includes("W120"),
      `Tk is loaded by the time \`winfo\` runs — the \`source\`d file requires ` +
        `it; got: ${JSON.stringify(codes)}`,
    );
    assert.ok(
      codes.includes("W123"),
      `a command nothing defines is still unknown — inheriting a sourced ` +
        `file's facts must not silence everything; got: ${JSON.stringify(codes)}`,
    );
  });

  test("a file with no Tk and no `source` still gets W120 — the TN control", async () => {
    // Proves the fix inherits real facts rather than silencing W120 globally.
    await openUntil(noTk, "W120");

    const codes = codesFor(noTk);
    assert.ok(
      codes.includes("W120"),
      `nothing loads Tk here, so the warning is correct and must stand; ` +
        `got: ${JSON.stringify(codes)}`,
    );
  });
});
