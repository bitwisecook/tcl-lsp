// tcl-lsp — browser contract for the GitHub Pages Compiler Explorer
// SPDX-License-Identifier: AGPL-3.0-or-later

import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, extname, join, normalize, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const site = resolve(
  process.argv[2] ?? join(here, "..", "..", "..", "..", "site"),
);
const types = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json",
  ".wasm": "application/wasm",
};

function serve() {
  const server = createServer((request, response) => {
    const url = new URL(request.url ?? "/", "http://localhost");
    const relative = normalize(decodeURIComponent(url.pathname)).replace(
      /^(\.\.[/\\])+/,
      "",
    );
    const path = join(
      site,
      relative === "/" ? "index.html" : relative.replace(/\/$/, "/index.html"),
    );
    if (!path.startsWith(site)) return response.writeHead(403).end("no");
    readFile(path).then(
      (body) => {
        response.writeHead(200, {
          "content-type": types[extname(path)] ?? "application/octet-stream",
        });
        response.end(body);
      },
      () => response.writeHead(404).end("not found"),
    );
  });
  return new Promise((resolveServer) =>
    server.listen(0, "127.0.0.1", () => resolveServer(server)),
  );
}

let chromium;
try {
  ({ chromium } = createRequire(import.meta.url)("playwright"));
} catch {
  console.log(
    "playwright not installed — skipping Compiler Explorer Pages boot check",
  );
  process.exit(0);
}

const server = await serve();
const { port } = server.address();
const browser = await chromium.launch();
const page = await browser.newPage();
const problems = [];
page.on("pageerror", (error) => problems.push(`page error: ${error.message}`));
page.on("console", (message) => {
  if (message.type() === "error")
    problems.push(`console error: ${message.text()}`);
});
page.on("requestfailed", (request) =>
  problems.push(
    `request failed: ${request.url()} (${request.failure()?.errorText ?? "?"})`,
  ),
);

try {
  await page.goto(`http://127.0.0.1:${port}/compiler-explorer/?pages-test=1`, {
    waitUntil: "load",
  });
  await page.waitForSelector("#monacoSource.monaco-mounted .monaco-editor", {
    timeout: 120_000,
  });
  const textareaHidden = await page.$eval(
    "#source",
    (element) => element.hidden,
  );
  if (!textareaHidden)
    throw new Error("the fallback textarea stayed visible beneath Monaco");
  const version = (await page.textContent("#versionBar"))?.trim();
  if (!version || version.includes("__TCL_LSP"))
    throw new Error("the Pages version was not stamped");
  if (problems.length > 0) throw new Error(problems.join("\n"));
  console.log(
    `Compiler Explorer mounted the shared Spec Studio Monaco editor (${version})`,
  );
} catch (error) {
  if (problems.length > 0) console.error(problems.join("\n"));
  throw error;
} finally {
  await page.close();
  await browser.close();
  await new Promise((done) => server.close(done));
}
