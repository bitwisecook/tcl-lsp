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
  guestMountPoint,
  RUST_SERVER_EXE,
  resolveRustServer,
  WASI_MODULE_RELATIVE_PATH,
  wasiRuntimeAction,
  wasiUriMapping,
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

/**
 * How the server spells a `file:` URI for a guest path it found itself:
 * `ls_types`' `from_file_path` rule, which `uri_norm.rs`'s
 * `encode_path_segment_bytes` mirrors — keep `A-Za-z0-9-._~` and `/`, escape
 * every other UTF-8 byte. Restated here deliberately: it is the contract the
 * guest prefix has to meet, so deriving it from the code under test would
 * prove nothing.
 */
function serverFileUri(guestPath: string): string {
  const encoded = [...new TextEncoder().encode(guestPath)]
    .map((byte) => {
      const char = String.fromCharCode(byte);
      return /[A-Za-z0-9\-._~/]/.test(char)
        ? char
        : `%${byte.toString(16).toUpperCase().padStart(2, "0")}`;
    })
    .join("");
  return `file://${encoded}`;
}

suite("WASI guest URI mapping", () => {
  const proj = { uri: "file:///home/me/proj", name: "proj" };
  const other = { uri: "file:///home/me/other", name: "other" };

  test("a single-root window mounts at /workspace, both ways", () => {
    const map = wasiUriMapping([proj]);
    assert.ok(map);
    assert.strictEqual(
      map.toGuest("file:///home/me/proj/src/a.tcl"),
      "file:///workspace/src/a.tcl",
    );
    assert.strictEqual(
      map.toEditor("file:///workspace/src/a.tcl"),
      "file:///home/me/proj/src/a.tcl",
    );
    // The folder root itself, not just files under it.
    assert.strictEqual(map.toGuest("file:///home/me/proj"), "file:///workspace");
    assert.strictEqual(map.toEditor("file:///workspace"), "file:///home/me/proj");
  });

  test("a multi-root window mounts at /workspaces/<name> — plural", () => {
    // The whole reason we do not use @vscode/wasm-wasi-lsp's converters:
    // 0.1.0-pre.9 maps this to the singular `file:///workspace/proj`, which is
    // not where @vscode/wasm-wasi@1.0.2's host mounts it.
    const map = wasiUriMapping([proj, other]);
    assert.ok(map);
    assert.strictEqual(
      map.toGuest("file:///home/me/proj/src/a.tcl"),
      "file:///workspaces/proj/src/a.tcl",
    );
    assert.strictEqual(
      map.toGuest("file:///home/me/other/b.tcl"),
      "file:///workspaces/other/b.tcl",
    );
    assert.strictEqual(
      map.toEditor("file:///workspaces/proj/src/a.tcl"),
      "file:///home/me/proj/src/a.tcl",
    );
    assert.strictEqual(
      map.toEditor("file:///workspaces/other/b.tcl"),
      "file:///home/me/other/b.tcl",
    );
  });

  test("every mapped URI round-trips", () => {
    for (const folders of [[proj], [proj, other]]) {
      const map = wasiUriMapping(folders);
      assert.ok(map);
      for (const folder of folders) {
        const uri = `${folder.uri}/deep/nested/file.tcl`;
        assert.strictEqual(map.toEditor(map.toGuest(uri)), uri, `round-trip ${uri}`);
      }
    }
  });

  test("a sibling whose path merely starts the same is not rewritten", () => {
    // `file:///home/me/proj2` starts with `file:///home/me/proj`. A plain
    // `startsWith` (what upstream does) would rewrite it into the guest and
    // hand the server a path nothing is mounted at.
    const map = wasiUriMapping([proj]);
    assert.ok(map);
    assert.strictEqual(map.toGuest("file:///home/me/proj2/a.tcl"), "file:///home/me/proj2/a.tcl");
  });

  test("a nested folder wins over its parent", () => {
    const inner = { uri: "file:///home/me/proj/vendor", name: "vendor" };
    const map = wasiUriMapping([proj, inner]);
    assert.ok(map);
    assert.strictEqual(
      map.toGuest("file:///home/me/proj/vendor/x.tcl"),
      "file:///workspaces/vendor/x.tcl",
    );
    assert.strictEqual(
      map.toGuest("file:///home/me/proj/src/x.tcl"),
      "file:///workspaces/proj/src/x.tcl",
    );
  });

  test("an unmapped URI and a folderless window pass through untouched", () => {
    const map = wasiUriMapping([proj]);
    assert.ok(map);
    assert.strictEqual(map.toGuest("untitled:Untitled-1"), "untitled:Untitled-1");
    assert.strictEqual(map.toEditor("file:///elsewhere/x.tcl"), "file:///elsewhere/x.tcl");
    // No folders: nothing is mounted, so there is nothing to convert.
    assert.strictEqual(wasiUriMapping([]), undefined);
  });

  test("a trailing slash on a folder URI does not double up", () => {
    const map = wasiUriMapping([{ uri: "file:///home/me/proj/", name: "proj" }]);
    assert.ok(map);
    assert.strictEqual(map.toGuest("file:///home/me/proj/a.tcl"), "file:///workspace/a.tcl");
  });

  test("the mount points match what the host documents", () => {
    assert.strictEqual(guestMountPoint(proj, false), "/workspace");
    assert.strictEqual(guestMountPoint(proj, true), "/workspaces/proj");
  });

  // A folder name is user data: a space, `#`, `?`, `%`, and non-ASCII all
  // reach the mount point as themselves. The mount stays raw — it is a
  // filesystem path — while the URI naming it must be percent-encoded, or it
  // either fails to parse (space) or parses as something else entirely (`#`
  // becomes a fragment, `?` a query, `%20` a decoded escape). This is the
  // aliasing class fixed in `rooted_file_uri` (c0aa8a25), one layer up.
  const hostileNames: Array<[string, string]> = [
    ["My Project", "file:///workspaces/My%20Project"],
    ["foo#bar", "file:///workspaces/foo%23bar"],
    ["foo?bar", "file:///workspaces/foo%3Fbar"],
    ["a%20b", "file:///workspaces/a%2520b"],
    ["café", "file:///workspaces/caf%C3%A9"],
    ["a&b=c;d", "file:///workspaces/a%26b%3Dc%3Bd"],
  ];

  test("a folder name with URI-significant characters is encoded in the guest URI", () => {
    for (const [name, guestRoot] of hostileNames) {
      const folder = { uri: `file:///home/me/${encodeURIComponent(name)}`, name };
      const map = wasiUriMapping([folder, other]);
      assert.ok(map);
      assert.strictEqual(
        map.toGuest(`${folder.uri}/src/a.tcl`),
        `${guestRoot}/src/a.tcl`,
        `guest URI for ${name}`,
      );
    }
  });

  test("a hostile folder name round-trips, including the server's own spelling", () => {
    for (const [name] of hostileNames) {
      const folder = { uri: `file:///home/me/${encodeURIComponent(name)}`, name };
      const map = wasiUriMapping([folder, other]);
      assert.ok(map);
      for (const uri of [folder.uri, `${folder.uri}/deep/nested/file.tcl`]) {
        assert.strictEqual(map.toEditor(map.toGuest(uri)), uri, `round-trip ${uri}`);
      }
      // Our own round trip is symmetric string surgery, so it holds even when
      // the guest spelling is wrong. The half that does not is the reply: the
      // server names a file it scanned under the mount with its own encoding
      // of the guest path, and `toEditor` has to recognise that string.
      const reply = serverFileUri(`${guestMountPoint(folder, true)}/deep/nested/file.tcl`);
      assert.strictEqual(
        map.toEditor(reply),
        `${folder.uri}/deep/nested/file.tcl`,
        `server reply ${reply}`,
      );
    }
  });

  test("the guest URI parses, and its path is the raw mount point", () => {
    for (const [name] of hostileNames) {
      const folder = { uri: `file:///home/me/${encodeURIComponent(name)}`, name };
      const map = wasiUriMapping([folder, other]);
      assert.ok(map);
      const guest = map.toGuest(folder.uri);
      // A URL the platform parser accepts, with nothing spilled into a query
      // or a fragment — and whose decoded path is exactly the filesystem path
      // the host mounted, which is what makes the server open the right file.
      const parsed = new URL(guest);
      assert.strictEqual(parsed.protocol, "file:");
      assert.strictEqual(parsed.search, "", `${name} leaked a query`);
      assert.strictEqual(parsed.hash, "", `${name} leaked a fragment`);
      assert.strictEqual(
        decodeURIComponent(parsed.pathname),
        guestMountPoint(folder, true),
        `decoded guest path for ${name}`,
      );
    }
  });

  test("an ordinary folder name is left exactly as it was", () => {
    // The encoding must be a byte-for-byte no-op on a name that needs none,
    // or every existing mapping would move.
    const map = wasiUriMapping([proj, other]);
    assert.ok(map);
    assert.strictEqual(map.toGuest("file:///home/me/proj/a.tcl"), "file:///workspaces/proj/a.tcl");
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
