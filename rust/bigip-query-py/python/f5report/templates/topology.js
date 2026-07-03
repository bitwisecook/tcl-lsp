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
    mermaid.initialize({
      startOnLoad: false, securityLevel: "loose", theme: "neutral",
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

  // ---- per-device index --------------------------------------------------
  function indexDevice(d) {
    var byOid = {}, adj = {}, short = {}, unshort = {}, i = 0;
    d.graph.nodes.forEach(function (n) {
      byOid[n.oid] = n; adj[n.oid] = {};
      var sid = "N" + (i++); short[n.oid] = sid; unshort[sid] = n.oid;
    });
    var edgesByPair = {};
    d.graph.edges.forEach(function (e) {
      if (!(e.from in adj) || !(e.to in adj)) return;
      adj[e.from][e.to] = true; adj[e.to][e.from] = true;
      edgesByPair[short[e.from] + "|" + short[e.to]] = e;
      edgesByPair[short[e.to] + "|" + short[e.from]] = e;
    });
    return { d: d, byOid: byOid, adj: adj, short: short, unshort: unshort, edgesByPair: edgesByPair };
  }
  var IDX = MODEL.devices.map(indexDevice);

  function activeDeviceIndex() {
    var el = document.querySelector(".device.active");
    return el ? parseInt(el.dataset.dev, 10) : 0;
  }

  function esc(s) { return String(s).replace(/"/g, "&quot;").replace(/\n/g, " "); }
  // Edge labels sit in Mermaid's `|...|` syntax, which chokes on (){}[]|" —
  // strip those to keep the flowchart parseable.
  function escLbl(s) { return String(s).replace(/[()|{}\[\]"]/g, "").replace(/\n/g, " "); }

  // BFS the undirected connected component containing `startOids`, bounded by
  // `depth` (Infinity = whole component).
  function neighborhood(ix, startOids, depth) {
    var seen = {}, frontier = [], d = 0;
    startOids.forEach(function (o) { if (ix.byOid[o]) { seen[o] = true; frontier.push(o); } });
    while (frontier.length && d < depth) {
      var next = [];
      frontier.forEach(function (o) {
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
    var drawer = document.getElementById("objDrawer");
    var body = drawer.querySelector(".drawer-body");
    drawer.querySelector(".drawer-title").innerHTML =
      '<span class="tag ' + n.type + '">' + TYPE_LABEL[n.type] + "</span> " + esc(n.name);
    drawer.querySelector(".drawer-sub").textContent = n.fullPath;

    var parts = [];
    if (n.type === "vs") parts.push(vsDetail(ix, oid));
    else if (n.type === "pool") parts.push(poolDetail(ix, oid));
    parts.push('<h4>Neighbourhood</h4><div class="diag-host drawer-diag"></div>');
    if (n.type === "vs") parts.push('<h4>Processing flow</h4><div class="diag-host flow-diag"></div>');
    body.innerHTML = parts.join("");

    var oids = Object.keys(neighborhood(ix, [oid], 2));
    renderInto(body.querySelector(".drawer-diag"),
      buildFlowchart(ix, oids, { dir: "LR" }), ix, function (o) { openDrawer(ix, o); });
    if (n.type === "vs") {
      renderInto(body.querySelector(".flow-diag"), buildFlow(ix, oid), ix,
        function (o) { openDrawer(ix, o); });
    }
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

    var staticProfiles = (v.profiles || []).map(function (p) {
      return '<span class="tag prof">' + esc(p.split("/").pop()) + "</span>";
    }).join("") || '<span class="muted">none</span>';

    var dyn = (v.dynamicProfiles || []).map(function (a) {
      return '<span class="tag amber" title="via iRule ' + esc(a.rule) + '">' +
        esc(a.effect) + (a.arg ? " " + esc(a.arg) : "") + " · " + esc(a.category) + "</span>";
    }).join("");
    var dynBlock = dyn
      ? '<h4>Dynamic (iRule-driven) changes</h4><div class="tagwrap">' + dyn + "</div>"
      : "";

    return '<h4>Listener</h4>' + meta +
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
    root.querySelectorAll("[data-oid]").forEach(function (el) {
      if (el._wired) return; el._wired = true;
      el.classList.add("objlink");
      el.addEventListener("click", function (ev) {
        ev.preventDefault(); ev.stopPropagation();
        openDrawer(ix, el.dataset.oid);
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

  // drawer close handlers
  var closeBtn = document.querySelector("#objDrawer .drawer-close");
  if (closeBtn) closeBtn.addEventListener("click", closeDrawer);
  var scrim = document.getElementById("drawerScrim");
  if (scrim) scrim.addEventListener("click", closeDrawer);
  document.addEventListener("keydown", function (e) { if (e.key === "Escape") closeDrawer(); });
})();
