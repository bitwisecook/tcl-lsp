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

// f5report — in-browser f5-query console, powered by the wasm build of the
// query engine (the same DSL as the `f5-query` CLI). Runs entirely client-side
// against the config embedded in the report; nothing leaves the page.
(function () {
  "use strict";

  var modelEl = document.getElementById("f5-model");
  if (!modelEl || typeof wasm_bindgen === "undefined") return;
  var MODEL = JSON.parse(modelEl.textContent);

  var B64 = document.getElementById("f5-wasm").textContent.trim();
  function b64ToBytes(b64) {
    var bin = atob(b64), u = new Uint8Array(bin.length);
    for (var i = 0; i < bin.length; i++) u[i] = bin.charCodeAt(i);
    return u;
  }

  var ready = null; // Promise resolving once the wasm module is initialised
  function initWasm() {
    if (!ready) {
      ready = wasm_bindgen(b64ToBytes(B64)).then(function () {
        return wasm_bindgen.engine_version();
      });
    }
    return ready;
  }

  var EXAMPLES = [
    [".ltm.virtual[] | {name, destination, pool}", "virtual servers → destination + pool"],
    [".ltm.pool[] | select((.members | length) == 0) | .name", "pools with no members"],
    [".ltm.pool[] | select((referenced_by(.) | length) == 0) | .name", "orphaned pools (nothing references them)"],
    [".ltm.virtual[] | select(.pool == \"\") | .name", "virtuals with no default pool"],
    ["[.ltm.node[]] | length", "count the nodes"],
    [".ltm.rule[] | {name, pools: .refs.pools}", "pools referenced inside each iRule"],
    [".ltm.virtual[] | select(.profiles | any(contains(\"clientssl\"))) | .name", "virtuals using a client-SSL profile"],
  ];

  function esc(s) { return String(s).replace(/[&<>]/g, function (c) { return { "&": "&amp;", "<": "&lt;", ">": "&gt;" }[c]; }); }

  function initConsole(deviceEl, dev) {
    var panel = deviceEl.querySelector('.panel[data-panel="console"]');
    if (!panel) return;
    var ta = panel.querySelector(".qc-expr");
    var mode = panel.querySelector(".qc-mode");
    var runBtn = panel.querySelector(".qc-run");
    var out = panel.querySelector(".qc-out");
    var status = panel.querySelector(".qc-status");
    var exSel = panel.querySelector(".qc-examples");

    exSel.innerHTML = '<option value="">Load an example query…</option>' +
      EXAMPLES.map(function (e, i) { return '<option value="' + i + '">' + esc(e[1]) + "</option>"; }).join("");
    exSel.addEventListener("change", function () {
      if (exSel.value !== "") { ta.value = EXAMPLES[+exSel.value][0]; exSel.value = ""; ta.focus(); }
    });

    var sources = JSON.stringify([[dev.uri, dev.configText]]);

    function run() {
      var expr = ta.value.trim();
      if (!expr) return;
      status.textContent = "running…";
      out.className = "qc-out";
      initWasm().then(function (ver) {
        var t0 = performance.now();
        try {
          var res = wasm_bindgen.run_query(expr, sources, mode.value, false);
          var dt = (performance.now() - t0).toFixed(1);
          out.textContent = res;
          status.textContent = "engine v" + ver + " · " + dt + " ms · in-browser (WASM)";
        } catch (e) {
          out.className = "qc-out qc-err";
          out.textContent = String(e && e.message ? e.message : e);
          status.textContent = "error";
        }
      }).catch(function (e) {
        status.textContent = "failed to load wasm engine: " + e;
      });
    }

    runBtn.addEventListener("click", run);
    ta.addEventListener("keydown", function (e) {
      if ((e.ctrlKey || e.metaKey) && e.key === "Enter") { e.preventDefault(); run(); }
    });

    // warm the engine + run the first example when the tab is first shown
    var warmed = false;
    deviceEl.querySelectorAll('.tab[data-panel="console"]').forEach(function (tab) {
      tab.addEventListener("click", function () {
        if (warmed) return; warmed = true;
        status.textContent = "loading wasm engine…";
        if (!ta.value) ta.value = EXAMPLES[0][0];
        initWasm().then(function () { run(); });
      });
    });
  }

  document.querySelectorAll(".device").forEach(function (deviceEl) {
    initConsole(deviceEl, MODEL.devices[parseInt(deviceEl.dataset.dev, 10)]);
  });
})();
