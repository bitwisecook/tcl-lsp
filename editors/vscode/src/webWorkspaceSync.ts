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
 * - **Event order within a file.** A `FileSystemWatcher` does not wait for one
 *   handler before delivering the next event, so two events for the same file
 *   overlap: a change's read can still be in flight when the delete that
 *   followed it has already been applied. Every event takes a per-URI sequence
 *   number synchronously, and a push whose number has moved on by the time its
 *   read returns applies nothing — otherwise the older read resurrects a file
 *   the store had just withdrawn, or pins a revision the next write already
 *   replaced, and the server answers out of it until some later event happens
 *   to correct it. The startup sweep needs none of this: it completes before
 *   `watch()` registers a single watcher.
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
 * What the sweep must carry that no derived glob names: the config files
 * `is_config_file` recognises, the sidecar stub bundle, and the classic
 * autoloader's `tclIndex`. None of them carries a Tcl source extension, and
 * `tclIndex` is not a `contributes.languages[].filenames` registration either
 * (that list is the `bigip.conf` variants and `presentation`), so nothing in
 * the derivation above reaches it.
 *
 * `tclIndex` is what `PackageResolver::scan_single_dir` reads to populate
 * `auto_index`. Without it in the store, every command an autoloader workspace
 * provides by bare name is unknown to the browser server: a false W123 on each
 * call and no navigation to its definition. Case-folded per character for the
 * same reason the generated source glob is — a case-insensitive filesystem
 * spells it `tclindex` just as readily, and the server matches it with
 * `eq_ignore_ascii_case`.
 */
const NON_SOURCE_GLOBS = [
  "**/.tcl-lsp.ini",
  "**/tcl-lsp/config.ini",
  "**/*.tcl.stubs",
  "**/[tT][cC][lL][iI][nN][dD][eE][xX]",
];

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

/** `FileChangeType` on the wire — the protocol's own numbering. */
const FileChange = { Created: 1, Changed: 2, Deleted: 3 } as const;

/**
 * Name at most `limit` URIs, then say how many more there are.
 *
 * A skipped-file list is diagnostic, and a workspace that overruns the budget
 * can overrun it by thousands — one log line holding every one of them is not
 * readable and is not what a reader needs to act.
 */
function listUris(uris: readonly vscode.Uri[], limit = 20): string {
  const shown = uris.slice(0, limit).map((uri) => uri.toString());
  const rest = uris.length - shown.length;
  return rest > 0 ? `${shown.join(", ")} (+${rest} more)` : shown.join(", ");
}

/**
 * Sweep the workspace into the server's store, then keep it live.
 *
 * One instance per session; `dispose` tears down the watchers.
 */
export class WorkspaceStoreSync {
  private readonly watchers: vscode.Disposable[] = [];

  /**
   * What the store currently holds, `uri.toString()` → byte length.
   *
   * The budget has to be a property of the *session*, not of the startup
   * sweep: without this, a workspace that generates files while the editor is
   * open grows the server's store without limit, because the watcher path had
   * no idea how much the sweep had already sent. It also makes a replacement
   * exact — an edited file's new size replaces its old one rather than adding
   * to the total — and it is what lets a removed folder's files be withdrawn.
   */
  private readonly stored = new Map<string, number>();
  private storedBytes = 0;

  /** Live-path budget refusals already logged, so a generated tree cannot spam. */
  private liveSkipsLogged = 0;

  /**
   * Per-URI event sequence numbers, held only while a push for that URI is in
   * flight — which is the only time anything has to be compared against them,
   * so the map cannot grow with the workspace.
   *
   * See the module comment's *Event order within a file*: this is what stops a
   * slower read from applying after the event that overtook it.
   */
  private readonly livePushes = new Map<string, { generation: number; inFlight: number }>();

  constructor(
    private readonly host: SourceStoreHost,
    private readonly globs: string[],
    private readonly log: (message: string) => void,
  ) {}

  /** Register (or replace) one file in the store, keeping the running totals exact. */
  private upsertTracked(uri: vscode.Uri, text: string, byteLength: number): void {
    const key = uri.toString();
    this.storedBytes += byteLength - (this.stored.get(key) ?? 0);
    this.stored.set(key, byteLength);
    this.host.upsert(key, text);
  }

  /** Withdraw one file from the store. Returns whether it held anything. */
  private deleteTracked(uri: vscode.Uri): boolean {
    const key = uri.toString();
    const held = this.stored.get(key);
    if (held === undefined) {
      this.host.delete(key);
      return false;
    }
    this.storedBytes -= held;
    this.stored.delete(key);
    this.host.delete(key);
    return true;
  }

  /**
   * Whether `byteLength` fits, counting a replacement as only its delta.
   */
  private fits(uri: vscode.Uri, byteLength: number, budget: WorkspaceSyncBudget): boolean {
    const key = uri.toString();
    const held = this.stored.get(key);
    if (held === undefined && this.stored.size >= budget.maxFiles) {
      return false;
    }
    return this.storedBytes - (held ?? 0) + byteLength <= budget.maxTotalBytes;
  }

  /** Open a push for `key`, returning the sequence number it must still hold. */
  private beginLivePush(key: string): number {
    const entry = this.livePushes.get(key) ?? { generation: 0, inFlight: 0 };
    entry.generation += 1;
    entry.inFlight += 1;
    this.livePushes.set(key, entry);
    return entry.generation;
  }

  /** Close a push, dropping the entry once nothing is left to compare against. */
  private endLivePush(key: string): void {
    const entry = this.livePushes.get(key);
    if (entry === undefined) {
      return;
    }
    entry.inFlight -= 1;
    if (entry.inFlight <= 0) {
      this.livePushes.delete(key);
    }
  }

  /** Whether a later event has overtaken the push holding `generation`. */
  private livePushSuperseded(key: string, generation: number): boolean {
    return this.livePushes.get(key)?.generation !== generation;
  }

  /**
   * Record an event that supersedes any push still in flight for `key` — a
   * delete, or a folder leaving the workspace.
   *
   * Nothing to record when no push is in flight: the number exists only to be
   * compared against one.
   */
  private supersedeLivePush(key: string): void {
    const entry = this.livePushes.get(key);
    if (entry !== undefined) {
      entry.generation += 1;
    }
  }

  /** Log a live-path budget refusal, capped so a burst cannot flood the channel. */
  private logLiveSkip(message: string): void {
    const CAP = 20;
    if (this.liveSkipsLogged < CAP) {
      this.log(message);
    } else if (this.liveSkipsLogged === CAP) {
      this.log(
        `[web] further workspace-budget messages suppressed for this session — ` +
          "raise tclLsp.web.workspaceSync.maxFiles / .maxTotalBytes and restart the server",
      );
    }
    this.liveSkipsLogged += 1;
  }

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
      // Cheap pre-check: no room for even the smallest file left.
      if (this.stored.size >= budget.maxFiles || this.storedBytes >= budget.maxTotalBytes) {
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
      // Checked BEFORE the file is counted, so the total cannot end up over
      // `maxTotalBytes` by most of one file. A file that does not fit is
      // skipped rather than ending the sweep — a smaller one after it still
      // gets in, and the report names every skip either way.
      if (!this.fits(uri, bytes.byteLength, budget)) {
        report.overBudget.push(uri);
        continue;
      }
      this.upsertTracked(uri, decoder.decode(bytes), bytes.byteLength);
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
    if (report.tooLarge.length > 0) {
      this.log(
        `[web] skipped ${report.tooLarge.length} file(s) over ` +
          `tclLsp.web.workspaceSync.maxFileBytes: ${listUris(report.tooLarge)}`,
      );
    }
    if (report.overBudget.length > 0) {
      this.log(
        `[web] skipped ${report.overBudget.length} file(s) — the workspace budget ran out. ` +
          "Cross-file results for them will be missing. Raise " +
          "tclLsp.web.workspaceSync.maxFiles / .maxTotalBytes to include them: " +
          listUris(report.overBudget),
      );
    }
    if (report.unreadable.length > 0) {
      this.log(
        `[web] skipped ${report.unreadable.length} unreadable file(s): ` +
          listUris(report.unreadable),
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
   *
   * The budget applies here exactly as it does to the startup sweep. It has to:
   * a build that writes generated `.tcl` while the editor is open would
   * otherwise grow the server's store without limit, one create at a time,
   * past every cap the user set.
   */
  watch(
    budget: WorkspaceSyncBudget,
    notify: (changes: Array<{ uri: string; type: number }>) => void,
  ): void {
    for (const glob of this.globs) {
      const watcher = vscode.workspace.createFileSystemWatcher(glob);
      this.watchers.push(
        watcher,
        watcher.onDidCreate((uri) => void this.pushFile(uri, FileChange.Created, budget, notify)),
        watcher.onDidChange((uri) => void this.pushFile(uri, FileChange.Changed, budget, notify)),
        watcher.onDidDelete((uri) => this.handleDelete(uri, notify)),
      );
    }
  }

  /**
   * Withdraw a file the watcher says is gone.
   *
   * Symmetric with `pushFile`, and the reason both exist as methods: the two
   * arms of the watcher must take their sequence numbers in the order the
   * events arrived, which only holds if the bump happens where the event is
   * handled. The notification goes out whether or not the store held the file
   * — the server is entitled to hear that a file it may have indexed is gone.
   */
  private handleDelete(
    uri: vscode.Uri,
    notify: (changes: Array<{ uri: string; type: number }>) => void,
  ): void {
    this.supersedeLivePush(uri.toString());
    this.deleteTracked(uri);
    notify([{ uri: uri.toString(), type: FileChange.Deleted }]);
  }

  /**
   * Sweep folders added to the workspace after startup, and withdraw the files
   * of folders removed from it.
   *
   * Without this an added folder is invisible until the server restarts, and a
   * removed one keeps answering out of the store for files no longer in the
   * workspace.
   */
  watchFolders(
    budget: WorkspaceSyncBudget,
    notify: (changes: Array<{ uri: string; type: number }>) => void,
  ): void {
    this.watchers.push(
      vscode.workspace.onDidChangeWorkspaceFolders((event) => {
        for (const folder of event.removed) {
          const prefix = folder.uri.toString().replace(/\/?$/, "/");
          // Every push still reading a file of this folder, whether or not it
          // has reached the store yet: a sweep of a folder that has since left
          // the workspace must not land its files behind it.
          for (const key of this.livePushes.keys()) {
            if (key.startsWith(prefix)) {
              this.supersedeLivePush(key);
            }
          }
          const withdrawn: Array<{ uri: string; type: number }> = [];
          for (const key of [...this.stored.keys()]) {
            if (key.startsWith(prefix)) {
              this.deleteTracked(vscode.Uri.parse(key));
              withdrawn.push({ uri: key, type: FileChange.Deleted });
            }
          }
          if (withdrawn.length > 0) {
            this.log(`[web] folder removed: withdrew ${withdrawn.length} file(s) from the store`);
            notify(withdrawn);
          }
        }
        if (event.added.length > 0) {
          void this.primeFolders(event.added, budget, notify);
        }
      }),
    );
  }

  /** Read one or more newly-added folders into the store, within budget. */
  private async primeFolders(
    folders: readonly vscode.WorkspaceFolder[],
    budget: WorkspaceSyncBudget,
    notify: (changes: Array<{ uri: string; type: number }>) => void,
  ): Promise<void> {
    this.warnAboutNonFileSchemes(folders);
    const found = new Map<string, vscode.Uri>();
    for (const folder of folders) {
      for (const glob of this.globs) {
        try {
          for (const uri of await vscode.workspace.findFiles(
            new vscode.RelativePattern(folder, glob),
          )) {
            found.set(uri.toString(), uri);
          }
        } catch (err) {
          this.log(`[web] file search failed for ${glob} in ${folder.name}: ${String(err)}`);
        }
      }
    }
    const added: Array<{ uri: string; type: number }> = [];
    for (const uri of [...found.values()].sort((left, right) =>
      left.toString().localeCompare(right.toString()),
    )) {
      if (await this.pushFile(uri, FileChange.Created, budget, () => {})) {
        added.push({ uri: uri.toString(), type: FileChange.Created });
      }
    }
    this.log(`[web] folder added: ${added.length}/${found.size} file(s) sent to the store`);
    if (added.length > 0) {
      notify(added);
    }
  }

  /**
   * Send one created/changed file, honouring the budget.
   *
   * Returns whether the store now holds the file's current contents.
   *
   * The three refusals below all *withdraw* a file the store already had
   * rather than leaving the previous copy in place. A file that grew past the
   * cap, or that has stopped being readable at all, is the case that matters:
   * leaving the old contents there means the server keeps answering —
   * confidently — out of a copy that no longer exists on disk. An absent file
   * gives a visibly incomplete answer; a stale one gives a wrong answer that
   * looks complete.
   *
   * The sequence number taken before the read is the ordering half: nothing
   * below it — refusal or upsert — is applied once a later event for the same
   * file has been handled.
   */
  private async pushFile(
    uri: vscode.Uri,
    changeType: number,
    budget: WorkspaceSyncBudget,
    notify: (changes: Array<{ uri: string; type: number }>) => void,
  ): Promise<boolean> {
    const key = uri.toString();
    const generation = this.beginLivePush(key);
    try {
      const withdraw = (why: string): boolean => {
        const held = this.deleteTracked(uri);
        this.logLiveSkip(`[web] ${key} ${why} — ${held ? "withdrawn from the store" : "not sent"}`);
        if (held) {
          notify([{ uri: key, type: FileChange.Deleted }]);
        }
        return false;
      };

      let bytes: Uint8Array | undefined;
      let failure: unknown;
      try {
        bytes = await vscode.workspace.fs.readFile(uri);
      } catch (err) {
        failure = err;
      }

      // Before anything is applied, and before the read failure is acted on: a
      // newer push may already have landed this file's current contents, and
      // withdrawing them because an older read failed would delete what is
      // right. Whichever event came last is the one that decides.
      if (this.livePushSuperseded(key, generation)) {
        return false;
      }
      if (bytes === undefined) {
        return withdraw(`could not be read (${String(failure)})`);
      }

      if (bytes.byteLength > budget.maxFileBytes) {
        return withdraw(
          `is over tclLsp.web.workspaceSync.maxFileBytes (${bytes.byteLength} bytes)`,
        );
      }
      if (!this.fits(uri, bytes.byteLength, budget)) {
        return withdraw("does not fit the workspace budget");
      }

      this.upsertTracked(uri, decoder.decode(bytes), bytes.byteLength);
      notify([{ uri: key, type: changeType }]);
      return true;
    } finally {
      this.endLivePush(key);
    }
  }

  dispose(): void {
    for (const watcher of this.watchers.splice(0)) {
      watcher.dispose();
    }
    // A read still in flight belongs to the session that just ended: superseded
    // here so it cannot upsert into a store that is about to be forgotten.
    for (const entry of this.livePushes.values()) {
      entry.generation += 1;
    }
    // The session this tracked is over and the store it described goes with
    // its worker; nothing should be able to read the accounting back.
    this.stored.clear();
    this.storedBytes = 0;
  }
}
