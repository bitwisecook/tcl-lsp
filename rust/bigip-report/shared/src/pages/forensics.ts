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

// f5report — Forensics tab. Renders the UCS file inventory + the ATT&CK-mapped
// hunting checklist from the embedded model. File contents (non-sensitive text
// only) sit behind a per-file reveal; shadow / private keys are never embedded.
// Everything is client-side — nothing left the browser.
(function () {
  "use strict";
  var MODEL = null;
  try { MODEL = JSON.parse(document.getElementById("f5-model").textContent); }
  catch (e) { return; }
  if (!MODEL || !MODEL.devices) return;

  function esc(s) {
    return String(s == null ? "" : s).replace(/[&<>"]/g, function (c) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c];
    });
  }
  function fmtSize(n) {
    n = +n || 0;
    if (n < 1024) return n + " B";
    if (n < 1024 * 1024) return (n / 1024).toFixed(1) + " KB";
    return (n / (1024 * 1024)).toFixed(1) + " MB";
  }
  var VERDICT_TONE = { alert: "red", warn: "amber", info: "blue", clear: "green", absent: "muted" };

  function renderChecklist(checklist) {
    if (!checklist || !checklist.length) return "";
    var cards = checklist.map(function (c) {
      var tone = VERDICT_TONE[c.verdict] || "muted";
      return '<div class="fx-check fx-' + tone + '">' +
        '<div class="fx-check-head"><span class="fx-verdict">' + esc(c.verdict) + "</span>" +
        '<span class="fx-check-label">' + esc(c.label) + "</span>" +
        '<span class="fx-attack">' + esc(c.attack) + "</span></div>" +
        '<div class="fx-check-detail">' + esc(c.detail) + "</div></div>";
    }).join("");
    return '<div class="fx-checklist">' + cards + "</div>";
  }

  function renderFiles(files) {
    if (!files || !files.length) {
      return '<p class="hint">No forensic files in this source. Generate the report from a ' +
        "<code>.ucs</code> archive (not a bare <code>bigip.conf</code>) to populate this view.</p>";
    }
    var rows = files.map(function (f, i) {
      var actions;
      if (f.sensitive) {
        actions = '<span class="tag amber" title="password hashes / private keys are never embedded in the report">masked — sensitive</span>';
      } else if (f.content != null) {
        actions = '<button type="button" class="fx-reveal" data-i="' + i + '">View</button>';
      } else {
        actions = '<span class="muted">—</span>';
      }
      var sha = esc((f.sha256 || "").slice(0, 16));
      var main = '<tr class="fx-row" data-cat="' + esc(f.category) + '">' +
        '<td class="mono strong">' + esc(f.path) + "</td>" +
        '<td><span class="tag">' + esc(f.category) + "</span></td>" +
        '<td class="mono small">' + fmtSize(f.size) + "</td>" +
        '<td class="mono small" title="' + esc(f.sha256) + '">' + sha + "…</td>" +
        "<td>" + actions + "</td></tr>";
      var detail = (f.content != null && !f.sensitive)
        ? '<tr class="fx-content" data-i="' + i + '" hidden><td colspan="5"><pre class="code">' +
          esc(f.content) + "</pre></td></tr>"
        : "";
      return main + detail;
    }).join("");
    return '<div class="tablewrap"><table class="grid fx-table"><thead><tr>' +
      "<th>Path</th><th>Category</th><th>Size</th><th>SHA-256</th><th></th>" +
      "</tr></thead><tbody>" + rows + "</tbody></table></div>";
  }

  function wire(view) {
    view.querySelectorAll(".fx-reveal").forEach(function (btn) {
      btn.addEventListener("click", function () {
        var row = view.querySelector('.fx-content[data-i="' + btn.getAttribute("data-i") + '"]');
        if (!row) return;
        row.hidden = !row.hidden;
        btn.textContent = row.hidden ? "View" : "Hide";
      });
    });
  }

  MODEL.devices.forEach(function (d, di) {
    var deviceEl = document.querySelector('.device[data-dev="' + di + '"]') ||
      document.querySelectorAll(".device")[di];
    if (!deviceEl) return;
    var view = deviceEl.querySelector('.panel[data-panel="forensics"] .forensics-view');
    if (!view) return;
    var fx = d.forensics || { files: [], checklist: [] };
    view.innerHTML = renderChecklist(fx.checklist) + renderFiles(fx.files);
    wire(view);
  });
})();
