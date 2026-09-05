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

// Packs as the browser's top level: which authoring pack declares a command,
// and how a dialect's commands divide between them.
//
// A dialect is not a flat thousand names — it is `commands/tcl/` plus
// `commands/tk/` plus whatever else reaches it, and that is the shape a spec
// author works in. Everything here is a pure function of what the wasm module
// reports, so the DOM half stays a rendering of a decision made here.

import type { IndexEntry, PackRow } from "./types.js";

/** The heading commands with no declared pack are filed under. */
export const UNFILED_LABEL = "Other commands";

/** One section of the registry browser. */
export interface PackSection {
  pack: PackRow;
  /** Every command this dialect gets from the pack, alphabetical. */
  commands: IndexEntry[];
  /** Those the filter admits — all of them when there is no filter. */
  matches: IndexEntry[];
}

/** Group an index by the pack that declares each command, names ascending. */
export function groupByPack(index: IndexEntry[]): Map<string, IndexEntry[]> {
  const byPack = new Map<string, IndexEntry[]>();
  for (const entry of index) {
    const list = byPack.get(entry.pack) ?? [];
    list.push(entry);
    byPack.set(entry.pack, list);
  }
  // Code-unit order, which is what the Rust index is already sorted in; doing
  // it again keeps this a function of its argument alone.
  for (const list of byPack.values()) list.sort((a, b) => (a.name < b.name ? -1 : 1));
  return byPack;
}

/** Look a pack up by id, for the chips that name one. */
export function packIndex(catalogue: PackRow[]): Map<string, PackRow> {
  return new Map(catalogue.map((pack) => [pack.id, pack]));
}

/**
 * Whether `entry` survives `query`, which the caller has already trimmed and
 * lower-cased. An empty query admits everything.
 */
export function matchesQuery(entry: IndexEntry, query: string): boolean {
  return (
    !query ||
    entry.name.toLowerCase().includes(query) ||
    (entry.summary ?? "").toLowerCase().includes(query)
  );
}

/**
 * A pack the catalogue does not describe but the index puts commands in.
 *
 * Defensive rather than theoretical: an unknown dialect has an empty
 * catalogue, and a browser that showed nothing at all would be worse than one
 * that shows the commands under a bare heading.
 */
function fallbackPack(id: string, commands: number): PackRow {
  return { id, label: id || UNFILED_LABEL, blurb: "", commands, path: "" };
}

/**
 * The browser's sections, over a [`groupByPack`] grouping: the catalogue's
 * packs in its own order, then any pack only the index knows about.
 *
 * With a filter active, only packs with a match are returned — an author
 * looking for `grid` should not have to read past eleven empty headings.
 */
export function packSections(
  catalogue: PackRow[],
  byPack: Map<string, IndexEntry[]>,
  query: string,
): PackSection[] {
  const sections: PackSection[] = [];
  const listed = new Set<string>();
  const section = (pack: PackRow, commands: IndexEntry[]): PackSection => ({
    pack,
    commands,
    matches: commands.filter((entry) => matchesQuery(entry, query)),
  });

  for (const pack of catalogue) {
    const commands = byPack.get(pack.id);
    if (!commands?.length) continue;
    listed.add(pack.id);
    sections.push(section(pack, commands));
  }
  for (const id of [...byPack.keys()].filter((id) => !listed.has(id)).sort()) {
    const commands = byPack.get(id) ?? [];
    sections.push(section(fallbackPack(id, commands.length), commands));
  }

  return query ? sections.filter((entry) => entry.matches.length > 0) : sections;
}

function plural(count: number, noun: string): string {
  return `${count} ${noun}${count === 1 ? "" : "s"}`;
}

/** One pack header's count: its size, or its share of a filtered result. */
export function packCountLabel(matched: number, total: number): string {
  return matched === total ? plural(total, "command") : `${matched} of ${total}`;
}

/**
 * The line above the browser: what is being viewed, in which dialect, and —
 * once a filter narrows it — across how many packs.
 */
export function browserCountLine(
  dialect: string,
  total: number,
  shown: number,
  packs: number,
): string {
  if (!total) return `no commands in ${dialect}`;
  const commands = total === 1 ? "command" : "commands";
  if (shown === total) return `${total} ${dialect} ${commands} in ${plural(packs, "pack")}`;
  if (!shown) return `no match in ${total} ${dialect} ${commands}`;
  return `${shown} of ${total} ${dialect} ${commands}, in ${plural(packs, "pack")}`;
}

/**
 * What to say when the same command name is authored in more than one pack.
 *
 * Real information for a spec author: `close` is three different specs, and
 * which one an editor uses is the dialect's choice, not the name's.
 */
export function alsoInSentence(name: string, alsoIn: string[]): string {
  const others = alsoIn.filter((id) => id);
  return others.length ? `${name} is also declared in ${others.join(", ")}.` : "";
}
