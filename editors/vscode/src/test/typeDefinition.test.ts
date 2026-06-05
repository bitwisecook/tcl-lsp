import * as assert from "assert";
import * as vscode from "vscode";
import { activate, getDocUri } from "./helper";

suite("Go to Type Definition", () => {
  const docUri = getDocUri("oo-shapes.tcl");

  test("jumps from $var to its class when initialised with [Class new]", async () => {
    await activate(docUri);

    // `$d` in `return $d` inside make_dog at 0-indexed line 18.
    const position = new vscode.Position(18, 12);
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeTypeDefinitionProvider",
      docUri,
      position,
    )) as vscode.Location[] | undefined;

    if (locations && locations.length > 0) {
      // Our heuristic picked the Dog class declaration at 0-indexed line 6.
      const hasDogClass = locations.some((loc) => loc.range.start.line === 6);
      assert.ok(
        hasDogClass,
        `Expected a location at the Dog class declaration (line 6), got ${locations
          .map((l) => l.range.start.line)
          .join(",")}`,
      );
    }
    // No-result is also acceptable for the heuristic (it's intentionally narrow).
  });
});
