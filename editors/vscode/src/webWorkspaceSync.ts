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
 * The VS Code-web half of the closed-file source store
 * (`docs/design/contracts/lsp-source-store.md`).
 *
 * The browser build of the server has no filesystem: every file the editor has
 * not opened — the sibling a `source` edge points at, the `pkgIndex.tcl` the
 * package database is built from, the `.tcl-lsp.ini` a session layers under the
 * editor settings — reaches it as a `{ tclLsp: "upsert", uri, text }` message
 * on the worker port. This module is what sends them: one sweep before the
 * client starts, then a `FileSystemWatcher` keeping the store live.
 *
 * Two properties matter and are enforced here rather than assumed:
 *
 * - **Ordering.** The sweep runs, and every upsert is posted, *before*
 *   `client.start()` sends `initialize` — `initialized` is what loads the pack
 *   set and runs the workspace scan, so a file that arrives after it is invisible
 *   until something re-reads it. `postMessage` is FIFO per port and the worker
 *   backlogs anything that arrives before wasm init, so posting early is safe.
 * - **No silent truncation.** A workspace bigger than the budget is common on
 *   github.dev; what is not acceptable is the server quietly analysing a subset
 *   and reporting confident, wrong cross-file answers. Everything skipped is
 *   counted and named in the output channel, and the budget is a setting.
 */

import * as vscode from "vscode";

/** What the host can do to the server's in-memory store. */
export interface SourceStoreHost {
  upsert(uri: string, text: string): void;
  delete(uri: string): void;
  /** `name` is relative to the virtual pack mount — bare names only. */
  upsertSpecPack(name: string, text: string): void;
}

/** Post the three store messages `rust/tcl-lsp-server-wasm/worker.js` accepts. */
export function workerSourceStoreHost(worker: Worker): SourceStoreHost {
  return {
    upsert: (uri, text) => worker.postMessage({ tclLsp: "upsert", uri, text }),
    delete: (uri) => worker.postMessage({ tclLsp: "delete", uri }),
    upsertSpecPack: (name, text) => worker.postMessage({ tclLsp: "upsertSpecPack", name, text }),
  };
}

export interface WorkspaceSyncBudget {
  /** Most files to send. */
  maxFiles: number;
  /** Most bytes to send in total, across all files. */
  maxTotalBytes: number;
  /** Largest single file to send. */
  maxFileBytes: number;
}

export const DEFAULT_BUDGET: WorkspaceSyncBudget = {
  maxFiles: 2000,
  maxTotalBytes: 32 * 1024 * 1024,
  maxFileBytes: 2 * 1024 * 1024,
};

export function readBudget(): WorkspaceSyncBudget {
  const cfg = vscode.workspace.getConfiguration("tclLsp.web.workspaceSync");
  const positive = (value: number | undefined, fallback: number): number =>
    typeof value === "number" && Number.isFinite(value) && value > 0 ? Math.floor(value) : fallback;
  return {
    maxFiles: positive(cfg.get<number>("maxFiles"), DEFAULT_BUDGET.maxFiles),
    maxTotalBytes: positive(cfg.get<number>("maxTotalBytes"), DEFAULT_BUDGET.maxTotalBytes),
    maxFileBytes: positive(cfg.get<number>("maxFileBytes"), DEFAULT_BUDGET.maxFileBytes),
  };
}

/** The manifest fields the globs are derived from. */
export interface SyncManifest {
  activationEvents?: string[];
  contributes?: {
    languages?: Array<{ extensions?: string[]; filenames?: string[] }>;
  };
}

/**
 * The config files `is_config_file` recognises, and the sidecar bundle the
 * server watches alongside Tcl source. Neither is a Tcl source extension, so
 * neither is covered by the manifest's source glob.
 */
const NON_SOURCE_GLOBS = ["**/.tcl-lsp.ini", "**/tcl-lsp/config.ini", "**/*.tcl.stubs"];

/**
 * The globs to sweep, derived from the extension's own manifest.
 *
 * The first is the `workspaceContains:` activation glob, which
 * `cargo xtask gen-vscode-package` generates from
 * `tcl_registry::dialects::TCL_SOURCE_EXTENSIONS` — the very list the server
 * indexes and watches (issue #1242), already case-folded per character so it
 * matches `UPPER.TCL` on Linux too. Deriving from it rather than restating it
 * is what keeps the web host's view of "a file the server cares about"
 * identical to the server's own, with no second list to drift.
 *
 * `contributes.languages[].filenames` adds the whole-basename registrations
 * (`bigip.conf`, `presentation`, …), which carry no extension and so appear in
 * no extension glob.
 */
export function deriveSyncGlobs(manifest: SyncManifest): string[] {
  const globs: string[] = [];

  for (const event of manifest.activationEvents ?? []) {
    if (event.startsWith("workspaceContains:")) {
      globs.push(event.slice("workspaceContains:".length));
    }
  }
  if (globs.length === 0) {
    // No generated activation glob (an unexpected manifest): fall back to the
    // registered language extensions, lower-case only.
    const extensions = new Set<string>();
    for (const language of manifest.contributes?.languages ?? []) {
      for (const extension of language.extensions ?? []) {
        extensions.add(extension.replace(/^\./, ""));
      }
    }
    if (extensions.size > 0) {
      globs.push(`**/*.{${[...extensions].sort().join(",")}}`);
    }
  }

  const filenames = new Set<string>();
  for (const language of manifest.contributes?.languages ?? []) {
    for (const filename of language.filenames ?? []) {
      filenames.add(filename);
    }
  }
  for (const filename of [...filenames].sort()) {
    globs.push(`**/${filename}`);
  }

  globs.push(...NON_SOURCE_GLOBS);
  return globs;
}

export interface SyncReport {
  sent: number;
  bytes: number;
  /** Files skipped because one file exceeded `maxFileBytes`. */
  tooLarge: vscode.Uri[];
  /** Files skipped because the file-count or total-byte budget ran out. */
  overBudget: vscode.Uri[];
  /** Files the filesystem provider refused to read. */
  unreadable: vscode.Uri[];
}

const decoder = new TextDecoder("utf-8");

/**
 * Sweep the workspace into the server's store, then keep it live.
 *
 * One instance per session; `dispose` tears down the watchers.
 */
export class WorkspaceStoreSync {
  private readonly watchers: vscode.Disposable[] = [];

  constructor(
    private readonly host: SourceStoreHost,
    private readonly globs: string[],
    private readonly log: (message: string) => void,
  ) {}

  /**
   * Read every matching workspace file and upsert it, within budget.
   *
   * Must complete before the language client starts: see the module comment.
   */
  async primeWorkspace(budget: WorkspaceSyncBudget): Promise<SyncReport> {
    const report: SyncReport = { sent: 0, bytes: 0, tooLarge: [], overBudget: [], unreadable: [] };
    const folders = vscode.workspace.workspaceFolders;
    if (!folders?.length) {
      this.log("[web] no workspace folder open — the server sees only the editor's open documents");
      return report;
    }
    this.warnAboutNonFileSchemes(folders);

    const found = new Map<string, vscode.Uri>();
    for (const glob of this.globs) {
      let matches: vscode.Uri[];
      try {
        matches = await vscode.workspace.findFiles(glob);
      } catch (err) {
        this.log(`[web] file search failed for ${glob}: ${String(err)}`);
        continue;
      }
      for (const uri of matches) {
        found.set(uri.toString(), uri);
      }
    }

    // Sorted so a workspace that overruns the budget truncates the same way
    // every session, rather than by whatever order the filesystem provider
    // happened to answer in.
    const uris = [...found.values()].sort((left, right) =>
      left.toString().localeCompare(right.toString()),
    );

    for (const uri of uris) {
      if (report.sent >= budget.maxFiles || report.bytes >= budget.maxTotalBytes) {
        report.overBudget.push(uri);
        continue;
      }
      let bytes: Uint8Array;
      try {
        const stat = await vscode.workspace.fs.stat(uri);
        if (stat.size > budget.maxFileBytes) {
          report.tooLarge.push(uri);
          continue;
        }
        bytes = await vscode.workspace.fs.readFile(uri);
      } catch (err) {
        report.unreadable.push(uri);
        this.log(`[web] could not read ${uri.toString()}: ${String(err)}`);
        continue;
      }
      if (bytes.byteLength > budget.maxFileBytes) {
        // A provider whose `stat` under-reports (or does not implement `size`).
        report.tooLarge.push(uri);
        continue;
      }
      this.host.upsert(uri.toString(), decoder.decode(bytes));
      report.sent += 1;
      report.bytes += bytes.byteLength;
    }

    this.reportBudget(report, uris.length, budget);
    return report;
  }

  /**
   * Say so, once, when the workspace is not on the `file:` scheme.
   *
   * `LspWorker::vfs_upsert` keys the store on the filesystem path a `file:` URI
   * names (`Uri::to_file_path`), and drops an upsert whose URI names none — so
   * on github.dev (`vscode-vfs:`) and every other virtual provider, the sweep
   * below runs, reads the workspace, and the server keeps none of it. The open
   * document still analyses in full, because `didOpen` carries its text and
   * never touches the store; what is missing is everything cross-file.
   *
   * The sweep still runs: it is cheap next to the wasm module, it is correct
   * the moment the store learns to key a non-path URI, and a `file:`-scheme web
   * host works today. Saying nothing would be the wrong trade — a user would
   * see confident single-file answers and silently absent cross-file ones.
   */
  private warnAboutNonFileSchemes(folders: readonly vscode.WorkspaceFolder[]): void {
    const virtual = folders.filter((folder) => folder.uri.scheme !== "file");
    if (virtual.length === 0) {
      return;
    }
    this.log(
      `[web] this workspace is on the ${[...new Set(virtual.map((f) => f.uri.scheme))].join("/")} ` +
        "scheme, not file:. The browser server keys its closed-file store on the path a file: URI " +
        "names, so it discards these files: analysis of open documents is complete, and cross-file " +
        "results (definitions in un-opened siblings, the package database, workspace symbols) are " +
        "not available.",
    );
  }

  private reportBudget(report: SyncReport, candidates: number, budget: WorkspaceSyncBudget): void {
    this.log(
      `[web] workspace store: ${report.sent}/${candidates} files, ` +
        `${report.bytes} bytes (budget ${budget.maxFiles} files / ${budget.maxTotalBytes} bytes / ` +
        `${budget.maxFileBytes} bytes per file)`,
    );
    const name = (uri: vscode.Uri) => uri.toString();
    if (report.tooLarge.length > 0) {
      this.log(
        `[web] skipped ${report.tooLarge.length} file(s) over tclLsp.web.workspaceSync.maxFileBytes: ` +
          report.tooLarge.map(name).join(", "),
      );
    }
    if (report.overBudget.length > 0) {
      this.log(
        `[web] skipped ${report.overBudget.length} file(s) — the workspace budget ran out. ` +
          "Cross-file results for them will be missing. Raise " +
          "tclLsp.web.workspaceSync.maxFiles / .maxTotalBytes to include them: " +
          report.overBudget.map(name).join(", "),
      );
    }
    if (report.unreadable.length > 0) {
      this.log(
        `[web] skipped ${report.unreadable.length} unreadable file(s): ` +
          report.unreadable.map(name).join(", "),
      );
    }
  }

  /**
   * Upsert the `.tclspec` packs staged into the extension under
   * `dist/web/specs/`, so the browser server loads the same EDA vendor
   * libraries the native server finds in a `specs/` directory beside its
   * executable.
   *
   * Driven by `specs/index.json`, a manifest the staging step writes, rather
   * than by listing the directory: an installed web extension's files are
   * served over http, and VS Code's http filesystem provider is read-only and
   * answers `readFile` only — `readDirectory` fails outright, which is how a
   * silent "no bundled spec packs" would otherwise be the *normal* result on
   * vscode.dev.
   */
  async primeSpecPacks(specsDir: vscode.Uri): Promise<number> {
    const names = await this.readPackManifest(specsDir);
    if (names.length === 0) {
      return 0;
    }
    let loaded = 0;
    for (const name of names) {
      try {
        const bytes = await vscode.workspace.fs.readFile(vscode.Uri.joinPath(specsDir, name));
        // A bare name: the worker refuses a rooted name or one carrying `..`,
        // so the mount can never be shadowed by a pack upsert.
        this.host.upsertSpecPack(name, decoder.decode(bytes));
        loaded += 1;
      } catch (err) {
        this.log(`[web] could not read bundled spec pack ${name}: ${String(err)}`);
      }
    }
    this.log(`[web] bundled spec packs upserted: ${loaded}/${names.length}`);
    return loaded;
  }

  private async readPackManifest(specsDir: vscode.Uri): Promise<string[]> {
    try {
      const bytes = await vscode.workspace.fs.readFile(vscode.Uri.joinPath(specsDir, "index.json"));
      const parsed: unknown = JSON.parse(decoder.decode(bytes));
      if (Array.isArray(parsed)) {
        return parsed
          .filter((name): name is string => typeof name === "string" && name.endsWith(".tclspec"))
          .sort();
      }
      this.log(`[web] bundled spec pack manifest is not a list of names`);
    } catch (err) {
      this.log(
        `[web] no bundled spec pack manifest at ${specsDir.toString()}/index.json: ${String(err)}`,
      );
    }
    return [];
  }

  /**
   * Keep the store live after startup.
   *
   * A changed file needs no new message shape: upsert it and post the ordinary
   * `workspace/didChangeWatchedFiles` an editor sends for a file changed
   * outside it. The upsert always lands first, because the server re-reads the
   * file *through the store* while handling the notification.
   */
  watch(
    budget: WorkspaceSyncBudget,
    notify: (changes: Array<{ uri: string; type: number }>) => void,
  ): void {
    for (const glob of this.globs) {
      const watcher = vscode.workspace.createFileSystemWatcher(glob);
      this.watchers.push(
        watcher,
        watcher.onDidCreate((uri) => void this.pushFile(uri, 1, budget, notify)),
        watcher.onDidChange((uri) => void this.pushFile(uri, 2, budget, notify)),
        watcher.onDidDelete((uri) => {
          this.host.delete(uri.toString());
          notify([{ uri: uri.toString(), type: 3 }]);
        }),
      );
    }
  }

  private async pushFile(
    uri: vscode.Uri,
    changeType: number,
    budget: WorkspaceSyncBudget,
    notify: (changes: Array<{ uri: string; type: number }>) => void,
  ): Promise<void> {
    try {
      const bytes = await vscode.workspace.fs.readFile(uri);
      if (bytes.byteLength > budget.maxFileBytes) {
        this.log(
          `[web] ${uri.toString()} changed but is over ` +
            `tclLsp.web.workspaceSync.maxFileBytes (${bytes.byteLength} bytes) — not sent`,
        );
        return;
      }
      this.host.upsert(uri.toString(), decoder.decode(bytes));
    } catch (err) {
      this.log(`[web] could not read changed file ${uri.toString()}: ${String(err)}`);
      return;
    }
    notify([{ uri: uri.toString(), type: changeType }]);
  }

  dispose(): void {
    for (const watcher of this.watchers.splice(0)) {
      watcher.dispose();
    }
  }
}
