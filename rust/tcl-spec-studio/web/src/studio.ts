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
import { asRecord, asString, makeEditors, STRUCTURAL_KINDS, type Editor } from "./editors.js";
import * as idb from "./idb.js";
import type {
  CommandIndex,
  DialectEntry,
  Draft,
  FieldSchema,
  ImportResult,
  IndexEntry,
  InferredCommand,
  Json,
  PackCommandView,
  PackStoreView,
  PackWrite,
  Rendered,
  Schema,
  StagedFile,
  StudioWasm,
  TestInspection,
  TestReport,
  TestToken,
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
  draft: Draft | null;
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
      const panel = helpParagraphs(groupHelp);
      summary.appendChild(helpButton(panel, `the ${group} group`));
      body.appendChild(panel);
    }
    const details = el("details", { class: "group" }, [summary, body]);
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
  const help = helpParagraphs(field.help);
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
          onclick: () => openReference(catalogue.title),
        }),
      ]),
    );
  }
  const lbl = el("div", { class: "lbl" }, [
    el("span", { class: "name", text: field.label }),
    el("code", { class: "key", text: field.key }),
    helpButton(help, field.label),
  ]);
  const node = el("div", { class: "field" }, [
    lbl,
    el("div", { class: "doc", text: field.doc }),
    help,
    ctl,
  ]);

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

function renderList(): void {
  const query = byId<HTMLInputElement>("filter").value.trim().toLowerCase();
  const list = byId("cmdlist");
  clear(list);

  const matches = state.index.filter(
    (entry) =>
      !query ||
      entry.name.toLowerCase().includes(query) ||
      (entry.summary ?? "").toLowerCase().includes(query),
  );

  byId("count").textContent =
    matches.length === state.index.length
      ? `${state.index.length} commands`
      : `${matches.length} of ${state.index.length} commands`;

  const current = typeof state.draft?.name === "string" ? state.draft.name : null;
  for (const entry of matches.slice(0, MAX_LISTED)) {
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
    list.appendChild(el("li", {}, [button]));
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

  if (matches.length > MAX_LISTED) {
    list.appendChild(
      el("li", {}, [
        el("span", {
          class: "sm",
          style: "display:block;padding:.5rem .3rem",
          text: `…and ${matches.length - MAX_LISTED} more — narrow the filter`,
        }),
      ]),
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
 * `fromDsl` says the edit came from the DSL textarea, which is the one case
 * where the textarea must *not* be written back to — doing so would move the
 * author's caret to the end of the file on every keystroke.
 */
function setPackSource(
  source: string,
  opts: { fromDsl?: boolean; refreshForm?: boolean } = {},
): void {
  state.pack.source = source;
  try {
    state.pack.view = unwrap<PackStoreView>(wasm.pack_load(source, state.dialect));
    setStatus("dslStatus", "");
  } catch (e) {
    state.pack.view = null;
    setStatus("dslStatus", `could not read the pack: ${message(e)}`, "err");
  }
  if (!opts.fromDsl) byId<HTMLTextAreaElement>("dslText").value = source;

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
    return `${where} ⚠ ${view.name} is also a shipped ${view.dialect} command, and this declaration does not say -override — so an editor would use the shipped spec, not this one.`;
  }
  if (view.origin === "override") {
    return `${where} This declaration replaces the shipped ${view.dialect} command of the same name (-override).`;
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
    const written = unwrap<PackWrite>(
      wasm.pack_set_command(state.pack.source, name, JSON.stringify(state.draft), false),
    );
    state.pack.open = name;
    setPackSource(written.source);
    openPackCommand(name);
    setStatus("status", `${name} added to pack ${state.pack.view?.pack ?? ""}`, "ok");
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

/** The always-visible list of the pack's own commands. */
function renderPackList(): void {
  const view = state.pack.view;
  const list = byId("packlist");
  clear(list);
  byId("packName").textContent = view?.pack || "—";

  const rows = view?.commands ?? [];
  byId("packCount").textContent = rows.length
    ? `${rows.length} command${rows.length === 1 ? "" : "s"}` +
      (view && view.summary.notices ? ` · ${view.summary.notices} notice(s)` : "")
    : "empty — add a command, or open a .tclspec on the Pack DSL tab";

  if (!rows.length) return;

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
          document.createTextNode(`Collisions with the shipped ${view.dialect} registry`),
          el("span", { class: "n", text: `${view.collisions.length}` }),
        ]),
        body,
      ]),
    );
  }

  if (!view.notices.length && !view.collisions.length && view.commands.length) {
    setStatus(
      "dslStatus",
      `${view.summary.commands} command(s), nothing dropped, no collision with the shipped ${view.dialect} registry`,
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
    });
  }, SAVE_MS);
}

/* Editing --------------------------------------------------------------- */

function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/**
 * Load `draft` into the form.
 *
 * `packCommand` names the pack declaration this draft came from, or is `null`
 * when the draft is a registry command or a fresh one — in which case edits go
 * nowhere near the pack document until the author says "Add to pack".
 */
function loadDraft(draft: Draft, origin: string, packCommand: string | null = null): void {
  state.draft = draft;
  state.pack.open = packCommand;
  formDirty = false;
  byId("editorSource").textContent = origin;

  renderUnrenderableWarning(draft);

  const form = byId("form");
  clear(form);
  buildForm(form, state.schema.command, draft, "command", onFormEdit);
  renderList();
  renderPackList();
  onDraftChanged();
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
    setStatus("status", `no command matches “${typed}” in ${state.dialect}`, "err");
    return;
  }
  setStatus(
    "status",
    `${partial.length} commands match “${typed}” — pick one from the list`,
    "err",
  );
}

function openCommand(name: string): void {
  try {
    const loaded = JSON.parse(wasm.load_command(name, state.dialect)) as Draft & { error?: string };
    if (loaded.error) {
      setStatus("status", loaded.error, "err");
      return;
    }
    loadDraft(
      loaded,
      `Loaded ${name} from the ${state.dialect} registry — reference material. ` +
        `Use “+ Add to pack” to start editing it as one of your own.`,
    );
    setStatus("status", "");
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
    byId("rsPath").textContent = rs.path;

    const mode = byId<HTMLSelectElement>("stubMode").value;
    const stub = JSON.parse(
      wasm.render_stub(JSON.stringify([state.draft]), mode, state.dialect),
    ) as Rendered;
    if (stub.error) throw new Error(stub.error);
    rendered.stub = stub;
    byId("stubOut").firstElementChild!.textContent = stub.source;
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

function readFiles(fileList: FileList | null): void {
  const files = Array.from(fileList ?? []);
  if (!files.length) return;
  setStatus("importStatus", `reading ${files.length} file(s)…`);
  Promise.all(files.map(async (file) => ({ name: file.name, text: await file.text() }))).then(
    (payload) => {
      setStatus("importStatus", "analysing…");
      // Let the status paint before the analyser blocks the main thread.
      window.setTimeout(() => runImport(payload), 0);
    },
    (e: unknown) => setStatus("importStatus", `could not read the files: ${String(e)}`, "err"),
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

  setStatus(
    "importStatus",
    state.imported.length ? `found ${state.imported.length} procedure(s)` : "no procedures found",
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
      ? `analysed against the ${report.dialect} registry with pack ${report.pack} installed`
      : `analysed against the plain ${report.dialect} registry — the pack declares no commands yet`,
    s.errors ? "err" : "ok",
  );

  // Keep whatever was being inspected selected across a re-run.
  if (inspected !== null) inspectAt(inspected, { keepScroll: true });
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
  const area = byId<HTMLTextAreaElement>("testText");
  const text = area.value.replace(/\s*$/, "");
  area.value = `${text ? `${text}\n` : ""}${name}${args ? ` ${args}` : ""}\n`;
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
    "aria-pressed": inspected === token.start ? "true" : "false",
    title:
      `${token.command} — ${token.detail}` +
      (roles ? `\nrole: ${roles}` : "") +
      (token.field ? `\nfrom: ${token.field}` : ""),
    onclick: () => inspectAt(token.start),
  });
}

/** Show the deep view of the word at byte `offset`. */
function inspectAt(offset: number, opts: { keepScroll?: boolean } = {}): void {
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
  if (!opts.keepScroll) {
    for (const node of byId("testTokens").querySelectorAll<HTMLElement>(".tok")) {
      node.setAttribute("aria-pressed", "false");
    }
  }
  renderInspection(view);
  // Re-mark the pressed token without a full repaint.
  const report = testReport;
  if (report) {
    const index = report.tokens.findIndex((t) => t.start === offset);
    const buttons = byId("testTokens").querySelectorAll<HTMLElement>(".tok");
    buttons.forEach((node, i) => {
      node.setAttribute("aria-pressed", i === index ? "true" : "false");
    });
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
    .map((c) => ({ name: c.name, where: "pack" as const, summary: c.summary }));
  const registryRows = state.index
    .filter(
      (e) =>
        !query ||
        e.name.toLowerCase().includes(query) ||
        (e.summary ?? "").toLowerCase().includes(query),
    )
    .map((e) => ({ name: e.name, where: "registry" as const, summary: e.summary }));
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
            el("span", { class: "where", text: row.where === "pack" ? "pack" : state.dialect }),
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
  // A new move truncates whatever was ahead — the browser's own rule.
  state.history = state.history.slice(0, state.historyAt + 1);
  state.history.push(visit);
  state.historyAt = state.history.length - 1;
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

/* Reference ------------------------------------------------------------- */

// The Reference tab renders the registry's whole vocabulary — every spec
// field and every catalogue (traits, argument roles, taint colours, …) with
// its long-form help — behind one search box. Sections and rows are built
// once at boot; searching toggles `hidden`, so filtering thousands of rows
// stays instant.

interface RefRow {
  node: HTMLElement;
  hay: string;
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

function refRow(term: string, badges: string[], doc: string, help?: string): RefRow {
  const head = el("div", { class: "hd" }, [el("code", { class: "term", text: term })]);
  for (const badge of badges) head.appendChild(el("span", { class: "badge", text: badge }));
  const kids: Child[] = [head, el("div", { class: "doc", text: doc })];
  if (help && help !== doc) {
    kids.push(el("details", {}, [el("summary", { text: "More" }), helpParagraphs(help)]));
  }
  const node = el("div", { class: "refrow" }, kids);
  return { node, hay: `${term} ${badges.join(" ")} ${doc} ${help ?? ""}`.toLowerCase() };
}

function refSection(title: string, intro: string, rows: RefRow[]): void {
  const body = el("div", { class: "body" }, [
    el("p", { class: "intro", text: intro }),
    ...rows.map((row) => row.node),
  ]);
  const count = el("span", { class: "n", text: `${rows.length}` });
  const node = el("details", { class: "group ref", open: true }, [
    el("summary", {}, [document.createTextNode(title), count]),
    body,
  ]);
  byId("refOut").appendChild(node);
  refSections.push({ node, hay: title.toLowerCase(), rows, count });
}

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
    fieldRows.push(refRow(field.key, badges, `${field.label} — ${field.doc}`, field.help));
  }
  refSection(
    "Spec fields",
    "Every field a command specification can set, with what it drives. The same keys appear in the editor form, grouped the same way.",
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
    refSection(
      help?.title ?? id,
      help?.intro ?? "",
      variants.map((variant) => refRow(variant.key, [], variant.doc)),
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

/** Switch to the Reference tab with `query` in the search box. */
function openReference(query: string): void {
  byId<HTMLInputElement>("refSearch").value = query;
  filterReference();
  selectTab("reference");
}

/* Tabs ------------------------------------------------------------------ */

function selectTab(name: Tab): void {
  currentTab = name;
  for (const tab of TABS) {
    const on = tab === name;
    byId(`tab-${tab}`).setAttribute("aria-selected", on ? "true" : "false");
    byId(`pane-${tab}`).hidden = !on;
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
    loadIndex();
    // The dialect decides what a pack collides with, so the merged world has
    // to be recomputed too.
    setPackSource(state.pack.source);
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

  // The one place a keystroke edits the model directly. Everything else — the
  // form, the pack list, the collision report — re-renders from what this
  // produces.
  byId("dslText").addEventListener("input", () => {
    const text = byId<HTMLTextAreaElement>("dslText").value;
    if (renderTimer !== undefined) window.clearTimeout(renderTimer);
    renderTimer = window.setTimeout(() => {
      setPackSource(text, { fromDsl: true, refreshForm: true });
    }, SETTLE_MS);
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

  byId("testText").addEventListener("input", scheduleTest);
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
}

function loadIndex(): void {
  const index = JSON.parse(wasm.command_index(state.dialect)) as CommandIndex;
  state.index = index.commands ?? [];
  renderList();
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
  if (session.dialect && session.dialect !== state.dialect) {
    state.dialect = session.dialect;
    byId<HTMLSelectElement>("dialect").value = session.dialect;
    loadIndex();
  }
  setPackSource(session.source);
  if (typeof session.sample === "string") {
    state.sample = session.sample;
    byId<HTMLTextAreaElement>("testText").value = session.sample;
  }
  byId("liveSave").textContent =
    `Restored from this browser: pack ${state.pack.view?.pack ?? ""}, ` +
    `${state.pack.view?.summary.commands ?? 0} command(s).`;
  if (session.open) openPackCommand(session.open);
}

function boot(): void {
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
        dialect: "tcl9.0",
        index: [],
        draft: null,
        files: [],
        imported: [],
        pack: { source: "", view: null, open: null },
        sample: SAMPLE_HINT,
        history: [],
        historyAt: -1,
      };

      editors = makeEditors({
        catalogues: schema.catalogues,
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
        picker.appendChild(el("option", { value: dialect.name, text: dialect.label }));
      }
      picker.value = state.dialect;

      bindUi();
      loadIndex();
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
        `${schema.command.length} spec fields · ${state.index.length} commands`;
      setStatus("status", "");
      void restoreSession();
    },
    (e: unknown) => setStatus("status", `could not start the engine: ${String(e)}`, "err"),
  );
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", boot);
} else {
  boot();
}
