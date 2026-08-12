# bigip-report-wasm — the BIG-IP report generator, in your browser

A single, self-contained web page that turns an F5 BIG-IP config into an
interactive HTML report **entirely client-side**. You drop in a `.ucs` backup
(plain **or** OpenPGP-encrypted) or a `bigip.conf` / SCF, and the page — running
`tcl-bigip-query` (the `f5-query` engine) and the report renderer compiled from
Rust to WebAssembly — decrypts, parses, analyses and renders the report locally,
then downloads it. **Nothing is uploaded**; no network request touches your data
once the WASM has loaded (you can verify that in DevTools → Network, or just go
offline).

It is the WebAssembly sibling of the Python [`f5report`](../python)
package: the Python version stays as the demonstration of driving the query
engine as a **library** (PyO3), while this crate is the same generator
([`bigip-report-gen-rust`](../rust)) compiled for the browser.

## Features

- **Upload UCS / SCF / `bigip.conf`**, one or more devices at once.
- **Encrypted UCS** — the passphrase is taken in-page and used by a pure-Rust
  OpenPGP implementation; it never leaves the browser.
- **`f5mku` secret decryption** — paste the base64 unit master key
  (`f5mku -K`) and the config's `$M$…` secrets (SSL private-key passphrases,
  monitor / RADIUS / SNMP secrets) are decrypted locally so the report shows
  them in clear.
- The generated report includes the object tables, reference/orphan analysis,
  the **SSL-certificate expiry inventory** (with live days-to-expiry and the
  paired private key), the Mermaid topology explorer, the listener simulator and
  the embedded in-browser `f5-query` console.

## Exported WASM API (`src/lib.rs`)

| Function | What |
|----------|------|
| `extract_source(name, bytes, passphrase)` | one uploaded file → SCF text (UCS decrypt/extract; `$M$` decryption is the separate `decrypt_secrets`). |
| `extract_cert_files(name, bytes, passphrase, scf)` | filestore SSL-cert PEMs → JSON `{cache_path: pem}`. |
| `extract_files(name, bytes, passphrase)` | UCS member inventory → JSON `[{path, size, sha256, isText, content?}]` (Forensics tab). |
| `secret_count(scf)` | number of `$M$…` encrypted secrets. |
| `decrypt_secrets(scf, master_key)` | decrypt `$M$…` secrets with the base64 `f5mku -K` key. |
| `generate_report(sources_json, cert_files_json, files_json, title, generated_at, embed_console, architecture_manifest, report_id)` | ordered `[[uri, scf], …]` + extras → standalone HTML report. |
| `build_architecture(devices_json, manifest)` | re-run architecture/topology detection for the builder's GUI editor → `architecture` JSON. |
| `engine_version()` | the report engine version string. |

The page (styles + upload controller) is the shared builder front-end
(`rust/bigip-report-gen/frontend/src/pages/input.ts`); `build-wasm.sh` inlines the
generator wasm behind it.

## Building

Requires the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` (matching the
pinned `wasm-bindgen` crate version), `wasm-opt` (binaryen) and `python3`:

```bash
bash build-wasm.sh          # → dist/index.html (WASM + glue inlined, one file)
```

`dist/index.html` is fully self-contained — host it on any static server, or
just open it from disk. `target/` and `dist/` are build outputs (gitignored);
CI (the `github-pages` workflow) builds and publishes it to
`/bigip-report-generator/`.

> Note: `build-wasm.sh` deliberately skips `wasm-opt`. On modern rustc layouts
> binaryen rebinds the `__wbindgen_externrefs` export onto the fixed-size
> funcref table, which makes `Table.grow` fail at runtime and the page never
> initialises; the raw wasm-bindgen output is correct and, gzipped, within ~1%
> of the optimised size.

This crate is **excluded from the Cargo workspace** (like `bigip-query-wasm`):
wasm-bindgen's generated glue needs `unsafe`, which the workspace
`unsafe_code = "forbid"` lint bans, and it targets wasm32 with its own lockfile.
