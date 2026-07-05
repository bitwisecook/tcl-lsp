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

// f5report — interactive topology / flow / listener explorer.
// Renders the object graph with Mermaid, maps the rendered SVG back to the
// model, and drives click-to-focus, connected-component highlighting, the
// per-virtual flow diagram, and the listener-matching table. No dependencies
// beyond the vendored Mermaid (already loaded), no network.
(function () {
  "use strict";

  var MODEL = null;
  try {
    MODEL = JSON.parse(document.getElementById("f5-model").textContent);
  } catch (e) { return; }

  if (window.mermaid) {
    // ELK (layered) layout for the Mermaid-drawn diagrams (topology, the
    // per-virtual traffic pipeline / flow, and the iRule flowcharts). ELK gives
    // a clean layered node placement; the edges use a smooth curve because
    // Mermaid does not expose ELK's orthogonal edge *routing* — a `step` curve
    // fakes right angles but overlaps parallel edges and mis-seats arrowheads.
    // The genuinely orthogonal diagram (the APM walk) is drawn by the dedicated
    // elkjs renderer (elk-graph.js), which routes edges through ELK itself.
    // The ELK layout emits the same SVG structure as the default layout
    // (`.node` / `flowchart-N…` ids, `path.flowchart-link` / `LS-`/`LE-`
    // classes), so the click-to-focus and edge-highlight wiring is unaffected.
    mermaid.initialize({
      startOnLoad: false, securityLevel: "loose", theme: "neutral", layout: "elk",
      flowchart: { htmlLabels: true, curve: "basis", nodeSpacing: 40, rankSpacing: 55 },
    });
  }

  var TYPE_CLASS = {
    vs: "vs", pool: "pool", node: "node", mon: "mon", rule: "rule",
    prof: "prof", persist: "persist", policy: "policy", snat: "snat", dg: "dg",
  };
  var TYPE_LABEL = {
    vs: "Virtual", pool: "Pool", node: "Node", mon: "Monitor", rule: "iRule",
    prof: "Profile", persist: "Persistence", policy: "Policy", snat: "SNAT Pool", dg: "Data Group",
  };
  // f5-query container path for each node type, for the selector status bar.
  var CONTAINER = {
    vs: ".ltm.virtual", pool: ".ltm.pool", node: ".ltm.node", mon: ".ltm.monitor",
    rule: ".ltm.rule", prof: ".ltm.profile", persist: ".ltm.persistence",
    policy: ".ltm.policy", snat: ".ltm.snatpool", dg: '.ltm."data-group"',
  };

  // ---- floating f5-query selector status bar -----------------------------
  function selectorFor(node) {
    var c = CONTAINER[node.type] || ".ltm";
    return c + '[] | select(."full-path" == "' + node.fullPath + '")';
  }
  function showSelector(node) {
    var bar = document.getElementById("f5qbar");
    if (!bar || !node) return;
    var expr = selectorFor(node);
    setBarExpr(bar, (TYPE_LABEL[node.type] || node.type), expr);
    bar.classList.add("show");
    // remember the active device + object so the path builder can default to it
    var dEl = document.querySelector(".device.active");
    bar.dataset.dev = dEl ? dEl.dataset.dev : "0";
    bar.dataset.oid = node.oid || "";
  }
  function setBarExpr(bar, typeLabel, expr) {
    bar.querySelector(".f5qbar-type").textContent = typeLabel;
    bar.querySelector(".f5qbar-expr").textContent = expr;
    bar.dataset.expr = expr;
  }

  // ---- source → destination path query builder ---------------------------
  // Shortest directed (source → target) path over the reference graph.
  function forwardPath(ix, srcOid, dstOid) {
    if (srcOid === dstOid) return [srcOid];
    var prev = {}, seen = {}; seen[srcOid] = true;
    var queue = [srcOid];
    while (queue.length) {
      var cur = queue.shift();
      var nbrs = ix.fadj[cur] || [];
      for (var i = 0; i < nbrs.length; i++) {
        var nb = nbrs[i];
        if (seen[nb]) continue;
        seen[nb] = true; prev[nb] = cur;
        if (nb === dstOid) {
          var path = [nb]; var p = cur;
          while (p !== undefined) { path.unshift(p); p = prev[p]; }
          return path;
        }
        queue.push(nb);
      }
    }
    return null;
  }
  // every object forward-reachable from `srcOid` (for the destination picker)
  function reachableFrom(ix, srcOid) {
    var seen = {}, out = [], queue = [srcOid];
    while (queue.length) {
      var cur = queue.shift();
      (ix.fadj[cur] || []).forEach(function (nb) {
        if (!seen[nb]) { seen[nb] = true; out.push(nb); queue.push(nb); }
      });
    }
    return out;
  }
  // one DSL navigation step for a directed edge (source object already selected)
  function stepFor(ix, edge, targetOid) {
    var fp = ix.byOid[targetOid].fullPath;
    switch (edge.kind) {
      case "pool": return " | .pool";
      case "snat": return " | .snatpool";
      case "monitor": return " | .monitor";
      case "rule": return ' | .rules[] | select(. == "' + fp + '")';
      case "profile": return ' | .profiles[] | select(startswith("' + fp + '"))';
      case "persist": return ' | .persist[] | select(. == "' + fp + '")';
      case "policy": return ' | .policies[] | select(. == "' + fp + '")';
      case "pool-irule": return ' | .refs.pools[] | select(. == "' + fp + '")';
      case "datagroup": return ' | .refs."data-groups"[] | select(. == "' + fp + '")';
      case "member":
        var addr = ix.nodeAddr[fp];
        return addr
          ? ' | .members[] | select(.address == "' + addr + '")'
          : ' | .members[] | select(.name | startswith("' + fp + ':"))';
      default: return ' | refs[] | select(. == "' + fp + '")';
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
      return navigator.clipboard.writeText(t).catch(function () { fallbackCopy(t); });
    }
    fallbackCopy(t); return Promise.resolve();
  }
  function fallbackCopy(t) {
    var ta = document.createElement("textarea");
    ta.value = t; ta.style.position = "fixed"; ta.style.opacity = "0";
    document.body.appendChild(ta); ta.focus(); ta.select();
    try { document.execCommand("copy"); } catch (e) { /* ignore */ }
    document.body.removeChild(ta);
  }
  (function initBar() {
    var bar = document.getElementById("f5qbar");
    if (!bar) return;
    var btn = bar.querySelector(".f5qbar-copy");
    btn.addEventListener("click", function () {
      copyText(bar.dataset.expr || "").then(function () {
        var old = btn.textContent; btn.textContent = "✓"; btn.classList.add("ok");
        setTimeout(function () { btn.textContent = old; btn.classList.remove("ok"); }, 1100);
      });
    });
    // click the expression to select it for manual copy
    bar.querySelector(".f5qbar-expr").addEventListener("click", function () {
      var r = document.createRange(); r.selectNodeContents(this);
      var s = window.getSelection(); s.removeAllRanges(); s.addRange(r);
    });
    bar.querySelector(".f5qbar-close").addEventListener("click", function () {
      bar.classList.remove("show");
    });

    // ---- path-builder popover ----
    var pop = bar.querySelector(".f5qbar-pop");
    var srcSel = pop.querySelector(".pb-src");
    var dstSel = pop.querySelector(".pb-dst");
    var note = pop.querySelector(".pb-note");
    var pathBtn = bar.querySelector(".f5qbar-path");

    function ixForBar() { return IDX[parseInt(bar.dataset.dev || "0", 10)]; }

    function optLabel(ix, oid) {
      var n = ix.byOid[oid];
      return (TYPE_LABEL[n.type] || n.type) + ": " + n.name;
    }
    function fillSources(ix, selectedOid) {
      var nodes = ix.d.graph.nodes.slice().sort(function (a, b) {
        return (a.type + a.name).localeCompare(b.type + b.name);
      });
      srcSel.innerHTML = nodes.map(function (n) {
        return '<option value="' + n.oid + '"' + (n.oid === selectedOid ? " selected" : "") + ">" +
          esc(optLabel(ix, n.oid)) + "</option>";
      }).join("");
    }
    function fillDests(ix) {
      var reach = reachableFrom(ix, srcSel.value)
        .map(function (o) { return { oid: o, label: optLabel(ix, o) }; })
        .sort(function (a, b) { return a.label.localeCompare(b.label); });
      if (!reach.length) {
        dstSel.innerHTML = '<option value="">(nothing reachable downstream)</option>';
      } else {
        dstSel.innerHTML = reach.map(function (r) {
          return '<option value="' + r.oid + '">' + esc(r.label) + "</option>";
        }).join("");
      }
      build();
    }
    function build() {
      var ix = ixForBar();
      var s = srcSel.value, d = dstSel.value;
      if (!s || !d) { note.textContent = ""; return; }
      var path = forwardPath(ix, s, d);
      if (!path) { note.textContent = "No downstream path from source to destination."; return; }
      var q = pathQuery(ix, path);
      if (!q) { note.textContent = "Could not express this path as a query."; return; }
      var hops = path.map(function (o) { return ix.byOid[o].name; }).join("  →  ");
      note.textContent = hops;
      setBarExpr(bar, "path", q);
    }

    pathBtn.addEventListener("click", function () {
      var ix = ixForBar();
      var open = pop.classList.toggle("open");
      pathBtn.classList.toggle("active", open);
      if (open) {
        fillSources(ix, bar.dataset.oid || (ix.d.graph.nodes[0] || {}).oid);
        fillDests(ix);
      }
    });
    srcSel.addEventListener("change", function () { fillDests(ixForBar()); });
    dstSel.addEventListener("change", build);
    document.addEventListener("keydown", function (e) {
      if (e.key === "Escape") { pop.classList.remove("open"); pathBtn.classList.remove("active"); }
    });
  })();

  // ---- per-device index --------------------------------------------------
  function indexDevice(d) {
    var byOid = {}, adj = {}, fadj = {}, short = {}, unshort = {}, i = 0;
    d.graph.nodes.forEach(function (n) {
      byOid[n.oid] = n; adj[n.oid] = {}; fadj[n.oid] = [];
      var sid = "N" + (i++); short[n.oid] = sid; unshort[sid] = n.oid;
    });
    var edgesByPair = {}, edgeDir = {};
    d.graph.edges.forEach(function (e) {
      if (!(e.from in adj) || !(e.to in adj)) return;
      adj[e.from][e.to] = true; adj[e.to][e.from] = true;
      fadj[e.from].push(e.to);                 // directed source → target
      edgeDir[e.from + "->" + e.to] = e;        // directed edge lookup
      edgesByPair[short[e.from] + "|" + short[e.to]] = e;
      edgesByPair[short[e.to] + "|" + short[e.from]] = e;
    });
    // node full-path → address, for member-step generation
    var nodeAddr = {};
    (d.nodes || []).forEach(function (n) { nodeAddr[n.fullPath] = n.address; });
    // full-path → oid, so a *reference* (by path, type unknown) can open the
    // object it points at.
    var byPath = {};
    d.graph.nodes.forEach(function (n) { if (!(n.fullPath in byPath)) byPath[n.fullPath] = n.oid; });
    return { d: d, byOid: byOid, adj: adj, fadj: fadj, short: short, unshort: unshort,
      edgesByPair: edgesByPair, edgeDir: edgeDir, nodeAddr: nodeAddr, byPath: byPath };
  }
  var IDX = MODEL.devices.map(indexDevice);

  function activeDeviceIndex() {
    var el = document.querySelector(".device.active");
    return el ? parseInt(el.dataset.dev, 10) : 0;
  }

  // Escape HTML metacharacters — these strings are config-derived (object names,
  // iRule bodies, data-group records, descriptions) and land in innerHTML and in
  // Mermaid (html) labels, so `<`, `>`, `&`, `"` must all be neutralised.
  var ESC_MAP = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" };
  function esc(s) {
    return String(s).replace(/[&<>"]/g, function (c) { return ESC_MAP[c]; }).replace(/\n/g, " ");
  }
  // Edge labels sit in Mermaid's `|...|` syntax, which chokes on (){}[]|" —
  // strip those to keep the flowchart parseable.
  function escLbl(s) { return String(s).replace(/[()|{}\[\]"]/g, "").replace(/\n/g, " "); }

  // ---- shared config-source rendering ------------------------------------
  // Escape config text for a <pre> *without* collapsing newlines (unlike esc(),
  // which is for single-line Mermaid labels): a config stanza is multi-line, so
  // its line breaks must survive.
  function escConf(s) {
    return String(s).replace(/[&<>"]/g, function (c) { return ESC_MAP[c]; });
  }

  // Extract one object's raw bigip.conf stanza (brace-matched) from the device
  // config text. iRules are shown from the model's highlighted body instead.
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
      else if (c === "}") { depth--; if (depth === 0) { j++; return cfg.slice(m.index, j); } }
    }
    return cfg.slice(m.index);
  }

  // Escape a config stanza, then wrap every object path (linkable when
  // isLinked(path) is truthy) and each line's leading keyword for syntax
  // highlighting. isLinked: (fullPath) -> bool, may be omitted for no links.
  function highlightConf(text, isLinked) {
    var out = escConf(text);
    out = out.replace(/(\/[A-Za-z][\w.\-]*(?:\/[\w.\-]+)+)/g, function (m0) {
      return isLinked && isLinked(m0)
        ? '<a class="conf-path conf-link" data-goto="' + m0 + '">' + m0 + "</a>"
        : '<span class="conf-path">' + m0 + "</span>";
    });
    out = out.replace(/^(\s*)([a-z][\w-]+)/gm, '$1<span class="conf-key">$2</span>');
    return out;
  }

  function ruleByPath(ix, fp) {
    var rules = ix.d.rules || [];
    for (var i = 0; i < rules.length; i++) { if (rules[i].fullPath === fp) return rules[i]; }
    return null;
  }

  // The syntax-highlighted, cross-linked source for one object: the model's
  // highlighted Tcl body for iRules, otherwise the brace-matched bigip.conf
  // stanza. Object paths resolving to another object in this device become
  // links (the caller wires the clicks). Empty string when no source is found.
  function sourceHtml(ix, n) {
    if (n.type === "rule") {
      var r = ruleByPath(ix, n.fullPath);
      if (r && (r.bodyHtml || r.body)) {
        return '<pre class="code tcl">' + (r.bodyHtml || escConf(r.body || "")) + "</pre>";
      }
    }
    var stanza = stanzaFor(ix.d.configText, n.fullPath);
    if (!stanza) return "";
    var html = highlightConf(stanza, function (fp) {
      return fp !== n.fullPath && !!ix.byPath[fp];
    });
    return '<pre class="code conf">' + html + "</pre>";
  }

  // Render an object full-path as a chip that wireObjLinks() makes clickable
  // when the path resolves to an object in this device (otherwise it stays
  // plain text). `extra` adds tag colour class(es).
  function refChip(p, extra) {
    if (!p) return '<span class="muted">—</span>';
    return '<span class="tag ' + (extra || "") + '" data-oref="' + esc(p) + '" title="' +
      esc(p) + '">' + esc(p.split("/").pop()) + "</span>";
  }
  function refChips(paths, extra) {
    if (!paths || !paths.length) return '<span class="muted">none</span>';
    return paths.map(function (p) { return refChip(p, extra); }).join(" ");
  }

  // BFS the undirected connected component containing `startOids`, bounded by
  // `depth` (Infinity = whole component).
  function isDefaultNode(ix, oid) { var n = ix.byOid[oid]; return !!(n && n.isDefault); }

  function neighborhood(ix, startOids, depth) {
    var seen = {}, frontier = [], d = 0;
    startOids.forEach(function (o) { if (ix.byOid[o]) { seen[o] = true; frontier.push(o); } });
    while (frontier.length && d < depth) {
      var next = [];
      frontier.forEach(function (o) {
        // Built-in/system objects (a shared default profile, `_sys_*` rule, …)
        // are terminal: shown where directly linked, never traversed *through*
        // — otherwise a default profile bridges one app's graph into another's.
        if (isDefaultNode(ix, o)) return;
        Object.keys(ix.adj[o] || {}).forEach(function (nb) {
          if (!seen[nb]) { seen[nb] = true; next.push(nb); }
        });
      });
      frontier = next; d++;
    }
    return seen; // set of oids
  }

  // ---- Mermaid text builder ---------------------------------------------
  function buildFlowchart(ix, oids, opts) {
    opts = opts || {};
    var dir = opts.dir || "LR";
    var lines = ["flowchart " + dir];
    var nodeSet = {};
    oids.forEach(function (o) { nodeSet[o] = true; });
    // nodes
    Object.keys(nodeSet).forEach(function (o) {
      var n = ix.byOid[o]; if (!n) return;
      var sid = ix.short[o];
      var label = esc(n.name);
      var cls = TYPE_CLASS[n.type] || "default";
      var shape = n.type === "vs" ? ['(["', '"])'] : n.type === "node" ? ['[("', '")]']
        : n.type === "rule" ? ['{{"', '"}}'] : ['["', '"]'];
      lines.push("  " + sid + shape[0] + label + shape[1] + ":::" + cls +
        (n.orphan ? " " : ""));
    });
    // edges (only those with both endpoints in the set)
    ix.d.graph.edges.forEach(function (e) {
      if (!nodeSet[e.from] || !nodeSet[e.to]) return;
      var a = ix.short[e.from], b = ix.short[e.to];
      var lbl = e.label ? ("|" + escLbl(e.label) + "|") : "";
      var arrow = e.kind === "pool-irule" ? " -.->" : " -->";
      lines.push("  " + a + arrow + lbl + " " + b);
    });
    // class styling
    lines.push("classDef vs fill:#dbeafe,stroke:#2563eb,color:#0b2b5e;");
    lines.push("classDef pool fill:#dcfce7,stroke:#16a34a,color:#064e2b;");
    lines.push("classDef node fill:#f1f5f9,stroke:#64748b,color:#1e293b;");
    lines.push("classDef mon fill:#fef9c3,stroke:#ca8a04,color:#4a3608;");
    lines.push("classDef rule fill:#ede9fe,stroke:#7c3aed,color:#3b1e75;");
    lines.push("classDef prof fill:#e0f2fe,stroke:#0284c7,color:#053345;");
    lines.push("classDef persist fill:#ffe4e6,stroke:#e11d48,color:#5c0a1e;");
    lines.push("classDef policy fill:#fae8ff,stroke:#c026d3,color:#4a0d52;");
    lines.push("classDef snat fill:#f5f5f4,stroke:#78716c,color:#292524;");
    lines.push("classDef dg fill:#ecfeff,stroke:#0891b2,color:#083344;");
    return lines.join("\n");
  }

  // ---- BIG-IP traffic-processing pipeline -----------------------------------
  // A virtual server processes traffic down the client-side profile stack
  // (L4 → SSL → L7), through the client-side iRule events (in firing order, with
  // priority), to the LB / forwarding decision, then back up the server-side
  // stack to the pool member. This renders that ordered pipeline (not the raw
  // reference graph), e.g. vs → tcp → clientssl → http → HTTP_REQUEST,500 →
  // proxy → LB_SELECTED → http → serverssl → tcp → node.
  var PROFILE_LAYER = {
    TCP: { layer: 1 }, UDP: { layer: 1 }, SCTP: { layer: 1 }, FASTL4: { layer: 1 },
    CLIENT_SSL: { layer: 2, side: "client" }, SERVER_SSL: { layer: 2, side: "server" },
    HTTP: { layer: 3 }, HTTP2: { layer: 3 }, ONE_CONNECT: { layer: 3 }, WEBSOCKET: { layer: 3 },
    REWRITE: { layer: 3 }, HTML: { layer: 3 }, FTP: { layer: 3 }, DNS: { layer: 3 },
    SIP: { layer: 3 }, DIAMETER: { layer: 3 }, RADIUS: { layer: 3 }, FIX: { layer: 3 },
    MQTT: { layer: 3 }, STREAM: { layer: 3 }, FASTHTTP: { layer: 3 }
  };
  var EVENT_PHASE = (function () {
    var o = {};
    [["CLIENT_ACCEPTED", "client"], ["CLIENTSSL_CLIENTHELLO", "client"],
      ["CLIENTSSL_CLIENTCERT", "client"], ["CLIENTSSL_HANDSHAKE", "client"],
      ["CLIENTSSL_SERVERHELLO_SEND", "client"], ["HTTP_REQUEST", "client"],
      ["HTTP_REQUEST_DATA", "client"], ["HTTP_PROXY_REQUEST", "client"],
      ["STREAM_MATCHED", "client"], ["CACHE_REQUEST", "client"], ["CLIENT_DATA", "client"],
      ["LB_SELECTED", "lb"], ["LB_FAILED", "lb"], ["PERSIST_DOWN", "lb"],
      ["SERVER_CONNECTED", "server"], ["SERVERSSL_HANDSHAKE", "server"],
      ["SERVERSSL_CLIENTCERT", "server"], ["SERVER_DATA", "server"],
      ["HTTP_RESPONSE", "server"], ["HTTP_RESPONSE_CONTINUE", "server"],
      ["HTTP_RESPONSE_DATA", "server"], ["CACHE_RESPONSE", "server"],
      ["HTTP_RESPONSE_RELEASE", "server"]
    ].forEach(function (e, i) { o[e[0]] = { phase: e[1], order: i }; });
    return o;
  })();
  function eventPhase(ev) { return EVENT_PHASE[ev] || { phase: "client", order: 900 }; }

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
      return pool.members.map(function (m) { return m.name || m.address || ""; }).filter(Boolean);
    }
    var out = [];
    (ix.fadj["pool:" + poolFp] || []).forEach(function (nb) {
      var n = ix.byOid[nb]; if (n && n.type === "node") out.push(n.name);
    });
    return out;
  }

  // Build the ordered traffic pipeline (Mermaid) for one virtual server.
  function buildTrafficPipeline(ix, vsOid) {
    var vs = findByPath(ix.d.virtuals, vsOid.replace(/^vs:/, ""));
    if (!vs) return null;
    var byLayer = { 1: [], 2: [], 3: [] };
    (vs.profiles || []).forEach(function (fp) {
      var prof = findByPath(ix.d.profiles, fp);
      var type = prof ? prof.type : "";
      var info = PROFILE_LAYER[type] || { layer: 3 };
      byLayer[info.layer].push({ name: prof ? prof.name : fp.split("/").pop(), side: info.side, oid: "prof:" + fp });
    });
    var l4 = byLayer[1], l7 = byLayer[3];
    var clientSSL = byLayer[2].filter(function (p) { return p.side !== "server"; });
    var serverSSL = byLayer[2].filter(function (p) { return p.side === "server"; });

    // events (with priority + rule) from the attached iRules
    var eventMap = {};
    (vs.rules || []).forEach(function (rp) {
      var rule = findByPath(ix.d.rules, rp);
      if (!rule || !rule.body) return;
      var re = /when\s+([A-Z][A-Z0-9_]*)((?:\s+priority\s+\d+)?)/g, m;
      while ((m = re.exec(rule.body))) {
        var pm = /priority\s+(\d+)/.exec(m[2]);
        (eventMap[m[1]] = eventMap[m[1]] || []).push({ rule: rule.name, prio: pm ? parseInt(pm[1], 10) : 500 });
      }
    });
    function eventsIn(phase) {
      return Object.keys(eventMap).filter(function (ev) { return eventPhase(ev).phase === phase; })
        .sort(function (a, b) { return eventPhase(a).order - eventPhase(b).order; })
        .map(function (ev) {
          var hs = eventMap[ev].slice().sort(function (x, y) { return x.prio - y.prio; });
          var rules = {}; hs.forEach(function (h) { rules[h.rule] = 1; });
          if (Object.keys(rules).length > 1) {
            return ev + " (" + hs.map(function (h) { return h.rule + ":" + h.prio; }).join(", ") + ")";
          }
          return ev + "," + hs.map(function (h) { return h.prio; }).join("/");
        });
    }

    var steps = [];
    var l7label = l7.map(function (p) { return p.name; }).join("~");
    steps.push({ t: "vs", l: vs.name, oid: vsOid });
    l4.forEach(function (p) { steps.push({ t: "prof", l: p.name, oid: p.oid }); });
    clientSSL.forEach(function (p) { steps.push({ t: "ssl", l: p.name, oid: p.oid }); });
    if (l7label) steps.push({ t: "prof", l: l7label });
    eventsIn("client").forEach(function (e) { steps.push({ t: "event", l: e }); });
    steps.push({ t: "proxy", l: "proxy" });
    eventsIn("lb").forEach(function (e) { steps.push({ t: "event", l: e }); });
    if (vs.pool) steps.push({ t: "pool", l: vs.pool.split("/").pop(), oid: "pool:" + vs.pool });
    eventsIn("server").forEach(function (e) { steps.push({ t: "event", l: e }); });
    if (l7label) steps.push({ t: "prof", l: l7label });
    serverSSL.forEach(function (p) { steps.push({ t: "ssl", l: p.name, oid: p.oid }); });
    l4.forEach(function (p) { steps.push({ t: "prof", l: p.name, oid: p.oid }); });
    var nn = poolNodeNames(ix, vs.pool);
    if (nn.length) steps.push({ t: "node", l: nn.join("\\n") });
    else if (vs.pool) steps.push({ t: "node", l: "(no members)" });
    else steps.push({ t: "node", l: "(no pool / iRule-selected)" });

    var lines = ["flowchart LR"];
    steps.forEach(function (s, i) {
      var sid = "P" + i;
      var shape = s.t === "vs" ? ['(["', '"])'] : s.t === "node" ? ['[("', '")]']
        : s.t === "event" ? ['{{"', '"}}'] : s.t === "proxy" ? ['[/"', '"/]'] : ['["', '"]'];
      lines.push("  " + sid + shape[0] + escLbl(s.l) + shape[1] + ":::" + s.t);
      if (i > 0) lines.push("  P" + (i - 1) + " --> " + sid);
    });
    lines.push("classDef vs fill:#dbeafe,stroke:#2563eb,color:#0b2b5e;");
    lines.push("classDef prof fill:#e0f2fe,stroke:#0284c7,color:#053345;");
    lines.push("classDef ssl fill:#fef9c3,stroke:#ca8a04,color:#4a3608;");
    lines.push("classDef event fill:#ede9fe,stroke:#7c3aed,color:#3b1e75;");
    lines.push("classDef proxy fill:#f1f5f9,stroke:#64748b,color:#1e293b;");
    lines.push("classDef pool fill:#dcfce7,stroke:#16a34a,color:#064e2b;");
    lines.push("classDef node fill:#f5f5f4,stroke:#78716c,color:#292524;");
    return { def: lines.join("\n"), steps: steps };
  }

  // Render `def` into `host`, then wire node/edge interactions.
  var _rid = 0;
  function renderInto(host, def, ix, onNodeClick) {
    if (!window.mermaid) { host.textContent = "Mermaid unavailable"; return; }
    var id = "mmd" + (_rid++);
    mermaid.render(id, def).then(function (res) {
      host.innerHTML = res.svg;
      var svg = host.querySelector("svg");
      if (svg) {
        // Keep Mermaid's intrinsic size (so a small graph stays small) but cap
        // it to the container / a sensible height.
        svg.style.maxWidth = "100%";
        svg.style.maxHeight = "70vh";
        svg.style.height = "auto";
      }
      wire(host, ix, onNodeClick);
    }).catch(function (err) {
      host.innerHTML = '<div class="diag-err">diagram error: ' + esc(err.message || err) + "</div>";
    });
  }

  function nodeSid(el) { var m = /flowchart-(N\d+)-/.exec(el.id || ""); return m ? m[1] : null; }
  function edgeEnds(el) {
    var cls = el.getAttribute("class") || "";
    var s = /LS-(N\d+)/.exec(cls), e = /LE-(N\d+)/.exec(cls);
    return s && e ? [s[1], e[1]] : null;
  }

  function wire(host, ix, onNodeClick) {
    // nodes: click to focus
    host.querySelectorAll(".node").forEach(function (el) {
      var sid = nodeSid(el); if (!sid) return;
      el.classList.add("mm-node"); el.dataset.sid = sid;
      el.addEventListener("click", function (ev) {
        ev.stopPropagation();
        clearHl(host);
        if (onNodeClick) onNodeClick(ix.unshort[sid]);
      });
    });
    // edges: click to light up the whole connected component
    host.querySelectorAll("path.flowchart-link").forEach(function (el) {
      var ends = edgeEnds(el); if (!ends) return;
      el.classList.add("mm-edge");
      el.style.cursor = "pointer";
      el.addEventListener("click", function (ev) {
        ev.stopPropagation();
        highlightComponent(host, ix, ends[0], ends[1]);
      });
    });
    // click empty space clears highlight
    host.addEventListener("click", function () { clearHl(host); });
  }

  function clearHl(host) {
    host.querySelectorAll(".mm-hl,.mm-dim").forEach(function (el) {
      el.classList.remove("mm-hl", "mm-dim");
    });
  }

  // Light up every node/edge reachable (undirected) from either endpoint.
  function highlightComponent(host, ix, sidA, sidB) {
    var oidA = ix.unshort[sidA], oidB = ix.unshort[sidB];
    var comp = neighborhood(ix, [oidA, oidB], Infinity); // whole component
    var compSids = {};
    Object.keys(comp).forEach(function (o) { compSids[ix.short[o]] = true; });
    clearHl(host);
    host.querySelectorAll(".node").forEach(function (el) {
      var sid = nodeSid(el); if (!sid) return;
      el.classList.add(compSids[sid] ? "mm-hl" : "mm-dim");
    });
    host.querySelectorAll("path.flowchart-link").forEach(function (el) {
      var ends = edgeEnds(el); if (!ends) return;
      var on = compSids[ends[0]] && compSids[ends[1]];
      el.classList.add(on ? "mm-hl" : "mm-dim");
    });
    host.querySelectorAll(".edgeLabel").forEach(function (el) { /* labels stay */ });
  }

  // ---- Cross-device architecture diagram ---------------------------------
  // The top-level Architecture section renders the tier graph (one node per
  // loaded device) interactively: click a device node to jump to its section.
  function activateDevice(di) {
    var tab = document.querySelector('.dev-tab[data-dev="' + di + '"]');
    if (tab) tab.click();
    var dev = document.querySelector('.device[data-dev="' + di + '"]');
    if (dev) dev.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  function initArchitecture() {
    var arch = MODEL.architecture;
    if (!arch || !arch.mermaid || (arch.deviceCount || 0) < 2) return;
    var host = document.getElementById("archDiagram");
    if (!host) return;
    if (!window.mermaid) { host.style.display = "none"; return; }
    var id = "archmmd" + (_rid++);
    mermaid.render(id, arch.mermaid).then(function (res) {
      host.innerHTML = res.svg;
      var svg = host.querySelector("svg");
      if (svg) { svg.style.maxWidth = "100%"; svg.style.maxHeight = "60vh"; svg.style.height = "auto"; }
      // Each device node's element id is `flowchart-d<index>-<seq>`.
      host.querySelectorAll(".node").forEach(function (el) {
        var m = /flowchart-d(\d+)-/.exec(el.id || "");
        if (!m) return;
        var di = parseInt(m[1], 10);
        el.classList.add("mm-node");
        el.style.cursor = "pointer";
        el.setAttribute("title", "Jump to this device");
        el.addEventListener("click", function (ev) {
          ev.stopPropagation();
          activateDevice(di);
        });
      });
    }).catch(function (err) {
      // Fall back to the static tier columns already in the section.
      host.style.display = "none";
      if (window.console) console.warn("architecture diagram error:", err);
    });
  }

  // ---- Topology tab ------------------------------------------------------
  function initTopology(deviceEl, ix) {
    var panel = deviceEl.querySelector('.panel[data-panel="topology"]');
    if (!panel) return;
    var host = panel.querySelector(".diag-host");
    var focusSel = panel.querySelector(".topo-focus");
    var depthSel = panel.querySelector(".topo-depth");
    var typeBoxes = panel.querySelectorAll(".topo-type");

    // populate focus selector
    var opts = ['<option value="">— whole estate —</option>'];
    ix.d.graph.nodes.slice().sort(function (a, b) {
      return (a.type + a.name).localeCompare(b.type + b.name);
    }).forEach(function (n) {
      opts.push('<option value="' + n.oid + '">' + TYPE_LABEL[n.type] + ": " + esc(n.name) + "</option>");
    });
    focusSel.innerHTML = opts.join("");

    function activeTypes() {
      var t = {};
      typeBoxes.forEach(function (b) { if (b.checked) t[b.value] = true; });
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
        oids = ix.d.graph.nodes.map(function (n) { return n.oid; });
      }
      // type filter (keep focus even if its type is off)
      oids = oids.filter(function (o) {
        return types[ix.byOid[o].type] || o === focus;
      });
      if (!oids.length) { host.innerHTML = '<div class="diag-empty">Nothing to show — enable object types.</div>'; return; }
      if (oids.length > 240) {
        host.innerHTML = '<div class="diag-empty">' + oids.length +
          ' objects — pick a focus object or reduce depth to render the graph.</div>';
        return;
      }
      var def = buildFlowchart(ix, oids, { dir: focus ? "LR" : "TB" });
      renderInto(host, def, ix, function (oid) { openDrawer(ix, oid); });
    }

    focusSel.addEventListener("change", draw);
    depthSel.addEventListener("change", draw);
    typeBoxes.forEach(function (b) { b.addEventListener("change", draw); });
    panel._draw = draw;
  }

  // ---- Object drawer (per-object diagram + details + flow) ---------------
  function openDrawer(ix, oid) {
    var n = ix.byOid[oid]; if (!n) return;
    showSelector(n);
    var drawer = document.getElementById("objDrawer");
    var body = drawer.querySelector(".drawer-body");
    drawer.querySelector(".drawer-title").innerHTML =
      '<span class="tag ' + n.type + '">' + TYPE_LABEL[n.type] + "</span> " + esc(n.name);
    drawer.querySelector(".drawer-sub").textContent = n.fullPath;

    var parts = [];
    if (n.type === "vs") parts.push(vsDetail(ix, oid));
    else if (n.type === "pool") parts.push(poolDetail(ix, oid));
    else if (n.type === "rule") {
      var refs = ruleRefs(ix, oid);
      if (refs) parts.push('<h4>References</h4>' + refs);
    }
    parts.push('<h4>Neighbourhood</h4><div class="diag-host drawer-diag"></div>');
    if (n.type === "vs") parts.push('<h4>Processing flow</h4><div class="diag-host flow-diag"></div>');
    var src = sourceHtml(ix, n);
    if (src) parts.push('<h4>Source</h4>' + src);
    body.innerHTML = parts.join("");

    var oids = Object.keys(neighborhood(ix, [oid], 2));
    renderInto(body.querySelector(".drawer-diag"),
      buildFlowchart(ix, oids, { dir: "LR" }), ix, function (o) { openDrawer(ix, o); });
    if (n.type === "vs") {
      renderInto(body.querySelector(".flow-diag"), buildFlow(ix, oid), ix,
        function (o) { openDrawer(ix, o); });
    }
    // Cross-links inside the source stanza jump to that object's drawer.
    body.querySelectorAll(".conf-link[data-goto]").forEach(function (a) {
      a.addEventListener("click", function (e) {
        e.preventDefault();
        var toOid = ix.byPath[a.getAttribute("data-goto")];
        if (toOid) openDrawer(ix, toOid);
      });
    });
    // Make the referenced-object chips (iRule refs, etc.) clickable.
    wireObjLinks(body, ix);
    drawer.classList.add("open");
    document.getElementById("drawerScrim").classList.add("open");
  }

  function findVirtual(ix, oid) {
    var fp = oid.split(":").slice(1).join(":");
    return ix.d.virtuals.find(function (v) { return v.fullPath === fp; });
  }
  function findPool(ix, oid) {
    var fp = oid.split(":").slice(1).join(":");
    return ix.d.pools.find(function (p) { return p.fullPath === fp; });
  }

  function vsDetail(ix, oid) {
    var v = findVirtual(ix, oid); if (!v) return "";
    var L = v.listener || {};
    var rows = [
      ["Destination", esc(L.address || "-") + (L.prefix != null && L.prefix < L.maxPrefix ? "/" + L.prefix : "") +
        (L.routeDomain ? " %" + L.routeDomain : "")],
      ["Port", esc(L.portRaw || "-")],
      ["Protocol", esc(L.protocol || "-")],
      ["Source", esc(L.source || "-")],
      ["VLANs", (L.vlans && L.vlans.length) ? esc(L.vlans.join(", ")) +
        (L.vlansDisabled ? " (disabled)" : L.vlansEnabled ? " (enabled)" : "") : "all"],
    ];
    var meta = '<table class="kv">' + rows.map(function (r) {
      return "<tr><th>" + r[0] + "</th><td>" + r[1] + "</td></tr>";
    }).join("") + "</table>";

    // Object properties, each linked to the object it references (clickable
    // when that object exists in this device — wired by wireObjLinks()).
    var snat = v.snatpool
      ? refChip(v.snatpool, "snat")
      : (v.sourceXlate ? esc(v.sourceXlate) : '<span class="muted">—</span>');
    var propRows = [
      ["Pool", refChip(v.pool, "pool")],
      ["iRules", refChips(v.rules, "rule")],
      ["Persistence", refChips(v.persist, "persist")],
      ["Policies", refChips(v.policies, "policy")],
      ["SNAT", snat],
    ];
    var props = '<table class="kv">' + propRows.map(function (r) {
      return "<tr><th>" + r[0] + "</th><td>" + r[1] + "</td></tr>";
    }).join("") + "</table>";

    var staticProfiles = refChips(v.profiles, "prof");

    var dyn = (v.dynamicProfiles || []).map(function (a) {
      return '<span class="tag amber" title="via iRule ' + esc(a.rule) + '">' +
        esc(a.effect) + (a.arg ? " " + esc(a.arg) : "") + " · " + esc(a.category) + "</span>";
    }).join("");
    var dynBlock = dyn
      ? '<h4>Dynamic (iRule-driven) changes</h4><div class="tagwrap">' + dyn + "</div>"
      : "";

    return '<h4>Listener</h4>' + meta +
      '<h4>Properties</h4>' + props +
      '<h4>Static profiles</h4><div class="tagwrap">' + staticProfiles + "</div>" + dynBlock;
  }

  function poolDetail(ix, oid) {
    var p = findPool(ix, oid); if (!p) return "";
    var rows = (p.members || []).map(function (m) {
      return "<tr><td class='mono'>" + esc(m.name.split("/").pop()) + "</td><td class='mono'>" +
        esc(m.address) + "</td><td class='mono'>" + esc(m.port) + "</td></tr>";
    }).join("") || "<tr><td colspan=3 class='muted'>no members</td></tr>";
    return "<h4>Members</h4><table class='grid mini'><thead><tr><th>Member</th><th>Address</th><th>Port</th></tr></thead><tbody>" +
      rows + "</tbody></table>";
  }

  function findRule(ix, oid) {
    var fp = oid.split(":").slice(1).join(":");
    return (ix.d.rules || []).find(function (r) { return r.fullPath === fp; });
  }

  // The iRule's referenced-object tables — statically referenced objects, plus
  // the objects each reconstructed dynamic filter could select, resolved per
  // attaching-virtual partition. Mirrors the inline iRule-row detail so the
  // drawer inspector carries the same linked / potentially-linked / resolved
  // information. Object chips are wired clickable by wireObjLinks().
  function ruleRefs(ix, oid) {
    var r = findRule(ix, oid); if (!r) return "";
    var ro = r.referencedObjects; if (!ro) return "";
    var hasStatic = ro.static && ro.static.length;
    var hasDyn = ro.dynamic && ro.dynamic.length;
    var out = '<div class="irule-refs">';
    if (hasStatic) {
      out += '<div class="refs-static"><span class="refs-lbl">Referenced objects:</span> ';
      ro.static.forEach(function (o) {
        out += '<span class="ref-obj" data-oid="' + esc(o.oid) + '" title="' +
          esc(o.type + " " + o.fullPath) + '">' + esc(o.type + " " + o.name) + "</span> ";
      });
      out += "</div>";
    }
    if (!hasStatic && !hasDyn) {
      out += '<div class="refs-none muted">No pool / node / snatpool / data-group references (static or dynamic) in this iRule.</div>';
    }
    (ro.dynamic || []).forEach(function (g) {
      out += '<div class="refs-dyn"><div class="refs-ctx">Potentially referenced objects';
      if (g.attached) {
        out += " — resolved for ";
        (g.virtuals || []).forEach(function (vn) { out += '<span class="tag">' + esc(vn) + "</span>"; });
        out += " in partition <code>/" + esc(g.partition) + "</code>";
      } else {
        out += " — this iRule is not attached to any virtual; filters resolved in <code>/" + esc(g.partition) + "</code>";
      }
      out += '</div><table class="refs-table"><thead><tr><th>Determined filter</th><th>Type</th><th>Matching objects (this partition)</th></tr></thead><tbody>';
      (g.filters || []).forEach(function (f) {
        out += '<tr><td><code class="attach-pat" title="' +
          esc("reconstructed from the iRule source: " + (f.raw || "")) + '">' + esc(f.glob || "") + "</code>";
        if (f.unconstrained) out += ' <span class="muted small">(any name)</span>';
        else if (f.exact) out += ' <span class="muted small">(exact)</span>';
        out += '</td><td class="mono small">' + esc(f.type || "") + "</td><td>";
        if (f.objects && f.objects.length) {
          f.objects.forEach(function (o) {
            out += '<span class="ref-obj" data-oid="' + esc(o.oid) + '" title="' +
              esc(o.fullPath) + '">' + esc(o.name) + "</span> ";
          });
        } else {
          out += '<span class="muted">' +
            (f.unconstrained ? "any " + esc(f.type || "") + " in scope" : "none defined match") + "</span>";
        }
        out += "</td></tr>";
      });
      out += "</tbody></table></div>";
    });
    out += "</div>";
    return out;
  }

  // ---- Per-virtual processing flow --------------------------------------
  function buildFlow(ix, vsOid) {
    var v = findVirtual(ix, vsOid); if (!v) return "flowchart LR\n  x[No data]";
    var L = v.listener || {};
    var s = [], n = 0;
    function id() { return "F" + (n++); }
    var lines = ["flowchart LR"];
    var client = id();
    lines.push("  " + client + '(["Client<br/>' + esc(L.source || "any") + '"]):::cl');
    var lsnr = id();
    lines.push("  " + lsnr + '["Listener<br/>' + esc(v.name) + "<br/>" +
      esc((L.address || "") + ":" + (L.portRaw || "")) + '"]:::vs');
    lines.push("  " + client + " --> " + lsnr);
    var prev = lsnr;
    if (L.vlans && L.vlans.length) {
      var vl = id();
      lines.push("  " + vl + '["VLAN<br/>' + esc(L.vlans.join(", ")) + '"]:::vlan');
      lines.push("  " + client + " -.-> " + vl + " -.-> " + lsnr);
    }
    // profiles
    (v.profiles || []).forEach(function (p) {
      var pid = id();
      lines.push("  " + pid + '["' + esc(p.split("/").pop()) + '"]:::prof');
      lines.push("  " + prev + " --> " + pid); prev = pid;
    });
    // dynamic (iRule) changes
    (v.dynamicProfiles || []).forEach(function (a) {
      var did = id();
      lines.push("  " + did + '["iRule: ' + esc(a.effect) + (a.arg ? " " + esc(a.arg) : "") + '"]:::dyn');
      lines.push("  " + prev + " -.-> " + did); prev = did;
    });
    // iRules
    (v.rules || []).forEach(function (r) {
      var rid = id();
      lines.push("  " + rid + '{{"iRule<br/>' + esc(r.split("/").pop()) + '"}}:::rule');
      lines.push("  " + prev + " --> " + rid); prev = rid;
    });
    // pool + members
    if (v.pool) {
      var pl = id();
      lines.push("  " + pl + '["Pool<br/>' + esc(v.pool.split("/").pop()) + '"]:::pool');
      lines.push("  " + prev + " --> " + pl);
      var pool = findPool(ix, "pool:" + v.pool);
      (pool ? pool.members : []).slice(0, 12).forEach(function (m) {
        var mid = id();
        lines.push("  " + mid + '[("' + esc(m.address) + ":" + esc(m.port) + '")]:::node');
        lines.push("  " + pl + " --> " + mid);
      });
    } else {
      var np = id();
      lines.push("  " + np + '["no default pool<br/>(forwarding / policy)"]:::muted');
      lines.push("  " + prev + " --> " + np);
    }
    lines.push("classDef cl fill:#f8fafc,stroke:#94a3b8;");
    lines.push("classDef vs fill:#dbeafe,stroke:#2563eb;");
    lines.push("classDef prof fill:#e0f2fe,stroke:#0284c7;");
    lines.push("classDef rule fill:#ede9fe,stroke:#7c3aed;");
    lines.push("classDef pool fill:#dcfce7,stroke:#16a34a;");
    lines.push("classDef node fill:#f1f5f9,stroke:#64748b;");
    lines.push("classDef dyn fill:#fef3c7,stroke:#d97706,stroke-dasharray:4 2;");
    lines.push("classDef vlan fill:#f5f5f4,stroke:#78716c;");
    lines.push("classDef muted fill:#f8fafc,stroke:#cbd5e1,color:#64748b;");
    return lines.join("\n");
  }

  function closeDrawer() {
    document.getElementById("objDrawer").classList.remove("open");
    document.getElementById("drawerScrim").classList.remove("open");
  }

  // ---- IP helpers (IPv4 + IPv6, BigInt-based) ----------------------------
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
      if (p.some(function (x) { return isNaN(x) || x < 0 || x > 255; })) return null;
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
    var mask = ((1n << BigInt(prefix)) - 1n) << BigInt(bits - prefix);
    return (ipBig & mask) === (netBig & mask);
  }

  function matchListeners(ix, q) {
    var qVer = ipVer(q.dst) || ipVer(q.src.split("/")[0]);
    var dstBig = ipToBig(q.dst);
    var out = [];
    ix.d.virtuals.forEach(function (v) {
      var L = v.listener || {}; if (v.disabled) return;
      var lVer = L.family === "IPv6" ? 6 : 4;
      if (qVer && !L.anyAddr && lVer !== qVer) return; // family must agree
      if (q.rd !== "" && String(L.routeDomain) !== String(q.rd)) return;
      if (!L.anyAddr && q.dst && !inNet(dstBig, lVer, L.address, L.prefix)) return;
      if (q.port !== "" && L.port !== 0 && String(L.port) !== String(q.port)) return;
      if (q.proto && q.proto !== "any" && L.protocol && L.protocol !== "any" &&
        L.protocol !== q.proto) return;
      if (q.vlan && L.vlans && L.vlans.length) {
        var on = L.vlans.indexOf(q.vlan) >= 0;
        if (L.vlansEnabled && !on) return;
        if (L.vlansDisabled && on) return;
      }
      if (q.src && q.src !== "0.0.0.0/0" && q.src !== "::/0" && L.source &&
        L.source !== "0.0.0.0/0" && L.source !== "::/0") {
        var sp = q.src.split("/"); var ls = L.source.split("/");
        var sver = ipVer(sp[0]);
        if (sver && ipVer(ls[0]) === sver &&
          !inNet(ipToBig(sp[0]), sver, ls[0], parseInt(ls[1] || String(sver === 6 ? 128 : 32), 10))) return;
      }
      out.push(v);
    });
    out.sort(function (a, b) {
      var la = a.listener, lb = b.listener;
      return (lb.prefix - la.prefix)
        || ((lb.port !== 0) - (la.port !== 0))
        || (lb.sourcePrefix - la.sourcePrefix)
        || ((lb.vlansEnabled ? 1 : 0) - (la.vlansEnabled ? 1 : 0))
        || a.name.localeCompare(b.name);
    });
    return out;
  }

  // ---- lookups -----------------------------------------------------------
  function profileByPath(ix, path) {
    return ix.d.profiles.find(function (p) { return p.fullPath === path; });
  }
  function policyByPath(ix, path) {
    return ix.d.policies.find(function (p) { return p.fullPath === path; });
  }

  // ---- request-processing simulation ------------------------------------
  function strOp(field, op, val, ci) {
    if (ci) { field = field.toLowerCase(); val = val.toLowerCase(); }
    switch (op) {
      case "starts-with": return field.indexOf(val) === 0;
      case "ends-with": return field.length >= val.length && field.slice(-val.length) === val;
      case "contains": return field.indexOf(val) >= 0;
      case "equals": return field === val;
      default: return field.indexOf(val) >= 0;
    }
  }

  // Evaluate an LTM policy against the request (first-match strategy).
  function evalPolicy(pol, req) {
    var fired = [];
    for (var i = 0; i < pol.rules.length; i++) {
      var r = pol.rules[i];
      if (!r.conditions.length) continue;
      var all = r.conditions.every(function (c) {
        var field = c.operand === "http-host" ? req.host
          : (c.selector === "path" ? req.uri.split("?")[0] : req.uri);
        var hit = (c.values || []).some(function (val) {
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

  // Best-effort extraction + evaluation of an iRule's request-time actions.
  function evalIrule(rule, req) {
    var body = rule.body || "", acts = [];
    function cond(ctx) {
      var m = /HTTP::(uri|host)\b[^\n]*?\b(starts_with|ends_with|contains|equals|eq)\s+"?([^"\n\]]+)"?/i.exec(ctx);
      if (!m) return { has: false, hit: true, text: "" };
      var field = m[1].toLowerCase() === "host" ? req.host : req.uri;
      var op = { starts_with: "starts-with", ends_with: "ends-with", contains: "contains", equals: "equals", eq: "equals" }[m[2].toLowerCase()];
      return { has: true, hit: strOp(field, op, m[3].trim(), true), text: "HTTP::" + m[1] + " " + m[2] + ' "' + m[3].trim() + '"' };
    }
    // pool selection (with nearby guard)
    var re = /\bpool\s+(\/[^\s;}"]+|\w[\w.\-]*)/g, m;
    while ((m = re.exec(body))) {
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
      [/\bnode\s+(\d[\d.:a-f]+)/ig, "node"],
    ];
    patterns.forEach(function (pp) {
      var r2 = pp[0], mm;
      while ((mm = r2.exec(body))) {
        acts.push({ kind: pp[1], target: mm[1] || "", arg: mm[2] || "", val: mm[3] || "" });
      }
    });
    return acts;
  }

  function pickSslProfile(ix, v, sni) {
    var ssl = (v.profiles || []).map(function (p) { return profileByPath(ix, p); })
      .filter(function (p) { return p && /CLIENT_SSL/.test(p.type); });
    if (!ssl.length) return null;
    if (sni) {
      var byName = ssl.find(function (p) { return p.name.toLowerCase().indexOf(sni.toLowerCase()) >= 0; });
      if (byName) return byName;
    }
    return ssl[0];
  }

  function simulate(ix, v, req) {
    var stages = [];
    var L = v.listener || {};
    var isHttp = (v.profiles || []).some(function (p) { return /\/http\b|http$/.test(p) || /_http/.test(p); });

    // 1. client SSL
    var ssl = pickSslProfile(ix, v, req.sni);
    if (ssl) {
      stages.push({
        title: "Client SSL", cls: "ssl",
        rows: [
          ["Profile", ssl.name],
          ["Certificate", ssl.cert || "—"],
          ["Key", ssl.key || "—"],
          ["Chain", ssl.chain || "—"],
          ["Ciphers", ssl.ciphers || "(profile default)"],
          ["SNI", req.sni ? req.sni : "(none supplied — default profile)"],
        ],
      });
    }

    // 2. HUD / profiles applied
    var profRows = (v.profiles || []).map(function (p) {
      var po = profileByPath(ix, p);
      return [p.split("/").pop(), po ? po.type.replace(/_/g, " ").toLowerCase() : "system default"];
    });
    stages.push({ title: "Profiles applied (HUD)", cls: "hud", rows: profRows.length ? profRows : [["(system defaults only)", ""]] });

    // 3. header transform + iRules + policy (request time)
    var headers = req.headers.slice();
    function setHeader(k, val) {
      var i = headers.findIndex(function (h) { return h[0].toLowerCase() === k.toLowerCase(); });
      if (i >= 0) headers[i] = [k, val]; else headers.push([k, val]);
    }
    var changes = [];
    var selectedPool = v.pool || "";
    var poolReason = v.pool ? "default pool" : "";

    // iRules
    (v.rules || []).forEach(function (rp) {
      var rule = ix.d.rules.find(function (r) { return r.fullPath === rp || r.name === rp.split("/").pop(); });
      if (!rule) return;
      var acts = evalIrule(rule, req);
      acts.forEach(function (a) {
        if (a.kind === "pool" && a.active) { selectedPool = a.target.indexOf("/") === 0 ? a.target : "/Common/" + a.target; poolReason = "iRule " + rule.name + (a.cond ? " (" + a.cond + ")" : ""); }
        else if (a.kind === "header") {
          if (a.target === "remove") { headers = headers.filter(function (h) { return h[0].toLowerCase() !== a.arg.toLowerCase(); }); changes.push("iRule " + rule.name + ": remove header " + a.arg); }
          else { setHeader(a.arg, a.val); changes.push("iRule " + rule.name + ": " + a.target + " " + a.arg + ": " + a.val); }
        } else if (a.kind === "redirect") { changes.push("iRule " + rule.name + ": HTTP::redirect " + a.target); }
        else if (a.kind === "respond") { changes.push("iRule " + rule.name + ": HTTP::respond " + a.target); }
        else if (a.kind === "persist") { changes.push("iRule " + rule.name + ": persist " + a.target); }
        else if (a.kind === "ssl") { changes.push("iRule " + rule.name + ": " + a.target + (a.arg ? " " + a.arg : "")); }
        else if (a.kind === "node") { selectedPool = "(node " + a.target + ")"; poolReason = "iRule " + rule.name + " node override"; }
      });
    });

    // LTM policy
    var policyRows = [];
    (v.policies || []).forEach(function (pp) {
      var pol = policyByPath(ix, pp);
      if (!pol) return;
      var fired = evalPolicy(pol, req);
      fired.forEach(function (r) {
        r.actions.forEach(function (a) {
          if (a.target === "forward" && a.pool) { selectedPool = a.pool; poolReason = "policy " + pol.name + " · rule " + r.name; }
          if (/http-header|http-uri|http-host/.test(a.target) || a.target === "http-header") {
            if (a.name) { setHeader(a.name, a.value); changes.push("policy " + pol.name + ": set " + a.name + ": " + a.value); }
          }
          policyRows.push([pol.name + " · " + r.name, a.verb + " " + a.target + (a.pool ? " → " + a.pool.split("/").pop() : (a.location ? " " + a.location : ""))]);
        });
      });
      if (!fired.length && pol.rules.length) policyRows.push([pol.name, "no rule matched this request"]);
    });
    if (policyRows.length) stages.push({ title: "LTM policy", cls: "policy", rows: policyRows });

    if (changes.length || headers.length) {
      stages.push({
        title: "Request after processing", cls: "req",
        pre: (req.method || "GET") + " " + (req.uri || "/") + " HTTP/1.1\n" +
          headers.map(function (h) { return h[0] + ": " + h[1]; }).join("\n"),
        notes: changes,
      });
    }

    // 4. load balancing / member selection
    var pool = selectedPool.indexOf("(") === 0 ? null : findPool(ix, "pool:" + selectedPool);
    var lbRows = [["Selected pool", selectedPool ? selectedPool.split("/").pop() : "none (dropped / forwarded)"], ["Reason", poolReason || "—"]];
    if (pool) {
      lbRows.push(["LB method", pool.lbMode || "round-robin"]);
      var up = pool.members.filter(function (m) { return m.state !== "down" && m.state !== "user-down"; });
      var pick = up[0] || pool.members[0];
      if (pick) lbRows.push(["Illustrative member", pick.address + ":" + pick.port +
        (pick.ratio ? " ratio " + pick.ratio : "") + (pick.priorityGroup ? " pg " + pick.priorityGroup : "")]);
    }
    stages.push({
      title: "Load balancing & member selection", cls: "lb", rows: lbRows,
      members: pool ? pool.members : null,
    });

    // 5. server SSL
    var sssl = (v.profiles || []).map(function (p) { return profileByPath(ix, p); })
      .filter(function (p) { return p && /SERVER_SSL/.test(p.type); })[0];
    if (sssl) stages.push({ title: "Server SSL", cls: "ssl", rows: [["Profile", sssl.name], ["Certificate", sssl.cert || "—"]] });

    return stages;
  }

  function renderSim(ix, v, host) {
    var L = v.listener || {};
    var hasSsl = !!pickSslProfile(ix, v, "");
    var isHttp = (v.profiles || []).some(function (p) { return /http/.test(p); });
    var h = ['<div class="sim-head"><b>Processing for ' + esc(v.name) + '</b> <span class="mono muted">' +
      esc(L.address) + ":" + esc(L.portRaw) + " · " + esc(L.protocol) + '</span></div>'];
    h.push('<div class="sim-req">');
    if (hasSsl) h.push('<div class="fld"><label>TLS SNI</label><input class="sim-sni lm-field" placeholder="www.example.com"></div>');
    h.push('<div class="fld"><label>Method</label><select class="sim-method"><option>GET</option><option>POST</option><option>PUT</option><option>DELETE</option></select></div>');
    h.push('<div class="fld grow"><label>URI</label><input class="sim-uri lm-field" placeholder="/path?query" value="/"></div>');
    h.push('<div class="fld"><label>Host</label><input class="sim-host lm-field" placeholder="app.example.com"></div>');
    h.push('<button class="lm-run sim-go">Simulate ▸</button>');
    h.push('</div>');
    h.push('<div class="fld"><label>Extra request headers (one per line, <code>Name: value</code>)</label><textarea class="sim-headers" rows="2" placeholder="X-Custom: 1"></textarea></div>');
    h.push('<div class="sim-out"></div>');
    host.innerHTML = h.join("");

    function go() {
      var headers = [];
      var host_ = host.querySelector(".sim-host").value.trim();
      if (host_) headers.push(["Host", host_]);
      (host.querySelector(".sim-headers").value || "").split("\n").forEach(function (ln) {
        var i = ln.indexOf(":"); if (i > 0) headers.push([ln.slice(0, i).trim(), ln.slice(i + 1).trim()]);
      });
      var req = {
        sni: hasSsl ? (host.querySelector(".sim-sni").value.trim()) : "",
        method: host.querySelector(".sim-method").value,
        uri: host.querySelector(".sim-uri").value.trim() || "/",
        host: host_, headers: headers,
      };
      var stages = simulate(ix, v, req);
      var o = host.querySelector(".sim-out");
      o.innerHTML = stages.map(function (s) {
        var inner = "";
        if (s.rows) inner += '<table class="kv">' + s.rows.map(function (r) {
          return "<tr><th>" + esc(r[0]) + "</th><td class='mono'>" + esc(r[1]) + "</td></tr>";
        }).join("") + "</table>";
        if (s.pre) inner += "<pre class='code'>" + esc(s.pre) + "</pre>";
        if (s.notes && s.notes.length) inner += "<ul class='sim-notes'>" + s.notes.map(function (n) { return "<li>" + esc(n) + "</li>"; }).join("") + "</ul>";
        if (s.members) inner += "<table class='grid mini'><thead><tr><th>Member</th><th>Addr</th><th>Port</th><th>Ratio</th><th>PG</th></tr></thead><tbody>" +
          s.members.map(function (m) { return "<tr><td class='mono'>" + esc(m.name.split("/").pop()) + "</td><td class='mono'>" + esc(m.address) + "</td><td class='mono'>" + esc(m.port) + "</td><td class='mono'>" + esc(m.ratio || "1") + "</td><td class='mono'>" + esc(m.priorityGroup || "0") + "</td></tr>"; }).join("") + "</tbody></table>";
        return '<div class="sim-stage ' + s.cls + '"><div class="sim-stage-t">' + esc(s.title) + "</div>" + inner + "</div>";
      }).join("");
    }
    host.querySelector(".sim-go").addEventListener("click", go);
    host.querySelectorAll(".sim-req .lm-field").forEach(function (el) {
      el.addEventListener("keydown", function (e) { if (e.key === "Enter") go(); });
    });
    go();
  }

  function initListener(deviceEl, ix) {
    var panel = deviceEl.querySelector('.panel[data-panel="listener"]');
    if (!panel) return;
    var f = {
      src: panel.querySelector(".lm-src"), dst: panel.querySelector(".lm-dst"),
      port: panel.querySelector(".lm-port"), proto: panel.querySelector(".lm-proto"),
      vlan: panel.querySelector(".lm-vlan"), rd: panel.querySelector(".lm-rd"),
    };
    var out = panel.querySelector(".lm-results");
    var sim = panel.querySelector(".lm-sim");
    var vlans = {};
    ix.d.virtuals.forEach(function (v) { (v.listener.vlans || []).forEach(function (x) { vlans[x] = 1; }); });
    f.vlan.innerHTML = '<option value="">any VLAN</option>' +
      Object.keys(vlans).sort().map(function (x) { return '<option>' + esc(x) + "</option>"; }).join("");

    // Reverse-populate the form with the most-specific flow that lands on `v`,
    // then re-match and open its processing simulator.
    function selectVs(v) {
      showSelector({ type: "vs", fullPath: v.fullPath });
      var L = v.listener;
      f.dst.value = L.anyAddr ? (L.family === "IPv6" ? "::" : "0.0.0.0") : L.address;
      f.port.value = L.port ? String(L.port) : "";
      f.proto.value = (L.protocol && L.protocol !== "any") ? L.protocol : "any";
      f.rd.value = String(L.routeDomain || 0);
      f.vlan.value = (L.vlansEnabled && L.vlans.length) ? L.vlans[0] : "";
      run(v);
    }

    function run(focusVs) {
      var q = {
        src: f.src.value.trim() || "0.0.0.0/0", dst: f.dst.value.trim(),
        port: f.port.value.trim(), proto: f.proto.value, vlan: f.vlan.value,
        rd: f.rd.value.trim(),
      };
      var matches = matchListeners(ix, q);
      sim.innerHTML = "";
      if (!matches.length) { out.innerHTML = '<div class="diag-empty">No virtual server matches this flow.</div>'; return; }
      var tiers = [];
      matches.forEach(function (v) {
        var key = v.listener.prefix + ":" + (v.listener.port !== 0 ? 1 : 0) + ":" + v.listener.sourcePrefix;
        var t = tiers.find(function (t) { return t.key === key; });
        if (!t) { t = { key: key, prefix: v.listener.prefix, port: v.listener.port, vs: [] }; tiers.push(t); }
        t.vs.push(v);
      });
      var focus = focusVs || matches[0];
      var html = ['<div class="lm-note">The most specific listener is at the top — that is where this flow lands. Listeners of equal specificity are shown side by side; lower tiers only receive traffic the tiers above do not. Click a listener to load the exact flow that reaches it and simulate its processing.</div>'];
      html.push('<div class="tiers">');
      tiers.forEach(function (t, ti) {
        html.push('<div class="tier' + (ti === 0 ? " match" : "") + '">');
        html.push('<div class="tier-key">/' + t.prefix + ' · ' + (t.port ? "port " + t.port : "any port") + '</div>');
        html.push('<div class="tier-vs">');
        t.vs.forEach(function (v) {
          var L = v.listener;
          html.push('<button class="lm-card" data-fp="' + esc(v.fullPath) + '"' + (v === focus ? ' data-focus="1"' : "") + '>' +
            '<span class="lm-name">' + esc(v.name) + (v === focus ? ' <span class="tag green">match</span>' : "") + '</span>' +
            '<span class="lm-dest mono">' + esc(L.address) + (L.prefix < L.maxPrefix ? "/" + L.prefix : "") +
            (L.routeDomain ? "%" + L.routeDomain : "") + ':' + esc(L.portRaw) + '</span>' +
            '<span class="lm-meta mono">' + esc(L.protocol) + (v.pool ? " → " + esc(v.pool.split("/").pop()) : " · no pool") + '</span>' +
            '</button>');
        });
        html.push('</div></div>');
        if (ti < tiers.length - 1) html.push('<div class="tier-arrow">falls through to ▾</div>');
      });
      html.push('</div>');
      out.innerHTML = html.join("");
      out.querySelectorAll(".lm-card").forEach(function (b) {
        b.addEventListener("click", function () {
          var v = ix.d.virtuals.find(function (x) { return x.fullPath === b.dataset.fp; });
          if (v) selectVs(v);
        });
      });
      // open the processing simulator for the focused match
      renderSim(ix, focus, sim);
    }
    panel.querySelector(".lm-run").addEventListener("click", function () { run(); });
    panel.querySelectorAll(".lm-field").forEach(function (el) {
      el.addEventListener("keydown", function (e) { if (e.key === "Enter") run(); });
    });
    run();
  }

  // ---- wire object links in tables + init per device ---------------------
  function wireObjLinks(root, ix) {
    root.querySelectorAll("[data-oid],[data-oref]").forEach(function (el) {
      if (el._wired) return; el._wired = true;
      // A reference (data-oref) is resolved by full-path to the object it points
      // at; a data-oid is already the exact object. An unresolved reference (the
      // target object isn't in this config) is left as plain, unclickable text.
      var oid = el.dataset.oid || (el.dataset.oref ? ix.byPath[el.dataset.oref] : "");
      if (!oid || !ix.byOid[oid]) return;
      el.classList.add("objlink");
      el.addEventListener("click", function (ev) {
        ev.preventDefault(); ev.stopPropagation();
        openDrawer(ix, oid);
      });
    });
  }

  document.querySelectorAll(".device").forEach(function (deviceEl) {
    var ix = IDX[parseInt(deviceEl.dataset.dev, 10)];
    initTopology(deviceEl, ix);
    initListener(deviceEl, ix);
    wireObjLinks(deviceEl, ix);
    // draw topology lazily the first time its tab is shown
    deviceEl.querySelectorAll('.tab[data-panel="topology"]').forEach(function (tab) {
      tab.addEventListener("click", function () {
        var panel = deviceEl.querySelector('.panel[data-panel="topology"]');
        if (panel && panel._draw && !panel._drawn) { panel._drawn = true; panel._draw(); }
      });
    });
  });

  // The cross-device architecture diagram (top-level, always visible).
  initArchitecture();

  // drawer close handlers
  var closeBtn = document.querySelector("#objDrawer .drawer-close");
  if (closeBtn) closeBtn.addEventListener("click", closeDrawer);
  var scrim = document.getElementById("drawerScrim");
  if (scrim) scrim.addEventListener("click", closeDrawer);
  document.addEventListener("keydown", function (e) { if (e.key === "Escape") closeDrawer(); });

  // ---- graph-aware global search ----------------------------------------
  // Filters every object table (across all tabs) of the active device to rows
  // that match the query, plus the rows *linked* to a match (one hop in the
  // reference graph), which render dimmed to show they didn't match directly.
  (function initSearch() {
    var search = document.getElementById("globalSearch");
    if (!search) return;

    function detailOf(row) {
      var d = row.nextElementSibling;
      return (d && d.classList.contains("detail")) ? d : null;
    }

    // Parse "ip" or "ip/prefix" (v4/v6) into a network, else null.
    function toNet(s) {
      var m = /^([0-9a-fA-F:.]+)(?:\/(\d+))?$/.exec(s);
      if (!m) return null;
      var ver = ipVer(m[1]); if (!ver) return null;
      var big = ipToBig(m[1]); if (big == null) return null;
      var full = ver === 6 ? 128 : 32;
      var pfx = m[2] !== undefined ? parseInt(m[2], 10) : full;
      if (pfx > full) return null;
      return { ver: ver, big: big, prefix: pfx, full: full };
    }
    // Does `outer` (the less-specific network) contain `inner`?
    function contains(outer, inner) {
      if (outer.ver !== inner.ver || inner.prefix < outer.prefix) return false;
      var mask = ((1n << BigInt(outer.prefix)) - 1n) << BigInt(outer.full - outer.prefix);
      return (outer.big & mask) === (inner.big & mask);
    }
    function ipOverlap(a, b) { return contains(a, b) || contains(b, a); }
    var IP_TOKEN = /[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+(?:\/[0-9]+)?|[0-9a-fA-F:]{2,}:[0-9a-fA-F:]*(?:\/[0-9]+)?/g;

    // Parse a search query as an address, honouring BIG-IP `%route-domain` and
    // an optional `:port` — e.g. `10.0.0.1%2:8443`, `192.168.1.0/24`, `::1`.
    function parseIpQuery(raw) {
      var s = raw.trim();
      if (!s) return null;
      var rd = null, port = null;
      // trailing :port only for IPv4 (or bracketed IPv6) to avoid eating v6 colons
      var pm = /^(.*?):(\d+)$/.exec(s);
      if (pm && pm[1].indexOf(".") >= 0) { s = pm[1]; port = pm[2]; }
      var rm = /^([^%]*)%(\d+)(.*)$/.exec(s);
      if (rm) { rd = parseInt(rm[2], 10); s = rm[1] + rm[3]; }
      var net = toNet(s);
      if (!net) return null;
      return { net: net, rd: rd, port: port, base: s.split("/")[0] };
    }
    function ruleByRef(ix, rp) {
      return ix.d.rules.find(function (r) { return r.fullPath === rp || r.name === rp.split("/").pop(); });
    }
    function polByPath(ix, pp) {
      return ix.d.policies.find(function (x) { return x.fullPath === pp; });
    }
    // Virtuals that can route to any of `poolPaths` (default pool / iRule / policy).
    function virtualsReaching(ix, poolPaths) {
      var out = {};
      ix.d.virtuals.forEach(function (v) {
        var reaches = poolPaths[v.pool]
          || (v.rules || []).some(function (rp) {
            var r = ruleByRef(ix, rp); return r && (r.refPools || []).some(function (p) { return poolPaths[p]; });
          })
          || (v.policies || []).some(function (pp) {
            var pol = polByPath(ix, pp);
            return pol && pol.rules.some(function (rr) {
              return rr.actions.some(function (a) { return poolPaths[a.pool]; });
            });
          });
        if (reaches) out["vs:" + v.fullPath] = true;
      });
      return out;
    }

    function run() {
      var dEl = document.querySelector(".device.active");
      if (!dEl) return;
      var ix = IDX[parseInt(dEl.dataset.dev, 10)];
      var raw = search.value.trim();
      var q = raw.toLowerCase();
      var rows = dEl.querySelectorAll(".panel tr.searchable");

      if (!q) {
        rows.forEach(function (r) {
          r.classList.remove("hidden", "search-linked", "search-hit");
          var d = detailOf(r); if (d) d.classList.remove("hidden");
        });
        return;
      }

      // If the query is an IP / CIDR (optionally `%rd` and `:port`), also match
      // objects (nodes, members, virtuals, data-group records, iRule bodies, …)
      // whose own fields contain an address within — or containing — it.
      var qParsed = parseIpQuery(raw);
      var qNet = qParsed ? qParsed.net : null;

      // 1st pass: direct matches on each object's OWN identity/fields
      // (`data-search`) — including the objects it references *inside an iRule*
      // and its data-group records — but not its cross-reference table columns.
      var info = [], direct = {};
      rows.forEach(function (r) {
        var el = r.querySelector("[data-oid]");
        var oid = el ? el.dataset.oid : null;
        var hay = (r.dataset.search || r.textContent).toLowerCase();
        var tm = hay.indexOf(q) >= 0;
        if (!tm && qNet) {
          var toks = hay.match(IP_TOKEN);
          if (toks) {
            for (var i = 0; i < toks.length && !tm; i++) {
              var tn = toNet(toks[i]);
              // skip wildcard tokens (e.g. a virtual's `source 0.0.0.0/0`),
              // which would otherwise "contain" any address the user types.
              if (tn && tn.prefix > 0 && ipOverlap(qNet, tn)) tm = true;
            }
          }
        }
        info.push({ row: r, oid: oid, tm: tm });
        if (tm && oid) direct[oid] = true;
      });

      // expand to one-hop neighbours of every directly-matched object
      var linked = {};
      Object.keys(direct).forEach(function (o) {
        Object.keys(ix.adj[o] || {}).forEach(function (nb) {
          if (!direct[nb]) linked[nb] = true;
        });
      });

      // Backend lookup, from the search box: if the query is an address (a node
      // / member IP that was hit, optionally `%rd` and `:port`), surface the
      // virtual servers that can route to it — including pools selected via an
      // iRule or LTM policy (which are more than one hop away in the graph).
      if (qParsed) {
        var matchPools = {};
        ix.d.pools.forEach(function (p) {
          (p.members || []).forEach(function (m) {
            var mn = toNet(m.address);
            if (mn && ipOverlap(qNet, mn) &&
              (!qParsed.port || String(m.port) === String(qParsed.port))) {
              matchPools[p.fullPath] = true;
            }
          });
        });
        if (Object.keys(matchPools).length) {
          var reaching = virtualsReaching(ix, matchPools);
          Object.keys(reaching).forEach(function (o) { if (!direct[o]) linked[o] = true; });
        }
      }

      info.forEach(function (ri) {
        var show = ri.tm || (ri.oid && linked[ri.oid]);
        ri.row.classList.toggle("hidden", !show);
        ri.row.classList.toggle("search-hit", !!ri.tm);            // direct match
        ri.row.classList.toggle("search-linked", !!(show && !ri.tm)); // linked only
        var d = detailOf(ri.row); if (d) d.classList.toggle("hidden", !show);
      });
    }

    search.addEventListener("input", run);
    // re-apply when switching devices so the filter follows the visible device
    document.querySelectorAll(".dev-tab").forEach(function (b) {
      b.addEventListener("click", function () { setTimeout(run, 0); });
    });
  })();

  // ---- Built-in / system objects: hidden from the flat catalog tables by
  //      default (they're shown wherever they're directly linked — on a virtual,
  //      in a reference table, in a diagram). A per-device toggle reveals them. --
  (function initSystemToggle() {
    MODEL.devices.forEach(function (d, di) {
      var deviceEl = document.querySelector('.device[data-dev="' + di + '"]') ||
        document.querySelectorAll(".device")[di];
      if (!deviceEl) return;
      var defaults = {};
      ["virtuals", "pools", "nodes", "monitors", "rules", "dataGroups",
        "profiles", "policies", "snatpools", "persistence", "certificates"].forEach(function (k) {
        (d[k] || []).forEach(function (o) { if (o && o.isDefault && o.fullPath) defaults[o.fullPath] = true; });
      });
      deviceEl.querySelectorAll(".grid tbody tr.searchable").forEach(function (row) {
        var m = (row.getAttribute("data-search") || "").match(/(\/[^\s]+)/);
        if (m && defaults[m[1]]) {
          row.classList.add("sys-row");
          var det = row.nextElementSibling;
          if (det && det.classList.contains("detail")) det.classList.add("sys-row");
        }
      });
      var chk = deviceEl.querySelector(".show-system");
      if (chk) chk.addEventListener("change", function () {
        deviceEl.classList.toggle("show-system-on", chk.checked);
      });
    });
  })();

  // ---- App bundling: group virtuals into a named "app", saved in the browser,
  //      with a combined flow diagram and every object it touches shown with a
  //      syntax-highlighted config and cross-links. ------------------------------
  (function initApps() {
    var LS_PREFIX = "f5report:apps:";
    function deviceKey(d) { return String(d.name || d.uri || "device").replace(/\s+/g, "_"); }
    function loadApps(d) {
      try { return JSON.parse(localStorage.getItem(LS_PREFIX + deviceKey(d)) || "[]") || []; }
      catch (e) { return []; }
    }
    function storeApps(d, apps) {
      try { localStorage.setItem(LS_PREFIX + deviceKey(d), JSON.stringify(apps)); return true; }
      catch (e) { return false; }
    }

    function blockId(oid) { return "appobj:" + oid; }
    function scrollToBlock(host, oid) {
      var el = host.querySelector('[id="' + (window.CSS && CSS.escape ? CSS.escape(blockId(oid)) : blockId(oid)) + '"]');
      if (!el) { var all = host.querySelectorAll(".app-obj"); for (var k = 0; k < all.length; k++) { if (all[k].getAttribute("data-oid") === oid) { el = all[k]; break; } } }
      if (el) { el.scrollIntoView({ behavior: "smooth", block: "start" }); el.classList.add("app-obj-flash"); setTimeout(function () { el.classList.remove("app-obj-flash"); }, 1200); }
    }

    // Everything an app's virtual servers forward-reach (their pools, nodes,
    // monitors, iRules, and the pools those iRules use) — the objects that make
    // up the application, not the whole connected component.
    function appScope(ix, startOids) {
      var seen = {}, queue = [];
      startOids.forEach(function (o) { if (ix.byOid[o]) { seen[o] = true; queue.push(o); } });
      while (queue.length) {
        var cur = queue.shift();
        // Default objects are terminal (see neighborhood): include them as leaves
        // but don't expand, so a shared default profile can't pull in other apps.
        if (isDefaultNode(ix, cur)) continue;
        (ix.fadj[cur] || []).forEach(function (nb) { if (!seen[nb]) { seen[nb] = true; queue.push(nb); } });
      }
      return seen;
    }

    // Render one app's detail: flow diagram + object blocks.
    function renderApp(deviceEl, ix, app, host) {
      var vsOids = (app.oids || []).filter(function (o) { return ix.byOid[o]; });
      var scope = appScope(ix, vsOids);
      var oids = Object.keys(scope);
      var inApp = {};
      oids.forEach(function (o) { var n = ix.byOid[o]; if (n) inApp[n.fullPath] = true; });

      var missing = (app.oids || []).length - vsOids.length;
      var html = '<div class="app-detail">';
      html += '<div class="app-detail-head"><h3>' + esc(app.name) + "</h3>" +
        '<span class="muted">' + vsOids.length + " virtual server(s), " +
        oids.length + " objects" + (missing ? ", " + missing + " not in this device" : "") + "</span></div>";
      // VS chips (scroll to block)
      html += '<div class="app-vs-chips">';
      vsOids.forEach(function (o) { html += '<button class="app-chip" data-goto-oid="' + o + '">' + esc(ix.byOid[o].name) + "</button>"; });
      html += "</div>";
      // traffic-flow pipeline, one per virtual server (client stack → iRule
      // events by priority → LB → server stack → node)
      html += '<div class="app-flow"><div class="app-flow-title">Traffic flow</div>';
      vsOids.forEach(function (o) {
        html += '<div class="app-pipe"><div class="app-pipe-vs">' + esc(ix.byOid[o].name) +
          '</div><div class="app-pipe-diagram diag-host" data-vs="' + o + '">building…</div></div>';
      });
      html += "</div>";
      // object blocks
      html += '<div class="app-objs">';
      var order = { vs: 0, pool: 1, node: 2, mon: 3, rule: 4, prof: 5, persist: 6, policy: 7, snat: 8, dg: 9 };
      oids.sort(function (a, b) {
        var na = ix.byOid[a], nb = ix.byOid[b];
        return (order[na.type] || 9) - (order[nb.type] || 9) || na.name.localeCompare(nb.name);
      });
      oids.forEach(function (o) {
        var n = ix.byOid[o];
        html += '<div class="app-obj" id="' + blockId(o) + '" data-oid="' + o + '">';
        html += '<div class="app-obj-head"><span class="app-obj-type">' + esc(TYPE_LABEL[n.type] || n.type) + '</span> ' +
          '<span class="app-obj-name mono">' + esc(n.fullPath) + '</span>' +
          '<button class="app-obj-drawer" data-oid="' + o + '" title="Open in inspector">⤢</button></div>';
        if (n.type === "rule") {
          var r = ruleByPath(ix, n.fullPath);
          if (r) {
            if (r.flowchart) html += '<div class="app-obj-flow diag-host" data-flow="' + esc(encodeURIComponent(r.flowchart)) + '">flow…</div>';
            html += '<pre class="code tcl">' + (r.bodyHtml || esc(r.body || "")) + "</pre>";
          }
        } else {
          var stanza = stanzaFor(ix.d.configText, n.fullPath);
          html += '<pre class="code conf">' + (stanza ? highlightConf(stanza, function (fp) { return !!inApp[fp]; }) : '<span class="muted">config not found</span>') + "</pre>";
        }
        html += "</div>";
      });
      html += "</div></div>";
      host.innerHTML = html;

      // render one traffic-flow pipeline per virtual server
      host.querySelectorAll(".app-pipe-diagram[data-vs]").forEach(function (ph) {
        var built = buildTrafficPipeline(ix, ph.getAttribute("data-vs"));
        if (!built || !window.mermaid) { ph.textContent = "(pipeline unavailable)"; return; }
        var id = "apppipe" + (_rid++);
        mermaid.render(id, built.def).then(function (res) {
          ph.innerHTML = res.svg;
          var svg = ph.querySelector("svg"); if (svg) { svg.style.maxWidth = "100%"; svg.style.height = "auto"; }
        }).catch(function (e) { ph.innerHTML = '<div class="diag-err">pipeline error: ' + esc(e.message || e) + "</div>"; });
      });
      // render each iRule control-flow flowchart
      host.querySelectorAll(".app-obj-flow[data-flow]").forEach(function (fh) {
        var def = decodeURIComponent(fh.getAttribute("data-flow") || "");
        if (!def || !window.mermaid) { fh.textContent = ""; return; }
        var id = "appflow" + (_rid++);
        mermaid.render(id, def).then(function (res) { fh.innerHTML = res.svg; }).catch(function () { fh.textContent = "(flowchart unavailable)"; });
      });
      // cross-links: config paths + VS chips + drawer buttons
      host.querySelectorAll(".conf-link[data-goto]").forEach(function (a) {
        a.addEventListener("click", function (e) { e.preventDefault(); var fp = a.getAttribute("data-goto"); var oid = ix.byPath[fp]; if (oid) scrollToBlock(host, oid); });
      });
      host.querySelectorAll(".app-chip[data-goto-oid]").forEach(function (b) {
        b.addEventListener("click", function () { scrollToBlock(host, b.getAttribute("data-goto-oid")); });
      });
      host.querySelectorAll(".app-obj-drawer[data-oid]").forEach(function (b) {
        b.addEventListener("click", function () { openDrawer(ix, b.getAttribute("data-oid")); });
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
        html += '<p class="muted">No saved apps yet. On the Virtual Servers tab, tick the virtual servers that make up an application, give it a name and hit <strong>Save app</strong>. Apps are stored in this browser (localStorage) and travel with this report file.</p>';
      } else {
        apps.forEach(function (app, i) {
          html += '<div class="app-card"><button class="app-open" data-i="' + i + '">' + esc(app.name) + "</button>" +
            '<span class="muted">' + (app.oids || []).length + " VS</span>" +
            '<button class="app-del" data-i="' + i + '" title="Delete app">✕</button></div>';
        });
      }
      html += '</div><div class="apps-detail"></div>';
      view.innerHTML = html;
      view.querySelectorAll(".app-open").forEach(function (b) {
        b.addEventListener("click", function () {
          view.querySelectorAll(".app-open").forEach(function (x) { x.classList.remove("active"); });
          b.classList.add("active");
          renderApp(deviceEl, ix, apps[parseInt(b.dataset.i, 10)], view.querySelector(".apps-detail"));
        });
      });
      view.querySelectorAll(".app-del").forEach(function (b) {
        b.addEventListener("click", function () {
          var a = loadApps(ix.d); a.splice(parseInt(b.dataset.i, 10), 1); storeApps(ix.d, a);
          renderAppsPanel(deviceEl, ix);
        });
      });
    }

    MODEL.devices.forEach(function (d, di) {
      var deviceEl = document.querySelector('.device[data-dev="' + di + '"]') ||
        document.querySelectorAll(".device")[di];
      if (!deviceEl) return;
      var ix = IDX[di];
      var vpanel = deviceEl.querySelector('.panel[data-panel="virtuals"]');
      var picks = function () { return vpanel ? vpanel.querySelectorAll(".app-pick") : []; };
      var countEl = deviceEl.querySelector(".app-count");
      function refreshCount() {
        var n = 0; picks().forEach(function (c) { if (c.checked) n++; });
        if (countEl) countEl.textContent = n + " selected";
      }
      picks().forEach(function (c) { c.addEventListener("change", refreshCount); });
      var all = deviceEl.querySelector(".app-pick-all");
      if (all) all.addEventListener("change", function () {
        picks().forEach(function (c) { var row = c.closest("tr"); if (!row || row.classList.contains("part-hidden") || row.classList.contains("hidden")) return; c.checked = all.checked; });
        refreshCount();
      });
      var saveBtn = deviceEl.querySelector(".app-save");
      var nameInp = deviceEl.querySelector(".app-name");
      if (saveBtn) saveBtn.addEventListener("click", function () {
        var oids = []; picks().forEach(function (c) { if (c.checked) oids.push(c.dataset.vs); });
        var name = (nameInp && nameInp.value.trim()) || "";
        if (!oids.length) { if (nameInp) nameInp.placeholder = "tick some virtual servers first"; return; }
        if (!name) { if (nameInp) { nameInp.focus(); nameInp.placeholder = "name the app first"; } return; }
        var apps = loadApps(ix.d);
        var existing = apps.filter(function (a) { return a.name === name; })[0];
        if (existing) existing.oids = oids; else apps.push({ name: name, oids: oids });
        storeApps(ix.d, apps);
        if (nameInp) nameInp.value = "";
        picks().forEach(function (c) { c.checked = false; }); refreshCount();
        renderAppsPanel(deviceEl, ix);
        // jump to the Apps tab
        var tab = deviceEl.querySelector('.tab[data-panel="apps"]'); if (tab) tab.click();
      });
      renderAppsPanel(deviceEl, ix);
    });
  })();
})();
