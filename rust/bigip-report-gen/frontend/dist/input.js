// SPDX-License-Identifier: AGPL-3.0-or-later
// Generated from rust/bigip-report-gen/frontend/src — DO NOT EDIT; edit the .ts source.
"use strict";
(() => {
  // src/report-backend.ts
  var WasmBackend = class {
    constructor(wasm, wasmBytes) {
      this.wasm = wasm;
      this.initPromise = Promise.resolve(wasm(wasmBytes)).then(() => void 0);
    }
    ready() {
      return this.initPromise;
    }
    async engineVersion() {
      await this.initPromise;
      return this.wasm.engine_version();
    }
    async extractAll(files, passphrase) {
      return Promise.all(
        files.map(async (f) => {
          const bytes = new Uint8Array(await f.arrayBuffer());
          try {
            const scf = this.wasm.extract_source(f.name, bytes, passphrase);
            return { name: f.name, scf, bytes };
          } catch (e) {
            const msg = e instanceof Error ? e.message : String(e);
            if (/passphrase/i.test(msg)) {
              throw new Error(`${f.name}: this UCS is encrypted \u2014 enter its passphrase above.`);
            }
            throw new Error(`${f.name}: ${msg}`);
          }
        })
      );
    }
    async probe(files, passphrase) {
      await this.initPromise;
      const items = await this.extractAll(files, passphrase);
      const secretCount = items.reduce((a, s) => a + this.wasm.secret_count(s.scf), 0);
      return { secretCount };
    }
    async generate(files, opts) {
      await this.initPromise;
      let items = await this.extractAll(files, opts.passphrase);
      const secretTotal = items.reduce((a, s) => a + this.wasm.secret_count(s.scf), 0);
      const certFiles = {};
      const forensicFiles = {};
      for (const it of items) {
        try {
          certFiles[it.name] = JSON.parse(
            this.wasm.extract_cert_files(it.name, it.bytes, opts.passphrase, it.scf)
          );
        } catch {
        }
        try {
          forensicFiles[it.name] = JSON.parse(
            this.wasm.extract_files(it.name, it.bytes, opts.passphrase)
          );
        } catch {
        }
      }
      if (opts.masterKey && secretTotal > 0) {
        items = items.map((s) => {
          try {
            return { name: s.name, scf: this.wasm.decrypt_secrets(s.scf, opts.masterKey), bytes: s.bytes };
          } catch (e) {
            const msg = e instanceof Error ? e.message : String(e);
            throw new Error(`${msg} \u2014 check the base64 key from \`f5mku -K\`.`);
          }
        });
      }
      const sources = items.map((s) => [s.name, s.scf]);
      const html = this.wasm.generate_report(
        JSON.stringify(sources),
        JSON.stringify(certFiles),
        JSON.stringify(forensicFiles),
        opts.title,
        opts.generatedAt,
        opts.embedConsole,
        opts.manifest,
        opts.reportId,
        JSON.stringify(opts.settings ?? {})
      );
      return { html, deviceCount: sources.length, locked: secretTotal > 0 && !opts.masterKey };
    }
    async buildArchitecture(devicesJson, manifest) {
      await this.initPromise;
      if (!this.wasm.build_architecture) throw new Error("this engine build has no build_architecture");
      return this.wasm.build_architecture(devicesJson, manifest);
    }
    async manual(topic) {
      await this.initPromise;
      if (!this.wasm.manual) return "";
      return this.wasm.manual(topic);
    }
  };
  var ServerBackend = class {
    constructor(base = "") {
      this.base = base;
    }
    ready() {
      return Promise.resolve();
    }
    async engineVersion() {
      try {
        const r = await fetch(`${this.base}/version`);
        return r.ok ? (await r.text()).trim() : "";
      } catch {
        return "";
      }
    }
    form(files, opts) {
      const fd = new FormData();
      for (const f of files) fd.append("files", f, f.name);
      for (const [k, v] of Object.entries(opts)) {
        if (v !== void 0 && v !== null) fd.append(k, typeof v === "boolean" ? String(v) : String(v));
      }
      return fd;
    }
    async probe(files, passphrase) {
      const r = await fetch(`${this.base}/probe`, { method: "POST", body: this.form(files, { passphrase }) });
      if (!r.ok) throw new Error(await r.text());
      return await r.json();
    }
    async generate(files, opts) {
      const r = await fetch(`${this.base}/generate`, { method: "POST", body: this.form(files, opts) });
      if (!r.ok) throw new Error(await r.text());
      const deviceCount = parseInt(r.headers.get("X-Device-Count") || String(files.length), 10);
      const locked = r.headers.get("X-Secrets-Locked") === "1";
      return { html: await r.text(), deviceCount, locked };
    }
    async buildArchitecture(devicesJson, manifest) {
      const fd = new FormData();
      fd.append("devices", devicesJson);
      fd.append("manifest", manifest);
      const r = await fetch(`${this.base}/architecture`, { method: "POST", body: fd });
      if (!r.ok) throw new Error(await r.text());
      return r.text();
    }
    async manual(topic) {
      const r = await fetch(`${this.base}/manual?topic=${encodeURIComponent(topic)}`);
      return r.ok ? r.text() : "";
    }
  };
  var UnavailableBackend = class {
    constructor(reason) {
      this.reason = reason;
    }
    fail() {
      return Promise.reject(new Error(this.reason));
    }
    ready() {
      return this.fail();
    }
    engineVersion() {
      return this.fail();
    }
    probe() {
      return this.fail();
    }
    generate() {
      return this.fail();
    }
    buildArchitecture() {
      return this.fail();
    }
    manual() {
      return this.fail();
    }
  };
  function selectBackend() {
    const payload = document.getElementById("report-wasm");
    const inlined = payload && payload.textContent ? payload.textContent.trim() : "";
    if (inlined) {
      if (typeof wasm_bindgen !== "function") {
        return new UnavailableBackend(
          "the in-browser report engine failed to load \u2014 reload the page (your configuration was not uploaded)"
        );
      }
      const bin = atob(inlined);
      const bytes = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
      return new WasmBackend(wasm_bindgen, bytes);
    }
    return new ServerBackend("");
  }

  // src/pages/input.ts
  function byId(id) {
    const el = document.getElementById(id);
    if (!el) throw new Error(`missing #${id}`);
    return el;
  }
  var ROLES = ["auto", "gtm", "ltm", "afm"];
  var LS_MANIFEST = "f5report-arch-manifest";
  var LS_META = "f5report-arch-meta";
  var LS_SETTINGS = "f5report-settings";
  (function() {
    "use strict";
    const statusEl = byId("status");
    const goBtn = byId("go");
    const picker = byId("picker");
    const drop = byId("drop");
    const fileListEl = byId("fileList");
    const passEl = byId("pass");
    const passGroup = byId("passGroup");
    const passNote = byId("passNote");
    const titleEl = byId("title");
    const consoleEl = byId("console");
    const resultEl = byId("result");
    const openTab = byId("openTab");
    const dlAgain = byId("dlAgain");
    const verEl = byId("ver");
    const manifestEl = byId("manifest");
    const mkuGroup = byId("mkuGroup");
    const mkuNote = byId("mkuNote");
    const frontMatterEl = byId("frontMatter");
    const copyrightEl = byId("copyright");
    const logoPreviewEl = byId("logoPreview");
    const logoClearBtn = byId("logoClear");
    let files = [];
    let logoDataUri = "";
    let meta = {};
    let lastUrl = null;
    let ready = false;
    const encState = /* @__PURE__ */ new WeakMap();
    const backend = selectBackend();
    function tclWord(s) {
      return /[\s{}[\]$";\\]/.test(s) ? "{" + s + "}" : s;
    }
    function buildManifest() {
      const raw = (manifestEl.value || "").trim();
      if (raw) return raw;
      const lines = [];
      for (const f of files) {
        const m = meta[f.name] || { role: "auto", tier: "" };
        const parts = [];
        if (m.role && m.role !== "auto") parts.push("-role " + m.role);
        if (m.tier !== void 0 && m.tier !== "" && m.tier !== null) {
          const t = parseInt(m.tier, 10);
          if (!isNaN(t)) parts.push("-tier " + t);
        }
        if (parts.length) lines.push("device " + tclWord(f.name) + " " + parts.join(" "));
      }
      return lines.join("\n");
    }
    function saveManifestState() {
      try {
        localStorage.setItem(LS_MANIFEST, manifestEl.value || "");
        localStorage.setItem(LS_META, JSON.stringify(meta));
      } catch {
      }
    }
    function restoreManifestState() {
      try {
        const m = localStorage.getItem(LS_MANIFEST);
        if (m) manifestEl.value = m;
        const mt = localStorage.getItem(LS_META);
        if (mt) meta = JSON.parse(mt) || {};
      } catch {
      }
    }
    function currentSettings() {
      return {
        frontMatter: frontMatterEl.value,
        copyright: copyrightEl.value,
        logo: logoDataUri
      };
    }
    function reflectLogo() {
      if (logoDataUri) {
        logoPreviewEl.src = logoDataUri;
        logoPreviewEl.style.display = "";
        logoClearBtn.style.display = "";
      } else {
        logoPreviewEl.removeAttribute("src");
        logoPreviewEl.style.display = "none";
        logoClearBtn.style.display = "none";
      }
    }
    function saveSettings() {
      try {
        localStorage.setItem(LS_SETTINGS, JSON.stringify(currentSettings()));
      } catch {
      }
    }
    function applySettings(s) {
      frontMatterEl.value = s.frontMatter || "";
      copyrightEl.value = s.copyright || "";
      logoDataUri = s.logo || "";
      reflectLogo();
    }
    function restoreSettings() {
      try {
        const raw = localStorage.getItem(LS_SETTINGS);
        if (raw) applySettings(JSON.parse(raw));
      } catch {
      }
    }
    function setStatus(msg, cls) {
      statusEl.textContent = msg;
      statusEl.className = "status" + (cls ? " " + cls : "");
    }
    function fmtSize(n) {
      if (n < 1024) return n + " B";
      if (n < 1024 * 1024) return (n / 1024).toFixed(1) + " KB";
      return (n / (1024 * 1024)).toFixed(1) + " MB";
    }
    function looksPgp(data) {
      if (data.length === 0) return false;
      const isWs = (b) => b === 32 || b === 9 || b === 10 || b === 12 || b === 13;
      let s = 0;
      while (s < data.length && isWs(data[s])) s++;
      const armor = "-----BEGIN PGP";
      if (data.length - s >= armor.length) {
        let armored = true;
        for (let k = 0; k < armor.length; k++) {
          if (data[s + k] !== armor.charCodeAt(k)) {
            armored = false;
            break;
          }
        }
        if (armored) return true;
      }
      const first = data[0];
      if ((first & 128) === 0) return false;
      const tag = (first & 64) !== 0 ? first & 63 : first >> 2 & 15;
      return tag === 1 || tag === 3 || tag === 9 || tag === 18;
    }
    async function isEncryptedUcs(file) {
      try {
        return looksPgp(new Uint8Array(await file.slice(0, 64).arrayBuffer()));
      } catch {
        return false;
      }
    }
    function renderFiles() {
      fileListEl.textContent = "";
      files.forEach((f, i) => {
        const li = document.createElement("li");
        const name = document.createElement("span");
        name.className = "name";
        name.textContent = f.name;
        const sz = document.createElement("span");
        sz.className = "sz";
        sz.textContent = fmtSize(f.size);
        const rm = document.createElement("button");
        rm.className = "rm";
        rm.title = "Remove";
        rm.setAttribute("aria-label", "Remove " + f.name);
        rm.textContent = "\u2715";
        rm.addEventListener("click", (e) => {
          e.stopPropagation();
          files.splice(i, 1);
          renderFiles();
        });
        li.appendChild(name);
        if (encState.get(f)) {
          const lk = document.createElement("span");
          lk.className = "lock";
          lk.title = "encrypted";
          lk.textContent = "\u{1F512}";
          li.appendChild(lk);
        }
        li.appendChild(sz);
        if (files.length > 1) {
          if (!meta[f.name]) meta[f.name] = { role: "auto", tier: "" };
          const mrec = meta[f.name];
          const role = document.createElement("select");
          role.className = "arch-role";
          role.title = "Device role";
          for (const r of ROLES) {
            const opt = document.createElement("option");
            opt.value = r;
            opt.textContent = r;
            if (r === mrec.role) opt.selected = true;
            role.appendChild(opt);
          }
          role.addEventListener("click", (e) => e.stopPropagation());
          role.addEventListener("change", () => {
            mrec.role = role.value;
            saveManifestState();
          });
          const tier = document.createElement("input");
          tier.type = "number";
          tier.min = "0";
          tier.className = "arch-tier";
          tier.placeholder = "tier";
          tier.title = "Tier (0 = front)";
          tier.value = mrec.tier;
          tier.addEventListener("click", (e) => e.stopPropagation());
          tier.addEventListener("input", () => {
            mrec.tier = tier.value;
            saveManifestState();
          });
          li.appendChild(role);
          li.appendChild(tier);
        }
        li.appendChild(rm);
        fileListEl.appendChild(li);
      });
      byId("archAdv").style.display = files.length > 1 ? "" : "none";
      updateReady();
    }
    function updateReady() {
      goBtn.disabled = !(ready && files.length > 0);
    }
    function addFiles(list) {
      for (const f of Array.from(list)) {
        if (!files.some((g) => g.name === f.name && g.size === f.size)) files.push(f);
      }
      renderFiles();
      void probeEncryption();
      void probeSecrets();
    }
    drop.addEventListener("click", () => picker.click());
    drop.addEventListener("keydown", (e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        picker.click();
      }
    });
    picker.addEventListener("change", () => {
      if (picker.files) addFiles(picker.files);
      picker.value = "";
    });
    ["dragenter", "dragover"].forEach(
      (ev) => drop.addEventListener(ev, (e) => {
        e.preventDefault();
        drop.classList.add("drag");
      })
    );
    ["dragleave", "drop"].forEach(
      (ev) => drop.addEventListener(ev, (e) => {
        e.preventDefault();
        drop.classList.remove("drag");
      })
    );
    drop.addEventListener("drop", (e) => {
      const dt = e.dataTransfer;
      if (dt && dt.files) addFiles(dt.files);
    });
    (function initManifestControls() {
      restoreManifestState();
      manifestEl.addEventListener("input", saveManifestState);
      byId("manifestFromFiles").addEventListener("click", () => {
        const keep = manifestEl.value;
        manifestEl.value = "";
        const gen = buildManifest();
        manifestEl.value = gen || keep;
        saveManifestState();
      });
      byId("manifestSave").addEventListener("click", () => {
        const text = manifestEl.value.trim() || buildManifest();
        if (!text) {
          setStatus("nothing to save \u2014 set roles/tiers or write a manifest", "err");
          return;
        }
        const blob = new Blob([text + "\n"], { type: "text/plain" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = "architecture.tcl";
        document.body.appendChild(a);
        a.click();
        a.remove();
        setTimeout(() => URL.revokeObjectURL(url), 1e3);
      });
      const fileInput = byId("manifestFile");
      byId("manifestLoad").addEventListener("click", () => fileInput.click());
      fileInput.addEventListener("change", () => {
        const f = fileInput.files && fileInput.files[0];
        if (!f) return;
        const r = new FileReader();
        r.onload = () => {
          manifestEl.value = String(r.result || "");
          saveManifestState();
        };
        r.readAsText(f);
        fileInput.value = "";
      });
      byId("manifestClear").addEventListener("click", () => {
        manifestEl.value = "";
        saveManifestState();
      });
    })();
    (function initSettingsControls() {
      restoreSettings();
      frontMatterEl.addEventListener("input", saveSettings);
      copyrightEl.addEventListener("input", saveSettings);
      const logoInput = byId("logoFile");
      byId("logoPick").addEventListener("click", () => logoInput.click());
      logoInput.addEventListener("change", () => {
        const f = logoInput.files && logoInput.files[0];
        if (!f) return;
        if (f.size > 512 * 1024) {
          setStatus("logo is larger than 512 KB \u2014 pick a smaller image", "err");
          logoInput.value = "";
          return;
        }
        const r = new FileReader();
        r.onload = () => {
          logoDataUri = String(r.result || "");
          reflectLogo();
          saveSettings();
        };
        r.readAsDataURL(f);
        logoInput.value = "";
      });
      logoClearBtn.addEventListener("click", () => {
        logoDataUri = "";
        reflectLogo();
        saveSettings();
      });
      byId("settingsExport").addEventListener("click", () => {
        const blob = new Blob([JSON.stringify(currentSettings(), null, 2) + "\n"], {
          type: "application/json"
        });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = "f5report-settings.json";
        document.body.appendChild(a);
        a.click();
        a.remove();
        setTimeout(() => URL.revokeObjectURL(url), 1e3);
      });
      const settingsFile = byId("settingsFile");
      byId("settingsImport").addEventListener("click", () => settingsFile.click());
      settingsFile.addEventListener("change", () => {
        const f = settingsFile.files && settingsFile.files[0];
        if (!f) return;
        const r = new FileReader();
        r.onload = () => {
          try {
            applySettings(JSON.parse(String(r.result || "{}")));
            saveSettings();
          } catch {
            setStatus("could not read that settings file", "err");
          }
        };
        r.readAsText(f);
        settingsFile.value = "";
      });
      byId("settingsClear").addEventListener("click", () => {
        applySettings({});
        saveSettings();
      });
    })();
    backend.ready().then(async () => {
      ready = true;
      try {
        const v = await backend.engineVersion();
        if (v) verEl.textContent = "engine v" + v;
      } catch {
      }
      setStatus("ready \u2014 pick a config to begin", "ok");
      updateReady();
      void probeSecrets();
    }).catch((e) => setStatus("failed to load the engine: " + String(e), "err"));
    passEl.addEventListener("input", () => void probeSecrets());
    function utcStamp() {
      const d = /* @__PURE__ */ new Date();
      const p = (n) => String(n).padStart(2, "0");
      return d.getUTCFullYear() + "-" + p(d.getUTCMonth() + 1) + "-" + p(d.getUTCDate()) + " " + p(d.getUTCHours()) + ":" + p(d.getUTCMinutes()) + ":" + p(d.getUTCSeconds()) + " UTC";
    }
    function stampName() {
      const d = /* @__PURE__ */ new Date();
      const p = (n) => String(n).padStart(2, "0");
      return "bigip-report-" + d.getFullYear() + p(d.getMonth() + 1) + p(d.getDate()) + "-" + p(d.getHours()) + p(d.getMinutes()) + p(d.getSeconds()) + ".html";
    }
    function reportId() {
      const c = window.crypto || {};
      return c.randomUUID ? c.randomUUID() : "r-" + Date.now().toString(36);
    }
    function revealPass(count) {
      passGroup.style.display = "";
      passNote.textContent = count + " encrypted UCS archive" + (count === 1 ? "" : "s") + " detected. Enter the passphrase to decrypt and read " + (count === 1 ? "it" : "them") + " \u2014 it stays in your browser and is never sent anywhere.";
    }
    async function probeEncryption() {
      let discovered = false;
      let count = 0;
      for (const f of files) {
        let enc = encState.get(f);
        if (enc === void 0) {
          enc = await isEncryptedUcs(f);
          encState.set(f, enc);
          if (enc) discovered = true;
        }
        if (enc) count++;
      }
      if (discovered) renderFiles();
      if (count > 0 && passGroup.style.display === "none") revealPass(count);
    }
    function revealMku(count) {
      mkuGroup.style.display = "";
      mkuNote.textContent = count + " encrypted secret" + (count === 1 ? "" : "s") + " detected (SSL private-key passphrases, \u2026). Enter the unit master key (f5mku -K) to decrypt and reveal them \u2014 without it those values stay locked in the report. Decryption happens here in your browser; the key is never sent anywhere.";
    }
    async function probeSecrets() {
      if (!ready || files.length === 0 || mkuGroup.style.display !== "none") return;
      try {
        const { secretCount } = await backend.probe(files, passEl.value || "");
        if (secretCount > 0) revealMku(secretCount);
      } catch {
      }
    }
    goBtn.addEventListener("click", () => {
      if (!ready || files.length === 0) return;
      resultEl.classList.remove("show");
      goBtn.disabled = true;
      statusEl.innerHTML = '<span class="spinner"></span> generating\u2026';
      setTimeout(() => {
        run().catch((e) => {
          setStatus(e instanceof Error ? e.message : String(e), "err");
          goBtn.disabled = false;
        });
      }, 30);
    });
    async function run() {
      const opts = {
        passphrase: passGroup.style.display === "none" ? "" : passEl.value,
        masterKey: mkuGroup.style.display === "none" ? "" : byId("mku").value.trim(),
        title: titleEl.value.trim() || "F5 BIG-IP Configuration Report",
        embedConsole: consoleEl.checked,
        manifest: buildManifest(),
        generatedAt: utcStamp(),
        reportId: reportId(),
        settings: currentSettings()
      };
      const res = await backend.generate(files, opts);
      if (res.locked) revealMku(1);
      deliver(res);
    }
    function deliver(res) {
      if (lastUrl) URL.revokeObjectURL(lastUrl);
      const blob = new Blob([res.html], { type: "text/html" });
      lastUrl = URL.createObjectURL(blob);
      const fname = stampName();
      const a = document.createElement("a");
      a.href = lastUrl;
      a.download = fname;
      document.body.appendChild(a);
      a.click();
      a.remove();
      openTab.href = lastUrl;
      dlAgain.href = lastUrl;
      dlAgain.download = fname;
      resultEl.classList.add("show");
      const sz = fmtSize(new Blob([res.html]).size);
      const extra = res.locked ? " \xB7 some secrets locked \u2014 enter the master key above to reveal" : "";
      setStatus(
        "done \u2014 " + res.deviceCount + " device" + (res.deviceCount === 1 ? "" : "s") + " \xB7 " + sz + extra,
        res.locked ? "" : "ok"
      );
      goBtn.disabled = false;
    }
  })();
})();
