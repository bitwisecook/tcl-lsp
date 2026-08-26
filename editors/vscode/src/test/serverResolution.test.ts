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
import * as path from "path";
import {
  bundlePlatformDir,
  bundledWasiModulePath,
  RUST_SERVER_EXE,
  resolveRustServer,
  WASI_MODULE_RELATIVE_PATH,
  wasiRuntimeAction,
} from "../serverResolution";

// A packaged install lives here; nothing on this path exists unless a test
// says it does, so every rung is exercised by naming exactly which files are
// present.
const EXT = path.join(path.sep, "ext", "tcl-lsp");
const BUNDLED_NATIVE = path.join(EXT, "server", bundlePlatformDir(), RUST_SERVER_EXE);
const BUNDLED_WASM = path.join(EXT, WASI_MODULE_RELATIVE_PATH);

function present(...paths: string[]): (candidate: string) => boolean {
  const set = new Set(paths);
  return (candidate) => set.has(candidate);
}

suite("Server resolution ladder", () => {
  test("an explicit binary wins over everything, and never falls through", () => {
    const explicit = path.join(path.sep, "opt", "tcl-lsp-server");
    assert.deepStrictEqual(
      resolveRustServer(explicit, "", EXT, present(explicit, BUNDLED_NATIVE, BUNDLED_WASM)),
      { kind: "native", path: explicit },
    );
    // A configured-but-missing explicit binary is an error, not a cue to
    // silently run something else.
    assert.strictEqual(
      resolveRustServer(explicit, "", EXT, present(BUNDLED_NATIVE, BUNDLED_WASM)),
      undefined,
    );
  });

  test("the bundled native binary wins over the bundled wasm module", () => {
    assert.deepStrictEqual(resolveRustServer("", "", EXT, present(BUNDLED_NATIVE, BUNDLED_WASM)), {
      kind: "native",
      path: BUNDLED_NATIVE,
    });
  });

  test("a dev checkout build wins over the bundled wasm module", () => {
    const checkout = path.join(path.sep, "src", "tcl-lsp");
    const built = path.join(checkout, "target", "release", RUST_SERVER_EXE);
    assert.deepStrictEqual(
      resolveRustServer("", checkout, EXT, present(built, BUNDLED_WASM)),
      { kind: "native", path: built },
      "release build",
    );
    const debugBuilt = path.join(checkout, "target", "debug", RUST_SERVER_EXE);
    assert.deepStrictEqual(
      resolveRustServer("", checkout, EXT, present(debugBuilt, BUNDLED_WASM)),
      { kind: "native", path: debugBuilt },
      "debug build",
    );
  });

  test("the wasm module is the last rung, taken only when no native binary exists", () => {
    assert.deepStrictEqual(resolveRustServer("", "", EXT, present(BUNDLED_WASM)), {
      kind: "wasm",
      modulePath: BUNDLED_WASM,
    });
    assert.strictEqual(bundledWasiModulePath(EXT), BUNDLED_WASM);
  });

  test("a configured serverPath is answered from that checkout or not at all", () => {
    // `serverPath` means "run what I built in this checkout".  Answering it
    // with the packaged wasm module would hide an unbuilt checkout, so the
    // wasm rung is skipped and the caller gets the loud no-server error.
    const checkout = path.join(path.sep, "src", "tcl-lsp");
    assert.strictEqual(
      resolveRustServer("", checkout, EXT, present(BUNDLED_NATIVE, BUNDLED_WASM)),
      undefined,
    );
  });

  test("nothing anywhere resolves to nothing", () => {
    assert.strictEqual(resolveRustServer("", "", EXT, present()), undefined);
  });

  test("the wasm module is staged under server/wasm/", () => {
    assert.strictEqual(
      WASI_MODULE_RELATIVE_PATH,
      path.join("server", "wasm", "tcl-lsp-server-wasi.wasm"),
    );
  });
});

suite("WASI runtime prompt gating", () => {
  test("an installed host runtime starts straight away", () => {
    assert.strictEqual(wasiRuntimeAction(true, false), "start");
    // A previous "don't ask again" must not veto a runtime that is now there.
    assert.strictEqual(wasiRuntimeAction(true, true), "start");
  });

  test("a missing host runtime prompts once, then stays quiet", () => {
    assert.strictEqual(wasiRuntimeAction(false, false), "prompt");
    assert.strictEqual(wasiRuntimeAction(false, true), "declined");
  });
});
