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
 * The dialects a user can pick, and the label the status bar shows for one.
 *
 * The server owns the list: `tcl-lsp.listDialects` answers with every
 * selectable environment (including the ones a pack declares), and
 * `tcl-lsp.getEffectiveConfig` says which one a document is analysed under.
 * The generated `./chat/dialectCatalog` is the offline copy — the answer before
 * a server has started, and the answer in a host that has none. Kept free of
 * `vscode` imports so both entry points and their unit tests share it.
 */

import { DIALECT_CATALOG, DialectKind } from "./chat/dialectCatalog";

/** One dialect a user can pick. */
export interface DialectChoice {
  /** Canonical name — what `tclLsp.dialect` and `tcl-lsp.setDialect` take. */
  readonly name: string;
  /** Full display name. */
  readonly label: string;
  /** Compact name for the status bar. */
  readonly shortLabel: string;
  /** The group a picker files it under. */
  readonly kind: DialectKind;
  /** One line: the name itself, or a Tcl release plus the packages it loads. */
  readonly description: string;
}

/** The picker's group headings: dialects, then Tcl releases with packages. */
export const DIALECT_GROUP_TITLES: Readonly<Record<DialectKind, string>> = {
  language: "Dialects",
  packages: "Tcl + packages",
};

const KIND_ORDER: readonly DialectKind[] = ["language", "packages"];

function isKind(value: unknown): value is DialectKind {
  return value === "language" || value === "packages";
}

function text(value: unknown): string | undefined {
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

/** The generated catalogue as choices, in selectable order. */
export function catalogueChoices(): DialectChoice[] {
  return DIALECT_CATALOG.map((entry) => ({
    name: entry.name,
    label: entry.label,
    shortLabel: entry.shortLabel,
    kind: entry.kind,
    description: entry.description,
  }));
}

/**
 * The choices in a `tcl-lsp.listDialects` answer, or `undefined` when the value
 * is not a non-empty list of well-formed entries — the caller then keeps the
 * catalogue rather than showing a partial picker.
 */
export function parseListedDialects(value: unknown): DialectChoice[] | undefined {
  if (!Array.isArray(value) || value.length === 0) {
    return undefined;
  }
  const choices: DialectChoice[] = [];
  for (const entry of value as Record<string, unknown>[]) {
    const name = text(entry?.name);
    const label = text(entry?.display_name);
    const shortLabel = text(entry?.short_name);
    const description = text(entry?.description);
    if (!name || !label || !shortLabel || !description || !isKind(entry.kind)) {
      return undefined;
    }
    choices.push({ name, label, shortLabel, kind: entry.kind, description });
  }
  return choices;
}

/**
 * The choice a name refers to. A name no list knows (a dialect the running
 * server has and this build does not) reads as itself.
 */
export function describeDialect(
  name: string,
  known: readonly DialectChoice[] = catalogueChoices(),
): DialectChoice {
  return (
    known.find((choice) => choice.name === name) ?? {
      name,
      label: name,
      shortLabel: name,
      kind: "language",
      description: name,
    }
  );
}

/**
 * The dialect a `tcl-lsp.getEffectiveConfig` answer says a document is
 * analysed under, with the server's own labels. `undefined` when the answer
 * names none.
 */
export function dialectFromEffectiveConfig(
  config: unknown,
  known: readonly DialectChoice[] = catalogueChoices(),
): DialectChoice | undefined {
  const fields = (config ?? {}) as Record<string, unknown>;
  const id = text(fields.dialect_id);
  if (!id) {
    const name = text(fields.dialect);
    return name ? describeDialect(name, known) : undefined;
  }
  const fallback = describeDialect(id, known);
  return {
    name: id,
    label: text(fields.dialect_display_name) ?? fallback.label,
    shortLabel: text(fields.dialect_short_name) ?? fallback.shortLabel,
    kind: isKind(fields.dialect_kind) ? fields.dialect_kind : fallback.kind,
    description: text(fields.dialect_description) ?? fallback.description,
  };
}

/** A group of choices under one heading, in picker order. */
export interface DialectGroup {
  readonly title: string;
  readonly choices: readonly DialectChoice[];
}

/** The choices grouped by kind, skipping a kind that has none. */
export function groupDialectChoices(choices: readonly DialectChoice[]): DialectGroup[] {
  return KIND_ORDER.map((kind) => ({
    title: DIALECT_GROUP_TITLES[kind],
    choices: choices.filter((choice) => choice.kind === kind),
  })).filter((group) => group.choices.length > 0);
}
