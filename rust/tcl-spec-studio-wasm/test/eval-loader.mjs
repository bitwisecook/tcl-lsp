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
 * Drive the **evaluation loader** through the real wasm module (design E
 * §15.2's WASM item).
 *
 * Compiling for wasm32 proves nothing about the eval loader: the pack
 * evaluator runs a Tcl program on the bytecode VM, and the two things that can
 * only fail on that target — reaching a host clock that is not there, and
 * arming a budget nothing can measure — both compile perfectly and then trap
 * or hang at run time (issue #1661 is exactly that failure mode one layer
 * up). So this loads two fixture packs through the *shipped* wasm exports:
 *
 *   1. `fixtures/canonical.tclspec` — literal registration calls only. The
 *      browser must browse it exactly as the native loader does, and must not
 *      classify it as a program.
 *   2. `fixtures/templated.tclspec` — a data table and a `foreach`. A CST walk
 *      sees no commands here at all, so every command the store reports is
 *      proof that the pack really *ran* inside the wasm module.
 *
 * It then exercises E-R12 on the templated pack: a form edit must leave the
 * author's program byte-for-byte and come back as a canonical patch pack.
 *
 * Usage: `node test/eval-loader.mjs <dir-with-wasm-bindgen-output>`, where the
 * directory holds `tcl_spec_studio_wasm.js` (the `no-modules` glue) and
 * `tcl_spec_studio_wasm_bg.wasm`. `build-wasm.sh` calls it with its own
 * staging directory, before the page is assembled.
 */

import { readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { runInThisContext } from "node:vm";

const here = dirname(fileURLToPath(import.meta.url));
const outDir = resolve(process.argv[2] ?? join(here, "..", "target", "bindgen"));

/** Fail with a message the build log can be read backwards from. */
function check(ok, what, detail) {
  if (ok) {
    console.log(`    ok — ${what}`);
    return;
  }
  throw new Error(`${what}\n       ${JSON.stringify(detail)}`);
}

/**
 * Boot the `no-modules` glue under node.
 *
 * The glue is written for a page: it reads `document.currentScript` to guess
 * where its `_bg.wasm` sits. `build-wasm.sh` already wraps that probe in a
 * try/catch (a blob: base makes `new URL` throw), and the same catch swallows
 * node's `ReferenceError: document is not defined` — so the file loads here
 * unmodified, and we hand it the bytes explicitly.
 */
async function boot() {
  const glue = await readFile(join(outDir, "tcl_spec_studio_wasm.js"), "utf8");
  const bytes = await readFile(join(outDir, "tcl_spec_studio_wasm_bg.wasm"));
  const bindgen = runInThisContext(`${glue}\n;wasm_bindgen;`);
  await bindgen({ module_or_path: bytes });
  return bindgen;
}

async function fixture(name) {
  return readFile(join(here, "fixtures", name), "utf8");
}

async function main() {
  console.log(`==> booting the studio wasm from ${outDir}`);
  const wasm = await boot();
  const load = (source) => JSON.parse(wasm.pack_load(source, "tcl"));

  // 1. A canonical pack browses as itself, and is not a program.
  const canonical = await fixture("canonical.tclspec");
  const plain = load(canonical);
  check(
    plain.commands?.length === 2 &&
      plain.commands.every((c) => c.declared_at?.expanded === false),
    "a canonical pack browses its two literal declarations",
    plain.commands,
  );
  check(
    plain.programmed === null && plain.patch === null,
    "a canonical pack is not classified as a program",
    { programmed: plain.programmed, patch: plain.patch },
  );

  // 2. A templated pack only has commands at all if the module *ran* it.
  const templated = await fixture("templated.tclspec");
  const expanded = load(templated);
  const names = (expanded.commands ?? []).map((c) => c.name);
  check(
    names.join(",") === "smoke::alpha,smoke::beta,smoke::gamma",
    "the evaluation loader ran the foreach inside wasm",
    names,
  );
  check(
    (expanded.commands ?? []).every((c) => c.declared_at?.expanded === true),
    "every templated declaration is marked as an expansion",
    expanded.commands,
  );
  check(
    expanded.programmed?.why === "expanded",
    "a templated pack is classified as a program (E-R12)",
    expanded.programmed,
  );
  check(
    expanded.target_dependent === false &&
      (expanded.notices ?? []).every((n) => !/budget/i.test(n.message ?? "")),
    "the evaluation completed inside its budget with no clock",
    expanded.notices,
  );

  // 3. E-R12: a form edit against the program becomes a patch pack.
  const draft = JSON.parse(wasm.pack_command(templated, "smoke::beta", "tcl"))
    .pack;
  draft.summary = "Edited in the browser, without touching the program.";
  const written = JSON.parse(
    wasm.pack_set_command(
      templated,
      "smoke::beta",
      JSON.stringify(draft),
      true,
    ),
  );
  check(
    written.writeback === "patched" && written.source === templated,
    "a form edit leaves the program byte-for-byte and patches instead",
    { writeback: written.writeback, changed: written.source !== templated },
  );
  check(
    written.patch?.source?.includes("command smoke::beta -override {") &&
      written.patch.standing_overrides?.[0]?.command === "smoke::beta",
    "the patch pack is canonical and the override stands",
    written.patch,
  );

  const reverted = JSON.parse(wasm.pack_remove_override(templated, "smoke::beta"));
  check(
    reverted.patch?.source === null &&
      (reverted.patch?.standing_overrides ?? []).length === 0,
    "removing the override restores the base",
    reverted.patch,
  );

  console.log("==> the evaluation loader runs under wasm32");
}

main().catch((error) => {
  console.error(`eval-loader smoke failed: ${error.message}`);
  process.exit(1);
});
