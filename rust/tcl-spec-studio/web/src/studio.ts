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
import type {
  CommandIndex,
  DialectEntry,
  Draft,
  FieldSchema,
  ImportResult,
  IndexEntry,
  InferredCommand,
  Json,
  Rendered,
  Schema,
  StagedFile,
  StudioWasm,
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

const TABS = ["editor", "rs", "stub", "import", "reference", "share"] as const;
type Tab = (typeof TABS)[number];

interface State {
  schema: Schema;
  defaultCommand: Draft;
  defaultSubcommand: Draft;
  dialect: string;
  index: IndexEntry[];
  draft: Draft | null;
  files: StagedFile[];
  imported: InferredCommand[];
}

let wasm: StudioWasm;
let state: State;
let editors: Record<string, Editor>;
const rendered: { rs: Rendered | null; stub: Rendered | null } = { rs: null, stub: null };
let renderTimer: number | undefined;

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

/* Editing --------------------------------------------------------------- */

function loadDraft(draft: Draft, origin: string): void {
  state.draft = draft;
  byId("editorSource").textContent = origin;

  renderUnrenderableWarning(draft);

  const form = byId("form");
  clear(form);
  buildForm(form, state.schema.command, draft, "command", onDraftChanged);
  renderList();
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
    loadDraft(loaded, `Loaded ${name} from the ${state.dialect} registry.`);
    setStatus("status", "");
  } catch (e) {
    setStatus("status", `could not load ${name}: ${String(e)}`, "err");
  }
}

function onDraftChanged(): void {
  if (renderTimer !== undefined) window.clearTimeout(renderTimer);
  renderTimer = window.setTimeout(renderOutputs, 80);
}

function renderOutputs(): void {
  if (!state.draft) return;
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
  for (const tab of TABS) {
    const on = tab === name;
    byId(`tab-${tab}`).setAttribute("aria-selected", on ? "true" : "false");
    byId(`pane-${tab}`).hidden = !on;
  }
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
    onDraftChanged();
  });

  byId("newCmd").addEventListener("click", () => {
    loadDraft(newCommandDraft(), "A new command — every field at its CommandSpec::DEFAULT.");
    selectTab("editor");
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
      };

      editors = makeEditors({
        catalogues: schema.catalogues,
        newSubcommand: () => JSON.parse(wasm.new_subcommand()) as Draft,
        buildSubcommandForm: (container, draft, onChange) => {
          buildForm(container, schema.subcommand, draft, "subcommand", () => {
            onChange();
            onDraftChanged();
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
      loadDraft(
        newCommandDraft(),
        "A new command — every field at its CommandSpec::DEFAULT. Pick one on the left to load a real spec.",
      );

      byId("ver").textContent =
        `${schema.command.length} spec fields · ${state.index.length} commands`;
      setStatus("status", "");
    },
    (e: unknown) => setStatus("status", `could not start the engine: ${String(e)}`, "err"),
  );
}

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", boot);
} else {
  boot();
}
