// f5report — render each iRule's control-flow flowchart (Mermaid) lazily when
// its row is expanded. The Mermaid source is produced offline from the real
// Tcl/iRules IR (tcl-diagram) and embedded per rule; here we just render it with
// the already-loaded Mermaid. No dependencies beyond that, no network.
(function () {
  "use strict";
  if (!window.mermaid) return;

  function renderFlow(detail) {
    var host = detail.querySelector(".irule-flow-diagram");
    var src = detail.querySelector(".irule-flow-src");
    if (!host || !src || host._rendered) return;
    var def = (src.textContent || "").trim();
    if (!def) { host._rendered = true; return; }
    host._rendered = true;
    host.textContent = "rendering…";
    try {
      var id = "irflow-" + Math.random().toString(36).slice(2);
      mermaid.render(id, def).then(function (res) {
        host.innerHTML = res.svg;
      }).catch(function () {
        host.textContent = "(flowchart could not be rendered)";
        host._rendered = false;
      });
    } catch (e) {
      host.textContent = "(flowchart could not be rendered)";
      host._rendered = false;
    }
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
