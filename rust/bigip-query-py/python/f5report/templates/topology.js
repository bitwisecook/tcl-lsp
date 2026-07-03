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

  // ---- Listener matching -------------------------------------------------
  function ipToInt(ip) {
    if (ip.indexOf(":") >= 0) return null; // v6 handled separately (coarse)
    var p = ip.split(".").map(Number);
    if (p.length !== 4 || p.some(function (x) { return isNaN(x) || x < 0 || x > 255; })) return null;
    return ((p[0] << 24) >>> 0) + (p[1] << 16) + (p[2] << 8) + p[3];
  }
  function inNet(ipInt, netIp, prefix) {
    if (ipInt == null) return true;
    var netInt = ipToInt(netIp);
    if (netInt == null) return true;
    if (prefix <= 0) return true;
    var mask = prefix >= 32 ? 0xffffffff : (~((1 << (32 - prefix)) - 1)) >>> 0;
    return ((ipInt & mask) >>> 0) === ((netInt & mask) >>> 0);
  }

  function matchListeners(ix, q) {
    // q: {dst, port, proto, rd, vlan, src}
    var dstInt = ipToInt(q.dst);
    var out = [];
    ix.d.virtuals.forEach(function (v) {
      var L = v.listener || {}; if (v.disabled) return;
      if (q.rd !== "" && String(L.routeDomain) !== String(q.rd)) return;
      // destination address containment (wildcards always match)
      if (!L.anyAddr && q.dst && !inNet(dstInt, L.address, L.prefix)) return;
      // port
      if (q.port !== "" && L.port !== 0 && String(L.port) !== String(q.port)) return;
      // protocol
      if (q.proto && q.proto !== "any" && L.protocol && L.protocol !== "any" &&
        L.protocol !== q.proto) return;
      // vlan
      if (q.vlan && L.vlans && L.vlans.length) {
        var on = L.vlans.indexOf(q.vlan) >= 0;
        if (L.vlansEnabled && !on) return;
        if (L.vlansDisabled && on) return;
      }
      // source containment
      if (q.src && q.src !== "0.0.0.0/0" && q.src !== "::/0") {
        var sp = q.src.split("/"); // coarse: skip if v6
        if (sp[0].indexOf(":") < 0 && L.source) {
          var ls = L.source.split("/");
          // require the listener source to contain the query's source base
          if (!inNet(ipToInt(sp[0]), ls[0], parseInt(ls[1] || "0", 10))) return;
        }
      }
      out.push(v);
    });
    // specificity: dest prefix desc, then port specific, then source prefix desc,
    // then vlan-scoped first.
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

  function initListener(deviceEl, ix) {
    var panel = deviceEl.querySelector('.panel[data-panel="listener"]');
    if (!panel) return;
    var f = {
      src: panel.querySelector(".lm-src"), dst: panel.querySelector(".lm-dst"),
      port: panel.querySelector(".lm-port"), proto: panel.querySelector(".lm-proto"),
      vlan: panel.querySelector(".lm-vlan"), rd: panel.querySelector(".lm-rd"),
    };
    var out = panel.querySelector(".lm-results");
    // populate vlan options
    var vlans = {};
    ix.d.virtuals.forEach(function (v) { (v.listener.vlans || []).forEach(function (x) { vlans[x] = 1; }); });
    f.vlan.innerHTML = '<option value="">any VLAN</option>' +
      Object.keys(vlans).sort().map(function (x) { return '<option>' + esc(x) + "</option>"; }).join("");

    function run() {
      var q = {
        src: f.src.value.trim() || "0.0.0.0/0", dst: f.dst.value.trim(),
        port: f.port.value.trim(), proto: f.proto.value, vlan: f.vlan.value,
        rd: f.rd.value.trim(),
      };
      var matches = matchListeners(ix, q);
      if (!matches.length) { out.innerHTML = '<div class="diag-empty">No virtual server matches this flow.</div>'; return; }
      // group into specificity tiers (same dest-prefix + port-specificity aligned horizontally)
      var tiers = [];
      matches.forEach(function (v) {
        var key = v.listener.prefix + ":" + (v.listener.port !== 0 ? 1 : 0) + ":" + v.listener.sourcePrefix;
        var t = tiers.find(function (t) { return t.key === key; });
        if (!t) { t = { key: key, prefix: v.listener.prefix, port: v.listener.port, vs: [] }; tiers.push(t); }
        t.vs.push(v);
      });
      var html = ['<div class="lm-note">Winner (most specific) at the top. Virtual servers at the same specificity are shown side by side.</div>'];
      html.push('<div class="tiers">');
      tiers.forEach(function (t, ti) {
        html.push('<div class="tier' + (ti === 0 ? " winner" : "") + '">');
        html.push('<div class="tier-key">/' + t.prefix + ' · ' + (t.port ? "port " + t.port : "any port") + '</div>');
        html.push('<div class="tier-vs">');
        t.vs.forEach(function (v) {
          var L = v.listener;
          html.push('<button class="lm-card objlink" data-oid="vs:' + esc(v.fullPath) + '">' +
            '<span class="lm-name">' + esc(v.name) + (ti === 0 && t.vs.length === 1 ? ' <span class="tag green">selected</span>' : '') + '</span>' +
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
      out.querySelectorAll(".objlink").forEach(function (b) {
        b.addEventListener("click", function () { openDrawer(ix, b.dataset.oid); });
      });
    }
    panel.querySelector(".lm-run").addEventListener("click", run);
    panel.querySelectorAll(".lm-field").forEach(function (el) {
      el.addEventListener("keydown", function (e) { if (e.key === "Enter") run(); });
    });
    // default source per family already 0.0.0.0/0
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
