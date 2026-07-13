// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/bigip-report-gen/frontend/src — DO NOT EDIT; edit the .ts source.
"use strict";
(() => {
  // src/pages/irule-format.ts
  (function() {
    "use strict";
    var wasmEl = document.getElementById("f5-wasm");
    var wasmReady = false;
    try {
      wasmReady = typeof wasm_bindgen !== "undefined";
    } catch (_e) {
      wasmReady = false;
    }
    if (!wasmEl || !wasmReady) return;
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
      return String(s).replace(/[&<>]/g, function(c) {
        return { "&": "&amp;", "<": "&lt;", ">": "&gt;" }[c];
      });
    }
    var scratch = document.createElement("div");
    function htmlToText(html) {
      scratch.innerHTML = html;
      return scratch.textContent;
    }
    var SEV_ORDER = ["error", "warning", "info", "suggestion", "hint"];
    function setup(pre) {
      if (pre.dataset.iruleTools) return;
      pre.dataset.iruleTools = "1";
      var originalHtml = pre.innerHTML;
      var rawSource = pre.textContent;
      var formattedText = null;
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
          panel.innerHTML = '<div class="irule-diags-empty">No analyser findings \u2014 clean iRule.</div>';
          panel.hidden = false;
          return;
        }
        var summary = SEV_ORDER.filter(function(s) {
          return counts[s];
        }).map(function(s) {
          return '<span class="diag-badge diag-' + s + '">' + counts[s] + " " + s + (counts[s] > 1 ? "s" : "") + "</span>";
        }).join("");
        var rows = diags.map(function(d) {
          return '<li class="diag-row diag-' + d.severity + '" data-diag="' + d.index + '"><span class="diag-sev diag-' + d.severity + '">' + d.severity + '</span><span class="diag-code">' + esc(d.code) + '</span><span class="diag-loc">' + d.line + ":" + d.col + '</span><span class="diag-msg">' + esc(d.message) + "</span></li>";
        }).join("");
        panel.innerHTML = '<div class="irule-diags-head">' + summary + '</div><ul class="diag-list">' + rows + "</ul>";
        panel.hidden = false;
        panel.querySelectorAll(".diag-row").forEach(function(row) {
          row.addEventListener("click", function() {
            flashInline(row.getAttribute("data-diag"));
          });
        });
      }
      function wireInlineHover() {
        pre.querySelectorAll(".diag[data-diag]").forEach(function(mark) {
          mark.addEventListener("mouseenter", function() {
            markRows(mark.getAttribute("data-diag"), true);
          });
          mark.addEventListener("mouseleave", function() {
            markRows(mark.getAttribute("data-diag"), false);
          });
        });
      }
      function markRows(ids, on) {
        (ids || "").split(/\s+/).forEach(function(id) {
          var row = panel.querySelector('.diag-row[data-diag="' + id + '"]');
          if (row) row.classList.toggle("hot", on);
        });
      }
      function flashInline(id) {
        var mark = pre.querySelector('.diag[data-diag~="' + id + '"]');
        if (!mark) return;
        mark.classList.add("flash");
        mark.scrollIntoView({ block: "nearest" });
        setTimeout(function() {
          mark.classList.remove("flash");
        }, 900);
      }
      function withWasm(btn, fn) {
        btn.disabled = true;
        initWasm().then(function() {
          try {
            fn();
          } finally {
            btn.disabled = false;
          }
        }).catch(function(e) {
          btn.disabled = false;
          if (window.console && console.error) console.error("wasm load failed", e);
        });
      }
      fmtBtn.addEventListener("click", function() {
        withWasm(fmtBtn, function() {
          if (!state.formatted && formattedText == null) {
            formattedText = htmlToText(wasm_bindgen.format_irule(rawSource));
          }
          state.formatted = !state.formatted;
          render();
        });
      });
      diagBtn.addEventListener("click", function() {
        withWasm(diagBtn, function() {
          state.diags = !state.diags;
          render();
        });
      });
    }
    function enhanceAll(root) {
      (root || document).querySelectorAll("pre.code.tcl").forEach(setup);
    }
    enhanceAll(document);
    var drawer = document.getElementById("objDrawer");
    if (drawer && typeof MutationObserver !== "undefined") {
      new MutationObserver(function() {
        enhanceAll(drawer);
      }).observe(drawer, {
        childList: true,
        subtree: true
      });
    }
  })();
})();
