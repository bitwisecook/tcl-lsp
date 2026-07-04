// f5report — render each APM access profile's dependency walk (Mermaid) lazily
// when the APM tab is first shown. The Mermaid source is produced offline by the
// APM walker (apm.rs) and embedded per profile; here we just render it with the
// already-loaded Mermaid. No dependencies beyond that, no network.
(function () {
  "use strict";
  if (!window.mermaid) return;

  function renderOne(host) {
    if (host._rendered) return;
    var src = host.querySelector(".apm-src");
    var out = host.querySelector(".apm-diagram");
    if (!src || !out) return;
    var def = (src.textContent || "").trim();
    host._rendered = true;
    if (!def) { out.textContent = "(no linked objects)"; return; }
    out.textContent = "rendering…";
    try {
      var id = "apm-" + Math.random().toString(36).slice(2);
      mermaid.render(id, def).then(function (res) {
        out.innerHTML = res.svg;
        var svg = out.querySelector("svg");
        if (svg) { svg.style.maxWidth = "100%"; svg.style.height = "auto"; }
      }).catch(function () {
        out.textContent = "(diagram could not be rendered)";
        host._rendered = false;
      });
    } catch (e) {
      out.textContent = "(diagram could not be rendered)";
      host._rendered = false;
    }
  }

  function renderVisible(panel) {
    panel.querySelectorAll(".apm-graph").forEach(renderOne);
  }

  // Render when the APM tab is first activated (Mermaid layout needs the panel
  // to be visible to size text), and once on load if it is already active.
  document.querySelectorAll(".device").forEach(function (device) {
    var panel = device.querySelector('.panel[data-panel="apm"]');
    if (!panel) return;
    if (panel.classList.contains("active")) renderVisible(panel);
    var tab = device.querySelector('.tab[data-panel="apm"]');
    if (tab) tab.addEventListener("click", function () {
      // let report.js flip `.active` first
      setTimeout(function () { renderVisible(panel); }, 0);
    });
  });
})();
