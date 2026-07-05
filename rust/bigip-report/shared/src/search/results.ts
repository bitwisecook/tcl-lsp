// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// The global "search everything" box. Unlike the old per-device row filter,
// this searches every device's objects at once and renders a single ranked,
// cross-device results list (name · type · device · tier). Picking a result
// jumps to that object in its device. Scope prefixes (`t<n>:`, `<devname>:`)
// and fuzzy/phonetic matching come from ./query and ./matcher.

import { MatchKind, scoreText, KIND_SCORE } from "./matcher";
import { ParsedQuery, deviceInScope, parseQuery } from "./query";

export interface DeviceMeta {
  name: string;
  tier: number | null;
}

export interface GlobalSearchConfig {
  input: HTMLInputElement;
  deviceMeta: DeviceMeta[];
  /** Given the raw query, return a predicate testing a row's indexed text for
   *  IP/CIDR overlap, or null when the query is not an address. Supplied by
   *  topology.ts, which owns the BigInt IP helpers. */
  ipMatcher?: (raw: string) => ((haystack: string) => boolean) | null;
}

interface Candidate {
  row: HTMLElement;
  deviceIndex: number;
  deviceName: string;
  tier: number | null;
  type: string;
  typeLabel: string;
  name: string;
  haystack: string;
}

interface Scored {
  cand: Candidate;
  score: number;
  kind: MatchKind | "ip";
}

const IP_SCORE = 65;
const MAX_RESULTS = 400;

const TYPE_LABELS: Record<string, string> = {
  vs: "Virtual Server",
  pool: "Pool",
  node: "Node",
  mon: "Monitor",
  rule: "iRule",
  prof: "Profile",
  persist: "Persistence",
  policy: "Policy",
  snat: "SNAT Pool",
  dg: "Data Group",
  cert: "Certificate",
  wideip: "GTM WideIP",
  gtmpool: "GTM Pool",
  gtmserver: "GTM Server",
  gtmlistener: "GTM Listener",
  fw: "Firewall Rule",
  nat: "NAT",
};

function typeLabel(type: string): string {
  return TYPE_LABELS[type] || (type ? type.charAt(0).toUpperCase() + type.slice(1) : "Object");
}

function labelForKind(kind: MatchKind | "ip"): string {
  switch (kind) {
    case "exact":
      return "exact";
    case "prefix":
    case "word":
      return "name";
    case "substring":
      return "name";
    case "subsequence":
      return "fuzzy";
    case "phonetic":
      return "phonetic";
    case "body":
      return "field";
    case "ip":
      return "address";
    default:
      return "";
  }
}

/** Read every searchable row across every device into a flat candidate list.
 *  Rows are static, so this runs once. */
function collectCandidates(deviceMeta: DeviceMeta[]): Candidate[] {
  const out: Candidate[] = [];
  const devices = document.querySelectorAll<HTMLElement>(".device");
  devices.forEach((device) => {
    const di = parseInt(device.dataset.dev || "0", 10);
    const meta = deviceMeta[di] || { name: `device ${di}`, tier: null };
    const rows = device.querySelectorAll<HTMLElement>(".panel tr.searchable");
    rows.forEach((row) => {
      const el = row.querySelector<HTMLElement>("[data-oid]");
      const oid = el ? el.dataset.oid || "" : "";
      const type = oid ? oid.split(":")[0] : "";
      const ds = row.dataset.search || row.textContent || "";
      const name = (el && el.textContent ? el.textContent : ds.split(/\s+/)[0] || "").trim();
      out.push({
        row,
        deviceIndex: di,
        deviceName: meta.name,
        tier: meta.tier,
        type,
        typeLabel: typeLabel(type),
        name,
        haystack: ds.toLowerCase(),
      });
    });
  });
  return out;
}

/** Rank candidates for a parsed query. `ipPred` (when set) means the query is
 *  an address: a candidate matches iff its text overlaps that address. */
function rank(
  candidates: Candidate[],
  pq: ParsedQuery,
  ipPred: ((haystack: string) => boolean) | null,
): Scored[] {
  const terms = pq.terms.toLowerCase();
  const scored: Scored[] = [];
  for (const cand of candidates) {
    if (!deviceInScope(pq.scope, cand.deviceName, cand.tier)) continue;
    let score = 0;
    let kind: MatchKind | "ip" = "none";
    if (ipPred) {
      if (ipPred(cand.haystack)) {
        score = IP_SCORE;
        kind = "ip";
      }
    } else {
      const m = scoreText(cand.name, cand.haystack, terms);
      score = m.score;
      kind = m.kind;
    }
    if (score > 0) scored.push({ cand, score, kind });
  }
  scored.sort((a, b) => b.score - a.score || a.cand.name.localeCompare(b.cand.name));
  return scored;
}

// ---- jumping to a picked result -------------------------------------------

function activateDevice(deviceIndex: number): void {
  const tab = document.querySelector<HTMLElement>(`.dev-tab[data-dev="${deviceIndex}"]`);
  if (tab) {
    tab.click(); // reuse report.ts's device switcher
  } else {
    // Single-device report: just make sure it's the active one.
    document.querySelectorAll<HTMLElement>(".device").forEach((d) => {
      d.classList.toggle("active", d.dataset.dev === String(deviceIndex));
    });
  }
}

function jumpTo(cand: Candidate): void {
  activateDevice(cand.deviceIndex);
  const panel = cand.row.closest<HTMLElement>(".panel");
  const device = cand.row.closest<HTMLElement>(".device");
  if (panel && device) {
    const tab = device.querySelector<HTMLElement>(`.tab[data-panel="${panel.dataset.panel}"]`);
    if (tab) tab.click();
  }
  // Reveal a collapsed detail row and flash the hit.
  cand.row.classList.remove("hidden", "part-hidden");
  window.requestAnimationFrame(() => {
    cand.row.scrollIntoView({ block: "center", behavior: "smooth" });
    cand.row.classList.add("search-hit");
    window.setTimeout(() => cand.row.classList.remove("search-hit"), 2400);
  });
}

// ---- the results overlay ---------------------------------------------------

class ResultsView {
  private section: HTMLElement;
  private list: HTMLElement;
  private countEl: HTMLElement;
  private hiddenSiblings: HTMLElement[] = [];

  constructor() {
    this.section = document.createElement("section");
    this.section.id = "global-search-results";
    this.section.className = "search-results";
    this.section.hidden = true;
    const head = document.createElement("div");
    head.className = "search-results-head";
    this.countEl = document.createElement("span");
    this.countEl.className = "search-results-count";
    head.appendChild(this.countEl);
    this.list = document.createElement("div");
    this.list.className = "search-results-list";
    this.section.appendChild(head);
    this.section.appendChild(this.list);

    // Insert above the first device (or at the end of <main>/<body>).
    const firstDevice = document.querySelector(".device");
    const parent = firstDevice ? firstDevice.parentNode : document.body;
    if (firstDevice && parent) parent.insertBefore(this.section, firstDevice);
    else document.body.appendChild(this.section);
  }

  show(scored: Scored[], query: string): void {
    const shown = scored.slice(0, MAX_RESULTS);
    this.list.textContent = "";
    for (const s of shown) {
      this.list.appendChild(this.renderRow(s));
    }
    const total = scored.length;
    const more = total > shown.length ? ` (showing first ${shown.length})` : "";
    this.countEl.textContent = total
      ? `${total} result${total === 1 ? "" : "s"} for “${query}”${more}`
      : `No results for “${query}”`;
    this.section.hidden = false;
    this.hideSiblings();
  }

  hide(): void {
    this.section.hidden = true;
    this.restoreSiblings();
  }

  private renderRow(s: Scored): HTMLElement {
    const item = document.createElement("button");
    item.type = "button";
    item.className = "search-result";
    const kindLabel = labelForKind(s.kind);

    const type = document.createElement("span");
    type.className = "sr-type";
    type.textContent = s.cand.typeLabel;

    const name = document.createElement("span");
    name.className = "sr-name mono";
    name.textContent = s.cand.name || "(unnamed)";

    const dev = document.createElement("span");
    dev.className = "sr-dev";
    dev.textContent = s.cand.deviceName;

    const tier = document.createElement("span");
    tier.className = "sr-tier";
    tier.textContent = s.cand.tier === null ? "" : `tier ${s.cand.tier}`;

    const why = document.createElement("span");
    why.className = "sr-why";
    why.textContent = kindLabel;

    item.appendChild(type);
    item.appendChild(name);
    item.appendChild(dev);
    item.appendChild(tier);
    item.appendChild(why);
    item.addEventListener("click", () => {
      this.hide();
      jumpTo(s.cand);
    });
    return item;
  }

  private hideSiblings(): void {
    if (this.hiddenSiblings.length) return;
    const sel = ".summary, .device-switch, .device, .architecture";
    document.querySelectorAll<HTMLElement>(sel).forEach((el) => {
      if (el === this.section) return;
      el.classList.add("search-obscured");
      this.hiddenSiblings.push(el);
    });
  }

  private restoreSiblings(): void {
    this.hiddenSiblings.forEach((el) => el.classList.remove("search-obscured"));
    this.hiddenSiblings = [];
  }
}

// ---- wiring ----------------------------------------------------------------

export function initGlobalSearch(cfg: GlobalSearchConfig): void {
  const input = cfg.input;
  const deviceNames = cfg.deviceMeta.map((d) => d.name);
  const candidates = collectCandidates(cfg.deviceMeta);
  const view = new ResultsView();

  function run(): void {
    const raw = input.value.trim();
    if (!raw) {
      view.hide();
      return;
    }
    const ipPred = cfg.ipMatcher ? cfg.ipMatcher(raw) : null;
    const pq: ParsedQuery = ipPred
      ? { scope: { tier: null, dev: null }, terms: "", hadScope: false }
      : parseQuery(raw, deviceNames);
    const scored = rank(candidates, pq, ipPred);
    view.show(scored, raw);
  }

  input.addEventListener("input", run);
  input.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      input.value = "";
      view.hide();
    }
  });
  // A stale value (e.g. restored on reload) should render immediately.
  if (input.value.trim()) run();
}

export const __test = { collectCandidates, rank, typeLabel, KIND_SCORE };
