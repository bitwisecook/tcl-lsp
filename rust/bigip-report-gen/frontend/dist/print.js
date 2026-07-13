// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/bigip-report-gen/frontend/src — DO NOT EDIT; edit the .ts source.
"use strict";
(() => {
  // src/pages/print.ts
  (function() {
    "use strict";
    var topbar = document.querySelector(".topbar-actions");
    if (!topbar) return;
    function esc(s) {
      return String(s).replace(/[&<>"]/g, function(c) {
        return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c];
      });
    }
    var devices = Array.prototype.slice.call(document.querySelectorAll("article.device[data-dev]"));
    function deviceLabel(dev, i) {
      var t = document.querySelector('.dev-tab[data-dev="' + dev.dataset.dev + '"]');
      var name = t && t.textContent.trim() || dev.getAttribute("data-name") || "";
      return name || "Device " + (i + 1);
    }
    var TOOLS = { console: true, listener: true };
    var sections = [];
    var seen = {};
    devices.forEach(function(dev) {
      dev.querySelectorAll(".tabs .tab[data-panel]").forEach(function(tab) {
        var key = tab.dataset.panel;
        if (seen[key] || TOOLS[key]) return;
        seen[key] = true;
        var label = tab.cloneNode(true);
        var n = label.querySelector(".n");
        if (n) n.remove();
        sections.push({ key, label: label.textContent.trim() || key });
      });
    });
    var hasConsole = !!document.getElementById("f5-wasm") && typeof wasm_bindgen !== "undefined";
    var btn = document.createElement("button");
    btn.id = "printBtn";
    btn.title = "Print / export to PDF";
    btn.setAttribute("aria-label", "Print");
    btn.textContent = "\u{1F5A8}";
    topbar.appendChild(btn);
    var scrim = document.createElement("div");
    scrim.className = "print-scrim";
    scrim.innerHTML = '<div class="print-dialog" role="dialog" aria-modal="true" aria-label="Print options"><h2>Print / export to PDF</h2><p class="print-sub">Everything is selected by default. Choose paper size (A4 or Letter, portrait) in your browser\u2019s print dialog. All diagrams are included; each section starts on a new page.</p><div class="print-shortcuts"><button type="button" data-all="1">Print everything</button><button type="button" data-current="1">Current view only</button></div>' + (devices.length > 1 ? '<fieldset><legend>Devices</legend><div class="print-grid print-devices">' + devices.map(function(d, i) {
      return '<label><input type="checkbox" class="pdev" value="' + i + '" checked> ' + esc(deviceLabel(d, i)) + "</label>";
    }).join("") + "</div></fieldset>" : "") + '<fieldset><legend>Sections</legend><div class="print-grid print-sections">' + sections.map(function(s) {
      return '<label><input type="checkbox" class="psec" value="' + esc(s.key) + '" checked> ' + esc(s.label) + "</label>";
    }).join("") + "</div></fieldset>" + (hasConsole ? '<fieldset><legend>iRules</legend><label><input type="checkbox" class="pfmt"> Format iRule code</label><label><input type="checkbox" class="pdiag"> Show diagnostics + optimiser suggestions</label></fieldset>' : "") + '<div class="print-actions"><button type="button" class="print-cancel">Cancel</button><button type="button" class="print-go">Print</button></div></div>';
    document.body.appendChild(scrim);
    function q(sel) {
      return scrim.querySelector(sel);
    }
    function qa(sel) {
      return Array.prototype.slice.call(scrim.querySelectorAll(sel));
    }
    function open() {
      scrim.classList.add("open");
    }
    function close() {
      scrim.classList.remove("open");
    }
    btn.addEventListener("click", open);
    q(".print-cancel").addEventListener("click", close);
    scrim.addEventListener("click", function(e) {
      if (e.target === scrim) close();
    });
    q("[data-all]").addEventListener("click", function() {
      qa(".pdev, .psec").forEach(function(c) {
        c.checked = true;
      });
    });
    q("[data-current]").addEventListener("click", function() {
      qa(".pdev").forEach(function(c) {
        c.checked = devices[+c.value].classList.contains("active");
      });
      var activePanel = null;
      var actDev = document.querySelector(".device.active") || devices[0];
      var ap = actDev && actDev.querySelector(".panel.active");
      if (ap) activePanel = ap.dataset.panel;
      qa(".psec").forEach(function(c) {
        c.checked = c.value === activePanel;
      });
    });
    var restore = [];
    function remember(fn) {
      restore.push(fn);
    }
    function chosenDeviceIdxs() {
      var boxes = qa(".pdev");
      if (!boxes.length) return devices.map(function(_, i) {
        return i;
      });
      return boxes.filter(function(c) {
        return c.checked;
      }).map(function(c) {
        return +c.value;
      });
    }
    function chosenSections() {
      return qa(".psec").filter(function(c) {
        return c.checked;
      }).map(function(c) {
        return c.value;
      });
    }
    function initWasm() {
      var wasmEl = document.getElementById("f5-wasm");
      if (!wasmEl) return Promise.reject(new Error("no wasm"));
      if (!window.__f5qReady) {
        var bin = atob(wasmEl.textContent.trim());
        var u = new Uint8Array(bin.length);
        for (var i = 0; i < bin.length; i++) u[i] = bin.charCodeAt(i);
        window.__f5qReady = wasm_bindgen(u);
      }
      return window.__f5qReady;
    }
    function applyIruleOptions(deviceEls, doFmt, doDiag) {
      if (!doFmt && !doDiag) return Promise.resolve();
      return initWasm().then(function() {
        deviceEls.forEach(function(dev) {
          dev.querySelectorAll('.panel[data-panel="rules"] pre.code.tcl').forEach(function(pre) {
            var original = pre.innerHTML;
            remember(function() {
              pre.innerHTML = original;
            });
            try {
              var src = pre.textContent;
              if (doDiag) {
                var res = JSON.parse(wasm_bindgen.analyze_irule(src));
                pre.innerHTML = res.html;
              } else if (doFmt) {
                pre.innerHTML = wasm_bindgen.format_irule(src);
              }
            } catch (e) {
            }
          });
        });
      }).catch(function() {
      });
    }
    function run() {
      var devIdxs = chosenDeviceIdxs();
      var secs = chosenSections();
      if (!devIdxs.length || !secs.length) {
        close();
        return;
      }
      var deviceEls = devIdxs.map(function(i) {
        return devices[i];
      });
      var doFmt = q(".pfmt") && q(".pfmt").checked;
      var doDiag = q(".pdiag") && q(".pdiag").checked;
      close();
      deviceEls.forEach(function(dev) {
        var activeTab = dev.querySelector(".tab.active");
        var activePanel = dev.querySelector(".panel.active");
        remember(function() {
          dev.querySelectorAll(".tab").forEach(function(t) {
            t.classList.toggle("active", t === activeTab);
          });
          dev.querySelectorAll(".panel").forEach(function(p) {
            p.classList.toggle("active", p === activePanel);
          });
        });
        secs.forEach(function(key) {
          var tab = dev.querySelector('.tab[data-panel="' + key + '"]');
          if (tab) tab.click();
        });
      });
      var activeDevice = document.querySelector(".device.active");
      remember(function() {
        devices.forEach(function(d) {
          d.classList.toggle("active", d === activeDevice);
        });
      });
      applyIruleOptions(deviceEls, doFmt, doDiag).then(function() {
        whenDrawn(deviceEls, secs, function() {
          markAndPrint(deviceEls, secs);
        });
      });
    }
    function whenDrawn(deviceEls, secs, done) {
      var deadline = Date.now() + 8e3;
      (function poll() {
        var undrawn = 0;
        deviceEls.forEach(function(dev) {
          secs.forEach(function(key) {
            var panel = dev.querySelector('.panel[data-panel="' + key + '"]');
            if (!panel) return;
            panel.querySelectorAll(".diag-host").forEach(function(host) {
              if (!host.querySelector("svg") && !host.textContent.trim()) undrawn++;
            });
          });
        });
        if (!undrawn || Date.now() > deadline) {
          done();
          return;
        }
        setTimeout(poll, 150);
      })();
    }
    function isEmptySection(dev, key) {
      var tab = dev.querySelector('.tab[data-panel="' + key + '"]');
      var badge = tab && tab.querySelector(".n");
      if (badge && /^\s*0\s*$/.test(badge.textContent)) return true;
      var panel = dev.querySelector('.panel[data-panel="' + key + '"]');
      if (!panel) return true;
      if (panel.querySelector("tbody tr, svg, pre, .card, .cert-row, .app-detail")) return false;
      return !panel.textContent.trim();
    }
    function parkInSheet(nodes) {
      if (!nodes.length) return;
      var sheet = document.createElement("table");
      sheet.className = "print-sheet";
      sheet.innerHTML = '<thead class="print-sheet-head"><tr><th></th></tr></thead><tfoot class="print-sheet-foot"><tr><td></td></tr></tfoot><tbody><tr><td class="print-sheet-body"></td></tr></tbody>';
      var cell = sheet.querySelector(".print-sheet-body");
      nodes[0].parentNode.insertBefore(sheet, nodes[0]);
      [
        [".print-running-head", "thead th"],
        [".print-running-foot", "tfoot td"]
      ].forEach(function(pair) {
        var el = document.querySelector(pair[0]);
        if (!el) return;
        var parent = el.parentNode, next = el.nextSibling;
        remember(function() {
          parent.insertBefore(el, next);
        });
        sheet.querySelector(pair[1]).appendChild(el);
      });
      nodes.forEach(function(n) {
        var parent = n.parentNode, next = n.nextSibling;
        remember(function() {
          parent.insertBefore(n, next);
        });
        cell.appendChild(n);
      });
      remember(function() {
        if (sheet.parentNode) sheet.parentNode.removeChild(sheet);
      });
    }
    function prepArchitecture() {
      var arch = document.getElementById("architecture");
      if (!arch) return null;
      arch.classList.add("print-include");
      remember(function() {
        arch.classList.remove("print-include");
      });
      var h = document.createElement("div");
      h.className = "print-heading";
      h.textContent = "Architecture";
      arch.insertBefore(h, arch.firstChild);
      remember(function() {
        if (h.parentNode) h.parentNode.removeChild(h);
      });
      var ta = arch.querySelector(".arch-editor-ta");
      var def = ta && ta.value ? ta.value.trim() : "";
      if (def) {
        var wrap = document.createElement("div");
        wrap.className = "print-arch-def";
        wrap.innerHTML = '<div class="print-arch-def-lbl">Architecture definition (manifest DSL)</div><pre class="code"></pre>';
        wrap.querySelector("pre").textContent = def;
        arch.appendChild(wrap);
        remember(function() {
          if (wrap.parentNode) wrap.parentNode.removeChild(wrap);
        });
      }
      return arch;
    }
    function markAndPrint(deviceEls, secs) {
      var secSet = {};
      secs.forEach(function(s) {
        secSet[s] = true;
      });
      deviceEls.forEach(function(dev, di) {
        dev.classList.add("print-include");
        remember(function() {
          dev.classList.remove("print-include");
        });
        secs.forEach(function(key) {
          var panel = dev.querySelector('.panel[data-panel="' + key + '"]');
          if (!panel || isEmptySection(dev, key)) return;
          panel.classList.add("print-include");
          remember(function() {
            panel.classList.remove("print-include");
          });
          var h = document.createElement("div");
          h.className = "print-heading";
          var secLabel = (sections.filter(function(s) {
            return s.key === key;
          })[0] || {}).label || key;
          h.innerHTML = esc(secLabel) + (deviceEls.length > 1 ? ' <span class="print-heading-dev">\u2014 ' + esc(deviceLabel(dev, di)) + "</span>" : "");
          panel.insertBefore(h, panel.firstChild);
          remember(function() {
            if (h.parentNode) h.parentNode.removeChild(h);
          });
          panel.querySelectorAll("tr.detail").forEach(function(tr) {
            tr.classList.add("print-include");
            remember(function() {
              tr.classList.remove("print-include");
            });
          });
        });
      });
      var summary = document.querySelector("section.summary");
      if (summary) {
        summary.classList.add("print-include");
        remember(function() {
          summary.classList.remove("print-include");
        });
      }
      var arch = prepArchitecture();
      parkInSheet([summary, arch].filter(Boolean).concat(deviceEls));
      document.documentElement.classList.add("printing");
      remember(function() {
        document.documentElement.classList.remove("printing");
      });
      var done = function() {
        window.removeEventListener("afterprint", done);
        for (var i = restore.length - 1; i >= 0; i--) {
          try {
            restore[i]();
          } catch (e) {
          }
        }
        restore = [];
      };
      window.addEventListener("afterprint", done);
      window.print();
      setTimeout(function() {
        if (restore.length) done();
      }, 6e4);
    }
    q(".print-go").addEventListener("click", run);
    document.addEventListener("keydown", function(e) {
      if ((e.ctrlKey || e.metaKey) && (e.key === "p" || e.key === "P")) {
        e.preventDefault();
        open();
      }
    });
  })();
})();
