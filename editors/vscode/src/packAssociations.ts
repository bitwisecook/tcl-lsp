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

// Runtime file-extension registration for SpecTcl-pack-declared extensions
// (issue #1626).
//
// # Why this exists
//
// The server side of extension routing is already lazy: `dialect_from_extension`
// consults the *discovered packs* before the built-in catalogue, so the moment
// a `.tclspec` declaring `file_extension foo -dialect …` is found, every
// server-side decision about a `bar.foo` — closed-document analysis, indexing,
// diagnostics — is already right.
//
// The editor is not. VS Code learns extension-to-language mappings from a
// static `package.json` written long before any user's pack existed, so a
// pack-claimed extension is invisible to it: `bar.foo` opens as plain text,
// the language client never attaches, and none of the server's correct
// answers ever reach the buffer.
//
// # What it does
//
// The server advertises the pairs its discovered packs claim and pushes
// `tcl-lsp/specPacksReloaded` once a reload has fully landed; this module
// projects the advertised set into workspace-scoped `files.associations` and
// flips any already-open document onto the resulting language.
//
// # The ownership rule
//
// Reconciliation writes and *deletes* configuration, so "is this entry mine"
// has to be answerable exactly. It is answered by remembering the value
// written alongside the key: an entry is ours only while the configuration
// still says what we last wrote there.
//
// That is what makes a user edit stick. Remembering only the key was not
// enough — a user who retargets `*.foo` to their own language still matched
// the remembered key, so the next sync overwrote their choice and removing the
// pack deleted their entry outright. Comparing the value instead means the
// first edit hands ownership back permanently: the entry no longer matches, so
// it is neither rewritten nor retired.
//
// New language *ids* cannot be created at runtime, so a pack extension rides
// an existing one: its dialect's editor language where the dialect has one,
// and plain `tcl` otherwise. The dialect itself is still decided server-side,
// so `tcl` here costs nothing but the grammar's first paint.

import * as vscode from "vscode";

import { isTclLanguage } from "./languageIds";

/** One extension a discovered pack claims, as the server advertises it. */
export interface PackFileExtension {
  /** Lower-case, no leading dot. */
  extension: string;
  /** The canonical dialect the row routes to, or null for none. */
  dialect?: string | null;
  /** The existing editor language id the extension should associate with. */
  language_id: string;
  /** The declaring pack, for the log line. */
  pack?: string;
}

/**
 * The `files.associations` entries this module owns, in workspace state: the
 * glob mapped to the language id we last wrote for it.
 *
 * Persisted rather than recomputed because the authoritative question on
 * cleanup is "did *we* write this, and is it still what we wrote" — and the
 * pack that would have told us is exactly the thing that has just gone away.
 */
const OWNED_KEY = "tclLsp.packFileAssociations.owned";

/**
 * The `files.associations` glob for an extension, matching **any** casing.
 *
 * `files.associations` globs are matched case-sensitively on a case-sensitive
 * filesystem, while everything on the server side of this — `is_tcl_source`,
 * `dialect_from_extension`, the watcher registrations — folds case. A plain
 * `*.foo` would therefore leave `SAMPLE.FOO` opening as plaintext with no
 * client attached, which is the same gap the contributed `filenames` had
 * (review finding P2-2).
 *
 * Per character rather than by listing variants, and deliberately the same
 * shape the manifest's generated `filenamePatterns` and the
 * `workspaceContains` activation glob use (`tcl_registry::dialects`'
 * `fold_case_in_glob`): an extension of n letters has 2^n casings and one
 * character class matches all of them exactly.
 */
export function globFor(extension: string): string {
  const folded = [...extension]
    .map((c) => (/[a-zA-Z]/.test(c) ? `[${c.toLowerCase()}${c.toUpperCase()}]` : c))
    .join("");
  return `*.${folded}`;
}

/**
 * What this module last wrote, tolerating the shape it used to persist.
 *
 * An earlier build stored a bare `string[]` of globs. Reading one of those as
 * "no remembered values" is the safe direction: every entry it named looks
 * user-owned, so the worst case is that a stale association survives until the
 * user removes it, rather than this module deleting something it cannot prove
 * it wrote.
 */
function readOwned(context: vscode.ExtensionContext): Record<string, string> {
  const stored = context.workspaceState.get<unknown>(OWNED_KEY);
  if (stored && typeof stored === "object" && !Array.isArray(stored)) {
    return stored as Record<string, string>;
  }
  return {};
}

/** Whether two `{glob: languageId}` records name the same pairs. */
function sameOwnership(a: Record<string, string>, b: Record<string, string>): boolean {
  const aKeys = Object.keys(a).sort();
  const bKeys = Object.keys(b).sort();
  return (
    aKeys.length === bKeys.length && aKeys.every((key, i) => key === bKeys[i] && a[key] === b[key])
  );
}

/**
 * Reconcile workspace `files.associations` with the extensions the server says
 * the discovered packs claim.
 *
 * Returns the globs in force after the sync, for tests and the log line. A
 * no-op — no packs claiming anything new, nothing of ours left over — writes
 * no configuration at all, so an ordinary workspace never grows a
 * `.vscode/settings.json` it did not ask for.
 */
export async function syncPackFileAssociations(
  context: vscode.ExtensionContext,
  advertised: readonly PackFileExtension[],
  log?: (message: string) => void,
): Promise<string[]> {
  if (!vscode.workspace.workspaceFolders?.length) {
    // No workspace to scope the association to; a single loose file gets the
    // server's own detection once it opens as Tcl, and nothing else.
    return [];
  }

  const filesConfig = vscode.workspace.getConfiguration("files");
  const inspected = filesConfig.inspect<Record<string, string>>("associations");
  const current: Record<string, string> = { ...(inspected?.workspaceValue ?? {}) };
  const previouslyOwned = readOwned(context);

  // Keyed by the glob actually written to configuration; the plain extension
  // rides along because `retargetOpenDocuments` matches a filename, not a
  // glob, and the written key is case-folded.
  const wanted = new Map<string, { languageId: string; extension: string }>();
  for (const row of advertised) {
    if (!row?.extension || !row.language_id) {
      continue;
    }
    wanted.set(globFor(row.extension), {
      languageId: row.language_id,
      extension: row.extension.toLowerCase(),
    });
  }

  let changed = false;
  const nowOwned: Record<string, string> = {};
  // Only the entries reconciliation actually installs or already owns. The
  // ones it skips must not reach `retargetOpenDocuments`, or a user's
  // preserved association would be overruled on the open buffer anyway —
  // silently undoing the choice the skip exists to respect.
  const owned = new Map<string, string>();
  for (const [glob, { languageId, extension }] of wanted) {
    const existing = current[glob];
    const weWrote = previouslyOwned[glob];
    const oursNow = existing !== undefined && existing === weWrote;
    if (existing !== undefined && !oursNow) {
      // Either somebody else always owned this glob, or they have since
      // edited what we wrote. Their choice wins from here on: we neither
      // overwrite it nor claim it, so a later pack removal cannot delete it.
      log?.(`[packs] ${glob} is associated with "${existing}" by hand; leaving it alone`);
      continue;
    }
    nowOwned[glob] = languageId;
    owned.set(extension, languageId);
    if (existing !== languageId) {
      current[glob] = languageId;
      changed = true;
    }
  }
  // Retire the entries we wrote for packs that no longer claim them — but
  // only while the configuration still says what we wrote. An entry the user
  // has since edited is theirs, and stays.
  for (const [glob, weWrote] of Object.entries(previouslyOwned)) {
    if (!wanted.has(glob) && current[glob] === weWrote) {
      delete current[glob];
      changed = true;
    }
  }

  if (changed) {
    await filesConfig.update(
      "associations",
      Object.keys(current).length > 0 ? current : undefined,
      vscode.ConfigurationTarget.Workspace,
    );
    log?.(`[packs] file associations now: ${Object.keys(nowOwned).join(", ") || "(none)"}`);
  }
  if (changed || !sameOwnership(previouslyOwned, nowOwned)) {
    await context.workspaceState.update(OWNED_KEY, nowOwned);
  }

  await retargetOpenDocuments(owned);
  return Object.keys(nowOwned);
}

/**
 * Flip already-open documents onto the language a freshly-registered
 * association gives them.
 *
 * `files.associations` decides the language a document opens *under*; it does
 * nothing for one already open, which is precisely the case a user hits — they
 * open `bar.foo`, see plain text, and only then add the pack. A document
 * already on one of our languages is left alone: an explicit
 * `setTextDocumentLanguage` by the user (or by the highlighting-health nudge)
 * outranks a pack's default.
 *
 * Takes only the associations reconciliation owns, never the whole advertised
 * set — see the `owned` map at the call site. Keyed by the plain lower-case
 * extension, since what it matches is a filename rather than the case-folded
 * glob written to configuration.
 */
async function retargetOpenDocuments(owned: ReadonlyMap<string, string>): Promise<void> {
  if (owned.size === 0) {
    return;
  }
  for (const document of vscode.workspace.textDocuments) {
    if (document.uri.scheme !== "file" || isTclLanguage(document.languageId)) {
      continue;
    }
    const basename = document.fileName.split(/[/\\]/).pop() ?? "";
    const dot = basename.lastIndexOf(".");
    if (dot < 0) {
      continue;
    }
    const target = owned.get(basename.slice(dot + 1).toLowerCase());
    if (target && target !== document.languageId) {
      await vscode.languages.setTextDocumentLanguage(document, target);
    }
  }
}
