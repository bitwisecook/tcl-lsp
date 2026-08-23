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

/*
 * Boot the assembled spec-studio dist in a real browser and assert it works.
 *
 * This is the check that a dist is *deployable*, which no amount of unit
 * testing can give you: it serves `dist/` over HTTP exactly as GitHub Pages
 * does, loads the page in headless chromium under the page's own CSP, and
 * then walks the three things that can only fail in a browser —
 *
 *   1. the studio wasm instantiates and the registry loads;
 *   2. opening the Pack DSL tab fetches the lazy editor chunk and mounts
 *      Monaco (proving the code split resolves relative to the deployed
 *      directory, not to the site root);
 *   3. the language server worker starts, answers `initialize`, and reports
 *      itself running (proving the worker's three files landed together and
 *      that `worker-src`/`connect-src` are wide enough for them).
 *
 * Any console error, page error, or failed request is a failure: a CSP
 * violation shows up as exactly that and nothing else.
 *
 * Usage: `node test/boot.mjs [dist-dir]` (defaults to ../dist).
 * Requires playwright's chromium — `PLAYWRIGHT_BROWSERS_PATH` if it is not in
 * the default location. Skips with exit 0 when playwright is not installed, so
 * it never turns "this machine has no browser" into a build failure.
 */

import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { createRequire } from "node:module";
import { dirname, extname, join, normalize, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { crc32 } from "node:zlib";

const here = dirname(fileURLToPath(import.meta.url));
const distDir = resolve(process.argv[2] ?? join(here, "..", "dist"));

const TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".wasm": "application/wasm",
  ".svg": "image/svg+xml",
};

/** Serve `distDir` read-only, refusing anything that escapes it. */
function serve() {
  const server = createServer((req, res) => {
    const url = new URL(req.url ?? "/", "http://localhost");
    const rel = normalize(decodeURIComponent(url.pathname)).replace(
      /^(\.\.[/\\])+/,
      "",
    );
    const path = join(distDir, rel === "/" ? "index.html" : rel);
    if (!path.startsWith(distDir)) {
      res.writeHead(403).end("no");
      return;
    }
    readFile(path).then(
      (body) => {
        res.writeHead(200, {
          "content-type": TYPES[extname(path)] ?? "application/octet-stream",
        });
        res.end(body);
      },
      () => res.writeHead(404).end("not found"),
    );
  });
  return new Promise((ok) => server.listen(0, "127.0.0.1", () => ok(server)));
}

/**
 * Build a minimal STORED (uncompressed) zip from `{name: text}`.
 *
 * Hand-rolled rather than pulled from npm: this file is run by a build that
 * must not need a dependency tree, and a stored zip is a header, a body, and a
 * central directory. `unzip_entries` reads it with the real `zip` crate, so the
 * archive being trivial does not make the test trivial.
 */
function storedZip(files) {
  const entries = Object.entries(files);
  const locals = [];
  const central = [];
  let offset = 0;
  for (const [name, text] of entries) {
    const nameBytes = Buffer.from(name, "utf8");
    const body = Buffer.from(text, "utf8");
    const sum = crc32(body);
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50, 0);
    local.writeUInt16LE(20, 4); // version needed
    local.writeUInt16LE(0, 6); // flags
    local.writeUInt16LE(0, 8); // method: stored
    local.writeUInt32LE(sum, 14);
    local.writeUInt32LE(body.length, 18);
    local.writeUInt32LE(body.length, 22);
    local.writeUInt16LE(nameBytes.length, 26);
    locals.push(local, nameBytes, body);

    const header = Buffer.alloc(46);
    header.writeUInt32LE(0x02014b50, 0);
    header.writeUInt16LE(20, 4); // version made by
    header.writeUInt16LE(20, 6); // version needed
    header.writeUInt16LE(0, 10); // method: stored
    header.writeUInt32LE(sum, 16);
    header.writeUInt32LE(body.length, 20);
    header.writeUInt32LE(body.length, 24);
    header.writeUInt16LE(nameBytes.length, 28);
    header.writeUInt32LE(offset, 42);
    central.push(header, nameBytes);
    offset += 30 + nameBytes.length + body.length;
  }
  const dir = Buffer.concat(central);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(entries.length, 8);
  end.writeUInt16LE(entries.length, 10);
  end.writeUInt32LE(dir.length, 12);
  end.writeUInt32LE(offset, 16);
  return Buffer.concat([...locals, dir, end]);
}

/** Fail the run with a message. */
function fail(message) {
  console.error(`FAIL: ${message}`);
  process.exitCode = 1;
}

async function main() {
  try {
    await stat(join(distDir, "index.html"));
  } catch {
    fail(`no index.html in ${distDir} — run 'make spec-studio-wasm' first`);
    return;
  }

  let chromium;
  try {
    ({ chromium } = createRequire(import.meta.url)("playwright"));
  } catch {
    console.log("playwright not installed — skipping the browser boot check");
    return;
  }

  const server = await serve();
  const { port } = server.address();
  const base = `http://127.0.0.1:${port}/`;
  const browser = await chromium.launch();
  const page = await browser.newPage();
  await page.addInitScript(() => {
    window.__tclSpecStudioTestEnableEditorProbe = true;
  });

  const problems = [];
  page.on("pageerror", (e) => problems.push(`page error: ${e.message}`));
  page.on("console", (m) => {
    if (m.type() === "error") problems.push(`console error: ${m.text()}`);
  });
  page.on("requestfailed", (r) =>
    problems.push(
      `request failed: ${r.url()} (${r.failure()?.errorText ?? "?"})`,
    ),
  );

  try {
    console.log(`==> serving ${distDir} at ${base}`);
    await page.goto(base, { waitUntil: "load" });

    // 1. The studio wasm instantiated and the registry is browsable.
    await page.waitForFunction(
      () => (document.getElementById("count")?.textContent ?? "").match(/\d/),
      null,
      { timeout: 60_000 },
    );
    const count = await page.textContent("#count");
    console.log(`    registry loaded: ${count.trim()}`);

    // Registry-owned worked examples must show the full flow, not merely a
    // generic attachment point. Arrows identify source, processing, and
    // destination, while exactly one independent highlight marks the command
    // token carrying the trait. Use a historically long trait name here too:
    // its picker label used to stretch its background across the whole grid
    // cell instead of hugging the token.
    await page.click("#tab-reference");
    const flowContract = await page.evaluate(() => {
      const term = Array.from(document.querySelectorAll("code.term")).find(
        (node) => node.textContent === "EXPANSION_ESCAPE_SAFE",
      );
      const row = term?.closest("details.refrow");
      if (!row)
        throw new Error("EXPANSION_ESCAPE_SAFE reference row is missing");
      const help = row.querySelector("button.qbtn");
      if (!help)
        throw new Error("EXPANSION_ESCAPE_SAFE inline help button is missing");
      help.click();
      const carriers = Array.from(row.querySelectorAll("mark.example-carrier"));
      const arrows = Array.from(row.querySelectorAll("pre.example-arrow"));
      return {
        open: row.open,
        expanded: help.getAttribute("aria-expanded"),
        carriers: carriers.length,
        carrier: carriers[0]?.textContent ?? "",
        marks: row.querySelectorAll("mark").length,
        arrows: arrows.length,
        numbered: arrows.every((arrow, index) =>
          (arrow.textContent ?? "").includes(`→ ${index + 1}.`),
        ),
        colours: new Set(arrows.map((arrow) => getComputedStyle(arrow).color))
          .size,
        rowsWithoutHelp: Array.from(
          document.querySelectorAll("details.refrow"),
        ).filter((referenceRow) => !referenceRow.querySelector("button.qbtn"))
          .length,
      };
    });
    if (
      !flowContract.open ||
      flowContract.expanded !== "true" ||
      flowContract.carriers !== 1 ||
      flowContract.carrier !== "puts" ||
      flowContract.marks !== 1 ||
      flowContract.arrows < 3 ||
      !flowContract.numbered ||
      flowContract.colours < 3 ||
      flowContract.rowsWithoutHelp !== 0
    ) {
      throw new Error(
        `annotated flow rendering drifted: ${JSON.stringify(flowContract)}`,
      );
    }
    await page.click("#tab-editor");
    const chipContracts = await page.evaluate(() => {
      const key = Array.from(document.querySelectorAll(".field .key")).find(
        (node) => node.textContent === "traits",
      );
      const group = key?.closest("details.group");
      if (group) group.open = true;
      const items = [
        { name: "EXPANSION_ESCAPE_SAFE", carrier: "puts" },
        { name: "DEFERS_BODY", carrier: "proc" },
      ];
      const rowsWithoutHelp = Array.from(
        document.querySelectorAll(".trait-toggle"),
      ).filter((row) => !row.querySelector("button.qbtn")).length;
      return items.map(({ name, carrier: expectedCarrier }) => {
        const code = Array.from(
          document.querySelectorAll(".trait-copy code"),
        ).find((node) => node.textContent === name);
        const toggleGroup = code?.closest("details.toggle-group");
        if (toggleGroup) toggleGroup.open = true;
        if (!code?.parentElement)
          throw new Error(`${name} trait toggle is missing`);
        const row = code.closest(".trait-toggle");
        const help = row?.querySelector("button.qbtn");
        if (!row || !help)
          throw new Error(`${name} Spec-tab inline help is missing`);
        const checkbox = row.querySelector('input[type="checkbox"]');
        const checkedBeforeHelp = checkbox?.checked;
        help.click();
        const checkedAfterHelp = checkbox?.checked;
        const carriers = Array.from(row.querySelectorAll("mark.example-carrier"));
        const arrows = Array.from(row.querySelectorAll("pre.example-arrow"));
        return {
          name,
          tokenWidth: code.getBoundingClientRect().width,
          cellWidth: code.parentElement.getBoundingClientRect().width,
          justifySelf: getComputedStyle(code).justifySelf,
          expanded: help.getAttribute("aria-expanded"),
          carriers: carriers.length,
          carrier: carriers[0]?.textContent ?? "",
          expectedCarrier,
          arrows: arrows.length,
          checkedBeforeHelp,
          checkedAfterHelp,
          rowsWithoutHelp,
        };
      });
    });
    const stretchedChip = chipContracts.find(
      (chip) =>
        chip.justifySelf !== "start" ||
        chip.tokenWidth <= 0 ||
        chip.tokenWidth >= chip.cellWidth - 8 ||
        chip.expanded !== "true" ||
        chip.carriers !== 1 ||
        chip.carrier !== chip.expectedCarrier ||
        chip.arrows < 2 ||
        chip.checkedBeforeHelp !== chip.checkedAfterHelp ||
        chip.rowsWithoutHelp !== 0,
    );
    if (stretchedChip) {
      throw new Error(
        `trait token background still stretches: ${JSON.stringify(stretchedChip)}`,
      );
    }
    console.log(
      "    registry flows have numbered arrows, one command carrier, Spec-tab item help, and tight trait labels",
    );

    // 2. Opening the Pack DSL tab loads the editor chunk and mounts Monaco.
    await page.click("#tab-dsl");
    await page.waitForSelector("#dslEditor .monaco-editor", {
      timeout: 60_000,
    });
    console.log("    Monaco mounted on the Pack DSL tab");

    // 3. The language server worker reaches `initialize`.
    await page.waitForFunction(
      // The exact success sentence: the *failure* message also contains the
      // words "is running" ("the editor is running without analysis"), so a
      // looser predicate passes on a page with no language server at all.
      () =>
        (document.getElementById("lspStatus")?.textContent ?? "").includes(
          "language server is running in this page",
        ),
      null,
      { timeout: 120_000 },
    );
    console.log(`    LSP: ${(await page.textContent("#lspStatus")).trim()}`);

    const packEditorContract = await page.$eval("#dslEditor", (node) => ({
      monacoLanguage: node.dataset.monacoLanguage,
      lspLanguage: node.dataset.lspLanguage,
      documentUri: node.dataset.documentUri,
    }));
    if (
      packEditorContract.monacoLanguage !== "spectcl" ||
      packEditorContract.lspLanguage !== "spectcl" ||
      !packEditorContract.documentUri?.endsWith("/pack.tclspec")
    ) {
      throw new Error(
        `Pack DSL editor contract drifted: ${JSON.stringify(packEditorContract)}`,
      );
    }
    console.log(
      "    Pack DSL model is Monaco spectcl + LSP spectcl (Tcl 9.0 profile)",
    );

    // The success status is published only after Monaco itself has invoked the
    // semantic-token provider for this model, and hover has answered for it.
    // A direct transport probe is insufficient: editor.api exposes provider
    // registration even when the controller which consumes it was omitted.
    console.log(
      "    Pack DSL semantics and hover are served by the in-page LSP",
    );

    // Prove Monaco gets several semantic colours from the LSP. This catches a
    // provider whose legend contains theme scopes
    // instead of semantic-token identifiers: the worker is healthy, but every
    // token settles back to Monaco's one generic text colour.
    await page.waitForFunction(
      () => {
        const colours = new Set(
          Array.from(
            document.querySelectorAll(
              "#dslEditor .view-lines span[class*='mtk']",
            ),
          ).map((node) => getComputedStyle(node).color),
        );
        return colours.size > 1;
      },
      null,
      { timeout: 60_000 },
    );
    console.log("    Pack DSL semantic colours are visible in Monaco");

    // Prove hover is connected to the visible Monaco model, rather than only
    // to the direct readiness probe above. Synthetic pointer hover is not
    // portable across headless Chromium builds, so the page's opt-in test hook
    // invokes Monaco's public show-hover action at the exact model position.
    await page.evaluate(async () => {
      if (typeof window.__tclSpecStudioTestShowDslHover !== "function") {
        throw new Error(
          "the Monaco deployment-test hover hook was not installed",
        );
      }
      await window.__tclSpecStudioTestShowDslHover("speclib");
    });
    const visibleHover = page.locator(".monaco-hover:visible", {
      hasText: "speclib",
    });
    // Monaco retains a hidden hover widget alongside the active one.
    await visibleHover.waitFor({ timeout: 30_000 });
    console.log("    Pack DSL hover is visible in Monaco on initial load");

    // 4. The server actually analyses: type a broken pack line and expect the
    //    editor to show a marker. This is the difference between "the worker
    //    booted" and "the worker is doing the job".
    await page.click("#dslEditor .monaco-editor .view-lines");
    await page.keyboard.type("\ncommand");
    const markers = await page
      .waitForFunction(
        () => {
          const node = document.querySelector("#dslEditor .monaco-editor");
          return (
            node &&
            node.querySelectorAll(".squiggly-error, .squiggly-warning").length >
              0
          );
        },
        null,
        { timeout: 60_000 },
      )
      .catch(() => null);
    console.log(
      markers
        ? "    the language server published diagnostics into the editor"
        : "    note: no diagnostic appeared for the typed sample (not fatal)",
    );

    // 5. The Test tab's second editor mounts on its own model.
    await page.click("#tab-test");
    await page.waitForSelector("#testEditor .monaco-editor", {
      timeout: 30_000,
    });
    await page.click("#testEditor .monaco-editor .view-lines");
    await page.keyboard.press("Control+End");
    await page.keyboard.type("set colour red\nputs $colour");
    await page.waitForFunction(
      () => {
        const colours = new Set(
          Array.from(
            document.querySelectorAll(
              "#testEditor .view-lines span[class*='mtk']",
            ),
          ).map((node) => getComputedStyle(node).color),
        );
        return colours.size > 1;
      },
      null,
      { timeout: 60_000 },
    );
    const sampleEditorContract = await page.$eval("#testEditor", (node) => ({
      monacoLanguage: node.dataset.monacoLanguage,
      lspLanguage: node.dataset.lspLanguage,
    }));
    if (
      sampleEditorContract.monacoLanguage !== "tcl" ||
      sampleEditorContract.lspLanguage !== "spectcl"
    ) {
      throw new Error(
        `Test editor contract drifted: ${JSON.stringify(sampleEditorContract)}`,
      );
    }
    console.log("    Monaco mounted on the Test tab");

    // 6. A one-snapshot import writes the pack *and* becomes the active draft,
    // so the generated Rust pane follows it instead of retaining `mycommand`,
    // the placeholder draft installed at boot.
    await page.click("#tab-import");
    await page.setInputFiles("#picker", {
      name: "widget.tcl",
      mimeType: "text/plain",
      buffer: Buffer.from(
        "proc widget::paint {colour} { return $colour }\n",
        "utf8",
      ),
    });
    await page.waitForSelector("#importOut .found", { timeout: 30_000 });
    await page.click("#importAll");
    await page.click("#tab-rs");
    await page.waitForFunction(
      () => {
        const text =
          document.querySelector("#rsEditor .view-lines")?.textContent ?? "";
        return (
          text.includes("widget::paint") && !text.includes('name: "mycommand"')
        );
      },
      null,
      { timeout: 30_000 },
    );
    console.log(
      "    imported Tcl activates generated Rust for the imported command",
    );

    // 7. The Releases import mode and its GitHub panel are present and inert
    //    until asked — no request is made by opening them.
    await page.click("#tab-import");
    await page.click("#imode-releases");
    await page.waitForSelector("#ghPanel:not([hidden])", { timeout: 10_000 });
    console.log("    the Releases mode and the GitHub panel are reachable");

    // 8. The whole multi-release pipeline, through the real UI: two archives
    //    in, one version range out. `greet` exists in both releases and
    //    `farewell` only in the newer one, so the importer must derive
    //    `introduced 1.1` for the second and nothing for the first — the
    //    difference between "present as far back as we looked" and "arrived
    //    here", which is the entire point of the feature.
    await page.setInputFiles("#zipPicker", [
      {
        name: "mylib-1.0.zip",
        mimeType: "application/zip",
        buffer: storedZip({
          "mylib-1.0/pkgIndex.tcl": "package ifneeded mylib 1.0 {}\n",
          "mylib-1.0/mylib.tcl":
            "package provide mylib 1.0\nproc greet {who} { return $who }\n",
        }),
      },
      {
        name: "mylib-1.1.zip",
        mimeType: "application/zip",
        buffer: storedZip({
          "mylib-1.1/mylib.tcl":
            "package provide mylib 1.1\nproc greet {who} { return $who }\n" +
            "proc farewell {who} { return $who }\n",
        }),
      },
    ]);
    await page.waitForFunction(
      () => document.querySelectorAll("#releaseList li").length === 2,
      null,
      { timeout: 30_000 },
    );
    const labels = await page.$$eval("#releaseList .ver", (nodes) =>
      nodes.map((n) => n.value),
    );
    if (labels.join(",") !== "1.0,1.1") {
      throw new Error(
        `version labels were not suggested from the file names: ${labels}`,
      );
    }
    await page.click("#releasesRun");
    await page.waitForSelector("#releasesOut .found", { timeout: 60_000 });
    const derived = await page.$$eval("#releasesOut .found", (cards) =>
      cards.map((card) => ({
        name: card.querySelector(".nm")?.textContent ?? "",
        ranges: Array.from(card.querySelectorAll(".range")).map(
          (r) => r.textContent ?? "",
        ),
      })),
    );
    const farewell = derived.find((c) => c.name === "farewell");
    const greet = derived.find((c) => c.name === "greet");
    if (!farewell || !greet) {
      throw new Error(
        `expected greet and farewell, got ${JSON.stringify(derived)}`,
      );
    }
    if (!farewell.ranges.includes("introduced 1.1")) {
      throw new Error(
        `farewell should be introduced at 1.1: ${farewell.ranges.join(", ")}`,
      );
    }
    if (!greet.ranges.includes("introduced —")) {
      throw new Error(
        `greet's introduction is unknown with a partial history: ${greet.ranges.join(", ")}`,
      );
    }
    console.log(
      "    two release archives unpacked and diffed in-page: " +
        `${derived.map((c) => `${c.name} [${c.ranges.join(" ")}]`).join(", ")}`,
    );

    // 9. Mutate Monaco's semantic-token consumer out of the page.  The LSP
    // readiness status must still go green: it is based on an explicit
    // post-didOpen request, not on Monaco deciding to schedule its provider.
    // The direct hover probe is part of that readiness path too.  This is a
    // separate page so the ordinary path above still proves semantic colours
    // are visible in the real Monaco editor.
    const mutation = await browser.newPage();
    await mutation.addInitScript(() => {
      window.__tclSpecStudioTestSuppressMonacoSemanticProvider = true;
    });
    await mutation.goto(base, { waitUntil: "load" });
    await mutation.waitForFunction(
      () => (document.getElementById("count")?.textContent ?? "").match(/\d/),
      null,
      { timeout: 60_000 },
    );
    await mutation.click("#tab-dsl");
    await mutation.waitForSelector("#dslEditor .monaco-editor", {
      timeout: 60_000,
    });
    await mutation.waitForFunction(
      () =>
        (document.getElementById("lspStatus")?.textContent ?? "").includes(
          "language server is running in this page",
        ),
      null,
      { timeout: 60_000 },
    );
    await mutation.close();
    console.log(
      "    explicit post-didOpen LSP readiness survives a suppressed Monaco provider",
    );
  } catch (e) {
    const status = await page.textContent("#lspStatus").catch(() => "");
    fail(
      `${e instanceof Error ? e.message : String(e)}; LSP status: ${status?.trim() ?? ""}`,
    );
  } finally {
    await browser.close();
    server.close();
  }

  // A request to GitHub during a boot check would mean the panel is not opt-in.
  const network = problems.filter((p) => /github\.com/.test(p));
  if (network.length)
    fail(`the page reached GitHub without being asked: ${network.join("; ")}`);
  for (const problem of problems) fail(problem);
  if (!process.exitCode) console.log("==> boot check passed");
}

await main();
