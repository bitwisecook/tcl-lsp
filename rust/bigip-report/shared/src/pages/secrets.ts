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

// f5report — Secrets tab. Decrypted secrets are hidden by default; a per-row
// (or "reveal all") button toggles them. Encrypted secrets have no clear value
// to show (regenerate the report with the f5mku master key). Nothing here left
// the browser.
(function () {
  "use strict";

  function wire(panel) {
    var rows = Array.prototype.slice.call(panel.querySelectorAll("tr.secret-row"));
    var revealable = rows.filter(function (r) { return r.querySelector(".secret-reveal"); });
    var encrypted = rows.length - revealable.length;

    function toggle(row, show) {
      var val = row.querySelector(".secret-val");
      var mask = row.querySelector(".secret-mask");
      var btn = row.querySelector(".secret-reveal");
      if (!val || !btn) return;
      if (show) { val.hidden = false; if (mask) mask.style.display = "none"; btn.textContent = "Hide"; }
      else { val.hidden = true; if (mask) mask.style.display = ""; btn.textContent = "Reveal"; }
    }

    revealable.forEach(function (row) {
      var btn = row.querySelector(".secret-reveal");
      btn.addEventListener("click", function () {
        toggle(row, row.querySelector(".secret-val").hidden);
      });
    });

    // Summary + a global reveal/hide toggle (only when there are clear values).
    var summary = panel.querySelector(".secrets-summary");
    if (!summary) return;
    var parts = [];
    if (revealable.length) parts.push("<b>" + revealable.length + "</b> decrypted");
    if (encrypted) parts.push("<b>" + encrypted + "</b> still encrypted (supply f5mku to reveal)");
    summary.innerHTML = parts.join(" · ");
    if (revealable.length) {
      var all = document.createElement("button");
      all.type = "button";
      all.className = "secret-reveal-all";
      var shown = false;
      all.textContent = "Reveal all";
      all.addEventListener("click", function () {
        shown = !shown;
        revealable.forEach(function (r) { toggle(r, shown); });
        all.textContent = shown ? "Hide all" : "Reveal all";
      });
      summary.appendChild(document.createTextNode(" "));
      summary.appendChild(all);
    }
  }

  document.querySelectorAll('.panel[data-panel="secrets"]').forEach(wire);
})();
