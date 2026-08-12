# tcl-bigip-report — the BIG-IP report generator (Rust)

The Rust port of the [`f5report`](../python) generator: given one or more
loaded `(uri, scf_text)` configs it produces a single, self-contained,
interactive HTML report — object tables, a reference/orphan analysis, an
**SSL-certificate expiry inventory**, an **APM access-profile walk** (a
Visual-Policy-Editor-style dependency graph per `apm profile access`), a Mermaid
topology explorer, a listener/flow simulator, an **F5 Sites** tab of community /
support / security incident-response references plus a UCS forensic
(ATT&CK-mapped) hunting checklist, and an embedded in-browser `f5-query` console
— with no server and no external assets.

The heavy lifting (config parsing, object projection, the `referenced_by`
reference-graph walk) is done by [`tcl-bigip-query`](../../tcl-bigip-query); this
crate only *shapes* that output into a model and *renders* it. It is pure Rust
and builds for wasm32, which is what lets the whole pipeline run in the browser
via [`bigip-report-wasm`](../wasm).

| | |
|---|---|
| `collect_model(sources, title)` | run the engine queries → the report model (`serde_json::Value`). |
| `build_report(sources, &RenderOptions)` | `collect_model` + render to one standalone HTML document. |
| `decrypt_secrets(scf, master_key)` | decrypt the config's `f5mku` `$M$…` secrets (via [`tcl-f5mku`](../../tcl-f5mku)). |

## Layout

| Path | What |
|------|------|
| `src/model.rs` | port of `f5report.report` — the per-object shaping + orphan/insight passes. |
| `src/graph.rs` | port of `f5report.graph` — the object graph, listener fields, iRule dynamic actions. |
| `src/certs.rs` | the SSL-certificate + private-key inventory (read from the parsed model, since the DSL only projects `ltm`). |
| `src/apm.rs` | the APM access-profile walk — parses the `apm …` stanzas from the config text and emits a per-profile `{nodes, edges}` dependency-graph model (rendered client-side by the elkjs orthogonal renderer, `templates/elk-graph.js`) plus its linked-object list. |
| `src/secrets.rs` | `f5mku` master-key secret decryption. |
| `src/render.rs` + `templates/` | minijinja rendering to one HTML file (CSS/JS/Mermaid/wasm-console embedded). |

The CSS/JS/Mermaid and the vendored WASM query-console assets are the same
artifacts the Python `f5report` package ships, embedded at compile time so both
generators emit the same page. The two are validated against the **same UCS
fixtures** (`tests/report.rs`).

## Relationship to the Python `f5report`

The Python package is kept intact as the demonstration of using the query engine
as a **library** (PyO3). This crate is the equivalent generator in Rust, so it
can be compiled to WebAssembly and run client-side. The two are feature-parity
(both grew the certificate tab and `f5mku` support together).
