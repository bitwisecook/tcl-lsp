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

import { byId, clear, clone, copyText, deepEqual, download, el, setStatus } from "./dom.js";
import { makeEditors, STRUCTURAL_KINDS, type Editor } from "./editors.js";
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

const TABS = ["editor", "rs", "stub", "import", "share"] as const;
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
  const lbl = el("div", { class: "lbl" }, [
    el("span", { class: "name", text: field.label }),
    el("code", { class: "key", text: field.key }),
  ]);
  const node = el("div", { class: "field" }, [
    lbl,
    el("div", { class: "doc", text: field.doc }),
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
      editor(field.kind, draft[field.key] ?? null, (next: Json) => {
        draft[field.key] = next;
        markSet();
        if (STRUCTURAL_KINDS.has(field.kind.tag)) rebuild();
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

  const lost = Array.isArray(draft.__unrenderable) ? draft.__unrenderable : [];
  const warn = byId("unrenderable");
  if (lost.length) {
    warn.hidden = false;
    clear(warn);
    warn.appendChild(
      el("b", {
        text: `This command sets ${lost.length} field${lost.length === 1 ? "" : "s"} the studio cannot read back.`,
      }),
    );
    warn.appendChild(
      document.createTextNode(
        " Rust can tell the field is set but not recover the expression that set it — a function pointer or a reference to a static descriptor. Each is listed below and in the rendered file as a TODO; fill it in under Advanced to emit it.",
      ),
    );
    const ul = el("ul", {});
    for (const key of lost) ul.appendChild(el("li", {}, [el("code", { text: String(key) })]));
    warn.appendChild(ul);
  } else {
    warn.hidden = true;
  }

  const form = byId("form");
  clear(form);
  buildForm(form, state.schema.command, draft, "command", onDraftChanged);
  renderList();
  onDraftChanged();
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
