// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Render the report model to a single, self-contained HTML document.
//!
//! A port of `f5report.render`: the interesting work is done by the query
//! engine and the model shaping ([`crate::model`]); this module only turns the
//! model into one standalone HTML file — embedded CSS/JS, the interactive
//! elkjs topology / listener / certificate views, and the in-browser
//! `f5-query` WebAssembly console — with no external assets and no CDN.
//!
//! Uses `minijinja` (pure Rust, so it also builds for wasm32) — the same
//! engine the Python `f5report` package renders through. The template is
//! [`REPORT_TEMPLATE`]; the CSS / JS / elkjs / wasm
//! assets are the same vendored artifacts the Python `f5report` package ships,
//! embedded at compile time so the two generators stay byte-identical.

use base64::Engine as _;
use minijinja::{AutoEscape, Environment};
use serde_json::{Map, Value as J, json};

use crate::model::collect_model_full;
use crate::query::{ReportError, Source};

// The single shared report template both generators render (`report.html.j2`).
const REPORT_TEMPLATE: &str = include_str!("../../templates/report.html.j2");

// Front-end shared with the Python `f5report` generator: TypeScript sources live
// in `rust/bigip-report/shared/src` (`pages/`, `search/`, `styles/`), built to
// `shared/dist/*.js` (committed) and embedded verbatim here so both generators
// emit the same page. Vendored third-party assets live in `shared/public`. The
// rendered report is a single self-contained HTML file — every asset inlined.
const REPORT_CSS: &str = include_str!("../../frontend/src/styles/report.css");
const TOPOLOGY_CSS: &str = include_str!("../../frontend/src/styles/topology.css");
const REPORT_JS: &str = include_str!("../../frontend/dist/report.js");
const TOPOLOGY_JS: &str = include_str!("../../frontend/dist/topology.js");
const CONSOLE_JS: &str = include_str!("../../frontend/dist/console.js");
const WASM_GLUE: &str = include_str!("../../assets/f5query_wasm.js");
const WASM_BIN: &[u8] = include_bytes!("../../assets/f5query_wasm_bg.wasm");

// The certificate + secrets + APM tabs (their scripts/styles also live in
// `shared/`; the APM walk itself is embedded only by this Rust generator, but
// elkjs + the renderer are shared with the Python generator).
const CERTS_CSS: &str = include_str!("../../frontend/src/styles/certs.css");
const CERTS_JS: &str = include_str!("../../frontend/dist/certs.js");
const SECRETS_CSS: &str = include_str!("../../frontend/src/styles/secrets.css");
const SECRETS_JS: &str = include_str!("../../frontend/dist/secrets.js");
const FORENSICS_CSS: &str = include_str!("../../frontend/src/styles/forensics.css");
const FORENSICS_JS: &str = include_str!("../../frontend/dist/forensics.js");
const IRULE_FLOW_JS: &str = include_str!("../../frontend/dist/irule-flow.js");
const IRULE_FORMAT_JS: &str = include_str!("../../frontend/dist/irule-format.js");
const PRINT_CSS: &str = include_str!("../../frontend/src/styles/print.css");
const PRINT_JS: &str = include_str!("../../frontend/dist/print.js");
const APM_CSS: &str = include_str!("../../frontend/src/styles/apm.css");
const APM_JS: &str = include_str!("../../frontend/dist/apm.js");
// elkjs (EPL-2.0), the ELK layout engine, for the orthogonal diagrams.
const ELK_JS: &str = include_str!("../../assets/elk.bundled.js");
const ELK_GRAPH_JS: &str = include_str!("../../frontend/dist/elk-graph.js");

// Project marks, inlined into the report as <svg> elements (not <img> data:
// URIs) so they inherit the page's theme and stay crisp at any zoom. Copies of
// the canonical `docs/*.svg`, propagated here by `make logo`; their ids are
// namespaced (`f5q-`, `tcl-`, `tcld-`) so all three can coexist in one document.
// tcl-lsp ships light and dark variants; the report shows whichever matches the
// active theme. f5-query has a single (dark squircle) variant that reads on both.
const LOGO_F5Q_SVG: &str = include_str!("../../assets/logo-f5q.svg");
const LOGO_TCL_LSP_SVG: &str = include_str!("../../assets/logo-tcl-lsp.svg");
const LOGO_TCL_LSP_DARK_SVG: &str = include_str!("../../assets/logo-tcl-lsp-dark.svg");

/// Options controlling report rendering.
pub struct RenderOptions {
    /// Document title.
    pub title: String,
    /// A human-readable generation timestamp (e.g. `2026-07-03 12:00:00 UTC`).
    /// Passed in so the caller — the browser — can stamp it with the local
    /// clock; the engine itself is time-free.
    pub generated_at: String,
    /// Embed the in-browser WASM `f5-query` console. Off yields a smaller page
    /// (e.g. for hosting behind a strict CSP that blocks WebAssembly).
    pub embed_console: bool,
    /// Certificate PEMs recovered from the UCS filestore, keyed **by source
    /// URI** and then by the `sys file ssl-cert` `cache-path`. Lets the certs
    /// tab parse metadata-free stanzas and reconstruct the trust chain. The
    /// outer key scopes PEMs to their device so a shared filestore
    /// `cache-path` across two UCS files in one report doesn't collide. Empty =
    /// config metadata only.
    pub cert_pems: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    /// UCS file inventory for the Forensics tab, keyed **by source URI** → the
    /// list of that device's extracted members (each a JSON object
    /// `{path, size, sha256, isText, content?}`, from
    /// [`tcl_bigip_io::list_ucs_members`] / `read_ucs_member`). Empty = no
    /// archive behind the source (e.g. a bare `bigip.conf`).
    pub files: std::collections::HashMap<String, Vec<serde_json::Value>>,
    /// Optional *architecture manifest* — a small Tcl script (see
    /// [`tcl_bigip_query::architecture`]). Declares each device's role/tier and can add
    /// explicit inter-device links, overriding and augmenting auto-detection.
    /// Empty = pure auto-detection.
    pub architecture: Option<String>,
    /// Stable per-report id, embedded as `<html data-report-id>` so the in-report
    /// architecture editor keys its localStorage per report. Empty = the report's
    /// own JS mints and persists one on first load.
    pub report_id: String,
    /// Optional copyright / confidentiality notice rendered in the report footer
    /// (screen, mobile and every printed page). Empty = no notice.
    pub copyright: String,
    /// Optional user-supplied **Markdown** front-matter. Rendered to HTML at
    /// generation time (raw HTML stripped) and shown in a dedicated "Front
    /// matter" tab. Empty = no tab.
    pub front_matter: String,
    /// Optional report logo as an inlined `data:` URI, shown in the report
    /// header. Empty = the f5-query mark, inlined as `<svg>`.
    pub logo: String,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            title: "F5 BIG-IP Configuration Report".to_string(),
            generated_at: String::new(),
            embed_console: true,
            cert_pems: std::collections::HashMap::new(),
            files: std::collections::HashMap::new(),
            architecture: None,
            report_id: String::new(),
            copyright: String::new(),
            front_matter: String::new(),
            logo: String::new(),
        }
    }
}

/// Serialise to JSON safe to embed inside a `<script>` element.
///
/// Escapes `<` / `>` / `&` so an iRule body (or a cert subject) containing e.g.
/// `</script>` can never break out of the tag. Mirrors
/// `f5report.render._script_safe_json`.
fn script_safe_json(value: &J) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "null".to_string())
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

fn topo_types() -> J {
    let pairs = [
        ("vs", "Virtual"),
        ("pool", "Pool"),
        ("node", "Node"),
        ("mon", "Monitor"),
        ("rule", "iRule"),
        ("prof", "Profile"),
        ("persist", "Persist"),
        ("policy", "Policy"),
        ("snat", "SNAT"),
        ("dg", "DataGroup"),
    ];
    J::Array(
        pairs
            .iter()
            .map(|(t, lbl)| json!({"t": t, "lbl": lbl}))
            .collect(),
    )
}

/// Render a pre-built report `model` to a standalone HTML document.
pub fn render_report(model: J, opts: &RenderOptions) -> Result<String, ReportError> {
    // The embedded model JSON is the *data* model plus `generated_at` — not the
    // asset strings, which are added to the render context afterwards (matching
    // the Python renderer, whose `model_json` is computed before the assets).
    let mut data = match model {
        J::Object(m) => m,
        _ => return Err(ReportError("report model must be a JSON object".into())),
    };
    data.insert("generated_at".into(), J::String(opts.generated_at.clone()));
    let model_json = script_safe_json(&J::Object(data.clone()));

    // Build the render context: the data model + assets + flags.
    let mut ctx: Map<String, J> = data;
    ctx.insert("title".into(), J::String(opts.title.clone()));
    ctx.insert("copyright".into(), J::String(opts.copyright.clone()));
    // User-supplied Markdown front-matter, rendered to HTML (raw HTML stripped).
    // Empty stays empty so the template's `{% if front_matter_html %}` tab guard
    // hides the tab when there is no front-matter.
    let front_matter_html = if opts.front_matter.trim().is_empty() {
        String::new()
    } else {
        crate::render_markdown(&opts.front_matter)
    };
    ctx.insert("front_matter_html".into(), J::String(front_matter_html));
    ctx.insert("logo".into(), J::String(opts.logo.clone()));
    ctx.insert("report_id".into(), J::String(opts.report_id.clone()));
    ctx.insert(
        "f5q_manual".into(),
        J::String(tcl_bigip_query::manual::format_manual()),
    );
    ctx.insert("model_json".into(), J::String(model_json));
    ctx.insert("report_css".into(), J::String(REPORT_CSS.into()));
    ctx.insert("topology_css".into(), J::String(TOPOLOGY_CSS.into()));
    ctx.insert("print_css".into(), J::String(PRINT_CSS.into()));
    ctx.insert("certs_css".into(), J::String(CERTS_CSS.into()));
    ctx.insert("secrets_css".into(), J::String(SECRETS_CSS.into()));
    ctx.insert("forensics_css".into(), J::String(FORENSICS_CSS.into()));
    ctx.insert("report_js".into(), J::String(REPORT_JS.into()));
    ctx.insert("topology_js".into(), J::String(TOPOLOGY_JS.into()));
    ctx.insert("certs_js".into(), J::String(CERTS_JS.into()));
    ctx.insert("secrets_js".into(), J::String(SECRETS_JS.into()));
    ctx.insert("forensics_js".into(), J::String(FORENSICS_JS.into()));
    ctx.insert("irule_flow_js".into(), J::String(IRULE_FLOW_JS.into()));
    ctx.insert("print_js".into(), J::String(PRINT_JS.into()));
    ctx.insert("apm_css".into(), J::String(APM_CSS.into()));
    ctx.insert("apm_js".into(), J::String(APM_JS.into()));
    ctx.insert("elk_js".into(), J::String(ELK_JS.into()));
    ctx.insert("elk_graph_js".into(), J::String(ELK_GRAPH_JS.into()));
    ctx.insert("logo_f5q_svg".into(), J::String(LOGO_F5Q_SVG.into()));
    ctx.insert(
        "logo_tcl_lsp_svg".into(),
        J::String(LOGO_TCL_LSP_SVG.into()),
    );
    ctx.insert(
        "logo_tcl_lsp_dark_svg".into(),
        J::String(LOGO_TCL_LSP_DARK_SVG.into()),
    );
    ctx.insert("topo_types".into(), topo_types());

    if opts.embed_console {
        ctx.insert("has_console".into(), J::Bool(true));
        ctx.insert("wasm_glue".into(), J::String(WASM_GLUE.into()));
        ctx.insert("console_js".into(), J::String(CONSOLE_JS.into()));
        ctx.insert("irule_format_js".into(), J::String(IRULE_FORMAT_JS.into()));
        ctx.insert(
            "wasm_b64".into(),
            J::String(base64::engine::general_purpose::STANDARD.encode(WASM_BIN)),
        );
    } else {
        ctx.insert("has_console".into(), J::Bool(false));
    }

    // The title is set on the model object too (the model may carry its own).
    if !opts.title.is_empty() {
        ctx.insert("title".into(), J::String(opts.title.clone()));
    }

    let mut env = Environment::new();
    env.set_auto_escape_callback(|_| AutoEscape::Html);
    env.add_filter("leaf", |s: &str| -> String {
        s.rsplit('/').next().unwrap_or(s).to_string()
    });
    env.add_template("report", REPORT_TEMPLATE)
        .map_err(|e| ReportError(format!("template compile error: {e}")))?;
    let tmpl = env.get_template("report").expect("template just added");
    tmpl.render(J::Object(ctx))
        .map_err(|e| ReportError(format!("template render error: {e}")))
}

/// Collect the model from `sources` and render it to a standalone HTML document.
pub fn build_report(sources: &[Source], opts: &RenderOptions) -> Result<String, ReportError> {
    let model = collect_model_full(
        sources,
        &opts.title,
        &opts.cert_pems,
        &opts.files,
        opts.architecture.as_deref(),
    );
    render_report(model, opts)
}
