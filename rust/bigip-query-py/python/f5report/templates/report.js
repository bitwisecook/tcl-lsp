// f5report — client-side interactivity. No dependencies, no network.
(function () {
  "use strict";

  // --- theme toggle: auto -> light -> dark, remembered in localStorage ------
  var root = document.documentElement;
  var order = ["auto", "light", "dark"];
  try {
    var saved = localStorage.getItem("f5report-theme");
    if (saved) root.setAttribute("data-theme", saved);
  } catch (e) {}
  var toggle = document.getElementById("themeToggle");
  if (toggle) {
    toggle.addEventListener("click", function () {
      var cur = root.getAttribute("data-theme") || "auto";
      var next = order[(order.indexOf(cur) + 1) % order.length];
      root.setAttribute("data-theme", next);
      try { localStorage.setItem("f5report-theme", next); } catch (e) {}
      toggle.title = "Theme: " + next;
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
})();
