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

//! Script timing of arguments and options, callback scripts and callback taint inputs.

use super::*;

#[test]
fn script_timing_is_exact_for_mixed_immediate_and_deferred_arguments() {
    use crate::hover::{OptionSpec, OptionValue, ScriptTiming};

    static OPTIONS: &[OptionSpec] = &[OptionSpec {
        name: "-later",
        value: OptionValue::deferred_script(),
        detail: "stored callback",
        ..OptionSpec::DEFAULT
    }];
    let mut reg = CommandRegistry::build_default();
    reg.insert(CommandSpec {
        name: "mixed-script-timing",
        arity: crate::Arity::exact(3),
        arg_roles: &[(0, ArgRole::Body)],
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    });

    let args = ["{error now}", "-later", "{error later}"];
    assert_eq!(
        reg.script_timing("mixed-script-timing", &args, 0, None),
        Some(ScriptTiming::SameInvocation),
    );
    assert_eq!(
        reg.script_timing("mixed-script-timing", &args, 2, None),
        Some(ScriptTiming::Deferred),
    );
    assert_eq!(
        reg.script_timing("mixed-script-timing", &args, 1, None),
        None,
    );
}

#[test]
fn shipped_script_options_distinguish_tk_callbacks_from_tcltest_bodies() {
    use crate::hover::ScriptTiming;

    let reg = callback_metadata_registry();
    let button = [".b", "-command", "{return}"];
    assert_eq!(
        reg.script_timing("button", &button, 2, None),
        Some(ScriptTiming::Deferred),
    );

    let test = ["sample", "description", "-body", "{error failure}"];
    assert_eq!(
        reg.script_timing("tcltest::test", &test, 3, None),
        Some(ScriptTiming::SameInvocation),
    );

    let dump = ["dump", "-command", "visit", "1.0"];
    assert_eq!(
        reg.instance_script_timing("text", ".receiver", &dump, 2, None),
        Some(ScriptTiming::SameInvocation),
        "text dump calls its prefix synchronously for each returned item"
    );
    let sync = ["sync", "-command", "ready"];
    assert_eq!(
        reg.instance_script_timing("text", ".receiver", &sync, 2, None),
        Some(ScriptTiming::Deferred),
        "text sync schedules its prefix after metrics become current"
    );
}

#[test]
fn callback_taint_inputs_are_exact_and_exclude_tk_bookkeeping() {
    use crate::CallbackTaintInput;

    let reg = callback_metadata_registry();
    let validation = [".entry", "-validatecommand", "{eval %P}"];
    assert_eq!(
        reg.callback_taint_inputs("entry", &validation, 2, None),
        &[
            CallbackTaintInput::TK_PROPOSED_VALUE,
            CallbackTaintInput::TK_CURRENT_VALUE,
            CallbackTaintInput::TK_EDIT_TEXT,
        ],
    );
    let bind = [".entry", "<Key>", "{eval %A}"];
    assert_eq!(
        reg.callback_taint_inputs("bind", &bind, 2, None),
        &[CallbackTaintInput::TK_EVENT_CHAR],
    );
    let plain_command = [".entry", "-command", "{eval %P}"];
    assert!(
        reg.callback_taint_inputs("button", &plain_command, 2, None)
            .is_empty(),
        "a deferred callback without user substitutions must stay clean"
    );

    let canvas_bind = ["bi", "item", "<Key>", "{eval %A}"];
    assert_eq!(
        reg.instance_callback_taint_inputs("canvas", ".canvas", &canvas_bind, 3, None),
        &[CallbackTaintInput::TK_EVENT_CHAR],
        "a unique-prefix widget subcommand must select its callback table"
    );

    let abbreviated_configure = ["conf", "-validatecommand", "{eval %P}"];
    assert_eq!(
        reg.instance_callback_taint_inputs("entry", ".entry", &abbreviated_configure, 2, None,),
        &[
            CallbackTaintInput::TK_PROPOSED_VALUE,
            CallbackTaintInput::TK_CURRENT_VALUE,
            CallbackTaintInput::TK_EDIT_TEXT,
        ],
        "a unique-prefix widget method must inherit constructor callback options"
    );

    let instance_bind = ["bind", "all", "<Key>", "{eval %A}"];
    assert_eq!(
        reg.instance_callback_taint_inputs("canvas", ".canvas", &instance_bind, 3, None,),
        &[CallbackTaintInput::TK_EVENT_CHAR],
        "a positional instance-method callback must use its method table"
    );
    assert!(
        reg.instance_callback_taint_inputs(
            "entry",
            ".entry",
            &["configure", "-validatecommand"],
            1,
            None,
        )
        .is_empty(),
        "the option word itself is not callback text"
    );
}

/// Classic `entry` and `spinbox` share `tkEntry.c` in Tk 8.4, 8.5, 8.6,
/// 9.0 and 9.1. Each declares `-vcmd` / `-invcmd` as `TK_OPTION_SYNONYM`s
/// of the validation options, and `ExpandPercents` gives the spinbox
/// `-command` (`VALIDATE_BUTTON`) only `%s` (current text), `%d` (arrow
/// direction) and `%W`.
#[test]
fn classic_entry_callback_synonyms_and_spinbox_command_carry_user_text() {
    use crate::CallbackTaintInput;

    let validation = [
        CallbackTaintInput::TK_PROPOSED_VALUE,
        CallbackTaintInput::TK_CURRENT_VALUE,
        CallbackTaintInput::TK_EDIT_TEXT,
    ];
    for (tcl, tk) in [
        ("tcl8.4", "8.4"),
        ("tcl8.5", "8.5"),
        ("tcl8.6", "8.6"),
        ("tcl9.0", "9.0"),
        ("tcl9.1", "9.1"),
    ] {
        let mut reg = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find(tcl).unwrap());
        reg.insert_ambient_package("Tk", tk);
        for command in ["entry", "spinbox"] {
            for option in ["-vcmd", "-invcmd"] {
                let words = [".w", option, "{eval %P}"];
                assert_eq!(
                    reg.callback_taint_inputs(command, &words, 2, None),
                    &validation,
                    "Tk {tk} {command} {option} is a synonym of a validation callback"
                );
                let method = ["configure", option, "{eval %P}"];
                assert_eq!(
                    reg.instance_callback_taint_inputs(command, ".w", &method, 2, None),
                    &validation,
                    "Tk {tk} {command} configure {option}"
                );
            }
        }
        let command = [".w", "-command", "{eval %s}"];
        assert_eq!(
            reg.callback_taint_inputs("spinbox", &command, 2, None),
            &[CallbackTaintInput::TK_CURRENT_VALUE],
            "Tk {tk} spinbox -command receives the editable text as %s"
        );
        assert!(
            reg.callback_taint_inputs("ttk::spinbox", &command, 2, None)
                .is_empty(),
            "Tk {tk} ttk::spinbox runs -command with `uplevel #0`, no % substitution"
        );
    }
}

/// The registry states which stored scripts are callbacks — the words a
/// consumer must treat as run later, outside the registering frame — and
/// which are a definition's body or run now.
#[test]
fn callback_scripts_are_the_deferred_words_of_a_command_that_stores_no_definition() {
    let reg = CommandRegistry::build_default();
    let callbacks: &[(&str, &[&str], &[usize])] = &[
        ("after", &["100", "{set done 1}"], &[1]),
        ("after", &["idle", "{set done 1}"], &[1]),
        (
            "trace",
            &["add", "variable", "x", "write", "{set go 0 ;#}"],
            &[4],
        ),
        ("bind", &[".", "<Key>", "{set go 0}"], &[2]),
        ("fileevent", &["stdin", "readable", "{set go 0}"], &[2]),
        ("chan", &["event", "stdin", "readable", "{set go 0}"], &[3]),
        ("interp", &["bgerror", "{}", "{set go 0 ;#}"], &[2]),
        ("button", &[".b", "-command", "{set go 0}"], &[2]),
    ];
    for (name, args, expected) in callbacks {
        assert_eq!(
            reg.callback_script_indices(name, args, None),
            *expected,
            "{name} {args:?}"
        );
    }
    // A definition's body is dormant and runs in a frame of its own, and
    // the scripts these run now or only name are no callback.
    let none: &[(&str, &[&str])] = &[
        ("proc", &["p", "{}", "{set x 1}"]),
        ("snit::method", &["T", "m", "{}", "{set x 1}"]),
        ("lambda", &["{}", "{set x 1}"]),
        ("catch", &["{set x 1}"]),
        ("if", &["1", "{set x 1}"]),
        ("uplevel", &["#0", "{set x 1}"]),
        ("trace", &["remove", "variable", "x", "write", "cb"]),
        ("set", &["x", "1"]),
    ];
    for (name, args) in none {
        assert!(
            reg.callback_script_indices(name, args, None).is_empty(),
            "{name} {args:?}"
        );
    }
}

#[test]
fn typed_callback_inputs_preserve_unknown_prefix_slots_and_decline_expansion() {
    use crate::{CallbackTaintInput, InvocationWord, InvocationWords};
    let registry = callback_metadata_registry();
    let unknown_window = [
        InvocationWord::Dynamic,
        InvocationWord::Literal("<Key>"),
        InvocationWord::Literal("eval %A"),
    ];
    let invocation = InvocationWords::structured(InvocationWord::Literal("bind"), &unknown_window);
    assert_eq!(
        registry.callback_taint_inputs_words(invocation, 2, None),
        &[CallbackTaintInput::TK_EVENT_CHAR]
    );
    assert_eq!(
        registry.callback_taint_inputs_words(invocation, 1, None),
        &[] as &[CallbackTaintInput]
    );
    for arguments in [&unknown_window[..1], &unknown_window[..2]] {
        assert_eq!(
            registry.callback_taint_inputs_words(
                InvocationWords::structured(InvocationWord::Literal("bind"), arguments),
                2,
                None,
            ),
            &[] as &[CallbackTaintInput]
        );
    }
    let invalid = [
        InvocationWord::Dynamic,
        InvocationWord::Literal("<Key>"),
        InvocationWord::Literal("eval %A"),
        InvocationWord::Literal("extra"),
    ];
    assert_eq!(
        registry.callback_taint_inputs_words(
            InvocationWords::structured(InvocationWord::Literal("bind"), &invalid),
            2,
            None,
        ),
        &[] as &[CallbackTaintInput]
    );
    let canvas = [
        InvocationWord::Literal("bind"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("<Key>"),
        InvocationWord::Literal("eval %A"),
    ];
    assert_eq!(
        registry.instance_callback_taint_inputs_words(
            "canvas",
            InvocationWords::structured(InvocationWord::Literal(".canvas"), &canvas),
            3,
            None,
        ),
        &[CallbackTaintInput::TK_EVENT_CHAR]
    );
    let expanded_window = [InvocationWord::Expanded, InvocationWord::Literal("eval %A")];
    assert_eq!(
        registry.callback_taint_inputs_words(
            InvocationWords::structured(InvocationWord::Literal("bind"), &expanded_window),
            1,
            None,
        ),
        &[] as &[CallbackTaintInput]
    );
    let unknown_option = [
        InvocationWord::Literal(".entry"),
        InvocationWord::Dynamic,
        InvocationWord::Literal("eval %P"),
    ];
    assert_eq!(
        registry.callback_taint_inputs_words(
            InvocationWords::structured(InvocationWord::Literal("entry"), &unknown_option),
            2,
            None,
        ),
        &[] as &[CallbackTaintInput]
    );
}

#[test]
fn typed_callback_inputs_do_not_override_argument_sensitive_timing() {
    use crate::hover::ScriptTiming;
    use crate::{CallbackTaintInput, InvocationWord, InvocationWords};
    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "timed-callback",
        arity: crate::Arity::exact(2),
        arg_roles: &[(1, ArgRole::Body)],
        traits: Traits::DEFERS_BODY,
        callback_taint_inputs: &[(1, &[CallbackTaintInput::TK_EVENT_CHAR])],
        script_timing_resolver: Some(|_| vec![(1, ScriptTiming::SameInvocation)]),
        ..CommandSpec::DEFAULT
    });
    let unknown = [InvocationWord::Dynamic, InvocationWord::Literal("eval %A")];
    assert_eq!(
        registry.callback_taint_inputs_words(
            InvocationWords::structured(InvocationWord::Literal("timed-callback"), &unknown),
            1,
            None,
        ),
        &[] as &[CallbackTaintInput]
    );
    let known = [
        InvocationWord::Literal("window"),
        InvocationWord::Literal("eval %A"),
    ];
    assert_eq!(
        registry.callback_taint_inputs_words(
            InvocationWords::structured(InvocationWord::Literal("timed-callback"), &known),
            1,
            None,
        ),
        &[] as &[CallbackTaintInput]
    );
}

#[test]
fn argument_sensitive_timing_distinguishes_registration_and_send_forms() {
    use crate::hover::ScriptTiming;

    let reg = callback_metadata_registry();
    assert_eq!(
        reg.instance_script_timing(
            "canvas",
            ".receiver",
            &["bind", "tag", "<Button-1>", "clicked"],
            3,
            None,
        ),
        Some(ScriptTiming::Deferred),
    );
    assert_eq!(
        reg.instance_script_timing(
            "canvas",
            ".receiver",
            &["bi", "tag", "<Button-1>", "clicked"],
            3,
            None,
        ),
        Some(ScriptTiming::Deferred),
        "canvas bind's unique prefix must select the positional timing resolver",
    );
    assert_eq!(
        reg.script_timing(
            "wm",
            &["protocol", ".w", "WM_DELETE_WINDOW", "close"],
            3,
            None,
        ),
        Some(ScriptTiming::Deferred),
    );
    assert_eq!(
        reg.script_timing("wm", &["prot", ".w", "WM_DELETE_WINDOW", "close"], 3, None,),
        Some(ScriptTiming::Deferred),
        "wm protocol's unique prefix must not fall back to same-invocation timing",
    );
    assert_eq!(
        reg.script_timing("send", &["other", "work"], 1, None),
        Some(ScriptTiming::SameInvocation),
    );
    assert_eq!(
        reg.script_timing("send", &["-async", "other", "work"], 2, None,),
        Some(ScriptTiming::Deferred),
    );
    assert_eq!(
        reg.script_timing("send", &["-async", "other", "work", "argument"], 2, None,),
        None,
        "a concatenated multi-word script is not a fictional Body position",
    );
    assert_eq!(
        reg.script_timing(
            "selection",
            &["handle", "-type", "UTF8_STRING", ".w", "provide"],
            4,
            None,
        ),
        Some(ScriptTiming::Deferred),
    );
    assert_eq!(
        reg.script_timing(
            "selection",
            &["h", "-type", "UTF8_STRING", ".w", "provide"],
            4,
            None,
        ),
        Some(ScriptTiming::Deferred),
        "selection handle's unique prefix must select its positional timing resolver",
    );
    assert_eq!(
        reg.script_timing("selection", &["o", "-command", "lost", ".w"], 2, None,),
        Some(ScriptTiming::Deferred),
        "subcommand option timing must use the same unique-prefix resolution",
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn command_prefix_hosts_report_their_exact_phase() {
    use crate::hover::ScriptTiming;

    let reg = callback_metadata_registry();
    let cases: &[(&str, &[&str], usize, ScriptTiming)] = &[
        // Immediate consumers execute the prefix before returning.
        (
            "lsort",
            &["-command", "compare", "{b a}"],
            1,
            ScriptTiming::SameInvocation,
        ),
        (
            "coroprobe",
            &["worker", "inspect"],
            1,
            ScriptTiming::SameInvocation,
        ),
        (
            "generator",
            &["reduce", "combine", "0", "values"],
            1,
            ScriptTiming::SameInvocation,
        ),
        // Registration and asynchronous forms store the prefix.
        (
            "socket",
            &["-server", "accept", "0"],
            1,
            ScriptTiming::Deferred,
        ),
        (
            "fcopy",
            &["in", "out", "-command", "done"],
            3,
            ScriptTiming::Deferred,
        ),
        (
            "chan",
            &["copy", "in", "out", "-command", "done"],
            4,
            ScriptTiming::Deferred,
        ),
        (
            "interp",
            &["alias", "", "aliasName", "", "target"],
            4,
            ScriptTiming::Deferred,
        ),
        (
            "interp",
            &["bgerror", "", "handler"],
            2,
            ScriptTiming::Deferred,
        ),
        (
            "namespace",
            &["unknown", "handler"],
            1,
            ScriptTiming::Deferred,
        ),
        (
            "package",
            &["unknown", "handler"],
            1,
            ScriptTiming::Deferred,
        ),
        (
            "coroinject",
            &["worker", "handler"],
            1,
            ScriptTiming::Deferred,
        ),
        (
            "tcltest::customMatch",
            &["mode", "handler"],
            1,
            ScriptTiming::Deferred,
        ),
        (
            "trace",
            &["add", "variable", "v", "write", "handler"],
            4,
            ScriptTiming::Deferred,
        ),
        (
            "generator",
            &["map", "transform", "values"],
            1,
            ScriptTiming::Deferred,
        ),
        (
            "hook",
            &["bind", "subject", "name", "observer", "handler"],
            4,
            ScriptTiming::Deferred,
        ),
        (
            "comm::comm",
            &["send", "-command", "handler", "peer", "work"],
            2,
            ScriptTiming::Deferred,
        ),
        (
            "mime::getbody",
            &["token", "-command", "handler"],
            2,
            ScriptTiming::Deferred,
        ),
        (
            "stringprep::register",
            &["profile", "-prohibitedCommand", "handler"],
            2,
            ScriptTiming::Deferred,
        ),
        (
            "tcl::chan::halfpipe",
            &["-close-command", "handler"],
            1,
            ScriptTiming::Deferred,
        ),
        // Removal forms name executable text without running or storing it.
        (
            "trace",
            &["remove", "variable", "v", "write", "handler"],
            4,
            ScriptTiming::ReferenceOnly,
        ),
    ];

    for &(command, args, index, expected) in cases {
        assert_eq!(
            reg.script_timing(command, args, index, None),
            Some(expected),
            "wrong phase for {command} {args:?} argument {index}",
        );
    }

    assert_eq!(
        reg.script_timing("trace", &["a", "v", "v", "write", "handler"], 4, None,),
        Some(ScriptTiming::Deferred),
        "unique-prefix subcommands and trace types preserve phase metadata",
    );
}

#[test]
fn legacy_trace_prefix_timing_requires_a_release_with_that_form() {
    use crate::hover::ScriptTiming;
    for (profile, available) in [
        ("tcl8.4", true),
        ("tcl8.5", true),
        ("tcl8.6", true),
        ("tcl9.0", false),
        ("tcl9.1", false),
    ] {
        let owner = crate::model::ingress::static_context_for(profile);
        let registry = owner.commands();
        for (operation, timing) in [
            ("variable", ScriptTiming::Deferred),
            ("vdelete", ScriptTiming::ReferenceOnly),
        ] {
            assert_eq!(
                registry.script_timing("trace", &[operation, "v", "w", "handler"], 3, None),
                available.then_some(timing),
                "{profile}: {operation}"
            );
        }
    }
}

#[test]
fn option_timing_resolver_overrides_static_default_for_bibtex_input_mode() {
    use crate::hover::ScriptTiming;

    let reg = callback_metadata_registry();
    let in_memory = ["-recordcommand", "record", "@book{x}"];
    assert_eq!(
        reg.script_timing("bibtex::parse", &in_memory, 1, None),
        Some(ScriptTiming::SameInvocation),
    );

    let channel = ["-recordcommand", "record", "-channel", "input"];
    assert_eq!(
        reg.script_timing("bibtex::parse", &channel, 1, None),
        Some(ScriptTiming::Deferred),
    );

    let completion = ["-command", "done", "-channel", "input"];
    assert_eq!(
        reg.script_timing("bibtex::parse", &completion, 1, None),
        Some(ScriptTiming::Deferred),
    );
}

#[test]
fn instance_timing_requires_the_selected_provider_and_method() {
    use crate::hover::ScriptTiming;

    let provided = callback_metadata_registry();
    let arguments = ["dump", "-command", "visit", "1.0"];
    assert_eq!(
        provided.instance_script_timing("text", ".receiver", &arguments, 2, None),
        Some(ScriptTiming::SameInvocation),
    );
    let unprovided = CommandRegistry::build_default()
        .project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
    assert_eq!(
        unprovided.instance_script_timing("text", ".receiver", &arguments, 2, None),
        None,
    );
    assert_eq!(
        provided.instance_script_timing("text", ".receiver", &["unknown", "visit"], 1, None),
        None,
    );
    assert_eq!(
        provided.instance_script_timing("text", ".receiver", &arguments, 1, None),
        None,
        "an option name is not executable callback text",
    );
}

#[test]
fn constructor_option_callbacks_keep_their_original_argument_positions() {
    use crate::hover::ScriptTiming;
    let registry = callback_metadata_registry();
    let constructor = [".canvas", "-xscrollcommand", "visit"];
    assert_eq!(
        registry.script_timing("canvas", &constructor, 2, None),
        Some(ScriptTiming::Deferred),
    );
    for index in [0, 1] {
        assert_eq!(
            registry.script_timing("canvas", &constructor, index, None),
            None
        );
    }
    assert_eq!(
        registry.script_timing("canvas", &[".canvas", "-xscrollcommand"], 2, None),
        None,
    );
    let method = ["configure", "-xscrollcommand", "visit"];
    assert_eq!(
        registry.instance_script_timing("canvas", ".canvas", &method, 2, None),
        Some(ScriptTiming::Deferred),
    );
    assert_eq!(
        registry.instance_script_timing("canvas", ".canvas", &method, 1, None),
        None,
    );
    for query in [vec!["configure"], vec!["configure", "-xscrollcommand"]] {
        let invocation = registry
            .resolve_instance_invocation("canvas", ".canvas", &query, None)
            .expect("the provided classic widget retains its configure query");
        assert!(!invocation.semantics.mutator);
        assert!(invocation.semantics.traits.contains(Traits::PURE));
        assert_eq!(
            registry.instance_script_timing("canvas", ".canvas", &query, 1, None),
            None,
            "query forms do not execute the stored option value",
        );
    }
    let setter = registry
        .resolve_instance_invocation("canvas", ".canvas", &method, None)
        .expect("the provided classic widget retains its configure setter");
    assert!(setter.semantics.mutator);
    assert!(
        setter
            .semantics
            .traits
            .contains(Traits::CONFIGURES_INSTANCE_OPTIONS)
    );
    let cget = ["cget", "-xscrollcommand"];
    let invocation = registry
        .resolve_instance_invocation("canvas", ".canvas", &cget, None)
        .expect("the provided classic widget retains its cget method");
    assert!(!invocation.semantics.mutator);
    assert_eq!(
        registry.instance_script_timing("canvas", ".canvas", &cget, 1, None),
        None,
    );
}

#[test]
fn known_stored_tk_command_prefixes_are_deferred() {
    use crate::hover::ScriptTiming;

    let reg = callback_metadata_registry();
    let stored = [
        ("canvas", "-xscrollcommand"),
        ("canvas", "-yscrollcommand"),
        ("entry", "-xscrollcommand"),
        ("listbox", "-xscrollcommand"),
        ("listbox", "-yscrollcommand"),
        ("menu", "-tearoffcommand"),
        ("scale", "-command"),
        ("scrollbar", "-command"),
        ("spinbox", "-xscrollcommand"),
        ("text", "-xscrollcommand"),
        ("text", "-yscrollcommand"),
        ("ttk::combobox", "-xscrollcommand"),
        ("ttk::entry", "-xscrollcommand"),
        ("ttk::scale", "-command"),
        ("ttk::scrollbar", "-command"),
        ("ttk::spinbox", "-xscrollcommand"),
        ("ttk::treeview", "-xscrollcommand"),
        ("ttk::treeview", "-yscrollcommand"),
        ("tk_chooseDirectory", "-command"),
        ("tk_getOpenFile", "-command"),
        ("tk_getSaveFile", "-command"),
        ("tk_messageBox", "-command"),
    ];
    for (command, option) in stored {
        let spec = reg
            .get_for_surface(command, reg.own_surface_query())
            .unwrap();
        let arguments = if spec.constructor_prefix_words().is_some() {
            vec![".w", option, "callback"]
        } else {
            vec![option, "callback"]
        };
        assert_eq!(
            reg.script_timing(command, &arguments, arguments.len() - 1, None),
            Some(ScriptTiming::Deferred),
            "{command} {option} stores its command prefix",
        );
    }

    assert_eq!(
        reg.instance_script_timing(
            "text",
            ".receiver",
            &["dump", "-command", "visit", "1.0"],
            2,
            None,
        ),
        Some(ScriptTiming::SameInvocation),
    );
    assert_eq!(
        reg.instance_script_timing(
            "ttk::treeview",
            ".receiver",
            &["sort", "root", "-command", "compare"],
            3,
            None,
        ),
        Some(ScriptTiming::SameInvocation),
    );
}
