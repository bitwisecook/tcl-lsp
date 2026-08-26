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
 * The web smoke test: does the extension actually work in a browser extension
 * host?
 *
 * Run by `npm run test:web`, which drives `@vscode/test-web` (headless
 * Chromium via Playwright) against `editors/vscode/testFixture` — the same
 * fixture folder the desktop suite uses. `@vscode/test-web` loads this module
 * in the web extension host and calls `run()`.
 *
 * Deliberately mocha-free: mocha would have to be bundled into the worker, and
 * this suite is four assertions about the one thing the desktop suite cannot
 * check — that the browser entry point activates, starts the wasm language
 * server, and produces real analysis for a workspace file that the extension
 * had to read and upsert itself, because the server has no filesystem.
 */

import * as vscode from "vscode";

const EXTENSION_ID = "bitwisecook.tcl-lsp";

/** Generous: the wasm module is ~25 MiB and instantiates on first activation. */
const ACTIVATION_TIMEOUT_MS = 180_000;
const ANALYSIS_TIMEOUT_MS = 180_000;

const results: string[] = [];
const failures: string[] = [];

function check(what: string, ok: boolean, detail?: string): void {
  if (ok) {
    results.push(`  ok   ${what}`);
  } else {
    failures.push(`${what}${detail ? ` — ${detail}` : ""}`);
    results.push(`  FAIL ${what}${detail ? ` — ${detail}` : ""}`);
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitFor<T>(
  what: string,
  timeoutMs: number,
  probe: () => Promise<T | undefined>,
): Promise<T | undefined> {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const value = await probe();
    if (value !== undefined) {
      return value;
    }
    if (Date.now() > deadline) {
      console.error(`tcl-lsp web smoke: timed out waiting for ${what}`);
      return undefined;
    }
    await sleep(500);
  }
}

export async function run(): Promise<void> {
  const extension = vscode.extensions.getExtension(EXTENSION_ID);
  check("the extension is installed in the web host", extension !== undefined);
  if (!extension) {
    throw new Error(`${EXTENSION_ID} is not installed`);
  }

  // A `browser` entry point is what makes VS Code load the extension at all in
  // a web host, so reaching an activated state proves the manifest, the
  // browser bundle, and its staged worker assets all line up.
  const activated = await waitFor("activation", ACTIVATION_TIMEOUT_MS, async () => {
    if (extension.isActive) {
      return true;
    }
    await extension.activate();
    return extension.isActive ? true : undefined;
  });
  check("the browser entry point activates", activated === true);

  const folder = vscode.workspace.workspaceFolders?.[0];
  check(
    "the fixture folder is open",
    folder !== undefined,
    `folders=${vscode.workspace.workspaceFolders?.length ?? 0}`,
  );
  if (!folder) {
    report();
    return;
  }
  console.log(`tcl-lsp web smoke: workspace ${folder.uri.toString()}`);

  const fixture = vscode.Uri.joinPath(folder.uri, "diagnostics-e001.tcl");
  const document = await vscode.workspace.openTextDocument(fixture);
  await vscode.window.showTextDocument(document);
  check(
    "the fixture opens as a Tcl document",
    document.languageId === "tcl",
    `languageId=${document.languageId}`,
  );

  // Ask the server directly, through the extension's exported client. This
  // separates the two halves of "it works": whether the wasm server answers at
  // all, and whether VS Code routes editor requests to it. They failed apart
  // once already — a document selector pinned to `file:` left the server
  // answering every direct request while the editor saw nothing, because no
  // provider was registered for a `vscode-test-web:` document.
  const api = extension.exports as
    | {
        getClient?: () =>
          { sendRequest?: (method: string, params: unknown) => Promise<unknown> } | undefined;
      }
    | undefined;
  let direct: { data?: number[] } | undefined;
  try {
    direct = (await api?.getClient?.()?.sendRequest?.("textDocument/semanticTokens/full", {
      textDocument: { uri: document.uri.toString() },
    })) as { data?: number[] } | undefined;
  } catch (err) {
    console.error(`tcl-lsp web smoke: direct semanticTokens request failed: ${String(err)}`);
  }
  check(
    "the wasm server answers a request put to it directly",
    (direct?.data?.length ?? 0) > 0,
    direct ? `${(direct.data?.length ?? 0) / 5} tokens` : "no result",
  );

  // Semantic tokens are the cheapest proof that the wasm server answered a
  // real request for this buffer.
  const tokens = await waitFor("semantic tokens", ANALYSIS_TIMEOUT_MS, async () => {
    const legend = await vscode.commands.executeCommand<vscode.SemanticTokens | undefined>(
      "vscode.provideDocumentSemanticTokens",
      document.uri,
    );
    return legend && legend.data.length > 0 ? legend : undefined;
  });
  check(
    "the wasm language server produces semantic tokens",
    tokens !== undefined,
    tokens ? `${tokens.data.length / 5} tokens` : "none within the timeout",
  );

  // …and diagnostics prove the analyser ran, not just the tokeniser. The
  // fixture's bare `string` is E001.
  const diagnostics = await waitFor("diagnostics", ANALYSIS_TIMEOUT_MS, async () => {
    const found = vscode.languages.getDiagnostics(document.uri);
    return found.length > 0 ? found : undefined;
  });
  check(
    "the wasm language server publishes diagnostics",
    diagnostics !== undefined,
    diagnostics
      ? `${diagnostics.length}: ${diagnostics
          .slice(0, 3)
          .map((d) => `${String(d.code)} ${d.message}`)
          .join(" | ")}`
      : "none within the timeout",
  );
  check(
    "the diagnostics include the fixture's E001",
    (diagnostics ?? []).some((d) => String(d.code).includes("E001")),
    (diagnostics ?? []).map((d) => String(d.code)).join(","),
  );

  report();
}

function report(): void {
  console.log(`tcl-lsp web smoke:\n${results.join("\n")}`);
  if (failures.length > 0) {
    throw new Error(
      `tcl-lsp web smoke: ${failures.length} check(s) failed:\n - ${failures.join("\n - ")}`,
    );
  }
}
