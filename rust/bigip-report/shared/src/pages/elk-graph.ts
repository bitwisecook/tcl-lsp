// @ts-nocheck -- migrated verbatim from JS; typed incrementally, not in the restructure commit.
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

// f5report — a tiny self-contained ELK → SVG renderer for orthogonal diagrams.
//
// Mermaid draws edges by interpolating a curve through a couple of points, so
// it cannot do real orthogonal routing (parallel edges overlap and arrowheads
// miss the node border). This renderer drives elkjs directly: ELK computes the
// layered node placement AND the orthogonal edge routes (bend points, and end
// points seated on the target node's border), and we draw the SVG from that —
// so every connector is a true right angle, edges are separated into channels,
// and each arrowhead points into its rectangle.
//
// `ElkGraph.render(hostEl, {nodes:[{id,label,cls}], edges:[{from,to,label}]},
// opts)` returns a Promise. Node classes drive styling via CSS
// (`.elk-node.<cls> rect { … }`). No dependency beyond the vendored elkjs.
(function () {
  "use strict";
  var NS = "http://www.w3.org/2000/svg";
  var PAD_X = 12, PAD_Y = 8, LINE_H = 15;

  function el(tag, attrs) {
    var e = document.createElementNS(NS, tag);
    if (attrs) Object.keys(attrs).forEach(function (k) { e.setAttribute(k, attrs[k]); });
    return e;
  }

  // Measure text with a throwaway, offscreen SVG (getBBox needs it laid out).
  function measurer() {
    var svg = el("svg", { class: "elk-measure" });
    svg.style.cssText = "position:absolute;left:-99999px;top:0;visibility:hidden";
    document.body.appendChild(svg);
    return {
      width: function (line) {
        var t = el("text", { class: "elk-label" });
        t.textContent = line || " ";
        svg.appendChild(t);
        var w = t.getBBox().width;
        svg.removeChild(t);
        return w;
      },
      done: function () { document.body.removeChild(svg); },
    };
  }

  function path(points) {
    return "M" + points.map(function (p) { return p.x + "," + p.y; }).join(" L");
  }

  async function render(host, data, opts) {
    opts = opts || {};
    host.textContent = "";
    if (!window.ELK) { host.textContent = "(diagram engine unavailable)"; return; }
    var nodes = (data && data.nodes) || [];
    var edges = (data && data.edges) || [];
    if (!nodes.length) { host.textContent = "(no linked objects)"; return; }

    var dir = opts.dir || "RIGHT";
    var vertical = dir === "DOWN" || dir === "UP";
    var OUT_SIDE = vertical ? "SOUTH" : "EAST";
    var IN_SIDE = vertical ? "NORTH" : "WEST";

    // Edges attach through explicit ports so their attachment points ("magnets")
    // sit at evenly-spaced positions on each box side (portAlignment DISTRIBUTED)
    // rather than wherever the router happens to meet the border.
    var outCount = {}, inCount = {};
    nodes.forEach(function (n) { outCount[n.id] = 0; inCount[n.id] = 0; });
    edges.forEach(function (e) {
      if (outCount[e.from] != null) outCount[e.from]++;
      if (inCount[e.to] != null) inCount[e.to]++;
    });

    var m = measurer();
    // Size each node to its (possibly multi-line) label, and give it one port
    // per incident edge on the appropriate side.
    var meta = {};
    var children = nodes.map(function (n) {
      var lines = String(n.label == null ? n.id : n.label).split("\n");
      var w = 0;
      lines.forEach(function (ln) { w = Math.max(w, m.width(ln)); });
      var width = Math.max(Math.ceil(w) + PAD_X * 2, 46);
      var height = lines.length * LINE_H + PAD_Y * 2;
      meta[n.id] = { lines: lines, cls: n.cls || "", width: width, height: height };
      var ports = [];
      for (var oi = 0; oi < outCount[n.id]; oi++) {
        ports.push({ id: n.id + "__o" + oi, layoutOptions: { "elk.port.side": OUT_SIDE } });
      }
      for (var ii = 0; ii < inCount[n.id]; ii++) {
        ports.push({ id: n.id + "__i" + ii, layoutOptions: { "elk.port.side": IN_SIDE } });
      }
      return {
        id: n.id, width: width, height: height, ports: ports,
        layoutOptions: {
          "elk.portConstraints": "FIXED_SIDE",
          "elk.portAlignment.default": "DISTRIBUTED",
        },
      };
    });
    var outUsed = {}, inUsed = {};
    nodes.forEach(function (n) { outUsed[n.id] = 0; inUsed[n.id] = 0; });
    // Per-edge presentation (dashed / extra class / endpoints) kept alongside
    // the ELK edge so the draw pass can style it — ELK's returned edges carry
    // only geometry, so we look this up by the edge's `e<i>` id.
    var edgeMeta = {};
    var elkEdges = edges.map(function (e, i) {
      var op = e.from + "__o" + (outUsed[e.from] != null ? outUsed[e.from]++ : 0);
      var ip = e.to + "__i" + (inUsed[e.to] != null ? inUsed[e.to]++ : 0);
      var eo = { id: "e" + i, sources: [op], targets: [ip] };
      if (e.label) {
        eo.labels = [{ text: e.label, width: Math.ceil(m.width(e.label)) + 6, height: 13 }];
      }
      edgeMeta[eo.id] = { cls: e.cls || "", dashed: !!e.dashed, from: e.from, to: e.to };
      return eo;
    });
    m.done();

    // Even spacing everywhere: one base value drives the node/edge/port gaps so
    // the columns, the parallel edge lanes, and the port pitch all read evenly;
    // BRANDES_KOEPF placement keeps nodes aligned across layers.
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
        "elk.spacing.edgeLabel": "3",
      },
      children: children,
      edges: elkEdges,
    };

    var res;
    try {
      res = await new window.ELK().layout(graph);
    } catch (err) {
      host.textContent = "(diagram layout error)";
      return;
    }

    var W = Math.ceil(res.width) + 2, H = Math.ceil(res.height) + 2;
    var svg = el("svg", {
      class: "elk-svg" + (opts.svgClass ? " " + opts.svgClass : ""),
      viewBox: "-1 -1 " + W + " " + H, width: W, height: H,
    });
    svg.style.maxWidth = "100%";
    svg.style.height = "auto";
    var defs = el("defs");
    var marker = el("marker", {
      id: "elk-arrow", markerWidth: "9", markerHeight: "9",
      refX: "8", refY: "4", orient: "auto", markerUnits: "userSpaceOnUse",
    });
    marker.appendChild(el("path", { d: "M0,0 L8,4 L0,8 z", class: "elk-arrowhead" }));
    defs.appendChild(marker);
    svg.appendChild(defs);

    // Edges (drawn under the nodes so a route never covers a box).
    (res.edges || []).forEach(function (e) {
      var em = edgeMeta[e.id] || {};
      var ecls = "elk-edge" + (em.dashed ? " elk-edge-dashed" : "") + (em.cls ? " " + em.cls : "");
      (e.sections || []).forEach(function (s) {
        var pts = [s.startPoint].concat(s.bendPoints || []).concat([s.endPoint]);
        var pe = el("path", { d: path(pts), class: ecls, "marker-end": "url(#elk-arrow)" });
        if (em.from != null) pe.setAttribute("data-from", em.from);
        if (em.to != null) pe.setAttribute("data-to", em.to);
        svg.appendChild(pe);
      });
      var lbl = e.labels && e.labels[0];
      if (lbl && lbl.x != null) {
        var g = el("g", { class: "elk-edge-label" });
        g.appendChild(el("rect", {
          x: lbl.x - 2, y: lbl.y - 1, width: (lbl.width || 0) + 4, height: (lbl.height || 12) + 2,
          rx: 2, class: "elk-edge-label-bg",
        }));
        var t = el("text", { x: lbl.x, y: lbl.y + (lbl.height || 12) - 2, class: "elk-edge-label-t" });
        t.textContent = lbl.text;
        g.appendChild(t);
        svg.appendChild(g);
      }
    });

    // Nodes.
    (res.children || []).forEach(function (c) {
      var info = meta[c.id];
      if (!info) return;
      var g = el("g", { class: "elk-node " + info.cls, "data-nid": c.id, transform: "translate(" + c.x + "," + c.y + ")" });
      g.appendChild(el("rect", { x: 0, y: 0, rx: 2, width: c.width, height: c.height, class: "elk-node-box" }));
      var t = el("text", { x: c.width / 2, y: 0, class: "elk-node-text", "text-anchor": "middle" });
      info.lines.forEach(function (line, i) {
        var span = el("tspan", { x: c.width / 2, dy: (i === 0 ? PAD_Y + LINE_H - 4 : LINE_H) });
        span.textContent = line;
        t.appendChild(span);
      });
      g.appendChild(t);
      svg.appendChild(g);
    });

    host.appendChild(svg);
  }

  window.ElkGraph = { render: render };
})();
