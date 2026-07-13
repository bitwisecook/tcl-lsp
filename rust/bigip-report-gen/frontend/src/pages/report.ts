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

// f5report — client-side interactivity. No dependencies, no network.
(function () {
  "use strict";

  // --- iOS Safari tap → navigation ------------------------------------------
  // Mobile Safari only synthesises a `click` for elements it considers natively
  // clickable; taps on <div>/<span> controls (the count chips, object links,
  // expandable rows) are frequently dropped even with cursor:pointer. Rather
  // than rely on that synthesis, translate a real *tap* (a touch that doesn't
  // move into a scroll) into the element's click ourselves, and preventDefault
  // so the delayed ghost click can't double-fire. Desktop (mouse) never hits
  // this path, so click handlers keep working there unchanged.
  (function iosTapBridge() {
    var ACTIONABLE =
      ".tab,.dev-tab,.chip[data-target],.objlink,.ref-obj,[data-oid],[data-oref],tr.expandable";
    var sx = 0, sy = 0, moved = false, t0 = 0;
    document.addEventListener("touchstart", function (e) {
      if (e.touches.length !== 1) { moved = true; return; }
      moved = false; t0 = Date.now();
      sx = e.touches[0].clientX; sy = e.touches[0].clientY;
    }, { passive: true });
    document.addEventListener("touchmove", function (e) {
      var t = e.touches[0]; if (!t) return;
      if (Math.abs(t.clientX - sx) > 10 || Math.abs(t.clientY - sy) > 10) moved = true;
    }, { passive: true });
    document.addEventListener("touchend", function (e) {
      if (moved || Date.now() - t0 > 700) return;
      var tgt = e.target;
      if (!tgt || typeof tgt.closest !== "function") return;
      // Let native form controls / real links handle their own taps.
      if (tgt.closest("input,select,textarea,option,label,a[href]")) return;
      var el = tgt.closest(ACTIONABLE);
      if (!el) return;
      e.preventDefault();      // cancel the simulated mouse/click that follows
      if (typeof el.click === "function") el.click();
    }, { passive: false });
  })();

  // --- theme toggle: auto -> light -> dark, remembered in localStorage ------
  // Each click MUST look like it did something. The three modes are auto /
  // light / dark, but on a light-preference OS "auto" and "light" render the
  // same, so cycling between them used to feel like a dead button. Give every
  // state a distinct glyph + label so the click is always visibly acknowledged,
  // even when the two light-looking modes are visually identical.
  var root = document.documentElement;
  var order = ["auto", "light", "dark"];
  var THEME_ICON = { auto: "◐", light: "☀", dark: "☾" };
  var THEME_LABEL = { auto: "Auto (match system)", light: "Light", dark: "Dark" };
  var toggle = document.getElementById("themeToggle");
  function reflectTheme(mode) {
    if (!toggle) return;
    toggle.textContent = THEME_ICON[mode] || THEME_ICON.auto;
    toggle.title = "Theme: " + (THEME_LABEL[mode] || mode) + " — click to change";
    toggle.setAttribute("aria-label", "Theme: " + (THEME_LABEL[mode] || mode));
  }
  var current = "auto";
  try {
    var saved = localStorage.getItem("f5report-theme");
    if (saved && order.indexOf(saved) !== -1) current = saved;
  } catch (e) {}
  root.setAttribute("data-theme", current);
  reflectTheme(current);
  if (toggle) {
    toggle.addEventListener("click", function () {
      current = order[(order.indexOf(current) + 1) % order.length];
      root.setAttribute("data-theme", current);
      try { localStorage.setItem("f5report-theme", current); } catch (e) {}
      reflectTheme(current);
    });
  }

  // --- device switcher ------------------------------------------------------
  document.querySelectorAll(".dev-tab").forEach(function (btn) {
    btn.addEventListener("click", function () {
      var id = btn.dataset.dev;
      document.querySelectorAll(".dev-tab").forEach(function (b) { b.classList.toggle("active", b === btn); });
      document.querySelectorAll(".device").forEach(function (d) {
        d.classList.toggle("active", d.dataset.dev === id);
      });
    });
  });

  // --- section tabs (scoped per device) -------------------------------------
  document.querySelectorAll(".device").forEach(function (device) {
    device.querySelectorAll(".tab").forEach(function (tab) {
      function activate() {
        var name = tab.dataset.panel;
        device.querySelectorAll(".tab").forEach(function (t) { t.classList.toggle("active", t === tab); });
        device.querySelectorAll(".panel").forEach(function (p) {
          p.classList.toggle("active", p.dataset.panel === name);
        });
        // Reveal the tapped tab within the (mobile) horizontally-scrollable
        // strip, and — if the tab strip has scrolled above the viewport — bring
        // it back so the freshly-shown panel is visible. Without this a tab
        // change on a small screen can feel like nothing happened.
        tab.scrollIntoView({ inline: "center", block: "nearest" });
        var strip = device.querySelector(".tabs");
        if (strip && strip.getBoundingClientRect().top < 0) {
          strip.scrollIntoView({ block: "start", behavior: "smooth" });
        }
      }
      tab.addEventListener("click", activate);
    });
  });

  // --- partition filter: narrow every table to one partition, but always keep
  //     shared /Common objects visible (they are referenceable from anywhere) --
  document.querySelectorAll(".device").forEach(function (device) {
    var sel = device.querySelector(".partition-filter");
    if (!sel) return;
    // The partition is the first `/<name>/` segment of the object's full path,
    // which is the second token of each row's data-search (name then full-path).
    function rowPartition(row) {
      var ds = row.getAttribute("data-search") || "";
      var m = ds.match(/(?:^|\s)\/([^/\s]+)\//);
      return m ? m[1] : null;
    }
    function apply() {
      var val = sel.value;
      device.querySelectorAll(".grid tbody tr.searchable").forEach(function (row) {
        var part = rowPartition(row);
        var show = val === "__all__" || part === null || part === val || part === "Common";
        row.classList.toggle("part-hidden", !show);
        var det = row.nextElementSibling;
        if (det && det.classList.contains("detail")) {
          det.classList.toggle("part-hidden", !show);
        }
      });
    }
    sel.addEventListener("change", apply);
  });

  // --- the mark + title are the way home ------------------------------------
  // Clicking either returns the report to the view it opened in. The opening
  // view is *recorded* here rather than assumed, so it stays right if the
  // default tab ever changes (a report with front-matter opens on that tab, not
  // on Virtual Servers). A reload would be the other way to do this, but a
  // report opened from a blob: URL — straight out of the in-browser generator —
  // cannot be reloaded, and this keeps it working there too.
  (function brandHome() {
    var homes = document.querySelectorAll(".brand-home");
    if (!homes.length) return;

    var openDevice = document.querySelector(".dev-tab.active");
    var openTabs = [];
    document.querySelectorAll("article.device[data-dev]").forEach(function (dev) {
      openTabs.push({ dev: dev, tab: dev.querySelector(".tab.active") });
    });

    function reset(ev) {
      if (ev) ev.preventDefault();
      // the device that was showing, and the tab each device was showing
      if (openDevice) openDevice.click();
      openTabs.forEach(function (o) { if (o.tab) o.tab.click(); });
      // filters and drilldowns: search, partition, system rows, expanded rows
      var search = document.getElementById("globalSearch");
      if (search && search.value) {
        search.value = "";
        search.dispatchEvent(new Event("input", { bubbles: true }));
      }
      document.querySelectorAll(".partition-filter").forEach(function (sel) {
        if (sel.selectedIndex !== 0) {
          sel.selectedIndex = 0;
          sel.dispatchEvent(new Event("change", { bubbles: true }));
        }
      });
      document.querySelectorAll("article.device.show-system-on").forEach(function (d) {
        d.classList.remove("show-system-on");
      });
      document.querySelectorAll("tr.expandable.open, tr.detail.open").forEach(function (r) {
        r.classList.remove("open");
      });
      // any open object drawer, and back to the top
      var close = document.querySelector("#objDrawer .drawer-close");
      if (close && document.getElementById("objDrawer").classList.contains("open")) close.click();
      window.scrollTo({ top: 0, behavior: "smooth" });
    }

    homes.forEach(function (a) { a.addEventListener("click", reset); });
  })();

  // --- expandable rows (pool members, iRule bodies, data-group records) -----
  document.querySelectorAll("tr.expandable").forEach(function (row) {
    var detail = row.nextElementSibling;
    if (!detail || !detail.classList.contains("detail")) return;
    row.addEventListener("click", function (ev) {
      if (ev.target.tagName === "A") return;
      row.classList.toggle("open");
      detail.classList.toggle("open");
    });
  });

  // --- summary count boxes jump to their tab -------------------------------
  document.querySelectorAll(".chip[data-target]").forEach(function (chip) {
    function go() {
      var panel = chip.dataset.target;
      var device = document.querySelector(".device.active") || document.querySelector(".device");
      if (!device) return;
      var tab = device.querySelector('.tab[data-panel="' + panel + '"]');
      if (tab) {
        tab.click();
        (device.querySelector(".tabs") || device).scrollIntoView({ behavior: "smooth", block: "start" });
      }
    }
    chip.addEventListener("click", go);
    chip.addEventListener("keydown", function (e) {
      if (e.key === "Enter" || e.key === " ") { e.preventDefault(); go(); }
    });
  });

  // --- search: "/" focuses the global box; the graph-aware filtering itself
  //     lives in topology.js (it needs the reference graph). ------------------
  var search = document.getElementById("globalSearch");
  if (search) {
    document.addEventListener("keydown", function (e) {
      if (e.key === "/" && document.activeElement !== search) {
        e.preventDefault();
        search.focus();
      }
    });
  }

  // --- diagram viewer: click a diagram to expand, then pan/zoom -------------
  //     Google-Maps-style: drag to pan, wheel / pinch / ± to zoom, double-click
  //     to zoom in, Esc to close. Covers every diagram host (iRule flow, app
  //     pipes/flows, architecture, topology graphs), including the ones
  //     built lazily when a tab is first shown (a MutationObserver decorates
  //     them as their <svg> arrives).
  (function diagramViewer() {
    var HOSTS =
      ".irule-flow-diagram,.app-pipe-diagram,.app-flow-diagram,.app-obj-flow,.arch-diagram,.diag-host";
    var overlay, viewport, canvas;
    var scale = 1, tx = 0, ty = 0, fit = 1, nat = { w: 0, h: 0 };
    var pointers = {};

    function clamp(s) { return Math.max(fit * 0.3, Math.min(fit * 30, s)); }
    function apply() {
      canvas.style.transform = "translate(" + tx + "px," + ty + "px) scale(" + scale + ")";
    }
    // Zoom by `factor` keeping the point (mx,my) — in viewport pixels — fixed.
    function zoomAt(factor, mx, my) {
      var ns = clamp(scale * factor), f = ns / scale;
      tx = mx - (mx - tx) * f;
      ty = my - (my - ty) * f;
      scale = ns;
    }
    // Intrinsic diagram size: explicit px width/height, else the viewBox, else
    // the on-page rendered box. A "100%" attribute is not a pixel size.
    function natSize(svg) {
      function px(v) {
        if (v == null) return NaN;
        v = String(v).trim();
        return /%$/.test(v) ? NaN : parseFloat(v);
      }
      var w = px(svg.getAttribute("width")), h = px(svg.getAttribute("height"));
      if (w > 0 && h > 0) return { w: w, h: h };
      var vb = svg.getAttribute("viewBox");
      if (vb) {
        var p = vb.split(/[\s,]+/), vw = parseFloat(p[2]), vh = parseFloat(p[3]);
        if (vw > 0 && vh > 0) return { w: vw, h: vh };
      }
      var r = svg.getBoundingClientRect();
      return { w: r.width || 900, h: r.height || 600 };
    }
    // Fit the whole diagram inside the window (contain), centered.
    function fitToWindow() {
      var vp = viewport.getBoundingClientRect();
      fit = Math.min(vp.width / nat.w, vp.height / nat.h) * 0.96;
      scale = fit;
      tx = (vp.width - nat.w * scale) / 2;
      ty = (vp.height - nat.h * scale) / 2;
      apply();
    }
    function onMove(e) {
      if (!(e.pointerId in pointers)) return;
      var ids = Object.keys(pointers), vp = viewport.getBoundingClientRect();
      if (ids.length >= 2) {
        // Two fingers: pan by the centroid, zoom by the change in spread.
        var pa = pointers[ids[0]], pb = pointers[ids[1]];
        var pd = Math.hypot(pa.x - pb.x, pa.y - pb.y);
        var pmx = (pa.x + pb.x) / 2, pmy = (pa.y + pb.y) / 2;
        pointers[e.pointerId] = { x: e.clientX, y: e.clientY };
        var a = pointers[ids[0]], b = pointers[ids[1]];
        var cd = Math.hypot(a.x - b.x, a.y - b.y);
        var cmx = (a.x + b.x) / 2, cmy = (a.y + b.y) / 2;
        tx += cmx - pmx; ty += cmy - pmy;
        if (pd > 0) zoomAt(cd / pd, cmx - vp.left, cmy - vp.top);
        apply();
      } else {
        var p = pointers[e.pointerId];
        tx += e.clientX - p.x; ty += e.clientY - p.y;
        pointers[e.pointerId] = { x: e.clientX, y: e.clientY };
        apply();
      }
    }
    function build() {
      overlay = document.createElement("div");
      overlay.className = "dv-overlay";
      overlay.hidden = true;
      overlay.innerHTML =
        '<div class="dv-toolbar">' +
        '<button type="button" class="dv-btn" data-act="out" title="Zoom out" aria-label="Zoom out">−</button>' +
        '<button type="button" class="dv-btn" data-act="reset" title="Fit to window">Fit</button>' +
        '<button type="button" class="dv-btn" data-act="in" title="Zoom in" aria-label="Zoom in">+</button>' +
        '<button type="button" class="dv-btn" data-act="close" title="Close (Esc)" aria-label="Close">✕</button>' +
        "</div>" +
        '<div class="dv-viewport"><div class="dv-canvas"></div></div>' +
        '<div class="dv-hint">drag to pan · scroll or pinch to zoom · double-click to zoom in</div>';
      document.body.appendChild(overlay);
      viewport = overlay.querySelector(".dv-viewport");
      canvas = overlay.querySelector(".dv-canvas");

      overlay.querySelector(".dv-toolbar").addEventListener("click", function (e) {
        var b = e.target.closest("[data-act]");
        if (!b) return;
        var vp = viewport.getBoundingClientRect();
        if (b.dataset.act === "close") close();
        else if (b.dataset.act === "reset") fitToWindow();
        else { zoomAt(b.dataset.act === "in" ? 1.4 : 1 / 1.4, vp.width / 2, vp.height / 2); apply(); }
      });
      viewport.addEventListener("wheel", function (e) {
        e.preventDefault();
        var vp = viewport.getBoundingClientRect();
        zoomAt(Math.exp(-e.deltaY * 0.0016), e.clientX - vp.left, e.clientY - vp.top);
        apply();
      }, { passive: false });
      viewport.addEventListener("dblclick", function (e) {
        var vp = viewport.getBoundingClientRect();
        zoomAt(1.6, e.clientX - vp.left, e.clientY - vp.top);
        apply();
      });
      viewport.addEventListener("pointerdown", function (e) {
        try { viewport.setPointerCapture(e.pointerId); } catch (_) {}
        pointers[e.pointerId] = { x: e.clientX, y: e.clientY };
        viewport.classList.add("grabbing");
      });
      viewport.addEventListener("pointermove", onMove);
      function up(e) {
        delete pointers[e.pointerId];
        try { viewport.releasePointerCapture(e.pointerId); } catch (_) {}
        if (!Object.keys(pointers).length) viewport.classList.remove("grabbing");
      }
      viewport.addEventListener("pointerup", up);
      viewport.addEventListener("pointercancel", up);
      document.addEventListener("keydown", function (e) {
        if (overlay && !overlay.hidden && e.key === "Escape") close();
      });
      window.addEventListener("resize", function () {
        if (overlay && !overlay.hidden) fitToWindow();
      });
    }
    function open(host) {
      var svg = host.querySelector("svg");
      if (!svg) return;
      if (!overlay) build();
      var clone = svg.cloneNode(true);
      clone.removeAttribute("style"); // drop the on-page max-width clamp
      nat = natSize(svg);
      canvas.textContent = "";
      canvas.style.width = nat.w + "px";
      canvas.style.height = nat.h + "px";
      canvas.appendChild(clone);
      pointers = {};
      overlay.hidden = false;
      document.body.classList.add("dv-open");
      fitToWindow();
    }
    function close() {
      overlay.hidden = true;
      document.body.classList.remove("dv-open");
      canvas.textContent = "";
    }

    function hasExpand(host) {
      for (var i = 0; i < host.children.length; i++) {
        if (host.children[i].classList && host.children[i].classList.contains("dv-expand")) return true;
      }
      return false;
    }
    // Add the corner expand button. Idempotent, and re-adds itself if a host
    // rebuilt its innerHTML (topology re-renders on filter changes).
    function decorate(host) {
      if (!host.querySelector("svg") || hasExpand(host)) return;
      if (getComputedStyle(host).position === "static") host.style.position = "relative";
      var btn = document.createElement("button");
      btn.type = "button";
      btn.className = "dv-expand";
      btn.title = "Expand — pan & zoom";
      btn.setAttribute("aria-label", "Expand diagram");
      btn.textContent = "⤢"; // ⤢
      btn.addEventListener("click", function (e) {
        e.preventDefault();
        e.stopPropagation();
        open(host);
      });
      host.appendChild(btn);
    }
    function decorateAll() {
      document.querySelectorAll(HOSTS).forEach(decorate);
    }

    decorateAll();
    if (window.MutationObserver) {
      var pending = false;
      new MutationObserver(function () {
        if (pending) return;
        pending = true;
        requestAnimationFrame(function () { pending = false; decorateAll(); });
      }).observe(document.body, { childList: true, subtree: true });
    }
  })();
})();
