// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/bigip-report-gen/frontend/src — DO NOT EDIT; edit the .ts source.
"use strict";
(() => {
  // src/search/matcher.ts
  var KIND_SCORE = {
    exact: 100,
    prefix: 85,
    word: 75,
    substring: 60,
    subsequence: 45,
    body: 40,
    phonetic: 30,
    scope: 5,
    none: 0
  };
  function subsequenceMatch(hay, needle) {
    if (!needle) return true;
    let i = 0;
    for (let j = 0; j < hay.length && i < needle.length; j++) {
      if (hay[j] === needle[i]) i++;
    }
    return i === needle.length;
  }
  function nameTokens(name) {
    return name.toLowerCase().split(/[^a-z0-9]+/).filter(Boolean);
  }
  function soundex(word) {
    const s = word.toUpperCase().replace(/[^A-Z]/g, "");
    if (!s) return "";
    const code = (c) => {
      if ("BFPV".includes(c)) return "1";
      if ("CGJKQSXZ".includes(c)) return "2";
      if ("DT".includes(c)) return "3";
      if (c === "L") return "4";
      if ("MN".includes(c)) return "5";
      if (c === "R") return "6";
      return "";
    };
    let out = s[0];
    let prev = code(s[0]);
    for (let i = 1; i < s.length && out.length < 4; i++) {
      const d = code(s[i]);
      if (d && d !== prev) out += d;
      if (s[i] !== "H" && s[i] !== "W") prev = d;
    }
    return (out + "000").slice(0, 4);
  }
  function metaphone(word) {
    let s = word.toUpperCase().replace(/[^A-Z]/g, "");
    if (!s) return "";
    s = s.replace(/^(AE|GN|KN|PN|WR)/, (m) => m[1]);
    if (s.startsWith("X")) s = "S" + s.slice(1);
    s = s.replace(/^WH/, "W");
    const isVowel = (c) => "AEIOU".includes(c);
    let out = "";
    for (let i = 0; i < s.length; i++) {
      const c = s[i];
      const next = s[i + 1] || "";
      const prev = s[i - 1] || "";
      if (c === prev && c !== "C") continue;
      switch (c) {
        case "A":
        case "E":
        case "I":
        case "O":
        case "U":
          if (i === 0) out += c;
          break;
        case "B":
          if (!(i === s.length - 1 && prev === "M")) out += "B";
          break;
        case "C":
          if (next === "H") out += "X";
          else if ("IEY".includes(next)) out += "S";
          else out += "K";
          break;
        case "D":
          out += "T";
          break;
        case "G":
          if (next === "H" && !isVowel(s[i + 2] || "")) break;
          out += "K";
          break;
        case "H":
          if (isVowel(prev) && !isVowel(next)) break;
          out += "H";
          break;
        case "K":
          if (prev !== "C") out += "K";
          break;
        case "P":
          out += next === "H" ? "F" : "P";
          break;
        case "Q":
          out += "K";
          break;
        case "S":
          out += next === "H" ? "X" : "S";
          break;
        case "T":
          out += next === "H" ? "0" : "T";
          break;
        case "V":
          out += "F";
          break;
        case "W":
        case "Y":
          if (isVowel(next)) out += c;
          break;
        case "X":
          out += "KS";
          break;
        case "Z":
          out += "S";
          break;
        default:
          out += c;
      }
    }
    return out;
  }
  function phoneticEqual(a, b) {
    if (a.length < 2 || b.length < 2) return false;
    const sa = soundex(a);
    if (sa && sa === soundex(b)) return true;
    const ma = metaphone(a);
    return !!ma && ma === metaphone(b);
  }
  function scoreText(name, haystack, terms) {
    const q = terms.trim();
    if (!q) return { score: KIND_SCORE.scope, kind: "scope" };
    const nameL = name.toLowerCase();
    if (nameL === q) return { score: KIND_SCORE.exact, kind: "exact" };
    if (nameL.startsWith(q)) return { score: KIND_SCORE.prefix, kind: "prefix" };
    const tokens = nameTokens(name);
    if (tokens.some((t) => t.startsWith(q))) return { score: KIND_SCORE.word, kind: "word" };
    if (nameL.includes(q)) return { score: KIND_SCORE.substring, kind: "substring" };
    if (subsequenceMatch(nameL, q)) return { score: KIND_SCORE.subsequence, kind: "subsequence" };
    if (haystack.includes(q)) return { score: KIND_SCORE.body, kind: "body" };
    if (!q.includes(" ") && tokens.some((t) => phoneticEqual(t, q))) {
      return { score: KIND_SCORE.phonetic, kind: "phonetic" };
    }
    return { score: 0, kind: "none" };
  }

  // src/search/query.ts
  var EMPTY_SCOPE = { tier: null, dev: null };
  function matchesDeviceName(prefix, deviceNames) {
    const p = prefix.toLowerCase();
    if (!p) return false;
    return deviceNames.some((n) => {
      const nl = n.toLowerCase();
      return nl.includes(p) || subsequenceMatch(nl, p);
    });
  }
  function parseQuery(raw, deviceNames) {
    const s = raw.trim();
    if (!s) return { scope: { ...EMPTY_SCOPE }, terms: "", hadScope: false };
    const tierM = /^t(\d+):(.*)$/i.exec(s);
    if (tierM) {
      return {
        scope: { tier: parseInt(tierM[1], 10), dev: null },
        terms: tierM[2].trim(),
        hadScope: true
      };
    }
    const devM = /^([^:\s]+):(.*)$/.exec(s);
    if (devM && matchesDeviceName(devM[1], deviceNames)) {
      return {
        scope: { tier: null, dev: devM[1] },
        terms: devM[2].trim(),
        hadScope: true
      };
    }
    return { scope: { ...EMPTY_SCOPE }, terms: s, hadScope: false };
  }
  function deviceInScope(scope, deviceName, deviceTier) {
    if (scope.tier !== null && deviceTier !== scope.tier) return false;
    if (scope.dev !== null && !matchesDeviceName(scope.dev, [deviceName])) return false;
    return true;
  }

  // src/search/results.ts
  var IP_SCORE = 65;
  var MAX_RESULTS = 400;
  var TYPE_LABELS = {
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
    nat: "NAT"
  };
  function typeLabel(type) {
    return TYPE_LABELS[type] || (type ? type.charAt(0).toUpperCase() + type.slice(1) : "Object");
  }
  function labelForKind(kind) {
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
  function collectCandidates(deviceMeta) {
    const out = [];
    const devices = document.querySelectorAll(".device");
    devices.forEach((device) => {
      const di = parseInt(device.dataset.dev || "0", 10);
      const meta = deviceMeta[di] || { name: `device ${di}`, tier: null };
      const rows = device.querySelectorAll(".panel tr.searchable");
      rows.forEach((row) => {
        const el = row.querySelector("[data-oid]");
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
          haystack: ds.toLowerCase()
        });
      });
    });
    return out;
  }
  function rank(candidates, pq, ipPred) {
    const terms = pq.terms.toLowerCase();
    const scored = [];
    for (const cand of candidates) {
      if (!deviceInScope(pq.scope, cand.deviceName, cand.tier)) continue;
      let score = 0;
      let kind = "none";
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
  function activateDevice(deviceIndex) {
    const tab = document.querySelector(`.dev-tab[data-dev="${deviceIndex}"]`);
    if (tab) {
      tab.click();
    } else {
      document.querySelectorAll(".device").forEach((d) => {
        d.classList.toggle("active", d.dataset.dev === String(deviceIndex));
      });
    }
  }
  function jumpTo(cand) {
    activateDevice(cand.deviceIndex);
    const panel = cand.row.closest(".panel");
    const device = cand.row.closest(".device");
    if (panel && device) {
      const tab = device.querySelector(`.tab[data-panel="${panel.dataset.panel}"]`);
      if (tab) tab.click();
    }
    cand.row.classList.remove("hidden", "part-hidden");
    window.requestAnimationFrame(() => {
      cand.row.scrollIntoView({ block: "center", behavior: "smooth" });
      cand.row.classList.add("search-hit");
      window.setTimeout(() => cand.row.classList.remove("search-hit"), 2400);
    });
  }
  var ResultsView = class {
    constructor() {
      this.hiddenSiblings = [];
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
      const firstDevice = document.querySelector(".device");
      const parent = firstDevice ? firstDevice.parentNode : document.body;
      if (firstDevice && parent) parent.insertBefore(this.section, firstDevice);
      else document.body.appendChild(this.section);
    }
    show(scored, query) {
      const shown = scored.slice(0, MAX_RESULTS);
      this.list.textContent = "";
      for (const s of shown) {
        this.list.appendChild(this.renderRow(s));
      }
      const total = scored.length;
      const more = total > shown.length ? ` (showing first ${shown.length})` : "";
      this.countEl.textContent = total ? `${total} result${total === 1 ? "" : "s"} for \u201C${query}\u201D${more}` : `No results for \u201C${query}\u201D`;
      this.section.hidden = false;
      this.hideSiblings();
    }
    hide() {
      this.section.hidden = true;
      this.restoreSiblings();
    }
    renderRow(s) {
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
    hideSiblings() {
      if (this.hiddenSiblings.length) return;
      const sel = ".summary, .device-switch, .device, .architecture";
      document.querySelectorAll(sel).forEach((el) => {
        if (el === this.section) return;
        el.classList.add("search-obscured");
        this.hiddenSiblings.push(el);
      });
    }
    restoreSiblings() {
      this.hiddenSiblings.forEach((el) => el.classList.remove("search-obscured"));
      this.hiddenSiblings = [];
    }
  };
  function initGlobalSearch(cfg) {
    const input = cfg.input;
    const deviceNames = cfg.deviceMeta.map((d) => d.name);
    const candidates = collectCandidates(cfg.deviceMeta);
    const view = new ResultsView();
    function run() {
      const raw = input.value.trim();
      if (!raw) {
        view.hide();
        return;
      }
      const ipPred = cfg.ipMatcher ? cfg.ipMatcher(raw) : null;
      const pq = ipPred ? { scope: { tier: null, dev: null }, terms: "", hadScope: false } : parseQuery(raw, deviceNames);
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
    if (input.value.trim()) run();
  }

  // src/arch/guide.ts
  var DSL_GUIDE = `# Architecture & topology manifest \u2014 a small Tcl script, one command per line.
# Comments start with '#'. Brace {a value} to keep whitespace as one word.

# Devices: role (gtm/ltm/afm) + tier (0 = front) + display label.
device edge.ucs -role ltm -tier 1 -label "Edge"
device core.ucs -role ltm -tier 2

# Explicit inter-device links (auto-detection from address overlap still runs).
link edge.ucs core.ucs -label internal

# Network zones \u2014 named sets of IP ranges (v4 and v6).
zone external -cidr 0.0.0.0/0
zone dmz      -cidr 192.0.2.0/24 -cidr 2001:db8:dmz::/48
zone internal -cidr 10.0.0.0/8 -cidr 172.16.0.0/12

# Device interfaces attached to a zone (+ address).
interface edge.ucs ext0 -zone external -address 203.0.113.10

# DNS zones (the .zone file is uploaded / supplied as a side-input).
dns-zone example.com -file example.com.zone -zone dmz

# Enrichment maps.
cidr-name   10.1.0.0/16 {Datacenter A}
service-map -file services.csv     ;# port,name overrides (default: F5 table)
nat-map     -file nat.csv           ;# source,dest[,source_cidr,dest_cidr]
`;

  // src/arch/editor.ts
  function win() {
    return window;
  }
  function readModel() {
    const el = document.getElementById("f5-model");
    if (!el || !el.textContent) return null;
    try {
      return JSON.parse(el.textContent);
    } catch {
      return null;
    }
  }
  function reportId() {
    const id = document.documentElement.getAttribute("data-report-id");
    return id && id.trim() ? id.trim() : "default";
  }
  function b64ToBytes(b64) {
    const bin = atob(b64.trim());
    const u = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) u[i] = bin.charCodeAt(i);
    return u;
  }
  var wasmReady = null;
  function initWasm() {
    const w = win();
    const payload = document.getElementById("f5-wasm");
    if (typeof w.wasm_bindgen !== "function" || !payload || !payload.textContent) return null;
    if (!wasmReady) {
      wasmReady = Promise.resolve(w.wasm_bindgen(b64ToBytes(payload.textContent))).then(() => void 0);
    }
    return wasmReady;
  }
  function initArchEditor() {
    const model = readModel();
    if (!model || !model.architecture) return;
    const anchor = document.getElementById("archDiagram") || document.querySelector(".f5q-manual") || document.querySelector(".device");
    if (!anchor) return;
    const insertAfter = anchor.id === "archDiagram";
    const key = `f5arch:${reportId()}`;
    const saved = safeGet(key);
    const section = document.createElement("details");
    section.className = "arch-editor";
    section.innerHTML = `
    <summary>\u270F\uFE0F Edit architecture &amp; topology (DSL)</summary>
    <p class="arch-editor-hint">Describe the estate \u2014 device roles/tiers, links, network zones,
      DNS zones and enrichment maps \u2014 in the manifest DSL. <b>Apply</b> re-runs detection and
      redraws the diagram (needs the query console); <b>Export</b> saves a <code>.tcl</code> you can
      feed back into the generator to pin it. Saved per report in this browser.</p>
    <textarea class="arch-editor-ta" spellcheck="false" rows="12"></textarea>
    <div class="arch-editor-actions">
      <button type="button" class="arch-apply">Apply</button>
      <button type="button" class="arch-export">Export .tcl \u2193</button>
      <button type="button" class="arch-reset">Reset</button>
      <span class="arch-editor-status"></span>
    </div>
    <details class="arch-editor-guide"><summary>DSL reference</summary><pre></pre></details>`;
    anchor.parentNode?.insertBefore(section, insertAfter ? anchor.nextSibling : anchor);
    const ta = section.querySelector(".arch-editor-ta");
    const status = section.querySelector(".arch-editor-status");
    ta.value = saved ?? "";
    ta.placeholder = DSL_GUIDE;
    section.querySelector(".arch-editor-guide pre").textContent = DSL_GUIDE;
    const setStatus = (m) => {
      status.textContent = m;
    };
    section.querySelector(".arch-reset").addEventListener("click", () => {
      ta.value = "";
      safeSet(key, "");
      setStatus("cleared");
    });
    section.querySelector(".arch-export").addEventListener("click", () => {
      const text = ta.value.trim();
      if (!text) {
        setStatus("nothing to export");
        return;
      }
      const url = URL.createObjectURL(new Blob([text + "\n"], { type: "text/plain" }));
      const a = document.createElement("a");
      a.href = url;
      a.download = "architecture.tcl";
      document.body.appendChild(a);
      a.click();
      a.remove();
      setTimeout(() => URL.revokeObjectURL(url), 1e3);
    });
    section.querySelector(".arch-apply").addEventListener("click", () => {
      const manifest = ta.value;
      safeSet(key, manifest);
      setStatus("saved");
      void applyManifest(model, manifest, setStatus);
    });
    if (saved && saved.trim()) void applyManifest(model, saved, setStatus);
  }
  async function applyManifest(model, manifest, setStatus) {
    const w = win();
    if (typeof w.wasm_bindgen !== "function" || !w.wasm_bindgen.build_architecture) {
      setStatus("saved \u2014 regenerate the report (or open the query console) to redraw");
      return;
    }
    const ready = initWasm();
    if (!ready) {
      setStatus("saved \u2014 engine unavailable; regenerate to apply");
      return;
    }
    try {
      await ready;
      const archJson = w.wasm_bindgen.build_architecture(JSON.stringify(model.devices ?? []), manifest);
      const arch = JSON.parse(archJson);
      model.architecture = arch;
      await redrawDiagram(arch);
      setStatus("applied \u2014 diagram updated");
    } catch (e) {
      setStatus("error: " + (e instanceof Error ? e.message : String(e)));
    }
  }
  async function redrawDiagram(arch) {
    const host = document.getElementById("archDiagram");
    const elk = win().ElkGraph;
    if (!host || !arch.graph || !elk) return;
    let model;
    try {
      model = JSON.parse(arch.graph);
    } catch {
      return;
    }
    await elk.render(host, model, { dir: "RIGHT", svgClass: "elk-report" });
  }
  function safeGet(k) {
    try {
      return localStorage.getItem(k);
    } catch {
      return null;
    }
  }
  function safeSet(k, v) {
    try {
      localStorage.setItem(k, v);
    } catch {
    }
  }

  // src/pages/topology.ts
  (function() {
    "use strict";
    var MODEL = null;
    try {
      MODEL = JSON.parse(document.getElementById("f5-model").textContent);
    } catch (e) {
      return;
    }
    var TYPE_CLASS = {
      vs: "vs",
      pool: "pool",
      node: "node",
      mon: "mon",
      rule: "rule",
      prof: "prof",
      persist: "persist",
      policy: "policy",
      snat: "snat",
      dg: "dg"
    };
    var TYPE_LABEL = {
      vs: "Virtual",
      pool: "Pool",
      node: "Node",
      mon: "Monitor",
      rule: "iRule",
      prof: "Profile",
      persist: "Persistence",
      policy: "Policy",
      snat: "SNAT Pool",
      dg: "Data Group"
    };
    var TYPE_PANEL = {
      vs: "virtuals",
      pool: "pools",
      node: "nodes",
      mon: "monitors",
      rule: "rules",
      prof: "profiles",
      dg: "dataGroups"
    };
    var CONTAINER = {
      vs: ".ltm.virtual",
      pool: ".ltm.pool",
      node: ".ltm.node",
      mon: ".ltm.monitor",
      rule: ".ltm.rule",
      prof: ".ltm.profile",
      persist: ".ltm.persistence",
      policy: ".ltm.policy",
      snat: ".ltm.snatpool",
      dg: '.ltm."data-group"'
    };
    function selectorFor(node) {
      var c = CONTAINER[node.type] || ".ltm";
      return c + '[] | select(."full-path" == "' + node.fullPath + '")';
    }
    function showSelector(node) {
      var bar = document.getElementById("f5qbar");
      if (!bar || !node) return;
      var expr = selectorFor(node);
      setBarExpr(bar, TYPE_LABEL[node.type] || node.type, expr);
      bar.classList.add("show");
      var dEl = document.querySelector(".device.active");
      bar.dataset.dev = dEl ? dEl.dataset.dev : "0";
      bar.dataset.oid = node.oid || "";
    }
    function setBarExpr(bar, typeLabel2, expr) {
      bar.querySelector(".f5qbar-type").textContent = typeLabel2;
      bar.querySelector(".f5qbar-expr").textContent = expr;
      bar.dataset.expr = expr;
    }
    function forwardPath(ix, srcOid, dstOid) {
      if (srcOid === dstOid) return [srcOid];
      var prev = {}, seen = {};
      seen[srcOid] = true;
      var queue = [srcOid];
      while (queue.length) {
        var cur = queue.shift();
        var nbrs = ix.fadj[cur] || [];
        for (var i = 0; i < nbrs.length; i++) {
          var nb = nbrs[i];
          if (seen[nb]) continue;
          seen[nb] = true;
          prev[nb] = cur;
          if (nb === dstOid) {
            var path = [nb];
            var p = cur;
            while (p !== void 0) {
              path.unshift(p);
              p = prev[p];
            }
            return path;
          }
          queue.push(nb);
        }
      }
      return null;
    }
    function reachableFrom(ix, srcOid) {
      var seen = {}, out = [], queue = [srcOid];
      while (queue.length) {
        var cur = queue.shift();
        (ix.fadj[cur] || []).forEach(function(nb) {
          if (!seen[nb]) {
            seen[nb] = true;
            out.push(nb);
            queue.push(nb);
          }
        });
      }
      return out;
    }
    function stepFor(ix, edge, targetOid) {
      var fp = ix.byOid[targetOid].fullPath;
      switch (edge.kind) {
        case "pool":
          return " | .pool";
        case "snat":
          return " | .snatpool";
        case "monitor":
          return " | .monitor";
        case "rule":
          return ' | .rules[] | select(. == "' + fp + '")';
        case "profile":
          return ' | .profiles[] | select(startswith("' + fp + '"))';
        case "persist":
          return ' | .persist[] | select(. == "' + fp + '")';
        case "policy":
          return ' | .policies[] | select(. == "' + fp + '")';
        case "pool-irule":
          return ' | .refs.pools[] | select(. == "' + fp + '")';
        case "datagroup":
          return ' | .refs."data-groups"[] | select(. == "' + fp + '")';
        case "member":
          var addr = ix.nodeAddr[fp];
          return addr ? ' | .members[] | select(.address == "' + addr + '")' : ' | .members[] | select(.name | startswith("' + fp + ':"))';
        default:
          return ' | refs[] | select(. == "' + fp + '")';
      }
    }
    function pathQuery(ix, path) {
      var q = selectorFor(ix.byOid[path[0]]);
      for (var i = 1; i < path.length; i++) {
        var e = ix.edgeDir[path[i - 1] + "->" + path[i]];
        if (!e) return null;
        q += stepFor(ix, e, path[i]);
      }
      return q;
    }
    function copyText(t) {
      if (navigator.clipboard && navigator.clipboard.writeText) {
        return navigator.clipboard.writeText(t).catch(function() {
          fallbackCopy(t);
        });
      }
      fallbackCopy(t);
      return Promise.resolve();
    }
    function fallbackCopy(t) {
      var ta = document.createElement("textarea");
      ta.value = t;
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.focus();
      ta.select();
      try {
        document.execCommand("copy");
      } catch (e) {
      }
      document.body.removeChild(ta);
    }
    (function initBar() {
      var bar = document.getElementById("f5qbar");
      if (!bar) return;
      var btn = bar.querySelector(".f5qbar-copy");
      btn.addEventListener("click", function() {
        copyText(bar.dataset.expr || "").then(function() {
          var old = btn.textContent;
          btn.textContent = "\u2713";
          btn.classList.add("ok");
          setTimeout(function() {
            btn.textContent = old;
            btn.classList.remove("ok");
          }, 1100);
        });
      });
      bar.querySelector(".f5qbar-expr").addEventListener("click", function() {
        var r = document.createRange();
        r.selectNodeContents(this);
        var s = window.getSelection();
        s.removeAllRanges();
        s.addRange(r);
      });
      bar.querySelector(".f5qbar-close").addEventListener("click", function() {
        bar.classList.remove("show");
      });
      var pop = bar.querySelector(".f5qbar-pop");
      var srcSel = pop.querySelector(".pb-src");
      var dstSel = pop.querySelector(".pb-dst");
      var note = pop.querySelector(".pb-note");
      var pathBtn = bar.querySelector(".f5qbar-path");
      function ixForBar() {
        return IDX[parseInt(bar.dataset.dev || "0", 10)];
      }
      function optLabel(ix, oid) {
        var n = ix.byOid[oid];
        return (TYPE_LABEL[n.type] || n.type) + ": " + n.name;
      }
      function fillSources(ix, selectedOid) {
        var nodes = ix.d.graph.nodes.slice().sort(function(a, b) {
          return (a.type + a.name).localeCompare(b.type + b.name);
        });
        srcSel.textContent = "";
        nodes.forEach(function(n) {
          srcSel.appendChild(optionEl(n.oid, optLabel(ix, n.oid), n.oid === selectedOid));
        });
      }
      function fillDests(ix) {
        var reach = reachableFrom(ix, srcSel.value).map(function(o) {
          return { oid: o, label: optLabel(ix, o) };
        }).sort(function(a, b) {
          return a.label.localeCompare(b.label);
        });
        dstSel.textContent = "";
        if (!reach.length) {
          dstSel.appendChild(optionEl("", "(nothing reachable downstream)", false));
        } else {
          reach.forEach(function(r) {
            dstSel.appendChild(optionEl(r.oid, r.label, false));
          });
        }
        build();
      }
      function build() {
        var ix = ixForBar();
        var s = srcSel.value, d = dstSel.value;
        if (!s || !d) {
          note.textContent = "";
          return;
        }
        var path = forwardPath(ix, s, d);
        if (!path) {
          note.textContent = "No downstream path from source to destination.";
          return;
        }
        var q = pathQuery(ix, path);
        if (!q) {
          note.textContent = "Could not express this path as a query.";
          return;
        }
        var hops = path.map(function(o) {
          return ix.byOid[o].name;
        }).join("  \u2192  ");
        note.textContent = hops;
        setBarExpr(bar, "path", q);
      }
      pathBtn.addEventListener("click", function() {
        var ix = ixForBar();
        var open = pop.classList.toggle("open");
        pathBtn.classList.toggle("active", open);
        if (open) {
          fillSources(ix, bar.dataset.oid || (ix.d.graph.nodes[0] || {}).oid);
          fillDests(ix);
        }
      });
      srcSel.addEventListener("change", function() {
        fillDests(ixForBar());
      });
      dstSel.addEventListener("change", build);
      document.addEventListener("keydown", function(e) {
        if (e.key === "Escape") {
          pop.classList.remove("open");
          pathBtn.classList.remove("active");
        }
      });
    })();
    function indexDevice(d) {
      var byOid = {}, adj = {}, fadj = {}, short = {}, unshort = {}, i = 0;
      d.graph.nodes.forEach(function(n) {
        byOid[n.oid] = n;
        adj[n.oid] = {};
        fadj[n.oid] = [];
        var sid = "N" + i++;
        short[n.oid] = sid;
        unshort[sid] = n.oid;
      });
      var edgesByPair = {}, edgeDir = {};
      d.graph.edges.forEach(function(e) {
        if (!(e.from in adj) || !(e.to in adj)) return;
        adj[e.from][e.to] = true;
        adj[e.to][e.from] = true;
        fadj[e.from].push(e.to);
        edgeDir[e.from + "->" + e.to] = e;
        edgesByPair[short[e.from] + "|" + short[e.to]] = e;
        edgesByPair[short[e.to] + "|" + short[e.from]] = e;
      });
      var nodeAddr = {};
      (d.nodes || []).forEach(function(n) {
        nodeAddr[n.fullPath] = n.address;
      });
      var byPath = {};
      d.graph.nodes.forEach(function(n) {
        if (!(n.fullPath in byPath)) byPath[n.fullPath] = n.oid;
      });
      return {
        d,
        byOid,
        adj,
        fadj,
        short,
        unshort,
        edgesByPair,
        edgeDir,
        nodeAddr,
        byPath
      };
    }
    var IDX = MODEL.devices.map(indexDevice);
    function optionEl(value, label, selected) {
      var o = document.createElement("option");
      o.value = value;
      o.textContent = label;
      if (selected) o.selected = true;
      return o;
    }
    var ESC_MAP = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" };
    function esc(s) {
      return String(s).replace(/[&<>"]/g, function(c) {
        return ESC_MAP[c];
      }).replace(/\n/g, " ");
    }
    function escConf(s) {
      return String(s).replace(/[&<>"]/g, function(c) {
        return ESC_MAP[c];
      });
    }
    var SAFE_MARKUP = /^(?:<\/span>|<span class="tk-[\w -]+">|&(?:amp|lt|gt|quot);)$/;
    function sanitiseHtml(markup) {
      return String(markup).replace(/<[^<>]*>|&[^\s&;]*;?|[<>"]/g, function(m0) {
        return SAFE_MARKUP.test(m0) ? m0 : escConf(m0);
      });
    }
    function stanzaFor(cfg, fullPath) {
      if (!cfg || !fullPath) return null;
      var q = fullPath.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
      var re = new RegExp("^[^\\n]*?\\s" + q + "\\s*\\{", "m");
      var m = re.exec(cfg);
      if (!m) return null;
      var i = cfg.indexOf("{", m.index);
      if (i < 0) return null;
      var depth = 0;
      for (var j = i; j < cfg.length; j++) {
        var c = cfg[j];
        if (c === "{") depth++;
        else if (c === "}") {
          depth--;
          if (depth === 0) {
            j++;
            return cfg.slice(m.index, j);
          }
        }
      }
      return cfg.slice(m.index);
    }
    function highlightConf(text, isLinked) {
      var out = escConf(text);
      out = out.replace(/(\/[A-Za-z][\w.\-]*(?:\/[\w.\-]+)+)/g, function(m0) {
        return isLinked && isLinked(m0) ? '<a class="conf-path conf-link" data-goto="' + m0 + '">' + m0 + "</a>" : '<span class="conf-path">' + m0 + "</span>";
      });
      out = out.replace(/^(\s*)([a-z][\w-]+)/gm, '$1<span class="conf-key">$2</span>');
      return out;
    }
    function ruleByPath(ix, fp) {
      var rules = ix.d.rules || [];
      for (var i = 0; i < rules.length; i++) {
        if (rules[i].fullPath === fp) return rules[i];
      }
      return null;
    }
    function sourceHtml(ix, n) {
      if (n.type === "rule") {
        var r = ruleByPath(ix, n.fullPath);
        if (r && (r.bodyHtml || r.body)) {
          return '<pre class="code tcl">' + (r.bodyHtml ? sanitiseHtml(r.bodyHtml) : escConf(r.body || "")) + "</pre>";
        }
      }
      var stanza = stanzaFor(ix.d.configText, n.fullPath);
      if (!stanza) return "";
      var html = highlightConf(stanza, function(fp) {
        return fp !== n.fullPath && !!ix.byPath[fp];
      });
      return '<pre class="code conf">' + html + "</pre>";
    }
    function refChip(p, extra) {
      if (!p) return '<span class="muted">\u2014</span>';
      return '<span class="tag ' + (extra || "") + '" data-oref="' + esc(p) + '" title="' + esc(p) + '">' + esc(p.split("/").pop()) + "</span>";
    }
    function refChips(paths, extra) {
      if (!paths || !paths.length) return '<span class="muted">none</span>';
      return paths.map(function(p) {
        return refChip(p, extra);
      }).join(" ");
    }
    function isDefaultNode(ix, oid) {
      var n = ix.byOid[oid];
      return !!(n && n.isDefault);
    }
    function neighborhood(ix, startOids, depth) {
      var seen = {}, frontier = [], d = 0;
      startOids.forEach(function(o) {
        if (ix.byOid[o]) {
          seen[o] = true;
          frontier.push(o);
        }
      });
      while (frontier.length && d < depth) {
        var next = [];
        frontier.forEach(function(o) {
          if (isDefaultNode(ix, o)) return;
          Object.keys(ix.adj[o] || {}).forEach(function(nb) {
            if (!seen[nb]) {
              seen[nb] = true;
              next.push(nb);
            }
          });
        });
        frontier = next;
        d++;
      }
      return seen;
    }
    function buildFlowchart(ix, oids, opts) {
      opts = opts || {};
      var nodeSet = {};
      oids.forEach(function(o) {
        nodeSet[o] = true;
      });
      var enodes = [], eedges = [];
      Object.keys(nodeSet).forEach(function(o) {
        var n = ix.byOid[o];
        if (!n) return;
        enodes.push({ id: ix.short[o], label: n.name, cls: TYPE_CLASS[n.type] || "default" });
      });
      ix.d.graph.edges.forEach(function(e) {
        if (!nodeSet[e.from] || !nodeSet[e.to]) return;
        eedges.push({
          from: ix.short[e.from],
          to: ix.short[e.to],
          label: e.label || "",
          dashed: e.kind === "pool-irule"
        });
      });
      return { nodes: enodes, edges: eedges, dir: opts.dir === "TB" ? "DOWN" : "RIGHT" };
    }
    var PROFILE_LAYER = {
      TCP: { layer: 1 },
      UDP: { layer: 1 },
      SCTP: { layer: 1 },
      FASTL4: { layer: 1 },
      CLIENT_SSL: { layer: 2, side: "client" },
      SERVER_SSL: { layer: 2, side: "server" },
      HTTP: { layer: 3 },
      HTTP2: { layer: 3 },
      ONE_CONNECT: { layer: 3 },
      WEBSOCKET: { layer: 3 },
      REWRITE: { layer: 3 },
      HTML: { layer: 3 },
      FTP: { layer: 3 },
      DNS: { layer: 3 },
      SIP: { layer: 3 },
      DIAMETER: { layer: 3 },
      RADIUS: { layer: 3 },
      FIX: { layer: 3 },
      MQTT: { layer: 3 },
      STREAM: { layer: 3 },
      FASTHTTP: { layer: 3 }
    };
    var EVENT_PHASE = (function() {
      var o = {};
      [
        ["CLIENT_ACCEPTED", "client"],
        ["CLIENTSSL_CLIENTHELLO", "client"],
        ["CLIENTSSL_CLIENTCERT", "client"],
        ["CLIENTSSL_HANDSHAKE", "client"],
        ["CLIENTSSL_SERVERHELLO_SEND", "client"],
        ["HTTP_REQUEST", "client"],
        ["HTTP_REQUEST_DATA", "client"],
        ["HTTP_PROXY_REQUEST", "client"],
        ["STREAM_MATCHED", "client"],
        ["CACHE_REQUEST", "client"],
        ["CLIENT_DATA", "client"],
        ["LB_SELECTED", "lb"],
        ["LB_FAILED", "lb"],
        ["PERSIST_DOWN", "lb"],
        ["SERVER_CONNECTED", "server"],
        ["SERVERSSL_HANDSHAKE", "server"],
        ["SERVERSSL_CLIENTCERT", "server"],
        ["SERVER_DATA", "server"],
        ["HTTP_RESPONSE", "server"],
        ["HTTP_RESPONSE_CONTINUE", "server"],
        ["HTTP_RESPONSE_DATA", "server"],
        ["CACHE_RESPONSE", "server"],
        ["HTTP_RESPONSE_RELEASE", "server"]
      ].forEach(function(e, i) {
        o[e[0]] = { phase: e[1], order: i };
      });
      return o;
    })();
    function eventPhase(ev) {
      return EVENT_PHASE[ev] || { phase: "client", order: 900 };
    }
    function findByPath(list, fp) {
      for (var i = 0; i < (list || []).length; i++) {
        if (list[i].fullPath === fp || list[i].fullPath.split("/").pop() === fp.split("/").pop()) return list[i];
      }
      return null;
    }
    function poolNodeNames(ix, poolFp) {
      if (!poolFp) return [];
      var pool = findByPath(ix.d.pools, poolFp);
      if (pool && pool.members && pool.members.length) {
        return pool.members.map(function(m) {
          return m.name || m.address || "";
        }).filter(Boolean);
      }
      var out = [];
      (ix.fadj["pool:" + poolFp] || []).forEach(function(nb) {
        var n = ix.byOid[nb];
        if (n && n.type === "node") out.push(n.name);
      });
      return out;
    }
    function ruleTrafficEffects(ix, vs) {
      var out = { v2v: [], hsl: [], sideband: false };
      var seenV = {}, seenH = {};
      (vs.rules || []).forEach(function(rp) {
        var rule = findByPath(ix.d.rules, rp);
        if (!rule || !rule.body) return;
        var body = rule.body, m;
        var rv = /(?:^|[\n;{[])\s*virtual\s+("[^"]+"|[^\s\]]+)/g;
        while (m = rv.exec(body)) {
          var tgt = m[1].replace(/^"|"$/g, "").trim();
          if (!tgt || tgt.charAt(0) === "$") continue;
          if (!seenV[tgt]) {
            seenV[tgt] = 1;
            out.v2v.push({ target: tgt });
          }
        }
        if (/\bHSL::(open|send)\b/.test(body)) {
          var pub = /\bHSL::open\b[^\n\]]*-publisher\s+("[^"]+"|[^\s\]]+)/.exec(body);
          var p = pub ? pub[1].replace(/^"|"$/g, "") : "";
          if (!seenH[p]) {
            seenH[p] = 1;
            out.hsl.push({ publisher: p });
          }
        }
        if (/(?:^|[\n;{[])\s*connect\s/.test(body)) out.sideband = true;
      });
      return out;
    }
    function vipDestMap(ix) {
      if (ix._vipMap) return ix._vipMap;
      var map = {};
      (ix.d.virtuals || []).forEach(function(v) {
        var L = v.listener || {};
        if (L.address != null && L.portRaw != null && L.portRaw !== "") map[L.address + ":" + L.portRaw] = v;
      });
      ix._vipMap = map;
      return map;
    }
    function buildTrafficPipeline(ix, vsOid) {
      var vs = findByPath(ix.d.virtuals, vsOid.replace(/^vs:/, ""));
      if (!vs) return null;
      var byLayer = { 1: [], 2: [], 3: [] };
      (vs.profiles || []).forEach(function(fp) {
        var prof = findByPath(ix.d.profiles, fp);
        var type = prof ? prof.type : "";
        var info = PROFILE_LAYER[type] || { layer: 3 };
        byLayer[info.layer].push({ name: prof ? prof.name : fp.split("/").pop(), side: info.side, oid: "prof:" + fp });
      });
      var l4 = byLayer[1], l7 = byLayer[3];
      var clientSSL = byLayer[2].filter(function(p) {
        return p.side !== "server";
      });
      var serverSSL = byLayer[2].filter(function(p) {
        return p.side === "server";
      });
      var eventMap = {};
      (vs.rules || []).forEach(function(rp) {
        var rule = findByPath(ix.d.rules, rp);
        if (!rule || !rule.body) return;
        var re = /when\s+([A-Z][A-Z0-9_]*)((?:\s+priority\s+\d+)?)/g, m;
        while (m = re.exec(rule.body)) {
          var pm = /priority\s+(\d+)/.exec(m[2]);
          (eventMap[m[1]] = eventMap[m[1]] || []).push({ rule: rule.name, prio: pm ? parseInt(pm[1], 10) : 500 });
        }
      });
      function eventsIn(phase) {
        return Object.keys(eventMap).filter(function(ev) {
          return eventPhase(ev).phase === phase;
        }).sort(function(a, b) {
          return eventPhase(a).order - eventPhase(b).order;
        }).map(function(ev) {
          var hs = eventMap[ev].slice().sort(function(x, y) {
            return x.prio - y.prio;
          });
          var rules = {};
          hs.forEach(function(h) {
            rules[h.rule] = 1;
          });
          if (Object.keys(rules).length > 1) {
            return ev + " (" + hs.map(function(h) {
              return h.rule + ":" + h.prio;
            }).join(", ") + ")";
          }
          return ev + "," + hs.map(function(h) {
            return h.prio;
          }).join("/");
        });
      }
      var effects = ruleTrafficEffects(ix, vs);
      var vipMap = vipDestMap(ix);
      var steps = [];
      var l7label = l7.map(function(p) {
        return p.name;
      }).join("~");
      steps.push({ t: "vs", l: vs.name, oid: vsOid });
      l4.forEach(function(p) {
        steps.push({ t: "prof", l: p.name, oid: p.oid });
      });
      clientSSL.forEach(function(p) {
        steps.push({ t: "ssl", l: p.name, oid: p.oid });
      });
      if (l7label) steps.push({ t: "prof", l: l7label });
      eventsIn("client").forEach(function(e) {
        steps.push({ t: "event", l: e });
      });
      var proxyIdx = steps.length;
      steps.push({ t: "proxy", l: "Proxy \xB7 client \u21C4 server" });
      eventsIn("lb").forEach(function(e) {
        steps.push({ t: "event", l: e });
      });
      if (vs.pool) steps.push({ t: "pool", l: vs.pool.split("/").pop(), oid: "pool:" + vs.pool });
      eventsIn("server").forEach(function(e) {
        steps.push({ t: "event", l: e });
      });
      if (l7label) steps.push({ t: "prof", l: l7label });
      serverSSL.forEach(function(p) {
        steps.push({ t: "ssl", l: p.name, oid: p.oid });
      });
      l4.forEach(function(p) {
        steps.push({ t: "prof", l: p.name, oid: p.oid });
      });
      var pool = vs.pool ? findByPath(ix.d.pools, vs.pool) : null;
      var members = pool && pool.members || [];
      var poolV2v = [];
      members.forEach(function(m) {
        var tv = vipMap[m.address + ":" + m.port];
        if (tv && tv.fullPath !== vs.fullPath) poolV2v.push(tv);
      });
      var nn = poolNodeNames(ix, vs.pool);
      if (nn.length) steps.push({ t: "node", l: nn.join("\n") + (poolV2v.length ? "\n(\u2192 virtual)" : "") });
      else if (vs.pool) steps.push({ t: "node", l: "(no members)" });
      else steps.push({ t: "node", l: "(no pool / iRule-selected)" });
      var nodeIdx = steps.length - 1;
      var branches = [];
      effects.v2v.forEach(function(x) {
        branches.push({ t: "v2v", l: "virtual: " + x.target.split("/").pop(), from: proxyIdx });
      });
      effects.hsl.forEach(function(x) {
        branches.push({ t: "hsl", l: "HSL log" + (x.publisher ? ": " + x.publisher.split("/").pop() : ""), from: proxyIdx });
      });
      if (effects.sideband) branches.push({ t: "sband", l: "sideband (connect)", from: proxyIdx });
      poolV2v.forEach(function(tv) {
        branches.push({ t: "v2v", l: "virtual: " + tv.name, from: nodeIdx >= 0 ? nodeIdx : proxyIdx });
      });
      var enodes = [], eedges = [];
      steps.forEach(function(s, i) {
        enodes.push({ id: "P" + i, label: s.l, cls: s.t });
        if (i > 0) eedges.push({ from: "P" + (i - 1), to: "P" + i });
      });
      branches.forEach(function(b, j) {
        enodes.push({ id: "B" + j, label: b.l, cls: b.t });
        eedges.push({ from: "P" + b.from, to: "B" + j, dashed: true });
      });
      return { nodes: enodes, edges: eedges, steps };
    }
    function renderInto(host, model, ix, onNodeClick) {
      if (!window.ElkGraph) {
        host.textContent = "diagram engine unavailable";
        return;
      }
      window.ElkGraph.render(host, model, { dir: model && model.dir || "RIGHT", svgClass: "elk-report" }).then(function() {
        var svg = host.querySelector("svg");
        if (svg) svg.style.maxHeight = "70vh";
        wire(host, ix, onNodeClick);
      }).catch(function(err) {
        host.innerHTML = '<div class="diag-err">diagram error: ' + esc(err && err.message || err) + "</div>";
      });
    }
    function wire(host, ix, onNodeClick) {
      host.querySelectorAll(".elk-node[data-nid]").forEach(function(el) {
        var oid = ix.unshort[el.getAttribute("data-nid")];
        if (!oid) return;
        el.classList.add("elk-clk");
        el.addEventListener("click", function(ev) {
          ev.stopPropagation();
          clearHl(host);
          if (onNodeClick) onNodeClick(oid);
        });
      });
      host.querySelectorAll("path.elk-edge[data-from][data-to]").forEach(function(el) {
        var a = el.getAttribute("data-from"), b = el.getAttribute("data-to");
        if (!ix.unshort[a] || !ix.unshort[b]) return;
        el.style.cursor = "pointer";
        el.addEventListener("click", function(ev) {
          ev.stopPropagation();
          highlightComponent(host, ix, a, b);
        });
      });
      host.addEventListener("click", function() {
        clearHl(host);
      });
    }
    function clearHl(host) {
      host.querySelectorAll(".elk-hl,.elk-dim").forEach(function(el) {
        el.classList.remove("elk-hl", "elk-dim");
      });
    }
    function highlightComponent(host, ix, sidA, sidB) {
      var oidA = ix.unshort[sidA], oidB = ix.unshort[sidB];
      var comp = neighborhood(ix, [oidA, oidB], Infinity);
      var compSids = {};
      Object.keys(comp).forEach(function(o) {
        compSids[ix.short[o]] = true;
      });
      clearHl(host);
      host.querySelectorAll(".elk-node[data-nid]").forEach(function(el) {
        el.classList.add(compSids[el.getAttribute("data-nid")] ? "elk-hl" : "elk-dim");
      });
      host.querySelectorAll("path.elk-edge").forEach(function(el) {
        var a = el.getAttribute("data-from"), b = el.getAttribute("data-to");
        el.classList.add(a && b && compSids[a] && compSids[b] ? "elk-hl" : "elk-dim");
      });
    }
    function activateDevice2(di) {
      var tab = document.querySelector('.dev-tab[data-dev="' + di + '"]');
      if (tab) tab.click();
      var dev = document.querySelector('.device[data-dev="' + di + '"]');
      if (dev) dev.scrollIntoView({ behavior: "smooth", block: "start" });
    }
    function initArchitecture() {
      var arch = MODEL.architecture;
      if (!arch || !arch.graph || (arch.deviceCount || 0) < 2) return;
      var host = document.getElementById("archDiagram");
      if (!host) return;
      if (!window.ElkGraph) {
        host.style.display = "none";
        return;
      }
      var model;
      try {
        model = JSON.parse(arch.graph);
      } catch (e) {
        host.style.display = "none";
        return;
      }
      window.ElkGraph.render(host, model, { dir: "RIGHT", svgClass: "elk-report" }).then(function() {
        var svg = host.querySelector("svg");
        if (svg) svg.style.maxHeight = "60vh";
        host.querySelectorAll(".elk-node[data-nid]").forEach(function(el) {
          var m = /^d(\d+)$/.exec(el.getAttribute("data-nid") || "");
          if (!m) return;
          var di = parseInt(m[1], 10);
          el.classList.add("elk-clk");
          el.setAttribute("title", "Jump to this device");
          el.addEventListener("click", function(ev) {
            ev.stopPropagation();
            activateDevice2(di);
          });
        });
      }).catch(function(err) {
        host.style.display = "none";
        if (window.console) console.warn("architecture diagram error:", err);
      });
    }
    function initTopology(deviceEl, ix) {
      var panel = deviceEl.querySelector('.panel[data-panel="topology"]');
      if (!panel) return;
      var host = panel.querySelector(".diag-host");
      var focusSel = panel.querySelector(".topo-focus");
      var depthSel = panel.querySelector(".topo-depth");
      var typeBoxes = panel.querySelectorAll(".topo-type");
      focusSel.textContent = "";
      focusSel.appendChild(optionEl("", "\u2014 whole estate \u2014", false));
      ix.d.graph.nodes.slice().sort(function(a, b) {
        return (a.type + a.name).localeCompare(b.type + b.name);
      }).forEach(function(n) {
        focusSel.appendChild(optionEl(n.oid, TYPE_LABEL[n.type] + ": " + n.name, false));
      });
      function activeTypes() {
        var t = {};
        typeBoxes.forEach(function(b) {
          if (b.checked) t[b.value] = true;
        });
        return t;
      }
      function draw() {
        var types = activeTypes();
        var focus = focusSel.value;
        var depth = parseInt(depthSel.value, 10);
        var oids;
        if (focus) {
          oids = Object.keys(neighborhood(ix, [focus], depth));
        } else {
          oids = ix.d.graph.nodes.map(function(n) {
            return n.oid;
          });
        }
        oids = oids.filter(function(o) {
          return types[ix.byOid[o].type] || o === focus;
        });
        if (!oids.length) {
          host.innerHTML = '<div class="diag-empty">Nothing to show \u2014 enable object types.</div>';
          return;
        }
        if (oids.length > 240) {
          host.innerHTML = '<div class="diag-empty">' + oids.length + " objects \u2014 pick a focus object or reduce depth to render the graph.</div>";
          return;
        }
        var def = buildFlowchart(ix, oids, { dir: focus ? "LR" : "TB" });
        renderInto(host, def, ix, function(oid) {
          openDrawer(ix, oid);
        });
      }
      focusSel.addEventListener("change", draw);
      depthSel.addEventListener("change", draw);
      typeBoxes.forEach(function(b) {
        b.addEventListener("change", draw);
      });
      panel._draw = draw;
    }
    function openDrawer(ix, oid) {
      var n = ix.byOid[oid];
      if (!n) return;
      showSelector(n);
      var drawer = document.getElementById("objDrawer");
      var body = drawer.querySelector(".drawer-body");
      var titleEl = drawer.querySelector(".drawer-title");
      var titleTag = document.createElement("span");
      titleTag.className = "tag " + n.type;
      titleTag.textContent = TYPE_LABEL[n.type];
      titleEl.textContent = "";
      titleEl.appendChild(titleTag);
      titleEl.appendChild(document.createTextNode(" " + n.name));
      drawer.querySelector(".drawer-sub").textContent = n.fullPath;
      if (TYPE_PANEL[n.type]) {
        titleEl.classList.add("drawer-title-link");
        titleEl.setAttribute("role", "link");
        titleEl.setAttribute("tabindex", "0");
        titleEl.title = "Open " + n.name + " on the " + TYPE_LABEL[n.type] + " tab";
        titleEl.onclick = function() {
          gotoObject(ix, oid);
        };
        titleEl.onkeydown = function(e) {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            gotoObject(ix, oid);
          }
        };
      } else {
        titleEl.classList.remove("drawer-title-link");
        titleEl.removeAttribute("role");
        titleEl.removeAttribute("tabindex");
        titleEl.title = "";
        titleEl.onclick = null;
        titleEl.onkeydown = null;
      }
      var parts = [];
      if (n.type === "vs") parts.push(vsDetail(ix, oid));
      else if (n.type === "pool") parts.push(poolDetail(ix, oid));
      else if (n.type === "rule") {
        var refs = ruleRefs(ix, oid);
        if (refs) parts.push("<h4>References</h4>" + refs);
      }
      parts.push('<h4>Neighbourhood</h4><div class="diag-host drawer-diag"></div>');
      if (n.type === "vs") parts.push('<h4>Processing flow</h4><div class="diag-host flow-diag"></div>');
      var src = sourceHtml(ix, n);
      if (src) parts.push("<h4>Source</h4>" + src);
      body.innerHTML = parts.join("");
      var oids = Object.keys(neighborhood(ix, [oid], 2));
      renderInto(
        body.querySelector(".drawer-diag"),
        buildFlowchart(ix, oids, { dir: "LR" }),
        ix,
        function(o) {
          openDrawer(ix, o);
        }
      );
      if (n.type === "vs") {
        renderInto(
          body.querySelector(".flow-diag"),
          buildFlow(ix, oid),
          ix,
          function(o) {
            openDrawer(ix, o);
          }
        );
      }
      body.querySelectorAll(".conf-link[data-goto]").forEach(function(a) {
        a.addEventListener("click", function(e) {
          e.preventDefault();
          var toOid = ix.byPath[a.getAttribute("data-goto")];
          if (toOid) openDrawer(ix, toOid);
        });
      });
      wireObjLinks(body, ix);
      drawer.classList.add("open");
      document.getElementById("drawerScrim").classList.add("open");
    }
    function findVirtual(ix, oid) {
      var fp = oid.split(":").slice(1).join(":");
      return ix.d.virtuals.find(function(v) {
        return v.fullPath === fp;
      });
    }
    function findPool(ix, oid) {
      var fp = oid.split(":").slice(1).join(":");
      return ix.d.pools.find(function(p) {
        return p.fullPath === fp;
      });
    }
    function vsDetail(ix, oid) {
      var v = findVirtual(ix, oid);
      if (!v) return "";
      var L = v.listener || {};
      var rows = [
        ["Destination", esc(L.address || "-") + (L.prefix != null && L.prefix < L.maxPrefix ? "/" + esc(L.prefix) : "") + (L.routeDomain ? " %" + esc(L.routeDomain) : "")],
        ["Port", esc(L.portRaw || "-")],
        ["Protocol", esc(L.protocol || "-")],
        ["Source", esc(L.source || "-")],
        ["VLANs", L.vlans && L.vlans.length ? esc(L.vlans.join(", ")) + (L.vlansDisabled ? " (disabled)" : L.vlansEnabled ? " (enabled)" : "") : "all"]
      ];
      var meta = '<table class="kv">' + rows.map(function(r) {
        return "<tr><th>" + r[0] + "</th><td>" + r[1] + "</td></tr>";
      }).join("") + "</table>";
      var snat = v.snatpool ? refChip(v.snatpool, "snat") : v.sourceXlate ? esc(v.sourceXlate) : '<span class="muted">\u2014</span>';
      var propRows = [
        ["Pool", refChip(v.pool, "pool")],
        ["iRules", refChips(v.rules, "rule")],
        ["Persistence", refChips(v.persist, "persist")],
        ["Policies", refChips(v.policies, "policy")],
        ["SNAT", snat]
      ];
      var props = '<table class="kv">' + propRows.map(function(r) {
        return "<tr><th>" + r[0] + "</th><td>" + r[1] + "</td></tr>";
      }).join("") + "</table>";
      var staticProfiles = refChips(v.profiles, "prof");
      var dyn = (v.dynamicProfiles || []).map(function(a) {
        return '<span class="tag amber" title="via iRule ' + esc(a.rule) + '">' + esc(a.effect) + (a.arg ? " " + esc(a.arg) : "") + " \xB7 " + esc(a.category) + "</span>";
      }).join("");
      var dynBlock = dyn ? '<h4>Dynamic (iRule-driven) changes</h4><div class="tagwrap">' + dyn + "</div>" : "";
      return "<h4>Listener</h4>" + meta + "<h4>Properties</h4>" + props + '<h4>Static profiles</h4><div class="tagwrap">' + staticProfiles + "</div>" + dynBlock;
    }
    function poolDetail(ix, oid) {
      var p = findPool(ix, oid);
      if (!p) return "";
      var rows = (p.members || []).map(function(m) {
        return "<tr><td class='mono'>" + esc(m.name.split("/").pop()) + "</td><td class='mono'>" + esc(m.address) + "</td><td class='mono'>" + esc(m.port) + "</td></tr>";
      }).join("") || "<tr><td colspan=3 class='muted'>no members</td></tr>";
      return "<h4>Members</h4><table class='grid mini'><thead><tr><th>Member</th><th>Address</th><th>Port</th></tr></thead><tbody>" + rows + "</tbody></table>";
    }
    function findRule(ix, oid) {
      var fp = oid.split(":").slice(1).join(":");
      return (ix.d.rules || []).find(function(r) {
        return r.fullPath === fp;
      });
    }
    function ruleRefs(ix, oid) {
      var r = findRule(ix, oid);
      if (!r) return "";
      var ro = r.referencedObjects;
      if (!ro) return "";
      var hasStatic = ro.static && ro.static.length;
      var hasDyn = ro.dynamic && ro.dynamic.length;
      var out = '<div class="irule-refs">';
      if (hasStatic) {
        out += '<div class="refs-static"><span class="refs-lbl">Referenced objects:</span> ';
        ro.static.forEach(function(o) {
          out += '<span class="ref-obj" data-oid="' + esc(o.oid) + '" title="' + esc(o.type + " " + o.fullPath) + '">' + esc(o.type + " " + o.name) + "</span> ";
        });
        out += "</div>";
      }
      if (!hasStatic && !hasDyn) {
        out += '<div class="refs-none muted">No pool / node / snatpool / data-group references (static or dynamic) in this iRule.</div>';
      }
      (ro.dynamic || []).forEach(function(g) {
        out += '<div class="refs-dyn"><div class="refs-ctx">Potentially referenced objects';
        if (g.attached) {
          out += " \u2014 resolved for ";
          (g.virtuals || []).forEach(function(vn) {
            out += '<span class="tag">' + esc(vn) + "</span>";
          });
          out += " in partition <code>/" + esc(g.partition) + "</code>";
        } else {
          out += " \u2014 this iRule is not attached to any virtual; filters resolved in <code>/" + esc(g.partition) + "</code>";
        }
        out += '</div><table class="refs-table"><thead><tr><th>Determined filter</th><th>Type</th><th>Matching objects (this partition)</th></tr></thead><tbody>';
        (g.filters || []).forEach(function(f) {
          out += '<tr><td><code class="attach-pat" title="' + esc("reconstructed from the iRule source: " + (f.raw || "")) + '">' + esc(f.glob || "") + "</code>";
          if (f.unconstrained) out += ' <span class="muted small">(any name)</span>';
          else if (f.exact) out += ' <span class="muted small">(exact)</span>';
          out += '</td><td class="mono small">' + esc(f.type || "") + "</td><td>";
          if (f.objects && f.objects.length) {
            f.objects.forEach(function(o) {
              out += '<span class="ref-obj" data-oid="' + esc(o.oid) + '" title="' + esc(o.fullPath) + '">' + esc(o.name) + "</span> ";
            });
          } else {
            out += '<span class="muted">' + (f.unconstrained ? "any " + esc(f.type || "") + " in scope" : "none defined match") + "</span>";
          }
          out += "</td></tr>";
        });
        out += "</tbody></table></div>";
      });
      out += "</div>";
      return out;
    }
    function buildFlow(ix, vsOid) {
      var v = findVirtual(ix, vsOid);
      if (!v) return { nodes: [{ id: "x", label: "No data", cls: "muted" }], edges: [], dir: "RIGHT" };
      var L = v.listener || {};
      var enodes = [], eedges = [], n = 0;
      function id() {
        return "F" + n++;
      }
      var client = id();
      enodes.push({ id: client, label: "Client\n" + (L.source || "any"), cls: "cl" });
      var lsnr = id();
      enodes.push({ id: lsnr, label: "Listener\n" + v.name + "\n" + ((L.address || "") + ":" + (L.portRaw || "")), cls: "vs" });
      eedges.push({ from: client, to: lsnr });
      var prev = lsnr;
      if (L.vlans && L.vlans.length) {
        var vl = id();
        enodes.push({ id: vl, label: "VLAN\n" + L.vlans.join(", "), cls: "vlan" });
        eedges.push({ from: client, to: vl, dashed: true });
        eedges.push({ from: vl, to: lsnr, dashed: true });
      }
      (v.profiles || []).forEach(function(p) {
        var pid = id();
        enodes.push({ id: pid, label: p.split("/").pop(), cls: "prof" });
        eedges.push({ from: prev, to: pid });
        prev = pid;
      });
      (v.dynamicProfiles || []).forEach(function(a) {
        var did = id();
        enodes.push({ id: did, label: "iRule: " + a.effect + (a.arg ? " " + a.arg : ""), cls: "dyn" });
        eedges.push({ from: prev, to: did, dashed: true });
        prev = did;
      });
      (v.rules || []).forEach(function(r) {
        var rid = id();
        enodes.push({ id: rid, label: "iRule\n" + r.split("/").pop(), cls: "rule" });
        eedges.push({ from: prev, to: rid });
        prev = rid;
      });
      if (v.pool) {
        var pl = id();
        enodes.push({ id: pl, label: "Pool\n" + v.pool.split("/").pop(), cls: "pool" });
        eedges.push({ from: prev, to: pl });
        var pool = findPool(ix, "pool:" + v.pool);
        (pool ? pool.members : []).slice(0, 12).forEach(function(m) {
          var mid = id();
          enodes.push({ id: mid, label: m.address + ":" + m.port, cls: "node" });
          eedges.push({ from: pl, to: mid });
        });
      } else {
        var np = id();
        enodes.push({ id: np, label: "no default pool\n(forwarding / policy)", cls: "muted" });
        eedges.push({ from: prev, to: np });
      }
      return { nodes: enodes, edges: eedges, dir: "RIGHT" };
    }
    function closeDrawer() {
      document.getElementById("objDrawer").classList.remove("open");
      document.getElementById("drawerScrim").classList.remove("open");
    }
    function gotoObject(ix, oid) {
      var n = ix.byOid[oid];
      if (!n) return false;
      var panelName = TYPE_PANEL[n.type];
      if (!panelName) return false;
      var di = IDX.indexOf(ix);
      var deviceEl = document.querySelector('.device[data-dev="' + di + '"]');
      if (!deviceEl) return false;
      closeDrawer();
      if (!deviceEl.classList.contains("active")) {
        var devTab = document.querySelector('.dev-tab[data-dev="' + di + '"]');
        if (devTab) devTab.click();
      }
      var tab = deviceEl.querySelector('.tab[data-panel="' + panelName + '"]');
      if (tab) tab.click();
      var panel = deviceEl.querySelector('.panel[data-panel="' + panelName + '"]');
      var link = panel && panel.querySelector('[data-oid="' + n.type + ":" + n.fullPath + '"]');
      var row = link && link.closest("tr");
      if (!row) return false;
      row.classList.remove("part-hidden");
      var detail = row.nextElementSibling;
      if (detail && detail.classList.contains("detail")) {
        detail.classList.remove("part-hidden");
        if (row.classList.contains("expandable")) {
          row.classList.add("open");
          detail.classList.add("open");
        }
      }
      row.scrollIntoView({ block: "center", behavior: "smooth" });
      row.classList.remove("row-flash");
      void row.offsetWidth;
      row.classList.add("row-flash");
      setTimeout(function() {
        row.classList.remove("row-flash");
      }, 1600);
      return true;
    }
    function ipVer(ip) {
      if (!ip) return null;
      if (ip.indexOf(":") >= 0) return 6;
      if (/^\d+\.\d+\.\d+\.\d+$/.test(ip)) return 4;
      return null;
    }
    function v6ToBig(ip) {
      ip = ip.split("%")[0];
      var halves = ip.split("::");
      if (halves.length > 2) return null;
      var head = halves[0] ? halves[0].split(":") : [];
      var groups;
      if (halves.length === 2) {
        var tail = halves[1] ? halves[1].split(":") : [];
        var mid = 8 - head.length - tail.length;
        if (mid < 0) return null;
        groups = head.concat(new Array(mid).fill("0"), tail);
      } else {
        groups = head;
        if (groups.length !== 8) return null;
      }
      var big = 0n;
      for (var i = 0; i < 8; i++) {
        var g = groups[i] || "0";
        if (!/^[0-9a-fA-F]{1,4}$/.test(g)) return null;
        big = (big << 16n) + BigInt(parseInt(g, 16));
      }
      return big;
    }
    function ipToBig(ip) {
      var v = ipVer(ip);
      if (v === 4) {
        var p = ip.split("%")[0].split(".").map(Number);
        if (p.some(function(x) {
          return isNaN(x) || x < 0 || x > 255;
        })) return null;
        return (BigInt(p[0]) << 24n) + (BigInt(p[1]) << 16n) + (BigInt(p[2]) << 8n) + BigInt(p[3]);
      }
      if (v === 6) return v6ToBig(ip);
      return null;
    }
    function inNet(ipBig, ver, netIp, prefix) {
      if (ipBig == null) return true;
      if (ipVer(netIp) !== ver) return false;
      var netBig = ipToBig(netIp);
      if (netBig == null) return true;
      var bits = ver === 6 ? 128 : 32;
      if (prefix <= 0) return true;
      if (prefix > bits) prefix = bits;
      var mask = (1n << BigInt(prefix)) - 1n << BigInt(bits - prefix);
      return (ipBig & mask) === (netBig & mask);
    }
    function matchListeners(ix, q) {
      var qVer = ipVer(q.dst) || ipVer(q.src.split("/")[0]);
      var dstBig = ipToBig(q.dst);
      var out = [];
      ix.d.virtuals.forEach(function(v) {
        var L = v.listener || {};
        if (v.disabled) return;
        var lVer = L.family === "IPv6" ? 6 : 4;
        if (qVer && !L.anyAddr && lVer !== qVer) return;
        if (q.rd !== "" && String(L.routeDomain) !== String(q.rd)) return;
        if (!L.anyAddr && q.dst && !inNet(dstBig, lVer, L.address, L.prefix)) return;
        if (q.port !== "" && L.port !== 0 && String(L.port) !== String(q.port)) return;
        if (q.proto && q.proto !== "any" && L.protocol && L.protocol !== "any" && L.protocol !== q.proto) return;
        if (q.vlan && L.vlans && L.vlans.length) {
          var on = L.vlans.indexOf(q.vlan) >= 0;
          if (L.vlansEnabled && !on) return;
          if (L.vlansDisabled && on) return;
        }
        if (q.src && q.src !== "0.0.0.0/0" && q.src !== "::/0" && L.source && L.source !== "0.0.0.0/0" && L.source !== "::/0") {
          var sp = q.src.split("/");
          var ls = L.source.split("/");
          var sver = ipVer(sp[0]);
          if (sver && ipVer(ls[0]) === sver && !inNet(ipToBig(sp[0]), sver, ls[0], parseInt(ls[1] || String(sver === 6 ? 128 : 32), 10))) return;
        }
        out.push(v);
      });
      out.sort(function(a, b) {
        var la = a.listener, lb = b.listener;
        return lb.prefix - la.prefix || (lb.port !== 0) - (la.port !== 0) || lb.sourcePrefix - la.sourcePrefix || (lb.vlansEnabled ? 1 : 0) - (la.vlansEnabled ? 1 : 0) || a.name.localeCompare(b.name);
      });
      return out;
    }
    function profileByPath(ix, path) {
      return ix.d.profiles.find(function(p) {
        return p.fullPath === path;
      });
    }
    function policyByPath(ix, path) {
      return ix.d.policies.find(function(p) {
        return p.fullPath === path;
      });
    }
    function strOp(field, op, val, ci) {
      if (ci) {
        field = field.toLowerCase();
        val = val.toLowerCase();
      }
      switch (op) {
        case "starts-with":
          return field.indexOf(val) === 0;
        case "ends-with":
          return field.length >= val.length && field.slice(-val.length) === val;
        case "contains":
          return field.indexOf(val) >= 0;
        case "equals":
          return field === val;
        default:
          return field.indexOf(val) >= 0;
      }
    }
    function evalPolicy(pol, req) {
      var fired = [];
      for (var i = 0; i < pol.rules.length; i++) {
        var r = pol.rules[i];
        if (!r.conditions.length) continue;
        var all = r.conditions.every(function(c) {
          var field = c.operand === "http-host" ? req.host : c.selector === "path" ? req.uri.split("?")[0] : req.uri;
          var hit = (c.values || []).some(function(val) {
            return strOp(field, c.operator, val, c.caseInsensitive);
          });
          return c.negate ? !hit : hit;
        });
        if (all) {
          fired.push(r);
          if (pol.strategy === "first-match" || !pol.strategy) break;
        }
      }
      return fired;
    }
    function evalIrule(rule, req) {
      var body = rule.body || "", acts = [];
      function cond(ctx2) {
        var m2 = /HTTP::(uri|host)\b[^\n]*?\b(starts_with|ends_with|contains|equals|eq)\s+"?([^"\n\]]+)"?/i.exec(ctx2);
        if (!m2) return { has: false, hit: true, text: "" };
        var field = m2[1].toLowerCase() === "host" ? req.host : req.uri;
        var op = { starts_with: "starts-with", ends_with: "ends-with", contains: "contains", equals: "equals", eq: "equals" }[m2[2].toLowerCase()];
        return { has: true, hit: strOp(field, op, m2[3].trim(), true), text: "HTTP::" + m2[1] + " " + m2[2] + ' "' + m2[3].trim() + '"' };
      }
      var re = /\bpool\s+(\/[^\s;}"]+|\w[\w.\-]*)/g, m;
      while (m = re.exec(body)) {
        var ctx = body.slice(Math.max(0, m.index - 140), m.index);
        var c = cond(ctx);
        acts.push({ kind: "pool", target: m[1], cond: c.text, active: c.hit });
      }
      var patterns = [
        [/HTTP::redirect\s+"?([^"\n]+)"?/ig, "redirect"],
        [/HTTP::respond\s+(\d+)/ig, "respond"],
        [/HTTP::header\s+(insert|replace|remove)\s+"?([\w-]+)"?(?:\s+"?([^"\n]+)"?)?/ig, "header"],
        [/persist\s+(\w+)/ig, "persist"],
        [/(SSL::disable|SSL::enable)(?:\s+(\w+))?/ig, "ssl"],
        [/\bnode\s+(\d[\d.:a-f]+)/ig, "node"]
      ];
      patterns.forEach(function(pp) {
        var r2 = pp[0], mm;
        while (mm = r2.exec(body)) {
          acts.push({ kind: pp[1], target: mm[1] || "", arg: mm[2] || "", val: mm[3] || "" });
        }
      });
      return acts;
    }
    function pickSslProfile(ix, v, sni) {
      var ssl = (v.profiles || []).map(function(p) {
        return profileByPath(ix, p);
      }).filter(function(p) {
        return p && /CLIENT_SSL/.test(p.type);
      });
      if (!ssl.length) return null;
      if (sni) {
        var byName = ssl.find(function(p) {
          return p.name.toLowerCase().indexOf(sni.toLowerCase()) >= 0;
        });
        if (byName) return byName;
      }
      return ssl[0];
    }
    function simulate(ix, v, req) {
      var stages = [];
      var ssl = pickSslProfile(ix, v, req.sni);
      if (ssl) {
        stages.push({
          title: "Client SSL",
          cls: "ssl",
          rows: [
            ["Profile", ssl.name],
            ["Certificate", ssl.cert || "\u2014"],
            ["Key", ssl.key || "\u2014"],
            ["Chain", ssl.chain || "\u2014"],
            ["Ciphers", ssl.ciphers || "(profile default)"],
            ["SNI", req.sni ? req.sni : "(none supplied \u2014 default profile)"]
          ]
        });
      }
      var profRows = (v.profiles || []).map(function(p) {
        var po = profileByPath(ix, p);
        return [p.split("/").pop(), po ? po.type.replace(/_/g, " ").toLowerCase() : "system default"];
      });
      stages.push({ title: "Profiles applied (HUD)", cls: "hud", rows: profRows.length ? profRows : [["(system defaults only)", ""]] });
      var headers = req.headers.slice();
      function setHeader(k, val) {
        var i = headers.findIndex(function(h) {
          return h[0].toLowerCase() === k.toLowerCase();
        });
        if (i >= 0) headers[i] = [k, val];
        else headers.push([k, val]);
      }
      var changes = [];
      var selectedPool = v.pool || "";
      var poolReason = v.pool ? "default pool" : "";
      (v.rules || []).forEach(function(rp) {
        var rule = ix.d.rules.find(function(r) {
          return r.fullPath === rp || r.name === rp.split("/").pop();
        });
        if (!rule) return;
        var acts = evalIrule(rule, req);
        acts.forEach(function(a) {
          if (a.kind === "pool" && a.active) {
            selectedPool = a.target.indexOf("/") === 0 ? a.target : "/Common/" + a.target;
            poolReason = "iRule " + rule.name + (a.cond ? " (" + a.cond + ")" : "");
          } else if (a.kind === "header") {
            if (a.target === "remove") {
              headers = headers.filter(function(h) {
                return h[0].toLowerCase() !== a.arg.toLowerCase();
              });
              changes.push("iRule " + rule.name + ": remove header " + a.arg);
            } else {
              setHeader(a.arg, a.val);
              changes.push("iRule " + rule.name + ": " + a.target + " " + a.arg + ": " + a.val);
            }
          } else if (a.kind === "redirect") {
            changes.push("iRule " + rule.name + ": HTTP::redirect " + a.target);
          } else if (a.kind === "respond") {
            changes.push("iRule " + rule.name + ": HTTP::respond " + a.target);
          } else if (a.kind === "persist") {
            changes.push("iRule " + rule.name + ": persist " + a.target);
          } else if (a.kind === "ssl") {
            changes.push("iRule " + rule.name + ": " + a.target + (a.arg ? " " + a.arg : ""));
          } else if (a.kind === "node") {
            selectedPool = "(node " + a.target + ")";
            poolReason = "iRule " + rule.name + " node override";
          }
        });
      });
      var policyRows = [];
      (v.policies || []).forEach(function(pp) {
        var pol = policyByPath(ix, pp);
        if (!pol) return;
        var fired = evalPolicy(pol, req);
        fired.forEach(function(r) {
          r.actions.forEach(function(a) {
            if (a.target === "forward" && a.pool) {
              selectedPool = a.pool;
              poolReason = "policy " + pol.name + " \xB7 rule " + r.name;
            }
            if (/http-header|http-uri|http-host/.test(a.target) || a.target === "http-header") {
              if (a.name) {
                setHeader(a.name, a.value);
                changes.push("policy " + pol.name + ": set " + a.name + ": " + a.value);
              }
            }
            policyRows.push([pol.name + " \xB7 " + r.name, a.verb + " " + a.target + (a.pool ? " \u2192 " + a.pool.split("/").pop() : a.location ? " " + a.location : "")]);
          });
        });
        if (!fired.length && pol.rules.length) policyRows.push([pol.name, "no rule matched this request"]);
      });
      if (policyRows.length) stages.push({ title: "LTM policy", cls: "policy", rows: policyRows });
      if (changes.length || headers.length) {
        stages.push({
          title: "Request after processing",
          cls: "req",
          pre: (req.method || "GET") + " " + (req.uri || "/") + " HTTP/1.1\n" + headers.map(function(h) {
            return h[0] + ": " + h[1];
          }).join("\n"),
          notes: changes
        });
      }
      var pool = selectedPool.indexOf("(") === 0 ? null : findPool(ix, "pool:" + selectedPool);
      var lbRows = [["Selected pool", selectedPool ? selectedPool.split("/").pop() : "none (dropped / forwarded)"], ["Reason", poolReason || "\u2014"]];
      if (pool) {
        lbRows.push(["LB method", pool.lbMode || "round-robin"]);
        var up = pool.members.filter(function(m) {
          return m.state !== "down" && m.state !== "user-down";
        });
        var pick = up[0] || pool.members[0];
        if (pick) lbRows.push(["Illustrative member", pick.address + ":" + pick.port + (pick.ratio ? " ratio " + pick.ratio : "") + (pick.priorityGroup ? " pg " + pick.priorityGroup : "")]);
      }
      stages.push({
        title: "Load balancing & member selection",
        cls: "lb",
        rows: lbRows,
        members: pool ? pool.members : null
      });
      var sssl = (v.profiles || []).map(function(p) {
        return profileByPath(ix, p);
      }).filter(function(p) {
        return p && /SERVER_SSL/.test(p.type);
      })[0];
      if (sssl) stages.push({ title: "Server SSL", cls: "ssl", rows: [["Profile", sssl.name], ["Certificate", sssl.cert || "\u2014"]] });
      return stages;
    }
    function renderSim(ix, v, host) {
      var L = v.listener || {};
      var hasSsl = !!pickSslProfile(ix, v, "");
      var h = ['<div class="sim-head"><b>Processing for ' + esc(v.name) + '</b> <span class="mono muted">' + esc(L.address) + ":" + esc(L.portRaw) + " \xB7 " + esc(L.protocol) + "</span></div>"];
      h.push('<div class="sim-req">');
      if (hasSsl) h.push('<div class="fld"><label>TLS SNI</label><input class="sim-sni lm-field" placeholder="www.example.com"></div>');
      h.push('<div class="fld"><label>Method</label><select class="sim-method"><option>GET</option><option>POST</option><option>PUT</option><option>DELETE</option></select></div>');
      h.push('<div class="fld grow"><label>URI</label><input class="sim-uri lm-field" placeholder="/path?query" value="/"></div>');
      h.push('<div class="fld"><label>Host</label><input class="sim-host lm-field" placeholder="app.example.com"></div>');
      h.push('<button class="lm-run sim-go">Simulate \u25B8</button>');
      h.push("</div>");
      h.push('<div class="fld"><label>Extra request headers (one per line, <code>Name: value</code>)</label><textarea class="sim-headers" rows="2" placeholder="X-Custom: 1"></textarea></div>');
      h.push('<div class="sim-out"></div>');
      host.innerHTML = h.join("");
      function go() {
        var headers = [];
        var host_ = host.querySelector(".sim-host").value.trim();
        if (host_) headers.push(["Host", host_]);
        (host.querySelector(".sim-headers").value || "").split("\n").forEach(function(ln) {
          var i = ln.indexOf(":");
          if (i > 0) headers.push([ln.slice(0, i).trim(), ln.slice(i + 1).trim()]);
        });
        var req = {
          sni: hasSsl ? host.querySelector(".sim-sni").value.trim() : "",
          method: host.querySelector(".sim-method").value,
          uri: host.querySelector(".sim-uri").value.trim() || "/",
          host: host_,
          headers
        };
        var stages = simulate(ix, v, req);
        var o = host.querySelector(".sim-out");
        o.innerHTML = stages.map(function(s) {
          var inner = "";
          if (s.rows) inner += '<table class="kv">' + s.rows.map(function(r) {
            return "<tr><th>" + esc(r[0]) + "</th><td class='mono'>" + esc(r[1]) + "</td></tr>";
          }).join("") + "</table>";
          if (s.pre) inner += "<pre class='code'>" + esc(s.pre) + "</pre>";
          if (s.notes && s.notes.length) inner += "<ul class='sim-notes'>" + s.notes.map(function(n) {
            return "<li>" + esc(n) + "</li>";
          }).join("") + "</ul>";
          if (s.members) inner += "<table class='grid mini'><thead><tr><th>Member</th><th>Addr</th><th>Port</th><th>Ratio</th><th>PG</th></tr></thead><tbody>" + s.members.map(function(m) {
            return "<tr><td class='mono'>" + esc(m.name.split("/").pop()) + "</td><td class='mono'>" + esc(m.address) + "</td><td class='mono'>" + esc(m.port) + "</td><td class='mono'>" + esc(m.ratio || "1") + "</td><td class='mono'>" + esc(m.priorityGroup || "0") + "</td></tr>";
          }).join("") + "</tbody></table>";
          return '<div class="sim-stage ' + s.cls + '"><div class="sim-stage-t">' + esc(s.title) + "</div>" + inner + "</div>";
        }).join("");
      }
      host.querySelector(".sim-go").addEventListener("click", go);
      host.querySelectorAll(".sim-req .lm-field").forEach(function(el) {
        el.addEventListener("keydown", function(e) {
          if (e.key === "Enter") go();
        });
      });
      go();
    }
    function initListener(deviceEl, ix) {
      var panel = deviceEl.querySelector('.panel[data-panel="listener"]');
      if (!panel) return;
      var f = {
        src: panel.querySelector(".lm-src"),
        dst: panel.querySelector(".lm-dst"),
        port: panel.querySelector(".lm-port"),
        proto: panel.querySelector(".lm-proto"),
        vlan: panel.querySelector(".lm-vlan"),
        rd: panel.querySelector(".lm-rd")
      };
      var out = panel.querySelector(".lm-results");
      var sim = panel.querySelector(".lm-sim");
      var vlans = {};
      ix.d.virtuals.forEach(function(v) {
        (v.listener.vlans || []).forEach(function(x) {
          vlans[x] = 1;
        });
      });
      f.vlan.innerHTML = '<option value="">any VLAN</option>' + Object.keys(vlans).sort().map(function(x) {
        return "<option>" + esc(x) + "</option>";
      }).join("");
      function selectVs(v) {
        showSelector({ type: "vs", fullPath: v.fullPath });
        var L = v.listener;
        f.dst.value = L.anyAddr ? L.family === "IPv6" ? "::" : "0.0.0.0" : L.address;
        f.port.value = L.port ? String(L.port) : "";
        f.proto.value = L.protocol && L.protocol !== "any" ? L.protocol : "any";
        f.rd.value = String(L.routeDomain || 0);
        f.vlan.value = L.vlansEnabled && L.vlans.length ? L.vlans[0] : "";
        run(v);
      }
      function run(focusVs) {
        var q = {
          src: f.src.value.trim() || "0.0.0.0/0",
          dst: f.dst.value.trim(),
          port: f.port.value.trim(),
          proto: f.proto.value,
          vlan: f.vlan.value,
          rd: f.rd.value.trim()
        };
        var matches = matchListeners(ix, q);
        sim.innerHTML = "";
        if (!matches.length) {
          out.innerHTML = '<div class="diag-empty">No virtual server matches this flow.</div>';
          return;
        }
        var tiers = [];
        matches.forEach(function(v) {
          var key = v.listener.prefix + ":" + (v.listener.port !== 0 ? 1 : 0) + ":" + v.listener.sourcePrefix;
          var t = tiers.find(function(t2) {
            return t2.key === key;
          });
          if (!t) {
            t = { key, prefix: v.listener.prefix, port: v.listener.port, vs: [] };
            tiers.push(t);
          }
          t.vs.push(v);
        });
        var focus = focusVs || matches[0];
        var html = ['<div class="lm-note">The most specific listener is at the top \u2014 that is where this flow lands. Listeners of equal specificity are shown side by side; lower tiers only receive traffic the tiers above do not. Click a listener to load the exact flow that reaches it and simulate its processing.</div>'];
        html.push('<div class="tiers">');
        tiers.forEach(function(t, ti) {
          html.push('<div class="tier' + (ti === 0 ? " match" : "") + '">');
          html.push('<div class="tier-key">/' + esc(t.prefix) + " \xB7 " + (t.port ? "port " + esc(t.port) : "any port") + "</div>");
          html.push('<div class="tier-vs">');
          t.vs.forEach(function(v) {
            var L = v.listener;
            html.push('<button class="lm-card" data-fp="' + esc(v.fullPath) + '"' + (v === focus ? ' data-focus="1"' : "") + '><span class="lm-name">' + esc(v.name) + (v === focus ? ' <span class="tag green">match</span>' : "") + '</span><span class="lm-dest mono">' + esc(L.address) + (L.prefix < L.maxPrefix ? "/" + esc(L.prefix) : "") + (L.routeDomain ? "%" + esc(L.routeDomain) : "") + ":" + esc(L.portRaw) + '</span><span class="lm-meta mono">' + esc(L.protocol) + (v.pool ? " \u2192 " + esc(v.pool.split("/").pop()) : " \xB7 no pool") + "</span></button>");
          });
          html.push("</div></div>");
          if (ti < tiers.length - 1) html.push('<div class="tier-arrow">falls through to \u25BE</div>');
        });
        html.push("</div>");
        out.innerHTML = html.join("");
        out.querySelectorAll(".lm-card").forEach(function(b) {
          b.addEventListener("click", function() {
            var v = ix.d.virtuals.find(function(x) {
              return x.fullPath === b.dataset.fp;
            });
            if (v) selectVs(v);
          });
        });
        renderSim(ix, focus, sim);
      }
      panel.querySelector(".lm-run").addEventListener("click", function() {
        run();
      });
      panel.querySelectorAll(".lm-field").forEach(function(el) {
        el.addEventListener("keydown", function(e) {
          if (e.key === "Enter") run();
        });
      });
      run();
    }
    function wireObjLinks(root, ix) {
      root.querySelectorAll("[data-oid],[data-oref]").forEach(function(el) {
        if (el._wired) return;
        el._wired = true;
        var oid = el.dataset.oid || (el.dataset.oref ? ix.byPath[el.dataset.oref] : "");
        if (!oid || !ix.byOid[oid]) return;
        el.classList.add("objlink");
        el.addEventListener("click", function(ev) {
          ev.preventDefault();
          ev.stopPropagation();
          openDrawer(ix, oid);
        });
      });
    }
    function initObjectIndex(deviceEl, ix) {
      var panel = deviceEl.querySelector('.panel[data-panel="objectIndex"]');
      if (!panel) return;
      var view = panel.querySelector(".objindex-view");
      var filter = panel.querySelector(".objindex-filter");
      var groups = {};
      Object.keys(ix.byOid).forEach(function(oid) {
        var n = ix.byOid[oid];
        if (!TYPE_PANEL[n.type]) return;
        (groups[n.type] = groups[n.type] || []).push(n);
      });
      var order = ["vs", "pool", "node", "mon", "rule", "dg", "prof"];
      var html = "";
      order.forEach(function(t) {
        var list = groups[t];
        if (!list || !list.length) return;
        list.sort(function(a, b) {
          return a.fullPath < b.fullPath ? -1 : a.fullPath > b.fullPath ? 1 : 0;
        });
        html += '<section class="objindex-group"><h3 class="objindex-h"><span class="tag ' + t + '">' + TYPE_LABEL[t] + '</span> <span class="objindex-count">' + list.length + '</span></h3><ul class="objindex-list">';
        list.forEach(function(n) {
          var key = (n.name + " " + n.fullPath).toLowerCase();
          html += '<li class="objindex-item" data-oid="' + esc(n.oid) + '" data-search="' + esc(key) + '"><span class="objindex-name">' + esc(n.name) + '</span> <span class="objindex-path mono">' + esc(n.fullPath) + "</span></li>";
        });
        html += "</ul></section>";
      });
      view.innerHTML = html || '<div class="diag-empty">No indexable objects.</div>';
      view.querySelectorAll(".objindex-item[data-oid]").forEach(function(li) {
        li.addEventListener("click", function() {
          gotoObject(ix, li.getAttribute("data-oid"));
        });
      });
      if (filter) {
        filter.addEventListener("input", function() {
          var q = filter.value.trim().toLowerCase();
          view.querySelectorAll(".objindex-item").forEach(function(li) {
            var show = !q || li.getAttribute("data-search").indexOf(q) !== -1;
            li.classList.toggle("oi-hidden", !show);
          });
          view.querySelectorAll(".objindex-group").forEach(function(g) {
            g.classList.toggle("oi-hidden", !g.querySelector(".objindex-item:not(.oi-hidden)"));
          });
        });
      }
    }
    document.querySelectorAll(".device").forEach(function(deviceEl) {
      var ix = IDX[parseInt(deviceEl.dataset.dev, 10)];
      initTopology(deviceEl, ix);
      initListener(deviceEl, ix);
      initObjectIndex(deviceEl, ix);
      wireObjLinks(deviceEl, ix);
      deviceEl.querySelectorAll('.tab[data-panel="topology"]').forEach(function(tab) {
        tab.addEventListener("click", function() {
          var panel = deviceEl.querySelector('.panel[data-panel="topology"]');
          if (panel && panel._draw && !panel._drawn) {
            panel._drawn = true;
            panel._draw();
          }
        });
      });
    });
    initArchitecture();
    try {
      initArchEditor();
    } catch (e) {
      if (window.console) console.warn("arch editor:", e);
    }
    var closeBtn = document.querySelector("#objDrawer .drawer-close");
    if (closeBtn) closeBtn.addEventListener("click", closeDrawer);
    var scrim = document.getElementById("drawerScrim");
    if (scrim) scrim.addEventListener("click", closeDrawer);
    document.addEventListener("keydown", function(e) {
      if (e.key === "Escape") closeDrawer();
    });
    (function initSearch() {
      var search = document.getElementById("globalSearch");
      if (!search) return;
      function toNet(s) {
        var m = /^([0-9a-fA-F:.]+)(?:\/(\d+))?$/.exec(s);
        if (!m) return null;
        var ver = ipVer(m[1]);
        if (!ver) return null;
        var big = ipToBig(m[1]);
        if (big == null) return null;
        var full = ver === 6 ? 128 : 32;
        var pfx = m[2] !== void 0 ? parseInt(m[2], 10) : full;
        if (pfx > full) return null;
        return { ver, big, prefix: pfx, full };
      }
      function contains(outer, inner) {
        if (outer.ver !== inner.ver || inner.prefix < outer.prefix) return false;
        var mask = (1n << BigInt(outer.prefix)) - 1n << BigInt(outer.full - outer.prefix);
        return (outer.big & mask) === (inner.big & mask);
      }
      function ipOverlap(a, b) {
        return contains(a, b) || contains(b, a);
      }
      var IP_TOKEN = /[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+(?:\/[0-9]+)?|[0-9a-fA-F:]{2,}:[0-9a-fA-F:]*(?:\/[0-9]+)?/g;
      function parseIpQuery(raw) {
        var s = raw.trim();
        if (!s) return null;
        var rd = null, port = null;
        var pm = /^(.*?):(\d+)$/.exec(s);
        if (pm && pm[1].indexOf(".") >= 0) {
          s = pm[1];
          port = pm[2];
        }
        var rm = /^([^%]*)%(\d+)(.*)$/.exec(s);
        if (rm) {
          rd = parseInt(rm[2], 10);
          s = rm[1] + rm[3];
        }
        var net = toNet(s);
        if (!net) return null;
        return { net, rd, port, base: s.split("/")[0] };
      }
      function ipMatcher(raw) {
        var qParsed = parseIpQuery(raw);
        if (!qParsed) return null;
        var qNet = qParsed.net;
        return function(hay) {
          var toks = hay.match(IP_TOKEN);
          if (!toks) return false;
          for (var i = 0; i < toks.length; i++) {
            var tn = toNet(toks[i]);
            if (tn && tn.prefix > 0 && ipOverlap(qNet, tn)) return true;
          }
          return false;
        };
      }
      var archDevs = MODEL.architecture && MODEL.architecture.devices || [];
      var deviceMeta = MODEL.devices.map(function(d, i) {
        var ad = archDevs[i] || {};
        return {
          name: ad.name || d.name || "device " + i,
          tier: typeof ad.tier === "number" ? ad.tier : null
        };
      });
      initGlobalSearch({ input: search, deviceMeta, ipMatcher });
    })();
    (function initSystemToggle() {
      MODEL.devices.forEach(function(d, di) {
        var deviceEl = document.querySelector('.device[data-dev="' + di + '"]') || document.querySelectorAll(".device")[di];
        if (!deviceEl) return;
        var defaults = {};
        [
          "virtuals",
          "pools",
          "nodes",
          "monitors",
          "rules",
          "dataGroups",
          "profiles",
          "policies",
          "snatpools",
          "persistence",
          "certificates"
        ].forEach(function(k) {
          (d[k] || []).forEach(function(o) {
            if (o && o.isDefault && o.fullPath) defaults[o.fullPath] = true;
          });
        });
        deviceEl.querySelectorAll(".grid tbody tr.searchable").forEach(function(row) {
          var m = (row.getAttribute("data-search") || "").match(/(\/[^\s]+)/);
          if (m && defaults[m[1]]) {
            row.classList.add("sys-row");
            var det = row.nextElementSibling;
            if (det && det.classList.contains("detail")) det.classList.add("sys-row");
          }
        });
        var chk = deviceEl.querySelector(".show-system");
        if (chk) chk.addEventListener("change", function() {
          deviceEl.classList.toggle("show-system-on", chk.checked);
        });
      });
    })();
    (function initApps() {
      var LS_PREFIX = "f5report:apps:";
      function deviceKey(d) {
        return String(d.name || d.uri || "device").replace(/\s+/g, "_");
      }
      function loadManual(d) {
        try {
          return JSON.parse(localStorage.getItem(LS_PREFIX + deviceKey(d)) || "[]") || [];
        } catch (e) {
          return [];
        }
      }
      function storeManual(d, apps) {
        try {
          localStorage.setItem(LS_PREFIX + deviceKey(d), JSON.stringify(apps));
          return true;
        } catch (e) {
          return false;
        }
      }
      function loadAuto(d) {
        return (d.apps || []).map(function(a) {
          return {
            name: a.name,
            oids: (a.entryPoints || []).map(function(fp) {
              return "vs:" + fp;
            }),
            auto: true,
            source: a.source,
            // "iapp" | "folder"
            folder: a.folder,
            memberCount: a.memberCount
          };
        });
      }
      function loadApps(d) {
        var manual = loadManual(d).map(function(a, mi) {
          return { name: a.name, oids: a.oids || [], auto: false, source: "manual", manualIndex: mi };
        });
        return loadAuto(d).concat(manual);
      }
      function blockId(oid) {
        return "appobj:" + oid;
      }
      function scrollToBlock(host, oid) {
        var el = host.querySelector('[id="' + (window.CSS && CSS.escape ? CSS.escape(blockId(oid)) : blockId(oid)) + '"]');
        if (!el) {
          var all = host.querySelectorAll(".app-obj");
          for (var k = 0; k < all.length; k++) {
            if (all[k].getAttribute("data-oid") === oid) {
              el = all[k];
              break;
            }
          }
        }
        if (el) {
          el.scrollIntoView({ behavior: "smooth", block: "start" });
          el.classList.add("app-obj-flash");
          setTimeout(function() {
            el.classList.remove("app-obj-flash");
          }, 1200);
        }
      }
      function appScope(ix, startOids) {
        var seen = {}, queue = [];
        startOids.forEach(function(o) {
          if (ix.byOid[o]) {
            seen[o] = true;
            queue.push(o);
          }
        });
        while (queue.length) {
          var cur = queue.shift();
          if (isDefaultNode(ix, cur)) continue;
          (ix.fadj[cur] || []).forEach(function(nb) {
            if (!seen[nb]) {
              seen[nb] = true;
              queue.push(nb);
            }
          });
        }
        return seen;
      }
      function renderApp(deviceEl, ix, app, host) {
        var vsOids = (app.oids || []).filter(function(o) {
          return ix.byOid[o];
        });
        var scope = appScope(ix, vsOids);
        var oids = Object.keys(scope);
        var inApp = {};
        oids.forEach(function(o) {
          var n = ix.byOid[o];
          if (n) inApp[n.fullPath] = true;
        });
        var missing = (app.oids || []).length - vsOids.length;
        var html = '<div class="app-detail">';
        html += '<div class="app-detail-head"><h3>' + esc(app.name) + '</h3><span class="muted">' + vsOids.length + " virtual server(s), " + oids.length + " objects" + (missing ? ", " + missing + " not in this device" : "") + "</span></div>";
        html += '<div class="app-vs-chips">';
        vsOids.forEach(function(o) {
          html += '<button class="app-chip" data-goto-oid="' + o + '">' + esc(ix.byOid[o].name) + "</button>";
        });
        html += "</div>";
        html += '<div class="app-flow"><div class="app-flow-title">Traffic flow</div>';
        vsOids.forEach(function(o) {
          html += '<div class="app-pipe"><div class="app-pipe-vs">' + esc(ix.byOid[o].name) + '</div><div class="app-pipe-diagram diag-host" data-vs="' + o + '">building\u2026</div></div>';
        });
        html += "</div>";
        html += '<div class="app-objs">';
        var order = { vs: 0, pool: 1, node: 2, mon: 3, rule: 4, prof: 5, persist: 6, policy: 7, snat: 8, dg: 9 };
        oids.sort(function(a, b) {
          var na = ix.byOid[a], nb = ix.byOid[b];
          return (order[na.type] || 9) - (order[nb.type] || 9) || na.name.localeCompare(nb.name);
        });
        oids.forEach(function(o) {
          var n = ix.byOid[o];
          html += '<div class="app-obj" id="' + blockId(o) + '" data-oid="' + o + '">';
          html += '<div class="app-obj-head"><span class="app-obj-type">' + esc(TYPE_LABEL[n.type] || n.type) + '</span> <span class="app-obj-name mono">' + esc(n.fullPath) + '</span><button class="app-obj-drawer" data-oid="' + o + '" title="Open in inspector">\u2922</button></div>';
          if (n.type === "rule") {
            var r = ruleByPath(ix, n.fullPath);
            if (r) {
              if (r.flowchart) html += '<div class="app-obj-flow diag-host" data-flow="' + esc(encodeURIComponent(r.flowchart)) + '">flow\u2026</div>';
              html += '<pre class="code tcl">' + (r.bodyHtml ? sanitiseHtml(r.bodyHtml) : esc(r.body || "")) + "</pre>";
            }
          } else {
            var stanza = stanzaFor(ix.d.configText, n.fullPath);
            html += '<pre class="code conf">' + (stanza ? highlightConf(stanza, function(fp) {
              return !!inApp[fp];
            }) : '<span class="muted">config not found</span>') + "</pre>";
          }
          html += "</div>";
        });
        html += "</div></div>";
        host.innerHTML = html;
        host.querySelectorAll(".app-pipe-diagram[data-vs]").forEach(function(ph) {
          var built = buildTrafficPipeline(ix, ph.getAttribute("data-vs"));
          if (!built || !window.ElkGraph) {
            ph.textContent = "(pipeline unavailable)";
            return;
          }
          window.ElkGraph.render(ph, built, { dir: "RIGHT", svgClass: "elk-report" }).catch(function(e) {
            ph.innerHTML = '<div class="diag-err">pipeline error: ' + esc(e && e.message || e) + "</div>";
          });
        });
        host.querySelectorAll(".app-obj-flow[data-flow]").forEach(function(fh) {
          var raw = decodeURIComponent(fh.getAttribute("data-flow") || "");
          if (!raw || !window.ElkGraph) {
            fh.textContent = "";
            return;
          }
          var model;
          try {
            model = JSON.parse(raw);
          } catch (e) {
            fh.textContent = "";
            return;
          }
          window.ElkGraph.render(fh, model, { dir: "DOWN", svgClass: "elk-report" }).catch(function() {
            fh.textContent = "(flowchart unavailable)";
          });
        });
        host.querySelectorAll(".conf-link[data-goto]").forEach(function(a) {
          a.addEventListener("click", function(e) {
            e.preventDefault();
            var fp = a.getAttribute("data-goto");
            var oid = ix.byPath[fp];
            if (oid) scrollToBlock(host, oid);
          });
        });
        host.querySelectorAll(".app-chip[data-goto-oid]").forEach(function(b) {
          b.addEventListener("click", function() {
            scrollToBlock(host, b.getAttribute("data-goto-oid"));
          });
        });
        host.querySelectorAll(".app-obj-drawer[data-oid]").forEach(function(b) {
          b.addEventListener("click", function() {
            openDrawer(ix, b.getAttribute("data-oid"));
          });
        });
      }
      function renderAppsPanel(deviceEl, ix) {
        var view = deviceEl.querySelector('.panel[data-panel="apps"] .apps-view');
        if (!view) return;
        var apps = loadApps(ix.d);
        var badge = deviceEl.querySelector(".app-badge");
        if (badge) badge.textContent = String(apps.length);
        var html = '<div class="apps-list">';
        if (!apps.length) {
          html += '<p class="muted">No applications detected. Apps are auto-grouped from BIG-IP folders (objects under <code>/partition/<em>app-folder</em>/\u2026</code>, including iApp <code>.app</code> folders). Objects that sit directly in a partition root are not grouped. You can also build one by hand on the Virtual Servers tab \u2014 tick the virtual servers, name it and hit <strong>Save app</strong> (stored in this browser).</p>';
        } else {
          apps.forEach(function(app, i) {
            var tag = app.auto ? '<span class="app-tag app-tag-' + (app.source === "iapp" ? "iapp" : "folder") + '" title="' + esc(app.folder || "") + '">' + (app.source === "iapp" ? "iApp" : "folder") + "</span>" : '<span class="app-tag app-tag-manual">manual</span>';
            html += '<div class="app-card"><button class="app-open" data-i="' + i + '">' + esc(app.name) + "</button>" + tag + '<span class="muted">' + (app.oids || []).length + " VS" + (app.memberCount ? " \xB7 " + app.memberCount + " objs" : "") + "</span>" + (app.auto ? "" : '<button class="app-del" data-mi="' + app.manualIndex + '" title="Delete app">\u2715</button>') + "</div>";
          });
        }
        html += '</div><div class="apps-detail"></div>';
        view.innerHTML = html;
        view.querySelectorAll(".app-open").forEach(function(b) {
          b.addEventListener("click", function() {
            view.querySelectorAll(".app-open").forEach(function(x) {
              x.classList.remove("active");
            });
            b.classList.add("active");
            renderApp(deviceEl, ix, apps[parseInt(b.dataset.i, 10)], view.querySelector(".apps-detail"));
          });
        });
        view.querySelectorAll(".app-del").forEach(function(b) {
          b.addEventListener("click", function() {
            var manual = loadManual(ix.d);
            manual.splice(parseInt(b.dataset.mi, 10), 1);
            storeManual(ix.d, manual);
            renderAppsPanel(deviceEl, ix);
          });
        });
      }
      MODEL.devices.forEach(function(d, di) {
        var deviceEl = document.querySelector('.device[data-dev="' + di + '"]') || document.querySelectorAll(".device")[di];
        if (!deviceEl) return;
        var ix = IDX[di];
        var vpanel = deviceEl.querySelector('.panel[data-panel="virtuals"]');
        var picks = function() {
          return vpanel ? vpanel.querySelectorAll(".app-pick") : [];
        };
        var countEl = deviceEl.querySelector(".app-count");
        function refreshCount() {
          var n = 0;
          picks().forEach(function(c) {
            if (c.checked) n++;
          });
          if (countEl) countEl.textContent = n + " selected";
        }
        picks().forEach(function(c) {
          c.addEventListener("change", refreshCount);
        });
        var all = deviceEl.querySelector(".app-pick-all");
        if (all) all.addEventListener("change", function() {
          picks().forEach(function(c) {
            var row = c.closest("tr");
            if (!row || row.classList.contains("part-hidden") || row.classList.contains("hidden")) return;
            c.checked = all.checked;
          });
          refreshCount();
        });
        var saveBtn = deviceEl.querySelector(".app-save");
        var nameInp = deviceEl.querySelector(".app-name");
        if (saveBtn) saveBtn.addEventListener("click", function() {
          var oids = [];
          picks().forEach(function(c) {
            if (c.checked) oids.push(c.dataset.vs);
          });
          var name = nameInp && nameInp.value.trim() || "";
          if (!oids.length) {
            if (nameInp) nameInp.placeholder = "tick some virtual servers first";
            return;
          }
          if (!name) {
            if (nameInp) {
              nameInp.focus();
              nameInp.placeholder = "name the app first";
            }
            return;
          }
          var manual = loadManual(ix.d);
          var existing = manual.filter(function(a) {
            return a.name === name;
          })[0];
          if (existing) existing.oids = oids;
          else manual.push({ name, oids });
          storeManual(ix.d, manual);
          if (nameInp) nameInp.value = "";
          picks().forEach(function(c) {
            c.checked = false;
          });
          refreshCount();
          renderAppsPanel(deviceEl, ix);
          var tab = deviceEl.querySelector('.tab[data-panel="apps"]');
          if (tab) tab.click();
        });
        renderAppsPanel(deviceEl, ix);
      });
    })();
  })();
})();
