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

// The wire contract between the studio's wasm module and this front-end.
//
// Every shape here mirrors a Rust type in `tcl-spec-studio`; the names match
// the JSON that crate emits, so a change on the Rust side shows up as a type
// error here rather than as a runtime surprise.

/** Any value that survives `JSON.parse`. */
export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };

/** A draft spec — a flat object keyed by Rust field name. */
export type Draft = Record<string, Json>;

/** One selectable value in a picker, from `catalogue::Variant`. */
export interface Variant {
  key: string;
  doc: string;
}

/** Catalogue id → its variants. Ids are named by the schema's field kinds. */
export type Catalogues = Record<string, Variant[]>;

/** How a field is edited, from `schema::FieldKind`. */
export interface FieldKind {
  tag: string;
  /** Present on `enum` / `flagSet`: the catalogue to pick from. */
  catalogue?: string;
  /** Present on `enum` / `flagSet`: whether the Rust field is an `Option`. */
  optional?: boolean;
  /** Present on `rustExpr`: an example of the expression shape expected. */
  hint?: string;
}

/** One editable field, from `schema::FieldSchema`. */
export interface FieldSchema {
  key: string;
  label: string;
  doc: string;
  group: string;
  kind: FieldKind;
}

/** The whole schema, from `schema::to_json`. */
export interface Schema {
  groups: string[];
  catalogues: Catalogues;
  command: FieldSchema[];
  subcommand: FieldSchema[];
}

/** One row of the registry browser, from `command_index`. */
export interface IndexEntry {
  name: string;
  summary: string;
  synopsis: string;
  subcommands: number;
  options: number;
  deprecated: boolean;
}

export interface CommandIndex {
  dialect: string;
  commands: IndexEntry[];
}

/** A dialect the studio can browse. */
export interface DialectEntry {
  name: string;
  label: string;
}

/** A rendered artefact — the suggested path plus its contents. */
export interface Rendered {
  path: string;
  source: string;
  error?: string;
}

/** One inferred command from an imported package, with its evidence. */
export interface InferredCommand {
  name: string;
  draft: Draft;
  notes: string[];
}

/** The result of importing a package, from `infer::Import`. */
export interface ImportResult {
  package: string | null;
  version: string | null;
  commands: InferredCommand[];
  warnings: string[];
  error?: string;
}

/** A file gathered for download or a GitHub issue. */
export interface StagedFile {
  path: string;
  source: string;
}

/**
 * The wasm module's exports.
 *
 * Every call takes and returns a JSON string: the Rust side marshals, so the
 * browser needs no generated bindings beyond wasm-bindgen's own glue.
 */
export interface StudioWasm {
  schema(): string;
  dialects(): string;
  command_index(dialect: string): string;
  load_command(name: string, dialect: string): string;
  new_command(): string;
  new_subcommand(): string;
  render_rs(draftJson: string, pack: string): string;
  render_stub(draftsJson: string, mode: string, dialect: string): string;
  import_package(filesJson: string, dialect: string): string;
}

/**
 * wasm-bindgen's `--target no-modules` glue installs this global: calling it
 * with the module bytes returns a promise that resolves once instantiated,
 * after which the exports hang off the same function object.
 */
declare global {
  var wasm_bindgen: ((module: BufferSource) => Promise<unknown>) & StudioWasm;
}
