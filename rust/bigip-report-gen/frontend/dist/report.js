// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/bigip-report-gen/frontend/src — DO NOT EDIT; edit the .ts source.
"use strict";
(() => {
  // src/pages/report.ts
  (function() {
    "use strict";
    (function iosTapBridge() {
      var ACTIONABLE = ".tab,.dev-tab,.chip[data-target],.objlink,.ref-obj,[data-oid],[data-oref],tr.expandable";
      var sx = 0, sy = 0, moved = false, t0 = 0;
      document.addEventListener("touchstart", function(e) {
        if (e.touches.length !== 1) {
          moved = true;
          return;
        }
        moved = false;
        t0 = Date.now();
        sx = e.touches[0].clientX;
        sy = e.touches[0].clientY;
      }, { passive: true });
      document.addEventListener("touchmove", function(e) {
        var t = e.touches[0];
        if (!t) return;
        if (Math.abs(t.clientX - sx) > 10 || Math.abs(t.clientY - sy) > 10) moved = true;
      }, { passive: true });
      document.addEventListener("touchend", function(e) {
        if (moved || Date.now() - t0 > 700) return;
        var tgt = e.target;
        if (!tgt || typeof tgt.closest !== "function") return;
        if (tgt.closest("input,select,textarea,option,label,a[href]")) return;
        var el = tgt.closest(ACTIONABLE);
        if (!el) return;
        e.preventDefault();
        if (typeof el.click === "function") el.click();
      }, { passive: false });
    })();
    var root = document.documentElement;
    var order = ["auto", "light", "dark"];
    var THEME_ICON = { auto: "\u25D0", light: "\u2600", dark: "\u263E" };
    var THEME_LABEL = { auto: "Auto (match system)", light: "Light", dark: "Dark" };
    var toggle = document.getElementById("themeToggle");
    function reflectTheme(mode) {
      if (!toggle) return;
      toggle.textContent = THEME_ICON[mode] || THEME_ICON.auto;
      toggle.title = "Theme: " + (THEME_LABEL[mode] || mode) + " \u2014 click to change";
      toggle.setAttribute("aria-label", "Theme: " + (THEME_LABEL[mode] || mode));
    }
    var current = "auto";
    try {
      var saved = localStorage.getItem("f5report-theme");
      if (saved && order.indexOf(saved) !== -1) current = saved;
    } catch (e) {
    }
    root.setAttribute("data-theme", current);
    reflectTheme(current);
    if (toggle) {
      toggle.addEventListener("click", function() {
        current = order[(order.indexOf(current) + 1) % order.length];
        root.setAttribute("data-theme", current);
        try {
          localStorage.setItem("f5report-theme", current);
        } catch (e) {
        }
        reflectTheme(current);
      });
    }
    document.querySelectorAll(".dev-tab").forEach(function(btn) {
      btn.addEventListener("click", function() {
        var id = btn.dataset.dev;
        document.querySelectorAll(".dev-tab").forEach(function(b) {
          b.classList.toggle("active", b === btn);
        });
        document.querySelectorAll(".device").forEach(function(d) {
          d.classList.toggle("active", d.dataset.dev === id);
        });
      });
    });
    document.querySelectorAll(".device").forEach(function(device) {
      device.querySelectorAll(".tab").forEach(function(tab) {
        function activate() {
          var name = tab.dataset.panel;
          device.querySelectorAll(".tab").forEach(function(t) {
            t.classList.toggle("active", t === tab);
          });
          device.querySelectorAll(".panel").forEach(function(p) {
            p.classList.toggle("active", p.dataset.panel === name);
          });
          tab.scrollIntoView({ inline: "center", block: "nearest" });
          var strip = device.querySelector(".tabs");
          if (strip && strip.getBoundingClientRect().top < 0) {
            strip.scrollIntoView({ block: "start", behavior: "smooth" });
          }
        }
        tab.addEventListener("click", activate);
      });
    });
    document.querySelectorAll(".device").forEach(function(device) {
      var sel = device.querySelector(".partition-filter");
      if (!sel) return;
      function rowPartition(row) {
        var ds = row.getAttribute("data-search") || "";
        var m = ds.match(/(?:^|\s)\/([^/\s]+)\//);
        return m ? m[1] : null;
      }
      function apply() {
        var val = sel.value;
        device.querySelectorAll(".grid tbody tr.searchable").forEach(function(row) {
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
    (function brandHome() {
      var homes = document.querySelectorAll(".brand-home");
      if (!homes.length) return;
      var openDevice = document.querySelector(".dev-tab.active");
      var openTabs = [];
      document.querySelectorAll("article.device[data-dev]").forEach(function(dev) {
        openTabs.push({ dev, tab: dev.querySelector(".tab.active") });
      });
      function reset(ev) {
        if (ev) ev.preventDefault();
        if (openDevice) openDevice.click();
        openTabs.forEach(function(o) {
          if (o.tab) o.tab.click();
        });
        var search2 = document.getElementById("globalSearch");
        if (search2 && search2.value) {
          search2.value = "";
          search2.dispatchEvent(new Event("input", { bubbles: true }));
        }
        document.querySelectorAll(".partition-filter").forEach(function(sel) {
          if (sel.selectedIndex !== 0) {
            sel.selectedIndex = 0;
            sel.dispatchEvent(new Event("change", { bubbles: true }));
          }
        });
        document.querySelectorAll("article.device.show-system-on").forEach(function(d) {
          d.classList.remove("show-system-on");
        });
        document.querySelectorAll("tr.expandable.open, tr.detail.open").forEach(function(r) {
          r.classList.remove("open");
        });
        var close = document.querySelector("#objDrawer .drawer-close");
        if (close && document.getElementById("objDrawer").classList.contains("open")) close.click();
        window.scrollTo({ top: 0, behavior: "smooth" });
      }
      homes.forEach(function(a) {
        a.addEventListener("click", reset);
      });
    })();
    document.querySelectorAll("tr.expandable").forEach(function(row) {
      var detail = row.nextElementSibling;
      if (!detail || !detail.classList.contains("detail")) return;
      row.addEventListener("click", function(ev) {
        if (ev.target.tagName === "A") return;
        row.classList.toggle("open");
        detail.classList.toggle("open");
      });
    });
    document.querySelectorAll(".chip[data-target]").forEach(function(chip) {
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
      chip.addEventListener("keydown", function(e) {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          go();
        }
      });
    });
    var search = document.getElementById("globalSearch");
    if (search) {
      document.addEventListener("keydown", function(e) {
        if (e.key === "/" && document.activeElement !== search) {
          e.preventDefault();
          search.focus();
        }
      });
    }
    (function diagramViewer() {
      var HOSTS = ".irule-flow-diagram,.app-pipe-diagram,.app-flow-diagram,.app-obj-flow,.arch-diagram,.diag-host";
      var overlay, viewport, canvas;
      var scale = 1, tx = 0, ty = 0, fit = 1, nat = { w: 0, h: 0 };
      var pointers = {};
      function clamp(s) {
        return Math.max(fit * 0.3, Math.min(fit * 30, s));
      }
      function apply() {
        canvas.style.transform = "translate(" + tx + "px," + ty + "px) scale(" + scale + ")";
      }
      function zoomAt(factor, mx, my) {
        var ns = clamp(scale * factor), f = ns / scale;
        tx = mx - (mx - tx) * f;
        ty = my - (my - ty) * f;
        scale = ns;
      }
      function natSize(svg) {
        function px(v) {
          if (v == null) return NaN;
          v = String(v).trim();
          return /%$/.test(v) ? NaN : parseFloat(v);
        }
        var w = px(svg.getAttribute("width")), h = px(svg.getAttribute("height"));
        if (w > 0 && h > 0) return { w, h };
        var vb = svg.getAttribute("viewBox");
        if (vb) {
          var p = vb.split(/[\s,]+/), vw = parseFloat(p[2]), vh = parseFloat(p[3]);
          if (vw > 0 && vh > 0) return { w: vw, h: vh };
        }
        var r = svg.getBoundingClientRect();
        return { w: r.width || 900, h: r.height || 600 };
      }
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
          var pa = pointers[ids[0]], pb = pointers[ids[1]];
          var pd = Math.hypot(pa.x - pb.x, pa.y - pb.y);
          var pmx = (pa.x + pb.x) / 2, pmy = (pa.y + pb.y) / 2;
          pointers[e.pointerId] = { x: e.clientX, y: e.clientY };
          var a = pointers[ids[0]], b = pointers[ids[1]];
          var cd = Math.hypot(a.x - b.x, a.y - b.y);
          var cmx = (a.x + b.x) / 2, cmy = (a.y + b.y) / 2;
          tx += cmx - pmx;
          ty += cmy - pmy;
          if (pd > 0) zoomAt(cd / pd, cmx - vp.left, cmy - vp.top);
          apply();
        } else {
          var p = pointers[e.pointerId];
          tx += e.clientX - p.x;
          ty += e.clientY - p.y;
          pointers[e.pointerId] = { x: e.clientX, y: e.clientY };
          apply();
        }
      }
      function build() {
        overlay = document.createElement("div");
        overlay.className = "dv-overlay";
        overlay.hidden = true;
        overlay.innerHTML = '<div class="dv-toolbar"><button type="button" class="dv-btn" data-act="out" title="Zoom out" aria-label="Zoom out">\u2212</button><button type="button" class="dv-btn" data-act="reset" title="Fit to window">Fit</button><button type="button" class="dv-btn" data-act="in" title="Zoom in" aria-label="Zoom in">+</button><button type="button" class="dv-btn" data-act="close" title="Close (Esc)" aria-label="Close">\u2715</button></div><div class="dv-viewport"><div class="dv-canvas"></div></div><div class="dv-hint">drag to pan \xB7 scroll or pinch to zoom \xB7 double-click to zoom in</div>';
        document.body.appendChild(overlay);
        viewport = overlay.querySelector(".dv-viewport");
        canvas = overlay.querySelector(".dv-canvas");
        overlay.querySelector(".dv-toolbar").addEventListener("click", function(e) {
          var b = e.target.closest("[data-act]");
          if (!b) return;
          var vp = viewport.getBoundingClientRect();
          if (b.dataset.act === "close") close();
          else if (b.dataset.act === "reset") fitToWindow();
          else {
            zoomAt(b.dataset.act === "in" ? 1.4 : 1 / 1.4, vp.width / 2, vp.height / 2);
            apply();
          }
        });
        viewport.addEventListener("wheel", function(e) {
          e.preventDefault();
          var vp = viewport.getBoundingClientRect();
          zoomAt(Math.exp(-e.deltaY * 16e-4), e.clientX - vp.left, e.clientY - vp.top);
          apply();
        }, { passive: false });
        viewport.addEventListener("dblclick", function(e) {
          var vp = viewport.getBoundingClientRect();
          zoomAt(1.6, e.clientX - vp.left, e.clientY - vp.top);
          apply();
        });
        viewport.addEventListener("pointerdown", function(e) {
          try {
            viewport.setPointerCapture(e.pointerId);
          } catch (_) {
          }
          pointers[e.pointerId] = { x: e.clientX, y: e.clientY };
          viewport.classList.add("grabbing");
        });
        viewport.addEventListener("pointermove", onMove);
        function up(e) {
          delete pointers[e.pointerId];
          try {
            viewport.releasePointerCapture(e.pointerId);
          } catch (_) {
          }
          if (!Object.keys(pointers).length) viewport.classList.remove("grabbing");
        }
        viewport.addEventListener("pointerup", up);
        viewport.addEventListener("pointercancel", up);
        document.addEventListener("keydown", function(e) {
          if (overlay && !overlay.hidden && e.key === "Escape") close();
        });
        window.addEventListener("resize", function() {
          if (overlay && !overlay.hidden) fitToWindow();
        });
      }
      function open(host) {
        var svg = host.querySelector("svg");
        if (!svg) return;
        if (!overlay) build();
        var clone = svg.cloneNode(true);
        clone.removeAttribute("style");
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
      function decorate(host) {
        if (!host.querySelector("svg") || hasExpand(host)) return;
        if (getComputedStyle(host).position === "static") host.style.position = "relative";
        var btn = document.createElement("button");
        btn.type = "button";
        btn.className = "dv-expand";
        btn.title = "Expand \u2014 pan & zoom";
        btn.setAttribute("aria-label", "Expand diagram");
        btn.textContent = "\u2922";
        btn.addEventListener("click", function(e) {
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
        new MutationObserver(function() {
          if (pending) return;
          pending = true;
          requestAnimationFrame(function() {
            pending = false;
            decorateAll();
          });
        }).observe(document.body, { childList: true, subtree: true });
      }
    })();
  })();
})();
