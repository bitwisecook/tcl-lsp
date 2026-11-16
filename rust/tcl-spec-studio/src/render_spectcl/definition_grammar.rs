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

//! Rows for grammars as data: manufacturers, runtime backing, semantic operations and hook windows, definition bodies with their members, clause grammars and object classes.

use super::*;

pub(super) fn manufacturer_rows(out: &mut Out, draft: &Draft) {
    for method in as_array(draft.get("manufacturer_methods").unwrap_or(&Value::Null)) {
        out.row(&manufacturer_row(method), "");
    }
}

/// One `manufacturer KEYWORD ?-unexported? ?-names-instance-at N?
/// ?-definition-body-at N? ?-constructor-args-from N?` row — the command's
/// own `manufacturer_methods` and a definition body's `manufacturers` alike.
fn manufacturer_row(method: &Value) -> Vec<String> {
    let mut row = vec![
        "manufacturer".to_owned(),
        name_word(str_of(&method["keyword"])),
    ];
    if str_of(&method["visibility"]) == "Unexported" {
        row.push("-unexported".to_owned());
    }
    for (key, flag) in [
        ("names_instance_at", "-names-instance-at"),
        ("definition_body_at", "-definition-body-at"),
    ] {
        if let Some(n) = method[key].as_u64() {
            row.push(flag.to_owned());
            row.push(n.to_string());
        }
    }
    if let Some(n) = method["constructor_args_from"].as_u64()
        && n != 0
    {
        row.push("-constructor-args-from".to_owned());
        row.push(n.to_string());
    }
    row
}

/// `runtime_backing none|host-native|shipped-builtin ID|tcl-body {…}` — the
/// draft holds the statement's own spelling, read and re-spelled through the
/// loader's parser ([`tcl_spectcl::BackingSyntax`]) so the row is always one
/// the loader reads back; a spelling that does not read is a `TODO`, not a
/// statement the loader would drop.
pub(super) fn runtime_backing_row(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if !ctx.set(draft, "runtime_backing") {
        return;
    }
    let Some(spelling) = draft["runtime_backing"].as_str() else {
        return;
    };
    match tcl_spectcl::BackingSyntax::parse_spelling(spelling) {
        Ok(syntax) => out.line(&format!("runtime_backing {}", syntax.spelling())),
        Err(_) => out.comment(&format!(
            "TODO(spectcl): `runtime_backing {spelling}` does not read; the loader \
             takes `none`, `host-native`, `shipped-builtin ID`, `tcl-body \
             {{-package-source PATH ?-evaluate?}}` or `tcl-body {{-pack-text {{TEXT}} \
             ?-evaluate?}}`."
        )),
    }
}

/// `semantic_operation Invoke|{Intrinsic ID}|{StructuredLowering ID}` — the
/// closed vocabulary in the loader's spelling
/// ([`tcl_spectcl::semantic_operation_spelling`]).
pub(super) fn semantic_operation_row(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    let Some(value) = draft
        .get("semantic_operation")
        .filter(|value| !value.is_null())
    else {
        return;
    };
    if !ctx.set(draft, "semantic_operation") {
        return;
    }
    match semantic_operation_word(value) {
        Some(spelling) => out.line(&format!("semantic_operation {spelling}")),
        None => todo(out, "semantic_operation"),
    }
}

/// The loader's spelling of a draft semantic operation, when the closed
/// vocabulary has one.
fn semantic_operation_word(value: &Value) -> Option<String> {
    let kind = str_of(&value["kind"]);
    let detail = value["detail"].as_str();
    tcl_spectcl::semantic_operations()
        .find(|operation| operation.kind_str() == kind && operation.detail_str() == detail)
        .map(tcl_spectcl::semantic_operation_spelling)
        .and_then(|spelling| word(&spelling))
}

/// ` -introduced V -deprecated V -retired V`, for the releases a window's
/// lifecycle states.
fn lifecycle_flags(lifecycle: &Value) -> String {
    let mut flags = String::new();
    for (flag, key) in [
        ("-introduced", "introduced"),
        ("-deprecated", "deprecated"),
        ("-retired", "retired"),
    ] {
        if let Some(at) = lifecycle.get(key).and_then(Value::as_str) {
            let _ = write!(flags, " {flag} {at}");
        }
    }
    flags
}

/// `KEY -native ID -introduced V ?-deprecated V? ?-retired V?` — one row per
/// window of a catalogue hook (`SpecTcl` 2.2), after the plain row.
pub(super) fn hook_window_rows(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    let windows = format!("{key}_windows");
    if !ctx.set(draft, &windows) {
        return;
    }
    for window in as_array(&draft[windows.as_str()]) {
        if let Some(id) = window["value"].as_str() {
            out.line(&format!(
                "{key} -native {id}{}",
                lifecycle_flags(&window["lifecycle"])
            ));
        }
    }
}

/// `semantic_operation SPELLING -introduced V ?-deprecated V? ?-retired V?` —
/// one row per window of the semantic operation, after the plain row.
pub(super) fn semantic_operation_window_rows(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if !ctx.set(draft, "semantic_operation_windows") {
        return;
    }
    for window in as_array(&draft["semantic_operation_windows"]) {
        match semantic_operation_word(&window["value"]) {
            Some(spelling) => out.line(&format!(
                "semantic_operation {spelling}{}",
                lifecycle_flags(&window["lifecycle"])
            )),
            None => todo(out, "semantic_operation"),
        }
    }
}

/// `definition_body NAME` for a shipped grammar, or `definition_body { … }`
/// spelling the grammar out row by row in the loader's vocabulary: `family`,
/// one `member` row per member — each with its `-effect`, and a wrapper's
/// `-shift` — one `member_option` row per accepted optional word, and the
/// object-model rows a class family declares.
pub(super) fn definition_body_block(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    let Some(value) = draft
        .get("definition_body")
        .filter(|value| !value.is_null())
    else {
        return;
    };
    if !ctx.set(draft, "definition_body") {
        return;
    }
    if let Some(name) = value.as_str() {
        out.line(&format!("definition_body {}", name_word(name)));
        return;
    }
    if as_array(&value["builtin_object_methods"])
        .iter()
        .any(|method| !method["operation"].is_null())
    {
        out.comment(
            "TODO(spectcl): `definition_body` contains native object-method operations. \
             Only an exact shipped grammar name preserves them; this custom block is omitted.",
        );
        out.losses.push(Loss {
            key: "definition_body".to_owned(),
            reason: "custom native object-method operations cannot be authored inline".to_owned(),
        });
        return;
    }
    let mut body = Out::at(out.indent + 1);
    let mut lost = false;
    body.line(&format!("family {}", str_of(&value["family"])));
    let members = as_array(&value["members"]);
    for member in members {
        let row = member_row(member, &mut lost, ctx.availability);
        body.row(&row, "-effect");
    }
    for member in members {
        for row in member_option_rows(member, &mut lost, ctx.availability) {
            body.row(&row, "");
        }
    }
    for key in [
        "implicit_vars",
        "implicit_receiver_vars",
        "member_body_namespace_path",
        "builtin_type_methods",
    ] {
        list_row(&mut body, &mut lost, value, key);
    }
    for method in as_array(&value["builtin_object_methods"]) {
        let mut row = vec![
            "builtin_object_method".to_owned(),
            name_word(str_of(&method["name"])),
        ];
        if str_of(&method["visibility"]) == "Unexported" {
            row.push("-unexported".to_owned());
        }
        row.push("-receiver".to_owned());
        row.push(str_of(&method["receiver"]).to_owned());
        push_flag(
            &mut row,
            &mut lost,
            "-detail",
            braced(str_of(&method["detail"])),
        );
        body.row(&row, "-detail");
    }
    list_row(&mut body, &mut lost, value, "builtin_terminating_methods");
    definition_body_command_rows(&mut body, &mut lost, value);
    if value["dynamic_method_dispatch"].as_bool() == Some(true) {
        body.line("dynamic_method_dispatch");
    }
    for method in as_array(&value["manufacturers"]) {
        body.row(&manufacturer_row(method), "");
    }
    if let Some(name) = value["unknown_dispatch_method"].as_str() {
        let mut row = vec!["unknown_dispatch_method".to_owned()];
        push_word(&mut row, &mut lost, word(name));
        body.row(&row, "");
    }
    list_row(&mut body, &mut lost, value, "property_accessor_methods");
    if lost {
        unwritable(out, "definition_body");
        return;
    }
    out.line("definition_body {");
    out.block(&body);
    out.line("}");
}

fn definition_body_command_rows(body: &mut Out, lost: &mut bool, value: &Value) {
    for command in as_array(&value["member_body_commands"]) {
        let mut row = vec![
            "member_body_command".to_owned(),
            name_word(str_of(&command["name"])),
        ];
        push_flag(
            &mut row,
            lost,
            "-detail",
            braced(str_of(&command["detail"])),
        );
        if let Some(expr) = command["binds_handle"].as_str() {
            push_flag(&mut row, lost, "-binds-handle", binds_handle_word(expr));
        }
        body.row(&row, "-binds-handle");
    }
    if value["bare_word_construction"].as_bool() == Some(true) {
        let mut row = vec!["bare_word_construction".to_owned()];
        if let Some(hint) = value
            .get("bare_word_construction_source_hint")
            .filter(|hint| !hint.is_null())
        {
            for (key, flag) in [("values", "-hint-values"), ("prefixes", "-hint-prefixes")] {
                row.push(flag.to_owned());
                let values = as_array(&hint[key]);
                let rendered: Option<Vec<_>> = values
                    .iter()
                    .map(|value| value.as_str().and_then(word))
                    .collect();
                if let Some(values) = rendered {
                    row.push(format!("{{{}}}", values.join(" ")));
                } else {
                    *lost = true;
                    row.push("{}".to_owned());
                }
            }
        }
        body.row(&row, "");
    }
}

/// A `KEY {word …}` row for a non-empty name list of a definition body.
fn list_row(out: &mut Out, lost: &mut bool, grammar: &Value, key: &str) {
    if as_array(&grammar[key]).is_empty() {
        return;
    }
    let mut row = vec![key.to_owned()];
    push_word(&mut row, lost, str_list_word(&grammar[key]));
    out.row(&row, "");
}

/// One `member KEYWORD …` row: the layout flags the loader reads, then the
/// member's `-effect` and a wrapper's `-shift`.
fn member_row(member: &Value, lost: &mut bool, algebra: bool) -> Vec<String> {
    let mut row = vec!["member".to_owned(), name_word(str_of(&member["keyword"]))];
    let roles: Vec<String> = as_array(&member["arg_roles"])
        .iter()
        .flat_map(|pair| {
            [
                pair["index"].as_u64().unwrap_or_default().to_string(),
                str_of(&pair["role"]).to_owned(),
            ]
        })
        .collect();
    if !roles.is_empty() {
        push_flag(&mut row, lost, "-roles", list_word(&roles));
    }
    if member["all_args_var"].as_bool() == Some(true) {
        row.push("-all-vars".to_owned());
    }
    if let Some(kind) = member["all_args_ref"].as_str() {
        row.push("-all-refs".to_owned());
        row.push(kind.to_owned());
    }
    let kind = str_of(&member["kind"]);
    if kind != "Flat" {
        row.push("-kind".to_owned());
        row.push(kind.to_owned());
    }
    if member["wrapper_block_body"].as_bool() == Some(true) {
        row.push("-block-body".to_owned());
    }
    if !member["surface"].is_null() {
        push_availability_flag(&mut row, lost, &member["surface"], algebra);
    }
    if let Some(retraction) = member["retraction"].as_str() {
        row.push("-retracts".to_owned());
        row.push(retraction.to_owned());
    }
    if let Some(slot) = member["slot"].as_object() {
        row.push("-slot".to_owned());
        row.push(str_of(&slot["default_op"]).to_owned());
        if slot.get("dedup").and_then(Value::as_bool) == Some(true) {
            row.push("-dedup".to_owned());
        }
    }
    if let Some(visibility) = member["visibility_effect"].as_str() {
        row.push("-visibility".to_owned());
        row.push(visibility.to_owned());
    }
    push_flag(
        &mut row,
        lost,
        "-effect",
        effect_word(&member["effect"], &member["arg_roles"]),
    );
    if let Some(shift) = member["wrapper_shift"].as_object() {
        let mut words = Vec::new();
        if let Some(receiver) = shift.get("receiver").and_then(Value::as_str) {
            words.extend(["-receiver".to_owned(), receiver.to_owned()]);
        }
        if let Some(visibility) = shift.get("visibility").and_then(Value::as_str) {
            words.extend(["-visibility".to_owned(), visibility.to_owned()]);
        }
        push_flag(&mut row, lost, "-shift", braced(&words.join(" ")));
    }
    row
}

/// A member's `-effect` value in the loader's spelling. A slot the loader
/// would position from the row's roles is left unwritten; a slot that differs
/// is written; `None` when the effect leaves a slot empty that the roles would
/// fill, which the loader has no spelling for.
fn effect_word(effect: &Value, roles: &Value) -> Option<String> {
    let first = |wanted: &[&str]| {
        wanted.iter().find_map(|role| {
            as_array(roles)
                .iter()
                .find(|pair| str_of(&pair["role"]) == *role)
                .and_then(|pair| pair["index"].as_u64())
        })
    };
    let mut words = vec![str_of(&effect["kind"]).to_owned()];
    let mut slot = |flag: &str, key: &str, derived: Option<u64>| -> Option<()> {
        match (effect[key].as_u64(), derived) {
            (Some(written), Some(derived)) if written == derived => {}
            (Some(written), _) => words.extend([flag.to_owned(), written.to_string()]),
            (None, None) => {}
            (None, Some(_)) => return None,
        }
        Some(())
    };
    match str_of(&effect["kind"]) {
        "callable" => {
            let (receiver, role) = (str_of(&effect["receiver"]), str_of(&effect["role"]));
            slot("-name", "name_slot", first(&["Name"]))?;
            slot("-params", "params_slot", first(&["ParamList"]))?;
            slot("-body", "body_slot", first(&["Body"]))?;
            let mut head = vec![
                "callable".to_owned(),
                "-receiver".to_owned(),
                receiver.to_owned(),
                "-role".to_owned(),
                role.to_owned(),
            ];
            head.extend(words.drain(1..));
            words = head;
        }
        "forward" => {
            slot("-name", "name_slot", first(&["Name"]))?;
            slot(
                "-prefix",
                "prefix_slot",
                first(&["CommandName", "CommandPrefix"]),
            )?;
        }
        "init-script" => {
            slot("-body", "body_slot", first(&["Body"]))?;
            words.extend(["-timing".to_owned(), str_of(&effect["timing"]).to_owned()]);
        }
        "state-declaration" => words.push(str_of(&effect["scope"]).to_owned()),
        "relation" => words.push(str_of(&effect["slot"]).to_owned()),
        _ => {}
    }
    list_word(&words)
}

/// The `member_option KEYWORD POSITION VALUE -role ROLE ?-visibility V?
/// ?-available V?` rows of one member's optional word, one per spelling.
fn member_option_rows(member: &Value, lost: &mut bool, algebra: bool) -> Vec<Vec<String>> {
    let Some(optional) = member["optional_argument"].as_object() else {
        return Vec::new();
    };
    let position = optional
        .get("position")
        .and_then(Value::as_u64)
        .unwrap_or_default()
        .to_string();
    as_array(optional.get("values").unwrap_or(&Value::Null))
        .iter()
        .map(|value| {
            let mut row = vec![
                "member_option".to_owned(),
                name_word(str_of(&member["keyword"])),
                position.clone(),
            ];
            push_word(&mut row, lost, word(str_of(&value["value"])));
            row.push("-role".to_owned());
            row.push(str_of(&value["role"]).to_owned());
            if let Some(visibility) = value["declared_visibility"].as_str() {
                row.push("-visibility".to_owned());
                row.push(visibility.to_owned());
            }
            if !value["surface"].is_null() {
                push_availability_flag(&mut row, lost, &value["surface"], algebra);
            }
            row
        })
        .collect()
}

/// `clause_grammar { … } ?-available V?` — the clause-grammar descriptor, one
/// row per statement, in the loader's spelling (`clause_grammar_value` in
/// `tcl-spectcl`).
///
/// A row writes only what differs from the loader's reading of a bare row:
/// `-timing` unless `selected`, `-pattern` for a handler slot, `-conditional`
/// for a conditionally bound binder, `?KEYWORD?` for an optional keyword, and
/// `-available` for a row's own releases. An empty head — the loops whose
/// first words are a group — is no row at all.
pub(super) fn clause_grammar_block(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    let Some(grammar) = draft.get("clause_grammar").filter(|value| !value.is_null()) else {
        return;
    };
    if !ctx.set(draft, "clause_grammar") {
        return;
    }
    let mut body = Out::at(out.indent + 1);
    let mut lost = false;
    let head = &grammar["head"];
    let empty_head = as_array(&head["slots"]).is_empty()
        && str_of(&head["timing"]) == "selected"
        && head["surface"].is_null();
    if !empty_head {
        clause_row(&mut body, &mut lost, ctx, "head", head);
    }
    for row in as_array(&grammar["rows"]) {
        let statement = match str_of(&row["shape"]) {
            "repeated" => "repeated",
            "group" => "group",
            _ => "once",
        };
        clause_row(&mut body, &mut lost, ctx, statement, row);
    }
    if !grammar["tail"].is_null() {
        clause_row(&mut body, &mut lost, ctx, "tail", &grammar["tail"]);
    }
    if let Some(marker) = grammar["fallthrough_body"].as_str() {
        let mut row = vec!["fallthrough_body".to_owned()];
        push_word(&mut row, &mut lost, word(marker));
        body.row(&row, "");
    }
    if let Some(default) = grammar["default_clause"].as_object() {
        let mut row = vec![
            "default_clause".to_owned(),
            default
                .get("row")
                .and_then(Value::as_u64)
                .map_or_else(|| "tail".to_owned(), |row| row.to_string()),
        ];
        if default.get("final_only").and_then(Value::as_bool) == Some(true) {
            row.push("-final-only".to_owned());
        }
        body.row(&row, "");
    }
    if str_of(&grammar["selection"]) != "first-match" {
        body.line(&format!("selection {}", str_of(&grammar["selection"])));
    }
    let mut closing = vec!["}".to_owned()];
    if !grammar["surface"].is_null() {
        push_availability_flag(
            &mut closing,
            &mut lost,
            &grammar["surface"],
            ctx.availability,
        );
    }
    if lost {
        unwritable(out, "clause_grammar");
        return;
    }
    out.gap();
    out.line("clause_grammar {");
    out.block(&body);
    out.row(&closing, "");
}

/// One `head` / `repeated` / `once` / `group` / `tail` row.
fn clause_row(out: &mut Out, lost: &mut bool, ctx: &Ctx<'_>, statement: &str, row: &Value) {
    let mut words = vec![statement.to_owned()];
    if statement == "group" {
        words.push(row["layout"].as_u64().unwrap_or_default().to_string());
    } else {
        if let Some(keyword) = row["keyword"].as_str() {
            let spelling = if row["keyword_required"].as_bool() == Some(false) {
                format!("?{keyword}?")
            } else {
                keyword.to_owned()
            };
            push_word(&mut words, lost, word(&spelling));
        }
        let slots: Vec<String> = as_array(&row["slots"])
            .iter()
            .map(|slot| {
                if let Some(noise) = slot["noise"].as_str() {
                    format!("?{noise}?")
                } else if slot["optional"].as_bool() == Some(true) {
                    format!("{} optional", str_of(&slot["role"]))
                } else {
                    str_of(&slot["role"]).to_owned()
                }
            })
            .collect();
        push_word(
            &mut words,
            lost,
            if slots.is_empty() {
                Some("{}".to_owned())
            } else {
                list_word(&slots)
            },
        );
    }
    let timing = str_of(&row["timing"]);
    if timing != "selected" {
        words.push("-timing".to_owned());
        words.push(timing.to_owned());
    }
    let slots = as_array(&row["slots"]);
    if let Some(handler) = slots.iter().find_map(|slot| slot["handler"].as_str()) {
        words.push("-pattern".to_owned());
        words.push(handler.to_owned());
    }
    if slots
        .iter()
        .any(|slot| slot["conditional_binding"].as_bool() == Some(true))
    {
        words.push("-conditional".to_owned());
    }
    if !row["surface"].is_null() {
        push_availability_flag(&mut words, lost, &row["surface"], ctx.availability);
    }
    out.row(&words, "-timing");
}

/// `object_class NAME ?-superclass {…}? ?-allow-unknown?`
/// `?-method-prefix-matching Enabled|Strict? ?{ method … }?`.
///
/// The ratified spelling puts the class's three one-word facts on the
/// statement and only the method table in the block, so a class with no
/// methods of its own — `SpiceGenTcl`'s `R`, an empty subclass that inherits
/// everything — is one row. The `method` rows are ordinary subcommand bodies:
/// [`ObjectClassSpec::instance_methods`](tcl_registry::spec::ObjectClassSpec::instance_methods)
/// is `&[SubCommand]`, so the same emitter writes both.
pub(super) fn object_class_block(out: &mut Out, ctx: &mut Ctx<'_>, draft: &Draft) {
    let Some(class) = draft.get("object_class").filter(|value| !value.is_null()) else {
        return;
    };
    let mut row = vec!["object_class".to_owned()];
    let mut lost = false;
    push_word(&mut row, &mut lost, word(str_of(&class["class_name"])));
    let superclasses: Vec<&str> = as_array(&class["superclasses"])
        .iter()
        .filter_map(Value::as_str)
        .collect();
    if !superclasses.is_empty() {
        push_flag(&mut row, &mut lost, "-superclass", list_word(&superclasses));
    }
    if class["allow_unknown_methods"].as_bool() == Some(true) {
        row.push("-allow-unknown".to_owned());
    }
    if str_of(&class["method_prefix_matching"]) == "Enabled" {
        row.push("-method-prefix-matching".to_owned());
        row.push("Enabled".to_owned());
    }
    let methods = as_array(&class["instance_methods"]);
    if lost {
        unwritable(out, "object_class");
        return;
    }
    out.gap();
    if methods.is_empty() {
        out.row(&row, "-superclass");
        return;
    }

    let mut scope = Ctx {
        defaults: ctx.defaults,
        scope: tcl_spectcl::loader::hook_path(&ctx.scope, str_of(&class["class_name"])),
        owner: ctx.owner.clone(),
        tables: ctx.tables,
        availability: ctx.availability,
        resolver_capabilities: ctx.resolver_capabilities,
        vocabulary: ctx.vocabulary,
    };
    let mut body = Out::at(out.indent + 1);
    for method in methods {
        let Some(method) = method.as_object() else {
            continue;
        };
        subcommand_block(&mut body, &mut scope, method, "method");
    }
    row.push("{".to_owned());
    out.row(&row, "-superclass");
    out.block(&body);
    out.line("}");
}
