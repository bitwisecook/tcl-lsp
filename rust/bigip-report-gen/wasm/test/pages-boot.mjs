// BIG-IP report-generator browser contract for the GitHub Pages deployment.
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
const fixture = resolve(
  process.argv[3] ??
    join(here, "..", "..", "..", "tcl-bigip", "tests", "fixtures", "bigip.conf"),
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
      /^(?:\.\.[/\\])+/,
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
    "playwright not installed — skipping BIG-IP report Pages boot check",
  );
  process.exit(0);
}

const server = await serve();
const { port } = server.address();
const browser = await chromium.launch();
const page = await browser.newPage({ acceptDownloads: true });
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
  await page.goto(
    `http://127.0.0.1:${port}/bigip-report-generator/?pages-test=1`,
    {
      waitUntil: "load",
    },
  );
  await page.waitForFunction(
    () => document.querySelector("#status")?.classList.contains("ok"),
    undefined,
    { timeout: 120_000 },
  );
  const version = (await page.textContent("#ver"))?.trim();
  if (!version || !version.startsWith("engine v"))
    throw new Error("the report WASM engine did not expose its version");

  await page.setInputFiles("#picker", fixture);
  await page.waitForFunction(
    () =>
      !(document.querySelector("#go") instanceof HTMLButtonElement) ||
      !document.querySelector("#go").disabled,
    undefined,
    { timeout: 30_000 },
  );
  const downloadPromise = page.waitForEvent("download", { timeout: 120_000 });
  await page.click("#go");
  const download = await downloadPromise;
  await page.waitForFunction(
    () =>
      document.querySelector("#status")?.textContent?.startsWith("done —") &&
      document.querySelector("#result")?.classList.contains("show"),
    undefined,
    { timeout: 120_000 },
  );
  if (!download.suggestedFilename().endsWith(".html"))
    throw new Error(`unexpected report filename: ${download.suggestedFilename()}`);
  const reportPath = await download.path();
  if (!reportPath) throw new Error("generated report download has no local path");
  const report = await readFile(reportPath, "utf8");
  if (!report.includes("www_vs"))
    throw new Error("generated report does not contain the fixture's virtual server");
  if (problems.length > 0) throw new Error(problems.join("\n"));
  console.log(
    `BIG-IP report generator generated ${download.suggestedFilename()} from a config fixture (${version})`,
  );
} catch (error) {
  if (problems.length > 0) console.error(problems.join("\n"));
  throw error;
} finally {
  await page.close();
  await browser.close();
  await new Promise((done) => server.close(done));
}
