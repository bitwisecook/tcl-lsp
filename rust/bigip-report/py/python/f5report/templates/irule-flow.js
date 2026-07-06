// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/bigip-report/shared/src — DO NOT EDIT; edit the .ts source.
"use strict";
(() => {
  // src/pages/irule-flow.ts
  (function() {
    "use strict";
    if (!window.ElkGraph) return;
    function renderFlow(detail) {
      var host = detail.querySelector(".irule-flow-diagram");
      var src = detail.querySelector(".irule-flow-src");
      if (!host || !src || host._rendered) return;
      var raw = (src.textContent || "").trim();
      if (!raw) {
        host._rendered = true;
        return;
      }
      var model;
      try {
        model = JSON.parse(raw);
      } catch (e) {
        host._rendered = true;
        return;
      }
      host._rendered = true;
      host.textContent = "rendering\u2026";
      window.ElkGraph.render(host, model, { dir: "DOWN", svgClass: "elk-report" }).catch(function() {
        host.textContent = "(flowchart could not be rendered)";
        host._rendered = false;
      });
    }
    document.querySelectorAll('.panel[data-panel="rules"] tr.expandable').forEach(function(row) {
      row.addEventListener("click", function(ev) {
        if (ev.target.tagName === "A") return;
        var detail = row.nextElementSibling;
        if (detail && detail.classList.contains("detail") && detail.classList.contains("open")) {
          renderFlow(detail);
        }
      });
    });
  })();
})();
