//! HTTP probe backend for the `url_*` builtins.
//!
//! The live request path (a port of `dialects/f5/query/_probes.url_request`)
//! is **deferred** in the Rust port: wiring `ureq` 3.x + a custom `rustls` CA
//! bundle proved fiddly to get byte-faithful, and the result is non-
//! deterministic / un-golden-testable anyway. This stub returns the same
//! result-dict shape Python emits (`{status, headers, body, body_json,
//! peer_cert, error}`) with an explanatory `error`, so a `--enable-probes`
//! `url_*` query degrades gracefully rather than failing to compile. The
//! deterministic probe surface (`x509_parse` / `x509_eq` / `dns`) is fully
//! implemented; live HTTP can be revisited as a focused follow-up.

use indexmap::IndexMap;

use crate::value::Value;

const DEFERRED: &str = "live HTTP probe is not yet implemented in the Rust port \
                        (run the query through the Python f5 CLI for live url_* probes)";

/// Shape the deferred `url_*` result dict.
pub(super) fn request(
    _method: &str,
    _url: &str,
    _body: Option<&str>,
    _headers: &[(String, String)],
    _ca_bundle: Option<&str>,
) -> Value {
    let mut m: IndexMap<String, Value> = IndexMap::new();
    m.insert("status".to_owned(), Value::Null);
    m.insert("headers".to_owned(), Value::Object(IndexMap::new()));
    m.insert("body".to_owned(), Value::Str(String::new()));
    m.insert("body_json".to_owned(), Value::Null);
    m.insert("peer_cert".to_owned(), Value::Null);
    m.insert("error".to_owned(), Value::Str(DEFERRED.to_owned()));
    Value::Object(m)
}
