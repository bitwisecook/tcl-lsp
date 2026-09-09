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

//! `xc_translate` — iRule → F5 Distributed Cloud (XC) static translation,
//! backed by the `f5-xc` crate (model + terraform / JSON-API renderers).
//!
//! The payload is `f5-xc`'s own reporting shape, so this tool and the
//! `tcl-lsp.xcTranslate` workspace command the editors invoke report a
//! translation identically.

use f5_xc::{DEFAULT_LB_NAME, DEFAULT_NAMESPACE, OutputFormat};
use serde_json::Value;

/// Translate `source` to XC constructs and render terraform / JSON-API per
/// `output_format` (`"terraform"` | `"json"` | `"both"`, default `"both"`).
pub fn xc_translate(args: &Value) -> Value {
    let source = args.get("source").and_then(Value::as_str).unwrap_or("");
    let output_format = OutputFormat::from_arg(args.get("output_format").and_then(Value::as_str));
    f5_xc::translation_payload(
        &f5_xc::translate_irule(source),
        DEFAULT_NAMESPACE,
        DEFAULT_LB_NAME,
        output_format,
    )
}
