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
import { globSync } from "glob";
import {
  deriveSyncGlobs,
  DEFAULT_BUDGET,
  SourceStoreHost,
  SyncManifest,
  WorkspaceStoreSync,
  WorkspaceSyncBudget,
} from "../webWorkspaceSync";

/** `FileChangeType` on the wire — the protocol's numbering, not `vscode.FileChangeType`'s. */
const FileChange = { Created: 1, Changed: 2, Deleted: 3 } as const;

/** What the worker would have been told, in the order it would have heard it. */
interface StoreCall {
  kind: "upsert" | "delete";
  uri: string;
  text?: string;
}

function recordingHost(calls: StoreCall[]): SourceStoreHost {
  return {
    upsert: (uri, text) => calls.push({ kind: "upsert", uri, text }),
    delete: (uri) => calls.push({ kind: "delete", uri }),
    upsertSpecPack: () => {},
  };
}

type PushFile = (
  uri: vscode.Uri,
  changeType: number,
  budget: WorkspaceSyncBudget,
  notify: (changes: Array<{ uri: string; type: number }>) => void,
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
      contributes: { languages: [{ filenames: ["bigip.conf", "presentation"] }] },
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
    // Case-folded per character (issue #1215), which is what makes the sweep
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
