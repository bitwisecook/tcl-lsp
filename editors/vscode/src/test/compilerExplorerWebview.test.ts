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

import * as assert from "assert";
import { getWebviewHtml } from "../compilerExplorerHtml";

/**
 * Regression guards for the compiler-explorer webview shell (issues
 * #1182 / #1183).
 *
 * A renderer that threw used to abort the whole `result` handler: the
 * remaining tabs never rendered and the compile spinner was never hidden,
 * so the panel throbbed forever with no clue why. The webview now runs each
 * pane through `runRenderSteps` (from the shared `explorer-core.js`) and
 * clears the spinner in a `finally`.
 *
 * The webview body only exists as generated HTML, so assert on that rather
 * than trying to reach into a live panel's DOM.
 *
 * `explorer-core.js` is inlined as a base64 `data:` URI (the webview CSP
 * forbids external scripts), so `inlinedCoreJs` decodes it back out.
 */
function inlinedCoreJs(html: string): string {
  const sources = [...html.matchAll(/src="data:text\/javascript;base64,([^"]+)"/g)].map((m) =>
    Buffer.from(m[1], "base64").toString("utf8"),
  );
  const core = sources.find((s) => s.includes("function renderDisassembly"));
  assert.ok(core, "the webview must inline explorer-core.js");
  return core;
}

suite("Compiler Explorer webview", () => {
  const html = getWebviewHtml();
  const core = inlinedCoreJs(html);

  test("bundles the shared explorer-core render-isolation helper", () => {
    assert.ok(
      core.includes("function runRenderSteps"),
      "the webview must bundle explorer-core.js's runRenderSteps helper",
    );
  });

  test("renderAll runs every pane through runRenderSteps", () => {
    const renderAll = html.slice(html.indexOf("function renderAll()"));
    assert.ok(renderAll.length > 0, "renderAll must be defined in the webview");
    const body = renderAll.slice(0, renderAll.indexOf("\n}"));
    assert.ok(
      body.includes("runRenderSteps"),
      `renderAll must delegate to runRenderSteps, got:\n${body}`,
    );
    for (const pane of ["#pane-ir", "#pane-wasm", "#pane-asm", "#pane-taint"]) {
      assert.ok(body.includes(pane), `renderAll must own ${pane}`);
    }
  });

  test("the spinner is cleared in a finally so a render failure cannot wedge it", () => {
    const handler = html.slice(html.indexOf("case 'result':"));
    const caseBody = handler.slice(0, handler.indexOf("case 'error':"));
    assert.ok(
      /\}\s*finally\s*\{/.test(caseBody),
      `the result handler must clear the spinner in a finally block, got:\n${caseBody}`,
    );
    assert.ok(
      caseBody.includes("$('#spinner').style.display = 'none'"),
      "the result handler must hide the spinner",
    );
  });

  test("render failures are reported to the extension host", () => {
    assert.ok(
      html.includes("'renderError'") || html.includes('"renderError"'),
      "pane render failures must be posted back to the extension host",
    );
  });

  test("the module header renderer tolerates a missing contract field", () => {
    // `entry.types` was absent from the payload for a while; reading it
    // unguarded took the whole WASM tab (and the spinner) down.
    const header = core.slice(core.indexOf("function renderWasmModuleHeader"));
    for (const guard of ["entry.imports || []", "entry.types || []", "entry.dataSegments || []"]) {
      assert.ok(
        header.includes(guard),
        `renderWasmModuleHeader must default its lists to empty arrays (${guard})`,
      );
    }
  });
});
