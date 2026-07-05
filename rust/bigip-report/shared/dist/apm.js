// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/bigip-report/shared/ts — DO NOT EDIT; edit the .ts source.
"use strict";
(() => {
  // ts/apm.ts
  (function() {
    "use strict";
    function renderOne(host) {
      if (host._rendered || !window.ElkGraph) return;
      var src = host.querySelector(".apm-graph-data");
      var out = host.querySelector(".apm-diagram");
      if (!src || !out) return;
      host._rendered = true;
      var data;
      try {
        data = JSON.parse(src.textContent || "{}");
      } catch (e) {
        out.textContent = "(diagram data error)";
        return;
      }
      out.textContent = "rendering\u2026";
      window.ElkGraph.render(out, data, { dir: "RIGHT" }).catch(function() {
        out.textContent = "(diagram could not be rendered)";
        host._rendered = false;
      });
    }
    function renderVisible(panel) {
      panel.querySelectorAll(".apm-graph").forEach(renderOne);
    }
    document.querySelectorAll(".device").forEach(function(device) {
      var panel = device.querySelector('.panel[data-panel="apm"]');
      if (!panel) return;
      if (panel.classList.contains("active")) renderVisible(panel);
      var tab = device.querySelector('.tab[data-panel="apm"]');
      if (tab) tab.addEventListener("click", function() {
        setTimeout(function() {
          renderVisible(panel);
        }, 0);
      });
    });
  })();
})();
