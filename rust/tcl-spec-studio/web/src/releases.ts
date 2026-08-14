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

// The Releases import mode's data, kept free of the DOM.
//
// One release is one archive plus the label the author has given it. The label
// is the whole basis of every range the importer derives, so it is editable and
// only ever *suggested* from the file name — a wrong guess the author can see
// and fix is safe; a wrong guess made silently is not.

import type { Json } from "./types.js";

/** One file pulled out of a release archive. */
export interface ReleaseEntry {
  name: string;
  text: string;
}

/** One release staged for import. */
export interface Release {
  /** Stable identity for the row, so re-rendering does not lose focus. */
  id: string;
  /** Where the archive came from — a file name or `owner/repo@tag`. */
  origin: string;
  /** The version label, as `package require` would spell it. */
  version: string;
  /** The Tcl sources the archive yielded. */
  entries: ReleaseEntry[];
  /** Members the extractor did not take, and why. */
  skipped: string[];
}

/** `unzip_entries`'s reply. */
export interface UnzipResult {
  entries: ReleaseEntry[];
  skipped: string[];
  members: number;
  bytes: number;
  error?: string;
}

/**
 * Suggest a version label from an archive's file name.
 *
 * The three shapes that actually turn up: GitHub's `Download ZIP` of a tag
 * (`mylib-1.2.0.zip`), a release asset named for the tag (`v1.2.0.zip`), and a
 * hand-rolled `mylib-1.2.0-src.zip`. Anything else yields the empty string —
 * the row then shows as needing a label rather than carrying a fabricated one.
 */
export function versionFromFileName(name: string): string {
  const base = (name.split("/").pop() ?? name).replace(/\.(zip|tar\.gz|tgz)$/i, "");
  // Anchored on a separator (or the start) so `utf8` in `mylib-utf8.zip` is not
  // read as version 8, and allowing a trailing `-src`/`-source` suffix.
  const match =
    /(?:^|[-_@v])(\d+(?:\.\d+)+(?:[.-]?(?:a|b|rc|alpha|beta)\d*)?)(?:[-_](?:src|source|release))?$/i.exec(
      base,
    );
  return match?.[1] ?? "";
}

/** A short, stable id for a staged release row. */
export function releaseId(origin: string, seq: number): string {
  return `rel-${seq}-${origin.replace(/[^\w.-]+/g, "_").slice(0, 40)}`;
}

/**
 * Everything wrong with the staged set that would make an import meaningless.
 *
 * Returned as messages rather than thrown: the panel shows them all at once
 * beside the rows they are about, which is how an author fixes three labels in
 * one pass instead of three.
 */
export function validateReleases(releases: Release[]): string[] {
  const problems: string[] = [];
  if (releases.length === 0) {
    problems.push("Add at least one release archive.");
    return problems;
  }
  if (releases.length === 1) {
    problems.push(
      "One release cannot witness a range — add a second so the importer has something to compare.",
    );
  }
  const seen = new Map<string, number>();
  for (const release of releases) {
    if (!release.version.trim()) {
      problems.push(`${release.origin} has no version label.`);
    }
    if (release.entries.length === 0) {
      problems.push(`${release.origin} yielded no Tcl sources.`);
    }
    const count = (seen.get(release.version.trim()) ?? 0) + 1;
    seen.set(release.version.trim(), count);
  }
  for (const [version, count] of seen) {
    if (version && count > 1) {
      problems.push(`Version ${version} is used by ${count} archives — labels must be unique.`);
    }
  }
  return problems;
}

/** The `snapshots_json` argument `import_package_versions` takes. */
export function snapshotsJson(releases: Release[]): string {
  return JSON.stringify(
    releases.map((release) => ({
      version: release.version.trim(),
      files: release.entries.map((entry) => ({ name: entry.name, text: entry.text })),
    })),
  );
}

/** One command as the multi-release importer reports it. */
export interface VersionedCommand {
  name: string;
  draft: Record<string, Json>;
  notes: string[];
}

/** `import_package_versions`'s reply. */
export interface VersionedImportResult {
  package: string | null;
  versions: string[];
  commands: VersionedCommand[];
  warnings: string[];
  error?: string;
}

/**
 * The lifecycle bounds a merged draft ended up carrying.
 *
 * Read back off the draft rather than out of a side channel, because the draft
 * *is* what gets written into the pack — showing anything else would be showing
 * a claim the author is not actually about to make.
 */
export function lifecycleOf(draft: Record<string, Json>): {
  introduced: string | null;
  retired: string | null;
  deprecated: string | null;
} {
  const read = (key: string): string | null => {
    const value = draft[key];
    return typeof value === "string" && value ? value : null;
  };
  return {
    introduced: read("introduced_version"),
    retired: read("retired_version"),
    deprecated: read("deprecated_version"),
  };
}
