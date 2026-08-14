// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

import * as assert from "assert";
import * as path from "path";
import * as vscode from "vscode";

import { activate, pollUntil, waitForEffectiveConfig } from "./helper";

interface DecodedToken {
  line: number;
  char: number;
  length: number;
  type: string;
}

function decodeTokens(
  tokens: vscode.SemanticTokens,
  legend: vscode.SemanticTokensLegend,
): DecodedToken[] {
  const decoded: DecodedToken[] = [];
  let line = 0;
  let char = 0;
  for (let i = 0; i < tokens.data.length; i += 5) {
    const [deltaLine, deltaChar, length, typeIndex] = tokens.data.slice(i, i + 5);
    if (deltaLine) {
      line += deltaLine;
      char = deltaChar;
    } else {
      char += deltaChar;
    }
    decoded.push({ line, char, length, type: legend.tokenTypes[typeIndex] });
  }
  return decoded;
}

function hoverText(hovers: vscode.Hover[] | undefined): string {
  return (hovers ?? [])
    .flatMap((hover) => hover.contents)
    .map((content) => (typeof content === "string" ? content : content.value))
    .join("\n");
}

suite("SpecTcl project packs", () => {
  test("a .tclspec changes hover, signature help, and semantic analysis", async () => {
    const project = path.resolve(__dirname, "../../../../tests/fixtures/spec-packs/tiny-project");
    const pack = path.join(project, ".tcl-lsp", "tcl_lsp_fixture.tclspec");
    const docUri = vscode.Uri.file(path.join(project, "consumer.tcl"));
    const config = vscode.workspace.getConfiguration("tclLsp", docUri);
    const original = config.inspect<string[]>("specPacks")?.workspaceValue;

    try {
      await config.update("specPacks", [], vscode.ConfigurationTarget.Workspace);
      const doc = await activate(docUri);
      await waitForEffectiveConfig(
        docUri,
        (effective) =>
          effective.spec_packs.length === 0 &&
          !effective.spec_packs_loaded.some((loaded) => loaded.name === "tcl_lsp_fixture"),
        { timeout: 20_000, label: "synthetic pack absent" },
      );

      const commandPosition = new vscode.Position(3, 2);
      const argumentPosition = new vscode.Position(3, 28);
      const legend = (await vscode.commands.executeCommand(
        "vscode.provideDocumentSemanticTokensLegend",
        docUri,
      )) as vscode.SemanticTokensLegend;

      const beforeHover = (await vscode.commands.executeCommand(
        "vscode.executeHoverProvider",
        docUri,
        commandPosition,
      )) as vscode.Hover[] | undefined;
      const beforeSignature = (await vscode.commands.executeCommand(
        "vscode.executeSignatureHelpProvider",
        docUri,
        argumentPosition,
      )) as vscode.SignatureHelp | undefined;
      const beforeTokens = decodeTokens(
        (await vscode.commands.executeCommand(
          "vscode.provideDocumentSemanticTokens",
          docUri,
        )) as vscode.SemanticTokens,
        legend,
      );

      assert.ok(
        !hoverText(beforeHover).includes("collecting a result"),
        "the pack-authored hover must be absent before the pack loads",
      );
      assert.ok(
        !beforeSignature || beforeSignature.signatures.length === 0,
        "unknown command must have no pack signature",
      );

      await config.update("specPacks", [pack], vscode.ConfigurationTarget.Workspace);
      await waitForEffectiveConfig(
        docUri,
        (effective) =>
          effective.spec_packs.includes(pack) &&
          effective.spec_packs_loaded.some(
            (loaded) => loaded.name === "tcl_lsp_fixture" && loaded.commands > 0,
          ),
        { timeout: 20_000, label: "synthetic pack loaded" },
      );

      const afterHover = (await pollUntil(
        () =>
          vscode.commands.executeCommand(
            "vscode.executeHoverProvider",
            docUri,
            commandPosition,
          ) as Thenable<vscode.Hover[] | undefined>,
        (hovers) => hoverText(hovers).includes("collecting a result"),
        { timeout: 20_000, label: "SpecTcl hover" },
      )) as vscode.Hover[];
      const afterSignature = (await pollUntil(
        () =>
          vscode.commands.executeCommand(
            "vscode.executeSignatureHelpProvider",
            docUri,
            argumentPosition,
          ) as Thenable<vscode.SignatureHelp | undefined>,
        (signature) =>
          Boolean(
            signature?.signatures.some((item) => item.label.includes("::tcl_lsp_fixture::collect")),
          ),
        { timeout: 20_000, label: "SpecTcl signature help" },
      )) as vscode.SignatureHelp;
      const afterTokens = await pollUntil(
        async () =>
          decodeTokens(
            (await vscode.commands.executeCommand(
              "vscode.provideDocumentSemanticTokens",
              docUri,
            )) as vscode.SemanticTokens,
            legend,
          ),
        (tokens) =>
          tokens.some(
            (token) =>
              token.line === 4 &&
              token.type === "function" &&
              doc.lineAt(token.line).text.slice(token.char, token.char + token.length) === "set",
          ),
        { timeout: 20_000, label: "SpecTcl body semantic tokens" },
      );

      assert.ok(hoverText(afterHover).includes("collecting a result"));
      assert.ok(
        afterSignature.signatures.some((item) => item.label.includes("::tcl_lsp_fixture::collect")),
      );

      const tokenText = (token: DecodedToken): string =>
        doc.lineAt(token.line).text.slice(token.char, token.char + token.length);
      assert.ok(
        !beforeTokens.some(
          (token) => token.line === 4 && token.type === "function" && tokenText(token) === "set",
        ),
        "without the pack, the custom command's braced body stays opaque",
      );
      for (const word of ["set", "lappend"]) {
        assert.ok(
          afterTokens.some((token) => token.type === "function" && tokenText(token) === word),
          `with the pack, '${word}' inside the Body argument must be a function token`,
        );
      }
      assert.ok(
        afterTokens.some(
          (token) => token.line === 3 && token.type === "variable" && tokenText(token) === "output",
        ),
        "the VarWrite argument must be a variable token",
      );
    } finally {
      await config.update("specPacks", original, vscode.ConfigurationTarget.Workspace);
    }
  });
});
