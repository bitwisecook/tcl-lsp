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

/** One arrow beneath a line in a help example. */
export interface HelpAnnotation {
  /** Zero-based source line. */
  line: number;
  /** Exact source text the arrow points at. */
  needle: string;
  /** Explanation printed after the arrow. */
  label: string;
}

/** The sole command token that carries the documented trait. */
export interface HelpCarrier {
  /** Zero-based source line. */
  line: number;
  /** Exact command token to highlight. */
  needle: string;
  /** Accessible explanation of why this token is marked. */
  label: string;
}

/** Small Tcl example shared by form help and the Reference tab. */
export interface CodeExample {
  code: string;
  annotations: HelpAnnotation[];
  /** Present for behavioural traits; independent of the flow arrows. */
  carrier?: HelpCarrier;
}

/** One selectable value in a picker, from `catalogue::Variant`. */
export interface Variant {
  key: string;
  doc: string;
  /** Registry-owned grouping, currently supplied for behavioural traits. */
  group?: string;
  example: CodeExample;
}

/** Catalogue id → its variants. Ids are named by the schema's field kinds. */
export type Catalogues = Record<string, Variant[]>;

/** How a field is edited, from `schema::FieldKind`. */
export interface FieldKind {
  tag: string;
  /** Present on `enum` / `flagSet` / `enumList`: the catalogue to pick from. */
  catalogue?: string;
  /** Present on `enum` / `flagSet`: whether the Rust field is an `Option`. */
  optional?: boolean;
  /** Present on `rustExpr`: an example of the expression shape expected. */
  hint?: string;
}

/**
 * A named cluster of settings that are read together.
 *
 * `pure` contradicting `side_effects`, `arity` against `arity_windows`, a
 * taint source with nothing that checks it: these are decided as a set, and a
 * form that shows one at a time never says so.
 */
export interface RelatedCluster {
  name: string;
  /** One sentence on why the cluster's members constrain each other. */
  why: string;
  /** The spec keys in the cluster, including the field carrying the entry. */
  keys: string[];
}

/** One editable field, from `schema::FieldSchema`. */
export interface FieldSchema {
  key: string;
  label: string;
  doc: string;
  group: string;
  /** Long-form help behind the field's ? button, from `help::field_help`. */
  help: string;
  example: CodeExample;
  kind: FieldKind;
  /**
   * The clusters this field belongs to, one entry each.
   *
   * Optional in the wire contract: a studio wasm built before the key existed
   * sends nothing, and the dock then shows the field alone.
   */
  related?: RelatedCluster[];
}

/** One documented property edited inside a composite field row. */
export interface NestedFieldSchema {
  key: string;
  label: string;
  doc: string;
  /** The Rust type carrying the property — a name, not somewhere to go. */
  owner: string;
  /**
   * The top-level spec key whose editor the property is edited inside, which
   * is the row a link to it can land on.
   *
   * Optional in the wire contract for the same reason `related` is: a studio
   * wasm built before the key existed sends nothing, and the property is then
   * documented without being linkable.
   */
  field?: string;
  group: string;
  help: string;
  example: CodeExample;
}

/** A catalogue's title and introduction, from `help::CATALOGUE_HELP`. */
export interface CatalogueHelp {
  title: string;
  intro: string;
  example: CodeExample;
}

/** The whole schema, from `schema::to_json`. */
export interface Schema {
  groups: string[];
  /** Long-form help per group heading, from `help::GROUP_HELP`. */
  groupHelp: Record<string, string>;
  /** Annotated example per group heading. */
  groupExamples: Record<string, CodeExample>;
  catalogues: Catalogues;
  /** Title and introduction per catalogue id, from `help::CATALOGUE_HELP`. */
  catalogueHelp: Record<string, CatalogueHelp>;
  /** Documented properties inside composite editors such as option rows. */
  nestedFields: NestedFieldSchema[];
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
  /** The `commands/<pack>/` module this very spec is declared in. */
  pack: string;
  /** The other packs declaring the same name — `close` is tcl, expect, irules. */
  also_in: string[];
}

export interface CommandIndex {
  dialect: string;
  commands: IndexEntry[];
}

/** One authoring pack, from `pack_catalogue`. */
export interface PackRow {
  id: string;
  label: string;
  blurb: string;
  /** How many of this dialect's commands the pack contributes. */
  commands: number;
  /** Where the pack lives in the repository, for an author who wants to look. */
  path: string;
}

/**
 * The packs a dialect browses, in the order the studio shows them: the core
 * language, then what layers on it, then the vendor and authoring surfaces.
 */
export interface PackCatalogue {
  dialect: string;
  packs: PackRow[];
}

/** A dialect the studio can browse. */
export interface DialectEntry {
  name: string;
  label: string;
}

/** Source returned by a pure formatter operation. */
export interface Formatted {
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

/* The pack export --------------------------------------------------------- */

/**
 * Which artefact one exported file is, from `pack_export`.
 *
 * `rs-mod` and `rs-mod-add` are the same render under the two ways it can be
 * applied: a whole `mod.rs` for a directory the registry does not ship, and
 * the lines to *add* to the one that is already there when it does. They are
 * two kinds rather than a flag because nothing about a drop-in file may read
 * the same as a patch to a populated one.
 */
export type ExportKind = "spectcl" | "rs" | "rs-mod" | "rs-mod-add" | "stub-file" | "stub-inline";

/** One file a finished pack produces. */
export interface ExportFile {
  kind: ExportKind;
  path: string;
  source: string;
  /** The command a rendered `.rs` came from; absent for every other kind. */
  command?: string;
}

/**
 * Two or more commands the export rendered to one path.
 *
 * Module stems follow the registry's own spelling, so `IP::ttl` and `ip_ttl`
 * are filed apart — but two names that differ only in a character no
 * identifier carries (`a-b` and `a_b`) still meet, and Rust can declare that
 * module once. Which name to change is the author's call, so the export
 * reports the meeting rather than resolving it.
 */
export interface ExportCollision {
  path: string;
  commands: string[];
}

/**
 * Every artefact the pack produces, in one reply.
 *
 * `pack` is the document's own name (`mylib`), not the registry directory the
 * `.rs` files were rendered into — that is an argument to the call.
 */
export interface PackExport {
  pack: string;
  dialect: string;
  commands: number;
  files: ExportFile[];
  collisions: ExportCollision[];
  error?: string;
}

/* The pack store ---------------------------------------------------------- */

/**
 * Where a name's *effective* definition comes from, from `store::Origin`.
 *
 * `shadowed` is the one that matters to an author: the pack declares the name,
 * a shipped spec already has it, and the pack did not say `-override` — so the
 * shipped spec is what an editor would use and the pack's declaration is inert.
 */
export type PackOrigin = "builtin" | "pack" | "override" | "shadowed";

/** One thing the loader dropped, from `spectcl::Notice`. */
export interface PackNotice {
  line: number;
  context: string;
  reason: string;
}

/** A name the pack and the shipped registry both define. */
export interface PackCollision {
  name: string;
  override: boolean;
  effect: "pack-spec-wins" | "shipped-spec-wins";
  reason: string;
}

/** One sidebar row: a pack command and its state at a glance. */
export interface PackCommandRow {
  name: string;
  origin: PackOrigin;
  override: boolean;
  summary: string;
  fields_set: number;
  subcommands: number;
  options: number;
  notices: number;
  unrenderable: number;
}

/** One extension the pack's language is written under. */
export interface PackFileExtension {
  /** Lower-case, without the leading dot (`upf`). */
  extension: string;
  /** What the file type is called (`Unified Power Format`). */
  display_name: string | null;
  /** The dialect profile files of this extension are read as. */
  dialect: string | null;
}

/** The whole store, from `store::Resolution::store_view`. */
export interface PackStoreView {
  pack: string;
  /** The pack's human-readable name (`display_name {IEEE 1801 UPF}`). */
  display_name: string | null;
  file_extensions: PackFileExtension[];
  dsl_version: string;
  dialect: string;
  commands: PackCommandRow[];
  notices: PackNotice[];
  collisions: PackCollision[];
  summary: {
    commands: number;
    notices: number;
    collisions: number;
    shadowed_commands: number;
    bytes: number;
    hooks?: number;
  };
  error?: string;
}

/** The merged view of one command, from `store::Resolution::view`. */
export interface PackCommandView {
  name: string;
  origin: PackOrigin;
  editable: boolean;
  dialect: string;
  /** The declaring pack's own metadata, prefixed: every unprefixed key here
   *  is the command's. */
  pack_display_name: string | null;
  pack_file_extensions: PackFileExtension[];
  override: boolean;
  effective: Draft | null;
  pack: Draft | null;
  builtin: Draft | null;
  notices: PackNotice[];
  error?: string;
}

/** The reply to a write-back: the new document, and how it was reached. */
export interface PackWrite {
  source: string;
  /**
   * How the store wrote the edit: byte splice, full render, vocabulary
   * upgrade — or `patched`, which did not touch `source` at all.
   *
   * A `patched` write is the E-R12 answer to a **programmed** document: the
   * author's program is never rewritten, so the edit stands in a patch pack
   * over it instead. The document pane therefore will not show it, which is
   * the one thing the author has to be told.
   */
  writeback?: "spliced" | "rerendered" | "vocabulary-upgraded" | "patched";
  /** Previous SpecTcl vocabulary when this edit required newer DSL words. */
  upgraded_from?: string;
  /** New declared SpecTcl vocabulary paired with `upgraded_from`. */
  upgraded_to?: string;
  /**
   * Properties the declaration stated before the edit and does not after it.
   *
   * A draft cannot recover a hook body — it holds a function pointer — so a
   * command re-rendered from its draft would lose the author's Tcl. Most such
   * statements are carried across verbatim; anything that could not be is named
   * here so the author is told rather than surprised.
   */
  dropped?: string[];
  name?: string;
  removed?: string;
  error?: string;
}

/* The Test tab ------------------------------------------------------------ */

/** One diagnostic the analyser raised over the sample. */
export interface TestDiagnostic {
  code: string;
  severity: string;
  message: string;
  line: number;
  column: number;
  end_line: number;
  end_column: number;
  start: number;
  end: number;
}

/** One word of the sample, with what the merged registry says about it. */
export interface TestToken {
  start: number;
  end: number;
  line: number;
  column: number;
  text: string;
  index: number;
  depth: number;
  command: string;
  origin: PackOrigin | "unknown";
  /** `command`, `subcommand`, `option`, `option-value`, `terminator`, … */
  kind: string;
  /** The spec property that produced this word, when one did. */
  field: string | null;
  detail: string;
  roles: { role: string; doc: string }[];
  severity: string | null;
}

/**
 * The sample split into chunks covering every byte, in order.
 *
 * Rust emits the text rather than offsets so the front-end never converts a
 * byte index into a JavaScript string index — a conversion that breaks the
 * moment the sample holds a character outside the BMP's ASCII range.
 */
export interface TestChunk {
  text: string;
  token: number | null;
}

/** The whole Test tab reply, from `sample::Bench::analyse`. */
export interface TestReport {
  dialect: string;
  pack: string;
  /** Whether the pack contributed anything to the registry that was used. */
  installed: boolean;
  render: TestChunk[];
  tokens: TestToken[];
  diagnostics: TestDiagnostic[];
  summary: {
    words: number;
    calls: number;
    pack_calls: number;
    unknown_commands: number;
    diagnostics: number;
    errors: number;
    warnings: number;
  };
  error?: string;
}

/** The deep view of one word, from `sample::Bench::inspect`. */
export interface TestInspection {
  word: {
    text: string;
    start: number;
    end: number;
    line: number;
    column: number;
    index: number;
    depth: number;
    kind: string;
  };
  call: { command: string; args: string[]; resolved: boolean };
  spec: {
    name: string;
    origin: PackOrigin | "unknown";
    source: string;
    dialect: string;
    summary: string;
    synopsis: string;
    arity: string;
    subcommands: number;
    options: number;
    required_package: string | null;
    return_type: string;
    fields_set: string[];
  } | null;
  subcommand: {
    name: string;
    detail: string;
    synopsis: string;
    arity: string;
    options: number;
  } | null;
  role: { roles: { role: string; doc: string }[]; field: string | null; detail: string };
  diagnostics: TestDiagnostic[];
  notices: PackNotice[];
  editable: boolean;
  error?: string;
}

/**
 * The wasm module's exports, as the studio calls them.
 *
 * Every call takes and returns a JSON string: the Rust side marshals, so the
 * browser needs no generated bindings beyond wasm-bindgen's own glue. The
 * per-command `render_rs` / `render_stub` are not here: the studio exports the
 * pack, and `pack_export` is what calls both.
 */
export interface StudioWasm {
  schema(): string;
  dialects(): string;
  command_index(dialect: string): string;
  pack_catalogue(dialect: string): string;
  load_command(name: string, dialect: string): string;
  new_command(): string;
  new_subcommand(): string;
  format_pack(source: string): string;
  import_package(filesJson: string, dialect: string): string;

  /* Release archives, read entirely in this page. `unzip_entries` takes the
     archive's bytes and returns its Tcl members as text (plus what it skipped
     and why); `import_package_versions` takes one entry per release and returns
     the merged drafts with the version ranges the releases actually witness.
     `completeHistory` is the caller's claim that the releases given are *all*
     of them, which is what licenses an `introduced_version` on the earliest. */
  unzip_entries(bytes: Uint8Array): string;
  import_package_versions(snapshotsJson: string, dialect: string, completeHistory: boolean): string;

  /* The pack store. Every one of these takes the `.tclspec` document, so the
     browser holds exactly one piece of state and Rust stays a pure function
     of it — which is what makes the DSL pane and the form two projections of
     one thing rather than two stores to reconcile. */
  pack_new(packName: string): string;
  pack_load(source: string, dialect: string): string;
  pack_command(source: string, name: string, dialect: string): string;
  pack_set_command(source: string, name: string, draftJson: string, overrides: boolean): string;
  pack_remove_command(source: string, name: string): string;
  pack_render(source: string): string;
  pack_validate(source: string, dialect: string): string;
  pack_export(source: string, pack: string, dialect: string): string;

  /* The Test tab: the sample analysed with the pack installed, and the
     per-word deep view. `offset` is a **byte** offset into `sample`, which is
     what a token carries — never a JavaScript string index. */
  pack_test_analyse(source: string, sample: string, dialect: string): string;
  pack_test_inspect(source: string, sample: string, dialect: string, offset: number): string;
}

/**
 * wasm-bindgen's `--target no-modules` glue installs this global: calling it
 * with the module bytes returns a promise that resolves once instantiated,
 * after which the exports hang off the same function object.
 */
declare global {
  var wasm_bindgen: ((module: BufferSource) => Promise<unknown>) & StudioWasm;
}
