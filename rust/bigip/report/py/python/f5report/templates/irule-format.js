// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/bigip-report/shared/src — DO NOT EDIT; edit the .ts source.
"use strict";
(() => {
  // src/pages/irule-format.ts
  (function() {
    "use strict";
    var wasmEl = document.getElementById("f5-wasm");
    if (!wasmEl || typeof wasm_bindgen === "undefined") return;
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
    document.querySelectorAll('.panel[data-panel="rules"] pre.code.tcl').forEach(function(pre) {
      var original = pre.innerHTML;
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
      btn.addEventListener("click", function() {
        if (formatted) {
          pre.innerHTML = original;
          btn.textContent = "Format";
          btn.classList.remove("is-active");
          formatted = false;
          return;
        }
        var src = pre.textContent;
        btn.disabled = true;
        btn.textContent = "Formatting\u2026";
        initWasm().then(function() {
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
        }).catch(function(e) {
          btn.disabled = false;
          btn.textContent = "Format";
          if (window.console && console.error) console.error("wasm load failed", e);
        });
      });
    });
  })();
})();
