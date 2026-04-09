import * as assert from "assert";
import * as vscode from "vscode";
import { activate, getDocUri } from "./helper";

suite("Go to Implementation", () => {
  const docUri = getDocUri("oo-shapes.tcl");

  test("returns subclass overrides for a base-class method", async () => {
    await activate(docUri);

    // `speak` in `method speak {}` inside Animal (line 2 col 11).
    const position = new vscode.Position(2, 11);
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeImplementationProvider",
      docUri,
      position,
    )) as vscode.Location[] | undefined;

    assert.ok(locations, "implementation result should not be null");
    assert.ok(
      locations.length >= 2,
      `Expected at least 2 implementations (Dog + Cat), got ${locations.length}`,
    );
    const lines = locations.map((l) => l.range.start.line);
    // Dog.speak at line 8, Cat.speak at line 13.
    assert.ok(lines.includes(8) || lines.includes(13));
  });

  test("returns subclass declarations for a class name", async () => {
    await activate(docUri);
    // `Animal` in its declaration at line 1 col 18.
    const position = new vscode.Position(1, 18);
    const locations = (await vscode.commands.executeCommand(
      "vscode.executeImplementationProvider",
      docUri,
      position,
    )) as vscode.Location[] | undefined;
    assert.ok(locations);
    assert.ok(locations.length >= 1, "Expected at least one subclass declaration");
  });
});
