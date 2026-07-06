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

// f5report — render each iRule's control-flow graph lazily when its row is
// expanded. The graph ({nodes, edges} JSON) is produced offline from the real
// Tcl/iRules IR (tcl-diagram) and embedded per rule; here we parse it and draw
// it with the orthogonal elkjs renderer. No dependencies beyond that, no network.
(function () {
  "use strict";
  if (!window.ElkGraph) return;

  function renderFlow(detail) {
    var host = detail.querySelector(".irule-flow-diagram");
    var src = detail.querySelector(".irule-flow-src");
    if (!host || !src || host._rendered) return;
    var raw = (src.textContent || "").trim();
    if (!raw) { host._rendered = true; return; }
    var model;
    try { model = JSON.parse(raw); } catch (e) { host._rendered = true; return; }
    host._rendered = true;
    host.textContent = "rendering…";
    window.ElkGraph.render(host, model, { dir: "DOWN", svgClass: "elk-report" })
      .catch(function () {
        host.textContent = "(flowchart could not be rendered)";
        host._rendered = false;
      });
  }

  // The iRule rows are `tr.expandable`; report.js toggles `.open` on the
  // following `tr.detail`. Render the flowchart the first time a row opens.
  document.querySelectorAll('.panel[data-panel="rules"] tr.expandable').forEach(function (row) {
    row.addEventListener("click", function (ev) {
      if (ev.target.tagName === "A") return;
      var detail = row.nextElementSibling;
      if (detail && detail.classList.contains("detail") && detail.classList.contains("open")) {
        renderFlow(detail);
      }
    });
  });
})();
