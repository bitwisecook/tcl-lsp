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

use super::diagram_data_for_dialect;
use serde_json::{Value, json};
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn loop_nodes_carry_their_compiler_exit_reason() {
    let source = r"
            proc loops {} {
                while {$ready} { set seen [clock seconds] }
                for {set i 0} {$i < 1} {incr i} { set seen [clock seconds] }
                foreach item $items { set seen [clock seconds] }
            }
        ";
    let data = diagram_data_for_dialect(
        source,
        tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let flow = data
        .pointer("/procedures/0/flow")
        .and_then(serde_json::Value::as_array)
        .expect("procedure flow");
    let exits: Vec<_> = flow
        .iter()
        .map(|node| node.get("exit").and_then(serde_json::Value::as_str))
        .collect();
    assert_eq!(exits, vec![Some("false"), Some("false"), Some("exhausted")]);
}

#[test]
fn try_projection_carries_exact_completions_and_handler_fallthrough() {
    let data = diagram_data_for_dialect(
        r"
                proc paths {} {
                    try { error boom } on error message - on return value {
                        set recovered [clock seconds]
                    } finally {
                        set cleaned [clock seconds]
                    }
                    set after [clock seconds]
                }
            ",
        tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let try_node = data.pointer("/procedures/0/flow/0").expect("try node");
    assert_eq!(try_node["kind"], "try");
    assert_eq!(try_node["body"][0]["completion"], "error");
    assert_eq!(try_node["handlers"][0]["fallthrough"], true);
    assert_eq!(try_node["handlers"][1]["fallthrough"], false);
    assert_eq!(try_node["finally"][0]["kind"], "assign");
    assert_eq!(try_node["finally"][0].get("completion"), None);

    // The following action remains in the projection; it is the renderer
    // that selects it only for the normal/handled completion paths.
    let flow = data
        .pointer("/procedures/0/flow")
        .and_then(Value::as_array)
        .unwrap();
    assert_eq!(flow[1]["value"], "[clock seconds]");
}

#[test]
fn return_projection_is_an_explicit_completion_not_a_renderer_guess() {
    let data = diagram_data_for_dialect(
        "proc paths {} { try { return stop } finally { set cleaned [clock seconds] } }",
        tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert_eq!(
        data.pointer("/procedures/0/flow/0/body/0/completion"),
        Some(&Value::String("return".to_owned()))
    );

    let overridden = diagram_data_for_dialect(
        "proc paths {} { try { set done [clock seconds] } finally { return stop } }",
        tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert_eq!(
        overridden.pointer("/procedures/0/flow/0/finally/0/completion"),
        Some(&Value::String("return".to_owned()))
    );
}

#[test]
fn literal_return_options_and_exit_keep_distinct_try_outcomes() {
    let data = diagram_data_for_dialect(
        r"
                proc paths {} {
                    try { return -code error payload } finally { set cleaned [clock seconds] }
                    try { return -level 0 -code error payload } finally { set cleaned [clock seconds] }
                    try { exit 0 } finally { set never [clock seconds] }
                }
            ",
        tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    // Tcl's default `return -level 1` is itself TCL_RETURN inside the
    // enclosing try; only level 0 exposes the configured error code.
    assert_eq!(
        data.pointer("/procedures/0/flow/0/body/0/completion"),
        Some(&Value::String("return".to_owned()))
    );
    assert_eq!(
        data.pointer("/procedures/0/flow/1/body/0/completion"),
        Some(&Value::String("error".to_owned()))
    );
    assert_eq!(
        data.pointer("/procedures/0/flow/2/body/0/completion"),
        Some(&Value::String("process_exit".to_owned()))
    );
}

#[test]
fn dynamic_return_forms_keep_their_distinct_possible_completions() {
    let data = diagram_data_for_dialect(
        r"
                proc paths {} {
                    try { return -code $code payload } finally { set cleaned yes }
                    try { return -level 0 -code $code payload } finally { set cleaned yes }
                    try { return -options $options payload } finally { set cleaned yes }
                }
            ",
        tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert_eq!(
        data.pointer("/procedures/0/flow/0/body/0/completion"),
        Some(&Value::String("dynamic_return_or_error".to_owned()))
    );
    assert_eq!(
        data.pointer("/procedures/0/flow/1/body/0/completion"),
        Some(&Value::String("dynamic".to_owned()))
    );
    assert_eq!(
        data.pointer("/procedures/0/flow/2/body/0/completion"),
        Some(&Value::String("dynamic".to_owned()))
    );
}

#[test]
fn exit_statuses_project_release_specific_errors_and_typed_dynamic_outcomes() {
    let source = r"
            proc paths {status} {
                try { exit 4294967296 } on error {message options} { set caught [clock seconds] } finally { set cleaned [clock seconds] }
                try { exit $status } on error {message options} { set caught_dynamic [clock seconds] } finally { set cleaned_dynamic [clock seconds] }
                set after [clock seconds]
            }
        ";
    let tcl86 = diagram_data_for_dialect(
        source,
        tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert_eq!(
        tcl86.pointer("/procedures/0/flow/0/body/0/completion"),
        Some(&Value::String("error".to_owned())),
        "Tcl 8.6 rejects the first value above Tcl_GetIntFromObj's UINT_MAX ceiling: {tcl86}"
    );
    assert_eq!(
        tcl86.pointer("/procedures/0/flow/0/handlers/0/body/0/kind"),
        Some(&Value::String("assign".to_owned())),
        "Tcl 8.6's static range error reaches on-error: {tcl86}"
    );
    assert_eq!(
        tcl86.pointer("/procedures/0/flow/0/finally/0/kind"),
        Some(&Value::String("assign".to_owned())),
        "Tcl 8.6's static range error reaches finally: {tcl86}"
    );
    assert_eq!(
        tcl86.pointer("/procedures/0/flow/2/kind"),
        Some(&Value::String("assign".to_owned())),
        "Tcl 8.6's handled error preserves the after path: {tcl86}"
    );

    let tcl90 = diagram_data_for_dialect(
        source,
        tcl_registry::model::ingress::static_context_for("tcl9.0").commands(),
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
    );
    assert_eq!(
        tcl90.pointer("/procedures/0/flow/0/body/0/completion"),
        Some(&Value::String("process_exit".to_owned())),
        "Tcl 9.0's bit-preserving wide conversion accepts 4294967296: {tcl90}"
    );
    for (dialect, data) in [("tcl8.6", &tcl86), ("tcl9.0", &tcl90)] {
        assert_eq!(
            data.pointer("/procedures/0/flow/1/body/0/completion"),
            Some(&Value::String("exit_or_error".to_owned())),
            "{dialect}: dynamic exit has no ordinary completion"
        );
    }
}

#[test]
fn literal_invalid_return_forms_project_as_catchable_errors() {
    let source = r"
            proc paths {} {
                try { return -code bogus payload } on error {m o} {}
                try { return -level -1 payload } on error {m o} {}
            }
        ";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let data = diagram_data_for_dialect(
            source,
            tcl_registry::model::ingress::static_context_for_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            )
            .commands(),
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        );
        for index in 0..2 {
            assert_eq!(
                data.pointer(&format!("/procedures/0/flow/{index}/body/0/completion")),
                Some(&Value::String("error".to_owned())),
                "{dialect}: {data}"
            );
        }
    }
}

#[test]
fn trailing_return_option_words_are_results_not_missing_values() {
    let source = r"
            proc paths {} {
                try { return -code } on return {m o} {}
                try { return -level } on return {m o} {}
                try { return -options } on return {m o} {}
            }
        ";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let data = diagram_data_for_dialect(
            source,
            tcl_registry::model::ingress::static_context_for_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            )
            .commands(),
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        );
        for index in 0..3 {
            assert_eq!(
                data.pointer(&format!("/procedures/0/flow/{index}/body/0/completion")),
                Some(&Value::String("return".to_owned())),
                "{dialect}: {data}"
            );
        }
    }
}

#[test]
fn invalid_known_terminator_arity_projects_as_catchable_error() {
    let source = r"
            proc paths {} {
                try { error } on error {m o} { set caught_error yes }
                try { throw TYPE } on error {m o} { set caught_throw yes }
                try { break extra } on error {m o} { set caught_break yes }
                try { exit 0 extra } on error {m o} { set caught_exit yes }
            }
        ";
    for dialect in ["tcl8.6", "tcl9.0"] {
        let data = diagram_data_for_dialect(
            source,
            tcl_registry::model::ingress::static_context_for_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            )
            .commands(),
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        );
        for index in 0..4 {
            assert_eq!(
                data.pointer(&format!("/procedures/0/flow/{index}/body/0/completion")),
                Some(&Value::String("error".to_owned())),
                "{dialect}: {data}"
            );
        }
    }
}

#[test]
fn return_completion_preserves_source_word_shape() {
    let source = r#"
            proc paths {} {
                try { return -code {$code} } on error {m o} {}
                try { return -code $code } on error {m o} {}
                try { return -code "$code" } on error {m o} {}
                try { return -code \x32 } on return {m o} {}
                try { return -code {\x32} } on error {m o} {}
            }
        "#;
    for dialect in ["tcl8.6", "tcl9.0"] {
        let data = diagram_data_for_dialect(
            source,
            tcl_registry::model::ingress::static_context_for_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            )
            .commands(),
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        );
        let completions = [
            "error",
            "dynamic_return_or_error",
            "dynamic_return_or_error",
            "return",
            "error",
        ];
        for (index, expected) in completions.into_iter().enumerate() {
            assert_eq!(
                data.pointer(&format!("/procedures/0/flow/{index}/body/0/completion")),
                Some(&Value::String(expected.to_owned())),
                "{dialect}: {data}"
            );
        }
    }
}

#[test]
fn numeric_return_codes_project_as_their_canonical_completions() {
    let data = diagram_data_for_dialect(
        "proc paths {} { try { return -level 0 -code 00 x } finally {} ; try { return -level 0 -code +1 x } finally {} ; try { return -level 0 -code 02 x } finally {} ; try { return -level 0 -code 0x3 x } finally {} ; try { return -level 0 -code 04 x } finally {} }",
        tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let expected = [
        None,
        Some("error"),
        Some("return"),
        Some("break"),
        Some("continue"),
    ];
    for (index, completion) in expected.into_iter().enumerate() {
        assert_eq!(
            data.pointer(&format!("/procedures/0/flow/{index}/body/0/completion")),
            completion
                .map(|value| Value::String(value.to_owned()))
                .as_ref(),
            "flow {index}"
        );
    }
}

#[test]
fn exact_custom_return_codes_keep_their_integer_payload() {
    let data = diagram_data_for_dialect(
        "proc paths {} { try { return -level 0 -code 42 x } finally {} ; try { return -level 0 -code 4294967295 x } finally {} }",
        tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert_eq!(
        data.pointer("/procedures/0/flow/0/body/0/completion"),
        Some(&Value::String("custom".to_owned()))
    );
    assert_eq!(
        data.pointer("/procedures/0/flow/0/body/0/completion_code"),
        Some(&Value::from(42))
    );
    assert_eq!(
        data.pointer("/procedures/0/flow/1/body/0/completion_code"),
        Some(&Value::from(-1))
    );
}

#[test]
fn handler_completion_codes_use_the_dialect_numeric_grammar() {
    let source = "proc paths {} { try { return -options $options payload } on 02 {message options} { set two yes } on +2 {message options} { set duplicate yes } on 0x2 {message options} { set duplicate_hex yes } on return {message options} { set symbolic yes } on 2147483648 {message options} { set wrapped_min yes } on 4294967295 {message options} { set wrapped_minus_one yes } on -2147483649 {message options} { set invalid_low yes } on nonsense {message options} { set invalid yes } }";
    // `try` itself begins in Tcl 8.6, so diagram handlers can only be
    // projected for releases that provide the command.
    for dialect in ["tcl8.6", "tcl9.0"] {
        let data = diagram_data_for_dialect(
            source,
            tcl_registry::model::ingress::static_context_for_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            )
            .commands(),
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        );
        let handlers = data
            .pointer("/procedures/0/flow/0/handlers")
            .and_then(Value::as_array)
            .expect("try handlers");
        let codes: Vec<_> = handlers
            .iter()
            .map(|handler| {
                handler
                    .get("completion_code")
                    .cloned()
                    .unwrap_or(Value::Null)
            })
            .collect();
        assert_eq!(
            codes,
            vec![
                Value::from(2),
                Value::from(2),
                Value::from(2),
                Value::from(2),
                Value::from(i32::MIN),
                Value::from(-1),
                Value::Null,
                Value::Null
            ],
            "{dialect}"
        );
    }
}

#[test]
fn trap_patterns_are_projected_as_tcl_list_elements() {
    let data = diagram_data_for_dialect(
        r#"proc paths {} { try { error x } trap {A \$B} {message options} {} trap {A {$B}} {message options} {} trap {A \[B\]} {message options} {} trap {A {[B]}} {message options} {} trap {A C:\\tmp} {message options} {} trap {A {C:\tmp}} {message options} {} trap "A \{" {message options} {} trap $pattern {message options} {} trap "A $pattern" {message options} {} trap [list A B] {message options} {} }"#,
        tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let expected = [
        json!(["A", "$B"]),
        json!(["A", "$B"]),
        json!(["A", "[B]"]),
        json!(["A", "[B]"]),
        json!(["A", r"C:\tmp"]),
        json!(["A", r"C:\tmp"]),
        Value::Null,
        Value::Null,
        Value::Null,
        Value::Null,
    ];
    for (index, expected) in expected.iter().enumerate() {
        assert_eq!(
            data.pointer(&format!(
                "/procedures/0/flow/0/handlers/{index}/trap_pattern"
            )),
            Some(expected),
            "handler {index}: {data}"
        );
    }
}

#[test]
fn trap_pattern_projection_preserves_the_dialect_escape_value() {
    let source = r#"proc paths {} { try { error x } trap "A \U0001F600" {message options} {} }"#;
    for (dialect, expected) in [("tcl8.6", "\u{fffd}"), ("tcl9.0", "😀")] {
        let data = diagram_data_for_dialect(
            source,
            tcl_registry::model::ingress::static_context_for_profile(
                tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            )
            .commands(),
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        );
        assert_eq!(
            data.pointer("/procedures/0/flow/0/handlers/0/trap_pattern"),
            Some(&json!(["A", expected])),
            "{dialect}: {data}",
        );
    }
}

#[test]
fn handler_number_release_vectors_share_tcl_numeric_owner() {
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0"] {
        let profile = tcl_registry::model::ingress::static_context_for_profile(
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
        )
        .commands()
        .profile()
        .expect("dialect profile");
        let numbers = tcl_syntax::number::Numbers::of_profile(Some(profile));
        assert_eq!(numbers.parse_wide("02"), Some(2), "{dialect}");
        assert_eq!(numbers.parse_wide("+2"), Some(2), "{dialect}");
        assert_eq!(numbers.parse_wide("0x2"), Some(2), "{dialect}");
        assert_eq!(
            tcl_registry::completion::canonical_completion_code("2147483648", numbers),
            Some(i32::MIN),
            "{dialect}"
        );
        assert_eq!(
            tcl_registry::completion::canonical_completion_code("4294967295", numbers),
            Some(-1),
            "{dialect}"
        );
        assert_eq!(
            tcl_registry::completion::canonical_completion_code("-2147483649", numbers),
            None,
            "{dialect}"
        );
        assert_eq!(
            tcl_registry::completion::canonical_completion_code("9223372036854775808", numbers),
            None,
            "{dialect}"
        );
    }
}

#[test]
fn procedure_calls_resolve_qualified_names_in_the_callers_namespace() {
    let data = diagram_data_for_dialect(
        r"
                namespace eval ::alpha {
                    proc helper {} {}
                    proc caller {} {
                        helper
                        ::alpha::helper
                    }
                }
                namespace eval ::beta {
                    proc helper {} {}
                    proc caller {} {
                        helper
                        alpha::helper
                        ::alpha::helper
                    }
                }
                namespace eval ::unrelated {
                    proc caller {} { helper }
                }
            ",
        tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );

    let procedures = data["procedures"].as_array().expect("procedures");
    assert_eq!(procedures.len(), 5, "{data}");
    assert_eq!(procedures[1]["name"], "caller");
    assert_eq!(procedures[3]["name"], "caller");
    assert_eq!(procedures[4]["name"], "caller");

    for index in [0, 1] {
        assert_eq!(procedures[1]["flow"][index]["kind"], "proc_call");
    }
    // A relative qualified command first probes `::beta::alpha::helper`,
    // then correctly falls back to `::alpha::helper`; rooted spelling is
    // a direct lookup of that same declaration.
    for index in [0, 1, 2] {
        assert_eq!(procedures[3]["flow"][index]["kind"], "proc_call");
    }
    assert!(
        procedures[4]["flow"].as_array().is_some_and(Vec::is_empty),
        "a same-tailed procedure in another namespace must not match: {data}"
    );
}

#[test]
fn registry_actions_resolve_relative_names_in_the_callers_namespace() {
    let data = diagram_data_for_dialect(
        "proc ::HTTP::caller {} { respond 200 content ok }",
        tcl_registry::model::ingress::static_context_for("f5-irules").commands(),
        tcl_dialect::DialectProfile::irules(),
    );
    let flow = data
        .pointer("/procedures/0/flow")
        .and_then(Value::as_array)
        .expect("caller flow");
    assert_eq!(flow.len(), 1, "{data}");
    assert_eq!(flow[0]["kind"], "action");
    assert_eq!(flow[0]["command"], "respond");
}

#[test]
fn procedure_aliases_and_rebound_registry_heads_stay_source_sensitive() {
    let data = diagram_data_for_dialect(
        r"
                proc ::library::helper {} {}
                interp alias {} invoke {} ::library::helper
                rename error saved_error
                proc error {args} {}
                proc caller {} {
                    invoke
                    error user
                    saved_error original
                }
            ",
        tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );

    let flow = data
        .pointer("/procedures/2/flow")
        .and_then(Value::as_array)
        .expect("caller flow");
    assert_eq!(flow.len(), 3, "{data}");
    assert_eq!(flow[0]["kind"], "proc_call");
    assert_eq!(flow[0]["command"], "invoke");
    assert_eq!(flow[1]["kind"], "proc_call");
    assert_eq!(flow[1]["command"], "error");
    assert_eq!(flow[2]["kind"], "action");
    assert_eq!(flow[2]["command"], "saved_error");
    assert_eq!(flow[2]["completion"], "error");
}

#[test]
fn real_tcl_oracle_distinguishes_default_and_level_zero_return_codes() {
    let script = r"
            proc probe {spec} {
                set log {}
                try { return {*}$spec payload } \
                    on error {message options} { lappend log error } \
                    on return {message options} { lappend log return } \
                    finally { lappend log finally }
                lappend log after
                return $log
            }
            proc probe_dynamic_code {code} {
                set log {}
                try { return -code $code payload } \
                    on ok {message options} { lappend log ok } \
                    on error {message options} { lappend log error } \
                    on return {message options} { lappend log return } \
                    on break {message options} { lappend log break } \
                    on continue {message options} { lappend log continue } \
                    finally { lappend log finally }
                lappend log after
                return $log
            }
            proc probe_dynamic_options {options} {
                set log {}
                try { return -options $options payload } \
                    on ok {message options} { lappend log ok } \
                    on error {message options} { lappend log error } \
                    on return {message options} { lappend log return } \
                    on break {message options} { lappend log break } \
                    on continue {message options} { lappend log continue } \
                    finally { lappend log finally }
                lappend log after
                return $log
            }
            proc probe_source_order {} {
                set log {}
                try { return -options {-code error -level 0 -errorcode {X Y}} payload } \
                    trap {X} {message options} { lappend log broad-trap } \
                    trap {X Y} {message options} { lappend log overlapping-trap } \
                    on error {message options} { lappend log first-error } \
                    on error {message options} { lappend log duplicate-error } \
                    finally { lappend log finally }
                return $log
            }
            proc probe_custom_code {code} {
                set log {}
                try { return -options [list -code $code -level 0] payload } \
                    on 42 {message options} { lappend log first-42 } \
                    on 42 {message options} { lappend log duplicate-42 } \
                    on 43 {message options} { lappend log code-43 } \
                    on -1 {message options} { lappend log minus-one } \
                    on error {message options} { lappend log symbolic-error } \
                    finally { lappend log finally }
                return $log
            }
            puts [probe {-code error}]
            puts [probe {-level 0 -code error}]
            puts [probe {-foo bar}]
            puts [probe {-c error}]
            puts [probe {-level 0x0 -code error}]
            puts [probe_dynamic_code error]
            puts [probe_dynamic_code bogus]
            foreach options {
                {-code ok -level 0}
                {-code error -level 0}
                {-code return -level 0}
                {-code break -level 0}
                {-code continue -level 0}
            } { puts [probe_dynamic_options $options] }
            puts [probe_source_order]
            puts [probe_custom_code 42]
            puts [probe_custom_code 43]
            puts [probe_custom_code 4294967295]
            puts [probe_custom_code error]
        ";
    let mut child = Command::new("tclsh")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("tclsh oracle available");
    child
        .stdin
        .as_mut()
        .expect("oracle stdin")
        .write_all(script.as_bytes())
        .expect("write oracle script");
    let output = child.wait_with_output().expect("wait for tclsh oracle");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).expect("oracle stdout"),
        concat!(
            "return finally after\nerror finally after\nreturn finally after\n",
            "return finally after\nerror finally after\n",
            "return finally after\nerror finally after\n",
            "ok finally after\nerror finally after\nreturn finally after\n",
            "break finally after\ncontinue finally after\n",
            "broad-trap finally\n",
            "first-42 finally\ncode-43 finally\nminus-one finally\nsymbolic-error finally\n",
        )
    );
}
