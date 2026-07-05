// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/bigip-report/shared/src — DO NOT EDIT; edit the .ts source.
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
    try {
      var saved = localStorage.getItem("f5report-theme");
      if (saved) root.setAttribute("data-theme", saved);
    } catch (e) {
    }
    var toggle = document.getElementById("themeToggle");
    if (toggle) {
      toggle.addEventListener("click", function() {
        var cur = root.getAttribute("data-theme") || "auto";
        var next = order[(order.indexOf(cur) + 1) % order.length];
        root.setAttribute("data-theme", next);
        try {
          localStorage.setItem("f5report-theme", next);
        } catch (e) {
        }
        toggle.title = "Theme: " + next;
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
  })();
})();
