import * as assert from "assert";
import * as vscode from "vscode";
import { activate, getDocUri } from "./helper";

suite("Go to Declaration", () => {
  const docUri = getDocUri("declarations.tcl");

  test("jumps from $shared_var to its global declaration", async () => {
    await activate(docUri);

    // The fixture (0-indexed):
    //   0: # comment
    //   1: proc with_global {} {
    //   2:     global shared_var
    //   3:     set shared_var 42
    //   4:     return $shared_var
    // Cursor on `$shared_var` at line 4.
    const position = new vscode.Position(4, 12);
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeDeclarationProvider",
      docUri,
      position,
    )) as vscode.Location[] | vscode.LocationLink[] | undefined;

    assert.ok(locations, "declaration result should not be null");
    assert.ok(locations.length >= 1, "Expected at least one declaration");

    const line = (() => {
      const loc = locations[0];
      if ("range" in loc) return loc.range.start.line;
      if ("targetRange" in loc) return loc.targetRange.start.line;
      return -1;
    })();
    assert.strictEqual(
      line,
      2,
      `Expected declaration on 'global shared_var' line 2, got line ${line}`,
    );
  });
});
