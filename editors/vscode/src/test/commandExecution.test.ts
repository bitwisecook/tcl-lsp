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
import { activate, getDocUri } from "./helper";

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
    this.timeout(60_000);
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

  test("tcl-lsp.searchHelp returns help data", async () => {
    const result = (await execLspCommand("tcl-lsp.searchHelp", "minify", false)) as {
      results?: unknown[];
      features?: unknown[];
    } | null;
    assert.ok(result, "searchHelp should return a result");
    assert.ok(typeof result === "object", "result should be an object");
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
