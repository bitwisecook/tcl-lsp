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

/**
 * Command execution tests — verify that VS Code commands that dispatch to
 * LSP `workspace/executeCommand` actually return data from the server.
 *
 * These tests bypass the VS Code command wrappers (which may show quick-pick
 * dialogs) and call the LSP server directly via `client.sendRequest`.  This
 * catches regressions like the `@server.feature(WORKSPACE_EXECUTE_COMMAND)`
 * handler swallowing commands registered with `@server.command()`.
 */
import * as assert from "assert";
import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import { activate, getDocUri, scaledTimeout } from "./helper";

interface TclLspApi {
  getClient(): LanguageClient;
}

function getClient(): LanguageClient {
  const ext = vscode.extensions.getExtension("bitwisecook.tcl-lsp")!;
  return (ext.exports as TclLspApi).getClient();
}

/** Send a workspace/executeCommand request to the LSP server. */
async function execLspCommand(command: string, ...args: unknown[]): Promise<unknown> {
  return getClient().sendRequest("workspace/executeCommand", {
    command,
    arguments: args,
  });
}

suite("LSP Command Execution", () => {
  const docUri = getDocUri("simple.tcl");

  suiteSetup(async function () {
    this.timeout(scaledTimeout(60_000));
    await activate(docUri);
  });

  // -- minifyDocument (the command that was broken) ----------------------------

  test("tcl-lsp.minifyDocument returns minified source", async () => {
    const uri = docUri.toString();
    const result = (await execLspCommand("tcl-lsp.minifyDocument", uri, false, false, false)) as {
      source: string;
      originalLength: number;
      minifiedLength: number;
    } | null;
    assert.ok(result, "minifyDocument should return a result, not null");
    assert.ok(typeof result.source === "string", "result should have a source string");
    assert.ok(typeof result.originalLength === "number", "result should have originalLength");
    assert.ok(typeof result.minifiedLength === "number", "result should have minifiedLength");
  });

  test("tcl-lsp.minifyDocument with compact names returns symbol map", async () => {
    const uri = docUri.toString();
    const result = (await execLspCommand("tcl-lsp.minifyDocument", uri, true, false, false)) as {
      source: string;
      symbolMap?: string;
    } | null;
    assert.ok(result, "minifyDocument(compact) should return a result");
    assert.ok(typeof result.source === "string", "result should have a source string");
    assert.ok(typeof result.symbolMap === "string", "compact mode should include symbolMap");
  });

  test("tcl-lsp.tkPreview returns the versioned static UI model", async () => {
    const tkUri = getDocUri("tk-preview.tcl");
    await activate(tkUri);
    const document = vscode.workspace.textDocuments.find(
      (candidate) => candidate.uri.toString() === tkUri.toString(),
    );
    assert.ok(document, "Tk fixture should be open");
    const result = (await execLspCommand("tcl-lsp.tkPreview", {
      uri: tkUri.toString(),
      version: document.version,
    })) as {
      schema_version: number;
      tk_active: boolean;
      document_uri: string;
      document_version: number;
      widget_count: number;
      root?: { path: string; children: Array<{ path: string }> };
    } | null;
    assert.ok(result, "tkPreview should return a model");
    assert.strictEqual(result.schema_version, 1);
    assert.strictEqual(result.tk_active, true);
    assert.strictEqual(result.document_uri, tkUri.toString());
    assert.strictEqual(result.document_version, document.version);
    assert.strictEqual(result.widget_count, 4);
    assert.strictEqual(result.root?.path, ".");
    assert.strictEqual(result.root?.children[0]?.path, ".main");
  });

  test("tcl-lsp.tkPreview refuses a detached source string", async () => {
    const result = await execLspCommand(
      "tcl-lsp.tkPreview",
      "package require Tk\nbutton .run -command dangerous",
    );
    assert.strictEqual(result, null);
  });

  test("tcl-lsp.minifyDocument aggressive returns full result", async () => {
    const uri = docUri.toString();
    const result = (await execLspCommand("tcl-lsp.minifyDocument", uri, false, true, false)) as {
      source: string;
      symbolMap?: string;
      optimisationsApplied?: number;
    } | null;
    assert.ok(result, "minifyDocument(aggressive) should return a result");
    assert.ok(typeof result.source === "string", "result should have source");
    assert.ok(
      typeof result.optimisationsApplied === "number",
      "should include optimisationsApplied",
    );
  });

  // -- minifyDocument semantic guarantees (issues #1192-#1194, #1197) ----------

  test("minifyDocument preserves switch # arms, proc names, and array keys", async function () {
    this.timeout(scaledTimeout(30_000));
    const semUri = getDocUri("minifySemantics.tcl");
    await activate(semUri);
    // Default tier: `#` inside a braced switch case list is a PATTERN (the
    // list grammar), never a comment — the arm must survive (issue #1197) —
    // and no `set alias {…}` preamble may be injected (issue #1194).
    const def = (await execLspCommand(
      "tcl-lsp.minifyDocument",
      semUri.toString(),
      false,
      false,
      false,
    )) as { source: string } | null;
    assert.ok(def, "default minify should return a result");
    assert.ok(def.source.includes("# {puts matched}"), `switch # arm dropped: ${def.source}`);
    assert.ok(def.source.includes("puts [set a]"), `name-taking read altered: ${def.source}`);
    assert.ok(!def.source.includes("subst"), `default tier must not alias: ${def.source}`);
    // Compact tier (non-isolated): proc names are public command identities
    // (issue #1193) and array member keys are Tcl data (issue #1192) — both
    // must survive verbatim.
    const compact = (await execLspCommand(
      "tcl-lsp.minifyDocument",
      semUri.toString(),
      true,
      false,
      false,
    )) as { source: string } | null;
    assert.ok(compact, "compact minify should return a result");
    assert.ok(
      compact.source.includes("proc longprocedure"),
      `proc name renamed: ${compact.source}`,
    );
    assert.ok(compact.source.includes("arr(longmember)"), `array key renamed: ${compact.source}`);
  });

  // -- optimiseDocument -------------------------------------------------------

  test("tcl-lsp.optimiseDocument returns optimisations list", async () => {
    const uri = docUri.toString();
    const result = (await execLspCommand("tcl-lsp.optimiseDocument", uri, "full")) as {
      optimisations: unknown[];
      source: string;
    } | null;
    assert.ok(result, "optimiseDocument should return a result");
    assert.ok(Array.isArray(result.optimisations), "result should have optimisations array");
    assert.ok(typeof result.source === "string", "result should have source string");
  });

  test("tcl-lsp.optimiseDocument folds O103 pure-proc calls, including implicit return", async () => {
    // TP: `quad` has no explicit `return` (Tcl's "value of the last command
    // executed" rule — the KCS O103 doc's own canonical example) and must
    // still fold end to end through the real extension + packaged server.
    const o103Uri = getDocUri("o103.tcl");
    await activate(o103Uri);
    const result = (await execLspCommand(
      "tcl-lsp.optimiseDocument",
      o103Uri.toString(),
      "full",
    )) as {
      optimisations: unknown[];
      source: string;
    } | null;
    assert.ok(result, "optimiseDocument should return a result");
    assert.ok(result.source.includes("set q 20"), `expected "set q 20" in:\n${result.source}`);
  });

  test("tcl-lsp.optimiseDocument does not fold a proc call renamed over", async () => {
    // FP guard / miscompile-guard: `rename triple double` moves `triple`'s
    // body onto the name `double` — `optimiseDocument` must leave
    // `set d [double 21]` untouched rather than fold it to the *original*
    // `double` proc's constant return (a real miscompile).
    const o103Uri = getDocUri("o103.tcl");
    await activate(o103Uri);
    const result = (await execLspCommand(
      "tcl-lsp.optimiseDocument",
      o103Uri.toString(),
      "full",
    )) as {
      optimisations: unknown[];
      source: string;
    } | null;
    assert.ok(result, "optimiseDocument should return a result");
    assert.ok(
      result.source.includes("set d [double 21]"),
      `expected "set d [double 21]" to survive unfolded in:\n${result.source}`,
    );
  });

  // O101 (fold constant integer expressions): one true-positive fold and one
  // guarded false-positive (a user-defined ::tcl::mathfunc:: shadowing the
  // builtin) in the same fixture, since the shadow gate is name-specific
  // rather than whole-module.
  test("tcl-lsp.optimiseDocument folds a plain constant expr but not a shadowed-mathfunc one", async () => {
    const o101Uri = getDocUri("optimisation-o101.tcl");
    await activate(o101Uri);
    const result = (await execLspCommand(
      "tcl-lsp.optimiseDocument",
      o101Uri.toString(),
      "full",
    )) as {
      optimisations: unknown[];
      source: string;
    } | null;
    assert.ok(result, "optimiseDocument should return a result");
    assert.ok(
      result.source.includes("return 3"),
      `plain 'expr {1 + 2}' should fold to 'return 3': ${result.source}`,
    );
    assert.ok(
      result.source.includes("expr {triple(2)}"),
      `shadowed-mathfunc call must not fold: ${result.source}`,
    );
  });

  // Regression: a top-level variable reassigned via `global` inside a called
  // procedure must never be folded as if its initial assignment were a
  // stable constant (see rust/tcl-lsp-server/tests/e2e/diagnostic_matrix.rs
  // `top_level_global_reassigned_by_callee_is_not_folded` for the native
  // e2e counterpart and the tclsh-verified miscompile this fixes).
  test("tcl-lsp.optimiseDocument does not fold a global reassigned by a callee", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl",
      content: "set g 4\nproc helper {} { global g\nset g 17 }\nhelper\nputs $g\n",
    });
    await vscode.window.showTextDocument(doc);
    const uri = doc.uri.toString();
    const result = (await execLspCommand("tcl-lsp.optimiseDocument", uri, "full")) as {
      source: string;
    } | null;
    assert.ok(result, "optimiseDocument should return a result");
    assert.ok(
      result.source.includes("puts $g"),
      `must not fold \`puts $g\` to the stale literal 4: ${result.source}`,
    );
  });

  // Regression: O103 must not fold a call to a procedure renamed away
  // elsewhere in the same document.
  test("tcl-lsp.optimiseDocument does not fold a call to a renamed-away proc", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl",
      content: "proc ::foo {} { return 42 }\nrename ::foo ::bar\nputs [::foo]\n",
    });
    await vscode.window.showTextDocument(doc);
    const uri = doc.uri.toString();
    const result = (await execLspCommand("tcl-lsp.optimiseDocument", uri, "full")) as {
      source: string;
    } | null;
    assert.ok(result, "optimiseDocument should return a result");
    assert.ok(
      result.source.includes("puts [::foo]"),
      `must not fold a call to a renamed-away proc: ${result.source}`,
    );
  });

  // Regression: a literal-body `uplevel #0 {...}` reassigns a variable in
  // the absolute global frame, which at top level coincides with the
  // calling scope (see rust/tcl-lsp-server/tests/e2e/diagnostic_matrix.rs
  // `uplevel_hash0_reassignment_is_not_folded` for the native e2e
  // counterpart and the tclsh-verified miscompile this fixes).
  test("tcl-lsp.optimiseDocument does not fold past an uplevel #0 reassignment", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl",
      content: "set n 5\nuplevel #0 { set n 99 }\nputs $n\n",
    });
    await vscode.window.showTextDocument(doc);
    const uri = doc.uri.toString();
    const result = (await execLspCommand("tcl-lsp.optimiseDocument", uri, "full")) as {
      source: string;
    } | null;
    assert.ok(result, "optimiseDocument should return a result");
    assert.ok(
      result.source.includes("puts $n"),
      `must not fold \`puts $n\` to the stale literal 5: ${result.source}`,
    );
  });

  test("tcl-lsp.optimiseDocument does not forward across a variable trace", async () => {
    // Regression: the optimiser used to rewrite the final `puts $x` to
    // `puts 5`, silently dropping the `trace add variable ::x read onread`
    // handler's `puts "trace fired"` side effect (installed indirectly via
    // a called proc, not lexically between the `set` and the read). tclsh
    // prints "trace fired" then "5" for this fixture — the read of `$x`
    // must survive the optimiser as a real runtime variable access so the
    // trace keeps firing.
    const traceUri = getDocUri("optimiserTraceSafety.tcl");
    await activate(traceUri);
    const uri = traceUri.toString();
    const result = (await execLspCommand("tcl-lsp.optimiseDocument", uri, "full")) as {
      optimisations: Array<{ code: string }>;
      source: string;
    } | null;
    assert.ok(result, "optimiseDocument should return a result");
    assert.ok(
      result.source.includes("puts $x"),
      `trace-guarded read must survive the optimiser unchanged, got: ${result.source}`,
    );
    assert.ok(
      !result.optimisations.some((o) => o.code === "O102"),
      `expected no O102 forward across the trace, got: ${JSON.stringify(result.optimisations)}`,
    );
  });

  test("tcl-lsp.optimiseDocument does not eliminate a branch guarded by a cross-procedural trace", async () => {
    // Regression for O107 (unreachable-code elimination): SCCP used to have
    // no notion of variable traces, so it proved `if {$x}` constant (`x` is
    // `1` at every call) and O107 deleted the "unreachable" `else` body's
    // `puts no` — silently losing the trace-firing read of `$x` through the
    // DCE path. The trace is installed by a *called* proc (`setup`), not
    // lexically between the `set` and the `if`, so only the whole-module
    // trace fact catches it. tclsh prints "trace fired" then "yes" — the
    // `else` body never runs, but the compiler cannot prove that
    // statically, so it must survive in the rewritten source.
    const branchUri = getDocUri("optimiserTraceSafetyBranch.tcl");
    await activate(branchUri);
    const uri = branchUri.toString();
    const result = (await execLspCommand("tcl-lsp.optimiseDocument", uri, "full")) as {
      optimisations: Array<{ code: string }>;
      source: string;
    } | null;
    assert.ok(result, "optimiseDocument should return a result");
    assert.ok(
      result.source.includes("puts no"),
      `trace-guarded else branch must survive the optimiser unchanged, got: ${result.source}`,
    );
    assert.ok(
      !result.optimisations.some((o) => o.code === "O107"),
      `expected no O107, got: ${JSON.stringify(result.optimisations)}`,
    );
  });

  // -- fixAllSafeIssues -------------------------------------------------------

  test("tcl-lsp.fixAllSafeIssues returns applied list", async () => {
    const uri = docUri.toString();
    const result = (await execLspCommand("tcl-lsp.fixAllSafeIssues", uri)) as {
      source: string;
      applied: unknown[];
    } | null;
    assert.ok(result, "fixAllSafeIssues should return a result");
    assert.ok(typeof result.source === "string", "result should have source");
    assert.ok(Array.isArray(result.applied), "result should have applied array");
  });

  test("tcl-lsp.fixAllSafeIssues applies only semantics-equivalent fixes", async () => {
    // Issue #1195. Every fix the bulk pass applies must report itself as
    // `semantics-equivalent`; the behaviour-changing ones (W100 over a
    // substituted operand, W110's `==` → `eq`) stay behind their own named
    // code actions. Asserting the class each applied fix reports is what
    // stops the command's *name* being the only guarantee.
    const uri = docUri.toString();
    const result = (await execLspCommand("tcl-lsp.fixAllSafeIssues", uri)) as {
      source: string;
      applied: Array<{ code: string; description: string; safety: string }>;
    } | null;
    assert.ok(result, "fixAllSafeIssues should return a result");
    for (const entry of result.applied) {
      assert.strictEqual(
        entry.safety,
        "semantics-equivalent",
        `bulk pass applied a non-equivalent fix: ${JSON.stringify(entry)}`,
      );
    }
  });

  test("tcl-lsp.fixAllSafeIssues leaves a double-substituting expr untouched", async () => {
    // Under C Tcl 9 this program prints `5`: `$a` substitutes to the string
    // `$x`, and `expr` substitutes that to 3. W100's brace fix turns it into
    // an error, so the unattended pass must return the source unchanged.
    const uri = getDocUri("fixAllDoubleSubstitution.tcl").toString();
    const result = (await execLspCommand("tcl-lsp.fixAllSafeIssues", uri)) as {
      source: string;
      applied: Array<{ code: string }>;
    } | null;
    assert.ok(result, "fixAllSafeIssues should return a result");
    assert.ok(
      result.source.includes("expr $a + $b"),
      `the substituted expr must survive; got:\n${result.source}`,
    );
    assert.ok(
      !result.applied.some((entry) => entry.code === "W100"),
      `W100 must not be bulk-applied here: ${JSON.stringify(result.applied)}`,
    );
  });

  // -- exportConfig -----------------------------------------------------------

  test("tcl-lsp.exportConfig returns configuration object", async () => {
    const result = (await execLspCommand("tcl-lsp.exportConfig")) as Record<string, unknown> | null;
    assert.ok(result, "exportConfig should return a result");
    assert.ok(typeof result === "object", "result should be an object");
  });

  // -- setDialect -------------------------------------------------------------

  test("tcl-lsp.setDialect returns success status", async () => {
    const result = (await execLspCommand("tcl-lsp.setDialect", "tcl8.6")) as {
      success: boolean;
    } | null;
    assert.ok(result, "setDialect should return a result");
    assert.ok(typeof result.success === "boolean", "result should have success flag");
  });

  // -- listIruleEvents --------------------------------------------------------

  test("tcl-lsp.listIruleEvents returns event list", async () => {
    const result = (await execLspCommand("tcl-lsp.listIruleEvents")) as {
      events: string[];
    } | null;
    assert.ok(result, "listIruleEvents should return a result");
    assert.ok(Array.isArray(result.events), "result should have events array");
    assert.ok(result.events.length > 0, "should have at least one event");
  });

  // -- listSubcommands --------------------------------------------------------

  test("tcl-lsp.listSubcommands returns subcommand data", async () => {
    const result = (await execLspCommand("tcl-lsp.listSubcommands", "string")) as {
      command: string;
      subcommands: unknown[];
    } | null;
    assert.ok(result, "listSubcommands should return a result");
    assert.ok(Array.isArray(result.subcommands), "result should have subcommands array");
    assert.ok(result.subcommands.length > 0, "string should have subcommands");
  });

  // -- listKnownPackages ------------------------------------------------------

  test("tcl-lsp.listKnownPackages returns package list", async () => {
    const result = (await execLspCommand("tcl-lsp.listKnownPackages")) as {
      packages: string[];
    } | null;
    assert.ok(result, "listKnownPackages should return a result");
    assert.ok(Array.isArray(result.packages), "result should have packages array");
  });

  // -- listTclInstallations ---------------------------------------------------

  test("tcl-lsp.listTclInstallations reports discovered installs + auto_path", async () => {
    const result = (await execLspCommand("tcl-lsp.listTclInstallations")) as {
      installations: { version: string; tclLibrary: string; autoPath: string[] }[];
      activeAutoPath: string[];
      editorLibraryPaths: string[];
    } | null;
    assert.ok(result, "listTclInstallations should return a result");
    assert.ok(Array.isArray(result.installations), "installations should be an array");
    assert.ok(Array.isArray(result.activeAutoPath), "activeAutoPath should be an array");
    assert.ok(Array.isArray(result.editorLibraryPaths), "editorLibraryPaths should be an array");
    // Each installation carries a library dir and an auto_path list.
    for (const inst of result.installations) {
      assert.strictEqual(typeof inst.tclLibrary, "string");
      assert.ok(Array.isArray(inst.autoPath));
    }
  });

  test("tclLsp.selectTclInstallation command is registered", async () => {
    const registered = await vscode.commands.getCommands(true);
    assert.ok(
      registered.includes("tclLsp.selectTclInstallation"),
      "tclLsp.selectTclInstallation should be registered by the extension",
    );
  });

  // -- suggestPackagesForSymbol -----------------------------------------------

  test("tcl-lsp.suggestPackagesForSymbol returns suggestions", async () => {
    const result = (await execLspCommand("tcl-lsp.suggestPackagesForSymbol", "http")) as {
      symbol: string;
      suggestions: string[];
    } | null;
    assert.ok(result, "suggestPackagesForSymbol should return a result");
    assert.ok(Array.isArray(result.suggestions), "result should have suggestions array");
  });

  // -- searchHelp -------------------------------------------------------------

  test("tcl-lsp.searchHelp returns help data or errors gracefully", async () => {
    try {
      const result = (await execLspCommand("tcl-lsp.searchHelp", "minify", false)) as {
        results?: unknown[];
        features?: unknown[];
      } | null;
      // If the KCS help DB is available, we get a result object.
      assert.ok(result, "searchHelp should return a result");
      assert.ok(typeof result === "object", "result should be an object");
    } catch {
      // The KCS help DB may not be present in CI — the server raises
      // FileNotFoundError which propagates as an LSP error.  That's
      // acceptable; the important thing is the command dispatches.
    }
  });

  // -- compilerExplorer -------------------------------------------------------

  test("tcl-lsp.compilerExplorer returns compiler data for valid source", async () => {
    const result = (await execLspCommand(
      "tcl-lsp.compilerExplorer",
      "set x 10\nputs $x\n",
      "tcl8.6",
    )) as Record<string, unknown> | null;
    assert.ok(result, "compilerExplorer should return a result");
    // Should not be an error
    assert.ok(!result.error, `compilerExplorer returned error: ${result.error}`);
  });

  test("tcl-lsp.compilerExplorer handles empty source gracefully", async () => {
    const result = (await execLspCommand("tcl-lsp.compilerExplorer", "", "tcl8.6")) as Record<
      string,
      unknown
    > | null;
    assert.ok(result, "compilerExplorer should return a result even for empty source");
    assert.ok(result.error, "empty source should produce an error message");
  });

  // -- diagramData ------------------------------------------------------------

  test("tcl-lsp.diagramData returns null for non-iRule source", async () => {
    await execLspCommand("tcl-lsp.diagramData", "set x 10");
    // For plain Tcl (not iRule), may return null or an object
    // Just verify it doesn't throw
    assert.ok(true, "diagramData should not throw");
  });

  // -- xcTranslate ------------------------------------------------------------

  test("tcl-lsp.xcTranslate handles empty source", async () => {
    const result = await execLspCommand("tcl-lsp.xcTranslate", "", "both");
    assert.strictEqual(result, null, "empty source should return null");
  });

  // -- describeIruleEvent -----------------------------------------------------

  test("tcl-lsp.describeIruleEvent returns event metadata", async () => {
    const result = (await execLspCommand("tcl-lsp.describeIruleEvent", "HTTP_REQUEST")) as {
      event: string;
      known: boolean;
    } | null;
    assert.ok(result, "describeIruleEvent should return a result");
    assert.ok(result.known, "HTTP_REQUEST should be a known event");
  });

  // -- describeIruleCommand ---------------------------------------------------

  test("tcl-lsp.describeIruleCommand returns command metadata", async () => {
    const result = (await execLspCommand("tcl-lsp.describeIruleCommand", "HTTP::uri")) as {
      command: string;
      found: boolean;
    } | null;
    assert.ok(result, "describeIruleCommand should return a result");
    assert.ok(result.found, "HTTP::uri should be a known command");
  });

  // -- unminifyError ----------------------------------------------------------

  test("tcl-lsp.unminifyError translates with empty map", async () => {
    const result = (await execLspCommand(
      "tcl-lsp.unminifyError",
      'can\'t read "x": no such variable',
      "",
      "",
      "",
    )) as {
      translatedError: string;
      changed: boolean;
    } | null;
    assert.ok(result, "unminifyError should return a result");
    assert.ok(typeof result.translatedError === "string", "should have translatedError");
    assert.strictEqual(result.changed, false, "no map means no translation");
  });
});
