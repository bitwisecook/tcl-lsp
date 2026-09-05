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

// The live documentation dock's model: what the author is looking at, what the
// schema says about it, and how that view is named in the URL.
//
// The dock is a *second* surface over the help text the inline `?` panels
// already show — never a second copy of it. Everything here is a pure function
// of the schema the wasm module reports, so the DOM half in `studio.ts` is a
// rendering of a decision made here and the two cannot drift apart.

import type { CodeExample, FieldSchema, PackRow, Schema } from "./types.js";

/**
 * What the dock is currently documenting.
 *
 * The five kinds are the five things a spec author can be *touching*: a
 * setting, the group it sits in, the catalogue a picker draws from, one value
 * out of that catalogue, and the pack a command is being filed under.
 */
export type DockSubject =
  | { kind: "field"; key: string }
  | { kind: "group"; name: string }
  | { kind: "catalogue"; id: string }
  | { kind: "value"; catalogue: string; key: string }
  | { kind: "pack"; id: string };

/** One key inside a cluster, resolved against the schema. */
export interface RelatedLink {
  key: string;
  /** The setting's label, or the bare key when the schema does not know it. */
  label: string;
  /** The subject's own key — shown as the current item rather than a link. */
  self: boolean;
  /** Whether the schema documents this key at all. */
  known: boolean;
}

/** A named cluster of settings that are read together. */
export interface RelatedGroup {
  name: string;
  why: string;
  links: RelatedLink[];
}

/** Everything the dock shows for one subject, in the order it shows it. */
export interface DockContent {
  subject: DockSubject;
  /** The subject's name — the dock's heading, and its collapsed strip. */
  title: string;
  /** Which of the five kinds of thing this is, in words. */
  kindLabel: string;
  /** The schema key or path, shown in monospace beside the title. */
  code: string | null;
  /** The one-line doc, empty when the subject has none. */
  doc: string;
  /** The long-form help already in the schema JSON. */
  help: string;
  /** The annotated example, rendered by `studio.ts`'s `annotatedExample`. */
  example: CodeExample | null;
  related: RelatedGroup[];
}

/** What `describeSubject` reads. Packs come from the pack catalogue. */
export interface DockSources {
  schema: Schema;
  packs: ReadonlyMap<string, PackRow>;
}

/* Anchors ---------------------------------------------------------------- */

/**
 * The stable DOM id of a setting's row in the form.
 *
 * Spec keys are Rust field names, so this is almost always the key untouched;
 * anything else is folded to a character an id and a URL fragment can both
 * carry rather than being rejected.
 */
export function fieldAnchorId(key: string): string {
  return `field-${key.replace(/[^A-Za-z0-9_-]/g, "-")}`;
}

/* Subjects --------------------------------------------------------------- */

/** Every documented key the schema carries, mapped to its display label. */
export function labelIndex(schema: Schema): Map<string, string> {
  const labels = new Map<string, string>();
  for (const field of [...schema.command, ...schema.subcommand, ...schema.nestedFields]) {
    if (!labels.has(field.key)) labels.set(field.key, field.label);
  }
  return labels;
}

/**
 * The clusters `field` belongs to, with each key resolved to a label.
 *
 * `related` is optional in the wire contract on purpose: a studio wasm built
 * before the key existed simply does not send it, and the dock then shows the
 * field's own documentation with no cluster section at all.
 */
export function relatedGroups(
  field: FieldSchema,
  labels: ReadonlyMap<string, string>,
): RelatedGroup[] {
  return (field.related ?? [])
    .map((cluster) => ({
      name: cluster.name,
      why: cluster.why,
      links: cluster.keys.map((key) => ({
        key,
        label: labels.get(key) ?? key,
        self: key === field.key,
        known: labels.has(key),
      })),
    }))
    .filter((cluster) => cluster.links.length > 0);
}

/** The command or subcommand field named `key`, whichever table has it. */
function findField(schema: Schema, key: string): FieldSchema | undefined {
  return (
    schema.command.find((field) => field.key === key) ??
    schema.subcommand.find((field) => field.key === key)
  );
}

/**
 * Everything the dock should show for `subject`, or `null` when the schema
 * knows nothing about it — in which case the dock keeps what it was showing
 * rather than blanking.
 */
export function describeSubject(sources: DockSources, subject: DockSubject): DockContent | null {
  const { schema } = sources;
  switch (subject.kind) {
    case "field": {
      const field = findField(schema, subject.key);
      if (field) {
        return {
          subject,
          title: field.label,
          kindLabel: `Setting · ${field.group}`,
          code: field.key,
          doc: field.doc,
          help: field.help,
          example: field.example,
          related: relatedGroups(field, labelIndex(schema)),
        };
      }
      // A property edited inside a composite row — an option's arity, an
      // argument's role — is documented too, just in another table.
      const nested = schema.nestedFields.find((candidate) => candidate.key === subject.key);
      if (!nested) return null;
      return {
        subject,
        title: nested.label,
        kindLabel: `Setting · inside ${nested.owner}`,
        code: nested.key,
        doc: nested.doc,
        help: nested.help,
        example: nested.example,
        related: [],
      };
    }
    case "group": {
      if (!schema.groups.includes(subject.name)) return null;
      const size = schema.command.filter((field) => field.group === subject.name).length;
      return {
        subject,
        title: subject.name,
        kindLabel: "Form group",
        code: null,
        doc: size ? `${size} setting${size === 1 ? "" : "s"} in this group.` : "",
        help: schema.groupHelp[subject.name] ?? "",
        example: schema.groupExamples[subject.name] ?? null,
        related: [],
      };
    }
    case "catalogue": {
      const help = schema.catalogueHelp[subject.id];
      const variants = schema.catalogues[subject.id];
      if (!help && !variants) return null;
      const size = variants?.length ?? 0;
      return {
        subject,
        title: help?.title ?? subject.id,
        kindLabel: "Catalogue",
        code: subject.id,
        doc: size ? `${size} value${size === 1 ? "" : "s"} to pick from.` : "",
        help: help?.intro ?? "",
        example: help?.example ?? null,
        related: [],
      };
    }
    case "value": {
      const variant = (schema.catalogues[subject.catalogue] ?? []).find(
        (candidate) => candidate.key === subject.key,
      );
      if (!variant) return null;
      const title = schema.catalogueHelp[subject.catalogue]?.title ?? subject.catalogue;
      return {
        subject,
        title: variant.key,
        kindLabel: `Value of ${title}`,
        code: variant.group ?? null,
        doc: variant.doc,
        help: "",
        example: variant.example,
        related: [],
      };
    }
    case "pack": {
      const pack = sources.packs.get(subject.id);
      if (!pack) return null;
      return {
        subject,
        title: pack.label,
        kindLabel: `Pack · ${pack.commands} command${pack.commands === 1 ? "" : "s"}`,
        code: pack.path || pack.id,
        doc: pack.blurb,
        help: "",
        example: null,
        related: [],
      };
    }
  }
}

/* Routing ---------------------------------------------------------------- */

/**
 * A linkable view of the studio.
 *
 * Two shapes, because there are two things worth sending someone: a command
 * (optionally with one of its settings in focus) and a Reference entry.
 */
export type Route =
  | { view: "command"; dialect: string; command: string; field: string | null }
  | { view: "reference"; catalogue: string; variant: string | null };

/** Split a fragment into decoded, non-empty segments. */
function segments(hash: string): string[] {
  return hash
    .replace(/^#/, "")
    .split("/")
    .filter((part) => part !== "")
    .map((part) => {
      try {
        return decodeURIComponent(part);
      } catch {
        // A hand-edited URL with a stray `%` is not a reason to throw during
        // boot; the segment stands for itself and simply will not match.
        return part;
      }
    });
}

/** Read a location fragment, or `null` when it names no view of the studio. */
export function parseHash(hash: string): Route | null {
  const parts = segments(hash);
  if (parts[0] === "c" && parts[1] && parts[2]) {
    return { view: "command", dialect: parts[1], command: parts[2], field: parts[3] ?? null };
  }
  if (parts[0] === "ref" && parts[1]) {
    return { view: "reference", catalogue: parts[1], variant: parts[2] ?? null };
  }
  return null;
}

/** The fragment for `route`, including its leading `#`. */
export function formatHash(route: Route): string {
  const parts =
    route.view === "command"
      ? ["c", route.dialect, route.command, route.field]
      : ["ref", route.catalogue, route.variant];
  return `#/${parts
    .filter((part): part is string => Boolean(part))
    .map(encodeURIComponent)
    .join("/")}`;
}

/**
 * Whether moving from `previous` to `next` earns a session-history entry.
 *
 * Back and forward move between *commands*, not between every setting a
 * pointer touched — so a focus move inside one command replaces the entry it
 * is already on, and anything else pushes a new one.
 */
export function historyMode(previous: Route | null, next: Route): "push" | "replace" {
  if (!previous || previous.view !== next.view) return "push";
  if (previous.view === "command" && next.view === "command") {
    return previous.dialect === next.dialect && previous.command === next.command
      ? "replace"
      : "push";
  }
  if (previous.view === "reference" && next.view === "reference") {
    // Writing the entry you are already on — which is what restoring a route
    // after a Back does — is not a place you went to twice.
    return previous.catalogue === next.catalogue && previous.variant === next.variant
      ? "replace"
      : "push";
  }
  return "push";
}

/** What the dock should document for `route`, when the route names something. */
export function routeSubject(route: Route): DockSubject | null {
  if (route.view === "command") {
    return route.field ? { kind: "field", key: route.field } : null;
  }
  return route.variant
    ? { kind: "value", catalogue: route.catalogue, key: route.variant }
    : { kind: "catalogue", id: route.catalogue };
}
