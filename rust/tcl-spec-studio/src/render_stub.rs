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

//! Render a draft as a Tcl dialect stub.
//!
//! A stub is the lightweight half of the registry: a one-line declaration the
//! compiler reads straight out of a user's own source, giving unknown-command
//! suppression and argument-role analysis for a command that has no
//! `CommandSpec`. See `docs/design/contracts/dialect-stubs.md`.
//!
//! Two deliveries, same line:
//!
//! - [`Mode::Inline`] wraps the lines in the `# tcl-lsp: stubs-begin` /
//!   `stubs-end` markers, to paste at the top of a `.tcl` file.
//! - [`Mode::File`] emits a standalone `<dialect>.tcl.stubs` file.
//!
//! The stub language is deliberately narrower than `CommandSpec` — it has no
//! subcommands, options, types, or hooks. Anything a stub cannot carry is
//! emitted as a comment beside it rather than silently dropped, so what the
//! stub does *not* say stays visible.

use serde_json::Value;

use crate::draft::Draft;

/// Which stub delivery to render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// A `# tcl-lsp: stubs-begin` … `stubs-end` block for inlining in a `.tcl`
    /// file.
    Inline,
    /// A standalone `<dialect>.tcl.stubs` file.
    File,
}

/// The stub-language role name for an [`ArgRole`](tcl_registry::ArgRole)
/// variant, or `None` when the role is the implicit `value` default.
///
/// The inverse of `StubOverlay::parse_role`, so a stub this renders parses
/// back to the role the draft declared.
fn stub_role(role: &str) -> Option<&'static str> {
    match role {
        "Body" => Some("body"),
        "Expr" => Some("expr"),
        "VarWrite" => Some("var"),
        "VarRead" => Some("var_read"),
        "Name" => Some("name"),
        "Pattern" => Some("pattern"),
        "Channel" => Some("channel"),
        "CommandPrefix" => Some("command_prefix"),
        _ => None,
    }
}

/// Roles the stub language cannot express — they fall back to `value` and are
/// listed in the accompanying comment so the loss is visible.
fn is_lossy_role(role: &str) -> bool {
    !matches!(role, "Value") && stub_role(role).is_none()
}

fn as_array(value: Option<&Value>) -> &[Value] {
    value.and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

/// The declared role at each 0-based argument index.
fn roles_by_index(draft: &Draft) -> Vec<(u64, String)> {
    as_array(draft.get("arg_roles"))
        .iter()
        .filter_map(|entry| {
            let index = entry.get("index")?.as_u64()?;
            let role = entry.get("role")?.as_str()?.to_owned();
            Some((index, role))
        })
        .collect()
}

/// A readable parameter name for the argument at `index`, taken from the
/// command's synopsis when one is available and from the role otherwise.
fn param_name(index: u64, role: &str) -> String {
    match role {
        "Body" => "body".to_owned(),
        "Expr" => "expr".to_owned(),
        "VarWrite" | "VarRead" => "varName".to_owned(),
        "Name" => "name".to_owned(),
        "Pattern" => "pattern".to_owned(),
        "Channel" => "channel".to_owned(),
        "CommandPrefix" => "cmdPrefix".to_owned(),
        "ParamList" => "args".to_owned(),
        "Subcommand" => "subcommand".to_owned(),
        _ => format!("arg{}", index + 1),
    }
}

/// Trait-derived stub flags, in the order the stub grammar documents them.
fn flags(draft: &Draft) -> Vec<&'static str> {
    let traits: Vec<&str> = as_array(draft.get("traits"))
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let has = |name: &str| traits.contains(&name);
    let writes_state = as_array(draft.get("side_effects"))
        .iter()
        .any(|e| e.get("writes").and_then(Value::as_bool).unwrap_or(false));

    let mut out = Vec::new();
    if has("CREATES_DYNAMIC_BARRIER") || has("CREATES_BARRIER") {
        out.push("-barrier");
    }
    if has("HAS_LOOP_BODY") {
        out.push("-loop");
    }
    if has("PURE") {
        out.push("-pure");
    }
    if writes_state && !has("PURE") {
        out.push("-mutator");
    }
    if has("UNSAFE")
        || draft
            .get("unsafe_command")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        out.push("-unsafe");
    }
    if has("CREATES_SCOPE_ALIAS") {
        out.push("-scope_alias");
    }
    out
}

/// The `stub NAME {params} ?flags?` line, with no delivery prefix.
#[must_use]
pub fn stub_line(draft: &Draft) -> String {
    let name = draft.get("name").and_then(Value::as_str).unwrap_or("");
    let roles = roles_by_index(draft);
    let arity = draft.get("arity");
    let min = arity
        .and_then(|a| a.get("min"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let max = arity.and_then(|a| a.get("max")).and_then(Value::as_u64);

    // How many parameter slots the stub spells out. A bounded command declares
    // through its maximum, so its optional positions survive into the stub —
    // stopping at `min` would advertise a narrower signature than the spec.
    // An unbounded one declares through its minimum and then takes `?args?`.
    // Either way a role declared past that point still pulls its slot in, so
    // no declared role is lost.
    let highest_role = roles.iter().map(|(i, _)| *i + 1).max().unwrap_or(0);
    let declared = max.unwrap_or(min).max(min).max(highest_role);

    let mut params: Vec<String> = Vec::new();
    for index in 0..declared {
        let role = roles
            .iter()
            .find(|(i, _)| *i == index)
            .map_or("Value", |(_, r)| r.as_str());
        let mut token = param_name(index, role);
        if let Some(stub) = stub_role(role) {
            token.push(':');
            token.push_str(stub);
        }
        if index >= min {
            token = format!("?{token}?");
        }
        params.push(token);
    }
    if max.is_none() {
        params.push("?args?".to_owned());
    }

    let mut line = format!("stub {name} {{{}}}", params.join(" "));
    for flag in flags(draft) {
        line.push(' ');
        line.push_str(flag);
    }
    line
}

/// Comment lines describing what the stub language cannot carry from this
/// draft, so nothing is dropped silently.
#[must_use]
pub fn caveats(draft: &Draft) -> Vec<String> {
    let mut out = Vec::new();

    let subcommands: Vec<&str> = as_array(draft.get("subcommands"))
        .iter()
        .filter_map(|s| s.get("name")?.as_str())
        .collect();
    if !subcommands.is_empty() {
        out.push(format!(
            "stubs have no subcommand grammar — {} declares: {}",
            draft
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("this command"),
            subcommands.join(", ")
        ));
    }

    let options: Vec<&str> = as_array(draft.get("options"))
        .iter()
        .filter_map(|o| o.get("name")?.as_str())
        .collect();
    if !options.is_empty() {
        out.push(format!(
            "stubs have no option table — declared options: {}",
            options.join(", ")
        ));
    }

    if let Some(return_type) = draft.get("return_type").and_then(Value::as_str) {
        out.push(format!(
            "stubs carry no return type — this command returns {return_type}"
        ));
    }

    let lossy: Vec<String> = roles_by_index(draft)
        .into_iter()
        .filter(|(_, role)| is_lossy_role(role))
        .map(|(index, role)| format!("argument {index} is {role}"))
        .collect();
    if !lossy.is_empty() {
        out.push(format!(
            "these roles have no stub spelling and fall back to value: {}",
            lossy.join(", ")
        ));
    }

    out
}

/// Render one or more drafts as a stub document in `mode`.
///
/// `dialect` names the stub file's dialect and appears in the file header; it
/// is unused for [`Mode::Inline`].
#[must_use]
pub fn render(drafts: &[Draft], mode: Mode, dialect: &str) -> String {
    let mut out = String::new();
    match mode {
        Mode::File => {
            out.push_str("# ");
            out.push_str(dialect);
            out.push_str(".tcl.stubs\n");
            out.push_str(
                "# Command stubs for the tcl-lsp compiler. Load this file alongside the\n",
            );
            out.push_str(
                "# sources that use these commands; see docs/design/contracts/dialect-stubs.md.\n",
            );
        }
        Mode::Inline => out.push_str("# tcl-lsp: stubs-begin\n"),
    }

    let prefix = match mode {
        Mode::Inline => "# tcl-lsp: ",
        Mode::File => "",
    };
    let comment = match mode {
        Mode::Inline => "# tcl-lsp: # ",
        Mode::File => "# ",
    };

    for (i, draft) in drafts.iter().enumerate() {
        if i > 0 || mode == Mode::File {
            out.push('\n');
        }
        for caveat in caveats(draft) {
            out.push_str(comment);
            out.push_str(&caveat);
            out.push('\n');
        }
        out.push_str(prefix);
        out.push_str(&stub_line(draft));
        out.push('\n');
    }

    if mode == Mode::Inline {
        out.push_str("# tcl-lsp: stubs-end\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draft;
    use serde_json::json;

    fn drafted(name: &str) -> Draft {
        let mut d = draft::default_command_draft();
        d.insert("name".into(), json!(name));
        d
    }

    #[test]
    fn renders_roles_in_the_stub_spelling() {
        let mut d = drafted("with_var");
        d.insert(
            "arg_roles".into(),
            json!([
                { "index": 0, "role": "VarWrite" },
                { "index": 2, "role": "Body" }
            ]),
        );
        d.insert(
            "arity".into(),
            json!({ "min": 3, "max": 3, "step": 0, "also_exact": null }),
        );
        assert_eq!(stub_line(&d), "stub with_var {varName:var arg2 body:body}");
    }

    #[test]
    fn optional_arguments_are_wrapped_and_variadics_get_args() {
        let mut d = drafted("get_cells");
        d.insert(
            "arg_roles".into(),
            json!([{ "index": 0, "role": "Pattern" }]),
        );
        d.insert(
            "arity".into(),
            json!({ "min": 1, "max": null, "step": 0, "also_exact": null }),
        );
        assert_eq!(stub_line(&d), "stub get_cells {pattern:pattern ?args?}");
    }

    #[test]
    fn a_bounded_range_declares_its_optional_positions() {
        // min 1, max 3 with no roles must still spell out the two optional
        // slots — otherwise the stub is narrower than the spec it came from.
        let mut d = drafted("ranged");
        d.insert(
            "arity".into(),
            json!({ "min": 1, "max": 3, "step": 0, "also_exact": null }),
        );
        assert_eq!(stub_line(&d), "stub ranged {arg1 ?arg2? ?arg3?}");
    }

    #[test]
    fn a_role_declared_past_the_maximum_still_reaches_the_stub() {
        let mut d = drafted("odd");
        d.insert(
            "arity".into(),
            json!({ "min": 1, "max": 1, "step": 0, "also_exact": null }),
        );
        d.insert("arg_roles".into(), json!([{ "index": 2, "role": "Body" }]));
        assert_eq!(stub_line(&d), "stub odd {arg1 ?arg2? ?body:body?}");
    }

    #[test]
    fn traits_become_stub_flags() {
        let mut d = drafted("my_eval");
        d.insert(
            "traits".into(),
            json!(["CREATES_DYNAMIC_BARRIER", "HAS_LOOP_BODY", "PURE"]),
        );
        let line = stub_line(&d);
        assert!(line.ends_with("-barrier -loop -pure"), "{line}");
    }

    #[test]
    fn inline_mode_wraps_in_the_marker_pair() {
        let out = render(&[drafted("foo")], Mode::Inline, "tcl");
        assert!(out.starts_with("# tcl-lsp: stubs-begin\n"));
        assert!(out.contains("# tcl-lsp: stub foo "));
        assert!(out.ends_with("# tcl-lsp: stubs-end\n"));
    }

    #[test]
    fn file_mode_writes_a_header_and_bare_lines() {
        let out = render(&[drafted("foo")], Mode::File, "synopsys");
        assert!(out.starts_with("# synopsys.tcl.stubs\n"));
        assert!(out.contains("\nstub foo "));
        assert!(!out.contains("tcl-lsp: stub"));
    }

    #[test]
    fn what_a_stub_cannot_carry_is_written_beside_it() {
        let mut d = drafted("dict");
        d.insert(
            "subcommands".into(),
            json!([{ "name": "get" }, { "name": "set" }]),
        );
        d.insert("return_type".into(), json!("Dict"));
        let notes = caveats(&d);
        assert!(notes.iter().any(|n| n.contains("get, set")));
        assert!(notes.iter().any(|n| n.contains("returns Dict")));
    }
}
