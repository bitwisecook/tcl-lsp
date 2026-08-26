// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! MCP projection of the shared, registry-driven static Tk UI model.

use serde_json::{Value, json};

/// Extract the same versioned Tk UI model consumed by the LSP preview.
///
/// This tool accepts source because MCP requests are not attached to an open
/// editor document. It never executes that source. Command identity, widget
/// constructors, geometry managers, options, spans, and uncertainty all come
/// from the shared tcl-lsp-core Tk preview analysis.
pub fn tk_layout(args: &Value) -> Value {
    let source = args.get("source").and_then(Value::as_str).unwrap_or("");
    let dialect = args.get("dialect").and_then(Value::as_str).unwrap_or("tk");
    let Some(profile) = crate::environment::known_profile_for_dialect(dialect) else {
        return json!({
            "error": format!("Unknown dialect '{dialect}'."),
            "schema_version": tcl_lsp_core::tk_preview::TK_UI_SCHEMA_VERSION,
        });
    };
    let registry = tcl_spectcl::bundled::registry_for_dialect(profile.name);
    serde_json::to_value(tcl_lsp_core::tk_preview::analyse_tk_ui(
        source, profile, &registry,
    ))
    .unwrap_or_else(|error| json!({ "error": error.to_string() }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_the_shared_versioned_schema_for_real_tcl_syntax() {
        let source = concat!(
            "package require Tk\n",
            "ttk::frame .main \\\n    -padding 8\n",
            "ttk::button .main.ok -text {Save changes}\n",
            "grid .main.ok -row 0 -column 0\n",
        );
        let result = tk_layout(&json!({
            "source": source,
            "dialect": "tk",
        }));
        assert_eq!(result["schema_version"], 1, "{result}");
        assert_eq!(result["tk_active"], true, "{result}");
        assert_eq!(result["widget_count"], 3, "{result}");
        assert_eq!(result["root"]["children"][0]["path"], ".main", "{result}");
        assert_eq!(
            result["root"]["children"][0]["children"][0]["options"]["-text"]["value"],
            "Save changes",
            "{result}"
        );
    }

    #[test]
    fn dynamic_paths_are_uncertainty_not_invented_widgets() {
        let result = tk_layout(&json!({
            "source": "package require Tk\nframe $parent.child",
            "dialect": "tk",
        }));
        assert_eq!(result["widget_count"], 1, "{result}");
        assert_eq!(result["uncertainties"][0]["kind"], "dynamic_widget_path");
    }

    #[test]
    fn unknown_dialect_is_an_explicit_error() {
        let result = tk_layout(&json!({ "source": "", "dialect": "not-a-dialect" }));
        assert!(result["error"].as_str().is_some(), "{result}");
    }
}
