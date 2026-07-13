// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// The seam that makes the report *builder* front-end generator-agnostic. The
// same input page (src/pages/input.ts) drives either:
//   - WasmBackend  — the in-browser generator: bytes → wasm → HTML, all client-
//                    side (rust/bigip-report-wasm), nothing uploaded; or
//   - ServerBackend — a Python `f5report.web` server that generates server-side.
// Both produce the same self-contained report HTML.

export interface GenerateOptions {
  passphrase: string;
  masterKey: string;
  title: string;
  embedConsole: boolean;
  /** The architecture / topology manifest (Tcl DSL), pre-embedded at generation. */
  manifest: string;
  /** Human generation timestamp (the engine is time-free; the caller stamps it). */
  generatedAt: string;
  /** Per-report UUID; the report keys its localStorage (arch editor) on this. */
  reportId: string;
  /**
   * Report settings the builder persists / exports as JSON: the report *content*
   * copyright notice (distinct from the fixed tooling copyright), and — in later
   * phases — the Markdown front-matter and logo. Serialised straight to the wasm
   * `generate_report` settings argument. Empty object = no settings.
   */
  settings?: ReportSettings;
}

/** Persisted/exported builder settings baked into the generated report. */
export interface ReportSettings {
  /** Report content copyright notice (the tooling copyright is fixed). */
  copyright?: string;
  /** Markdown front-matter for the Front-matter tab. */
  frontMatter?: string;
  /** Inlined report logo as a data-URI. */
  logo?: string;
}

export interface GenerateResult {
  html: string;
  deviceCount: number;
  /** True when the config carried $M$ secrets but no master key was supplied. */
  locked: boolean;
}

export interface ProbeResult {
  /** Number of `$M$…` encrypted secrets detected across the selected files. */
  secretCount: number;
}

/** A report generator backend. `probe`/`generate` take the raw selected files;
 *  each backend decides where the work happens (client wasm vs server). */
export interface ReportBackend {
  /** Resolves once the backend is ready to generate (wasm loaded / server reachable). */
  ready(): Promise<void>;
  engineVersion(): Promise<string>;
  probe(files: File[], passphrase: string): Promise<ProbeResult>;
  generate(files: File[], opts: GenerateOptions): Promise<GenerateResult>;
  /** Re-run architecture/topology detection for the GUI editor (Phase C). */
  buildArchitecture(devicesJson: string, manifest: string): Promise<string>;
  /** The embedded f5-query manual for a topic ("", "dsl", "builtins", …). */
  manual(topic: string): Promise<string>;
}

// The wasm-bindgen (no-modules) global the report generator exposes.
interface WasmBindgen {
  (init: Uint8Array): Promise<unknown>;
  engine_version(): string;
  extract_source(name: string, bytes: Uint8Array, passphrase: string): string;
  extract_cert_files(name: string, bytes: Uint8Array, passphrase: string, scf: string): string;
  extract_files(name: string, bytes: Uint8Array, passphrase: string): string;
  secret_count(scf: string): number;
  decrypt_secrets(scf: string, masterKey: string): string;
  generate_report(
    sourcesJson: string,
    certFilesJson: string,
    forensicFilesJson: string,
    title: string,
    generatedAt: string,
    embedConsole: boolean,
    manifest: string,
    reportId: string,
    settingsJson: string,
  ): string;
  // Added in Phase C (guarded with `in` checks until the wasm exports them).
  build_architecture?(devicesJson: string, manifest: string): string;
  manual?(topic: string): string;
}

interface ExtractedSource {
  name: string;
  scf: string;
  bytes: Uint8Array;
}

/** In-browser backend: everything runs client-side via the inlined wasm. */
export class WasmBackend implements ReportBackend {
  private wasm: WasmBindgen;
  private initPromise: Promise<void>;

  constructor(wasm: WasmBindgen, wasmBytes: Uint8Array) {
    this.wasm = wasm;
    this.initPromise = Promise.resolve(wasm(wasmBytes)).then(() => undefined);
  }

  ready(): Promise<void> {
    return this.initPromise;
  }

  async engineVersion(): Promise<string> {
    await this.initPromise;
    return this.wasm.engine_version();
  }

  private async extractAll(files: File[], passphrase: string): Promise<ExtractedSource[]> {
    return Promise.all(
      files.map(async (f) => {
        const bytes = new Uint8Array(await f.arrayBuffer());
        try {
          const scf = this.wasm.extract_source(f.name, bytes, passphrase);
          return { name: f.name, scf, bytes };
        } catch (e) {
          const msg = e instanceof Error ? e.message : String(e);
          if (/passphrase/i.test(msg)) {
            throw new Error(`${f.name}: this UCS is encrypted — enter its passphrase above.`);
          }
          throw new Error(`${f.name}: ${msg}`);
        }
      }),
    );
  }

  async probe(files: File[], passphrase: string): Promise<ProbeResult> {
    await this.initPromise;
    const items = await this.extractAll(files, passphrase);
    const secretCount = items.reduce((a, s) => a + this.wasm.secret_count(s.scf), 0);
    return { secretCount };
  }

  async generate(files: File[], opts: GenerateOptions): Promise<GenerateResult> {
    await this.initPromise;
    let items = await this.extractAll(files, opts.passphrase);
    const secretTotal = items.reduce((a, s) => a + this.wasm.secret_count(s.scf), 0);

    // Certs + forensics come straight from each archive's filestore (unaffected
    // by the f5mku secret decryption below); best-effort per source.
    const certFiles: Record<string, unknown> = {};
    const forensicFiles: Record<string, unknown> = {};
    for (const it of items) {
      try {
        certFiles[it.name] = JSON.parse(
          this.wasm.extract_cert_files(it.name, it.bytes, opts.passphrase, it.scf),
        );
      } catch {
        /* fall back to config metadata */
      }
      try {
        forensicFiles[it.name] = JSON.parse(
          this.wasm.extract_files(it.name, it.bytes, opts.passphrase),
        );
      } catch {
        /* no forensic view for this source */
      }
    }

    if (opts.masterKey && secretTotal > 0) {
      items = items.map((s) => {
        try {
          return { name: s.name, scf: this.wasm.decrypt_secrets(s.scf, opts.masterKey), bytes: s.bytes };
        } catch (e) {
          const msg = e instanceof Error ? e.message : String(e);
          throw new Error(`${msg} — check the base64 key from \`f5mku -K\`.`);
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
      JSON.stringify(opts.settings ?? {}),
    );
    return { html, deviceCount: sources.length, locked: secretTotal > 0 && !opts.masterKey };
  }

  async buildArchitecture(devicesJson: string, manifest: string): Promise<string> {
    await this.initPromise;
    if (!this.wasm.build_architecture) throw new Error("this engine build has no build_architecture");
    return this.wasm.build_architecture(devicesJson, manifest);
  }

  async manual(topic: string): Promise<string> {
    await this.initPromise;
    if (!this.wasm.manual) return "";
    return this.wasm.manual(topic);
  }
}

/** Server backend: uploads the files and lets `f5report.web` generate the
 *  report server-side. Endpoints mirror the wasm capabilities. */
export class ServerBackend implements ReportBackend {
  constructor(private base: string = "") {}

  ready(): Promise<void> {
    return Promise.resolve();
  }

  async engineVersion(): Promise<string> {
    try {
      const r = await fetch(`${this.base}/version`);
      return r.ok ? (await r.text()).trim() : "";
    } catch {
      return "";
    }
  }

  private form(files: File[], opts: Partial<GenerateOptions>): FormData {
    const fd = new FormData();
    for (const f of files) fd.append("files", f, f.name);
    for (const [k, v] of Object.entries(opts)) {
      if (v !== undefined && v !== null) fd.append(k, typeof v === "boolean" ? String(v) : String(v));
    }
    return fd;
  }

  async probe(files: File[], passphrase: string): Promise<ProbeResult> {
    const r = await fetch(`${this.base}/probe`, { method: "POST", body: this.form(files, { passphrase }) });
    if (!r.ok) throw new Error(await r.text());
    return (await r.json()) as ProbeResult;
  }

  async generate(files: File[], opts: GenerateOptions): Promise<GenerateResult> {
    const r = await fetch(`${this.base}/generate`, { method: "POST", body: this.form(files, opts) });
    if (!r.ok) throw new Error(await r.text());
    const deviceCount = parseInt(r.headers.get("X-Device-Count") || String(files.length), 10);
    const locked = r.headers.get("X-Secrets-Locked") === "1";
    return { html: await r.text(), deviceCount, locked };
  }

  async buildArchitecture(devicesJson: string, manifest: string): Promise<string> {
    const fd = new FormData();
    fd.append("devices", devicesJson);
    fd.append("manifest", manifest);
    const r = await fetch(`${this.base}/architecture`, { method: "POST", body: fd });
    if (!r.ok) throw new Error(await r.text());
    return r.text();
  }

  async manual(topic: string): Promise<string> {
    const r = await fetch(`${this.base}/manual?topic=${encodeURIComponent(topic)}`);
    return r.ok ? r.text() : "";
  }
}

/** Stand-in for the standalone in-browser app when its inlined wasm generator
 *  is present on the page but failed to load. It performs NO network I/O: this
 *  app's whole promise is that the config never leaves the browser, so a broken
 *  engine must surface an error — never silently fall back to uploading via
 *  {@link ServerBackend}. Every call rejects with the load reason, so the input
 *  page reports it and keeps generation disabled. */
class UnavailableBackend implements ReportBackend {
  constructor(private reason: string) {}
  private fail(): Promise<never> {
    return Promise.reject(new Error(this.reason));
  }
  ready(): Promise<void> {
    return this.fail();
  }
  engineVersion(): Promise<string> {
    return this.fail();
  }
  probe(): Promise<ProbeResult> {
    return this.fail();
  }
  generate(): Promise<GenerateResult> {
    return this.fail();
  }
  buildArchitecture(): Promise<string> {
    return this.fail();
  }
  manual(): Promise<string> {
    return this.fail();
  }
}

// The `no-modules` wasm-bindgen glue the standalone app inlines *ahead* of this
// bundle declares its init function with a top-level `let wasm_bindgen = …`. A
// top-level `let` is a global *lexical* binding: reachable as a bare identifier
// from later classic scripts (as here), but — unlike `var`/`function` — NOT a
// property of `window`/`globalThis`, so `window.wasm_bindgen` is always
// `undefined`. It must be read as a bare identifier. Declared optional so the
// `typeof` guard type-checks; `typeof` also keeps the bare reference from
// throwing on the server page, where the glue (and this binding) are absent.
declare const wasm_bindgen: WasmBindgen | undefined;

/** Pick the backend for the current page: the inlined wasm generator when it's
 *  present (the standalone in-browser app), else the server. */
export function selectBackend(): ReportBackend {
  const payload = document.getElementById("report-wasm");
  const inlined = payload && payload.textContent ? payload.textContent.trim() : "";
  // A `#report-wasm` payload means THIS is the standalone in-browser app, so the
  // whole pipeline runs client-side and nothing is ever uploaded. In that case
  // we must use the wasm backend — and if its glue didn't load, fail loudly
  // rather than fall through to the uploading server backend below.
  if (inlined) {
    if (typeof wasm_bindgen !== "function") {
      return new UnavailableBackend(
        "the in-browser report engine failed to load — reload the page (your configuration was not uploaded)",
      );
    }
    const bin = atob(inlined);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    return new WasmBackend(wasm_bindgen, bytes);
  }
  // No inlined payload → the f5report web server page: generate server-side.
  return new ServerBackend("");
}
