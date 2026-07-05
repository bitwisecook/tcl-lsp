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

//! HTTP probe backend for the `url_*` builtins.
//!
//! The live request path is **not implemented**: a real `ureq` 3.x + custom
//! `rustls` CA bundle request is non-deterministic / un-golden-testable, so
//! this backend instead returns the same result-dict shape the live path would emit
//! (`{status, headers, body, body_json, peer_cert, error}`) with an
//! explanatory `error`, so a `--enable-probes` `url_*` query degrades
//! gracefully rather than failing to compile. The deterministic probe surface
//! (`x509_parse` / `x509_eq` / `dns`) is fully implemented; live HTTP is not.

use indexmap::IndexMap;

use crate::value::Value;

const DEFERRED: &str = "live HTTP probe is not yet implemented \
                        (live url_* probes are unsupported)";

/// Shape the unsupported-`url_*` result dict.
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
