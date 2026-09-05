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

// What `/` searches, in what order it answers, and how a row shows why it is
// there.
//
// The registry browser has said what it is viewing since packs became its top
// level (`browserCountLine` in `packs.ts`); the palette searched more surfaces
// than the browser and said less about any of them. A result you cannot place
// is a result you have to open to evaluate, which is the cost the palette
// exists to remove — so every hit carries the surface it came from, the pack
// that declares it, and where in its text the query actually matched.
//
// Pure, like `packs.ts`: the DOM half in `studio.ts` renders a decision made
// here, and the two cannot drift apart.

/** The three places a `/` search looks. */
export type PaletteSurface = "pack" | "registry" | "reference";

/** What choosing a row opens. */
export type PaletteTarget =
  | { open: "command"; name: string; where: "pack" | "registry" }
  | { open: "reference"; catalogue: string; variant: string | null };

/** One row offered to the search, before ranking. */
export interface PaletteCandidate {
  surface: PaletteSurface;
  /** What the row is called — matched first, and highlighted. */
  name: string;
  /** The one-line description, matched after the name. */
  summary: string;
  /** The pack that declares a shipped command; empty on the other surfaces. */
  pack: string;
  target: PaletteTarget;
}

/** A run of text split around the query, so a row can mark what matched. */
export interface Highlight {
  before: string;
  /** The matched run, verbatim from the source text. Empty when none matched. */
  match: string;
  after: string;
}

/** One row the search kept, with the matched runs already located. */
export interface PaletteHit {
  candidate: PaletteCandidate;
  name: Highlight;
  summary: Highlight;
}

/** What the search found, before and after the cap on what is shown. */
export interface PaletteResult {
  hits: PaletteHit[];
  /** Matches per surface across everything searched, not only what is shown. */
  counts: Record<PaletteSurface, number>;
  /** Total matches, before the cap. */
  total: number;
}

/** The names a summary line uses for the two surfaces that have one. */
export interface PaletteNames {
  /** The pack under edit, or empty before one is named. */
  pack: string;
  /** The dialect whose shipped packs were searched. */
  dialect: string;
}

/** Split `text` around the first case-insensitive occurrence of `query`. */
export function highlight(text: string, query: string): Highlight {
  const at = query ? text.toLowerCase().indexOf(query.toLowerCase()) : -1;
  if (at < 0) return { before: text, match: "", after: "" };
  return {
    before: text.slice(0, at),
    match: text.slice(at, at + query.length),
    after: text.slice(at + query.length),
  };
}

/** How well a candidate answers `query`; lower sorts first, -1 is no match. */
function tier(candidate: PaletteCandidate, query: string): number {
  const name = candidate.name.toLowerCase();
  if (name === query) return 0;
  if (name.startsWith(query)) return 1;
  if (name.includes(query)) return 2;
  return candidate.summary.toLowerCase().includes(query) ? 3 : -1;
}

/** The pack under edit is the deliverable, so it answers before the rest. */
const SURFACES = ["pack", "registry", "reference"] as const;
const SURFACE_ORDER: Record<PaletteSurface, number> = { pack: 0, registry: 1, reference: 2 };

/**
 * Rank `candidates` against `query` and keep the best `limit` of them.
 *
 * The order is the order the answer is *wanted* in: an exact name, then a
 * prefix, then a name containing it, then only a summary containing it; within
 * a tier the pack under edit first, then the shipped registry, then the
 * Reference vocabulary; then the shortest name, because a shorter name
 * containing the query is more of it.
 *
 * An empty query ranks nothing — it offers the surfaces in their own order, so
 * opening the palette shows the pack you are writing rather than the alphabet.
 */
export function searchPalette(
  candidates: PaletteCandidate[],
  query: string,
  limit: number,
): PaletteResult {
  const needle = query.trim().toLowerCase();
  const counts: Record<PaletteSurface, number> = { pack: 0, registry: 0, reference: 0 };
  const ranked: { candidate: PaletteCandidate; tier: number; at: number }[] = [];

  candidates.forEach((candidate, at) => {
    const rank = needle ? tier(candidate, needle) : SURFACE_ORDER[candidate.surface];
    if (rank < 0) return;
    counts[candidate.surface] += 1;
    ranked.push({ candidate, tier: rank, at });
  });

  ranked.sort((left, right) => {
    if (left.tier !== right.tier) return left.tier - right.tier;
    const surfaces = SURFACE_ORDER[left.candidate.surface] - SURFACE_ORDER[right.candidate.surface];
    if (surfaces !== 0) return surfaces;
    if (!needle) return left.at - right.at;
    if (left.candidate.name.length !== right.candidate.name.length) {
      return left.candidate.name.length - right.candidate.name.length;
    }
    return left.candidate.name < right.candidate.name ? -1 : 1;
  });

  return {
    hits: ranked.slice(0, limit).map(({ candidate }) => ({
      candidate,
      name: highlight(candidate.name, needle),
      summary: highlight(candidate.summary, needle),
    })),
    counts,
    total: ranked.length,
  };
}

/** How a row names the surface it came from — short, it sits inside a row. */
export function surfaceLabel(surface: PaletteSurface, names: PaletteNames): string {
  if (surface === "pack") return names.pack ? `pack ${names.pack}` : "this pack";
  if (surface === "registry") return `shipped · ${names.dialect}`;
  return "Reference";
}

/** How the summary names a surface — a phrase, it sits inside a sentence. */
function surfacePhrase(surface: PaletteSurface, names: PaletteNames): string {
  if (surface === "pack") return names.pack ? `pack ${names.pack}` : "the pack under edit";
  if (surface === "registry") return `the shipped ${names.dialect} packs`;
  return "the Reference vocabulary";
}

/** The three surfaces in a sentence, as "a, b and c" or "a, b or c". */
function surfaceList(names: PaletteNames, join: "and" | "or"): string {
  const parts = SURFACES.map((surface) => surfacePhrase(surface, names));
  return `${parts.slice(0, -1).join(", ")} ${join} ${parts[parts.length - 1]}`;
}

/**
 * The line above the results: which surfaces were searched, and how much of
 * each answered — the palette's counterpart to `browserCountLine`.
 *
 * A hit's own row says where it came from; this says what was looked in, which
 * is the thing a result list can never show on its own. A surface that matched
 * nothing is still named when nothing matched at all, because "no match" is
 * only useful once you know what was searched.
 */
export function paletteSummary(query: string, result: PaletteResult, names: PaletteNames): string {
  if (!query.trim()) return `Searching ${surfaceList(names, "and")}.`;
  if (!result.total) return `No match in ${surfaceList(names, "or")}.`;
  const where = SURFACES.filter((surface) => result.counts[surface] > 0)
    .map((surface) => `${result.counts[surface]} in ${surfacePhrase(surface, names)}`)
    .join(", ");
  const matches = result.total === 1 ? "match" : "matches";
  const head =
    result.hits.length < result.total
      ? `${result.hits.length} of ${result.total} ${matches}`
      : `${result.total} ${matches}`;
  return `${head} — ${where}`;
}
