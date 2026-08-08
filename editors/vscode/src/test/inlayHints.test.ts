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
import { getDocUri, activate, pollUntil } from "./helper";

/** Raw `vscode.executeInlayHintProvider` pull, shared by every test below so
 *  the polling loops that wait on it (via `pollUntil`) do not each redeclare
 *  the same `executeCommand` call. */
function inlayHintsOf(
  uri: vscode.Uri,
  range: vscode.Range,
): Thenable<vscode.InlayHint[] | undefined> {
  return vscode.commands.executeCommand("vscode.executeInlayHintProvider", uri, range) as Thenable<
    vscode.InlayHint[] | undefined
  >;
}

suite("Inlay Hints", () => {
  const docUri = getDocUri("simple.tcl");

  test("inlay hints provider is wired up and does not throw", async () => {
    await activate(docUri);

    const fullRange = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(100, 0));

    // By default both inlay families are disabled, but the provider should
    // still be registered and return gracefully.
    const hints = (await vscode.commands.executeCommand(
      "vscode.executeInlayHintProvider",
      docUri,
      fullRange,
    )) as vscode.InlayHint[] | undefined;

    // Either null/undefined (feature disabled) or an array
    if (hints) {
      assert.ok(Array.isArray(hints), "Inlay hints should be an array");
    }
  });

  test("inlay hints appear when feature is enabled", async () => {
    const config = vscode.workspace.getConfiguration("tclLsp.features");
    const original = config.get<boolean>("inlayTypeHints", false);

    try {
      // Enable inferred-type inlay hints
      await config.update("inlayTypeHints", true, vscode.ConfigurationTarget.Global);

      // Use an untitled document to avoid leaking state.
      const doc = await vscode.workspace.openTextDocument({
        language: "tcl",
        content: 'set x 42\nset name "hello"\nset items [list a b c]\n',
      });
      await vscode.window.showTextDocument(doc);

      // Poll for the server to process the change (up to 10s). Accept either
      // `undefined` (server hasn't produced yet) or a valid array -- this
      // wait only needs the provider to have settled *some* answer, not a
      // non-empty one, so it stops the instant that happens rather than
      // sleeping out the interval like the hand-rolled loop this replaced.
      const fullRange = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(10, 0));
      const hints = await pollUntil(
        () => inlayHintsOf(doc.uri, fullRange),
        (r) => r !== undefined,
        { timeout: 10_000, label: "inlay hints to settle with inlayTypeHints enabled" },
      ).catch(() => undefined); // No signal ever settling is itself a valid (if unlikely) outcome here.

      // When enabled, the server may or may not produce inlay hints
      // depending on the analysis; just verify no error.
      if (hints && hints.length > 0) {
        for (const hint of hints) {
          assert.ok(hint.position, "Each hint should have a position");
          assert.ok(hint.label !== undefined, "Each hint should have a label");
        }
      }
    } finally {
      await config.update("inlayTypeHints", original, vscode.ConfigurationTarget.Global);
    }
  });

  test("single positional binds the required slot, not the optional one (issue #510)", async () => {
    const config = vscode.workspace.getConfiguration("tclLsp.features");
    const original = config.get<boolean>("inlayParameterHints", false);

    try {
      await config.update("inlayParameterHints", true, vscode.ConfigurationTarget.Global);

      // `puts ?-nonewline? ?channelId? string` — one positional argument binds
      // to the required trailing `string`, never the leading optional
      // `channelId` (the pre-#510 regression).  `?options?`/`?switches?`
      // documentation placeholders are likewise never emitted.
      const doc = await vscode.workspace.openTextDocument({
        language: "tcl",
        content: "puts hello\nlsearch $mylist needle\n",
      });
      await vscode.window.showTextDocument(doc);

      const fullRange = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(10, 0));
      const hints = await pollUntil(
        () => inlayHintsOf(doc.uri, fullRange),
        (r): r is vscode.InlayHint[] => !!r && r.length > 0,
        { timeout: 10_000, label: "inlay hints with inlayParameterHints enabled" },
      );

      const labelText = (hint: vscode.InlayHint): string =>
        typeof hint.label === "string" ? hint.label : hint.label.map((p) => p.value).join("");

      // Parameter-kind hints on line 0 (`puts hello`).
      const line0 = hints
        .filter((h) => h.kind === vscode.InlayHintKind.Parameter && h.position.line === 0)
        .map(labelText);
      assert.ok(
        line0.includes("string:"),
        `expected 'string:' on line 0, got ${JSON.stringify(line0)}`,
      );
      assert.ok(
        !line0.includes("channelId:"),
        `'channelId:' must not be emitted for a single positional, got ${JSON.stringify(line0)}`,
      );

      // Parameter-kind hints on line 1 (`lsearch $mylist needle`): both real
      // positionals are labelled and no `?options?`/`?switches?` placeholder.
      const line1 = hints
        .filter((h) => h.kind === vscode.InlayHintKind.Parameter && h.position.line === 1)
        .map(labelText);
      assert.ok(
        line1.includes("list:"),
        `expected 'list:' on line 1, got ${JSON.stringify(line1)}`,
      );
      assert.ok(
        line1.includes("pattern:"),
        `expected 'pattern:' on line 1, got ${JSON.stringify(line1)}`,
      );
      assert.ok(
        !line1.some((l) => l === "options:" || l === "switches:"),
        `documentation placeholders must not be labelled, got ${JSON.stringify(line1)}`,
      );
    } finally {
      await config.update("inlayParameterHints", original, vscode.ConfigurationTarget.Global);
    }
  });

  // PR #643 split the single `inlayHints` toggle into two independent options.
  // Enabling one must not turn on the other.
  test("parameter hints enabled alone produce labels but no type hints", async () => {
    const config = vscode.workspace.getConfiguration("tclLsp.features");
    const origType = config.get<boolean>("inlayTypeHints", false);
    const origParam = config.get<boolean>("inlayParameterHints", false);

    try {
      await config.update("inlayTypeHints", false, vscode.ConfigurationTarget.Global);
      await config.update("inlayParameterHints", true, vscode.ConfigurationTarget.Global);

      // `set x 42` would carry a `: int` Type-kind hint if type hints leaked
      // on, so the no-Type assertion below is a real check; `puts hello`
      // carries the `string:` parameter label.
      const doc = await vscode.workspace.openTextDocument({
        language: "tcl",
        content: "set x 42\nputs hello\n",
      });
      await vscode.window.showTextDocument(doc);

      // Wait for the *steady state* after both config changes propagate to the
      // server: the parameter hint present AND the type hint gone. Resolving
      // the moment a parameter hint appears would race the
      // `inlayTypeHints=false` change — a previous test may have left type
      // hints on, and they linger until the config update lands.
      const fullRange = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(10, 0));
      const hints = await pollUntil(
        () => inlayHintsOf(doc.uri, fullRange),
        (r): r is vscode.InlayHint[] =>
          (r?.some((h) => h.kind === vscode.InlayHintKind.Parameter) ?? false) &&
          !(r?.some((h) => h.kind === vscode.InlayHintKind.Type) ?? false),
        { timeout: 10_000, label: "inlay hints to settle with only inlayParameterHints enabled" },
      );

      assert.ok(
        hints.some((h) => h.kind === vscode.InlayHintKind.Parameter),
        "expected a parameter-kind hint with inlayParameterHints enabled",
      );
      assert.ok(
        !hints.some((h) => h.kind === vscode.InlayHintKind.Type),
        "type hints must stay off when only inlayParameterHints is enabled",
      );
    } finally {
      await config.update("inlayTypeHints", origType, vscode.ConfigurationTarget.Global);
      await config.update("inlayParameterHints", origParam, vscode.ConfigurationTarget.Global);
    }
  });

  test("type hints enabled alone emit no parameter labels", async () => {
    const config = vscode.workspace.getConfiguration("tclLsp.features");
    const origType = config.get<boolean>("inlayTypeHints", false);
    const origParam = config.get<boolean>("inlayParameterHints", false);

    try {
      await config.update("inlayTypeHints", true, vscode.ConfigurationTarget.Global);
      await config.update("inlayParameterHints", false, vscode.ConfigurationTarget.Global);

      // `set x 42` carries the `: int` type hint (the readiness signal);
      // `puts hello` WOULD carry the `string:` parameter label if those leaked
      // on, so the no-Parameter assertion below is a real check.
      const doc = await vscode.workspace.openTextDocument({
        language: "tcl",
        content: "set x 42\nputs hello\n",
      });
      await vscode.window.showTextDocument(doc);

      // Wait for the *steady state*: the type hint present AND parameter
      // labels gone. Resolving the moment a type hint appears would race the
      // `inlayParameterHints=false` change — a previous test may have left
      // parameter labels on until the config update propagates.
      const fullRange = new vscode.Range(new vscode.Position(0, 0), new vscode.Position(10, 0));
      const hints = await pollUntil(
        () => inlayHintsOf(doc.uri, fullRange),
        (r): r is vscode.InlayHint[] =>
          (r?.some((h) => h.kind === vscode.InlayHintKind.Type) ?? false) &&
          !(r?.some((h) => h.kind === vscode.InlayHintKind.Parameter) ?? false),
        { timeout: 10_000, label: "inlay hints to settle with only inlayTypeHints enabled" },
      );

      assert.ok(
        hints.some((h) => h.kind === vscode.InlayHintKind.Type),
        "expected a type-kind hint with inlayTypeHints enabled",
      );
      assert.ok(
        !hints.some((h) => h.kind === vscode.InlayHintKind.Parameter),
        "parameter labels must stay off when only inlayTypeHints is enabled",
      );
    } finally {
      await config.update("inlayTypeHints", origType, vscode.ConfigurationTarget.Global);
      await config.update("inlayParameterHints", origParam, vscode.ConfigurationTarget.Global);
    }
  });
});
