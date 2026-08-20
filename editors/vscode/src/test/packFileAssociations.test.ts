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

// Lazy file-extension registration for SpecTcl packs, through the extension
// host (issue #1626).
//
// The server half of this is already lazy and has a unit test of its own: a
// discovered pack's `file_extension` rows are consulted by
// `dialect_from_extension` before the built-in catalogue. What that cannot
// prove is the half the user experiences — that dropping a `.tclspec` into a
// project makes the editor itself recognise the extension it claims. VS Code
// learns associations from a manifest written long before any user's pack
// existed, so without a runtime channel the file opens as plain text and the
// language client never attaches, however right the server is.
//
// So this suite drives it end to end: a scratch pack declaring an extension
// nothing else in the repo owns, and the assertion that the server advertises
// it and the client turns that into a real, reversible workspace association.
//
// The scratch directory is this suite's alone (a plain directory, not
// `.tcl-lsp/`, so nothing is discovered by convention), which keeps every
// other suite's registry free of `extpack`.

import * as assert from "assert";
import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";

import { activate, getDocUri, pollUntil } from "./helper";

/** The scratch directory the suite owns; git-ignored. */
const SCRATCH = path.resolve(__dirname, "../../testFixture/packExtensionScratch");
const PACK = path.join(SCRATCH, "extensions.tclspec");
const PACK_NAME = "extpack";

/** The extension the pack claims. Owned by nothing in the catalogue, and by
 *  no other pack — so any association for it can only have come from here. */
const PACK_EXTENSION = "irulex";
const PACK_GLOB = `*.${PACK_EXTENSION}`;
/** The dialect the row routes to, and hence the language id the client must
 *  choose: `f5-irules` has a dedicated editor language, so the file must land
 *  on `tcl-irule` rather than the plain `tcl` fallback. */
const PACK_DIALECT = "f5-irules";
const EXPECTED_LANGUAGE = "tcl-irule";

/** A pack whose only job is to claim one file extension. */
const PACK_BODY = `speclib ${PACK_NAME} 1.1 {

file_extension ${PACK_EXTENSION} -name {Extension Pack Rules} -dialect ${PACK_DIALECT}

command ::extpack::noop {
    dialects tcl8.6+
    arity 1
}

}
`;

const docUri = getDocUri("specPackTorture.tcl");

interface EffectiveConfigWithPacks {
  pack_file_extensions?: Array<{
    extension: string;
    dialect?: string | null;
    language_id: string;
    pack?: string;
  }>;
}

/** The `pack_file_extensions` the server currently advertises. */
async function advertisedExtensions(): Promise<
  NonNullable<EffectiveConfigWithPacks["pack_file_extensions"]>
> {
  const cfg = (await vscode.commands.executeCommand(
    "tcl-lsp.getEffectiveConfig",
    docUri.toString(),
  )) as EffectiveConfigWithPacks | undefined;
  return cfg?.pack_file_extensions ?? [];
}

/** The workspace-scoped `files.associations` map, as VS Code has it now. */
function workspaceAssociations(): Record<string, string> {
  return (
    vscode.workspace
      .getConfiguration("files")
      .inspect<Record<string, string>>("associations")?.workspaceValue ?? {}
  );
}

suite("Pack-declared file extensions through the extension host", () => {
  let originalPacks: string[] | undefined;
  let originalAssociations: Record<string, string> | undefined;

  suiteSetup(async function () {
    this.timeout(180_000);
    fs.mkdirSync(SCRATCH, { recursive: true });
    fs.writeFileSync(PACK, PACK_BODY, "utf8");

    const tclConfig = vscode.workspace.getConfiguration("tclLsp", docUri);
    originalPacks = tclConfig.inspect<string[]>("specPacks")?.workspaceValue;
    originalAssociations = workspaceAssociations();
    await tclConfig.update("specPacks", [SCRATCH], vscode.ConfigurationTarget.Workspace);

    await activate(docUri);
  });

  suiteTeardown(async function () {
    this.timeout(60_000);
    try {
      await vscode.workspace
        .getConfiguration("tclLsp", docUri)
        .update("specPacks", originalPacks, vscode.ConfigurationTarget.Workspace);
      await vscode.workspace
        .getConfiguration("files")
        .update(
          "associations",
          originalAssociations && Object.keys(originalAssociations).length > 0
            ? originalAssociations
            : undefined,
          vscode.ConfigurationTarget.Workspace,
        );
    } catch {
      // Best effort — index.ts restores the committed fixture settings anyway.
    }
    fs.rmSync(SCRATCH, { recursive: true, force: true });
  });

  test("the server advertises the extension its discovered pack claims", async function () {
    this.timeout(120_000);
    const rows = await pollUntil(
      advertisedExtensions,
      (found) => found.some((row) => row.extension === PACK_EXTENSION),
      { timeout: 60_000, label: `the server to advertise .${PACK_EXTENSION}` },
    );
    const row = rows.find((r) => r.extension === PACK_EXTENSION);
    assert.ok(row, `no advertised row for .${PACK_EXTENSION}`);
    assert.strictEqual(row.dialect, PACK_DIALECT);
    // A pack extension cannot invent an editor language, so it must land on
    // the one its dialect already has — not the plain `tcl` fallback, which
    // would be the answer for a dialect with no dedicated language.
    assert.strictEqual(row.language_id, EXPECTED_LANGUAGE);
    assert.strictEqual(row.pack, PACK_NAME);

    // NEGATIVE control: an extension the shipped catalogue already owns is
    // registered statically by every editor and must not be re-advertised —
    // a redundant association would be indistinguishable from a real one on
    // cleanup.
    assert.ok(
      !rows.some((r) => r.extension === "irule" || r.extension === "tcl"),
      `catalogue-owned extensions must not be advertised: ${JSON.stringify(rows)}`,
    );
  });

  test("the client turns the advertisement into a workspace association", async function () {
    this.timeout(120_000);
    const associations = await pollUntil(
      async () => workspaceAssociations(),
      (found) => found[PACK_GLOB] !== undefined,
      { timeout: 60_000, label: `${PACK_GLOB} to be associated` },
    );
    assert.strictEqual(associations[PACK_GLOB], EXPECTED_LANGUAGE);
  });

  test("a file with the pack's extension opens as its dialect's language", async function () {
    this.timeout(120_000);
    const filePath = path.join(SCRATCH, `sample.${PACK_EXTENSION}`);
    fs.writeFileSync(filePath, 'when HTTP_REQUEST {\n    log local0. "hi"\n}\n', "utf8");
    const document = await vscode.workspace.openTextDocument(vscode.Uri.file(filePath));
    assert.strictEqual(
      document.languageId,
      EXPECTED_LANGUAGE,
      "the pack-claimed extension must open under its dialect's language",
    );
  });

  test("removing the pack retires the association it added", async function () {
    this.timeout(180_000);
    fs.rmSync(PACK, { force: true });
    // The watcher fires on the delete; the server reloads its pack set and
    // stops advertising, and the client retires exactly the key it wrote.
    await pollUntil(
      async () => workspaceAssociations(),
      (found) => found[PACK_GLOB] === undefined,
      { timeout: 90_000, label: `${PACK_GLOB} to be retired` },
    );
    // Put it back so a later run of this suite starts from the same place.
    fs.writeFileSync(PACK, PACK_BODY, "utf8");
  });
});
