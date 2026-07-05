// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/bigip-report/shared/src — DO NOT EDIT; edit the .ts source.
"use strict";
(() => {
  // src/pages/irule-flow.ts
  (function() {
    "use strict";
    if (!window.mermaid) return;
    function renderFlow(detail) {
      var host = detail.querySelector(".irule-flow-diagram");
      var src = detail.querySelector(".irule-flow-src");
      if (!host || !src || host._rendered) return;
      var def = (src.textContent || "").trim();
      if (!def) {
        host._rendered = true;
        return;
      }
      host._rendered = true;
      host.textContent = "rendering\u2026";
      try {
        var id = "irflow-" + Math.random().toString(36).slice(2);
        mermaid.render(id, def).then(function(res) {
          host.innerHTML = res.svg;
        }).catch(function() {
          host.textContent = "(flowchart could not be rendered)";
          host._rendered = false;
        });
      } catch (e) {
        host.textContent = "(flowchart could not be rendered)";
        host._rendered = false;
      }
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
