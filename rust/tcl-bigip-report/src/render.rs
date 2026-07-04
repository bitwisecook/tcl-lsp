//! Render the report model to a single, self-contained HTML document.
//!
//! A port of `f5report.render`: the interesting work is done by the query
//! engine and the model shaping ([`crate::model`]); this module only turns the
//! model into one standalone HTML file — embedded CSS/JS, the interactive
//! Mermaid topology / listener / certificate views, and the in-browser
//! `f5-query` WebAssembly console — with no external assets and no CDN.
//!
//! Uses `minijinja` (pure Rust, so it also builds for wasm32) in place of
//! Jinja2. The template is [`REPORT_TEMPLATE`]; the CSS / JS / Mermaid / wasm
//! assets are the same vendored artifacts the Python `f5report` package ships,
//! embedded at compile time so the two generators stay byte-identical.

use base64::Engine as _;
use minijinja::{AutoEscape, Environment};
use serde_json::{Map, Value as J, json};

use crate::model::collect_model_with_certs;
use crate::query::{ReportError, Source};

// The template we own (adapted from `f5report`'s `report.html.j2` for minijinja
// and extended with the SSL-certificate tab).
const REPORT_TEMPLATE: &str = include_str!("../templates/report.html.j2");

// Assets shared with — and canonically owned by — the Python `f5report`
// package, embedded verbatim so both generators emit the same page.
const REPORT_CSS: &str = include_str!("../../bigip-query-py/python/f5report/templates/report.css");
const TOPOLOGY_CSS: &str =
    include_str!("../../bigip-query-py/python/f5report/templates/topology.css");
const REPORT_JS: &str = include_str!("../../bigip-query-py/python/f5report/templates/report.js");
const TOPOLOGY_JS: &str =
    include_str!("../../bigip-query-py/python/f5report/templates/topology.js");
const CONSOLE_JS: &str = include_str!("../../bigip-query-py/python/f5report/templates/console.js");
const MERMAID_JS: &str = include_str!("../../bigip-query-py/python/f5report/vendor/mermaid.min.js");
const WASM_GLUE: &str = include_str!("../../bigip-query-py/python/f5report/vendor/f5query_wasm.js");
const WASM_BIN: &[u8] =
    include_bytes!("../../bigip-query-py/python/f5report/vendor/f5query_wasm_bg.wasm");

// Assets this crate adds (the certificate + secrets tabs).
const CERTS_CSS: &str = include_str!("../templates/certs.css");
const CERTS_JS: &str = include_str!("../templates/certs.js");
const SECRETS_CSS: &str = include_str!("../templates/secrets.css");
const SECRETS_JS: &str = include_str!("../templates/secrets.js");
const IRULE_FLOW_JS: &str = include_str!("../templates/irule-flow.js");

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
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            title: "F5 BIG-IP Configuration Report".to_string(),
            generated_at: String::new(),
            embed_console: true,
            cert_pems: std::collections::HashMap::new(),
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
    ctx.insert("model_json".into(), J::String(model_json));
    ctx.insert("report_css".into(), J::String(REPORT_CSS.into()));
    ctx.insert("topology_css".into(), J::String(TOPOLOGY_CSS.into()));
    ctx.insert("certs_css".into(), J::String(CERTS_CSS.into()));
    ctx.insert("secrets_css".into(), J::String(SECRETS_CSS.into()));
    ctx.insert("report_js".into(), J::String(REPORT_JS.into()));
    ctx.insert("topology_js".into(), J::String(TOPOLOGY_JS.into()));
    ctx.insert("certs_js".into(), J::String(CERTS_JS.into()));
    ctx.insert("secrets_js".into(), J::String(SECRETS_JS.into()));
    ctx.insert("irule_flow_js".into(), J::String(IRULE_FLOW_JS.into()));
    ctx.insert("mermaid_js".into(), J::String(MERMAID_JS.into()));
    ctx.insert("topo_types".into(), topo_types());

    if opts.embed_console {
        ctx.insert("has_console".into(), J::Bool(true));
        ctx.insert("wasm_glue".into(), J::String(WASM_GLUE.into()));
        ctx.insert("console_js".into(), J::String(CONSOLE_JS.into()));
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
    let model = collect_model_with_certs(sources, &opts.title, &opts.cert_pems);
    render_report(model, opts)
}
