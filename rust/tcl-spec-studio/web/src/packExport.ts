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

// The pack export, as a decision rather than a rendering.
//
// `pack_export` hands back a flat list of artefacts; an author reads a pack
// contribution as three sets — the document the loader reads, the registry
// modules a pull request adds, and the dialect stub. Which set a file belongs
// to, what it is called, which of the two read-only editor surfaces can show
// it, and what the line above the list says are all functions of that reply,
// so they live here and `studio.ts` only paints the answer.

import type { ExportFile, ExportKind, PackExport } from "./types.js";

/**
 * The two stub spellings are one artefact seen two ways, so the pane offers
 * them as a view toggle rather than listing both and inviting a choice
 * between files that say the same thing.
 */
export type StubView = "inline" | "file";

/** Which read-only editor surface can render an artefact. */
export type ExportSurface = "rust" | "tcl";

/** One heading of the file list, with the files filed under it. */
export interface ExportGroup {
  title: string;
  /** What the set is for — one line, above its rows. */
  note: string;
  files: ExportFile[];
}

/** How a row names an artefact's kind. */
export function kindLabel(kind: ExportKind): string {
  switch (kind) {
    case "spectcl":
      return "pack document";
    case "rs":
      return "registry command";
    case "rs-mod":
      return "module collector";
    case "stub-file":
      return "stub file";
    case "stub-inline":
      return "inline stub";
  }
}

/**
 * The studio has exactly two read-only code surfaces, one Rust and one Tcl,
 * and every artefact is written in one of those two languages — so a pane
 * showing one file at a time needs no third surface.
 */
export function surfaceOf(kind: ExportKind): ExportSurface {
  return kind === "rs" || kind === "rs-mod" ? "rust" : "tcl";
}

/** The stub spelling the other view would show. */
function hiddenStub(view: StubView): ExportKind {
  return view === "inline" ? "stub-file" : "stub-inline";
}

/** The export minus whichever stub spelling the view toggle is not showing. */
export function visibleFiles(files: ExportFile[], view: StubView): ExportFile[] {
  const hidden = hiddenStub(view);
  return files.filter((file) => file.kind !== hidden);
}

const GROUPS: { title: string; note: string; kinds: ExportKind[] }[] = [
  {
    title: "Spec pack",
    note: "The document itself — what the language server loads, and the studio's own save file.",
    kinds: ["spectcl"],
  },
  {
    title: "Registry sources",
    note: "Drop-in tcl-registry modules: one per command, plus the mod.rs that declares and collects them.",
    kinds: ["rs", "rs-mod"],
  },
  {
    title: "Dialect stub",
    note: "The same signatures as a stub an editor can read beside the sources it describes.",
    kinds: ["stub-file", "stub-inline"],
  },
];

/**
 * The file list's sections, in the order a contribution is read: the document,
 * then the registry modules, then the stub. A section with nothing in it is
 * dropped rather than shown empty — an empty pack has no `.rs` at all.
 */
export function exportGroups(files: ExportFile[]): ExportGroup[] {
  const groups: ExportGroup[] = [];
  for (const group of GROUPS) {
    const members = files.filter((file) => group.kinds.includes(file.kind));
    if (members.length) groups.push({ title: group.title, note: group.note, files: members });
  }
  return groups;
}

function plural(count: number, noun: string): string {
  return `${count} ${noun}${count === 1 ? "" : "s"}`;
}

/**
 * The line above the list: whose pack this is, how much of it there is, and
 * which dialect the stub was rendered for.
 *
 * `listed` is what the list actually shows, which is one fewer than the export
 * holds whenever the stub toggle is hiding the other spelling.
 */
export function exportSummary(exp: PackExport, dialect: string, listed: number): string {
  if (!exp.commands) return `${exp.pack}: nothing to export yet.`;
  return `${exp.pack}: ${plural(exp.commands, "command")}, ${plural(listed, "file")} for ${dialect}.`;
}

/** What the pane says instead of a list when the pack declares nothing. */
export function emptyExportNotice(exp: PackExport): string {
  return (
    `${exp.pack} declares no commands yet. Add one from the registry, import a package, or ` +
    "write it in the Pack DSL — every file the pack produces then appears here."
  );
}

/**
 * Which file stays selected across a recompute.
 *
 * The export is rebuilt on every document change, so holding the *path* rather
 * than the file keeps the reader where they were while they type — and falls
 * back to the top of the list only when the file they were reading is gone.
 */
export function selectedPath(files: ExportFile[], wanted: string | null): string | null {
  if (wanted !== null && files.some((file) => file.path === wanted)) return wanted;
  return files[0]?.path ?? null;
}

/** The file name a row leads with, and the download's name. */
export function fileBase(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}

/** The directory above it, empty for a file the export puts at the root. */
export function fileDir(path: string): string {
  const cut = path.lastIndexOf("/");
  return cut < 0 ? "" : path.slice(0, cut);
}
