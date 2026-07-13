// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/bigip-report-gen/frontend/src — DO NOT EDIT; edit the .ts source.
"use strict";
(() => {
  // src/pages/console.ts
  (function() {
    "use strict";
    var modelEl = document.getElementById("f5-model");
    var wasmReady = false;
    try {
      wasmReady = typeof wasm_bindgen !== "undefined";
    } catch (_e) {
      wasmReady = false;
    }
    if (!modelEl || !wasmReady) return;
    var MODEL = JSON.parse(modelEl.textContent);
    var B64 = document.getElementById("f5-wasm").textContent.trim();
    function b64ToBytes(b64) {
      var bin = atob(b64), u = new Uint8Array(bin.length);
      for (var i = 0; i < bin.length; i++) u[i] = bin.charCodeAt(i);
      return u;
    }
    function initWasm() {
      if (!window.__f5qReady) {
        window.__f5qReady = wasm_bindgen(b64ToBytes(B64));
      }
      return window.__f5qReady.then(function() {
        return MODEL.engine_version || wasm_bindgen.engine_version();
      });
    }
    var EXAMPLES = [
      [".ltm.virtual[] | {name, destination, pool}", "virtual servers \u2192 destination + pool"],
      [".ltm.pool[] | select((.members | length) == 0) | .name", "pools with no members"],
      [".ltm.pool[] | select((referenced_by(.) | length) == 0) | .name", "orphaned pools (nothing references them)"],
      ['.ltm.virtual[] | select(.pool == "") | .name', "virtuals with no default pool"],
      ["[.ltm.node[]] | length", "count the nodes"],
      [".ltm.rule[] | {name, pools: .refs.pools}", "pools referenced inside each iRule"],
      ['.ltm.virtual[] | select(.profiles | any(contains("clientssl"))) | .name', "virtuals using a client-SSL profile"]
    ];
    function esc(s) {
      return String(s).replace(/[&<>]/g, function(c) {
        return { "&": "&amp;", "<": "&lt;", ">": "&gt;" }[c];
      });
    }
    function initConsole(deviceEl, dev) {
      var panel = deviceEl.querySelector('.panel[data-panel="console"]');
      if (!panel) return;
      var ta = panel.querySelector(".qc-expr");
      var mode = panel.querySelector(".qc-mode");
      var runBtn = panel.querySelector(".qc-run");
      var out = panel.querySelector(".qc-out");
      var status = panel.querySelector(".qc-status");
      var exSel = panel.querySelector(".qc-examples");
      exSel.innerHTML = '<option value="">Load an example query\u2026</option>' + EXAMPLES.map(function(e, i) {
        return '<option value="' + i + '">' + esc(e[1]) + "</option>";
      }).join("");
      exSel.addEventListener("change", function() {
        if (exSel.value !== "") {
          ta.value = EXAMPLES[+exSel.value][0];
          exSel.value = "";
          ta.focus();
        }
      });
      var sources = JSON.stringify([[dev.uri, dev.configText]]);
      function run() {
        var expr = ta.value.trim();
        if (!expr) return;
        status.textContent = "running\u2026";
        out.className = "qc-out";
        initWasm().then(function(ver) {
          var t0 = performance.now();
          try {
            var res = wasm_bindgen.run_query(expr, sources, mode.value, false);
            var dt = (performance.now() - t0).toFixed(1);
            out.textContent = res;
            status.textContent = "engine v" + ver + " \xB7 " + dt + " ms \xB7 in-browser (WASM)";
          } catch (e) {
            out.className = "qc-out qc-err";
            out.textContent = String(e && e.message ? e.message : e);
            status.textContent = "error";
          }
        }).catch(function(e) {
          status.textContent = "failed to load wasm engine: " + e;
        });
      }
      runBtn.addEventListener("click", run);
      ta.addEventListener("keydown", function(e) {
        if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
          e.preventDefault();
          run();
        }
      });
      var warmed = false;
      deviceEl.querySelectorAll('.tab[data-panel="console"]').forEach(function(tab) {
        tab.addEventListener("click", function() {
          if (warmed) return;
          warmed = true;
          status.textContent = "loading wasm engine\u2026";
          if (!ta.value) ta.value = EXAMPLES[0][0];
          initWasm().then(function() {
            run();
          });
        });
      });
    }
    document.querySelectorAll(".device").forEach(function(deviceEl) {
      initConsole(deviceEl, MODEL.devices[parseInt(deviceEl.dataset.dev, 10)]);
    });
  })();
})();
