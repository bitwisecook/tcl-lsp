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
import fileUriPathPolicy from "./generated/fileUriPathPolicy.json";

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
 *
 * A literal, forward-slashed string, not `path.join`: this names an entry
 * inside a VSIX, whose separator is `/` on every host.  `bundledWasiModulePath`
 * puts it through `path.join`, which turns it into a host path where that
 * matters; going the other way — a `path.join` constant carrying `\` on
 * Windows — cannot be undone by a caller that needs the archive spelling.
 */
export const WASI_MODULE_RELATIVE_PATH = "server/wasm/tcl-lsp-server-wasi.wasm";

/**
 * The bundled SpecTcl loadables staged beside the module, relative to the
 * extension root.
 *
 * The native rung gets them as a `specs/` directory beside the executable,
 * which is what `tcl_spectcl::discovery::bundled_dir` looks for.  A WASI guest
 * has no executable to sit beside, so the same directory is mounted into the
 * guest and named by `TCL_LSP_SPEC_PACK_DIR` instead — see `./wasiServer`.
 *
 * Forward-slashed for the same reason as `WASI_MODULE_RELATIVE_PATH`, and one
 * more: this is handed straight to `@vscode/wasm-wasi` as an
 * `extensionLocation` mount descriptor path, which is a URI path and never a
 * host path, so a `\` from `path.join` on a Windows host would be wrong at
 * the point of use rather than merely unidiomatic.
 */
export const WASI_SPECS_RELATIVE_PATH = "server/wasm/specs";

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

/**
 * One workspace folder, reduced to the two strings the guest mapping needs.
 * Taken as plain strings so the mapping below stays free of the `vscode` API.
 */
export interface WasiWorkspaceFolder {
  /** The folder's URI as the editor spells it, e.g. `file:///home/me/proj`. */
  readonly uri: string;
  /** The folder's display name — the guest path segment in a multi-root window. */
  readonly name: string;
}

/** Translates URIs across the sandbox boundary, in both directions. */
export interface WasiUriMapping {
  /** An editor URI as the guest will see it. */
  toGuest(uri: string): string;
  /** A guest URI as the editor spells it. */
  toEditor(uri: string): string;
}

/**
 * Where the WASM WASI host mounts each workspace folder inside the guest.
 *
 * Single-root is `/workspace`; multi-root is `/workspaces/<folder name>` —
 * note the **plural** in the multi-root form.
 *
 * This is why we do not use `@vscode/wasm-wasi-lsp`'s `createUriConverters()`:
 * the two halves of that upstream disagree. `@vscode/wasm-wasi@1.0.2`'s
 * `WorkspaceFolderDescriptor` documents the mount as `/workspace` (single) and
 * `/workspaces/folder-name` (multi), and the wasm-wasi-core extension mounts
 * accordingly — but `@vscode/wasm-wasi-lsp@0.1.0-pre.9`'s `createUriConverters`
 * maps multi-root folders to `file:///workspace/<name>`, singular
 * (`lib/main.js` lines 146-157). Single-root agrees; multi-root does not, so
 * every URI in a multi-root window would be translated to a path the guest has
 * nothing mounted at. Verified against those two exact versions; re-check this
 * comment when either is bumped.
 *
 * The result is the guest's **filesystem** path, so the folder name goes in
 * raw: it is the string the host created the mount with, and the string a
 * guest `open` has to name. The URI that spells the same mount is a different
 * string — see `encodeGuestPath` below.
 */
export function guestMountPoint(folder: WasiWorkspaceFolder, multiRoot: boolean): string {
  return multiRoot ? `/workspaces/${folder.name}` : "/workspace";
}

/** The bytes `encodeGuestPath` leaves alone: `unreserved` plus the separator. */
const UNRESERVED_GUEST_PATH_BYTE = /[A-Za-z0-9\-._~/]/;

/**
 * Percent-encode a guest mount path for the `file:` URI that names it.
 *
 * The server's own rule, byte for byte — `uri_norm.rs`'s
 * `encode_path_segment_bytes`, which is `ls_types`' `from_file_path` set: keep
 * `A-Za-z0-9-._~` and the `/` separator, escape every other UTF-8 byte with
 * upper-case hex. Neither `encodeURI` nor `encodeURIComponent` is that rule
 * (both keep `!*'()`, and `encodeURI` keeps the gen-delims too), and the
 * spelling has to agree to the byte: the URI the server constructs for a file
 * it scanned under the mount comes back through `toEditor`, where it is
 * matched against this prefix. `/` is kept for the same reason — it is a
 * separator in the guest path on both sides of that comparison.
 *
 * Unconditional, not a fallback for a name that fails to parse, exactly as in
 * `rooted_file_uri`. A folder named `foo#bar` yields a URI that parses
 * perfectly well — the path `/workspaces/foo` plus a fragment — and so names a
 * different guest path entirely; `My Project` yields no valid URI at all, and
 * the server's repair pass percent-encodes it to a spelling an unencoded
 * prefix would then fail to match.
 */
function encodeGuestPath(mountPoint: string): string {
  let encoded = "";
  for (const byte of new TextEncoder().encode(mountPoint)) {
    const char = String.fromCharCode(byte);
    encoded += UNRESERVED_GUEST_PATH_BYTE.test(char)
      ? char
      : `%${byte.toString(16).toUpperCase().padStart(2, "0")}`;
  }
  return encoded;
}

/**
 * Build the URI translation for a window's workspace folders, or `undefined`
 * when there are none (nothing is mounted, so there is nothing to translate).
 *
 * Folders are matched longest-editor-URI-first, and only on a whole path
 * component, so a nested folder wins over its parent and a sibling directory
 * whose path merely *starts* with another's (`…/proj` and `…/proj2`) cannot be
 * mistaken for it. Upstream's converter does neither.
 */
export function wasiUriMapping(
  folders: readonly WasiWorkspaceFolder[],
): WasiUriMapping | undefined {
  if (folders.length === 0) {
    return undefined;
  }
  const multiRoot = folders.length > 1;
  const pairs = folders
    .map((folder) => ({
      editor: trimTrailingSlash(folder.uri),
      // The mount point and the URI naming it are two spellings of the same
      // place, and only one of them is percent-encoded: `My Project` mounts at
      // `/workspaces/My Project` and is spelled
      // `file:///workspaces/My%20Project`, which is the spelling the server
      // produces for that path and so the one a reply must be matched against.
      guest: `file://${encodeGuestPath(guestMountPoint(folder, multiRoot))}`,
    }))
    .sort((a, b) => b.editor.length - a.editor.length);

  const translate = (value: string, from: "editor" | "guest"): string => {
    const to = from === "editor" ? "guest" : "editor";
    for (const pair of pairs) {
      const prefix = pair[from];
      if (value === prefix) {
        return pair[to];
      }
      if (value.startsWith(`${prefix}/`)) {
        return pair[to] + value.slice(prefix.length);
      }
    }
    return value;
  };

  return {
    toGuest: (uri) => translate(uri, "editor"),
    toEditor: (uri) => translate(uri, "guest"),
  };
}

function trimTrailingSlash(uri: string): string {
  return uri.endsWith("/") ? uri.slice(0, -1) : uri;
}

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

/** Actual URI view supplied by the VS Code provider, never a display label. */
export interface WasiMountUri {
  readonly scheme: string;
  readonly authority: string;
  readonly path: string;
  readonly query: string;
  readonly fragment: string;
  toString(): string;
  with(change: { path: string }): WasiMountUri;
}

export interface WasiAllocatedMount {
  readonly editor: WasiMountUri;
  readonly guestPath: string;
}

/** A current process's provider-verified explicit mount allocation. */
export function allocatedWasiUriMapping(
  mounts: readonly WasiAllocatedMount[],
  parse: (value: string) => WasiMountUri,
  isCurrent: () => boolean,
): { toGuest(value: WasiMountUri): string; toEditor(value: string): WasiMountUri } {
  const selected = [...mounts];
  const validPath = (path: string): boolean =>
    path.startsWith("/") &&
    !path.includes("\0") &&
    !path.split("/").some((part) => part === "." || part === "..");
  const components = (path: string): string[] => {
    const parts = path.split("/");
    if (parts.length > 1 && parts[parts.length - 1] === "") parts.pop();
    return parts;
  };
  const suffix = (path: string, root: string): string[] | undefined => {
    const all = components(path),
      base = components(root);
    return base.every((part, i) => all[i] === part) ? all.slice(base.length) : undefined;
  };
  const join = (root: string, tail: readonly string[]): string =>
    tail.length ? `${root.endsWith("/") ? root.slice(0, -1) : root}/${tail.join("/")}` : root;
  for (const [index, mount] of selected.entries()) {
    if (
      !validPath(mount.guestPath) ||
      !validPath(mount.editor.path) ||
      mount.editor.query ||
      mount.editor.fragment ||
      selected.some(
        (other, otherIndex) =>
          otherIndex !== index &&
          (other.guestPath === mount.guestPath ||
            (other.editor.scheme === mount.editor.scheme &&
              other.editor.authority === mount.editor.authority &&
              components(other.editor.path).join("/") === components(mount.editor.path).join("/"))),
      )
    ) {
      throw new Error("WASI mount allocation is ambiguous or unavailable");
    }
  }
  const current = (): void => {
    if (!isCurrent()) throw new Error("WASI mount session is no longer current");
  };
  return {
    toGuest(value) {
      current();
      if (!validPath(value.path) || value.query || value.fragment) {
        if (value.scheme !== "file") return value.toString();
        throw new Error("WASI document URI has unsupported path geometry");
      }
      const candidates = selected
        .flatMap((mount) => {
          if (value.scheme !== mount.editor.scheme || value.authority !== mount.editor.authority)
            return [];
          const tail = suffix(value.path, mount.editor.path);
          return tail ? [{ mount, tail }] : [];
        })
        .sort(
          (a, b) => components(b.mount.editor.path).length - components(a.mount.editor.path).length,
        );
      const found = candidates[0];
      if (!found) {
        if (value.scheme !== "file") return value.toString();
        throw new Error("Document is outside the current WASI mounts");
      }
      const guest = join(found.mount.guestPath, found.tail);
      let encoded = "";
      const bytes = new TextEncoder().encode(guest);
      if (new TextDecoder("utf-8", { fatal: true }).decode(bytes) !== guest) {
        throw new Error("WASI guest path is not lossless Unicode");
      }
      for (const byte of bytes) encoded += fileUriPathPolicy[byte];
      current();
      return `file://${encoded}`;
    },
    toEditor(value) {
      current();
      const parsed = parse(value);
      if (parsed.scheme !== "file") return parsed;
      if (parsed.authority || parsed.query || parsed.fragment || !validPath(parsed.path)) {
        throw new Error("Guest URI has unsupported path geometry");
      }
      // Only the Core producer's exact forward spelling is admitted. Parsing
      // cannot donate encoded separators or another spelling to a mount.
      let encoded = "";
      const bytes = new TextEncoder().encode(parsed.path);
      if (new TextDecoder("utf-8", { fatal: true }).decode(bytes) !== parsed.path) {
        throw new Error("Guest URI path is not lossless Unicode");
      }
      for (const byte of bytes) encoded += fileUriPathPolicy[byte];
      if (value !== `file://${encoded}`)
        throw new Error("Guest URI is not a selected forward spelling");
      const candidates = selected
        .flatMap((mount) => {
          const tail = suffix(parsed.path, mount.guestPath);
          return tail ? [{ mount, tail }] : [];
        })
        .sort(
          (a, b) => components(b.mount.guestPath).length - components(a.mount.guestPath).length,
        );
      const found = candidates[0];
      if (!found) throw new Error("Guest URI is outside the current WASI mounts");
      const result = found.mount.editor.with({ path: join(found.mount.editor.path, found.tail) });
      current();
      return result;
    },
  };
}
