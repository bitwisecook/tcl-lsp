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

    var m = measurer();
    // Size each node to its (possibly multi-line) label.
    var meta = {};
    var children = nodes.map(function (n) {
      var lines = String(n.label == null ? n.id : n.label).split("\n");
      var w = 0;
      lines.forEach(function (ln) { w = Math.max(w, m.width(ln)); });
      var width = Math.max(Math.ceil(w) + PAD_X * 2, 46);
      var height = lines.length * LINE_H + PAD_Y * 2;
      meta[n.id] = { lines: lines, cls: n.cls || "", width: width, height: height };
      return { id: n.id, width: width, height: height };
    });
    var elkEdges = edges.map(function (e, i) {
      var eo = { id: "e" + i, sources: [e.from], targets: [e.to] };
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
        "elk.direction": opts.dir || "RIGHT",
        "elk.edgeRouting": "ORTHOGONAL",
        "elk.layered.spacing.nodeNodeBetweenLayers": "58",
        "elk.spacing.nodeNode": "22",
        "elk.spacing.edgeNode": "16",
        "elk.spacing.edgeEdge": "12",
        "elk.layered.spacing.edgeEdgeBetweenLayers": "12",
        "elk.layered.spacing.edgeNodeBetweenLayers": "16",
        "elk.layered.nodePlacement.strategy": "NETWORK_SIMPLEX",
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
    var svg = el("svg", { class: "elk-svg", viewBox: "-1 -1 " + W + " " + H, width: W, height: H });
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
      (e.sections || []).forEach(function (s) {
        var pts = [s.startPoint].concat(s.bendPoints || []).concat([s.endPoint]);
        svg.appendChild(el("path", { d: path(pts), class: "elk-edge", "marker-end": "url(#elk-arrow)" }));
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
      var g = el("g", { class: "elk-node " + info.cls, transform: "translate(" + c.x + "," + c.y + ")" });
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
