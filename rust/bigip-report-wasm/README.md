# bigip-report-wasm — the BIG-IP report generator, in your browser

A single, self-contained web page that turns an F5 BIG-IP config into an
interactive HTML report **entirely client-side**. You drop in a `.ucs` backup
(plain **or** OpenPGP-encrypted) or a `bigip.conf` / SCF, and the page — running
`tcl-bigip-query` (the `f5-query` engine) and the report renderer compiled from
Rust to WebAssembly — decrypts, parses, analyses and renders the report locally,
then downloads it. **Nothing is uploaded**; no network request touches your data
once the WASM has loaded (you can verify that in DevTools → Network, or just go
offline).

It is the WebAssembly sibling of the Python [`f5report`](../bigip-query-py)
package: the Python version stays as the demonstration of driving the query
engine as a **library** (PyO3), while this crate is the same generator
([`tcl-bigip-report`](../tcl-bigip-report)) compiled for the browser.

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
| `extract_source(name, bytes, passphrase, master_key)` | one uploaded file → SCF text (UCS decrypt/extract + optional `$M$` secret decryption). |
| `generate_report(sources_json, title, generated_at, embed_console)` | ordered `[[uri, scf], …]` → standalone HTML report. |
| `engine_version()` | the report engine version string. |

## Building

Requires the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` (matching the
pinned `wasm-bindgen` crate version), `wasm-opt` (binaryen) and `python3`:

```bash
bash build-wasm.sh          # → dist/index.html (WASM + glue inlined, one file)
```

`dist/index.html` is fully self-contained — host it on any static server, or
just open it from disk. `target/` and `dist/` are build outputs (gitignored);
CI (`rust/bigip-query-py/deploy/github-pages.yml`) builds and publishes it to
`/bigip-report/`.

This crate is **excluded from the Cargo workspace** (like `bigip-query-wasm`):
wasm-bindgen's generated glue needs `unsafe`, which the workspace
`unsafe_code = "forbid"` lint bans, and it targets wasm32 with its own lockfile.
