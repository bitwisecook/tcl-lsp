// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/bigip-report/shared/ts — DO NOT EDIT; edit the .ts source.
"use strict";
(() => {
  // ts/elk-graph.ts
  (function() {
    "use strict";
    var NS = "http://www.w3.org/2000/svg";
    var PAD_X = 12, PAD_Y = 8, LINE_H = 15;
    function el(tag, attrs) {
      var e = document.createElementNS(NS, tag);
      if (attrs) Object.keys(attrs).forEach(function(k) {
        e.setAttribute(k, attrs[k]);
      });
      return e;
    }
    function measurer() {
      var svg = el("svg", { class: "elk-measure" });
      svg.style.cssText = "position:absolute;left:-99999px;top:0;visibility:hidden";
      document.body.appendChild(svg);
      return {
        width: function(line) {
          var t = el("text", { class: "elk-label" });
          t.textContent = line || " ";
          svg.appendChild(t);
          var w = t.getBBox().width;
          svg.removeChild(t);
          return w;
        },
        done: function() {
          document.body.removeChild(svg);
        }
      };
    }
    function path(points) {
      return "M" + points.map(function(p) {
        return p.x + "," + p.y;
      }).join(" L");
    }
    async function render(host, data, opts) {
      opts = opts || {};
      host.textContent = "";
      if (!window.ELK) {
        host.textContent = "(diagram engine unavailable)";
        return;
      }
      var nodes = data && data.nodes || [];
      var edges = data && data.edges || [];
      if (!nodes.length) {
        host.textContent = "(no linked objects)";
        return;
      }
      var dir = opts.dir || "RIGHT";
      var vertical = dir === "DOWN" || dir === "UP";
      var OUT_SIDE = vertical ? "SOUTH" : "EAST";
      var IN_SIDE = vertical ? "NORTH" : "WEST";
      var outCount = {}, inCount = {};
      nodes.forEach(function(n) {
        outCount[n.id] = 0;
        inCount[n.id] = 0;
      });
      edges.forEach(function(e) {
        if (outCount[e.from] != null) outCount[e.from]++;
        if (inCount[e.to] != null) inCount[e.to]++;
      });
      var m = measurer();
      var meta = {};
      var children = nodes.map(function(n) {
        var lines = String(n.label == null ? n.id : n.label).split("\n");
        var w = 0;
        lines.forEach(function(ln) {
          w = Math.max(w, m.width(ln));
        });
        var width = Math.max(Math.ceil(w) + PAD_X * 2, 46);
        var height = lines.length * LINE_H + PAD_Y * 2;
        meta[n.id] = { lines, cls: n.cls || "", width, height };
        var ports = [];
        for (var oi = 0; oi < outCount[n.id]; oi++) {
          ports.push({ id: n.id + "__o" + oi, layoutOptions: { "elk.port.side": OUT_SIDE } });
        }
        for (var ii = 0; ii < inCount[n.id]; ii++) {
          ports.push({ id: n.id + "__i" + ii, layoutOptions: { "elk.port.side": IN_SIDE } });
        }
        return {
          id: n.id,
          width,
          height,
          ports,
          layoutOptions: {
            "elk.portConstraints": "FIXED_SIDE",
            "elk.portAlignment.default": "DISTRIBUTED"
          }
        };
      });
      var outUsed = {}, inUsed = {};
      nodes.forEach(function(n) {
        outUsed[n.id] = 0;
        inUsed[n.id] = 0;
      });
      var elkEdges = edges.map(function(e, i) {
        var op = e.from + "__o" + (outUsed[e.from] != null ? outUsed[e.from]++ : 0);
        var ip = e.to + "__i" + (inUsed[e.to] != null ? inUsed[e.to]++ : 0);
        var eo = { id: "e" + i, sources: [op], targets: [ip] };
        if (e.label) {
          eo.labels = [{ text: e.label, width: Math.ceil(m.width(e.label)) + 6, height: 13 }];
        }
        return eo;
      });
      m.done();
      var graph = {
        id: "root",
        layoutOptions: {
          "elk.algorithm": "layered",
          "elk.direction": dir,
          "elk.edgeRouting": "ORTHOGONAL",
          "elk.layered.spacing.baseValue": "22",
          "elk.layered.spacing.nodeNodeBetweenLayers": "70",
          "elk.spacing.nodeNode": "26",
          "elk.spacing.edgeEdge": "14",
          "elk.spacing.edgeNode": "18",
          "elk.spacing.portPort": "14",
          "elk.layered.nodePlacement.strategy": "BRANDES_KOEPF",
          "elk.layered.nodePlacement.bk.fixedAlignment": "BALANCED",
          "elk.layered.crossingMinimization.strategy": "LAYER_SWEEP",
          "elk.edgeLabels.placement": "CENTER",
          "elk.spacing.edgeLabel": "3"
        },
        children,
        edges: elkEdges
      };
      var res;
      try {
        res = await new window.ELK().layout(graph);
      } catch (err) {
        host.textContent = "(diagram layout error)";
        return;
      }
      var W = Math.ceil(res.width) + 2, H = Math.ceil(res.height) + 2;
      var svg = el("svg", { class: "elk-svg", viewBox: "-1 -1 " + W + " " + H, width: W, height: H });
      svg.style.maxWidth = "100%";
      svg.style.height = "auto";
      var defs = el("defs");
      var marker = el("marker", {
        id: "elk-arrow",
        markerWidth: "9",
        markerHeight: "9",
        refX: "8",
        refY: "4",
        orient: "auto",
        markerUnits: "userSpaceOnUse"
      });
      marker.appendChild(el("path", { d: "M0,0 L8,4 L0,8 z", class: "elk-arrowhead" }));
      defs.appendChild(marker);
      svg.appendChild(defs);
      (res.edges || []).forEach(function(e) {
        (e.sections || []).forEach(function(s) {
          var pts = [s.startPoint].concat(s.bendPoints || []).concat([s.endPoint]);
          svg.appendChild(el("path", { d: path(pts), class: "elk-edge", "marker-end": "url(#elk-arrow)" }));
        });
        var lbl = e.labels && e.labels[0];
        if (lbl && lbl.x != null) {
          var g = el("g", { class: "elk-edge-label" });
          g.appendChild(el("rect", {
            x: lbl.x - 2,
            y: lbl.y - 1,
            width: (lbl.width || 0) + 4,
            height: (lbl.height || 12) + 2,
            rx: 2,
            class: "elk-edge-label-bg"
          }));
          var t = el("text", { x: lbl.x, y: lbl.y + (lbl.height || 12) - 2, class: "elk-edge-label-t" });
          t.textContent = lbl.text;
          g.appendChild(t);
          svg.appendChild(g);
        }
      });
      (res.children || []).forEach(function(c) {
        var info = meta[c.id];
        if (!info) return;
        var g = el("g", { class: "elk-node " + info.cls, transform: "translate(" + c.x + "," + c.y + ")" });
        g.appendChild(el("rect", { x: 0, y: 0, rx: 2, width: c.width, height: c.height, class: "elk-node-box" }));
        var t = el("text", { x: c.width / 2, y: 0, class: "elk-node-text", "text-anchor": "middle" });
        info.lines.forEach(function(line, i) {
          var span = el("tspan", { x: c.width / 2, dy: i === 0 ? PAD_Y + LINE_H - 4 : LINE_H });
          span.textContent = line;
          t.appendChild(span);
        });
        g.appendChild(t);
        svg.appendChild(g);
      });
      host.appendChild(svg);
    }
    window.ElkGraph = { render };
  })();
})();
