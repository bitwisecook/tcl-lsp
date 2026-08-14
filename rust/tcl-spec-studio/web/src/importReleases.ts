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

// The Import tab's Releases mode: several archives in, version ranges out.
//
// The panel is a staging area, not a pipeline: archives arrive (dropped,
// picked, or fetched from GitHub), each becomes a row with an editable version
// label, and nothing is derived until the author presses the button. That order
// matters because the labels are the entire evidential basis of every range —
// a label the author never saw is a range they never agreed to.
//
// The result view leads with the *notes*. A derived `introduced_version` is a
// claim about a package's history; the sentence explaining which two releases
// disagreed is what makes it checkable, so it is not tucked behind a disclosure
// triangle.

import { byId, clear, el, setStatus } from "./dom.js";
import {
  browserHttp,
  fetchZipball,
  listTags,
  parseRepoRef,
  type Http,
  type RepoRef,
  type TagRef,
} from "./github.js";
import {
  lifecycleOf,
  releaseId,
  snapshotsJson,
  validateReleases,
  versionFromFileName,
  type Release,
  type UnzipResult,
  type VersionedCommand,
  type VersionedImportResult,
} from "./releases.js";
import type { Draft, StudioWasm } from "./types.js";

/** How many tags the panel offers before the "show more" control appears. */
const TAG_PAGE = 20;

/** The ceiling on tags fetched at all, however many times "more" is pressed. */
const TAG_CEILING = 200;

/** What the panel needs from the studio around it. */
export interface ReleasesDeps {
  wasm: StudioWasm;
  /** The dialect every snapshot is analysed as, read at the moment of use. */
  dialect(): string;
  /** Write drafts into the pack document; returns what landed and what did not. */
  addToPack(commands: { name: string; draft: Draft }[]): { written: number; failed: string[] };
  /** Open one draft in the spec editor form. */
  openDraft(draft: Draft, origin: string): void;
  /** The network, injected so the GitHub half can be driven by a stub. */
  http?: Http;
}

/** Everything the panel is holding, between presses. */
interface PanelState {
  releases: Release[];
  seq: number;
  result: VersionedImportResult | null;
  repo: RepoRef | null;
  tags: TagRef[];
  shown: number;
  selected: Set<string>;
}

/**
 * Stand the Releases panel up and wire every control on it.
 *
 * Called once at boot; everything after that is driven by the DOM.
 */
export function initReleasesPanel(deps: ReleasesDeps): void {
  const http = deps.http ?? browserHttp;
  const panel: PanelState = {
    releases: [],
    seq: 0,
    result: null,
    repo: null,
    tags: [],
    shown: TAG_PAGE,
    selected: new Set(),
  };

  /* Staging archives ---------------------------------------------------- */

  /** Unpack one archive's bytes and stage it as a release. */
  function stage(origin: string, bytes: Uint8Array, suggested: string): void {
    let reply: UnzipResult;
    try {
      reply = JSON.parse(deps.wasm.unzip_entries(bytes)) as UnzipResult;
    } catch (e) {
      setStatus("releasesStatus", `could not read ${origin}: ${String(e)}`, "err");
      return;
    }
    if (reply.error) {
      setStatus("releasesStatus", `${origin}: ${reply.error}`, "err");
      return;
    }
    panel.seq += 1;
    panel.releases.push({
      id: releaseId(origin, panel.seq),
      origin,
      version: suggested,
      entries: reply.entries ?? [],
      skipped: reply.skipped ?? [],
    });
    renderReleases();
  }

  /** Read chosen `.zip` files and stage each as a release. */
  function stageFiles(files: FileList | null): void {
    const chosen = Array.from(files ?? []).filter((file) => /\.zip$/i.test(file.name));
    if (!chosen.length) {
      setStatus("releasesStatus", "no .zip archives in that selection", "err");
      return;
    }
    setStatus("releasesStatus", `reading ${chosen.length} archive(s)…`);
    Promise.all(
      chosen.map(async (file) => ({
        name: file.name,
        bytes: new Uint8Array(await file.arrayBuffer()),
      })),
    ).then(
      (loaded) => {
        for (const { name, bytes } of loaded) {
          stage(name, bytes, versionFromFileName(name));
        }
        setStatus("releasesStatus", `${panel.releases.length} release(s) staged`, "ok");
      },
      (e: unknown) =>
        setStatus("releasesStatus", `could not read the archives: ${String(e)}`, "err"),
    );
  }

  /** Repaint the staged-release rows. */
  function renderReleases(): void {
    const list = byId("releaseList");
    clear(list);
    for (const release of panel.releases) {
      const label = el("input", {
        type: "text",
        class: "ver",
        value: release.version,
        placeholder: "version",
        "aria-label": `Version label for ${release.origin}`,
        oninput: (event: Event) => {
          release.version = (event.target as HTMLInputElement).value;
          (event.target as HTMLInputElement)
            .closest("li")
            ?.classList.toggle("nolabel", !release.version.trim());
        },
      });
      const files = `${release.entries.length} Tcl file(s)`;
      const skipped = release.skipped.length ? ` · ${release.skipped.length} skipped` : "";
      list.appendChild(
        el("li", { class: release.version.trim() ? "" : "nolabel" }, [
          el("span", { class: "origin", text: release.origin }),
          label,
          el("span", {
            class: "files",
            text: files + skipped,
            title: release.skipped.join("\n") || undefined,
          }),
          el("button", {
            type: "button",
            class: "ghost",
            text: "Remove",
            onclick: () => {
              panel.releases = panel.releases.filter((other) => other.id !== release.id);
              renderReleases();
            },
          }),
        ]),
      );
    }
  }

  /* Deriving the ranges -------------------------------------------------- */

  function run(): void {
    const problems = validateReleases(panel.releases);
    // A single release is a warning, not a refusal — deriving from one is still
    // the ordinary importer's answer, and refusing would hide that.
    const fatal = problems.filter((p) => !p.includes("cannot witness a range"));
    if (fatal.length) {
      setStatus("releasesStatus", fatal.join(" "), "err");
      return;
    }
    const complete = byId<HTMLInputElement>("completeHistory").checked;
    let result: VersionedImportResult;
    try {
      result = JSON.parse(
        deps.wasm.import_package_versions(snapshotsJson(panel.releases), deps.dialect(), complete),
      ) as VersionedImportResult;
    } catch (e) {
      setStatus("releasesStatus", `the import failed: ${String(e)}`, "err");
      return;
    }
    if (result.error) {
      setStatus("releasesStatus", result.error, "err");
      return;
    }
    panel.result = result;
    renderResult(result, problems);
  }

  /** Paint the derived commands, their bounds, and the evidence for both. */
  function renderResult(result: VersionedImportResult, advisories: string[]): void {
    const out = byId("releasesOut");
    clear(out);

    out.appendChild(
      el("div", { class: "note info" }, [
        el("b", {
          text: result.package
            ? `Package ${result.package}`
            : "No package name declared by any release",
        }),
        document.createTextNode(
          ` — ${result.versions.length} release(s) compared, oldest first: ${result.versions.join(", ")}.`,
        ),
      ]),
    );
    for (const advisory of advisories) {
      out.appendChild(el("div", { class: "note warn", text: advisory }));
    }
    for (const warning of result.warnings ?? []) {
      out.appendChild(el("div", { class: "note warn", text: warning }));
    }

    for (const found of result.commands ?? []) {
      out.appendChild(commandCard(found));
    }

    const all = byId("releasesAll");
    all.hidden = (result.commands ?? []).length === 0;
    all.textContent = `Add all ${(result.commands ?? []).length} to the pack`;
    setStatus(
      "releasesStatus",
      `${(result.commands ?? []).length} command(s) across ${result.versions.length} release(s)`,
      "ok",
    );
  }

  /** One command's card: its derived bounds, then every note behind them. */
  function commandCard(found: VersionedCommand): HTMLElement {
    const life = lifecycleOf(found.draft);
    const ranges = el("div", { class: "ranges" });
    const bound = (label: string, value: string | null): void => {
      ranges.appendChild(
        el("span", {
          class: value ? "range set" : "range",
          text: value ? `${label} ${value}` : `${label} —`,
          title: value
            ? `${label} is recorded on the draft as ${value}`
            : `no ${label} was derived — the releases do not witness one`,
        }),
      );
    };
    bound("introduced", life.introduced);
    bound("retired", life.retired);
    bound("deprecated", life.deprecated);

    const evidence = el("ul", { class: "evidence" });
    for (const note of found.notes ?? []) evidence.appendChild(el("li", { text: note }));

    return el("div", { class: "found" }, [
      el("div", { class: "hdr" }, [
        el("span", { class: "nm", text: found.name }),
        el("button", {
          type: "button",
          class: "ghost",
          text: "Edit this spec →",
          onclick: () => deps.openDraft(found.draft, `Merged from ${found.name}'s releases`),
        }),
      ]),
      ranges,
      evidence,
    ]);
  }

  function addAll(): void {
    const commands = panel.result?.commands ?? [];
    if (!commands.length) return;
    const { written, failed } = deps.addToPack(
      commands.map((command) => ({ name: command.name, draft: command.draft })),
    );
    setStatus(
      "releasesStatus",
      `${written} command(s) written into the pack` +
        (failed.length ? ` — ${failed.length} could not be written: ${failed.join(", ")}` : ""),
      failed.length ? "err" : "ok",
    );
  }

  /* The GitHub panel ----------------------------------------------------- */

  function renderTags(): void {
    const list = byId("ghTags");
    clear(list);
    for (const tag of panel.tags.slice(0, panel.shown)) {
      const box = el("input", {
        type: "checkbox",
        id: `ghtag-${tag.tag}`,
        checked: panel.selected.has(tag.tag),
        onchange: (event: Event) => {
          if ((event.target as HTMLInputElement).checked) panel.selected.add(tag.tag);
          else panel.selected.delete(tag.tag);
        },
      });
      list.appendChild(
        el("li", {}, [
          box,
          el("label", { for: `ghtag-${tag.tag}`, class: "tag", text: tag.tag }),
          el("span", { class: "as", text: `→ version ${tag.version}` }),
        ]),
      );
    }
    byId("ghActions").hidden = panel.tags.length === 0;
    byId("ghMore").hidden = panel.tags.length <= panel.shown;
  }

  async function loadTags(limit: number): Promise<void> {
    const ref = parseRepoRef(byId<HTMLInputElement>("ghRepo").value);
    if (!ref) {
      setStatus("ghStatus", "give a repository as owner/repo, or paste its GitHub URL", "err");
      return;
    }
    panel.repo = ref;
    setStatus("ghStatus", `asking api.github.com for ${ref.owner}/${ref.repo}'s tags…`);
    try {
      panel.tags = await listTags(http, ref, limit);
    } catch (e) {
      setStatus(
        "ghStatus",
        `${e instanceof Error ? e.message : String(e)} You can still do this offline: download the release archives from GitHub yourself and drop them on the box above.`,
        "err",
      );
      return;
    }
    if (!panel.tags.length) {
      setStatus(
        "ghStatus",
        `${ref.owner}/${ref.repo} has no tags — there are no releases to compare. Drop archives on the box above instead.`,
        "err",
      );
      renderTags();
      return;
    }
    // Newest-first, capped: comparing every release of a long-lived package is
    // both slow and rarely what anyone wants, and the cap is liftable.
    panel.shown = Math.min(TAG_PAGE, panel.tags.length);
    panel.selected = new Set(panel.tags.slice(0, panel.shown).map((tag) => tag.tag));
    renderTags();
    setStatus(
      "ghStatus",
      `${panel.tags.length} tag(s); the newest ${panel.shown} are selected`,
      "ok",
    );
  }

  async function fetchSelected(): Promise<void> {
    const ref = panel.repo;
    if (!ref) return;
    const chosen = panel.tags.filter((tag) => panel.selected.has(tag.tag));
    if (!chosen.length) {
      setStatus("ghStatus", "select at least one tag", "err");
      return;
    }
    let done = 0;
    for (const tag of chosen) {
      setStatus("ghStatus", `downloading ${tag.tag} (${done + 1} of ${chosen.length})…`);
      try {
        const bytes = await fetchZipball(http, ref, tag.tag);
        stage(`${ref.owner}/${ref.repo}@${tag.tag}`, bytes, tag.version);
        done += 1;
      } catch (e) {
        setStatus(
          "ghStatus",
          `${e instanceof Error ? e.message : String(e)} ${done} of ${chosen.length} downloaded. Download the rest from GitHub yourself and drop them on the box above.`,
          "err",
        );
        return;
      }
    }
    setStatus(
      "ghStatus",
      `${done} release(s) staged — check the version labels, then derive the ranges`,
      "ok",
    );
  }

  /* Wiring --------------------------------------------------------------- */

  const zipPicker = byId<HTMLInputElement>("zipPicker");
  const zipDrop = byId("zipDrop");
  zipDrop.addEventListener("click", () => zipPicker.click());
  zipDrop.addEventListener("keydown", (event) => {
    const key = (event as KeyboardEvent).key;
    if (key === "Enter" || key === " ") {
      event.preventDefault();
      zipPicker.click();
    }
  });
  zipPicker.addEventListener("change", () => stageFiles(zipPicker.files));
  for (const name of ["dragenter", "dragover"]) {
    zipDrop.addEventListener(name, (event) => {
      event.preventDefault();
      zipDrop.classList.add("drag");
    });
  }
  for (const name of ["dragleave", "drop"]) {
    zipDrop.addEventListener(name, (event) => {
      event.preventDefault();
      zipDrop.classList.remove("drag");
    });
  }
  zipDrop.addEventListener("drop", (event) =>
    stageFiles((event as DragEvent).dataTransfer?.files ?? null),
  );

  byId("releasesRun").addEventListener("click", run);
  byId("releasesAll").addEventListener("click", addAll);
  byId("releasesClear").addEventListener("click", () => {
    panel.releases = [];
    panel.result = null;
    renderReleases();
    clear(byId("releasesOut"));
    byId("releasesAll").hidden = true;
    setStatus("releasesStatus", "");
  });

  byId("ghList").addEventListener("click", () => {
    void loadTags(TAG_PAGE);
  });
  byId<HTMLInputElement>("ghRepo").addEventListener("keydown", (event) => {
    if ((event as KeyboardEvent).key === "Enter") {
      event.preventDefault();
      void loadTags(TAG_PAGE);
    }
  });
  byId("ghMore").addEventListener("click", () => {
    if (panel.shown < panel.tags.length) {
      panel.shown = Math.min(panel.shown + TAG_PAGE, panel.tags.length);
      renderTags();
      return;
    }
    void loadTags(Math.min(panel.tags.length + TAG_PAGE, TAG_CEILING));
  });
  byId("ghNone").addEventListener("click", () => {
    panel.selected.clear();
    renderTags();
  });
  byId("ghFetch").addEventListener("click", () => {
    void fetchSelected();
  });

  // The mode switch. Both bodies stay in the DOM so the staged releases survive
  // a trip back to the single-snapshot importer.
  const modes: [string, string][] = [
    ["imode-files", "imode-files-body"],
    ["imode-releases", "imode-releases-body"],
  ];
  for (const [tab] of modes) {
    byId(tab).addEventListener("click", () => {
      for (const [other, body] of modes) {
        const on = other === tab;
        byId(other).setAttribute("aria-selected", on ? "true" : "false");
        byId(body).hidden = !on;
      }
    });
  }
}
