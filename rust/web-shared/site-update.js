// tcl-lsp — shared GitHub Pages update monitor
// SPDX-License-Identifier: AGPL-3.0-or-later
(function (global) {
  "use strict";
  var timer = null;
  function onPages() {
    return (
      (location.protocol === "https:" &&
        /(?:^|\.)github\.io$/i.test(location.hostname)) ||
      new URLSearchParams(location.search).has("pages-test")
    );
  }
  function show(current, latest) {
    if (document.getElementById("tcl-lsp-site-update")) return;
    var banner = document.createElement("aside");
    banner.id = "tcl-lsp-site-update";
    banner.setAttribute("role", "status");
    banner.style.cssText =
      "position:fixed;z-index:2147483647;left:12px;right:12px;bottom:12px;padding:10px 14px;border:1px solid #f9e2af;border-radius:6px;background:#181825;color:#f9e2af;font:13px system-ui,sans-serif;box-shadow:0 4px 18px #0008";
    var text = document.createElement("span");
    text.textContent =
      "A newer tcl-lsp web build is available (" +
      latest +
      "; this tab is " +
      current +
      "). ";
    var reload = document.createElement("button");
    reload.type = "button";
    reload.textContent = "Reload now";
    reload.style.cssText = "margin-left:8px;padding:3px 8px;cursor:pointer";
    reload.addEventListener("click", function () {
      location.reload();
    });
    banner.append(text, reload);
    document.body.appendChild(banner);
  }
  async function check(options) {
    try {
      var url = new URL(
        options.manifestUrl || "build-info.json",
        document.baseURI,
      );
      url.searchParams.set("update-check", String(Date.now()));
      var response = await fetch(url.href, {
        cache: "no-store",
        credentials: "omit",
      });
      if (!response.ok) return;
      var latest = await response.json();
      if (
        latest &&
        latest.version &&
        latest.version !== options.currentVersion
      ) {
        show(options.currentVersion, latest.version);
      }
    } catch (_) {
      // Update checks are advisory. Offline use must remain completely quiet.
    }
  }
  global.TclLspSiteUpdate = {
    start: function (options) {
      if (!onPages() || !options || !options.currentVersion) return;
      check(options);
      if (timer !== null) clearInterval(timer);
      timer = setInterval(function () {
        check(options);
      }, options.intervalMs || 300000);
      document.addEventListener("visibilitychange", function () {
        if (document.visibilityState === "visible") check(options);
      });
    },
  };
})(window);
