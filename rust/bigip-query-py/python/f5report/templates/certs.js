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

// f5report — SSL certificate expiry tab. Pure client-side: days-remaining is
// computed against the viewer's clock every time the report is opened, so the
// inventory is always current. No dependencies, no network.
(function () {
  "use strict";

  var DAY = 86400 * 1000;

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
    if (!tbody) return; // no certs
    var rows = Array.prototype.slice.call(panel.querySelectorAll("tr.cert-row"));
    var windowSel = panel.querySelector(".cert-window");
    var onlyProblems = panel.querySelector(".cert-only-problems");
    var summaryEl = panel.querySelector(".cert-summary");

    // Pair each cert row with its detail sibling and parse the epoch once.
    var items = rows.map(function (row) {
      var detail = row.nextElementSibling;
      if (detail && !detail.classList.contains("detail")) detail = null;
      var raw = (row.getAttribute("data-epoch") || "").trim();
      var epoch = /^\d+$/.test(raw) ? parseInt(raw, 10) : NaN;
      var hasEpoch = !isNaN(epoch) && epoch > 0;
      return { row: row, detail: detail, epoch: epoch, hasEpoch: hasEpoch };
    });

    // Sort soonest-to-expire first; unknown expiry sinks to the bottom.
    items.sort(function (a, b) {
      if (a.hasEpoch && b.hasEpoch) return a.epoch - b.epoch;
      if (a.hasEpoch) return -1;
      if (b.hasEpoch) return 1;
      return 0;
    });
    items.forEach(function (it) {
      tbody.appendChild(it.row);
      if (it.detail) tbody.appendChild(it.detail);
    });

    function render() {
      var now = Date.now();
      var window = parseInt(windowSel ? windowSel.value : "30", 10) || 30;
      var expired = 0, soon = 0, ok = 0, unknown = 0;
      items.forEach(function (it) {
        var cell = it.row.querySelector(".cert-remaining");
        it.row.classList.remove("cert-ok", "cert-warn", "cert-expired", "cert-unknown");
        var days = it.hasEpoch ? Math.floor((it.epoch * 1000 - now) / DAY) : NaN;
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
        parts.push('<span class="cs warn"><b>' + soon + "</b> expiring ≤ " + window + "d</span>");
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
