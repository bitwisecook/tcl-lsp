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

// f5report — Print. A Print button opens a dialog to choose which devices and
// sections to print (everything by default), with optional iRule Diagnostics /
// Formatting. It force-renders lazily-drawn panels (topology / apps / forensics)
// and expands detail rows, then prints portrait A4/Letter (see print.css). All
// client-side; nothing leaves the page.
(function () {
  "use strict";

  var topbar = document.querySelector(".topbar-actions");
  if (!topbar) return;

  function esc(s) {
    return String(s).replace(/[&<>"]/g, function (c) {
      return { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c];
    });
  }

  // ---- Enumerate devices + sections from the DOM ------------------------
  // `[data-dev]` matters: the architecture diagram draws each device as an SVG
  // node that also carries `class="device"` (`g.elk-node.device`), so a bare
  // `.device` picks those up too — they would show up as phantom devices in the
  // dialog, and parking them in the print sheet would tear them out of their
  // <svg> and blank the diagram.
  var devices = Array.prototype.slice.call(document.querySelectorAll("article.device[data-dev]"));
  function deviceLabel(dev, i) {
    var t = document.querySelector('.dev-tab[data-dev="' + dev.dataset.dev + '"]');
    var name = (t && t.textContent.trim()) || dev.getAttribute("data-name") || "";
    return name || "Device " + (i + 1);
  }
  // Interactive tools rather than report content: the query console prints as an
  // empty prompt and the listener matcher as an empty form. Neither is offered
  // in the dialog, and print.css hides them for the raw Ctrl-P path too.
  var TOOLS = { console: true, listener: true };

  // Union of sections (data-panel) across all devices, first-seen order, with a
  // human label taken from the tab text (minus its count badge).
  var sections = [];
  var seen = {};
  devices.forEach(function (dev) {
    dev.querySelectorAll(".tabs .tab[data-panel]").forEach(function (tab) {
      var key = tab.dataset.panel;
      if (seen[key] || TOOLS[key]) return;
      seen[key] = true;
      var label = tab.cloneNode(true);
      var n = label.querySelector(".n"); if (n) n.remove();
      sections.push({ key: key, label: label.textContent.trim() || key });
    });
  });
  var hasConsole = !!document.getElementById("f5-wasm") && typeof wasm_bindgen !== "undefined";

  // ---- Build the Print button + dialog ----------------------------------
  var btn = document.createElement("button");
  btn.id = "printBtn";
  btn.title = "Print / export to PDF";
  btn.setAttribute("aria-label", "Print");
  btn.textContent = "🖨";
  topbar.appendChild(btn);

  var scrim = document.createElement("div");
  scrim.className = "print-scrim";
  scrim.innerHTML =
    '<div class="print-dialog" role="dialog" aria-modal="true" aria-label="Print options">' +
    "<h2>Print / export to PDF</h2>" +
    '<p class="print-sub">Everything is selected by default. Choose paper size (A4 or Letter, portrait) in your browser’s print dialog. All diagrams are included; each section starts on a new page.</p>' +
    '<div class="print-shortcuts">' +
    '<button type="button" data-all="1">Print everything</button>' +
    '<button type="button" data-current="1">Current view only</button>' +
    "</div>" +
    (devices.length > 1
      ? '<fieldset><legend>Devices</legend><div class="print-grid print-devices">' +
        devices
          .map(function (d, i) {
            return '<label><input type="checkbox" class="pdev" value="' + i + '" checked> ' + esc(deviceLabel(d, i)) + "</label>";
          })
          .join("") +
        "</div></fieldset>"
      : "") +
    '<fieldset><legend>Sections</legend><div class="print-grid print-sections">' +
    sections
      .map(function (s) {
        return '<label><input type="checkbox" class="psec" value="' + esc(s.key) + '" checked> ' + esc(s.label) + "</label>";
      })
      .join("") +
    "</div></fieldset>" +
    (hasConsole
      ? '<fieldset><legend>iRules</legend>' +
        '<label><input type="checkbox" class="pfmt"> Format iRule code</label>' +
        '<label><input type="checkbox" class="pdiag"> Show diagnostics + optimiser suggestions</label>' +
        "</fieldset>"
      : "") +
    '<div class="print-actions">' +
    '<button type="button" class="print-cancel">Cancel</button>' +
    '<button type="button" class="print-go">Print</button>' +
    "</div>" +
    "</div>";
  document.body.appendChild(scrim);

  function q(sel) { return scrim.querySelector(sel); }
  function qa(sel) { return Array.prototype.slice.call(scrim.querySelectorAll(sel)); }

  function open() { scrim.classList.add("open"); }
  function close() { scrim.classList.remove("open"); }
  btn.addEventListener("click", open);
  q(".print-cancel").addEventListener("click", close);
  scrim.addEventListener("click", function (e) { if (e.target === scrim) close(); });

  q("[data-all]").addEventListener("click", function () {
    qa(".pdev, .psec").forEach(function (c) { c.checked = true; });
  });
  q("[data-current]").addEventListener("click", function () {
    // only the on-screen active device + its active panel
    qa(".pdev").forEach(function (c) {
      c.checked = devices[+c.value].classList.contains("active");
    });
    var activePanel = null;
    var actDev = document.querySelector(".device.active") || devices[0];
    var ap = actDev && actDev.querySelector(".panel.active");
    if (ap) activePanel = ap.dataset.panel;
    qa(".psec").forEach(function (c) { c.checked = c.value === activePanel; });
  });

  // ---- The print run ----------------------------------------------------
  var restore = [];
  function remember(fn) { restore.push(fn); }

  function chosenDeviceIdxs() {
    var boxes = qa(".pdev");
    if (!boxes.length) return devices.map(function (_, i) { return i; }); // single device
    return boxes.filter(function (c) { return c.checked; }).map(function (c) { return +c.value; });
  }
  function chosenSections() {
    return qa(".psec").filter(function (c) { return c.checked; }).map(function (c) { return c.value; });
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

  // Apply Format / Diagnostics to every iRule body inside the print scope.
  function applyIruleOptions(deviceEls, doFmt, doDiag) {
    if (!doFmt && !doDiag) return Promise.resolve();
    return initWasm().then(function () {
      deviceEls.forEach(function (dev) {
        dev.querySelectorAll('.panel[data-panel="rules"] pre.code.tcl').forEach(function (pre) {
          var original = pre.innerHTML;
          remember(function () { pre.innerHTML = original; });
          try {
            var src = pre.textContent;
            if (doDiag) {
              var res = JSON.parse(wasm_bindgen.analyze_irule(src));
              pre.innerHTML = res.html;
            } else if (doFmt) {
              pre.innerHTML = wasm_bindgen.format_irule(src);
            }
          } catch (e) { /* leave original on failure */ }
        });
      });
    }).catch(function () { /* wasm unavailable — print as-is */ });
  }

  function run() {
    var devIdxs = chosenDeviceIdxs();
    var secs = chosenSections();
    if (!devIdxs.length || !secs.length) { close(); return; }
    var deviceEls = devIdxs.map(function (i) { return devices[i]; });
    var doFmt = q(".pfmt") && q(".pfmt").checked;
    var doDiag = q(".pdiag") && q(".pdiag").checked;
    close();

    // Force lazily-rendered panels (topology / apps / forensics) to draw by
    // activating each chosen tab; the rendered content persists in the DOM.
    deviceEls.forEach(function (dev) {
      var activeTab = dev.querySelector(".tab.active");
      var activePanel = dev.querySelector(".panel.active");
      remember(function () {
        dev.querySelectorAll(".tab").forEach(function (t) { t.classList.toggle("active", t === activeTab); });
        dev.querySelectorAll(".panel").forEach(function (p) { p.classList.toggle("active", p === activePanel); });
      });
      secs.forEach(function (key) {
        var tab = dev.querySelector('.tab[data-panel="' + key + '"]');
        if (tab) tab.click();
      });
    });
    var activeDevice = document.querySelector(".device.active");
    remember(function () {
      devices.forEach(function (d) { d.classList.toggle("active", d === activeDevice); });
    });

    applyIruleOptions(deviceEls, doFmt, doDiag).then(function () {
      whenDrawn(deviceEls, secs, function () { markAndPrint(deviceEls, secs); });
    });
  }

  // The topology / apps / architecture diagrams are laid out asynchronously, and
  // on a large estate that takes well past the fixed delay this used to allow —
  // printing early gave pages with an empty diagram box on them. Wait for the
  // panels being printed to actually have their <svg>, and cap the wait so a
  // host that legitimately never draws one ("no linked objects") can't hang the
  // print run.
  function whenDrawn(deviceEls, secs, done) {
    var deadline = Date.now() + 8000;
    (function poll() {
      var undrawn = 0;
      deviceEls.forEach(function (dev) {
        secs.forEach(function (key) {
          var panel = dev.querySelector('.panel[data-panel="' + key + '"]');
          if (!panel) return;
          panel.querySelectorAll(".diag-host").forEach(function (host) {
            if (!host.querySelector("svg") && !host.textContent.trim()) undrawn++;
          });
        });
      });
      if (!undrawn || Date.now() > deadline) { done(); return; }
      setTimeout(poll, 150);
    })();
  }

  // An empty section still costs a sheet of paper: its own forced page, a
  // heading, and nothing under it. A device with no monitors should simply not
  // have a Monitors page. Judged on the tab's count badge where it has one, and
  // otherwise on whether the panel drew anything at all.
  function isEmptySection(dev, key) {
    var tab = dev.querySelector('.tab[data-panel="' + key + '"]');
    var badge = tab && tab.querySelector(".n");
    if (badge && /^\s*0\s*$/.test(badge.textContent)) return true;
    var panel = dev.querySelector('.panel[data-panel="' + key + '"]');
    if (!panel) return true;
    if (panel.querySelector("tbody tr, svg, pre, .card, .cert-row, .app-detail")) return false;
    return !panel.textContent.trim();
  }

  // ---- Running head/foot on every printed page --------------------------
  // Only a table repeats content per page in Chrome: a position:fixed box that
  // sits in the page *margin* gets attributed to the neighbouring page (the head
  // lands at the foot of the page before it, the foot at the head of the page
  // after), and one inside the page box overlaps the content. So the print run
  // parks everything printable in a single-cell table whose thead/tfoot carry
  // the running title and attribution — the browser then repeats them on every
  // sheet and reserves the room. `.print-running-head` / `.print-running-foot`
  // in the template are the (hidden) source of that markup.
  // ONE sheet per section, not one sheet for the whole report. A forced page
  // break *inside* a table cell is the case Gecko gets wrong — Firefox emitted
  // page after blank page from it — so each section is parked in a table of its
  // own and the breaks fall between the tables, at block level, where both
  // engines agree. Each sheet repeats its own head/foot over the pages it spans.
  function parkInSheets(nodes) {
    if (!nodes.length) return;
    var headSrc = document.querySelector(".print-running-head");
    var footSrc = document.querySelector(".print-running-foot");

    // All sheets live side by side at top level: a panel parked in a table still
    // nested inside its device's table would put the forced break back inside a
    // cell, which is the very thing this avoids.
    var host = document.createElement("div");
    host.className = "print-host";
    nodes[0].parentNode.insertBefore(host, nodes[0]);

    // The running head/foot carry the marks as inline <svg>, so a clone per
    // sheet would repeat every gradient id. Re-namespace each copy's ids and the
    // url(#…) / href="#…" references that point at them.
    function runner(cell, src, tag) {
      if (!src || !cell) return;
      var clone = src.cloneNode(true);
      clone.innerHTML = clone.innerHTML.replace(
        /(id="|url\(#|href="#)([A-Za-z][\w.:-]*)/g,
        function (_m, prefix, id) { return prefix + tag + "-" + id; }
      );
      cell.appendChild(clone);
    }

    nodes.forEach(function (n, i) {
      var sheet = document.createElement("table");
      sheet.className = "print-sheet";
      sheet.innerHTML =
        '<thead class="print-sheet-head"><tr><th></th></tr></thead>' +
        '<tfoot class="print-sheet-foot"><tr><td></td></tr></tfoot>' +
        '<tbody><tr><td class="print-sheet-body"></td></tr></tbody>';
      runner(sheet.querySelector("thead th"), headSrc, "sh" + i);
      runner(sheet.querySelector("tfoot td"), footSrc, "sf" + i);

      var parent = n.parentNode, next = n.nextSibling;
      remember(function () { parent.insertBefore(n, next); });
      host.appendChild(sheet);
      sheet.querySelector(".print-sheet-body").appendChild(n);
    });

    // Unwound first (restore runs last-registered-first): the host and its
    // tables go, then every node is put back where it came from — panels before
    // the device they belong to, since they were registered after it.
    remember(function () { if (host.parentNode) host.parentNode.removeChild(host); });
  }

  // The cross-device architecture view is a top-level section, not a device tab,
  // so it needs its own printed heading — and its manifest lives in an editable
  // textarea that would print as an empty box, so the definition is re-emitted
  // as a code block.
  function prepArchitecture() {
    var arch = document.getElementById("architecture");
    if (!arch) return null;
    arch.classList.add("print-include");
    remember(function () { arch.classList.remove("print-include"); });

    var h = document.createElement("div");
    h.className = "print-heading";
    h.textContent = "Architecture";
    arch.insertBefore(h, arch.firstChild);
    remember(function () { if (h.parentNode) h.parentNode.removeChild(h); });

    var ta = arch.querySelector(".arch-editor-ta");
    var def = ta && ta.value ? ta.value.trim() : "";
    if (def) {
      var wrap = document.createElement("div");
      wrap.className = "print-arch-def";
      wrap.innerHTML =
        '<div class="print-arch-def-lbl">Architecture definition (manifest DSL)</div>' +
        '<pre class="code"></pre>';
      wrap.querySelector("pre").textContent = def;
      arch.appendChild(wrap);
      remember(function () { if (wrap.parentNode) wrap.parentNode.removeChild(wrap); });
    }
    return arch;
  }

  function markAndPrint(deviceEls, secs) {
    var secSet = {};
    secs.forEach(function (s) { secSet[s] = true; });

    deviceEls.forEach(function (dev, di) {
      dev.classList.add("print-include");
      remember(function () { dev.classList.remove("print-include"); });
      secs.forEach(function (key) {
        var panel = dev.querySelector('.panel[data-panel="' + key + '"]');
        if (!panel || isEmptySection(dev, key)) return;
        panel.classList.add("print-include");
        remember(function () { panel.classList.remove("print-include"); });
        // printed section heading
        var h = document.createElement("div");
        h.className = "print-heading";
        var secLabel = (sections.filter(function (s) { return s.key === key; })[0] || {}).label || key;
        h.innerHTML = esc(secLabel) +
          (deviceEls.length > 1 ? ' <span class="print-heading-dev">— ' + esc(deviceLabel(dev, di)) + "</span>" : "");
        panel.insertBefore(h, panel.firstChild);
        remember(function () { if (h.parentNode) h.parentNode.removeChild(h); });
        // expand detail rows so their content prints
        panel.querySelectorAll("tr.detail").forEach(function (tr) {
          tr.classList.add("print-include");
          remember(function () { tr.classList.remove("print-include"); });
        });
      });
    });

    // Linearise: the estate summary, the architecture view (when the report has
    // one), then each device — its heading block, then one section at a time —
    // in document order, so the printed page follows the on-screen one. Each of
    // these becomes a sheet of its own, and a sheet is a page.
    var summary = document.querySelector("section.summary");
    if (summary) {
      summary.classList.add("print-include");
      remember(function () { summary.classList.remove("print-include"); });
    }
    var arch = prepArchitecture();

    var order = [];
    if (summary) order.push(summary);
    if (arch) order.push(arch);
    deviceEls.forEach(function (dev) {
      order.push(dev);
      secs.forEach(function (key) {
        var panel = dev.querySelector('.panel[data-panel="' + key + '"]');
        if (panel && panel.classList.contains("print-include")) order.push(panel);
      });
    });
    parkInSheets(order);

    document.documentElement.classList.add("printing");
    remember(function () { document.documentElement.classList.remove("printing"); });

    var done = function () {
      window.removeEventListener("afterprint", done);
      // unwind everything (last-registered first)
      for (var i = restore.length - 1; i >= 0; i--) { try { restore[i](); } catch (e) {} }
      restore = [];
    };
    window.addEventListener("afterprint", done);
    window.print();
    // Safari/Firefox sometimes skip afterprint if the dialog is cancelled fast;
    // a fallback cleanup keeps the page usable.
    setTimeout(function () { if (restore.length) done(); }, 60000);
  }

  q(".print-go").addEventListener("click", run);

  // Ctrl/Cmd-P opens the chooser instead of the raw browser dialog.
  document.addEventListener("keydown", function (e) {
    if ((e.ctrlKey || e.metaKey) && (e.key === "p" || e.key === "P")) {
      e.preventDefault();
      open();
    }
  });
})();
