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
      tab.addEventListener("click", function () {
        var name = tab.dataset.panel;
        device.querySelectorAll(".tab").forEach(function (t) { t.classList.toggle("active", t === tab); });
        device.querySelectorAll(".panel").forEach(function (p) {
          p.classList.toggle("active", p.dataset.panel === name);
        });
      });
    });
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

  // --- search: global box filters the visible panel's rows ------------------
  function filter(scope, q) {
    q = q.trim().toLowerCase();
    scope.querySelectorAll("tr.searchable").forEach(function (row) {
      var hit = !q || row.textContent.toLowerCase().indexOf(q) !== -1;
      row.classList.toggle("hidden", !hit);
      // keep a searchable row's detail row in sync
      var det = row.nextElementSibling;
      if (det && det.classList.contains("detail")) det.classList.toggle("hidden", !hit);
    });
  }
  var search = document.getElementById("globalSearch");
  if (search) {
    search.addEventListener("input", function () {
      var device = document.querySelector(".device.active") || document;
      device.querySelectorAll(".panel").forEach(function (p) { filter(p, search.value); });
    });
    // "/" focuses search
    document.addEventListener("keydown", function (e) {
      if (e.key === "/" && document.activeElement !== search) {
        e.preventDefault();
        search.focus();
      }
    });
  }
})();
