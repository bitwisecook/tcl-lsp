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

// Controller for the command-registry spec studio.
//
// The form is *generated* from the schema the wasm module reports, never
// hand-written: a new field on `CommandSpec` shows up here with no change to
// this file. What lives here is the app around that form — the registry
// browser, the output panes, the file tray, the GitHub issue, and the package
// importer.

import {
  byId,
  clear,
  clone,
  copyText,
  deepEqual,
  download,
  el,
  setStatus,
  type Child,
} from "./dom.js";
import { assetUrl, verifyAssetVersion, verifyBuildInfo, type BuildInfo } from "./buildInfo.js";
import {
  describeSubject,
  fieldAnchorId,
  formatHash,
  historyMode,
  parseHash,
  routeSubject,
  type DockContent,
  type DockSubject,
  type RelatedGroup,
  type Route,
} from "./docsDock.js";
import { asRecord, asString, makeEditors, STRUCTURAL_KINDS, type Editor } from "./editors.js";
import type { EditorHost, MonacoHostModule } from "./editorHost.js";
import * as idb from "./idb.js";
import { initReleasesPanel } from "./importReleases.js";
import {
  alsoInSentence,
  browserCountLine,
  groupByPack,
  packCountLabel,
  packIndex,
  packSections,
  type PackSection,
} from "./packs.js";
import type {
  CommandIndex,
  CodeExample,
  DialectEntry,
  Draft,
  FieldSchema,
  Formatted,
  ImportResult,
  IndexEntry,
  InferredCommand,
  Json,
  PackCatalogue,
  PackCommandView,
  PackRow,
  PackStoreView,
  PackWrite,
  Rendered,
  Schema,
  StagedFile,
  StudioWasm,
  TestInspection,
  TestReport,
  TestToken,
  Variant,
} from "./types.js";

const GITHUB_REPO = "bitwisecook/tcl-lsp";

/**
 * GitHub rejects very long URLs, so keep the pre-filled body well under the
 * practical limit — past this we fall back to copy-and-paste, which has none.
 */
const MAX_ISSUE_URL = 6000;

/** The registry runs to thousands of commands; the filter box is right there. */
const MAX_LISTED = 400;
/// How many names to offer the browser's native autocomplete at once.
const MAX_SUGGESTED = 50;

const TABS = ["editor", "dsl", "test", "rs", "stub", "import", "reference", "share"] as const;
type Tab = (typeof TABS)[number];

/** How many commands the `/` search offers at once. */
const MAX_PALETTE = 60;

/** The sample the Test tab starts with, before the author writes their own. */
const SAMPLE_HINT = "# Call your pack's commands here.\n";

/** The picker's label per dialect name, filled once from `wasm.dialects()`. */
const dialectLabels = new Map<string, string>();

/**
 * How a sentence names a dialect: the label the picker shows, falling back to
 * the registry name for a dialect the picker does not list.
 */
function dialectLabel(name: string): string {
  return dialectLabels.get(name) ?? name;
}

/**
 * The lazily-imported editor chunk, relative to the page.
 *
 * Loaded on first entry to an editor tab, never at boot: it carries Monaco and
 * the language client, about 2.5 MB of the dist, and a visitor who only browses
 * the registry should not pay for an editor they never open. A failure to load
 * it — an old browser, a `file://` page where module fetches are blocked, a
 * partial deployment — leaves an explicit unavailable editor. There is no
 * second editor implementation whose language behaviour can drift.
 */
const EDITOR_CHUNK = "assets/monaco-host.js";
const NATIVE_EDITOR_CHUNK = "assets/native-editor-host.js";

/** Where the language server worker's three files sit in the dist. */
const LSP_WORKER_DIR = "lsp";

/** How long a keystroke waits before the models are asked to catch up. */
const SETTLE_MS = 120;
/** How long a change waits before it is written to browser storage. */
const SAVE_MS = 250;

/**
 * The pack under edit.
 *
 * `source` is the **whole model**: the `.tclspec` document. `view` is derived
 * from it by the wasm store on every change and holds nothing the document does
 * not already say, and `open` is only place-keeping — which of the pack's
 * commands the form is currently a projection of.
 */
interface PackState {
  source: string;
  view: PackStoreView | null;
  open: string | null;
}

/**
 * Where a visited command came from, so history can reopen it the same way.
 *
 * The two are genuinely different actions — a pack command opens as an
 * editable projection of the document, a registry one as reference material —
 * and going back has to land on the one that was actually open.
 */
interface Visit {
  name: string;
  where: "pack" | "registry";
}

interface State {
  schema: Schema;
  defaultCommand: Draft;
  defaultSubcommand: Draft;
  dialect: string;
  index: IndexEntry[];
  /** The dialect's authoring packs, in the order the browser shows them. */
  packs: PackRow[];
  /** The same packs by id, for the chips that name one. */
  packById: Map<string, PackRow>;
  /** `state.index` divided between those packs — the browser's own shape. */
  byPack: Map<string, IndexEntry[]>;
  /**
   * Which shipped pack sections the author has opened.
   *
   * Kept across dialect switches and reloads: a spec author works in one or
   * two packs for an afternoon, and re-opening `commands/tk/` on every visit
   * is a tax on that.
   */
  expandedPacks: Set<string>;
  draft: Draft | null;
  /**
   * The sentence above the form, kept so the provenance beside it can be
   * redrawn without reloading the draft — a dialect switch can change which
   * pack wins a name (`close` is `tcl`'s in Tcl 9.0 and `irules`' in iRules)
   * without changing the draft at all.
   */
  editorOrigin: string;
  files: StagedFile[];
  imported: InferredCommand[];
  pack: PackState;
  /** The Test tab's sample code — persisted with the pack, per the design. */
  sample: string;
  /** The commands visited, oldest first, and where the cursor sits in them. */
  history: Visit[];
  historyAt: number;
}

let wasm: StudioWasm;
let state: State;
let editors: Record<string, Editor>;
const rendered: { rs: Rendered | null; stub: Rendered | null } = { rs: null, stub: null };
let renderTimer: number | undefined;
let saveTimer: number | undefined;
let testTimer: number | undefined;
/** The last Test tab report, and which word (byte offset) is inspected. */
let testReport: TestReport | null = null;
let inspected: number | null = null;
/** Which tab is showing — the Test tab only re-analyses while it is visible. */
let currentTab: Tab = "editor";
/** True while history is doing the opening, so it does not record its own moves. */
let navigating = false;
/**
 * Whether the draft in the form has been *edited* since it was loaded.
 *
 * Loading a command into the form is not an edit, and must not rewrite the
 * author's file: opening `lsort` and touching nothing has to leave the document
 * byte for byte as it was.
 */
let formDirty = false;
/**
 * The mounted Monaco/LSP surface, once it exists.
 *
 * `null` means no editor tab has mounted Monaco yet, or Monaco failed to load.
 * Every write goes to hidden form/state storage and to this host when present,
 * so the controller and model never disagree.
 */
let editorHost: EditorHost | null = null;
/** Set while the chunk is loading, so two tab clicks do not mount twice. */
let editorMounting: Promise<void> | null = null;
/** Provenance for this assembled page and every external asset it names. */
let activeBuildInfo: BuildInfo;
/** What the documentation dock is showing, or `null` before anything is. */
let dockContent: DockContent | null = null;
/** Whether the dock is expanded. Remembered in the session. */
let dockOpen = true;
/** How long a deep link's landing outline stays on the field it found. */
const FLASH_MS = 1200;

/* Drafts ---------------------------------------------------------------- */

/** Whether `draft` differs from the default for `field`. */
function isSet(draft: Draft, field: FieldSchema, which: "command" | "subcommand"): boolean {
  const base = which === "subcommand" ? state.defaultSubcommand : state.defaultCommand;
  return !deepEqual(draft[field.key], base[field.key]);
}

/* Help ------------------------------------------------------------------ */

/** Long-form help text as a stack of paragraphs. */
function helpParagraphs(text: string): HTMLElement {
  const node = el("div", { class: "helptext" });
  for (const para of text.split(/\n\n+/)) {
    if (para.trim()) node.appendChild(el("p", { text: para }));
  }
  return node;
}

/** A compact code sample with source-aligned arrows beneath the relevant words. */
function annotatedExample(example: CodeExample): HTMLElement {
  const node = el("figure", { class: "code-example" });
  const lines = example.code.split("\n");
  const ordered = example.annotations.map((annotation, index) => ({ annotation, step: index + 1 }));
  lines.forEach((line, lineNumber) => {
    const onLine = ordered
      .filter(({ annotation }) => annotation.line === lineNumber)
      .map(({ annotation, step }) => ({ annotation, step, at: line.indexOf(annotation.needle) }))
      // Draw arrows in source order while retaining semantic step numbers.
      // A nested throw therefore appears as `catch` step 2 before the
      // lexically-later `error` step 1, matching Tcl's evaluation flow.
      .sort((left, right) => left.at - right.at);
    const source = el("pre", { class: "example-line" });
    const carrier = example.carrier?.line === lineNumber ? example.carrier : undefined;
    const carrierAt = carrier ? line.indexOf(carrier.needle) : -1;
    if (carrier && carrierAt >= 0) {
      source.appendChild(document.createTextNode(line.slice(0, carrierAt)));
      source.appendChild(
        el("mark", {
          class: "example-carrier",
          text: carrier.needle,
          title: carrier.label,
        }),
      );
      source.appendChild(document.createTextNode(line.slice(carrierAt + carrier.needle.length)));
    } else {
      source.appendChild(document.createTextNode(line || " "));
    }
    node.appendChild(source);
    for (const { annotation, step, at } of onLine) {
      const arrow =
        `${" ".repeat(Math.max(0, at))}└${"─".repeat(Math.max(1, annotation.needle.length - 1))}` +
        `→ ${step}. ${annotation.label}`;
      node.appendChild(
        el("pre", {
          class: `example-arrow flow-step-${((step - 1) % 4) + 1}`,
          text: arrow,
        }),
      );
    }
  });
  return node;
}

/** Long-form prose followed by its registry-backed attachment example. */
function helpWithExample(text: string, example: CodeExample): HTMLElement {
  const node = helpParagraphs(text);
  node.appendChild(annotatedExample(example));
  return node;
}

/**
 * A `?` button that toggles `panel`. The panel starts hidden; the button
 * carries `aria-expanded` so the state is visible to assistive tech too.
 */
function helpButton(panel: HTMLElement, label: string): HTMLButtonElement {
  panel.hidden = true;
  const button = el("button", {
    type: "button",
    class: "qbtn",
    text: "?",
    title: `About ${label}`,
    "aria-label": `About ${label}`,
    "aria-expanded": "false",
  });
  button.addEventListener("click", (event) => {
    // Inside a <summary>, a plain click would also toggle the group open or
    // closed — the reader asked for help, not for the group to collapse.
    event.preventDefault();
    event.stopPropagation();
    panel.hidden = !panel.hidden;
    button.setAttribute("aria-expanded", panel.hidden ? "false" : "true");
  });
  return button;
}

/* Form ------------------------------------------------------------------ */

function buildForm(
  container: HTMLElement,
  fields: FieldSchema[],
  draft: Draft,
  which: "command" | "subcommand",
  onChange: () => void,
): void {
  const groups = new Map<string, FieldSchema[]>();
  for (const field of fields) {
    const list = groups.get(field.group) ?? [];
    list.push(field);
    groups.set(field.group, list);
  }

  for (const group of state.schema.groups) {
    const list = groups.get(group);
    if (!list?.length) continue;

    const body = el("div", { class: "body" });
    const setCount = list.filter((field) => isSet(draft, field, which)).length;
    const summary = el("summary", {}, [
      document.createTextNode(group),
      el("span", {
        class: "n",
        text: setCount ? `${setCount} set of ${list.length}` : `${list.length} fields`,
      }),
    ]);
    const groupHelp = state.schema.groupHelp[group];
    if (groupHelp) {
      const panel = helpWithExample(groupHelp, state.schema.groupExamples[group]);
      summary.appendChild(helpButton(panel, `the ${group} group`));
      body.appendChild(panel);
    }
    // `data-group` is how the dock recognises a group heading under the
    // pointer or the caret; the same markup serves a nested subcommand form.
    const details = el("details", { class: "group", "data-group": group }, [summary, body]);
    // Open the group the author starts in, plus any group that already
    // carries a non-default value.
    if (setCount > 0 || group === "Identity") details.setAttribute("open", "");

    for (const field of list) body.appendChild(buildField(field, draft, which, onChange));
    container.appendChild(details);
  }
}

function buildField(
  field: FieldSchema,
  draft: Draft,
  which: "command" | "subcommand",
  onChange: () => void,
): HTMLElement {
  const ctl = el("div", { class: "ctl" });
  const help = helpWithExample(field.help, field.example);
  // An enum / flag-set field's vocabulary has a fuller write-up on the
  // Reference tab; link straight to it from the help panel.
  const catalogueId = field.kind.catalogue;
  const catalogue = catalogueId ? state.schema.catalogueHelp[catalogueId] : undefined;
  if (catalogue) {
    help.appendChild(
      el("p", {}, [
        el("button", {
          type: "button",
          class: "ghost",
          text: `All ${catalogue.title.toLowerCase()} on the Reference tab →`,
          onclick: () => openReferenceEntry(catalogueId ?? "", null),
        }),
      ]),
    );
  }
  const lbl = el("div", { class: "lbl" }, [
    el("span", { class: "name", text: field.label }),
    el("code", { class: "key", text: field.key }),
    helpButton(help, field.label),
  ]);
  const node = el("div", { class: "field", "data-field-key": field.key }, [
    lbl,
    el("div", { class: "doc", text: field.doc }),
    help,
    ctl,
  ]);
  // Only the top-level form owns the anchor id. A subcommand's form builds the
  // same keys again inside its row, and a deep link needs exactly one place to
  // land — `data-field-key` still tells the dock what a nested control is.
  if (which === "command") node.id = fieldAnchorId(field.key);

  const markSet = (): void => {
    const existing = lbl.querySelector(".set");
    const on = isSet(draft, field, which);
    if (on && !existing) lbl.appendChild(el("span", { class: "set", text: "set" }));
    if (!on && existing) existing.remove();
  };

  const rebuild = (): void => {
    clear(ctl);
    const editor = editors[field.kind.tag];
    if (!editor) {
      ctl.appendChild(el("span", { class: "hint", text: `no editor for ${field.kind.tag}` }));
      return;
    }
    ctl.appendChild(
      editor(field.kind, draft[field.key] ?? null, (next: Json, structural = true) => {
        draft[field.key] = next;
        markSet();
        // A composite editor reports a plain text edit inside a row as
        // non-structural; rebuilding there would tear down the input the user
        // is typing into.
        if (structural && STRUCTURAL_KINDS.has(field.kind.tag)) rebuild();
        onChange();
      }),
    );
  };

  rebuild();
  markSet();
  return node;
}

/* Registry browser ------------------------------------------------------ */

// Packs are the top level here, not a flat alphabet. A dialect is
// `commands/tcl/` plus whatever layers on it, and an author browsing for
// somewhere to put a command is looking for the pack first. The decisions —
// what is in which pack, what a filter leaves, what the headers say — live in
// `packs.ts`; this half only paints them.

/** One command row, the same markup the browser has always used. */
function commandRow(entry: IndexEntry, current: string | null): HTMLElement {
  const name = el("span", { class: "nm", text: entry.name });
  if (entry.subcommands || entry.options) {
    const badge = [
      entry.subcommands ? `${entry.subcommands} sub` : null,
      entry.options ? `${entry.options} opt` : null,
    ]
      .filter(Boolean)
      .join(" · ");
    name.appendChild(el("span", { class: "badge", text: ` ${badge}` }));
  }
  const button = el(
    "button",
    {
      type: "button",
      "aria-current": current === entry.name ? "true" : "false",
      onclick: () => openCommand(entry.name),
    },
    [name, el("span", { class: "sm", text: entry.summary || entry.synopsis || "" })],
  );
  return el("li", {}, [button]);
}

/** A chip naming the pack a command is declared in. */
function packChip(id: string): HTMLElement {
  const pack = state.packById.get(id);
  return el("span", {
    class: "tag pack",
    text: id,
    title: pack ? `${pack.label} — ${pack.path}` : `declared in ${id}`,
  });
}

/** Record a section's open state, which outlives both filter and dialect. */
function setPackExpanded(id: string, open: boolean): void {
  if (open) state.expandedPacks.add(id);
  else state.expandedPacks.delete(id);
  scheduleSave();
}

/**
 * One pack's section of the browser.
 *
 * `rows` is what the cap left room for, which can be fewer than the section
 * matched; the header always reports the real numbers.
 */
function packSectionNode(
  section: PackSection,
  rows: IndexEntry[],
  open: boolean,
  current: string | null,
): HTMLElement {
  const { pack } = section;
  const summary = el("summary", {}, [
    document.createTextNode(pack.label),
    el("span", {
      class: "n",
      text: packCountLabel(section.matches.length, section.commands.length),
    }),
  ]);
  const body = el("div", { class: "body" });
  if (pack.blurb || pack.path) {
    const blurb = el("div", { class: "helptext packblurb" }, [
      pack.blurb ? el("span", { text: pack.blurb }) : null,
      pack.path ? el("code", { text: pack.path }) : null,
    ]);
    summary.appendChild(helpButton(blurb, `the ${pack.label} pack`));
    body.appendChild(blurb);
  }
  const list = el("ul", { class: "cmdlist" });
  for (const entry of rows) list.appendChild(commandRow(entry, current));
  body.appendChild(list);

  const details = el("details", { class: "group packsection", "data-pack": pack.id }, [
    summary,
    body,
  ]);
  if (open) details.setAttribute("open", "");
  // Read from the click rather than the `toggle` event: only a person clicking
  // (or pressing Enter on) the summary is a preference worth remembering, and
  // the state has not flipped yet when this runs.
  summary.addEventListener("click", () => setPackExpanded(pack.id, !details.open));
  return details;
}

function renderList(): void {
  const query = byId<HTMLInputElement>("filter").value.trim().toLowerCase();
  const browser = byId("cmdlist");
  clear(browser);

  const sections = packSections(state.packs, state.byPack, query);
  const shown = sections.reduce((total, section) => total + section.matches.length, 0);
  byId("count").textContent = browserCountLine(
    dialectLabel(state.dialect),
    state.index.length,
    shown,
    sections.length,
  );

  const current = typeof state.draft?.name === "string" ? state.draft.name : null;
  let room = MAX_LISTED;
  for (const section of sections) {
    const rows = section.matches.slice(0, room);
    room -= rows.length;
    // A filter has already thrown away the packs with nothing in them, so what
    // survives it is worth opening; so is the pack holding the open command,
    // and a dialect with one pack has no navigation to do. A section the cap
    // left no rows for stays shut whatever else is true — an open, empty
    // section reads as a bug rather than as "narrow the filter".
    const open =
      rows.length > 0 &&
      (Boolean(query) ||
        sections.length === 1 ||
        state.expandedPacks.has(section.pack.id) ||
        section.commands.some((entry) => entry.name === current));
    browser.appendChild(packSectionNode(section, rows, open, current));
  }

  // Feed the native autocomplete. The list itself matches summaries too, but
  // an autocomplete that answers "lin" with `::tcl::mathfunc::ceil` (whose
  // *summary* says "linear") is noise — suggestions are name matches only,
  // prefix before substring, so `lindex`/`linsert` come first. Capped: a
  // datalist with thousands of options is slow to open on a phone.
  const options = byId("cmdOptions");
  clear(options);
  if (query) {
    const byName = state.index.filter((e) => e.name.toLowerCase().includes(query));
    const prefix = byName.filter((e) => e.name.toLowerCase().startsWith(query));
    const rest = byName.filter((e) => !e.name.toLowerCase().startsWith(query));
    for (const entry of [...prefix, ...rest].slice(0, MAX_SUGGESTED)) {
      options.appendChild(el("option", { value: entry.name }));
    }
  }

  if (shown > MAX_LISTED) {
    browser.appendChild(
      el("div", {
        class: "more",
        text: `…and ${shown - MAX_LISTED} more — narrow the filter`,
      }),
    );
  }
}

/* The pack store --------------------------------------------------------- */

// One document, many projections. Everything below either *reads* the pack
// source to paint a surface, or *dispatches an edit* that produces a new pack
// source — never both, and no surface keeps a copy of the truth.

/** Parse a wasm reply, turning its `{"error": …}` convention into a throw. */
function unwrap<T extends { error?: string }>(json: string): T {
  const value = JSON.parse(json) as T;
  if (value.error) throw new Error(value.error);
  return value;
}

/**
 * Adopt `source` as the pack document and re-render every projection of it.
 *
 * Monaco preserves its own caret while the formatter's full-document
 * replacement is written back; IDE embeddings update their native document.
 */
function setPackSource(source: string, opts: { refreshForm?: boolean } = {}): void {
  const formatted = unwrap<Formatted>(wasm.format_pack(source)).source;
  state.pack.source = formatted;
  try {
    state.pack.view = unwrap<PackStoreView>(wasm.pack_load(formatted, state.dialect));
    setStatus("dslStatus", "");
  } catch (e) {
    state.pack.view = null;
    setStatus("dslStatus", `could not read the pack: ${message(e)}`, "err");
  }
  writeDsl(formatted);

  // A command the document no longer declares cannot stay open.
  const names = new Set((state.pack.view?.commands ?? []).map((c) => c.name));
  if (state.pack.open && !names.has(state.pack.open)) state.pack.open = null;

  renderPackList();
  renderDslReport();
  if (opts.refreshForm && state.pack.open) refreshOpenCommand();
  // The pack decides what the sample resolves to, so a document change is a
  // test result change — but only pay for it while the tab is on screen.
  if (currentTab === "test") scheduleTest();
  scheduleSave();
}

/** The pack's file name, used for downloads and the file tray. */
function packPath(): string {
  const name = state.pack.view?.pack ?? "";
  return `${name || "pack"}.tclspec`;
}

/** Re-read the open command out of the store and rebuild the form from it. */
function refreshOpenCommand(): void {
  const name = state.pack.open;
  if (!name) return;
  try {
    const view = unwrap<PackCommandView>(wasm.pack_command(state.pack.source, name, state.dialect));
    if (view.pack) loadDraft(view.pack, packOrigin(view), name);
  } catch {
    // The document no longer declares it; the sidebar already says so.
    state.pack.open = null;
  }
}

/** The sentence above the form explaining which definition is being edited. */
function packOrigin(view: PackCommandView): string {
  const where = `Editing ${view.name} in pack ${state.pack.view?.pack ?? ""} — the form and the Pack DSL tab are two views of the same document.`;
  if (view.origin === "shadowed") {
    return `${where} ⚠ ${view.name} is also a shipped ${dialectLabel(view.dialect)} command, and this declaration does not say -override — so an editor would use the shipped spec, not this one.`;
  }
  if (view.origin === "override") {
    return `${where} This declaration replaces the shipped ${dialectLabel(view.dialect)} command of the same name (-override).`;
  }
  return where;
}

/** Open one of the pack's own commands in the form. */
function openPackCommand(name: string): void {
  try {
    const view = unwrap<PackCommandView>(wasm.pack_command(state.pack.source, name, state.dialect));
    if (!view.pack) {
      setStatus("status", `${name} is not declared by this pack`, "err");
      return;
    }
    state.pack.open = name;
    loadDraft(view.pack, packOrigin(view), name);
    selectTab("editor");
    setStatus("status", "");
    pushHistory({ name, where: "pack" });
    scheduleSave();
  } catch (e) {
    setStatus("status", `could not open ${name}: ${message(e)}`, "err");
  }
}

/**
 * Push the form's draft into the document.
 *
 * This is the *only* path from a form edit to the DSL text, and it runs on
 * every settled keystroke — which is what makes "live everywhere" a property of
 * the architecture rather than a matrix of pairwise syncs.
 */
function writeBackOpenCommand(): void {
  const target = state.pack.open;
  if (!target || !state.draft || !formDirty) return;
  const overrides = state.pack.view?.commands.find((c) => c.name === target)?.override ?? false;
  try {
    const written = unwrap<PackWrite>(
      wasm.pack_set_command(state.pack.source, target, JSON.stringify(state.draft), overrides),
    );
    // A rename is an ordinary edit: the declaration keeps its place in the
    // file and the sidebar follows it to its new name.
    const renamed = typeof state.draft.name === "string" ? state.draft.name : target;
    state.pack.open = renamed;
    setPackSource(written.source);
    // Both of these are losses the author has a right to hear about the moment
    // they happen, not when they read the file back.
    if (written.dropped?.length) {
      setStatus(
        "dslStatus",
        `this edit could not keep ${written.dropped.join(", ")} — a hook body the draft model ` +
          `cannot re-render and the write could not reach. Undo, or restore it by hand in this pane.`,
        "err",
      );
    } else if (written.upgraded_from) {
      setStatus("dslStatus", vocabularyUpgradeMessage(written) ?? "", "ok");
    } else if (written.writeback === "patched") {
      setStatus(
        "dslStatus",
        "this document is a program, so it was not rewritten — the edit stands as a patch over it and will not appear in this pane",
        "ok",
      );
    } else if (written.writeback === "rerendered") {
      setStatus(
        "dslStatus",
        "the whole document was re-rendered — a targeted edit could not be verified, so comments and layout were rebuilt",
        "err",
      );
    }
  } catch (e) {
    setStatus("dslStatus", `could not write the edit back: ${message(e)}`, "err");
  }
}

/** Copy whatever is in the editor into the pack, and start editing it there. */
function addDraftToPack(): void {
  if (!state.draft) return;
  const name = typeof state.draft.name === "string" ? state.draft.name : "";
  if (!name) {
    setStatus("status", "give the command a name first", "err");
    return;
  }
  try {
    // Preserve an existing declaration's collision policy. In particular, a
    // registry command selected below is seeded as `-override`; pressing this
    // button again must not silently turn its live replacement into a shadowed
    // reference copy.
    const overrides =
      state.pack.view?.commands.find((command) => command.name === name)?.override ?? false;
    const written = unwrap<PackWrite>(
      wasm.pack_set_command(state.pack.source, name, JSON.stringify(state.draft), overrides),
    );
    state.pack.open = name;
    setPackSource(written.source);
    openPackCommand(name);
    setStatus(
      "status",
      vocabularyUpgradeMessage(written) ?? `${name} added to pack ${state.pack.view?.pack ?? ""}`,
      "ok",
    );
  } catch (e) {
    setStatus("status", `could not add ${name} to the pack: ${message(e)}`, "err");
  }
}

/** Drop a command from the pack. */
function removeFromPack(name: string): void {
  try {
    const written = unwrap<PackWrite>(wasm.pack_remove_command(state.pack.source, name));
    if (state.pack.open === name) state.pack.open = null;
    setPackSource(written.source);
    setStatus("status", `${name} removed from the pack`, "ok");
  } catch (e) {
    setStatus("status", message(e), "err");
  }
}

/**
 * The pack's own identity line: what the library is called and what its files
 * are called — `IEEE 1801 UPF — .upf (Unified Power Format)`.
 *
 * Empty when the document declares neither, which is the common case for a
 * pack that only adds commands to an existing language.
 */
function packMetaLine(view: PackStoreView): string {
  const extensions = view.file_extensions.map((row) =>
    row.display_name ? `.${row.extension} (${row.display_name})` : `.${row.extension}`,
  );
  return [view.display_name ?? "", extensions.join(", ")].filter((part) => part).join(" — ");
}

/** The always-visible list of the pack's own commands. */
function renderPackList(): void {
  const view = state.pack.view;
  const list = byId("packlist");
  clear(list);
  byId("packName").textContent = view?.pack || "—";
  const meta = byId("packMeta");
  meta.textContent = view ? packMetaLine(view) : "";
  meta.hidden = !meta.textContent;

  const rows = view?.commands ?? [];
  byId("packCount").textContent = rows.length
    ? `${rows.length} command${rows.length === 1 ? "" : "s"}` +
      (view && view.summary.notices ? ` · ${view.summary.notices} notice(s)` : "")
    : "empty";

  if (!rows.length) {
    list.appendChild(
      el("li", {}, [
        el("span", {
          class: "empty",
          text: "add a command, or open a .tclspec on the Pack DSL tab",
        }),
      ]),
    );
    return;
  }

  for (const row of rows) {
    const name = el("span", { class: "nm", text: row.name });
    const tags = el("span", { class: "state" });
    const tag = (text: string, kind = "tag"): void => {
      tags.appendChild(el("span", { class: kind, text }));
    };
    tag(`${row.fields_set} field${row.fields_set === 1 ? "" : "s"}`);
    if (row.subcommands) tag(`${row.subcommands} sub`);
    if (row.options) tag(`${row.options} opt`);
    if (row.origin === "override") tag("overrides shipped", "tag live");
    if (row.origin === "shadowed") tag("shadowed by shipped", "tag warn");
    if (row.notices) tag(`${row.notices} notice`, "tag warn");
    if (row.unrenderable) tag(`${row.unrenderable} unreadable`, "tag warn");

    const button = el(
      "button",
      {
        type: "button",
        "aria-current": state.pack.open === row.name ? "true" : "false",
        onclick: () => openPackCommand(row.name),
      },
      [name, el("span", { class: "sm", text: row.summary || "" }), tags],
    );
    const remove = el("button", {
      type: "button",
      class: "rm",
      title: `remove ${row.name} from the pack`,
      "aria-label": `remove ${row.name} from the pack`,
      text: "✕",
      onclick: () => removeFromPack(row.name),
    });
    list.appendChild(el("li", { class: "packrow" }, [button, remove]));
  }
}

/** The DSL pane's report: what the loader dropped, and what collides. */
function renderDslReport(): void {
  const out = byId("dslReport");
  clear(out);
  byId("dslPath").textContent = packPath();
  const view = state.pack.view;
  if (!view) return;

  if (view.notices.length) {
    const body = el("div", { class: "body" });
    for (const notice of view.notices) {
      body.appendChild(
        el("div", { class: "dslrow" }, [
          el("span", { class: "where", text: `${notice.context}:${notice.line}` }),
          el("span", { class: "what", text: notice.reason }),
        ]),
      );
    }
    out.appendChild(
      el("details", { class: "group", open: true }, [
        el("summary", {}, [
          document.createTextNode("Dropped by the loader"),
          el("span", { class: "n", text: `${view.notices.length}` }),
        ]),
        body,
      ]),
    );
  }

  if (view.collisions.length) {
    const body = el("div", { class: "body" });
    for (const collision of view.collisions) {
      body.appendChild(
        el("div", { class: "dslrow" }, [
          el("span", { class: "where", text: collision.name }),
          el("span", { class: "what", text: collision.reason }),
        ]),
      );
    }
    out.appendChild(
      el("details", { class: "group", open: true }, [
        el("summary", {}, [
          document.createTextNode(
            `Collisions with the shipped ${dialectLabel(view.dialect)} registry`,
          ),
          el("span", { class: "n", text: `${view.collisions.length}` }),
        ]),
        body,
      ]),
    );
  }

  if (!view.notices.length && !view.collisions.length && view.commands.length) {
    setStatus(
      "dslStatus",
      `${view.summary.commands} command(s), nothing dropped, no collision with the shipped ${dialectLabel(view.dialect)} registry`,
      "ok",
    );
  }
}

/** Read a `.tclspec` off disk into the store. */
function readPackFile(files: FileList | null): void {
  const file = files?.[0];
  if (!file) return;
  file.text().then(
    (text) => {
      state.pack.open = null;
      setPackSource(text);
      selectTab("dsl");
      setStatus("dslStatus", `loaded ${file.name}`, "ok");
    },
    (e: unknown) => setStatus("dslStatus", `could not read ${file.name}: ${String(e)}`, "err"),
  );
}

/* Live save -------------------------------------------------------------- */

function scheduleSave(): void {
  if (saveTimer !== undefined) window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    void idb.save({
      source: state.pack.source,
      open: state.pack.open,
      dialect: state.dialect,
      sample: state.sample,
      expanded: [...state.expandedPacks],
      dockOpen,
    });
  }, SAVE_MS);
}

/* Editing --------------------------------------------------------------- */

function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

function vocabularyUpgradeMessage(write: PackWrite): string | null {
  return write.upgraded_from
    ? `Pack DSL upgraded from SpecTcl ${write.upgraded_from} to ${write.upgraded_to ?? "the current vocabulary"} because this command uses newer vocabulary.`
    : null;
}

/**
 * Load `draft` into the form.
 *
 * `packCommand` names the pack declaration this draft came from, or is `null`
 * when the draft is fresh — registry selections are first copied into the
 * working pack, so the form and Pack DSL remain projections of one document.
 */
function loadDraft(draft: Draft, origin: string, packCommand: string | null = null): void {
  state.draft = draft;
  state.pack.open = packCommand;
  formDirty = false;
  state.editorOrigin = origin;
  renderEditorSource();

  renderUnrenderableWarning(draft);

  const form = byId("form");
  clear(form);
  buildForm(form, state.schema.command, draft, "command", onFormEdit);
  renderList();
  renderPackList();
  onDraftChanged();
}

/**
 * The sentence above the form, plus the provenance the sentence leaves out:
 * which pack ships this name, and — the thing a spec author cannot guess —
 * which other packs declare it too.
 */
function renderEditorSource(): void {
  const node = byId("editorSource");
  clear(node);
  node.appendChild(document.createTextNode(state.editorOrigin));
  const name = typeof state.draft?.name === "string" ? state.draft.name : null;
  const entry = name ? state.index.find((candidate) => candidate.name === name) : undefined;
  if (!entry) return;
  node.appendChild(document.createTextNode(" "));
  node.appendChild(packChip(entry.pack));
  const also = alsoInSentence(entry.name, entry.also_in ?? []);
  if (also) node.appendChild(el("span", { class: "alsoin", text: ` ${also}` }));
}

/**
 * A field in the form actually changed.
 *
 * Separate from [`onDraftChanged`] because *loading* a draft also has to
 * re-render the output panes, and must not be mistaken for an edit — see
 * [`formDirty`].
 */
function onFormEdit(): void {
  formDirty = true;
  onDraftChanged();
}

/// Render the "cannot read back" panel for `draft`.
///
/// Recomputed on every draft change, not just on load: the option-arity hook
/// entry names the options still missing a hook, so supplying one has to clear
/// it. Field entries are static, but running them through the same path keeps
/// one code path rather than two.
function renderUnrenderableWarning(draft: Record<string, Json>): void {
  const lost = Array.isArray(draft.__unrenderable) ? draft.__unrenderable : [];
  const pending = lost.filter((key) => !isLostResolved(draft, key));
  const warn = byId("unrenderable");
  if (!pending.length) {
    warn.hidden = true;
    return;
  }
  warn.hidden = false;
  clear(warn);
  warn.appendChild(
    el("b", {
      text: `This command sets ${pending.length} field${pending.length === 1 ? "" : "s"} the studio cannot read back.`,
    }),
  );
  warn.appendChild(
    document.createTextNode(
      " Rust can tell the field is set but not recover the expression that set it — a function pointer or a reference to a static descriptor. Each is listed below and in the rendered file as a TODO; fill it in where it appears in the form (most sit under Advanced) to emit it.",
    ),
  );
  const ul = el("ul", {});
  for (const key of pending)
    ul.appendChild(el("li", {}, [el("code", { text: lostLabel(draft, key) })]));
  warn.appendChild(ul);
}

/// Whether the author has since supplied what `key` was missing.
///
/// Mirrors the renderer's own test, so the panel and the emitted `TODO` agree.
function isLostResolved(draft: Record<string, Json>, key: unknown): boolean {
  if (String(key) === "options.arity_hook") return unfilledHooks(draft).length === 0;
  const value = draft[String(key)];
  return typeof value === "string" && value.trim() !== "";
}

/// Option names whose arity is a hook with no expression supplied yet.
function unfilledHooks(draft: Record<string, Json>): string[] {
  const opts = Array.isArray(draft.options) ? draft.options : [];
  return opts
    .filter((o) => {
      const arity = asRecord(asRecord(asRecord(o).value).arity);
      return asString(arity.kind) === "Hook" && !asString(arity.hook).trim();
    })
    .map((o) => asString(asRecord(o).name))
    .filter(Boolean);
}

/// Display text for one `__unrenderable` entry.
///
/// Most entries are a spec field name and read fine as-is. Option-arity hooks
/// are not a top-level field — they sit in an option row — so name the options
/// that still need one, which is where the author has to go.
function lostLabel(draft: Record<string, Json>, key: unknown): string {
  const name = String(key);
  if (name !== "options.arity_hook") return name;
  const pending = unfilledHooks(draft);
  return pending.length ? `options → ${pending.join(", ")} (arity hook)` : name;
}

/// Load whatever the filter box currently names.
///
/// Typing a name only ever narrowed the list, and on a phone the list is below
/// the fold — so a typed name looked like it did nothing at all. Resolve it
/// here instead: an exact name wins, then a unique case-insensitive match, then
/// a sole surviving filter match. Anything else is reported rather than
/// guessed, because loading the wrong command silently is worse than saying so.
function loadTypedCommand(): void {
  const typed = byId<HTMLInputElement>("filter").value.trim();
  if (!typed) {
    setStatus("status", "type a command name first", "err");
    return;
  }
  const lower = typed.toLowerCase();
  const exact = state.index.find((e) => e.name === typed);
  const caseless = state.index.filter((e) => e.name.toLowerCase() === lower);
  const partial = state.index.filter((e) => e.name.toLowerCase().includes(lower));

  const target =
    exact?.name ??
    (caseless.length === 1 ? caseless[0]?.name : undefined) ??
    (partial.length === 1 ? partial[0]?.name : undefined);

  if (target) {
    openCommand(target);
    return;
  }
  if (partial.length === 0) {
    setStatus("status", `no command matches “${typed}” in ${dialectLabel(state.dialect)}`, "err");
    return;
  }
  setStatus(
    "status",
    `${partial.length} commands match “${typed}” — pick one from the list`,
    "err",
  );
}

function openCommand(name: string): void {
  // A pack declaration is the editable source of truth, even when the same
  // name also appears in the registry list. Re-opening it must not overwrite
  // the author's in-progress pack edit with a fresh shipped snapshot.
  if (state.pack.view?.commands.some((command) => command.name === name)) {
    openPackCommand(name);
    return;
  }
  try {
    const loaded = JSON.parse(wasm.load_command(name, state.dialect)) as Draft & { error?: string };
    if (loaded.error) {
      setStatus("status", loaded.error, "err");
      return;
    }
    // The Pack DSL is deliberately the one authoritative document rather than
    // a second, export-only projection. Seed a selected shipped command into
    // it before displaying the form, and use `-override` so the selected draft
    // is also the one the pack would install over the registry.
    const written = unwrap<PackWrite>(
      wasm.pack_set_command(state.pack.source, name, JSON.stringify(loaded), true),
    );
    state.pack.open = name;
    setPackSource(written.source);
    const view = unwrap<PackCommandView>(wasm.pack_command(state.pack.source, name, state.dialect));
    if (!view.pack) throw new Error(`the pack did not retain ${name}`);
    loadDraft(view.pack, packOrigin(view), name);
    const upgraded = vocabularyUpgradeMessage(written);
    setStatus("status", upgraded ?? "", upgraded ? "ok" : undefined);
    pushHistory({ name, where: "registry" });
  } catch (e) {
    setStatus("status", `could not load ${name}: ${String(e)}`, "err");
  }
}

function onDraftChanged(): void {
  if (renderTimer !== undefined) window.clearTimeout(renderTimer);
  renderTimer = window.setTimeout(renderOutputs, SETTLE_MS);
}

function renderOutputs(): void {
  if (!state.draft) return;
  // The document first: every other pane renders from the draft, but the
  // draft's home is the pack source, and an edit is not real until it is
  // written there.
  writeBackOpenCommand();
  renderUnrenderableWarning(state.draft);
  try {
    const pack = byId<HTMLInputElement>("rsPack").value || "tcl";
    const rs = JSON.parse(wasm.render_rs(JSON.stringify(state.draft), pack)) as Rendered;
    if (rs.error) throw new Error(rs.error);
    rendered.rs = rs;
    byId("rsOut").firstElementChild!.textContent = rs.source;
    editorHost?.setRustText(rs.source);
    byId("rsPath").textContent = rs.path;

    const mode = byId<HTMLSelectElement>("stubMode").value;
    const stub = JSON.parse(
      wasm.render_stub(JSON.stringify([state.draft]), mode, state.dialect),
    ) as Rendered;
    if (stub.error) throw new Error(stub.error);
    rendered.stub = stub;
    byId("stubOut").firstElementChild!.textContent = stub.source;
    editorHost?.setStubText(stub.source);
    byId("stubPath").textContent = stub.path;
  } catch (e) {
    setStatus("status", `render failed: ${e instanceof Error ? e.message : String(e)}`, "err");
  }
}

/* Files ----------------------------------------------------------------- */

function addFile(path: string, source: string): void {
  const existing = state.files.find((file) => file.path === path);
  if (existing) existing.source = source;
  else state.files.push({ path, source });
  renderFiles();
  setStatus("status", `${path.split("/").pop()} added to files`, "ok");
}

function renderFiles(): void {
  const list = byId("fileList");
  clear(list);
  if (!state.files.length) {
    list.appendChild(
      el("li", {}, [
        el("span", {
          class: "sm",
          text: "No files yet — render a spec and choose “Add to files”.",
        }),
      ]),
    );
  }
  state.files.forEach((file, i) => {
    list.appendChild(
      el("li", {}, [
        el("span", { class: "name", text: file.path }),
        el("span", { class: "sz", text: `${(file.source.length / 1024).toFixed(1)} KiB` }),
        el("button", {
          type: "button",
          class: "rm",
          "aria-label": `remove ${file.path}`,
          text: "✕",
          onclick: () => {
            state.files.splice(i, 1);
            renderFiles();
          },
        }),
      ]),
    );
  });
  updateIssueSize();
}

/* GitHub issue ---------------------------------------------------------- */

function issueBody(): string {
  const notes = byId<HTMLTextAreaElement>("issueNotes").value.trim();
  const parts: string[] = [
    notes || "_Describe the command and where its behaviour is documented._",
    "",
    "Produced with the tcl-lsp command-registry spec studio.",
    "",
  ];
  for (const file of state.files) {
    const ext = file.path.slice(file.path.lastIndexOf(".") + 1);
    const lang = ext === "rs" ? "rust" : ext === "stubs" || ext === "tcl" ? "tcl" : "";
    parts.push(`### \`${file.path}\``, "", "```" + lang, file.source.replace(/\r/g, ""), "```", "");
  }
  return parts.join("\n");
}

function issueTitle(): string {
  return byId<HTMLInputElement>("issueTitle").value.trim() || "Command registry: new command spec";
}

function issueUrl(): string {
  return (
    `https://github.com/${GITHUB_REPO}/issues/new?title=` +
    encodeURIComponent(issueTitle()) +
    "&body=" +
    encodeURIComponent(issueBody())
  );
}

function updateIssueSize(): void {
  const node = byId("issueSize");
  if (!state.files.length) {
    node.className = "note info";
    node.textContent = "Add at least one rendered file, or write notes, before opening an issue.";
    return;
  }
  const size = Math.round(issueUrl().length / 1024);
  if (issueUrl().length > MAX_ISSUE_URL) {
    node.className = "note warn";
    node.textContent =
      `${state.files.length} file(s), ${size} KiB — too large to pre-fill through a URL. ` +
      "Use “Copy the issue body”, then paste it into the blank issue form that opens.";
  } else {
    node.className = "note info";
    node.textContent = `${state.files.length} file(s) will be embedded in the issue body (${size} KiB).`;
  }
}

function openIssue(): void {
  if (!state.files.length && !byId<HTMLTextAreaElement>("issueNotes").value.trim()) {
    setStatus("issueStatus", "nothing to report yet", "err");
    return;
  }
  // Download the files too: GitHub's issue form cannot accept attachments from
  // a link, so having them on disk is what makes attaching them possible —
  // drag them onto the issue once it opens.
  for (const file of state.files) download(file.path, file.source);

  let url = issueUrl();
  if (url.length > MAX_ISSUE_URL) {
    copyText(issueBody(), "issueStatus");
    url = `https://github.com/${GITHUB_REPO}/issues/new?title=${encodeURIComponent(issueTitle())}`;
    setStatus("issueStatus", "body copied — paste it into the form that just opened", "ok");
  } else {
    setStatus("issueStatus", "issue opened in a new tab", "ok");
  }
  window.open(url, "_blank", "noopener");
}

/* Package import -------------------------------------------------------- */

/** The extensions a Tcl library is written in — what a directory pass keeps. */
const TCL_EXTENSIONS = [".tcl", ".tm", ".test", ".itcl", ".itk"];

/**
 * Read a chosen set of files and infer a spec for every `proc` in them.
 *
 * A directory pass hands over everything under the folder, so it is filtered
 * to Tcl sources here — the alternative is analysing a `.png` and reporting
 * that it had no procedures. File names keep their path relative to the
 * chosen directory, because "which file was this proc in" is the first thing
 * an author checks in the evidence.
 */
function readFiles(fileList: FileList | null, opts: { directory?: boolean } = {}): void {
  let files = Array.from(fileList ?? []);
  if (!files.length) return;
  const total = files.length;
  if (opts.directory) {
    files = files.filter((file) =>
      TCL_EXTENSIONS.some((ext) => file.name.toLowerCase().endsWith(ext)),
    );
    if (!files.length) {
      setStatus(
        "importStatus",
        `no Tcl sources in that directory (looked at ${total} file(s) for ${TCL_EXTENSIONS.join(", ")})`,
        "err",
      );
      return;
    }
  }
  setStatus(
    "importStatus",
    opts.directory
      ? `reading ${files.length} Tcl source(s) of ${total} file(s)…`
      : `reading ${files.length} file(s)…`,
  );
  Promise.all(
    files.map(async (file) => ({
      name: file.webkitRelativePath || file.name,
      text: await file.text(),
    })),
  ).then(
    (payload) => {
      setStatus("importStatus", `analysing ${payload.length} file(s)…`);
      // Let the status paint before the analyser blocks the main thread.
      window.setTimeout(() => runImport(payload), 0);
    },
    (e: unknown) => setStatus("importStatus", `could not read the files: ${String(e)}`, "err"),
  );
}

/**
 * Write every inferred command into the pack document.
 *
 * The importer's whole point is the *library*, so the finish line is the pack,
 * not a list of drafts to open one at a time. Each write goes through the same
 * store call a form edit does, so the document that results is exactly what
 * the DSL pane shows and what live-save persists.
 */
function writeDraftsToPack(commands: { name: string; draft: Draft }[]): {
  written: number;
  failed: string[];
  firstWritten: string | null;
  patched: number;
} {
  let source = state.pack.source;
  let written = 0;
  let patched = 0;
  const failed: string[] = [];
  let firstWritten: string | null = null;
  for (const found of commands) {
    try {
      const out = unwrap<PackWrite>(
        wasm.pack_set_command(source, found.name, JSON.stringify(found.draft), false),
      );
      source = out.source;
      written += 1;
      if (out.writeback === "patched") patched += 1;
      firstWritten ??= found.name;
    } catch {
      failed.push(found.name);
    }
  }
  state.pack.open = null;
  setPackSource(source);
  return { written, failed, firstWritten, patched };
}

function addImportedToPack(): void {
  if (!state.imported.length) return;
  const { written, failed, firstWritten, patched } = writeDraftsToPack(state.imported);
  // The generated-code panes render the active draft, not the pack as a
  // whole. Import used to write a perfectly good `.tclspec` and leave the
  // boot-time `mycommand` placeholder active, so Rust and stub output looked
  // as though generation had failed. Make the first successfully imported
  // command the active pack draft while leaving the author on the Import tab.
  //
  // Reading it back can still fail — the store is the authority on what it
  // holds — and a throw out of a click handler leaves the page half-updated
  // with nothing said. Report it in the panel instead.
  let unreadable: string | null = null;
  if (firstWritten) {
    try {
      const view = unwrap<PackCommandView>(
        wasm.pack_command(state.pack.source, firstWritten, state.dialect),
      );
      if (view.pack) loadDraft(view.pack, packOrigin(view), firstWritten);
    } catch (e) {
      unreadable = message(e);
    }
  }
  setStatus(
    "importStatus",
    `${written} command(s) written into pack ${state.pack.view?.pack ?? ""}` +
      // E-R12: a programmed document is never rewritten, so the write stands
      // in a patch pack over it. Say so — the DSL pane will not show it.
      (patched
        ? ` — ${patched} of them as a patch over this document, which is a program and is not rewritten`
        : "") +
      (failed.length ? ` — ${failed.length} could not be written: ${failed.join(", ")}` : "") +
      (unreadable ? ` — could not reopen ${firstWritten ?? ""}: ${unreadable}` : ""),
    failed.length || unreadable ? "err" : "ok",
  );
}

function runImport(payload: { name: string; text: string }[]): void {
  let result: ImportResult;
  try {
    result = JSON.parse(
      wasm.import_package(JSON.stringify(payload), state.dialect),
    ) as ImportResult;
  } catch (e) {
    setStatus("importStatus", `import failed: ${String(e)}`, "err");
    return;
  }
  if (result.error) {
    setStatus("importStatus", result.error, "err");
    return;
  }
  state.imported = result.commands ?? [];

  const out = byId("importOut");
  clear(out);

  if (result.package) {
    out.appendChild(
      el("div", { class: "note info" }, [
        document.createTextNode("Package "),
        el("code", { text: result.package + (result.version ? ` ${result.version}` : "") }),
        document.createTextNode(" — every command below is gated on a matching `package require`."),
      ]),
    );
  }

  for (const warning of result.warnings ?? []) {
    out.appendChild(el("div", { class: "note warn", text: warning }));
  }

  for (const found of state.imported) {
    const evidence = el("ul", { class: "evidence" });
    for (const note of found.notes ?? []) evidence.appendChild(el("li", { text: note }));
    out.appendChild(
      el("div", { class: "found" }, [
        el("div", { class: "hdr" }, [
          el("span", { class: "nm", text: found.name }),
          el("button", {
            type: "button",
            class: "ghost",
            text: "Edit this spec →",
            onclick: () => {
              loadDraft(clone(found.draft), `Inferred from imported source: ${found.name}`);
              selectTab("editor");
            },
          }),
        ]),
        evidence,
      ]),
    );
  }

  byId("importAll").hidden = state.imported.length === 0;
  byId("importAll").textContent = `Add all ${state.imported.length} to the pack`;
  setStatus(
    "importStatus",
    state.imported.length
      ? `found ${state.imported.length} procedure(s) across ${payload.length} file(s)`
      : "no procedures found",
    state.imported.length ? "ok" : "err",
  );
}

/* The Test tab ----------------------------------------------------------- */

// "My stuff is working" is observed, not asserted. The sample goes through the
// wasm bench, which installs the pack into a real registry and runs the real
// analyser over it — so nothing here decides what a word means; it only paints
// what the merged registry answered.

/** Ask the bench to re-analyse, once the keystrokes have settled. */
function scheduleTest(): void {
  if (testTimer !== undefined) window.clearTimeout(testTimer);
  testTimer = window.setTimeout(runTest, SETTLE_MS);
}

/** Re-analyse the sample and repaint the tab. */
function runTest(): void {
  state.sample = byId<HTMLTextAreaElement>("testText").value;
  try {
    testReport = unwrap<TestReport>(
      wasm.pack_test_analyse(state.pack.source, state.sample, state.dialect),
    );
  } catch (e) {
    testReport = null;
    setStatus("testStatus", `could not analyse the sample: ${message(e)}`, "err");
    return;
  }
  renderTestReport();
  scheduleSave();
}

function renderTestReport(): void {
  const report = testReport;
  const view = byId("testTokens");
  const diags = byId("testDiags");
  clear(view);
  clear(diags);
  if (!report) return;

  const code = el("code", {});
  for (const chunk of report.render) {
    if (chunk.token === null) {
      code.appendChild(document.createTextNode(chunk.text));
      continue;
    }
    const token = report.tokens[chunk.token];
    if (!token) {
      code.appendChild(document.createTextNode(chunk.text));
      continue;
    }
    code.appendChild(tokenButton(token, chunk.text));
  }
  view.appendChild(code);

  for (const d of report.diagnostics) {
    diags.appendChild(
      el("div", { class: `diagrow ${d.severity}` }, [
        el("span", { class: "at", text: `${d.line}:${d.column}` }),
        el("span", { class: "code", text: d.code }),
        el("span", { class: "what", text: d.message }),
      ]),
    );
  }
  if (!report.diagnostics.length) {
    diags.appendChild(el("div", { class: "note info", text: "No diagnostics." }));
  }

  const s = report.summary;
  byId("testSummary").textContent =
    `${s.calls} call(s), ${s.pack_calls} into the pack` +
    (s.unknown_commands ? `, ${s.unknown_commands} unknown` : "") +
    ` · ${s.errors} error(s), ${s.warnings} warning(s)`;
  setStatus(
    "testStatus",
    report.installed
      ? `analysed against the ${dialectLabel(report.dialect)} registry with pack ${report.pack} installed`
      : `analysed against the plain ${dialectLabel(report.dialect)} registry — the pack declares no commands yet`,
    "ok",
  );

  // Keep whatever was being inspected selected across a re-run.
  if (inspected !== null) inspectAt(inspected);
}

/**
 * Append a call to one of the pack's own commands, filled to its declared
 * minimum arity.
 *
 * The point is a first keystroke that already exercises the spec: a call that
 * satisfies the arity proves the command resolves, and deleting a word from it
 * is the fastest way to see the arity diagnostic fire.
 */
function insertSampleCall(): void {
  const name = state.pack.open ?? state.pack.view?.commands[0]?.name;
  if (!name) {
    setStatus("testStatus", "the pack has no commands to call yet", "err");
    return;
  }
  let min = 0;
  try {
    const view = unwrap<PackCommandView>(wasm.pack_command(state.pack.source, name, state.dialect));
    const arity = asRecord(asRecord(view.pack ?? {}).arity);
    min = typeof arity.min === "number" ? arity.min : 0;
  } catch {
    min = 0;
  }
  const args = Array.from({ length: min }, (_, i) => `arg${i + 1}`).join(" ");
  const text = byId<HTMLTextAreaElement>("testText").value.replace(/\s*$/, "");
  writeSample(`${text ? `${text}\n` : ""}${name}${args ? ` ${args}` : ""}\n`);
  runTest();
}

/** One word of the sample, as a button carrying its resolution. */
function tokenButton(token: TestToken, text: string): HTMLButtonElement {
  const classes = ["tok", `k-${token.kind}`, `o-${token.origin}`];
  if (token.severity) classes.push(`s-${token.severity}`);
  const roles = token.roles.map((r) => r.role).join(", ");
  return el("button", {
    type: "button",
    class: classes.join(" "),
    text,
    // A word that holds other words contributes several chunks, so the
    // pressed state is keyed on the word's offset rather than on DOM order.
    "data-tok": String(token.start),
    "aria-pressed": inspected === token.start ? "true" : "false",
    title:
      `${token.command} — ${token.detail}` +
      (roles ? `\nrole: ${roles}` : "") +
      (token.field ? `\nfrom: ${token.field}` : ""),
    onclick: () => inspectAt(token.start),
  });
}

/** Show the deep view of the word at byte `offset`. */
function inspectAt(offset: number): void {
  let view: TestInspection;
  try {
    view = unwrap<TestInspection>(
      wasm.pack_test_inspect(state.pack.source, state.sample, state.dialect, offset),
    );
  } catch {
    inspected = null;
    return;
  }
  inspected = offset;
  renderInspection(view);
  // Mark every chunk of the inspected word, without repainting the view.
  for (const node of byId("testTokens").querySelectorAll<HTMLElement>(".tok")) {
    node.setAttribute("aria-pressed", node.dataset.tok === String(offset) ? "true" : "false");
  }
}

function renderInspection(view: TestInspection): void {
  const panel = byId("testInspect");
  clear(panel);
  panel.appendChild(el("label", { class: "fld", text: "Inspector" }));

  const row = (key: string, kids: Child[]): void => {
    panel.appendChild(
      el("div", { class: "insprow" }, [
        el("span", { class: "k", text: key }),
        el("span", { class: "v" }, kids),
      ]),
    );
  };

  row("word", [
    el("code", { text: view.word.text }),
    document.createTextNode(
      ` · ${view.word.kind} · line ${view.word.line}, column ${view.word.column}` +
        (view.word.index === 0 ? " · command head" : ` · argument ${view.word.index}`),
    ),
  ]);

  if (view.spec) {
    row("resolved spec", [
      el("code", { text: view.spec.name }),
      document.createTextNode(` — the ${view.spec.source} spec`),
    ]);
    if (view.spec.summary) row("summary", [document.createTextNode(view.spec.summary)]);
    if (view.spec.synopsis) row("synopsis", [el("code", { text: view.spec.synopsis })]);
    row("arity", [
      el("code", { text: view.spec.arity }),
      document.createTextNode(
        ` · ${view.spec.subcommands} subcommand(s), ${view.spec.options} option(s)`,
      ),
    ]);
    if (view.spec.required_package) {
      row("package gate", [el("code", { text: view.spec.required_package })]);
    }
    if (view.spec.fields_set.length) {
      row("fields the pack sets", [document.createTextNode(view.spec.fields_set.join(", "))]);
    }
  } else {
    row("resolved spec", [
      el("b", { text: "none" }),
      document.createTextNode(` — ${view.role.detail}`),
    ]);
  }

  if (view.subcommand) {
    row("subcommand", [
      el("code", { text: view.subcommand.name }),
      document.createTextNode(
        ` · arity ${view.subcommand.arity}` +
          (view.subcommand.detail ? ` — ${view.subcommand.detail}` : ""),
      ),
    ]);
  }

  row("argument role", [
    view.role.roles.length
      ? el("code", { text: view.role.roles.map((r) => r.role).join(", ") })
      : el("span", { text: "none" }),
    document.createTextNode(` — ${view.role.detail}`),
  ]);
  for (const role of view.role.roles) {
    if (role.doc) row(role.role, [document.createTextNode(role.doc)]);
  }
  row("produced by", [
    view.role.field
      ? el("code", { text: view.role.field })
      : el("span", { text: "no spec property — this position is undeclared" }),
  ]);

  for (const d of view.diagnostics) {
    row(`${d.severity} ${d.code}`, [document.createTextNode(d.message)]);
  }
  for (const n of view.notices) {
    row(`loader notice (line ${n.line})`, [document.createTextNode(n.reason)]);
  }

  if (view.editable && view.spec) {
    const name = view.spec.name;
    panel.appendChild(
      el("div", { class: "actions" }, [
        el("button", {
          type: "button",
          class: "ghost",
          text: `Edit ${name} →`,
          onclick: () => openPackCommand(name),
        }),
      ]),
    );
  }
}

/* Command search and history --------------------------------------------- */

// `/` anywhere opens search over both models; the arrows walk the commands
// visited. Both are about the same thing: a pack is many commands and one
// deliverable, so moving between them must never cost a page hunt.

let paletteRows: { name: string; where: "pack" | "registry" }[] = [];
let paletteAt = 0;

function openPalette(): void {
  const box = byId("palette");
  box.hidden = false;
  const input = byId<HTMLInputElement>("paletteInput");
  input.value = "";
  renderPalette();
  input.focus();
}

function closePalette(): void {
  byId("palette").hidden = true;
}

function renderPalette(): void {
  const query = byId<HTMLInputElement>("paletteInput").value.trim().toLowerCase();
  const packRows = (state.pack.view?.commands ?? [])
    .filter((c) => !query || c.name.toLowerCase().includes(query))
    .map((c) => ({ name: c.name, where: "pack" as const, summary: c.summary, pack: "" }));
  const registryRows = state.index
    .filter(
      (e) =>
        !query ||
        e.name.toLowerCase().includes(query) ||
        (e.summary ?? "").toLowerCase().includes(query),
    )
    .map((e) => ({ name: e.name, where: "registry" as const, summary: e.summary, pack: e.pack }));
  const rows = [...packRows, ...registryRows].slice(0, MAX_PALETTE);
  paletteRows = rows.map((r) => ({ name: r.name, where: r.where }));
  paletteAt = Math.min(paletteAt, Math.max(0, rows.length - 1));

  const list = byId("paletteList");
  clear(list);
  rows.forEach((row, i) => {
    list.appendChild(
      el("li", { role: "option" }, [
        el(
          "button",
          {
            type: "button",
            "aria-selected": i === paletteAt ? "true" : "false",
            onclick: () => choosePalette(i),
          },
          [
            el("span", { class: "nm", text: row.name }),
            el("span", { class: "sm", text: row.summary || "" }),
            // A palette row is out of the browser's context, so it says where
            // the command is declared as well as which model it came from.
            row.pack ? packChip(row.pack) : null,
            el("span", {
              class: "where",
              text: row.where === "pack" ? "pack" : dialectLabel(state.dialect),
            }),
          ],
        ),
      ]),
    );
  });
  if (!rows.length) {
    list.appendChild(el("li", {}, [el("span", { class: "foot", text: "no match" })]));
  }
}

function movePalette(delta: number): void {
  if (!paletteRows.length) return;
  paletteAt = (paletteAt + delta + paletteRows.length) % paletteRows.length;
  const buttons = byId("paletteList").querySelectorAll<HTMLElement>("button");
  buttons.forEach((node, i) =>
    node.setAttribute("aria-selected", i === paletteAt ? "true" : "false"),
  );
  buttons[paletteAt]?.scrollIntoView({ block: "nearest" });
}

function choosePalette(index: number): void {
  const row = paletteRows[index];
  if (!row) return;
  closePalette();
  if (row.where === "pack") openPackCommand(row.name);
  else openCommand(row.name);
}

/** Record a command as visited, unless history itself is doing the opening. */
function pushHistory(visit: Visit): void {
  if (navigating) return;
  const current = state.history[state.historyAt];
  if (current && current.name === visit.name && current.where === visit.where) return;
  // A new move truncates whatever was ahead — the browser's own rule, and
  // `pushState` applies it to the session history at the same moment.
  state.history = state.history.slice(0, state.historyAt + 1);
  state.history.push(visit);
  state.historyAt = state.history.length - 1;
  visitEntries = visitEntries.slice(0, state.historyAt);
  visitEntries[state.historyAt] = writeRoute(visitRoute(visit), state.historyAt);
  renderHistoryButtons();
}

function renderHistoryButtons(): void {
  byId<HTMLButtonElement>("navBack").disabled = state.historyAt <= 0;
  byId<HTMLButtonElement>("navFwd").disabled = state.historyAt >= state.history.length - 1;
}

/**
 * Step through the visited commands.
 *
 * Unsaved edits are not a reason to block: every keystroke is already in the
 * document and in browser storage, so moving away loses nothing — which is
 * exactly what the design asks for ("carried by the live-save layer rather
 * than blocked by prompts").
 */
function navigate(delta: number): void {
  const at = state.historyAt + delta;
  if (!state.history[at]) return;
  const target = visitEntries[at];
  const here = currentEntry()?.index;
  if (routing && typeof target === "number" && target > 0 && typeof here === "number") {
    // Move the browser instead of opening directly: `popstate` does the
    // opening, so these buttons and the browser's own Back are one act. The
    // delta is over the entries that carry *visits*, so a Reference entry
    // sitting between two commands is stepped over rather than landed on —
    // and a zero delta (two visits the address bar cannot tell apart, such as
    // one name opened from the pack and from the registry) is opened directly,
    // because `go(0)` would reload the page.
    const delta = target - here;
    if (delta !== 0) {
      window.history.go(delta);
      return;
    }
  }
  openVisit(at);
}

/* Reference ------------------------------------------------------------- */

// The Reference tab renders the registry's whole vocabulary — every spec
// field and every catalogue (traits, argument roles, taint colours, …) with
// its long-form help — behind one search box. Sections and rows are built
// once at boot; searching toggles `hidden`, so filtering thousands of rows
// stays instant.

interface RefRow {
  node: HTMLElement;
  hay: string;
  /** The catalogue value or spec key this row is, for `#/ref/…` links. */
  term: string;
}

interface RefSection {
  node: HTMLElement;
  /**
   * Matches against the section's title only — an introduction that merely
   * *mentions* taint must not make every trait a hit for "taint".
   */
  hay: string;
  rows: RefRow[];
  /** The count badge in the section heading, updated as filtering changes. */
  count: HTMLElement;
}

const refSections: RefSection[] = [];

function refRow(
  term: string,
  badges: string[],
  doc: string,
  example: CodeExample,
  help?: string,
): RefRow {
  const head = el("div", { class: "hd" }, [el("code", { class: "term", text: term })]);
  for (const badge of badges) head.appendChild(el("span", { class: "badge", text: badge }));
  const detail = el("div", { class: "refdetail" }, [annotatedExample(example)]);
  if (help && help !== doc) detail.prepend(helpParagraphs(help));
  const button = el("button", {
    type: "button",
    class: "qbtn",
    text: "?",
    title: `Show ${term} inline help`,
    "aria-label": `Show ${term} inline help`,
    "aria-expanded": "false",
  });
  head.appendChild(button);
  const node = el("details", { class: "refrow" }, [
    el("summary", {}, [head, el("div", { class: "doc", text: doc })]),
    detail,
  ]);
  const syncButton = (): void => {
    button.setAttribute("aria-expanded", node.open ? "true" : "false");
    button.setAttribute("aria-label", `${node.open ? "Hide" : "Show"} ${term} inline help`);
  };
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    node.open = !node.open;
    syncButton();
  });
  node.addEventListener("toggle", syncButton);
  return { node, term, hay: `${term} ${badges.join(" ")} ${doc} ${help ?? ""}`.toLowerCase() };
}

function refSection(
  title: string,
  intro: string,
  example: CodeExample,
  rows: RefRow[],
): RefSection {
  const body = el("div", { class: "body" }, [
    el("div", { class: "refintro" }, [
      el("p", { class: "intro", text: intro }),
      annotatedExample(example),
    ]),
    ...rows.map((row) => row.node),
  ]);
  const count = el("span", { class: "n", text: `${rows.length}` });
  const node = el("details", { class: "group ref", open: true }, [
    el("summary", {}, [document.createTextNode(title), count]),
    body,
  ]);
  byId("refOut").appendChild(node);
  const section = { node, hay: title.toLowerCase(), rows, count };
  refSections.push(section);
  return section;
}

/**
 * The catalogue sections by id, so `#/ref/<catalogue>/<value>` can find the
 * row it names rather than only putting words in the search box.
 */
const refByCatalogue = new Map<string, RefSection>();

function buildReference(): void {
  const schema = state.schema;

  // Spec fields, both tables merged: most keys exist on the command and its
  // subcommands with the same meaning, so one row carries where it applies.
  const onCommand = new Set(schema.command.map((f) => f.key));
  const onSubcommand = new Set(schema.subcommand.map((f) => f.key));
  const seen = new Set<string>();
  const fieldRows: RefRow[] = [];
  for (const field of [...schema.command, ...schema.subcommand]) {
    if (seen.has(field.key)) continue;
    seen.add(field.key);
    const badges = [field.group];
    if (!onSubcommand.has(field.key)) badges.push("command only");
    else if (!onCommand.has(field.key)) badges.push("subcommand only");
    fieldRows.push(
      refRow(field.key, badges, `${field.label} — ${field.doc}`, field.example, field.help),
    );
  }
  for (const field of schema.nestedFields) {
    fieldRows.push(
      refRow(
        field.key,
        [field.group, field.owner],
        `${field.label} — ${field.doc}`,
        field.example,
        field.help,
      ),
    );
  }
  refSection(
    "Spec fields",
    "Every field a command specification can set, with what it drives. The same keys appear in the editor form, grouped the same way.",
    schema.groupExamples.Identity,
    fieldRows,
  );

  // One section per catalogue, ordered by title.
  const ids = Object.keys(schema.catalogues).sort((a, b) => {
    const ta = schema.catalogueHelp[a]?.title ?? a;
    const tb = schema.catalogueHelp[b]?.title ?? b;
    return ta.localeCompare(tb);
  });
  for (const id of ids) {
    const help = schema.catalogueHelp[id];
    const variants = schema.catalogues[id] ?? [];
    refByCatalogue.set(
      id,
      refSection(
        help?.title ?? id,
        help?.intro ?? "",
        help.example,
        variants.map((variant) =>
          refRow(variant.key, variant.group ? [variant.group] : [], variant.doc, variant.example),
        ),
      ),
    );
  }

  byId("refSearch").addEventListener("input", filterReference);
  filterReference();
}

function filterReference(): void {
  const query = byId<HTMLInputElement>("refSearch").value.trim().toLowerCase();
  const tokens = query.split(/\s+/).filter(Boolean);
  let shown = 0;
  let total = 0;
  for (const section of refSections) {
    // A token can match at either level: a section whose title matches keeps
    // every row ("taint" keeps all taint colours), otherwise rows filter one
    // by one.
    const sectionWide = tokens.every((t) => section.hay.includes(t));
    let visible = 0;
    for (const row of section.rows) {
      total += 1;
      const on = sectionWide || tokens.every((t) => row.hay.includes(t));
      row.node.hidden = !on;
      if (on) {
        visible += 1;
        shown += 1;
      }
    }
    section.node.hidden = visible === 0;
    section.count.textContent =
      tokens.length && visible < section.rows.length
        ? `${visible} of ${section.rows.length}`
        : `${section.rows.length}`;
    if (tokens.length && visible > 0) section.node.setAttribute("open", "");
  }
  byId("refCount").textContent = tokens.length
    ? `${shown} of ${total} entries match`
    : `${total} entries`;
}

/**
 * Show one Reference entry: a whole catalogue, or one value out of it.
 *
 * This is what `#/ref/<catalogue>[/<value>]` restores, and what the dock and
 * the form's "all of these on the Reference tab" link both go through, so
 * there is one way to arrive at a Reference entry however it was asked for.
 */
function openReferenceEntry(catalogueId: string, variantKey: string | null): void {
  const section = refByCatalogue.get(catalogueId);
  const title = state.schema.catalogueHelp[catalogueId]?.title ?? catalogueId;
  byId<HTMLInputElement>("refSearch").value = variantKey ?? title;
  filterReference();
  selectTab("reference");
  const row = variantKey ? section?.rows.find((entry) => entry.term === variantKey) : undefined;
  const target = row?.node ?? section?.node;
  if (target instanceof HTMLDetailsElement) target.open = true;
  target?.scrollIntoView({ block: "center", behavior: motionOk() ? "smooth" : "auto" });
  routeTo({ view: "reference", catalogue: catalogueId, variant: variantKey });
  setDockSubject(
    variantKey
      ? { kind: "value", catalogue: catalogueId, key: variantKey }
      : { kind: "catalogue", id: catalogueId },
  );
}

/* The documentation dock ------------------------------------------------- */

// A persistent region that documents whatever the author is currently
// touching. The inline `?` panels stay exactly as they were — on a narrow
// viewport they remain the primary surface, and they are what the contract
// tests exercise — but they push the form around, and the field you are
// editing must not move when you ask what it means. So this is a second
// *surface* over the same schema help, never a second copy of the text: both
// render `helpParagraphs` and `annotatedExample` from the very same entries.
//
// It also answers the question one field at a time cannot: which settings are
// read together. `field.related` names those clusters, and the dock turns each
// key into a link that navigates to the setting and flashes it.

/** Whether the viewer has not asked for less animation. */
function motionOk(): boolean {
  return !window.matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/** Whether the dock is its own column rather than an overlay at the bottom. */
function dockIsSidebar(): boolean {
  return window.matchMedia("(min-width: 75rem)").matches;
}

function dockSources(): { schema: Schema; packs: ReadonlyMap<string, PackRow> } {
  return { schema: state.schema, packs: state.packById };
}

/**
 * Point the dock at `subject`.
 *
 * Returns whether it took: a subject the schema knows nothing about leaves the
 * dock showing what it had, which is also what "nothing focused" does. The
 * dock never blanks once it has said something.
 */
function setDockSubject(subject: DockSubject): boolean {
  const content = describeSubject(dockSources(), subject);
  if (!content) return false;
  dockContent = content;
  renderDock();
  // A focused setting is the linkable thing in the address bar; a group or a
  // catalogue is context around it, and does not rewrite the URL.
  if (subject.kind === "field") noteFieldInRoute(subject.key);
  return true;
}

function renderDock(): void {
  const body = byId("dockBody");
  clear(body);
  const content = dockContent;
  byId("dockSubject").textContent = content ? content.title : "Focus a setting";
  if (!content) {
    body.appendChild(
      el("p", {
        class: "dockempty",
        text: "Focus a setting, a group heading, or a picker and its documentation appears here.",
      }),
    );
    return;
  }
  body.appendChild(el("div", { class: "dockkind", text: content.kindLabel }));
  body.appendChild(el("h3", { class: "docktitle", text: content.title }));
  if (content.code) {
    body.appendChild(el("div", {}, [el("code", { class: "dockcode", text: content.code })]));
  }
  if (content.doc) body.appendChild(el("p", { class: "dockdoc", text: content.doc }));
  if (content.help) body.appendChild(helpParagraphs(content.help));
  if (content.example) body.appendChild(annotatedExample(content.example));
  // No `related` at all is the ordinary case for a studio wasm built before
  // the key existed: the dock simply ends after the example.
  for (const cluster of content.related) body.appendChild(relatedNode(cluster));
}

/** One named cluster: why its members constrain each other, and links to them. */
function relatedNode(cluster: RelatedGroup): HTMLElement {
  const list = el("ul", {});
  for (const link of cluster.links) {
    if (link.self || !link.known) {
      list.appendChild(
        el("li", {}, [
          el("span", {
            class: link.self ? "self" : "unknown",
            text: link.key,
            title: link.self ? "the setting shown above" : "not in this registry's schema",
          }),
        ]),
      );
      continue;
    }
    const anchor = el("a", {
      href: relatedHref(link.key),
      text: link.key,
      title: link.label,
      onclick: (event: Event) => {
        event.preventDefault();
        revealField(link.key);
      },
    });
    list.appendChild(el("li", {}, [anchor]));
  }
  return el("section", { class: "dockrel" }, [
    el("h4", { text: cluster.name }),
    el("p", { class: "why", text: cluster.why }),
    list,
  ]);
}

/** The address a related link points at: the deep link when there is one. */
function relatedHref(key: string): string {
  const command = openCommandName();
  return command
    ? formatHash({ view: "command", dialect: state.dialect, command, field: key })
    : `#${fieldAnchorId(key)}`;
}

/**
 * Navigate to the setting `key`: the editor tab, its group open, scrolled to,
 * outlined, focused — and then documented, so following a chain of links
 * works link after link.
 */
function revealField(key: string): void {
  if (currentTab !== "editor") selectTab("editor");
  const node = document.getElementById(fieldAnchorId(key));
  if (!node) {
    setStatus("status", `${key} is not a setting this command has`, "err");
    return;
  }
  let group = node.parentElement?.closest("details");
  while (group) {
    group.open = true;
    group = group.parentElement?.closest("details") ?? null;
  }
  node.scrollIntoView({ block: "center", behavior: motionOk() ? "smooth" : "auto" });
  flashField(node);
  // `preventScroll` because the scroll above already put it where it belongs —
  // the browser's own focus scroll would fight the bottom bar for the room.
  node
    .querySelector<HTMLElement>(".ctl input, .ctl select, .ctl textarea, .ctl button")
    ?.focus({ preventScroll: true });
  setDockSubject({ kind: "field", key });
}

/** Outline the field a link landed on, briefly and without blocking anything. */
function flashField(node: HTMLElement): void {
  node.classList.remove("dock-target");
  // Reading the layout is what restarts the animation on a repeat visit; with
  // reduced motion the same class holds a static outline for the same time.
  void node.offsetWidth;
  node.classList.add("dock-target");
  window.setTimeout(() => node.classList.remove("dock-target"), FLASH_MS);
}

/** Expand or collapse the dock, remembering which in the session. */
function setDockOpen(open: boolean, opts: { save?: boolean } = {}): void {
  dockOpen = open;
  byId("docsDock").classList.toggle("collapsed", !open);
  byId("dockToggle").setAttribute("aria-expanded", open ? "true" : "false");
  // The bottom-bar layouts reserve room for the dock at the end of the page;
  // how much depends on this, so CSS has to be able to see it.
  document.documentElement.dataset.dock = open ? "open" : "collapsed";
  if (opts.save !== false) scheduleSave();
}

/**
 * Scroll `node` out from under the dock.
 *
 * Only the overlay shapes can cover anything, and on a phone an expanded dock
 * takes half the viewport — so a control focused underneath it has to come up.
 */
function keepClearOfDock(node: Element): void {
  if (dockIsSidebar() || !dockOpen) return;
  const bar = byId("docsDock").getBoundingClientRect();
  const box = node.getBoundingClientRect();
  if (box.bottom <= bar.top) return;
  node.scrollIntoView({ block: "center", behavior: motionOk() ? "smooth" : "auto" });
}

/**
 * Re-target the dock from something in the form.
 *
 * `committed` marks a `change` — the moment a picker's value was actually
 * chosen, as against merely opened. Nothing here listens for the pointer
 * passing over: a cursor crossing 137 settings must not churn the panel.
 */
function retargetFromForm(target: EventTarget | null, committed = false): void {
  if (!(target instanceof Element)) return;
  // A group heading is asked about first: it is never a control, and the
  // innermost one wins — a subcommand's own groups sit *inside* the field that
  // holds the subcommand, so testing the field first would never reach them.
  const heading = target.closest("summary")?.parentElement;
  if (
    heading instanceof HTMLElement &&
    heading.dataset.group &&
    setDockSubject({ kind: "group", name: heading.dataset.group })
  ) {
    return;
  }
  const picker = target.closest<HTMLElement>("[data-catalogue]");
  if (picker?.dataset.catalogue) {
    const chosen = chosenVariant(target, picker, committed);
    const subject: DockSubject = chosen
      ? { kind: "value", catalogue: picker.dataset.catalogue, key: chosen }
      : { kind: "catalogue", id: picker.dataset.catalogue };
    if (setDockSubject(subject)) {
      keepClearOfDock(target);
      return;
    }
  }
  const field = target.closest<HTMLElement>("[data-field-key]");
  if (field?.dataset.fieldKey && setDockSubject({ kind: "field", key: field.dataset.fieldKey })) {
    keepClearOfDock(target);
  }
}

/** Which catalogue value the author is on, when they are on one at all. */
function chosenVariant(target: Element, picker: HTMLElement, committed: boolean): string | null {
  const toggle = target.closest<HTMLElement>("[data-variant]");
  if (toggle?.dataset.variant) return toggle.dataset.variant;
  // A `<select>` only names a value once it has been chosen; merely opening
  // one asks about the vocabulary, not about whatever was already in it.
  if (committed && picker instanceof HTMLSelectElement && picker.value) return picker.value;
  return null;
}

/** Re-target the dock from the registry browser: a pack section's heading. */
function retargetFromBrowser(target: EventTarget | null): void {
  if (!(target instanceof Element)) return;
  const section = target.closest("summary")?.parentElement;
  if (section instanceof HTMLElement && section.dataset.pack) {
    setDockSubject({ kind: "pack", id: section.dataset.pack });
  }
}

function bindDock(): void {
  byId("dockToggle").addEventListener("click", () => setDockOpen(!dockOpen));

  const form = byId("form");
  form.addEventListener("focusin", (event) => retargetFromForm(event.target));
  form.addEventListener("click", (event) => retargetFromForm(event.target));
  form.addEventListener("change", (event) => retargetFromForm(event.target, true));

  const browser = byId("browser");
  browser.addEventListener("focusin", (event) => retargetFromBrowser(event.target));
  browser.addEventListener("click", (event) => retargetFromBrowser(event.target));

  // A phone starts on the summary line: the dock is one tap away and covers
  // nothing until it is asked for.
  setDockOpen(!window.matchMedia("(max-width: 34rem)").matches, { save: false });
  // Seed with the first group so the dock explains itself before anything has
  // been focused; `renderDock` paints the empty state when there is no group
  // to seed from.
  renderDock();
  setDockSubject({ kind: "group", name: state.schema.groups[0] ?? "" });
}

/* URL routing ------------------------------------------------------------ */

// One history, not two. The visit stack (`state.history`) stays the record of
// which commands were opened and in what order; every visit is *mirrored* as
// one session-history entry tagged with its index. So the in-page ◀ ▶ buttons
// no longer open anything themselves — they move the browser's history to the
// entry carrying the visit they want, and `popstate` is the single path that
// opens it. Alt+← , the buttons, and the browser's own Back are then the same
// act rather than two stacks racing.
//
// A focus move inside a command *replaces* the entry it is on, so Back and
// Forward move between commands and not between every setting a pointer
// touched — which is what `historyMode` decides.

/** What this page wrote into a session-history entry. */
interface StudioEntry {
  /**
   * Where the entry sits in this page's own run of session history, so the
   * delta between two of them can be computed. A *position* rather than a
   * serial number, because a push from a back position throws away what was
   * ahead and the new entry takes the place that was vacated — a counter
   * would go on climbing and `go()` would overshoot.
   */
  index: number;
  /** The visit the entry opens, when it opens one. */
  visit: number | null;
}

/**
 * Whether the address bar can be written at all.
 *
 * `pushState` throws on a `file://` page — an opaque origin cannot own a URL —
 * and this page is built to be saved and opened from disk. Losing the deep
 * links there is fine; losing the Back button is not, so routing switches off
 * and the visit stack carries navigation on its own.
 */
let routing = true;
/** The last route written, which is what `historyMode` compares against. */
let lastRoute: Route | null = null;
/** Where each visit's session-history entry sits, parallel to `state.history`. */
let visitEntries: number[] = [];

function currentEntry(): StudioEntry | null {
  const value = window.history.state as Partial<StudioEntry> | null;
  return value && typeof value.index === "number"
    ? { index: value.index, visit: typeof value.visit === "number" ? value.visit : null }
    : null;
}

/** The name of the command the form is a projection of, if any. */
function openCommandName(): string | null {
  const name = state.draft?.name;
  return typeof name === "string" && name ? name : state.pack.open;
}

/** The route for a visited command. */
function visitRoute(visit: Visit): Route {
  return { view: "command", dialect: state.dialect, command: visit.name, field: null };
}

/**
 * Write `route` into the address bar, pushing or replacing as `historyMode`
 * decides. Returns the entry id now showing it, or -1 when routing is off.
 */
function writeRoute(route: Route, visit: number | null): number {
  if (!routing) return -1;
  const here = currentEntry();
  // The entry the page was loaded on is adopted rather than pushed over: the
  // first command opened in a session is where the session starts, not a
  // second place it went.
  const mode = here === null ? "replace" : historyMode(lastRoute, route);
  const index = mode === "replace" ? (here?.index ?? 1) : (here?.index ?? 0) + 1;
  try {
    const entry: StudioEntry = { index, visit };
    if (mode === "replace") window.history.replaceState(entry, "", formatHash(route));
    else window.history.pushState(entry, "", formatHash(route));
  } catch {
    routing = false;
    return -1;
  }
  lastRoute = route;
  return index;
}

/** Write a route that opens no new visit — a tab move, a Reference entry. */
function routeTo(route: Route): void {
  // History is doing the opening: it must not record its own moves.
  if (navigating) return;
  writeRoute(route, currentEntry()?.visit ?? null);
}

/** Put the focused setting in the address bar, without a new history entry. */
function noteFieldInRoute(key: string): void {
  if (navigating) return;
  const command = openCommandName();
  if (!command) return;
  writeRoute(
    { view: "command", dialect: state.dialect, command, field: key },
    currentEntry()?.visit ?? null,
  );
}

/** Open the visit at `at` without recording it as a new one. */
function openVisit(at: number): void {
  const visit = state.history[at];
  if (!visit) return;
  state.historyAt = at;
  navigating = true;
  try {
    if (visit.where === "pack") openPackCommand(visit.name);
    else openCommand(visit.name);
  } finally {
    navigating = false;
  }
  renderHistoryButtons();
}

/** Restore whatever a fragment names — on load, and on an untagged popstate. */
function applyRoute(route: Route): void {
  if (route.view === "reference") {
    openReferenceEntry(route.catalogue, route.variant);
    return;
  }
  if (route.dialect !== state.dialect && dialectLabels.has(route.dialect)) {
    // Take the same path a person does, so the language server, the pack's
    // collisions and the browser list all follow as they always do.
    byId<HTMLSelectElement>("dialect").value = route.dialect;
    byId("dialect").dispatchEvent(new Event("change"));
  }
  if (openCommandName() !== route.command) openCommand(route.command);
  else selectTab("editor");
  if (route.field) revealField(route.field);
  else {
    const subject = routeSubject(route);
    if (subject) setDockSubject(subject);
  }
}

function bindRouting(): void {
  // The fragment the page was opened on is where the first write starts from,
  // so restoring a link replaces that entry rather than pushing a copy of it.
  lastRoute = parseHash(window.location.hash);
  window.addEventListener("popstate", (event) => {
    const entry = event.state as Partial<StudioEntry> | null;
    const route = parseHash(window.location.hash);
    lastRoute = route;
    if (typeof entry?.visit === "number" && state.history[entry.visit]) {
      openVisit(entry.visit);
      if (route?.view === "command" && route.field) revealField(route.field);
      return;
    }
    if (route) applyRoute(route);
  });
}

/* The editor surface ---------------------------------------------------- */

// Standalone Studio uses Monaco as its only editor. An IDE embedding delegates
// these surfaces to ordinary native file tabs beside the Studio panel. Neither
// mode exposes the hidden state textarea as an alternate editor.

/**
 * Load the editor chunk and mount both surfaces, once.
 *
 * The specifier is built at run time rather than written as a literal, which is
 * what keeps esbuild from pulling the chunk back into the main bundle: a literal
 * `import("./monacoHost.js")` in an IIFE build is inlined, and the whole point
 * of the split is that it is not.
 */
async function mountEditorHost(): Promise<void> {
  if (editorHost || editorMounting) {
    await editorMounting;
    return;
  }
  // `#lspStatus` sits under the Pack DSL editor, which is not on screen when
  // the Test tab triggered the mount — so a *degradation* is repeated into the
  // Test tab's own status line. Announcing a lost language server only where
  // the reader cannot see it is the same as not announcing it.
  const report = (message: string, kind?: "ok" | "err"): void => {
    setStatus("lspStatus", message, kind);
    if (kind === "err") setStatus("testStatus", message, kind);
  };
  editorMounting = (async () => {
    report("loading the editor…");
    // Standalone and Pages use Monaco exclusively. IDE integrations inject a
    // bridge which materialises these surfaces as ordinary native file tabs,
    // beside the Studio panel, so an editor never embeds another editor.
    const native = window.__tclSpecStudioHost !== undefined;
    const specifier = native
      ? (window.__tclSpecStudioNativeModuleUrl ??
        assetUrl(activeBuildInfo, "native-editor-controller", NATIVE_EDITOR_CHUNK))
      : assetUrl(activeBuildInfo, "editor-controller", EDITOR_CHUNK);
    const chunk = (await import(specifier)) as MonacoHostModule;
    if (!verifyAssetVersion(activeBuildInfo, "editor controller", chunk.buildVersion)) return;
    editorHost = await chunk.mountEditors({
      workerUrl: assetUrl(activeBuildInfo, "lsp-worker", `${LSP_WORKER_DIR}/worker.js`),
      stylesheetUrl: assetUrl(activeBuildInfo, "editor-style", "assets/monaco-host.css"),
      grammarUrl: assetUrl(activeBuildInfo, "tcl-grammar", "assets/tcl.tmLanguage.json"),
      onigurumaUrl: assetUrl(activeBuildInfo, "oniguruma", "assets/onig.wasm"),
      dialect: state.dialect,
      report,
      dsl: {
        container: byId("dslEditor"),
        textarea: byId<HTMLTextAreaElement>("dslText"),
        onChange: (text) => setPackSource(text, { refreshForm: true }),
      },
      sample: {
        container: byId("testEditor"),
        textarea: byId<HTMLTextAreaElement>("testText"),
        onChange: () => runTest(),
      },
      rust: {
        container: byId("rsEditor"),
        source: byId("rsOut"),
      },
      stub: {
        container: byId("stubEditor"),
        source: byId("stubOut"),
      },
    });
    // Generated-code state remains in hidden elements which seed Monaco or
    // the IDE's native file tabs.
    byId("rsOut").hidden = true;
    byId("stubOut").hidden = true;
  })();
  try {
    await editorMounting;
  } catch (e) {
    editorHost = null;
    byId<HTMLTextAreaElement>("testText").hidden = true;
    const editorName = window.__tclSpecStudioHost ? "Native editor integration" : "Monaco";
    byId("dslEditor").replaceChildren(
      el("div", { class: "editor-unavailable", text: `${editorName} could not be loaded.` }),
    );
    byId("testEditor").replaceChildren(
      el("div", { class: "editor-unavailable", text: `${editorName} could not be loaded.` }),
    );
    report(
      `the ${editorName.toLowerCase()} could not load (${message(e)}); reload after restoring the editor assets`,
      "err",
    );
  } finally {
    editorMounting = null;
  }
}

/** Push text into state storage and the active editor host, without echo. */
function writeDsl(source: string): void {
  const textarea = byId<HTMLTextAreaElement>("dslText");
  textarea.value = source;
  editorHost?.setDslText(source);
}

/** The same, for the Test tab's sample. */
function writeSample(sample: string): void {
  byId<HTMLTextAreaElement>("testText").value = sample;
  editorHost?.setSampleText(sample);
}

/* Tabs ------------------------------------------------------------------ */

function selectTab(name: Tab): void {
  currentTab = name;
  // Only two tabs are *views* the route vocabulary can name; the rest are
  // panes over the one command already in the address bar, and pushing an
  // entry for each of them would fill Back with places nothing came from.
  if (name === "editor") {
    const command = openCommandName();
    const field = dockContent?.subject.kind === "field" ? dockContent.subject.key : null;
    if (command) routeTo({ view: "command", dialect: state.dialect, command, field });
  }
  for (const tab of TABS) {
    const on = tab === name;
    byId(`tab-${tab}`).setAttribute("aria-selected", on ? "true" : "false");
    byId(`pane-${tab}`).hidden = !on;
  }
  if (name === "dsl" || name === "test" || name === "rs" || name === "stub") {
    void mountEditorHost().then(() => {
      // Monaco measures the container it was created in; a container inside a
      // `hidden` pane measures zero, so every reveal needs a re-layout.
      editorHost?.layout();
    });
  }
  // The bench is the most expensive surface in the studio — it installs a
  // registry and runs the analyser — so it only works while it is on screen.
  if (name === "test") runTest();
}

/* Boot ------------------------------------------------------------------ */

function newCommandDraft(): Draft {
  const fresh = JSON.parse(wasm.new_command()) as Draft;
  fresh.name = "mycommand";
  return fresh;
}

function bindUi(): void {
  for (const tab of TABS) {
    byId(`tab-${tab}`).addEventListener("click", () => selectTab(tab));
  }

  byId("filter").addEventListener("input", renderList);
  byId("loadCmd").addEventListener("click", loadTypedCommand);
  // Enter (and the on-screen keyboard's "go") loads without reaching for the
  // button — the whole point on a phone.
  byId("filter").addEventListener("keydown", (event) => {
    if ((event as KeyboardEvent).key === "Enter") {
      event.preventDefault();
      loadTypedCommand();
    }
  });
  // Picking a datalist suggestion commits it straight away on desktop; iOS
  // fires `change` when the field loses focus, so this stays a deliberate act.
  byId("filter").addEventListener("change", () => {
    const typed = byId<HTMLInputElement>("filter").value.trim();
    if (typed && state.index.some((e) => e.name === typed)) {
      openCommand(typed);
    }
  });

  byId<HTMLSelectElement>("dialect").addEventListener("change", () => {
    state.dialect = byId<HTMLSelectElement>("dialect").value;
    loadDialect();
    // The dialect decides what a pack collides with, so the merged world has
    // to be recomputed too.
    setPackSource(state.pack.source);
    // The language server takes the dialect as the sample document's language
    // id, so the sample is closed and re-opened under the new one — the same
    // route an editor takes when a file's language changes, and the only place
    // the studio's dialect and the server's dialect can be made to agree.
    editorHost?.setDialect(state.dialect);
    onDraftChanged();
  });

  byId("newCmd").addEventListener("click", () => {
    loadDraft(newCommandDraft(), "A new command — every field at its CommandSpec::DEFAULT.");
    selectTab("editor");
  });

  /* The pack panel and the DSL pane. */

  byId("packAdd").addEventListener("click", addDraftToPack);
  byId("packNew").addEventListener("click", () => {
    state.pack.open = null;
    setPackSource(unwrap<PackWrite>(wasm.pack_new("mylib")).source);
    selectTab("dsl");
  });

  byId("dslCopy").addEventListener("click", () => copyText(state.pack.source, "dslStatus"));
  byId("dslDownload").addEventListener("click", () => download(packPath(), state.pack.source));
  byId("dslAdd").addEventListener("click", () => addFile(packPath(), state.pack.source));
  byId("dslCanonical").addEventListener("click", () => {
    try {
      const out = unwrap<PackWrite>(wasm.pack_render(state.pack.source));
      setPackSource(out.source, { refreshForm: true });
      setStatus("dslStatus", "re-rendered from the pack's commands", "ok");
    } catch (e) {
      setStatus("dslStatus", `could not re-render: ${message(e)}`, "err");
    }
  });
  const packPicker = byId<HTMLInputElement>("dslPicker");
  byId("dslOpen").addEventListener("click", () => packPicker.click());
  packPicker.addEventListener("change", () => readPackFile(packPicker.files));

  /* The Test tab. */

  byId("testRun").addEventListener("click", runTest);
  byId("testSample").addEventListener("click", insertSampleCall);

  /* `/` search and history. */

  byId("navBack").addEventListener("click", () => navigate(-1));
  byId("navFwd").addEventListener("click", () => navigate(1));
  byId("paletteInput").addEventListener("input", renderPalette);
  byId("paletteInput").addEventListener("keydown", (event) => {
    const key = (event as KeyboardEvent).key;
    if (key === "ArrowDown") {
      event.preventDefault();
      movePalette(1);
    } else if (key === "ArrowUp") {
      event.preventDefault();
      movePalette(-1);
    } else if (key === "Enter") {
      event.preventDefault();
      choosePalette(paletteAt);
    } else if (key === "Escape") {
      event.preventDefault();
      closePalette();
    }
  });
  byId("palette").addEventListener("click", (event) => {
    // A click on the backdrop dismisses; one inside the box does not.
    if (event.target === byId("palette")) closePalette();
  });
  document.addEventListener("keydown", (event) => {
    const key = (event as KeyboardEvent).key;
    const target = event.target as HTMLElement | null;
    const typing =
      target instanceof HTMLInputElement ||
      target instanceof HTMLTextAreaElement ||
      target instanceof HTMLSelectElement;
    if (key === "/" && !typing) {
      event.preventDefault();
      openPalette();
      return;
    }
    if ((event as KeyboardEvent).altKey && key === "ArrowLeft") {
      event.preventDefault();
      navigate(-1);
    } else if ((event as KeyboardEvent).altKey && key === "ArrowRight") {
      event.preventDefault();
      navigate(1);
    }
  });

  byId("rsPack").addEventListener("input", onDraftChanged);
  byId("stubMode").addEventListener("change", onDraftChanged);

  byId("rsCopy").addEventListener("click", () => {
    if (rendered.rs) copyText(rendered.rs.source, "status");
  });
  byId("rsDownload").addEventListener("click", () => {
    if (rendered.rs) download(rendered.rs.path, rendered.rs.source);
  });
  byId("rsAdd").addEventListener("click", () => {
    if (rendered.rs) addFile(rendered.rs.path, rendered.rs.source);
  });

  byId("stubCopy").addEventListener("click", () => {
    if (rendered.stub) copyText(rendered.stub.source, "status");
  });
  byId("stubDownload").addEventListener("click", () => {
    if (rendered.stub) download(rendered.stub.path, rendered.stub.source);
  });
  byId("stubAdd").addEventListener("click", () => {
    if (rendered.stub) addFile(rendered.stub.path, rendered.stub.source);
  });

  byId("filesDownload").addEventListener("click", () => {
    for (const file of state.files) download(file.path, file.source);
  });
  byId("filesClear").addEventListener("click", () => {
    state.files = [];
    renderFiles();
  });

  byId("issueOpen").addEventListener("click", openIssue);
  byId("issueCopy").addEventListener("click", () => copyText(issueBody(), "issueStatus"));
  byId("issueTitle").addEventListener("input", updateIssueSize);
  byId("issueNotes").addEventListener("input", updateIssueSize);

  const drop = byId("drop");
  const picker = byId<HTMLInputElement>("picker");
  drop.addEventListener("click", () => picker.click());
  drop.addEventListener("keydown", (event) => {
    const key = (event as KeyboardEvent).key;
    if (key === "Enter" || key === " ") {
      event.preventDefault();
      picker.click();
    }
  });
  picker.addEventListener("change", () => readFiles(picker.files));
  const dirPicker = byId<HTMLInputElement>("dirPicker");
  byId("importDir").addEventListener("click", () => dirPicker.click());
  dirPicker.addEventListener("change", () => readFiles(dirPicker.files, { directory: true }));
  byId("importAll").addEventListener("click", addImportedToPack);
  for (const name of ["dragenter", "dragover"]) {
    drop.addEventListener(name, (event) => {
      event.preventDefault();
      drop.classList.add("drag");
    });
  }
  for (const name of ["dragleave", "drop"]) {
    drop.addEventListener(name, (event) => {
      event.preventDefault();
      drop.classList.remove("drag");
    });
  }
  drop.addEventListener("drop", (event) => {
    readFiles((event as DragEvent).dataTransfer?.files ?? null);
  });

  // The multi-release importer and its opt-in GitHub panel. Everything it
  // needs from here is passed in — the dialect is read at the moment of use so
  // a change to the selector applies to the next derivation, not the last.
  initReleasesPanel({
    wasm,
    dialect: () => state.dialect,
    addToPack: writeDraftsToPack,
    openDraft: (draft, origin) => {
      loadDraft(clone(draft), origin);
      selectTab("editor");
    },
  });
}

/**
 * Read the dialect's command index and its pack catalogue together.
 *
 * The two are one fact seen twice — which commands there are, and which packs
 * declare them — so nothing may hold one without the other.
 */
function loadDialect(): void {
  const index = JSON.parse(wasm.command_index(state.dialect)) as CommandIndex;
  state.index = index.commands ?? [];
  const catalogue = JSON.parse(wasm.pack_catalogue(state.dialect)) as PackCatalogue;
  state.packs = catalogue.packs ?? [];
  state.packById = packIndex(state.packs);
  state.byPack = groupByPack(state.index);
  renderList();
  // Which pack wins a name is the dialect's choice, so the chip beside the
  // open command is stale the moment the picker moves.
  renderEditorSource();
}

/**
 * Pick up exactly where the last visit left off.
 *
 * The persisted record is the pack document plus which command was open, which
 * is all that is needed because everything else in the studio is derived from
 * those two. Storage being unavailable is not an error — the page reports that
 * it will not remember and carries on.
 */
async function restoreSession(): Promise<void> {
  if (!(await idb.available())) {
    byId("liveSave").textContent =
      "this browser will not persist your work (IndexedDB unavailable) — download the pack before you leave";
    byId("liveSave").className = "note warn";
    return;
  }
  const session = await idb.load();
  if (!session) {
    byId("liveSave").textContent = "Live save is on: every edit is kept in this browser.";
    return;
  }
  if (session.expanded) {
    state.expandedPacks = new Set(session.expanded);
    renderList();
  }
  if (typeof session.dockOpen === "boolean") setDockOpen(session.dockOpen, { save: false });
  // A session saved before a dialect left the catalogue keeps the default
  // rather than restoring a picker value that no longer exists.
  if (session.dialect && session.dialect !== state.dialect && dialectLabels.has(session.dialect)) {
    state.dialect = session.dialect;
    byId<HTMLSelectElement>("dialect").value = session.dialect;
    loadDialect();
  }
  setPackSource(session.source);
  if (typeof session.sample === "string") {
    state.sample = session.sample;
    writeSample(session.sample);
  }
  byId("liveSave").textContent =
    `Restored from this browser: pack ${state.pack.view?.pack ?? ""}, ` +
    `${state.pack.view?.summary.commands ?? 0} command(s).`;
  if (session.open) openPackCommand(session.open);
}

function boot(): void {
  const buildInfo = verifyBuildInfo();
  if (!buildInfo) return;
  activeBuildInfo = buildInfo;
  window.TclLspSiteUpdate?.start({ currentVersion: buildInfo.version });
  const payload = byId("studio-wasm").textContent?.trim() ?? "";
  const binary = Uint8Array.from(atob(payload), (c) => c.charCodeAt(0));

  // The glue declares `wasm_bindgen` with a top-level `let`, which lands in the
  // global *lexical* environment — shared across classic scripts, but never a
  // property of `window`. It has to be reached as a bare identifier.
  wasm_bindgen(binary).then(
    () => {
      wasm = wasm_bindgen;

      const schema = JSON.parse(wasm.schema()) as Schema;
      state = {
        schema,
        defaultCommand: JSON.parse(wasm.new_command()) as Draft,
        defaultSubcommand: JSON.parse(wasm.new_subcommand()) as Draft,
        dialect: "spectcl",
        index: [],
        packs: [],
        packById: new Map(),
        byPack: new Map(),
        expandedPacks: new Set(),
        draft: null,
        editorOrigin: "Pick a command on the left, or start a new one.",
        files: [],
        imported: [],
        pack: { source: "", view: null, open: null },
        sample: SAMPLE_HINT,
        history: [],
        historyAt: -1,
      };

      editors = makeEditors({
        catalogues: schema.catalogues,
        variantHelp: (variant: Variant) => {
          const panel = helpWithExample(variant.doc, variant.example);
          return { button: helpButton(panel, variant.key), panel };
        },
        fieldHelp: (key: string) => {
          const field = schema.nestedFields.find((candidate) => candidate.key === key);
          if (!field) throw new Error(`No nested-field help for ${key}`);
          const panel = helpWithExample(field.help, field.example);
          return { button: helpButton(panel, field.label), panel };
        },
        newSubcommand: () => JSON.parse(wasm.new_subcommand()) as Draft,
        buildSubcommandForm: (container, draft, onChange) => {
          buildForm(container, schema.subcommand, draft, "subcommand", () => {
            onChange();
            onFormEdit();
          });
        },
      });

      const picker = byId<HTMLSelectElement>("dialect");
      for (const dialect of JSON.parse(wasm.dialects()) as DialectEntry[]) {
        dialectLabels.set(dialect.name, dialect.label);
        picker.appendChild(el("option", { value: dialect.name, text: dialect.label }));
      }
      picker.value = state.dialect;

      bindUi();
      bindDock();
      bindRouting();
      loadDialect();
      renderFiles();
      buildReference();
      byId<HTMLTextAreaElement>("testText").value = state.sample;
      renderHistoryButtons();
      setPackSource(unwrap<PackWrite>(wasm.pack_new("mylib")).source);
      loadDraft(
        newCommandDraft(),
        "A new command — every field at its CommandSpec::DEFAULT. Pick one on the left to load a real spec.",
      );

      byId("ver").textContent =
        `${buildInfo.version} · ${schema.command.length} spec fields · ${state.index.length} commands`;
      setStatus("status", "");
      // The address bar wins over the resumed session: a link someone was sent
      // names the view they were sent to, and it is applied last.
      void restoreSession().then(() => {
        const route = parseHash(window.location.hash);
        if (route) applyRoute(route);
      });
    },
    (e: unknown) => setStatus("status", `could not start the engine: ${String(e)}`, "err"),
  );
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", boot);
} else {
  boot();
}
