// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/bigip-report/shared/src — DO NOT EDIT; edit the .ts source.
"use strict";
(() => {
  // src/pages/certs.ts
  (function() {
    "use strict";
    var DAY = 86400 * 1e3;
    function fmtRemaining(days) {
      if (days === 0) return "expires today";
      if (days > 0) {
        if (days < 45) return "in " + days + " day" + (days === 1 ? "" : "s");
        if (days < 400) return "in " + Math.round(days / 30) + " mo";
        return "in " + (days / 365).toFixed(1) + " yr";
      }
      var a = -days;
      if (a < 45) return "expired " + a + " day" + (a === 1 ? "" : "s") + " ago";
      if (a < 400) return "expired " + Math.round(a / 30) + " mo ago";
      return "expired " + (a / 365).toFixed(1) + " yr ago";
    }
    function classify(days, hasEpoch, window) {
      if (!hasEpoch) return "cert-unknown";
      if (days < 0) return "cert-expired";
      if (days <= window) return "cert-warn";
      return "cert-ok";
    }
    function initPanel(panel) {
      var tbody = panel.querySelector("tbody");
      if (!tbody) return;
      var rows = Array.prototype.slice.call(panel.querySelectorAll("tr.cert-row"));
      var windowSel = panel.querySelector(".cert-window");
      var onlyProblems = panel.querySelector(".cert-only-problems");
      var summaryEl = panel.querySelector(".cert-summary");
      var items = rows.map(function(row) {
        var detail = row.nextElementSibling;
        if (detail && !detail.classList.contains("detail")) detail = null;
        var raw = (row.getAttribute("data-epoch") || "").trim();
        var epoch = /^\d+$/.test(raw) ? parseInt(raw, 10) : NaN;
        var hasEpoch = !isNaN(epoch) && epoch > 0;
        return { row, detail, epoch, hasEpoch };
      });
      items.sort(function(a, b) {
        if (a.hasEpoch && b.hasEpoch) return a.epoch - b.epoch;
        if (a.hasEpoch) return -1;
        if (b.hasEpoch) return 1;
        return 0;
      });
      items.forEach(function(it) {
        tbody.appendChild(it.row);
        if (it.detail) tbody.appendChild(it.detail);
      });
      function render() {
        var now = Date.now();
        var window = parseInt(windowSel ? windowSel.value : "30", 10) || 30;
        var expired = 0, soon = 0, ok = 0, unknown = 0;
        items.forEach(function(it) {
          var cell = it.row.querySelector(".cert-remaining");
          it.row.classList.remove("cert-ok", "cert-warn", "cert-expired", "cert-unknown");
          var days = it.hasEpoch ? Math.floor((it.epoch * 1e3 - now) / DAY) : NaN;
          var cls = classify(days, it.hasEpoch, window);
          it.row.classList.add(cls);
          if (cell) cell.textContent = it.hasEpoch ? fmtRemaining(days) : "unknown";
          if (cls === "cert-expired") expired++;
          else if (cls === "cert-warn") soon++;
          else if (cls === "cert-ok") ok++;
          else unknown++;
          var problem = cls === "cert-expired" || cls === "cert-warn";
          var hide = onlyProblems && onlyProblems.checked && !problem;
          it.row.style.display = hide ? "none" : "";
          if (it.detail && hide) {
            it.detail.style.display = "none";
            it.detail.classList.remove("open");
            it.row.classList.remove("open");
          } else if (it.detail && !it.detail.classList.contains("open")) {
            it.detail.style.display = "";
          }
        });
        if (summaryEl) {
          var parts = [];
          parts.push('<span class="cs bad"><b>' + expired + "</b> expired</span>");
          parts.push('<span class="cs warn"><b>' + soon + "</b> expiring \u2264 " + window + "d</span>");
          parts.push('<span class="cs ok"><b>' + ok + "</b> valid</span>");
          if (unknown) parts.push('<span class="cs"><b>' + unknown + "</b> no expiry in config</span>");
          summaryEl.innerHTML = parts.join("");
        }
      }
      if (windowSel) windowSel.addEventListener("change", render);
      if (onlyProblems) onlyProblems.addEventListener("change", render);
      render();
    }
    document.querySelectorAll('.panel[data-panel="certificates"]').forEach(initPanel);
  })();
})();
