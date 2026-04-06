import * as assert from "assert";
import * as vscode from "vscode";

suite("Language Registration", () => {
  const expectedLanguageIds = [
    "tcl",
    "tcl-irule",
    "tcl-iapp",
    "tcl-apl",
    "tcl-bigip",
    "tcl8.4",
    "tcl8.5",
    "tcl9.0",
    "tcl-synopsys",
    "tcl-cadence",
    "tcl-xilinx",
    "tcl-quartus",
    "tcl-mentor",
    "tcl-expect",
  ];

  let registeredLanguages: string[];

  suiteSetup(async () => {
    registeredLanguages = await vscode.languages.getLanguages();
  });

  for (const langId of expectedLanguageIds) {
    test(`language '${langId}' is registered`, () => {
      assert.ok(registeredLanguages.includes(langId), `Language '${langId}' should be registered`);
    });
  }

  test("tcl files are associated with the tcl language", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl",
      content: "set x 1\n",
    });
    assert.strictEqual(doc.languageId, "tcl");
  });

  test("iRule content can be opened with tcl-irule language", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl-irule",
      content: 'when HTTP_REQUEST {\n    log local0. "test"\n}\n',
    });
    assert.strictEqual(doc.languageId, "tcl-irule");
  });

  test("iApp content can be opened with tcl-iapp language", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl-iapp",
      content: "set x 1\n",
    });
    assert.strictEqual(doc.languageId, "tcl-iapp");
  });

  test("BIG-IP config can be opened with tcl-bigip language", async () => {
    const doc = await vscode.workspace.openTextDocument({
      language: "tcl-bigip",
      content: "ltm virtual /Common/test {\n}\n",
    });
    assert.strictEqual(doc.languageId, "tcl-bigip");
  });
});
