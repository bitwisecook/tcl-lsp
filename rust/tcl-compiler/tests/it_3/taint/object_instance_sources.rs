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

//! Sources on object instances: registry-typed receivers, their classes and the methods that expose external input.

// Registry object-instance sources.  Widget paths and object variables are
// commands at runtime, so the constructor's `object_class` / `creates_instance_at`
// metadata must carry the `TAINT_SOURCE` trait on `get` through that dispatch.
// These cases deliberately name no widget or getter in compiler code.
mod object_instance_taint_sources {
    use super::super::*;

    #[test]
    fn literal_widget_path_get_taints() {
        let source = "ttk::entry .user\nset value [.user get]\neval $value";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "value"),
            "registry instance source should reach eval: {warnings:?}"
        );
    }

    #[test]
    fn instance_taint_sources_follow_the_resolved_widget_lifecycle() {
        // ttk::treeview `current` is a zero-argument taint source added in
        // Tk 9.1. The generic instance-invocation resolver must not replay
        // that source fact for a tcl9.0 profile merely because the class is
        // otherwise known there.
        let source = "ttk::treeview .tree\nset value [.tree current]\neval $value";
        for (dialect, expects_taint) in [("tcl9.0", false), ("tcl9.1", true)] {
            assert_eq!(
                !of_code(source, dialect, "T100").is_empty(),
                expects_taint,
                "ttk::treeview current taint source under {dialect}"
            );
        }
    }

    #[test]
    fn widget_handle_variable_get_taints() {
        let source = "set widget [ttk::entry .user]\nset value [$widget get]\neval $value";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "value"),
            "typed object handle source should reach eval: {warnings:?}"
        );
    }

    #[test]
    fn instance_source_flows_through_proc_summary() {
        let source = "proc user_value {} {\n    ttk::entry .user\n    return [.user get]\n}\nset value [user_value]\neval $value";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "value"),
            "instance source in a proc return should reach eval: {warnings:?}"
        );
    }

    #[test]
    fn top_level_widget_is_a_source_inside_callback_procedure() {
        let source = "ttk::entry .user\nproc submit {} {\n    set value [.user get]\n    eval $value\n}\nsubmit";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "value"),
            "top-level widget commands remain visible in callback procedures: {warnings:?}"
        );
    }

    #[test]
    fn text_dump_and_file_chooser_results_are_input_sources() {
        for source in [
            "text .editor\nset value [.editor dump 1.0 end]\neval $value",
            "set value [tk_getOpenFile]\neval $value",
            "set value [tk_chooseDirectory]\neval $value",
        ] {
            let warnings = of_code(source, D, "T100");
            assert!(
                warnings.iter().any(|warning| warning.variable == "value"),
                "user-selected or user-authored Tk value must be tainted: {warnings:?}"
            );
        }
    }

    #[test]
    fn ambiguous_instance_class_abstains() {
        let source = "ttk::entry .widget\nlistbox .widget\nset value [.widget get]\neval $value";
        assert!(
            of_code(source, D, "T100").is_empty(),
            "a receiver with incompatible registry classes must not be guessed"
        );
    }

    #[test]
    fn linked_entry_variable_is_user_input_and_show_is_not_a_sanitiser() {
        let source = "entry .password -show * -textvariable password\neval $password";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings
                .iter()
                .any(|warning| warning.variable == "password"),
            "password masking must not sanitize linked user input: {warnings:?}"
        );
    }

    #[test]
    fn deferred_validation_callback_preserves_link_and_instance_taint() {
        let source = "entry .password -textvariable password -validatecommand {return 1}\nset typed [.password get]\neval $password\neval $typed";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings
                .iter()
                .any(|warning| warning.variable == "password"),
            "the deferred callback must not hide the linked-variable definition: {warnings:?}"
        );
        assert!(
            warnings.iter().any(|warning| warning.variable == "typed"),
            "the deferred callback must not hide the constructor's instance class: {warnings:?}"
        );
    }

    #[test]
    fn validation_percent_values_taint_inline_and_list_prefix_callbacks() {
        for source in [
            "entry .password -validatecommand {set proposed %P; eval $proposed}",
            "entry .password -validatecommand [list eval %P]",
            "entry .password -validatecommand \"[list eval %P]\"",
            "entry .password -validatecommand \"eval %P\"",
            r"entry .password -validatecommand eval\ %P",
            "bind .password <Key> {set typed %A; eval $typed}",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "registry-declared callback input must reach its sink: {source}"
            );
        }
    }

    #[test]
    fn validation_percent_value_in_command_position_is_dynamic_dispatch() {
        for source in [
            "entry .password -validatecommand {%P}",
            "entry .password -validatecommand {{*}%P}",
            "entry .password -validatecommand [list %P]",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "a callback-provided command head is tainted dynamic dispatch: {source}"
            );
        }
    }

    #[test]
    fn callback_replay_scaffold_ignores_user_catch_and_proc_commands() {
        for source in [
            "proc catch args {}\nentry .password -validatecommand {eval %P}",
            "proc proc args {}\nentry .password -validatecommand {eval %P}",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "user commands named like replay plumbing must not suppress the callback sink: {source}"
            );
        }
    }

    #[test]
    fn resolved_widget_instance_callbacks_replay_registry_declared_inputs() {
        for source in [
            "canvas .canvas\n.canvas bind all <Key> {eval %A}",
            "entry .entry\n.entry configure -validatecommand {eval %P}",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "a statically typed instance callback must use its registry descriptor: {source}"
            );
        }
    }

    #[test]
    fn dynamic_or_unknown_widget_receivers_do_not_invent_callback_sources() {
        for source in [
            ".unknown bind all <Key> {eval %A}",
            "canvas .canvas\nset receiver .canvas\n$receiver bind all <Key> {eval %A}",
        ] {
            assert!(
                of_code(source, D, "T100").is_empty(),
                "a non-singleton/literal receiver must not be guessed as a callback host: {source}"
            );
        }
    }

    #[test]
    fn validation_callback_percent_source_survives_braced_quoted_and_embedded_words() {
        for source in [
            // Tk substitutes `%P` before evaluating this script. Its braced
            // word is a user value, not a trusted literal for the later eval.
            "entry .password -validatecommand {eval {%P}}",
            "entry .password -validatecommand {eval \"prefix-%P\"}",
            "entry .password -validatecommand {set proposed {%P}; eval $proposed}",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "Tk callback replacement must remain tainted across word quoting: {source}"
            );
        }
    }

    #[test]
    fn validation_callback_replay_preserves_later_word_substitutions() {
        for source in [
            "entry .password -validatecommand {set x %P; eval \"puts $x\"}",
            "entry .password -validatecommand {set x %P; eval prefix-$x}",
            "entry .password -validatecommand {set x %P; eval \"puts [set y $x]\"}",
            "entry .password -validatecommand {set command eval; $command %P}",
            r"entry .password -validatecommand {set x %P; e\166al $x}",
            r"entry .password -validatecommand {set x %P; eval \[eval\ \$x\]}",
            r"entry .password -validatecommand {set x %P; [e\166al $x]}",
            "entry .password -validatecommand {set x %P; [puts safe; eval $x]}",
            "entry .password -validatecommand {set command eval; set x %P; [$command $x]}",
            "entry .password -validatecommand {set x %P; [set y $x; eval {$y}]}",
            r"entry .password -validatecommand [list e\166al %P]",
            r"entry .password -validatecommand {set x %P; eval {eval\
 $x}}",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "replay must preserve quoted, compound, and command substitutions: {source}"
            );
        }
    }

    #[test]
    fn validation_callback_replay_preserves_argument_expansion() {
        for source in [
            "entry .password -validatecommand {set x %P; {*}{{eval} safe} $x}",
            "entry .password -validatecommand {set x %P; set prefix [list eval safe]; {*}$prefix $x}",
            "entry .password -validatecommand [list {*}{{eval} safe} %P]",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "callback replay must retain the list-expanded eval prefix: {source}"
            );
        }
        assert!(
            of_code(
                "entry .password -validatecommand {set x %P; {*}{{puts} safe} $x}",
                D,
                "T100",
            )
            .is_empty(),
            "a list-expanded non-sink callback command must remain clean"
        );
    }

    #[test]
    fn validation_list_prefix_builder_uses_source_position_command_identity() {
        for source in [
            "interp alias {} make {} list\nentry .password -validatecommand [make eval %P]",
            "interp alias {} make {} list\nentry .password -validatecommand \"[make eval %P]\"",
            "rename list make\nentry .password -validatecommand [make eval %P]",
        ] {
            assert!(
                !of_code(source, D, "T100").is_empty(),
                "an alias or rename of list constructs the same callback prefix: {source}"
            );
        }
        for source in [
            "proc list args {return {puts safe}}\nentry .password -validatecommand [list eval %P]",
            "interp alias {} make {} list puts\nentry .password -validatecommand [make eval %P]",
        ] {
            assert!(
                of_code(source, D, "T100").is_empty(),
                "a rebound builder or alias with prepended argv must not inherit bare list semantics: {source}"
            );
        }
    }

    #[test]
    fn validation_callback_replay_decodes_static_non_head_backslashes() {
        let source = r"entry .password -validatecommand {set x %P; eval puts\ \$x}";
        assert!(
            !of_code(source, D, "T101").is_empty(),
            "a backslash-built script argument must retain its decoded output-sink semantics"
        );
    }

    #[test]
    fn validation_list_prefix_preserves_a_quoted_command_name() {
        assert!(
            of_code(
                "entry .password -validatecommand [list {eval foo} %P]",
                D,
                "T100",
            )
            .is_empty(),
            "a list element naming one command with a space must not be replayed as builtin eval"
        );
    }

    #[test]
    fn validation_callback_replay_input_name_cannot_collide_with_program_variables() {
        let source =
            "entry .password -validatecommand {set __tcl_lsp_callback_input trusted; eval {%P}}";
        assert!(
            !of_code(source, D, "T100").is_empty(),
            "the replay source name must avoid a callback/program variable collision"
        );
    }

    #[test]
    fn validation_callback_seed_ignores_shadowed_scaffolding_commands() {
        let source = "proc gets {args} { return trusted }\n\
                      proc set {args} { return trusted }\n\
                      entry .password -validatecommand {eval {%P}}";
        assert!(
            !of_code(source, D, "T100").is_empty(),
            "Tk callback input is an external source even when gets/set are shadowed"
        );
    }

    #[test]
    fn validation_callback_direct_input_diagnostic_hides_synthetic_state() {
        let source = "entry .password -validatecommand {eval %P}";
        let warnings = of_code(source, D, "T100");
        let warning = warnings
            .iter()
            .find(|warning| warning.variable == "Tk callback input %P")
            .expect("direct Tk input warning: {warnings:?}");
        assert!(
            warning.message.contains("Tk callback input %P"),
            "the user-facing message must identify the Tk source: {warning:?}"
        );
        assert!(
            !warning.message.contains("__tcl_lsp_callback"),
            "synthetic replay state must not leak to the user: {warning:?}"
        );
        assert_eq!(
            warning.span.start(),
            u32::try_from(source.find("{eval %P}").expect("callback span"))
                .expect("test source fits in u32"),
            "the direct callback sink maps back to its registration"
        );
        assert!(
            warning.replacement.is_none() && warning.fixes.is_empty(),
            "synthetic replay diagnostics cannot carry synthetic-source edits: {warning:?}"
        );
    }

    #[test]
    fn callback_replays_isolate_abnormal_completion_and_handler_locals() {
        for first in ["return %P", "error %P"] {
            let source = format!(
                "entry .first -validatecommand {{{first}}}\n\
                 entry .second -validatecommand {{eval %P}}"
            );
            let second_start = u32::try_from(source.rfind("{eval %P}").expect("second callback"))
                .expect("test source fits in u32");
            assert!(
                of_code(&source, D, "T100")
                    .iter()
                    .any(|warning| warning.span.start() == second_start),
                "an abnormal first callback cannot make its sibling unreachable: {source}"
            );
        }

        let clean_then_tainted = "entry .first -validatecommand {set x safe; return %P}\n\
                                  entry .second -validatecommand {set x %P; eval $x}";
        assert!(
            !of_code(clean_then_tainted, D, "T100").is_empty(),
            "a prior callback's clean local cannot kill the later handler's source"
        );

        let tainted_then_clean = "entry .first -validatecommand {set x %P}\n\
                                  entry .second -validatecommand {set unused %P; eval $x}";
        assert!(
            of_code(tainted_then_clean, D, "T100").is_empty(),
            "a prior callback's local cannot invent a warning in a sibling handler"
        );
    }

    #[test]
    fn validation_list_prefix_taints_a_named_proc_parameter() {
        let source = "proc validate {proposed} { eval $proposed }\nentry .password -validatecommand [list validate %P]";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings
                .iter()
                .any(|warning| warning.variable == "proposed"),
            "a list-built callback prefix must seed its procedure parameter: {warnings:?}"
        );
        let sink_start = u32::try_from(source.find("$proposed").expect("procedure sink variable"))
            .expect("test source fits in u32");
        assert!(
            warnings
                .iter()
                .any(|warning| warning.variable == "proposed" && warning.span.start() == sink_start),
            "a proc-prefix warning belongs on the real sink, not the registration: {warnings:?}"
        );
    }

    #[test]
    fn callback_framework_metadata_and_escaped_percent_stay_clean() {
        for source in [
            "entry .password -validatecommand {eval %W}",
            "entry .password -validatecommand {eval %%P}",
            "bind .password <Key> {eval %W}",
            "bind .password <Key> {eval %K}",
            "entry .password -validatecommand [format {eval %P}]",
        ] {
            assert!(
                of_code(source, D, "T100").is_empty(),
                "metadata, escaped markers, and dynamic callback builders stay untainted: {source}"
            );
        }
    }

    #[test]
    fn user_controlled_non_text_widget_variable_is_tainted() {
        let source = "checkbutton .choice -variable choice\neval $choice";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "choice"),
            "user-controlled linked widget state must be tainted: {warnings:?}"
        );
    }

    #[test]
    fn display_only_textvariable_and_widget_handle_stay_clean() {
        assert!(
            of_code(
                "set safe fixed\nlabel .display -textvariable safe\neval $safe",
                D,
                "T100"
            )
            .is_empty(),
            "a display-only textvariable is not a user-input source"
        );
        assert!(
            of_code("set widget [entry .user]\neval $widget", D, "T100").is_empty(),
            "a widget constructor's trusted handle is not user input"
        );
    }

    #[test]
    fn mixed_widget_links_taint_only_the_user_editable_variable() {
        let source = "set caption fixed\ncheckbutton .choice -textvariable caption -variable choice\neval $caption\neval $choice";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "choice"),
            "the selection variable is user controlled: {warnings:?}"
        );
        assert!(
            warnings.iter().all(|warning| warning.variable != "caption"),
            "the display-only textvariable must stay clean: {warnings:?}"
        );
    }

    #[test]
    fn instance_configure_adds_a_phase_correct_external_input_definition() {
        let source = "entry .user\neval $value\n.user configure -textvariable value\neval $value";
        let warnings = of_code(source, D, "T100");
        assert_eq!(
            warnings
                .iter()
                .filter(|warning| warning.variable == "value")
                .count(),
            1,
            "only the use after the registry-typed configure link is tainted: {warnings:?}"
        );

        let source = "set widget [ttk::entry .user]\n$widget configure -textvariable value\nset copy $value\neval $copy";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "copy"),
            "a typed variable receiver's configured input link must propagate: {warnings:?}"
        );
    }

    #[test]
    fn display_only_instance_configure_does_not_taint_its_variable() {
        let source = "label .caption\n.caption configure -textvariable caption\neval $caption";
        assert!(
            of_code(source, D, "T100").is_empty(),
            "a display-only class option must not become an input source"
        );
    }

    #[test]
    fn argument_sensitive_widget_getters_taint_only_user_state_forms() {
        for source in [
            "scale .s\nset user [.s get]\nset derived [.s get 10 20]\neval $user\neval $derived",
            "ttk::toggleswitch .s\nset user [.s get]\nset derived [.s get min]\neval $user\neval $derived",
            "ttk::toggleswitch .s\nset user [.s switchstate]\nset derived [.s switchstate true]\neval $user\neval $derived",
            "ttk::toggleswitch .s\nset user [.s xcoord]\nset derived [.s xcoord 0.5]\neval $user\neval $derived",
        ] {
            let warnings = of_code(source, D, "T100");
            assert!(
                warnings.iter().any(|warning| warning.variable == "user"),
                "the zero-argument getter reads user state: {warnings:?}"
            );
            assert!(
                !warnings.iter().any(|warning| warning.variable == "derived"),
                "the explicit coordinate/selector getter is derived, not input: {warnings:?}"
            );
        }
    }

    #[test]
    fn overloaded_treeview_and_notebook_getters_only_taint_zero_arg_queries() {
        for (dialect, query, setter) in [
            (
                D,
                "ttk::combobox .combo\nset user [.combo current]\neval $user",
                "ttk::combobox .combo\nset value [.combo current 0]\neval $value",
            ),
            (
                D,
                "ttk::treeview .tree\nset user [.tree selection]\neval $user",
                "ttk::treeview .tree\nset value [.tree selection set item]\neval $value",
            ),
            (
                "tcl9.0",
                "ttk::treeview .tree\nset user [.tree cellselection]\neval $user",
                "ttk::treeview .tree\nset value [.tree cellselection set item]\neval $value",
            ),
            (
                "tcl9.1",
                "ttk::treeview .tree\nset user [.tree current]\neval $user",
                "ttk::treeview .tree\nset value [.tree current ignored]\neval $value",
            ),
            (
                D,
                "ttk::treeview .tree\nset user [.tree focus]\neval $user",
                "ttk::treeview .tree\nset value [.tree focus item]\neval $value",
            ),
            (
                "tcl9.1",
                "ttk::treeview .tree\nset user [.tree cellfocus]\neval $user",
                "ttk::treeview .tree\nset value [.tree cellfocus 0,0]\neval $value",
            ),
            (
                D,
                "ttk::notebook .book\nset user [.book select]\neval $user",
                "ttk::notebook .book\nset value [.book select .tab]\neval $value",
            ),
        ] {
            assert!(
                of_code(query, dialect, "T100")
                    .iter()
                    .any(|warning| warning.variable == "user"),
                "zero-argument Tk UI query must be a source under {dialect}: {query}"
            );
            assert!(
                of_code(setter, dialect, "T100").is_empty(),
                "setter/argument overload must not be a source under {dialect}: {setter}"
            );
        }
    }

    #[test]
    fn pointer_position_queries_are_user_input_sources() {
        for query in [
            "winfo pointerx .",
            "winfo pointery .",
            "winfo pointerxy .",
            "winfo containing 10 20",
            "winfo containing -displayof . 10 20",
        ] {
            let source = format!("set user [{query}]\neval $user");
            let warnings = of_code(&source, D, "T100");
            assert!(
                warnings.iter().any(|warning| warning.variable == "user"),
                "pointer-dependent query must be a source: {query}; {warnings:?}"
            );
        }
    }

    #[test]
    fn later_constructor_does_not_retroactively_type_an_earlier_receiver() {
        assert!(
            of_code(
                "set value [.future get]\nttk::entry .future\neval $value",
                D,
                "T100"
            )
            .is_empty(),
            "receiver typing must respect source order"
        );
    }

    #[test]
    fn destroy_kills_local_instance_class_and_recreate_restores_it() {
        assert!(
            of_code(
                "entry .user\ndestroy .user\nset value [.user get]\neval $value",
                D,
                "T100"
            )
            .is_empty(),
            "a registry teardown must remove the destroyed widget from local receiver facts"
        );
        assert!(
            of_code(
                "entry .user\ndestroy .user\nentry .user\nset value [.user get]\neval $value",
                D,
                "T100"
            )
            .iter()
            .any(|warning| warning.variable == "value"),
            "a later constructor must recreate the receiver class after teardown"
        );

        assert!(
            of_code(
                "frame .panel\nentry .panel.user\ndestroy .\nset value [.panel.user get]\neval $value",
                D,
                "T100"
            )
            .is_empty(),
            "destroying the Tk root must kill every descendant receiver fact"
        );
    }

    #[test]
    fn rename_moves_local_and_global_instance_class_facts() {
        let local = "entry .user\nrename .user .renamed\nset value [.renamed get]\neval $value";
        assert!(
            of_code(local, D, "T100")
                .iter()
                .any(|warning| warning.variable == "value"),
            "registry command-table rename must move a local widget class"
        );

        let global = "entry .user\nrename .user .renamed\nproc read_value {} {\n    set value [.renamed get]\n    eval $value\n}\nread_value";
        assert!(
            of_code(global, D, "T100")
                .iter()
                .any(|warning| warning.variable == "value"),
            "registry command-table rename must move global callback class facts"
        );
    }

    #[test]
    fn global_destroy_kills_callback_instance_class_facts() {
        let source = "entry .user\ndestroy .user\nproc read_value {} {\n    set value [.user get]\n    eval $value\n}\nread_value";
        assert!(
            of_code(source, D, "T100").is_empty(),
            "a top-level registry teardown must not leave a stale global callback receiver"
        );
    }

    #[test]
    fn instance_method_abbreviations_resolve_only_when_unique() {
        let warnings = of_code(
            "entry .user\nset value [.user g]\neval $value\n.user co -textvariable linked\neval $linked",
            D,
            "T100",
        );
        assert!(
            warnings.iter().any(|warning| warning.variable == "value"),
            "the unique `get` prefix must retain source semantics: {warnings:?}"
        );
        assert!(
            warnings.iter().any(|warning| warning.variable == "linked"),
            "the unique `configure` prefix must retain option-write semantics: {warnings:?}"
        );

        assert!(
            of_code(
                "entry .user\n.user c -textvariable linked\neval $linked",
                D,
                "T100"
            )
            .is_empty(),
            "`c` is ambiguous between cget/configure and must abstain"
        );
    }

    #[test]
    fn literal_global_receiver_configure_taints_across_procedures() {
        let source = "entry .user\nproc link_input {} {\n    .user configure -textvariable ::value\n}\nlink_input\neval $::value";
        let warnings = of_code(source, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "::value"),
            "a callee's proven widget link must define and taint the caller's global: {warnings:?}"
        );

        let nested = "entry .user\nproc inner {} {\n    .user configure -textvariable ::value\n}\nproc outer {} { inner }\nouter\neval $::value";
        let warnings = of_code(nested, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "::value"),
            "instance-option global writes must close transitively over calls: {warnings:?}"
        );
    }

    #[test]
    fn tk_unqualified_link_targets_global_not_proc_local_state() {
        let false_positive = "package require Tk\nproc p {} {\n    set value safe\n    entry .e -textvariable value\n    eval $value\n}\np";
        let warnings = of_code(false_positive, D, "T100");
        assert!(
            warnings.is_empty(),
            "Tk's unqualified link is ::value; it must not taint proc-local value: {warnings:?}"
        );

        let false_negative = "package require Tk\nproc p {} {\n    entry .e -textvariable value\n}\np\neval $::value";
        let warnings = of_code(false_negative, D, "T100");
        assert!(
            warnings.iter().any(|warning| warning.variable == "::value"),
            "Tk's unqualified link must taint the documented global ::value: {warnings:?}"
        );
    }

    #[test]
    fn constructor_and_configure_global_links_close_over_proc_calls() {
        for source in [
            "proc inner {} { entry .user -textvariable value }\nproc outer {} { inner }\nouter\neval $::value",
            "entry .user\nproc inner {} { .user configure -textvariable value }\nproc outer {} { inner }\nouter\neval $::value",
            "proc inner {} { entry .user -textvariable ::value }\ninner\neval $::value",
        ] {
            let warnings = of_code(source, D, "T100");
            assert!(
                warnings.iter().any(|warning| warning.variable == "::value"),
                "registry-global Tk link must propagate through callers: {warnings:?}"
            );
        }
    }

    #[test]
    fn dynamic_parameter_receiver_configure_abstains() {
        let source = "entry .user\nproc link_input {widget} {\n    $widget configure -textvariable ::value\n}\nlink_input .user\neval $::value";
        assert!(
            of_code(source, D, "T100").is_empty(),
            "a parameter receiver has no uniform class proof and must not be guessed"
        );
    }

    #[test]
    fn eager_call_and_conditional_constructor_do_not_create_global_receiver_facts() {
        let called_too_early = "proc link_input {} {\n    .user configure -textvariable ::value\n}\nlink_input\nentry .user\neval $::value";
        assert!(
            of_code(called_too_early, D, "T100").is_empty(),
            "a constructor after an eager call cannot type that invocation"
        );

        let conditional = "if {$flag} { entry .user }\nproc link_input {} {\n    .user configure -textvariable ::value\n}\nlink_input\neval $::value";
        assert!(
            of_code(conditional, D, "T100").is_empty(),
            "a conditional constructor does not prove a receiver exists"
        );

        let conditional_rebind = "entry .user\nif {$flag} { label .user }\nproc read_input {} {\n    set value [.user get]\n    eval $value\n}\nread_input";
        assert!(
            of_code(conditional_rebind, D, "T100").is_empty(),
            "a possible later constructor withdraws an earlier receiver proof"
        );
    }

    /// A caught call before the constructor fails at `.user` with "invalid
    /// command name" and evaluates nothing; the stock `unknown` defines
    /// nothing, so the later call still reaches the entry's text (#2427).
    /// tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0, with `proc .user` in place
    /// of the widget: `puts [list first [catch {read_input} msg] $msg]` prints
    /// `first 1 {invalid command name ".user"}`, then the second `read_input`
    /// runs the evaluated text.
    #[test]
    fn a_caught_early_call_does_not_withdraw_the_receiver_from_a_later_call() {
        let source = "package require Tk\nproc read_input {} {\n    set value [.user get]\n    eval $value\n}\ncatch {read_input}\nentry .user\nread_input";
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            assert!(
                dialect_of_code(source, dialect, "T100")
                    .iter()
                    .any(|warning| warning.variable == "value"),
                "{dialect}: the call after the constructor evaluates the entry's text"
            );
        }
    }

    #[test]
    fn receiver_rebinding_and_branch_joins_use_reaching_classes() {
        assert!(
            of_code(
                "set widget [entry .user]\nset widget [label .display]\nset value [$widget get]\neval $value",
                D,
                "T100"
            )
            .is_empty(),
            "a later non-source receiver binding must kill the earlier entry class"
        );

        let rebound = "set widget [label .display]\nset widget [entry .user]\nset value [$widget get]\neval $value";
        assert!(
            of_code(rebound, D, "T100")
                .iter()
                .any(|warning| warning.variable == "value"),
            "the currently reaching entry binding must be used"
        );

        let same_class_join = "if {$flag} {\n    set widget [entry .one]\n} else {\n    set widget [entry .two]\n}\nset value [$widget get]\neval $value";
        assert!(
            of_code(same_class_join, D, "T100")
                .iter()
                .any(|warning| warning.variable == "value"),
            "a join whose paths agree on class remains typed"
        );

        let mixed_join = "if {$flag} {\n    set widget [entry .one]\n} else {\n    set widget [label .two]\n}\nset value [$widget get]\neval $value";
        assert!(
            of_code(mixed_join, D, "T100").is_empty(),
            "a join with multiple possible classes must abstain"
        );

        let global_rebound = "entry .shared\nlabel .shared\nproc use_value {} {\n    set value [.shared get]\n    eval $value\n}\nuse_value";
        assert!(
            of_code(global_rebound, D, "T100").is_empty(),
            "the last unconditional constructor determines a global literal receiver"
        );

        let global_rebound_to_source = "label .shared\nentry .shared\nproc use_value {} {\n    set value [.shared get]\n    eval $value\n}\nuse_value";
        assert!(
            of_code(global_rebound_to_source, D, "T100")
                .iter()
                .any(|warning| warning.variable == "value"),
            "a global literal receiver rebound to an entry remains a source"
        );
    }
}
