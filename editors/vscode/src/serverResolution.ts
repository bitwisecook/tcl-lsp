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
 * Which language server the **node** entry point runs, and what that choice
 * needs on disk.
 *
 * Deliberately free of the `vscode` API and of `@vscode/wasm-wasi*`: every
 * decision here is a pure function of a few strings plus a filesystem
 * existence predicate, so the whole ladder — including the WASI rung's gating
 * — is unit-testable without an extension host and without the WASM WASI Core
 * extension, which cannot be installed in CI (see `./wasiServer` for the parts
 * that genuinely need both).
 */

import { existsSync } from "fs";
import * as path from "path";

/** The native server filename for the current OS. */
export const RUST_SERVER_EXE =
  process.platform === "win32" ? "tcl-lsp-server.exe" : "tcl-lsp-server";

/**
 * VSIX bundle directory for the current platform, e.g. `darwin-arm64`,
 * `linux-x64`, `linux-riscv64`, `win32-arm64`.  Node's `process.platform` /
 * `process.arch` map 1:1 onto the directory names every VSIX (the universal
 * package and each of the six platform-targeted packages) ships under
 * `server/<platform>-<arch>/` (and onto VS Code's own target slugs).
 */
export function bundlePlatformDir(): string {
  return `${process.platform}-${process.arch}`;
}

/**
 * Where the universal VSIX stages the WASI language server module, relative to
 * the extension root.  Mirrored by the Makefile's `$(VSIX_FILE)` staging and
 * asserted by `verify-vsix`; the six platform-targeted VSIXes deliberately do
 * NOT carry it, because each already ships its own native binary.
 */
export const WASI_MODULE_RELATIVE_PATH = path.join("server", "wasm", "tcl-lsp-server-wasi.wasm");

/**
 * The bundled SpecTcl loadables staged beside the module, relative to the
 * extension root.
 *
 * The native rung gets them as a `specs/` directory beside the executable,
 * which is what `tcl_spectcl::discovery::bundled_dir` looks for.  A WASI guest
 * has no executable to sit beside, so the same directory is mounted into the
 * guest and named by `TCL_LSP_SPEC_PACK_DIR` instead — see `./wasiServer`.
 */
export const WASI_SPECS_RELATIVE_PATH = path.join("server", "wasm", "specs");

/** The bundled WASI module's absolute path inside an installed extension. */
export function bundledWasiModulePath(extensionPath: string): string {
  return path.join(extensionPath, WASI_MODULE_RELATIVE_PATH);
}

/**
 * The rung of the ladder that answered.  `native` is an executable to spawn;
 * `wasm` is the bundled WASI module, which needs a host to run it.
 */
export type ServerResolution =
  | { readonly kind: "native"; readonly path: string }
  | { readonly kind: "wasm"; readonly modulePath: string };

/**
 * Locate the language server the node entry should run, in ladder order:
 *
 * 1. an explicit path (`tclLsp.rustServerPath` or `TCL_LSP_SERVER_BIN`);
 * 2. the native binary bundled inside the VSIX
 *    (`server/<platform>-<arch>/tcl-lsp-server`);
 * 3. a dev checkout's `target/{release,debug}/tcl-lsp-server`;
 * 4. the bundled WASI module (`server/wasm/tcl-lsp-server-wasi.wasm`).
 *
 * Rung 4 is reached only when no native binary exists at all — it is the
 * universal VSIX's answer for a platform none of the seven cross-compiled
 * triples covers.  A platform WITH a native binary never pays for it: this
 * function returns before the wasm branch, and `extension.ts` only imports
 * `./wasiServer` once that branch is taken.
 *
 * `exists` is injectable so the ladder can be exercised without a filesystem.
 */
export function resolveRustServer(
  configuredBin: string,
  configuredServerPath: string,
  extensionPath: string,
  exists: (candidate: string) => boolean = existsSync,
): ServerResolution | undefined {
  const explicit = configuredBin.trim();
  if (explicit) {
    return exists(explicit) ? { kind: "native", path: explicit } : undefined;
  }
  // A configured `serverPath` means "run from this checkout" and must take
  // precedence over the binary bundled in the VSIX — otherwise an installed
  // user who points at a local checkout to test server changes would silently
  // keep getting the packaged binary.  Only consult the bundled binary when no
  // serverPath is set.
  if (!configuredServerPath.trim()) {
    // Packaged install: whichever VSIX this is (the universal package, or
    // one of the six platform-targeted packages), it ships this platform's
    // binary at the same server/<platform>-<arch>/ path.
    const bundled = path.join(extensionPath, "server", bundlePlatformDir(), RUST_SERVER_EXE);
    if (exists(bundled)) {
      return { kind: "native", path: bundled };
    }
  }
  // Dev checkout: pick up a locally built binary.
  const root = resolveServerDir(configuredServerPath, extensionPath, exists);
  for (const profile of ["release", "debug"]) {
    const candidate = path.join(root, "target", profile, RUST_SERVER_EXE);
    if (exists(candidate)) {
      return { kind: "native", path: candidate };
    }
  }
  // Last rung: the universal VSIX's WebAssembly fallback.  An explicit
  // `serverPath` is not consulted here — it named a checkout to run the native
  // binary from, and answering it with the packaged module would hide the fact
  // that the checkout was never built.
  if (!configuredServerPath.trim()) {
    const module = bundledWasiModulePath(extensionPath);
    if (exists(module)) {
      return { kind: "wasm", modulePath: module };
    }
  }
  return undefined;
}

function hasRustWorkspace(dir: string, exists: (candidate: string) => boolean): boolean {
  return exists(path.join(dir, "Cargo.toml"));
}

function resolveServerDir(
  configuredPath: string,
  extensionPath: string,
  exists: (candidate: string) => boolean,
): string {
  const configured = configuredPath.trim();
  if (configured) {
    return configured;
  }
  // Walk up from the extension directory to find the workspace root.
  // Handles both repo-root layouts (extension at /) and nested layouts
  // (extension at editors/vscode/).
  let dir = extensionPath;
  for (let i = 0; i < 3; i++) {
    if (hasRustWorkspace(dir, exists)) {
      return dir;
    }
    const parent = path.resolve(dir, "..");
    if (parent === dir) break;
    dir = parent;
  }
  return extensionPath;
}

/**
 * The extension that supplies a WASI host to VS Code.  A **soft** dependency,
 * never `extensionDependencies`: the universal package and the six targeted
 * packages share one `package.json`, so declaring it hard would force every
 * user on a platform that has a native binary to install a runtime they will
 * never execute.
 */
export const WASM_WASI_CORE_EXTENSION_ID = "ms-vscode.wasm-wasi-core";

/** What the WASI rung should do about its missing-or-present host runtime. */
export type WasiRuntimeAction = "start" | "prompt" | "declined";

/**
 * Gate the one-time install prompt.
 *
 * `dismissed` is the persisted "don't ask again" answer (a `globalState`
 * memento, not a setting — the WASI rung is unreachable on every platform that
 * has a native binary, so a user-visible setting would be noise in the
 * settings UI for all but a handful of installs).
 */
export function wasiRuntimeAction(installed: boolean, dismissed: boolean): WasiRuntimeAction {
  if (installed) {
    return "start";
  }
  return dismissed ? "declined" : "prompt";
}
