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

// f5report — render each APM access profile's dependency walk as an orthogonal
// graph, lazily when the APM tab is first shown. The `{nodes, edges}` model is
// produced offline by the APM walker (apm.rs) and embedded per profile; the
// dedicated elkjs renderer (elk-graph.js) lays it out with true right-angle
// edge routing. No network.
(function () {
  "use strict";

  function renderOne(host) {
    if (host._rendered || !window.ElkGraph) return;
    var src = host.querySelector(".apm-graph-data");
    var out = host.querySelector(".apm-diagram");
    if (!src || !out) return;
    host._rendered = true;
    var data;
    try { data = JSON.parse(src.textContent || "{}"); }
    catch (e) { out.textContent = "(diagram data error)"; return; }
    out.textContent = "rendering…";
    window.ElkGraph.render(out, data, { dir: "RIGHT" }).catch(function () {
      out.textContent = "(diagram could not be rendered)";
      host._rendered = false;
    });
  }

  function renderVisible(panel) {
    panel.querySelectorAll(".apm-graph").forEach(renderOne);
  }

  // Render when the APM tab is first activated (layout needs the panel visible
  // to measure label text), and once on load if it is already active.
  document.querySelectorAll(".device").forEach(function (device) {
    var panel = device.querySelector('.panel[data-panel="apm"]');
    if (!panel) return;
    if (panel.classList.contains("active")) renderVisible(panel);
    var tab = device.querySelector('.tab[data-panel="apm"]');
    if (tab) tab.addEventListener("click", function () {
      setTimeout(function () { renderVisible(panel); }, 0);
    });
  });
})();
