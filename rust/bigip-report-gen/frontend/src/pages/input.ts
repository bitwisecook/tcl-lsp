// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// The BIG-IP report *builder* / input page controller. Collects config files +
// options + the architecture/topology manifest, then hands them to a
// ReportBackend (in-browser wasm, or the f5report web server) to generate the
// self-contained report. Ported from the wasm app's inline page; the
// generation pipeline now lives behind ./report-backend.
//
// prettier-ignore-file is not used — this is hand-maintained typed code.

import {
  GenerateOptions,
  GenerateResult,
  ReportBackend,
  ReportSettings,
  selectBackend,
} from "../report-backend";

function byId<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`missing #${id}`);
  return el as T;
}

interface FileMeta {
  role: string;
  tier: string;
}

const ROLES = ["auto", "gtm", "ltm", "afm"];
const LS_MANIFEST = "f5report-arch-manifest";
const LS_META = "f5report-arch-meta";
// Report branding/front-matter the builder remembers on this device and inlines
// into the generated report. Mirrors the wasm `ReportSettings` shape.
const LS_SETTINGS = "f5report-settings";

(function () {
  "use strict";

  const statusEl = byId("status");
  const goBtn = byId<HTMLButtonElement>("go");
  const picker = byId<HTMLInputElement>("picker");
  const drop = byId("drop");
  const fileListEl = byId("fileList");
  const passEl = byId<HTMLInputElement>("pass");
  const passGroup = byId("passGroup");
  const passNote = byId("passNote");
  const titleEl = byId<HTMLInputElement>("title");
  const consoleEl = byId<HTMLInputElement>("console");
  const resultEl = byId("result");
  const openTab = byId<HTMLAnchorElement>("openTab");
  const dlAgain = byId<HTMLAnchorElement>("dlAgain");
  const verEl = byId("ver");
  const manifestEl = byId<HTMLTextAreaElement>("manifest");
  const mkuGroup = byId("mkuGroup");
  const mkuNote = byId("mkuNote");
  const frontMatterEl = byId<HTMLTextAreaElement>("frontMatter");
  const copyrightEl = byId<HTMLInputElement>("copyright");
  const logoPreviewEl = byId<HTMLImageElement>("logoPreview");
  const logoClearBtn = byId<HTMLButtonElement>("logoClear");

  let files: File[] = [];
  // Current report logo as an inlined data: URI ("" = none), set by the picker.
  let logoDataUri = "";
  let meta: Record<string, FileMeta> = {};
  let lastUrl: string | null = null;
  let ready = false;
  // Per-file "is this an encrypted (passphrase-protected) UCS?" verdict, filled
  // in lazily by probeEncryption() from the file's leading bytes.
  const encState = new WeakMap<File, boolean>();

  const backend: ReportBackend = selectBackend();

  // Quote a word for the Tcl manifest: brace it when it carries whitespace or
  // Tcl-special characters, so a filename with spaces stays one word.
  function tclWord(s: string): string {
    return /[\s{}[\]$";\\]/.test(s) ? "{" + s + "}" : s;
  }

  // Build the manifest from the per-file role/tier choices. A raw manifest
  // (typed into the textarea) wins. Returns "" for pure auto-detection.
  function buildManifest(): string {
    const raw = (manifestEl.value || "").trim();
    if (raw) return raw;
    const lines: string[] = [];
    for (const f of files) {
      const m = meta[f.name] || { role: "auto", tier: "" };
      const parts: string[] = [];
      if (m.role && m.role !== "auto") parts.push("-role " + m.role);
      if (m.tier !== undefined && m.tier !== "" && m.tier !== null) {
        const t = parseInt(m.tier, 10);
        if (!isNaN(t)) parts.push("-tier " + t);
      }
      if (parts.length) lines.push("device " + tclWord(f.name) + " " + parts.join(" "));
    }
    return lines.join("\n");
  }

  function saveManifestState(): void {
    try {
      localStorage.setItem(LS_MANIFEST, manifestEl.value || "");
      localStorage.setItem(LS_META, JSON.stringify(meta));
    } catch {
      /* storage unavailable */
    }
  }
  function restoreManifestState(): void {
    try {
      const m = localStorage.getItem(LS_MANIFEST);
      if (m) manifestEl.value = m;
      const mt = localStorage.getItem(LS_META);
      if (mt) meta = (JSON.parse(mt) as Record<string, FileMeta>) || {};
    } catch {
      /* ignore */
    }
  }

  // --- report settings (front matter / copyright / logo) --------------------
  // A logo is an inlined base64 data: image URI, produced here by
  // FileReader.readAsDataURL. A stored or imported setting is not, so strip
  // every character that cannot appear in one and drop the value unless what
  // remains still is one — it goes on to <img src> and into the report.
  const LOGO_URI = /^data:image\/[\w.+-]+;base64,[A-Za-z0-9+/=]*$/;
  function cleanLogoUri(uri: string | undefined): string {
    const s = String(uri || "").replace(/[^\w+/=:;,.-]/g, "");
    return LOGO_URI.test(s) ? s : "";
  }

  function currentSettings(): ReportSettings {
    return {
      frontMatter: frontMatterEl.value,
      copyright: copyrightEl.value,
      logo: logoDataUri,
    };
  }
  function reflectLogo(): void {
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
  function saveSettings(): void {
    try {
      localStorage.setItem(LS_SETTINGS, JSON.stringify(currentSettings()));
    } catch {
      /* storage unavailable */
    }
  }
  function applySettings(s: ReportSettings): void {
    frontMatterEl.value = s.frontMatter || "";
    copyrightEl.value = s.copyright || "";
    logoDataUri = cleanLogoUri(s.logo);
    reflectLogo();
  }
  function restoreSettings(): void {
    try {
      const raw = localStorage.getItem(LS_SETTINGS);
      if (raw) applySettings(JSON.parse(raw) as ReportSettings);
    } catch {
      /* ignore */
    }
  }

  function setStatus(msg: string, cls?: string): void {
    statusEl.textContent = msg;
    statusEl.className = "status" + (cls ? " " + cls : "");
  }

  function fmtSize(n: number): string {
    if (n < 1024) return n + " B";
    if (n < 1024 * 1024) return (n / 1024).toFixed(1) + " KB";
    return (n / (1024 * 1024)).toFixed(1) + " MB";
  }

  // Recognise an OpenPGP-encrypted (i.e. passphrase-protected) UCS from its
  // leading bytes, mirroring tcl_bigip_io::is_pgp_bytes — an ASCII-armored
  // "-----BEGIN PGP" message, or a binary OpenPGP packet header whose tag opens
  // an encrypted message (PKESK/SKESK/SED/SEIPD). A plain gzip UCS or a bare
  // bigip.conf needs no passphrase and is not matched.
  function looksPgp(data: Uint8Array): boolean {
    if (data.length === 0) return false;
    const isWs = (b: number) =>
      b === 0x20 || b === 0x09 || b === 0x0a || b === 0x0c || b === 0x0d;
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
    if ((first & 0x80) === 0) return false; // not an OpenPGP packet header
    const tag = (first & 0x40) !== 0 ? first & 0x3f : (first >> 2) & 0x0f;
    return tag === 1 || tag === 3 || tag === 9 || tag === 18;
  }

  async function isEncryptedUcs(file: File): Promise<boolean> {
    try {
      return looksPgp(new Uint8Array(await file.slice(0, 64).arrayBuffer()));
    } catch {
      return false;
    }
  }

  function renderFiles(): void {
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
      rm.textContent = "✕";
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
        lk.textContent = "🔒";
        li.appendChild(lk);
      }
      li.appendChild(sz);

      // Per-file role + tier, shown once more than one device is loaded.
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

  function updateReady(): void {
    goBtn.disabled = !(ready && files.length > 0);
  }

  function addFiles(list: FileList | File[]): void {
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
  ["dragenter", "dragover"].forEach((ev) =>
    drop.addEventListener(ev, (e) => {
      e.preventDefault();
      drop.classList.add("drag");
    }),
  );
  ["dragleave", "drop"].forEach((ev) =>
    drop.addEventListener(ev, (e) => {
      e.preventDefault();
      drop.classList.remove("drag");
    }),
  );
  drop.addEventListener("drop", (e) => {
    const dt = (e as DragEvent).dataTransfer;
    if (dt && dt.files) addFiles(dt.files);
  });

  // --- architecture manifest: persistence + file load/save ------------------
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
        setStatus("nothing to save — set roles/tiers or write a manifest", "err");
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
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    });

    const fileInput = byId<HTMLInputElement>("manifestFile");
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

  // --- report settings: persistence, logo picker, export/import -------------
  (function initSettingsControls() {
    restoreSettings();
    frontMatterEl.addEventListener("input", saveSettings);
    copyrightEl.addEventListener("input", saveSettings);

    const logoInput = byId<HTMLInputElement>("logoFile");
    byId("logoPick").addEventListener("click", () => logoInput.click());
    logoInput.addEventListener("change", () => {
      const f = logoInput.files && logoInput.files[0];
      if (!f) return;
      if (f.size > 512 * 1024) {
        setStatus("logo is larger than 512 KB — pick a smaller image", "err");
        logoInput.value = "";
        return;
      }
      const r = new FileReader();
      r.onload = () => {
        logoDataUri = String(r.result || ""); // a data: URI (readAsDataURL)
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
        type: "application/json",
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = "f5report-settings.json";
      document.body.appendChild(a);
      a.click();
      a.remove();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    });

    const settingsFile = byId<HTMLInputElement>("settingsFile");
    byId("settingsImport").addEventListener("click", () => settingsFile.click());
    settingsFile.addEventListener("change", () => {
      const f = settingsFile.files && settingsFile.files[0];
      if (!f) return;
      const r = new FileReader();
      r.onload = () => {
        try {
          applySettings(JSON.parse(String(r.result || "{}")) as ReportSettings);
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

  // --- backend readiness ----------------------------------------------------
  backend
    .ready()
    .then(async () => {
      ready = true;
      try {
        const v = await backend.engineVersion();
        if (v) verEl.textContent = "engine v" + v;
      } catch {
        /* ignore */
      }
      setStatus("ready — pick a config to begin", "ok");
      updateReady();
      void probeSecrets();
    })
    .catch((e: unknown) => setStatus("failed to load the engine: " + String(e), "err"));

  passEl.addEventListener("input", () => void probeSecrets());

  function utcStamp(): string {
    const d = new Date();
    const p = (n: number) => String(n).padStart(2, "0");
    return (
      d.getUTCFullYear() +
      "-" +
      p(d.getUTCMonth() + 1) +
      "-" +
      p(d.getUTCDate()) +
      " " +
      p(d.getUTCHours()) +
      ":" +
      p(d.getUTCMinutes()) +
      ":" +
      p(d.getUTCSeconds()) +
      " UTC"
    );
  }

  function stampName(): string {
    const d = new Date();
    const p = (n: number) => String(n).padStart(2, "0");
    return (
      "bigip-report-" +
      d.getFullYear() +
      p(d.getMonth() + 1) +
      p(d.getDate()) +
      "-" +
      p(d.getHours()) +
      p(d.getMinutes()) +
      p(d.getSeconds()) +
      ".html"
    );
  }

  function reportId(): string {
    const c = (window.crypto || ({} as Crypto)) as Crypto & { randomUUID?: () => string };
    return c.randomUUID ? c.randomUUID() : "r-" + Date.now().toString(36);
  }

  function revealPass(count: number): void {
    passGroup.style.display = "";
    passNote.textContent =
      count +
      " encrypted UCS archive" +
      (count === 1 ? "" : "s") +
      " detected. Enter the passphrase to decrypt and read " +
      (count === 1 ? "it" : "them") +
      " — it stays in your browser and is never sent anywhere.";
  }

  // Reveal the passphrase box only once we spot an encrypted UCS among the
  // selected files — the same reveal-on-detect flow as the master-key box.
  // Detection is byte-based (no passphrase needed), so it works for both the
  // wasm and server backends. Once shown it stays, as revealMku does.
  async function probeEncryption(): Promise<void> {
    let discovered = false;
    let count = 0;
    for (const f of files) {
      let enc = encState.get(f);
      if (enc === undefined) {
        enc = await isEncryptedUcs(f);
        encState.set(f, enc);
        if (enc) discovered = true;
      }
      if (enc) count++;
    }
    if (discovered) renderFiles(); // newly-locked files get their 🔒
    if (count > 0 && passGroup.style.display === "none") revealPass(count);
  }

  function revealMku(count: number): void {
    mkuGroup.style.display = "";
    mkuNote.textContent =
      count +
      " encrypted secret" +
      (count === 1 ? "" : "s") +
      " detected (SSL private-key passphrases, …). Enter the unit master key (f5mku -K) to decrypt and reveal them — without it those values stay locked in the report. Decryption happens here in your browser; the key is never sent anywhere.";
  }

  async function probeSecrets(): Promise<void> {
    if (!ready || files.length === 0 || mkuGroup.style.display !== "none") return;
    try {
      const { secretCount } = await backend.probe(files, passEl.value || "");
      if (secretCount > 0) revealMku(secretCount);
    } catch {
      /* passphrase not ready yet — try again on the next change */
    }
  }

  goBtn.addEventListener("click", () => {
    if (!ready || files.length === 0) return;
    resultEl.classList.remove("show");
    goBtn.disabled = true;
    statusEl.innerHTML = '<span class="spinner"></span> generating…';
    // Let the spinner paint before the (possibly synchronous) work.
    setTimeout(() => {
      run().catch((e: unknown) => {
        setStatus(e instanceof Error ? e.message : String(e), "err");
        goBtn.disabled = false;
      });
    }, 30);
  });

  async function run(): Promise<void> {
    const opts: GenerateOptions = {
      passphrase: passGroup.style.display === "none" ? "" : passEl.value,
      masterKey: mkuGroup.style.display === "none" ? "" : byId<HTMLInputElement>("mku").value.trim(),
      title: titleEl.value.trim() || "F5 BIG-IP Configuration Report",
      embedConsole: consoleEl.checked,
      manifest: buildManifest(),
      generatedAt: utcStamp(),
      reportId: reportId(),
      settings: currentSettings(),
    };
    const res = await backend.generate(files, opts);
    if (res.locked) revealMku(1);
    deliver(res);
  }

  function deliver(res: GenerateResult): void {
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
    const extra = res.locked ? " · some secrets locked — enter the master key above to reveal" : "";
    setStatus(
      "done — " +
        res.deviceCount +
        " device" +
        (res.deviceCount === 1 ? "" : "s") +
        " · " +
        sz +
        extra,
      res.locked ? "" : "ok",
    );
    goBtn.disabled = false;
  }
})();
