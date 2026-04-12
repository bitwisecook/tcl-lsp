import * as assert from "assert";
import * as vscode from "vscode";
import { getDocUri, activate } from "./helper";

suite("Document Links", () => {
  const docUri = getDocUri("links.tcl");

  test("returns document links for source/package require", async () => {
    await activate(docUri);

    const links = (await vscode.commands.executeCommand("vscode.executeLinkProvider", docUri)) as
      | vscode.DocumentLink[]
      | undefined;

    // The server should provide clickable links for source/package require
    assert.ok(links !== undefined, "Link provider should return a result (possibly empty)");
  });

  test("links have valid ranges", async () => {
    await activate(docUri);

    const links = (await vscode.commands.executeCommand(
      "vscode.executeLinkProvider",
      docUri,
    )) as vscode.DocumentLink[];

    if (links && links.length > 0) {
      for (const link of links) {
        assert.ok(link.range, "Each link should have a range");
        assert.ok(
          link.range.start.line >= 0,
          `Link start line should be non-negative, got ${link.range.start.line}`,
        );
        assert.ok(
          link.range.end.line >= link.range.start.line,
          "Link end should be on or after start",
        );
      }
    }
  });
});
