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

use super::*;
use tcl_dialect::model::EnvironmentRegistry;

#[test]
fn command_info_preserves_exact_literal_names() {
    let ordinary = dispatch("command_info", &json!({ "command_name": "HTTP::uri" }))
        .expect("command_info tool");
    assert_eq!(ordinary["command"], "HTTP::uri");
    assert_eq!(ordinary["found"], true);
    for name in [
        " HTTP::uri ",
        " HTTP::uri",
        "HTTP::uri ",
        "\tHTTP::uri",
        "HTTP::uri\n",
        "HTTP::\0uri",
        "missing-command-name",
    ] {
        let result =
            dispatch("command_info", &json!({ "command_name": name })).expect("command_info tool");
        assert_eq!(result["command"], name, "{name:?}");
        assert_eq!(result["found"], false, "{name:?}");
    }
}

/// Serialise the tests that move the session dialect — it is process
/// state shared by the whole parallel test binary — and restore it on
/// drop, so neither test order nor interleaving can change what another
/// test's auto-detected dialect resolves to.
struct SessionDialectGuard {
    saved: String,
    _serialised: std::sync::MutexGuard<'static, ()>,
}

impl SessionDialectGuard {
    fn new() -> Self {
        static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let serialised = match SERIAL.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        Self {
            saved: session_dialect().lock().expect("session lock").clone(),
            _serialised: serialised,
        }
    }
}

impl Drop for SessionDialectGuard {
    fn drop(&mut self) {
        self.saved
            .clone_into(&mut session_dialect().lock().expect("session lock"));
    }
}

fn dialect_property(tool: &str) -> Value {
    let (_, _, schema) = tool_schemas()
        .into_iter()
        .find(|(name, _, _)| *name == tool)
        .expect("tool in catalogue");
    schema["properties"][DIALECT_PARAM].clone()
}

/// Every dialect-taking tool's enum, the `set_dialect` rejection's
/// `valid_dialects` and the description pairs are the registry's
/// selectable canonical ids, in selectable order.
#[test]
fn every_runtime_enumeration_is_the_registry() {
    let selectable = EnvironmentRegistry::compiled_selectable();
    let expected: Vec<Value> = selectable
        .iter()
        .map(|environment| json!(environment.id.as_str()))
        .collect();
    let mut checked = 0;
    for (name, _, schema) in tool_schemas() {
        let Some(property) = schema["properties"].get(DIALECT_PARAM) else {
            continue;
        };
        assert_eq!(property["enum"], Value::Array(expected.clone()), "{name}");
        let description = property["description"].as_str().unwrap_or_default();
        for environment in selectable {
            assert!(
                description.contains(&format!(
                    "{}: {}",
                    environment.id,
                    environment.description()
                )),
                "{name}: {description}"
            );
        }
        checked += 1;
    }
    assert!(checked >= 4, "expected several dialect-taking tools");
    let rejected = dispatch("set_dialect", &json!({ "dialect": "klingon" })).expect("tool");
    assert_eq!(rejected["valid_dialects"], Value::Array(expected));
}

/// `jim`, `tk` and the tool shells are advertised; the lenient sink is not.
#[test]
fn the_dialect_enum_offers_jim_and_tk_but_not_the_sink() {
    let property = dialect_property("analyze");
    let names: Vec<&str> = property["enum"]
        .as_array()
        .expect("enum")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    for name in ["jim", "tk", "xilinx-eda-tcl", "tcl8.6"] {
        assert!(names.contains(&name), "{name}");
    }
    assert!(
        !names.contains(&"tcl"),
        "the sink is a fallback, not a choice"
    );
}

#[test]
fn set_dialect_canonicalises_an_alias() {
    let _guard = SessionDialectGuard::new();
    let result = dispatch("set_dialect", &json!({ "dialect": "irules" })).expect("tool");
    assert_eq!(result["dialect"], "f5-irules");
    assert_eq!(result["display_name"], "F5 iRules");
    assert_eq!(result["requested"], "irules");
    assert_eq!(
        session_dialect().lock().expect("session lock").as_str(),
        "f5-irules"
    );
}

#[test]
fn set_dialect_rejects_an_unknown_name_without_changing_the_session() {
    let _guard = SessionDialectGuard::new();
    let before = session_dialect().lock().expect("session lock").clone();
    let result = dispatch("set_dialect", &json!({ "dialect": "klingon" })).expect("tool");
    let error = result["error"].as_str().expect("error message");
    assert!(error.contains("klingon"), "{error}");
    assert!(error.contains("f5-irules"), "{error}");
    assert_eq!(result["valid_dialects"], json!(dialect_names()));
    assert_eq!(*session_dialect().lock().expect("session lock"), before);
}

#[test]
fn dialect_enum_lists_canonical_names_only() {
    let property = dialect_property("analyze");
    let names: Vec<&str> = property["enum"]
        .as_array()
        .expect("enum")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(names.contains(&"f5-irules"));
    assert!(!names.contains(&"irules"), "aliases stay out of the enum");
}
