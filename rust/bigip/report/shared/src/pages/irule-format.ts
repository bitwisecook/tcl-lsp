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

// f5report — iRule "Format" button. Runs the same Tcl formatter the LSP and
// `tcl` CLI use (F5 iRules Style Guide defaults) via the wasm build of the
// engine (`format_irule`), entirely client-side against the code already in the
// page — nothing leaves the browser, the report stays self-contained.
(function () {
  "use strict";

  var wasmEl = document.getElementById("f5-wasm");
  if (!wasmEl || typeof wasm_bindgen === "undefined") return;

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

  // iRule bodies are the `<pre class="code tcl">` blocks inside each device's
  // iRules panel. Give each one a Format toggle.
  document.querySelectorAll('.panel[data-panel="rules"] pre.code.tcl').forEach(function (pre) {
    var original = pre.innerHTML; // syntax-highlighted source, as generated
    var toolbar = document.createElement("div");
    toolbar.className = "code-toolbar";
    var btn = document.createElement("button");
    btn.type = "button";
    btn.className = "code-format-btn";
    btn.textContent = "Format";
    btn.title = "Reformat this iRule with the F5 iRules Style Guide formatter";
    toolbar.appendChild(btn);
    pre.parentNode.insertBefore(toolbar, pre);

    var formatted = false;
    btn.addEventListener("click", function () {
      if (formatted) {
        // Toggle back to the config's original layout.
        pre.innerHTML = original;
        btn.textContent = "Format";
        btn.classList.remove("is-active");
        formatted = false;
        return;
      }
      var src = pre.textContent; // highlight_tcl preserves text → the raw source
      btn.disabled = true;
      btn.textContent = "Formatting…";
      initWasm().then(function () {
        try {
          pre.innerHTML = wasm_bindgen.format_irule(src);
          btn.textContent = "Restore";
          btn.classList.add("is-active");
          formatted = true;
        } catch (e) {
          btn.textContent = "Format failed";
          if (window.console && console.error) console.error("format_irule", e);
        } finally {
          btn.disabled = false;
        }
      }).catch(function (e) {
        btn.disabled = false;
        btn.textContent = "Format";
        if (window.console && console.error) console.error("wasm load failed", e);
      });
    });
  });
})();
