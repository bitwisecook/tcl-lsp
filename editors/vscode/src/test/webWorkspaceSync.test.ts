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
 * The browser host's workspace sweep derives its globs from the extension's own
 * manifest rather than restating the server's extension set, and keeps the
 * store honest afterwards. These tests run on the desktop — the derivation is
 * pure, the live path needs only a filesystem, and the web smoke test can tell
 * neither a subtly-narrow glob from a correct one (both produce *some* files)
 * nor a stale store entry from a fresh one.
 */

import * as assert from "assert";
import * as fs from "fs";
import * as os from "os";
import * as path from "path";
import * as vscode from "vscode";
import * as vm from "vm";
import { globSync } from "glob";
import {
  deriveSyncGlobs,
  DEFAULT_BUDGET,
  SourceStoreHost,
  SyncManifest,
  WorkspaceStoreSync,
  WorkspaceSyncBudget,
  workerSourceStoreHost,
} from "../webWorkspaceSync";

/** `FileChangeType` on the wire — the protocol's numbering, not `vscode.FileChangeType`'s. */
const FileChange = { Created: 1, Changed: 2, Deleted: 3 } as const;

/** What the worker would have been told, in the order it would have heard it. */
interface StoreCall {
  kind: "upsert" | "delete";
  uri: string;
  text?: string;
}

function recordingHost(
  calls: StoreCall[],
  byteCalls: Uint8Array[] = [],
  inventoryCalls: Array<{
    rootUri: string;
    selection: string;
    status: "complete" | "incomplete";
  }> = [],
): SourceStoreHost {
  return {
    createDirectory: async () => true,
    upsert: async (uri, text, originalBytes) => {
      calls.push({ kind: "upsert", uri, text });
      byteCalls.push(originalBytes.slice());
      return true;
    },
    delete: (uri) => calls.push({ kind: "delete", uri }),
    setSourceInventory: (rootUri, selection, status) =>
      inventoryCalls.push({ rootUri, selection, status }),
    isSafeSpecPackName: async (name) => !name.startsWith("../") && !name.startsWith("/"),
    upsertSpecPack: () => {},
  };
}

type PushFile = (
  uri: vscode.Uri,
  changeType: number,
  budget: WorkspaceSyncBudget,
  notify: (changes: Array<{ uri: string; type: number }>) => void,
  scanOwner?: { rootUri: string; generation: number },
) => Promise<boolean>;

/**
 * The live path's one entry point, which `watch()` wires to every create and
 * change event. Reached directly because a `FileSystemWatcher` event cannot be
 * delivered on demand, and this is the seam the watcher itself uses.
 */
function pushFileOf(sync: WorkspaceStoreSync): PushFile {
  return (sync as unknown as { pushFile: PushFile }).pushFile.bind(sync);
}

function tempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "tcl-lsp-web-sync-"));
}

suite("Web workspace sync", () => {
  test("derives the source glob from the generated workspaceContains activation event", () => {
    const globs = deriveSyncGlobs({
      activationEvents: ["onLanguage:tcl", "workspaceContains:**/*.{[tT][cC][lL]}"],
    });
    assert.ok(
      globs.includes("**/*.{[tT][cC][lL]}"),
      `expected the activation glob verbatim, got ${globs.join(" ")}`,
    );
  });

  test("adds the whole-basename language registrations", () => {
    const globs = deriveSyncGlobs({
      activationEvents: ["workspaceContains:**/*.{tcl}"],
      contributes: {
        languages: [{ filenames: ["bigip.conf", "presentation"] }],
      },
    });
    assert.ok(globs.includes("**/bigip.conf"));
    assert.ok(globs.includes("**/presentation"));
  });

  test("always covers the project config files and sidecar stubs", () => {
    const globs = deriveSyncGlobs({});
    assert.ok(globs.includes("**/.tcl-lsp.ini"));
    assert.ok(globs.includes("**/tcl-lsp/config.ini"));
    assert.ok(globs.includes("**/*.tcl.stubs"));
  });

  test("falls back to the registered extensions when no activation glob exists", () => {
    const globs = deriveSyncGlobs({
      contributes: { languages: [{ extensions: [".tcl", ".irule"] }] },
    });
    assert.ok(
      globs.some((glob) => glob.includes("irule") && glob.includes("tcl")),
      `expected an extension glob, got ${globs.join(" ")}`,
    );
  });

  test("the real manifest yields the server's own case-folded source glob", () => {
    const extension = vscode.extensions.getExtension("bitwisecook.tcl-lsp");
    assert.ok(extension, "the extension under test is not installed");
    const globs = deriveSyncGlobs(extension.packageJSON as SyncManifest);
    // Case-folded per character, which is what makes the sweep
    // pick up `UPPER.TCL` on Linux — the same reason the server's watcher
    // registration folds it.
    const source = globs.find((glob) => glob.startsWith("**/*.{"));
    assert.ok(source, `no source glob derived: ${globs.join(" ")}`);
    assert.ok(source.includes("[tT][cC][lL]"), `the source glob is not case-folded: ${source}`);
    assert.ok(
      source.includes("[tT][cC][lL][sS][pP][eE][cC]"),
      `the source glob omits .tclspec packs: ${source}`,
    );
    assert.ok(globs.includes("**/bigip.conf"), "the BIG-IP filenames are missing");
  });

  test("the shipped budget is finite and per-file bounded", () => {
    assert.ok(DEFAULT_BUDGET.maxFiles > 0 && Number.isFinite(DEFAULT_BUDGET.maxFiles));
    assert.ok(DEFAULT_BUDGET.maxFileBytes < DEFAULT_BUDGET.maxTotalBytes);
  });

  test("sweeps a classic autoloader's tclIndex, in either case", () => {
    // `tclIndex` carries no source extension and is not one of the manifest's
    // `filenames` registrations, so no derived glob reaches it — yet it is
    // what `PackageResolver::scan_single_dir` builds `auto_index` from. Absent
    // from the store, every autoloaded command is a false W123 with no
    // definition to go to.
    const globs = deriveSyncGlobs({
      activationEvents: ["workspaceContains:**/*.{[tT][cC][lL]}"],
    });
    assert.ok(
      globs.includes("**/[tT][cC][lL][iI][nN][dD][eE][xX]"),
      `no tclIndex glob derived: ${globs.join(" ")}`,
    );

    // …and it has to actually match, in both spellings the server accepts
    // (`eq_ignore_ascii_case`) — the source glob is case-folded per character
    // for the same reason.
    const root = tempDir();
    try {
      fs.mkdirSync(path.join(root, "lib"));
      fs.mkdirSync(path.join(root, "vendor"));
      fs.writeFileSync(path.join(root, "lib", "tclIndex"), "set auto_index(greet) {}\n");
      fs.writeFileSync(path.join(root, "vendor", "TCLINDEX"), "set auto_index(vend) {}\n");
      const matched = new Set(globs.flatMap((glob) => globSync(glob, { cwd: root })));
      assert.ok(matched.has(path.join("lib", "tclIndex")), `not swept: ${[...matched].join(" ")}`);
      assert.ok(
        matched.has(path.join("vendor", "TCLINDEX")),
        `upper-case not swept: ${[...matched].join(" ")}`,
      );
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });
});

type Notify = (changes: Array<{ uri: string; type: number }>) => void;

/** The watcher's delete arm, reached the same way as its create/change arm. */
type HandleDelete = (uri: vscode.Uri, notify: Notify) => void;

function handleDeleteOf(sync: WorkspaceStoreSync): HandleDelete {
  return (sync as unknown as { handleDelete: HandleDelete }).handleDelete.bind(sync);
}

type PrimeFolders = (
  folders: readonly vscode.WorkspaceFolder[],
  budget: WorkspaceSyncBudget,
  notify: Notify,
) => Promise<void>;

function primeFoldersOf(sync: WorkspaceStoreSync): PrimeFolders {
  return (sync as unknown as { primeFolders: PrimeFolders }).primeFolders.bind(sync);
}

function productionStoreWorker(server: Record<string, unknown>): Worker {
  const source = fs.readFileSync(
    path.resolve(__dirname, "../../../../rust/tcl-lsp-server-wasm/worker.js"),
    "utf8",
  );
  const start = source.indexOf("function dispatch(data) {");
  const end = source.indexOf("\nself.onmessage =", start);
  assert.ok(start >= 0 && end > start, "the production worker dispatcher is unavailable");
  const dispatch = vm.runInNewContext(`${source.slice(start, end)}; dispatch`, {
    server,
    Uint8Array,
  }) as (message: unknown) => void;
  return { postMessage: dispatch } as unknown as Worker;
}

suite("Web workspace sync — directory admission", () => {
  test("admits a real empty directory before completing its empty source census", async () => {
    const root = tempDir();
    const uri = vscode.Uri.file(root);
    const order: string[] = [];
    const calls: StoreCall[] = [];
    const host: SourceStoreHost = {
      ...recordingHost(calls),
      createDirectory: async (original) => {
        assert.strictEqual(original, uri.toString());
        assert.ok(fs.statSync(root).isDirectory());
        assert.strictEqual(fs.readdirSync(root).length, 0);
        order.push("directory");
        return true;
      },
      setSourceInventory: (_root, _selection, status) => order.push(status),
    };
    try {
      const sync = new WorkspaceStoreSync(host, [], () => {});
      await primeFoldersOf(sync)([{ uri, name: "empty", index: 0 }], DEFAULT_BUDGET, () => {});
      assert.strictEqual(calls.length, 0, "directory admission must not invent source files");
      assert.ok(order.indexOf("directory") > order.indexOf("incomplete"));
      assert.ok(order.indexOf("complete") > order.indexOf("directory"));
      assert.strictEqual(order[order.length - 1], "complete");
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  test("a file root and a refused directory never complete the source census", async () => {
    const root = tempDir();
    const file = path.join(root, "not-a-directory");
    fs.writeFileSync(file, "source");
    try {
      for (const [uri, expectedRequests] of [
        [vscode.Uri.file(file), 0],
        [vscode.Uri.file(root), 1],
      ] as const) {
        const requests: string[] = [];
        const receipts: string[] = [];
        const host: SourceStoreHost = {
          ...recordingHost([]),
          createDirectory: async (original) => {
            requests.push(original);
            return false;
          },
          setSourceInventory: (_root, _selection, status) => receipts.push(status),
        };
        const sync = new WorkspaceStoreSync(host, [], () => {});
        await primeFoldersOf(sync)([{ uri, name: "refused", index: 0 }], DEFAULT_BUDGET, () => {});
        assert.strictEqual(requests.length, expectedRequests);
        assert.ok(receipts.length > 0);
        assert.ok(receipts.every((status) => status === "incomplete"));
      }
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  test("a late accepted directory cannot complete a newer refused root census", async () => {
    const root = tempDir();
    const uri = vscode.Uri.file(root);
    let settle!: (accepted: boolean) => void;
    const pending = new Promise<boolean>((resolve) => {
      settle = resolve;
    });
    let requests = 0;
    const receipts: string[] = [];
    const host: SourceStoreHost = {
      ...recordingHost([]),
      createDirectory: () => {
        requests += 1;
        return requests === 1 ? pending : Promise.resolve(false);
      },
      setSourceInventory: (_root, _selection, status) => receipts.push(status),
    };
    try {
      const sync = new WorkspaceStoreSync(host, [], () => {});
      const folder = { uri, name: "empty", index: 0 };
      const old = primeFoldersOf(sync)([folder], DEFAULT_BUDGET, () => {});
      for (let attempt = 0; attempt < 200 && requests === 0; attempt += 1) {
        await new Promise((resolve) => setTimeout(resolve, 5));
      }
      assert.strictEqual(requests, 1, "first directory request did not reach the worker");
      await primeFoldersOf(sync)([folder], DEFAULT_BUDGET, () => {});
      settle(true);
      await old;
      assert.strictEqual(receipts[receipts.length - 1], "incomplete");
      assert.ok(!receipts.includes("complete"));
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  test("a real worker file refusal never closes an optimistic source census", async () => {
    const root = tempDir();
    const uri = vscode.Uri.file(root);
    const receipts: string[] = [];
    const refused: string[] = [];
    const transport = workerSourceStoreHost(
      productionStoreWorker({
        vfs_upsert_bytes(original: string) {
          refused.push(original);
          return false;
        },
      }),
    );
    const host: SourceStoreHost = {
      ...recordingHost([]),
      upsert: transport.upsert,
      setSourceInventory: (_root, _selection, status) => receipts.push(status),
    };
    try {
      const sync = new WorkspaceStoreSync(host, [], () => {});
      await primeFoldersOf(sync)([{ uri, name: "empty", index: 0 }], DEFAULT_BUDGET, () => {});
      receipts.length = 0;
      const file = path.join(root, "refused.tcl");
      fs.writeFileSync(file, "set held 1\n");
      assert.strictEqual(
        await pushFileOf(sync)(vscode.Uri.file(file), FileChange.Created, DEFAULT_BUDGET, () => {}),
        false,
      );
      assert.deepStrictEqual(refused, [vscode.Uri.file(file).toString()]);
      assert.ok(receipts.length > 0 && receipts.every((status) => status === "incomplete"));
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  test("pending file admissions retain the file budget and incomplete census", async () => {
    const root = tempDir();
    const uri = vscode.Uri.file(root);
    let accept!: (admitted: boolean) => void;
    const pending = new Promise<boolean>((resolve) => {
      accept = resolve;
    });
    const writes: string[] = [];
    const receipts: string[] = [];
    const host: SourceStoreHost = {
      ...recordingHost([]),
      upsert: async (original) => {
        writes.push(original);
        return pending;
      },
      setSourceInventory: (_root, _selection, status) => receipts.push(status),
    };
    try {
      const sync = new WorkspaceStoreSync(host, [], () => {});
      await primeFoldersOf(sync)([{ uri, name: "empty", index: 0 }], DEFAULT_BUDGET, () => {});
      receipts.length = 0;
      const a = path.join(root, "a.tcl"),
        b = path.join(root, "b.tcl");
      fs.writeFileSync(a, "set a 1\n");
      fs.writeFileSync(b, "set b 1\n");
      const budget = { ...DEFAULT_BUDGET, maxFiles: 1 };
      const first = pushFileOf(sync)(vscode.Uri.file(a), FileChange.Created, budget, () => {});
      for (let i = 0; i < 200 && writes.length === 0; i += 1)
        await new Promise((resolve) => setTimeout(resolve, 5));
      assert.strictEqual(writes.length, 1);
      const second = pushFileOf(sync)(vscode.Uri.file(b), FileChange.Created, budget, () => {});
      await new Promise((resolve) => setTimeout(resolve, 20));
      assert.strictEqual(
        writes.length,
        1,
        "the second file cannot race an unacknowledged budget reservation",
      );
      assert.ok(receipts.every((status) => status === "incomplete"));
      accept(true);
      assert.strictEqual(await first, true);
      assert.strictEqual(await second, false);
      assert.strictEqual(writes.length, 1);
      assert.strictEqual(receipts[receipts.length - 1], "incomplete");
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  test("a stale accepted file is withdrawn before its successor is posted", async () => {
    const root = tempDir();
    const uri = vscode.Uri.file(root);
    let accept!: (admitted: boolean) => void;
    const pending = new Promise<boolean>((resolve) => {
      accept = resolve;
    });
    const order: string[] = [];
    const receipts: string[] = [];
    const host: SourceStoreHost = {
      ...recordingHost([]),
      upsert: async (_original, text) => {
        order.push(text);
        return order.length === 1 ? pending : true;
      },
      delete: () => order.push("delete"),
      setSourceInventory: (_root, _selection, status) => receipts.push(status),
    };
    try {
      const sync = new WorkspaceStoreSync(host, [], () => {});
      await primeFoldersOf(sync)([{ uri, name: "empty", index: 0 }], DEFAULT_BUDGET, () => {});
      receipts.length = 0;
      const file = path.join(root, "changing.tcl");
      fs.writeFileSync(file, "set version 1\n");
      const first = pushFileOf(sync)(
        vscode.Uri.file(file),
        FileChange.Changed,
        DEFAULT_BUDGET,
        () => {},
      );
      for (let i = 0; i < 200 && order.length === 0; i += 1) {
        await new Promise((resolve) => setTimeout(resolve, 5));
      }
      assert.strictEqual(order.length, 1);
      fs.writeFileSync(file, "set version 2\n");
      const second = pushFileOf(sync)(
        vscode.Uri.file(file),
        FileChange.Changed,
        DEFAULT_BUDGET,
        () => {},
      );
      await new Promise((resolve) => setTimeout(resolve, 20));
      assert.strictEqual(order.length, 1);
      assert.ok(receipts.every((status) => status === "incomplete"));
      accept(true);
      assert.strictEqual(await first, false);
      assert.strictEqual(await second, true);
      assert.deepStrictEqual(order, ["set version 1\n", "delete", "set version 2\n"]);
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  test("a lost file acknowledgement leaves no orphan for a later empty census", async () => {
    const root = tempDir();
    const uri = vscode.Uri.file(root);
    const file = path.join(root, "removed.tcl");
    const fileUri = vscode.Uri.file(file);
    const held = new Set<string>();
    const receipts: string[] = [];
    const host: SourceStoreHost = {
      ...recordingHost([]),
      upsert: async (original) => {
        held.add(original);
        throw new Error("acknowledgement lost after worker commit");
      },
      delete: (original) => {
        held.delete(original);
      },
      setSourceInventory: (_root, _selection, status) => receipts.push(status),
    };
    try {
      const sync = new WorkspaceStoreSync(host, [], () => {});
      const folder = { uri, name: "empty", index: 0 };
      await primeFoldersOf(sync)([folder], DEFAULT_BUDGET, () => {});
      receipts.length = 0;
      fs.writeFileSync(file, "set removed 1\n");
      // Startup and watchers share this admission boundary; exercise it without
      // the watcher's additional withdrawal hiding a missing startup cleanup.
      const admission = sync as unknown as {
        upsertTracked(
          uri: vscode.Uri,
          text: string,
          bytes: Uint8Array,
          byteLength: number,
          budget: WorkspaceSyncBudget,
          current: () => boolean,
        ): Promise<string>;
      };
      const bytes = new TextEncoder().encode("set removed 1\n");
      assert.strictEqual(
        await admission.upsertTracked(
          fileUri,
          "set removed 1\n",
          bytes,
          bytes.byteLength,
          DEFAULT_BUDGET,
          () => true,
        ),
        "refused",
      );
      assert.strictEqual(held.size, 0, "a committed but unacknowledged file must be withdrawn");
      assert.ok(receipts.every((status) => status === "incomplete"));
      fs.unlinkSync(file);
      await primeFoldersOf(sync)([folder], DEFAULT_BUDGET, () => {});
      assert.strictEqual(receipts[receipts.length - 1], "complete");
      assert.strictEqual(held.size, 0, "the empty rescan must not certify an orphaned file");
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  test("a live refusal cannot delete its queued admitted successor", async () => {
    const root = tempDir();
    const file = path.join(root, "changing.tcl");
    const uri = vscode.Uri.file(file);
    const held = new Map<string, string>();
    let refuse = false;
    let successor: Promise<string> | undefined;
    let sync!: WorkspaceStoreSync;
    const host: SourceStoreHost = {
      ...recordingHost([]),
      upsert: async (original, text) => {
        if (refuse) return false;
        held.set(original, text);
        return true;
      },
      delete: (original) => {
        held.delete(original);
      },
    };
    try {
      sync = new WorkspaceStoreSync(host, [], () => {});
      fs.writeFileSync(file, "set version 1\n");
      assert.strictEqual(
        await pushFileOf(sync)(uri, FileChange.Created, DEFAULT_BUDGET, () => {}),
        true,
      );
      refuse = true;
      fs.writeFileSync(file, "set version 2\n");
      const refused = await pushFileOf(sync)(uri, FileChange.Changed, DEFAULT_BUDGET, (changes) => {
        assert.strictEqual(changes[0]?.type, FileChange.Deleted);
        refuse = false;
        const bytes = new TextEncoder().encode("set version 3\n");
        // Queue at the shared admission boundary while the older live handler
        // is suspended, so a second outer withdrawal would remove this write.
        const admission = sync as unknown as {
          upsertTracked(
            uri: vscode.Uri,
            text: string,
            bytes: Uint8Array,
            byteLength: number,
            budget: WorkspaceSyncBudget,
            current: () => boolean,
          ): Promise<string>;
        };
        successor = admission.upsertTracked(
          uri,
          "set version 3\n",
          bytes,
          bytes.byteLength,
          DEFAULT_BUDGET,
          () => true,
        );
      });
      assert.strictEqual(refused, false);
      assert.ok(successor, "the withdrawal must notify once for the held file");
      assert.strictEqual(await successor, "admitted");
      assert.strictEqual(held.get(uri.toString()), "set version 3\n");
    } finally {
      sync?.dispose();
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  test("a throwing withdrawal notification cannot republish a complete census", async () => {
    const root = tempDir();
    const uri = vscode.Uri.file(root);
    const file = path.join(root, "refused.tcl");
    const receipts: string[] = [];
    let refuse = false;
    const host: SourceStoreHost = {
      ...recordingHost([]),
      upsert: async () => !refuse,
      setSourceInventory: (_root, _selection, status) => receipts.push(status),
    };
    try {
      const sync = new WorkspaceStoreSync(host, [], () => {});
      await primeFoldersOf(sync)([{ uri, name: "empty", index: 0 }], DEFAULT_BUDGET, () => {});
      fs.writeFileSync(file, "set held 1\n");
      assert.strictEqual(
        await pushFileOf(sync)(vscode.Uri.file(file), FileChange.Created, DEFAULT_BUDGET, () => {}),
        true,
      );
      receipts.length = 0;
      refuse = true;
      await assert.rejects(
        pushFileOf(sync)(vscode.Uri.file(file), FileChange.Changed, DEFAULT_BUDGET, () => {
          throw new Error("notification failed");
        }),
        /notification failed/,
      );
      assert.ok(receipts.length > 0);
      assert.ok(receipts.every((status) => status === "incomplete"));
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  test("a failed stale withdrawal blocks later rescans until a fresh store is used", async () => {
    const root = tempDir();
    const uri = vscode.Uri.file(root);
    const file = path.join(root, "removed.tcl");
    const fileUri = vscode.Uri.file(file);
    const held = new Set<string>();
    const receipts: string[] = [];
    let current = true;
    let failDelete = true;
    let accept!: (accepted: boolean) => void;
    const pending = new Promise<boolean>((resolve) => {
      accept = resolve;
    });
    const host: SourceStoreHost = {
      ...recordingHost([]),
      upsert: async (original) => {
        held.add(original);
        return pending;
      },
      delete: (original) => {
        if (failDelete) throw new Error("worker delete post failed");
        held.delete(original);
      },
      setSourceInventory: (_root, _selection, status) => receipts.push(status),
    };
    try {
      const sync = new WorkspaceStoreSync(host, [], () => {});
      const folder = { uri, name: "empty", index: 0 };
      await primeFoldersOf(sync)([folder], DEFAULT_BUDGET, () => {});
      receipts.length = 0;
      fs.writeFileSync(file, "set removed 1\n");
      const admission = sync as unknown as {
        upsertTracked(
          uri: vscode.Uri,
          text: string,
          bytes: Uint8Array,
          byteLength: number,
          budget: WorkspaceSyncBudget,
          current: () => boolean,
        ): Promise<string>;
      };
      const bytes = new TextEncoder().encode("set removed 1\n");
      const writing = admission.upsertTracked(
        fileUri,
        "set removed 1\n",
        bytes,
        bytes.byteLength,
        DEFAULT_BUDGET,
        () => current,
      );
      for (let i = 0; i < 200 && held.size === 0; i += 1) {
        await new Promise((resolve) => setTimeout(resolve, 5));
      }
      assert.strictEqual(held.size, 1);
      current = false;
      accept(true);
      await assert.rejects(writing, /worker delete post failed/);
      fs.unlinkSync(file);
      await primeFoldersOf(sync)([folder], DEFAULT_BUDGET, () => {});
      await primeFoldersOf(sync)([folder], DEFAULT_BUDGET, () => {});
      assert.strictEqual(held.size, 1);
      assert.ok(receipts.every((status) => status === "incomplete"));
      failDelete = false;
      await primeFoldersOf(sync)([folder], DEFAULT_BUDGET, () => {});
      assert.strictEqual(held.size, 1);
      assert.ok(
        receipts.every((status) => status === "incomplete"),
        "transport recovery and a rescan cannot renew authority over an old orphan",
      );
      const freshReceipts: Array<{
        rootUri: string;
        selection: string;
        status: "complete" | "incomplete";
      }> = [];
      const fresh = new WorkspaceStoreSync(recordingHost([], [], freshReceipts), [], () => {});
      await primeFoldersOf(fresh)([folder], DEFAULT_BUDGET, () => {});
      assert.strictEqual(freshReceipts[freshReceipts.length - 1]?.status, "complete");
      fresh.dispose();
    } finally {
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  test("worker directory transport retains the original URI and refused verdict", async () => {
    const requests: string[] = [];
    const worker = productionStoreWorker({
      vfs_create_directory(original: string) {
        requests.push(original);
        return original === "file:///empty";
      },
    });
    const host = workerSourceStoreHost(worker);
    assert.strictEqual(await host.createDirectory("file:///empty"), true);
    assert.strictEqual(await host.createDirectory("file:///empty/%00invalid"), false);
    assert.deepStrictEqual(requests, ["file:///empty", "file:///empty/%00invalid"]);
  });
});

/**
 * A filesystem provider whose reads finish when the test says so, and in
 * whatever order it says.
 *
 * The races under test are between two overlapping watcher handlers, which
 * needs reads that resolve out of the order they were started — something the
 * real filesystem will not do on demand. Registered on its own scheme, so
 * `vscode.workspace.fs.readFile` reaches it by exactly the route it reaches
 * `file:` by, with no part of the code under test aware of the difference.
 */
class DeferredFileSystem implements vscode.FileSystemProvider {
  private readonly reads: Array<(text: string) => void> = [];
  readonly onDidChangeFile = new vscode.EventEmitter<vscode.FileChangeEvent[]>().event;

  /** How many reads have reached the provider so far. */
  get started(): number {
    return this.reads.length;
  }

  /** Finish the `index`th read (in start order) with `text`. */
  settle(index: number, text: string): void {
    const resolve = this.reads[index];
    assert.ok(resolve, `no read started at index ${index}`);
    resolve(text);
  }

  readFile(): Promise<Uint8Array> {
    return new Promise<Uint8Array>((resolve) => {
      this.reads.push((text) => resolve(new TextEncoder().encode(text)));
    });
  }

  // Never reached: the sync layer reads, and nothing else.
  watch(): vscode.Disposable {
    return new vscode.Disposable(() => {});
  }
  stat(): vscode.FileStat {
    throw vscode.FileSystemError.FileNotFound();
  }
  readDirectory(): Array<[string, vscode.FileType]> {
    throw vscode.FileSystemError.FileNotFound();
  }
  createDirectory(): void {
    throw vscode.FileSystemError.NoPermissions();
  }
  writeFile(): void {
    throw vscode.FileSystemError.NoPermissions();
  }
  delete(): void {
    throw vscode.FileSystemError.NoPermissions();
  }
  rename(): void {
    throw vscode.FileSystemError.NoPermissions();
  }
}

/** Wait until `count` reads have reached the provider — i.e. every push is parked. */
async function waitForReads(provider: DeferredFileSystem, count: number): Promise<void> {
  for (let attempt = 0; attempt < 500 && provider.started < count; attempt += 1) {
    await new Promise((resolve) => setTimeout(resolve, 5));
  }
  assert.strictEqual(provider.started, count, "a read never reached the provider");
}

/**
 * The live watcher path, which keeps the store honest after the startup sweep.
 *
 * These run against the real filesystem through `vscode.workspace.fs`: the
 * refusal under test is a read that fails, and a file that is not there is
 * exactly how one fails.
 */
suite("Web workspace sync — spec-pack manifest paths", () => {
  test("worker host returns the WASM path owner's validation verdict", async () => {
    const requests: string[] = [];
    const worker = {
      postMessage(message: { tclLsp?: string; name?: string; replyPort?: MessagePort }) {
        if (message.tclLsp === "validateSpecPackName" && message.replyPort) {
          requests.push(message.name ?? "");
          message.replyPort.postMessage(message.name === "nested/資料.tclspec");
        }
      },
    } as unknown as Worker;
    const host = workerSourceStoreHost(worker);
    assert.strictEqual(await host.isSafeSpecPackName("nested/資料.tclspec"), true);
    assert.strictEqual(await host.isSafeSpecPackName("../資料.tclspec"), false);
    assert.deepStrictEqual(requests, ["nested/資料.tclspec", "../資料.tclspec"]);
  });

  test("validates original names through the host owner before joining or reading", async () => {
    const files = new Map<string, Uint8Array>([
      [
        "/bundle/specs/index.json",
        new TextEncoder().encode(JSON.stringify(["../outside.tclspec", "vendor/資料.tclspec"])),
      ],
      ["/bundle/outside.tclspec", new TextEncoder().encode("outside sentinel\n")],
      ["/bundle/specs/vendor/資料.tclspec", new TextEncoder().encode("speclib valid 1 {}\n")],
    ]);
    const reads: string[] = [];
    const provider: vscode.FileSystemProvider = {
      onDidChangeFile: new vscode.EventEmitter<vscode.FileChangeEvent[]>().event,
      readFile: (uri) => {
        reads.push(uri.path);
        const bytes = files.get(uri.path);
        if (!bytes) throw vscode.FileSystemError.FileNotFound(uri);
        return bytes;
      },
      watch: () => new vscode.Disposable(() => {}),
      stat: () => {
        throw vscode.FileSystemError.FileNotFound();
      },
      readDirectory: () => {
        throw vscode.FileSystemError.FileNotFound();
      },
      createDirectory: () => {
        throw vscode.FileSystemError.NoPermissions();
      },
      writeFile: () => {
        throw vscode.FileSystemError.NoPermissions();
      },
      delete: () => {
        throw vscode.FileSystemError.NoPermissions();
      },
      rename: () => {
        throw vscode.FileSystemError.NoPermissions();
      },
    };
    const registration = vscode.workspace.registerFileSystemProvider("lsp-pack-test", provider, {
      isCaseSensitive: true,
    });
    const accepted: string[] = [];
    const upserts: Array<{ name: string; text: string }> = [];
    const host: SourceStoreHost = {
      createDirectory: async () => true,
      upsert: async () => true,
      delete: () => {},
      setSourceInventory: () => {},
      isSafeSpecPackName: async (name) => {
        accepted.push(name);
        return name === "vendor/資料.tclspec";
      },
      upsertSpecPack: (name, text) => upserts.push({ name, text }),
    };
    try {
      const sync = new WorkspaceStoreSync(host, [], () => {});
      assert.strictEqual(
        await sync.primeSpecPacks(vscode.Uri.parse("lsp-pack-test:/bundle/specs")),
        1,
      );
      assert.deepStrictEqual(accepted, ["../outside.tclspec", "vendor/資料.tclspec"]);
      assert.deepStrictEqual(reads, [
        "/bundle/specs/index.json",
        "/bundle/specs/vendor/資料.tclspec",
      ]);
      assert.deepStrictEqual(upserts, [
        { name: "vendor/資料.tclspec", text: "speclib valid 1 {}\n" },
      ]);
    } finally {
      registration.dispose();
    }
  });
});

suite("Web workspace sync — live refusals", () => {
  test("a file that stops being readable is withdrawn, not left stale", async () => {
    const root = tempDir();
    const file = path.join(root, "lib.tcl");
    fs.writeFileSync(file, "proc greet {} {}\n");
    const uri = vscode.Uri.file(file);
    const calls: StoreCall[] = [];
    const notified: Array<{ uri: string; type: number }> = [];
    const notify = (changes: Array<{ uri: string; type: number }>) => notified.push(...changes);
    const sync = new WorkspaceStoreSync(recordingHost(calls), [], () => {});
    const pushFile = pushFileOf(sync);
    try {
      assert.strictEqual(
        await pushFile(uri, FileChange.Created, DEFAULT_BUDGET, notify),
        true,
        "the readable file should have been sent",
      );
      assert.deepStrictEqual(calls, [
        { kind: "upsert", uri: uri.toString(), text: "proc greet {} {}\n" },
      ]);

      // Now it cannot be read — a permissions change, a remote provider
      // dropping out, or (here) the file going away between the watcher event
      // and the read. The copy the store holds is no longer anything that
      // exists, so it has to go.
      fs.rmSync(file);
      calls.length = 0;
      notified.length = 0;
      assert.strictEqual(
        await pushFile(uri, FileChange.Changed, DEFAULT_BUDGET, notify),
        false,
        "an unreadable file cannot be reported as held",
      );
      assert.deepStrictEqual(
        calls,
        [{ kind: "delete", uri: uri.toString() }],
        "the held copy must be deleted from the store",
      );
      assert.deepStrictEqual(
        notified,
        [{ uri: uri.toString(), type: FileChange.Deleted }],
        "the server must be told the file is gone",
      );
    } finally {
      sync.dispose();
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  test("a live upsert retains the exact admitted source bytes", async () => {
    const root = tempDir();
    const file = path.join(root, "unicode.tcl");
    const source = "# café\nset value 1\n";
    const originalBytes = Buffer.from(source, "utf8");
    fs.writeFileSync(file, originalBytes);
    const uri = vscode.Uri.file(file);
    const calls: StoreCall[] = [];
    const byteCalls: Uint8Array[] = [];
    const sync = new WorkspaceStoreSync(recordingHost(calls, byteCalls), [], () => {});
    try {
      assert.strictEqual(
        await pushFileOf(sync)(uri, FileChange.Created, DEFAULT_BUDGET, () => {}),
        true,
      );
      assert.deepStrictEqual(Array.from(byteCalls[0] ?? []), Array.from(originalBytes));
      assert.strictEqual(calls[0]?.text, source);
    } finally {
      sync.dispose();
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  test("invalid UTF-8 is withdrawn and leaves its root inventory incomplete", async () => {
    const root = tempDir();
    const file = path.join(root, "invalid.tcl");
    fs.writeFileSync(file, Buffer.from([0x73, 0x65, 0x74, 0x20, 0xff, 0x0a]));
    const uri = vscode.Uri.file(file);
    const calls: StoreCall[] = [];
    const inventoryCalls: Array<{
      rootUri: string;
      selection: string;
      status: "complete" | "incomplete";
    }> = [];
    const sync = new WorkspaceStoreSync(recordingHost(calls, [], inventoryCalls), [], () => {});
    // Model the active root state created by the startup sweep; the private
    // live path is exercised through the same host protocol as a watcher.
    (
      sync as unknown as {
        sourceInventories: Map<string, { rootUri: string; selection: string; complete: boolean }>;
      }
    ).sourceInventories.set(root, {
      rootUri: root,
      selection: "selection-v1",
      complete: true,
    });
    try {
      assert.strictEqual(
        await pushFileOf(sync)(uri, FileChange.Created, DEFAULT_BUDGET, () => {}),
        false,
      );
      assert.deepStrictEqual(inventoryCalls, [
        { rootUri: root, selection: "selection-v1", status: "incomplete" },
      ]);
      assert.deepStrictEqual(calls, [{ kind: "delete", uri: uri.toString() }]);
    } finally {
      sync.dispose();
      fs.rmSync(root, { recursive: true, force: true });
    }
  });

  test("a file the store never held is refused without a spurious deletion", async () => {
    const root = tempDir();
    const uri = vscode.Uri.file(path.join(root, "never-existed.tcl"));
    const calls: StoreCall[] = [];
    const notified: Array<{ uri: string; type: number }> = [];
    const sync = new WorkspaceStoreSync(recordingHost(calls), [], () => {});
    try {
      assert.strictEqual(
        await pushFileOf(sync)(uri, FileChange.Created, DEFAULT_BUDGET, (changes) =>
          notified.push(...changes),
        ),
        false,
      );
      assert.deepStrictEqual(notified, [], "nothing was held, so nothing changed");
    } finally {
      sync.dispose();
      fs.rmSync(root, { recursive: true, force: true });
    }
  });
});

/**
 * Two events for one file, overlapping.
 *
 * The watcher hands each event to a handler and does not wait for it, so with
 * a slow provider the reads finish in an order the events did not.
 */
suite("Web workspace sync — per-root census ownership", () => {
  test("only the current scan can close its root, independently of other roots", () => {
    type Inventory = {
      rootUri: string;
      selection: string;
      complete: boolean;
      failed: boolean;
      censusComplete: boolean;
      pendingPushes: number;
    };
    type CensusInternals = {
      sourceInventories: Map<string, Inventory>;
      rootScans: Map<string, number>;
      beginRootScan(rootUri: string): number;
      finishRootScan(rootUri: string, generation: number): void;
    };
    const sync = new WorkspaceStoreSync(recordingHost([]), [], () => {});
    const state = sync as unknown as CensusInternals;
    const rootA = "file:///workspace/a";
    const rootB = "file:///workspace/b";
    state.sourceInventories.set(rootA, {
      rootUri: rootA,
      selection: "A",
      complete: false,
      failed: false,
      censusComplete: false,
      pendingPushes: 0,
    });
    state.sourceInventories.set(rootB, {
      rootUri: rootB,
      selection: "B",
      complete: false,
      failed: false,
      censusComplete: false,
      pendingPushes: 0,
    });

    const oldA = state.beginRootScan(rootA);
    const currentA = state.beginRootScan(rootA);
    const currentB = state.beginRootScan(rootB);
    state.finishRootScan(rootA, oldA);
    state.finishRootScan(rootB, currentB);
    assert.strictEqual(
      state.sourceInventories.get(rootA)?.complete,
      false,
      "a stale overlapping A scan must not certify A",
    );
    assert.strictEqual(
      state.sourceInventories.get(rootB)?.complete,
      true,
      "B may close only its own completed scan",
    );

    state.finishRootScan(rootA, currentA);
    assert.strictEqual(state.sourceInventories.get(rootA)?.complete, true);
    sync.dispose();
  });

  test("a removed root cannot be closed by its outstanding scan", () => {
    type Inventory = {
      rootUri: string;
      selection: string;
      complete: boolean;
      failed: boolean;
      censusComplete: boolean;
      pendingPushes: number;
    };
    type CensusInternals = {
      sourceInventories: Map<string, Inventory>;
      rootScans: Map<string, number>;
      beginRootScan(rootUri: string): number;
      finishRootScan(rootUri: string, generation: number): void;
    };
    const sync = new WorkspaceStoreSync(recordingHost([]), [], () => {});
    const state = sync as unknown as CensusInternals;
    const root = "file:///workspace/removed";
    state.sourceInventories.set(root, {
      rootUri: root,
      selection: "removed",
      complete: false,
      failed: false,
      censusComplete: false,
      pendingPushes: 0,
    });
    const pending = state.beginRootScan(root);
    state.sourceInventories.delete(root);
    state.rootScans.delete(root);
    state.finishRootScan(root, pending);
    assert.strictEqual(state.sourceInventories.has(root), false);
    sync.dispose();
  });
});

suite("Web workspace sync — pending live source reads", () => {
  test("the complete receipt is withdrawn during a push and restored after its upsert", async () => {
    type Inventory = {
      rootUri: string;
      selection: string;
      complete: boolean;
      failed: boolean;
      censusComplete: boolean;
      pendingPushes: number;
    };
    type CensusInternals = {
      sourceInventories: Map<string, Inventory>;
      rootScans: Map<string, number>;
    };
    const provider = new DeferredFileSystem();
    const registration = vscode.workspace.registerFileSystemProvider(
      "tcl-lsp-census-pending",
      provider,
    );
    const uri = vscode.Uri.parse("tcl-lsp-census-pending:/lib.tcl");
    const rootUri = "tcl-lsp-census-pending:/";
    const calls: StoreCall[] = [];
    const inventoryCalls: Array<{
      rootUri: string;
      selection: string;
      status: "complete" | "incomplete";
    }> = [];
    const sync = new WorkspaceStoreSync(recordingHost(calls, [], inventoryCalls), [], () => {});
    const state = sync as unknown as CensusInternals;
    state.sourceInventories.set(rootUri, {
      rootUri,
      selection: "selected-root",
      complete: true,
      failed: false,
      censusComplete: true,
      pendingPushes: 0,
    });
    state.rootScans.set(rootUri, 7);
    const pushFile = pushFileOf(sync);
    try {
      const pushing = pushFile(uri, FileChange.Changed, DEFAULT_BUDGET, () => {}, {
        rootUri,
        generation: 7,
      });
      await waitForReads(provider, 1);
      assert.strictEqual(
        inventoryCalls[inventoryCalls.length - 1]?.status,
        "incomplete",
        "an in-flight source mutation must not leave the old complete receipt current",
      );
      provider.settle(0, "set fresh 1\n");
      assert.strictEqual(await pushing, true);
      assert.strictEqual(calls[calls.length - 1]?.kind, "upsert");
      assert.strictEqual(
        inventoryCalls[inventoryCalls.length - 1]?.status,
        "complete",
        "the unchanged completed census may be restored after the new bytes land",
      );
    } finally {
      sync.dispose();
      registration.dispose();
    }
  });
});

suite("Web workspace sync — overlapping watcher events", () => {
  test("a delete overtaking a slower change does not resurrect the file", async () => {
    const provider = new DeferredFileSystem();
    const registration = vscode.workspace.registerFileSystemProvider(
      "tcl-lsp-race-delete",
      provider,
    );
    const uri = vscode.Uri.parse("tcl-lsp-race-delete:/lib.tcl");
    const calls: StoreCall[] = [];
    const notified: Array<{ uri: string; type: number }> = [];
    const notify: Notify = (changes) => notified.push(...changes);
    const sync = new WorkspaceStoreSync(recordingHost(calls), [], () => {});
    const pushFile = pushFileOf(sync);
    const handleDelete = handleDeleteOf(sync);
    try {
      const created = pushFile(uri, FileChange.Created, DEFAULT_BUDGET, notify);
      await waitForReads(provider, 1);
      provider.settle(0, "proc greet {} {}\n");
      assert.strictEqual(await created, true, "the store should hold the file to begin with");

      calls.length = 0;
      notified.length = 0;

      // A change arrives, and its read is still outstanding…
      const changed = pushFile(uri, FileChange.Changed, DEFAULT_BUDGET, notify);
      await waitForReads(provider, 2);
      // …when the delete for the same file is handled.
      handleDelete(uri, notify);
      // Only now does the older read come back, carrying contents that named a
      // file which no longer exists.
      provider.settle(1, "proc greet {} { puts stale }\n");

      assert.strictEqual(await changed, false, "an overtaken change cannot report the file held");
      assert.deepStrictEqual(
        calls,
        [{ kind: "delete", uri: uri.toString() }],
        "the delete must be the last word — no re-upsert behind it",
      );
      assert.deepStrictEqual(
        notified,
        [{ uri: uri.toString(), type: FileChange.Deleted }],
        "the server must not be told the deleted file changed",
      );
    } finally {
      sync.dispose();
      registration.dispose();
    }
  });

  test("two changes resolving out of order leave the later write in the store", async () => {
    const provider = new DeferredFileSystem();
    const registration = vscode.workspace.registerFileSystemProvider(
      "tcl-lsp-race-change",
      provider,
    );
    const uri = vscode.Uri.parse("tcl-lsp-race-change:/lib.tcl");
    const calls: StoreCall[] = [];
    const sync = new WorkspaceStoreSync(recordingHost(calls), [], () => {});
    const pushFile = pushFileOf(sync);
    try {
      const first = pushFile(uri, FileChange.Changed, DEFAULT_BUDGET, () => {});
      await waitForReads(provider, 1);
      const second = pushFile(uri, FileChange.Changed, DEFAULT_BUDGET, () => {});
      await waitForReads(provider, 2);

      // The second event's read answers first, then the first event's.
      provider.settle(1, "set version 2\n");
      provider.settle(0, "set version 1\n");
      await Promise.all([first, second]);

      const upserts = calls.filter((call) => call.kind === "upsert");
      assert.strictEqual(
        upserts[upserts.length - 1]?.text,
        "set version 2\n",
        `the store must end on the later event, got ${JSON.stringify(upserts)}`,
      );
    } finally {
      sync.dispose();
      registration.dispose();
    }
  });
});
