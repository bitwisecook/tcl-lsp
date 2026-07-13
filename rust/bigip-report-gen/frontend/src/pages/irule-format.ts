// @ts-nocheck -- vanilla DOM glue, typed incrementally.
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

// f5report — iRule Format + Diagnostics toolbar. Both run the same engine the
// LSP / `tcl` CLI use, compiled to wasm: `format_irule` (F5 iRules Style Guide
// formatter) and `analyze_irule` (full analyser — diagnostics *and* optimiser
// suggestions, `f5-irules` dialect) rendered inline on the highlighted code.
// Entirely client-side against the code already in the page — nothing leaves
// the browser, the report stays self-contained.
(function () {
  "use strict";

  var wasmEl = document.getElementById("f5-wasm");
  // A bare `typeof wasm_bindgen` THROWS, it does not return "undefined", when the
  // glue's `let wasm_bindgen = (function(){…})(…)` initialiser failed and left the
  // binding in the temporal dead zone (it does `new URL(document.currentScript.src,
  // …)`, which fails for an inlined <script>). It must stay a bare identifier — a
  // top-level `let` is not a property of globalThis — so catch instead.
  // See pages/print.ts for the full story.
  var wasmReady = false;
  try {
    wasmReady = typeof wasm_bindgen !== "undefined";
  } catch (_e) {
    wasmReady = false;
  }
  if (!wasmEl || !wasmReady) return;

  // Share one wasm instantiation with the query console (whichever loads first
  // wins) so the module is only initialised once per report.
  function initWasm() {
    if (!window.__f5qReady) {
      var b64 = wasmEl.textContent.trim();
      var bin = atob(b64);
      var u = new Uint8Array(bin.length);
      for (var i = 0; i < bin.length; i++) u[i] = bin.charCodeAt(i);
      window.__f5qReady = wasm_bindgen(u);
    }
    return window.__f5qReady;
  }

  function esc(s) {
    return String(s).replace(/[&<>]/g, function (c) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;" }[c];
    });
  }

  // Recover plain source text from a fragment of highlighted HTML.
  var scratch = document.createElement("div");
  function htmlToText(html) {
    scratch.innerHTML = html;
    return scratch.textContent;
  }

  var SEV_ORDER = ["error", "warning", "info", "suggestion", "hint"];

  function setup(pre) {
    if (pre.dataset.iruleTools) return; // already enhanced
    pre.dataset.iruleTools = "1";
    var originalHtml = pre.innerHTML; // config's own highlighted source
    var rawSource = pre.textContent; // highlight preserves text → raw iRule
    var formattedText = null; // lazily filled once

    var state = { formatted: false, diags: false };

    var toolbar = document.createElement("div");
    toolbar.className = "code-toolbar";
    var fmtBtn = mkBtn("Format", "Reformat with the F5 iRules Style Guide formatter");
    var diagBtn = mkBtn("Diagnostics", "Show analyser diagnostics + optimiser suggestions inline");
    toolbar.appendChild(fmtBtn);
    toolbar.appendChild(diagBtn);
    pre.parentNode.insertBefore(toolbar, pre);

    var panel = document.createElement("div");
    panel.className = "irule-diags";
    panel.hidden = true;
    pre.parentNode.insertBefore(panel, pre.nextSibling);

    function mkBtn(label, title) {
      var b = document.createElement("button");
      b.type = "button";
      b.className = "code-tool-btn";
      b.textContent = label;
      b.title = title;
      return b;
    }

    function currentSource() {
      return state.formatted && formattedText != null ? formattedText : rawSource;
    }

    // Render the pre + panel for the current toggle state.
    function render() {
      if (state.diags) {
        var res = JSON.parse(wasm_bindgen.analyze_irule(currentSource()));
        pre.innerHTML = res.html;
        renderPanel(res.diagnostics, res.counts);
        wireInlineHover();
      } else {
        panel.hidden = true;
        if (state.formatted) {
          pre.innerHTML = wasm_bindgen.format_irule(currentSource());
        } else {
          pre.innerHTML = originalHtml;
        }
      }
      fmtBtn.classList.toggle("is-active", state.formatted);
      diagBtn.classList.toggle("is-active", state.diags);
    }

    function renderPanel(diags, counts) {
      if (!diags.length) {
        panel.innerHTML = '<div class="irule-diags-empty">No analyser findings — clean iRule.</div>';
        panel.hidden = false;
        return;
      }
      var summary = SEV_ORDER.filter(function (s) { return counts[s]; })
        .map(function (s) { return '<span class="diag-badge diag-' + s + '">' + counts[s] + " " + s + (counts[s] > 1 ? "s" : "") + "</span>"; })
        .join("");
      var rows = diags.map(function (d) {
        return '<li class="diag-row diag-' + d.severity + '" data-diag="' + d.index + '">' +
          '<span class="diag-sev diag-' + d.severity + '">' + d.severity + "</span>" +
          '<span class="diag-code">' + esc(d.code) + "</span>" +
          '<span class="diag-loc">' + d.line + ":" + d.col + "</span>" +
          '<span class="diag-msg">' + esc(d.message) + "</span></li>";
      }).join("");
      panel.innerHTML = '<div class="irule-diags-head">' + summary + "</div><ul class=\"diag-list\">" + rows + "</ul>";
      panel.hidden = false;
      // Clicking a row flashes the matching inline marker.
      panel.querySelectorAll(".diag-row").forEach(function (row) {
        row.addEventListener("click", function () { flashInline(row.getAttribute("data-diag")); });
      });
    }

    // Two-way hover: hovering an inline marker highlights its list rows.
    function wireInlineHover() {
      pre.querySelectorAll(".diag[data-diag]").forEach(function (mark) {
        mark.addEventListener("mouseenter", function () { markRows(mark.getAttribute("data-diag"), true); });
        mark.addEventListener("mouseleave", function () { markRows(mark.getAttribute("data-diag"), false); });
      });
    }
    function markRows(ids, on) {
      (ids || "").split(/\s+/).forEach(function (id) {
        var row = panel.querySelector('.diag-row[data-diag="' + id + '"]');
        if (row) row.classList.toggle("hot", on);
      });
    }
    function flashInline(id) {
      var mark = pre.querySelector('.diag[data-diag~="' + id + '"]');
      if (!mark) return;
      mark.classList.add("flash");
      mark.scrollIntoView({ block: "nearest" });
      setTimeout(function () { mark.classList.remove("flash"); }, 900);
    }

    function withWasm(btn, fn) {
      btn.disabled = true;
      initWasm().then(function () {
        try { fn(); } finally { btn.disabled = false; }
      }).catch(function (e) {
        btn.disabled = false;
        if (window.console && console.error) console.error("wasm load failed", e);
      });
    }

    fmtBtn.addEventListener("click", function () {
      withWasm(fmtBtn, function () {
        if (!state.formatted && formattedText == null) {
          formattedText = htmlToText(wasm_bindgen.format_irule(rawSource));
        }
        state.formatted = !state.formatted;
        render();
      });
    });
    diagBtn.addEventListener("click", function () {
      withWasm(diagBtn, function () {
        state.diags = !state.diags;
        render();
      });
    });
  }

  // Enhance every iRule body: the inline rows in the iRules panel, and the
  // per-object slide-over drawer (which builds its own <pre> on each open).
  function enhanceAll(root) {
    (root || document).querySelectorAll("pre.code.tcl").forEach(setup);
  }
  enhanceAll(document);

  // The object drawer re-renders its body (new <pre>) each time you click an
  // iRule name; watch it and enhance the fresh code block.
  var drawer = document.getElementById("objDrawer");
  if (drawer && typeof MutationObserver !== "undefined") {
    new MutationObserver(function () { enhanceAll(drawer); }).observe(drawer, {
      childList: true,
      subtree: true,
    });
  }
})();
