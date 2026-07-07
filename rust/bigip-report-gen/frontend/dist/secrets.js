// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/bigip-report-gen/frontend/src — DO NOT EDIT; edit the .ts source.
"use strict";
(() => {
  // src/pages/secrets.ts
  (function() {
    "use strict";
    function wire(panel) {
      var rows = Array.prototype.slice.call(panel.querySelectorAll("tr.secret-row"));
      var revealable = rows.filter(function(r) {
        return r.querySelector(".secret-reveal");
      });
      var encrypted = rows.length - revealable.length;
      function toggle(row, show) {
        var val = row.querySelector(".secret-val");
        var mask = row.querySelector(".secret-mask");
        var btn = row.querySelector(".secret-reveal");
        if (!val || !btn) return;
        if (show) {
          val.hidden = false;
          if (mask) mask.style.display = "none";
          btn.textContent = "Hide";
        } else {
          val.hidden = true;
          if (mask) mask.style.display = "";
          btn.textContent = "Reveal";
        }
      }
      revealable.forEach(function(row) {
        var btn = row.querySelector(".secret-reveal");
        btn.addEventListener("click", function() {
          toggle(row, row.querySelector(".secret-val").hidden);
        });
      });
      var summary = panel.querySelector(".secrets-summary");
      if (!summary) return;
      var parts = [];
      if (revealable.length) parts.push("<b>" + revealable.length + "</b> decrypted");
      if (encrypted) parts.push("<b>" + encrypted + "</b> still encrypted (supply f5mku to reveal)");
      summary.innerHTML = parts.join(" \xB7 ");
      if (revealable.length) {
        var all = document.createElement("button");
        all.type = "button";
        all.className = "secret-reveal-all";
        var shown = false;
        all.textContent = "Reveal all";
        all.addEventListener("click", function() {
          shown = !shown;
          revealable.forEach(function(r) {
            toggle(r, shown);
          });
          all.textContent = shown ? "Hide all" : "Reveal all";
        });
        summary.appendChild(document.createTextNode(" "));
        summary.appendChild(all);
      }
    }
    document.querySelectorAll('.panel[data-panel="secrets"]').forEach(wire);
  })();
})();
