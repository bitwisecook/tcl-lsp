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

#[cfg(test)]
mod tests {
    use tcl_irules::{IlxExtension, ilx_method_calls};
    use tcl_registry::CommandRegistry;
    use tcl_registry::remote_method::RemoteDispatch;

    fn irules_registry() -> std::sync::Arc<CommandRegistry> {
        tcl_registry::model::ingress::static_context_for("f5-irules")
            .commands()
            .snapshot()
            .shared_registry()
    }

    fn calls(source: &str) -> Vec<(String, Option<IlxExtension>, RemoteDispatch)> {
        ilx_method_calls(source, &irules_registry())
            .into_iter()
            .map(|call| (call.method, call.target, call.dispatch))
            .collect()
    }

    #[test]
    fn a_written_handle_does_not_issue_runtime_extension_identity() {
        crate::support::install_surfaces();
        let got = calls(concat!(
            "when HTTP_REQUEST {\n",
            "  set h [ILX::init my_plugin my_extension]\n",
            "  set reply [ILX::call $h my_js_function arg1]\n",
            "}\n",
        ));
        assert_eq!(
            got,
            vec![(
                "my_js_function".to_owned(),
                None,
                RemoteDispatch::Synchronous
            )]
        );
    }

    #[test]
    fn the_timeout_option_and_terminator_are_not_the_method() {
        crate::support::install_surfaces();
        let got = calls(concat!(
            "when HTTP_REQUEST {\n",
            "  set h [ILX::init p e]\n",
            "  ILX::call $h -timeout 3000 -- real_method x\n",
            "}\n",
        ));
        assert_eq!(
            got,
            vec![("real_method".to_owned(), None, RemoteDispatch::Synchronous)]
        );
    }

    #[test]
    fn notify_is_a_notification_sharing_the_method_target() {
        crate::support::install_surfaces();
        let got = calls(concat!(
            "when HTTP_REQUEST {\n",
            "  set h [ILX::init p e]\n",
            "  ILX::notify $h fire_and_forget a b\n",
            "}\n",
        ));
        assert_eq!(
            got,
            vec![(
                "fire_and_forget".to_owned(),
                None,
                RemoteDispatch::Notification
            )]
        );
    }

    #[test]
    fn an_inline_construction_keeps_handle_result_authority_independent() {
        crate::support::install_surfaces();
        let got = calls("when RULE_INIT {\n  ILX::call [ILX::init p e] m\n}\n");
        assert_eq!(
            got,
            vec![("m".to_owned(), None, RemoteDispatch::Synchronous)]
        );
    }

    #[test]
    fn dynamic_plugin_extension_or_reassignment_abstains() {
        crate::support::install_surfaces();
        // Every one of these keeps the method word (hover can name it) and
        // drops the target (navigation must not guess).
        for source in [
            "when X {\n set h [ILX::init $p e]\n ILX::call $h m\n}\n",
            "when X {\n set h [ILX::init p $e]\n ILX::call $h m\n}\n",
            "when X {\n set h [ILX::init p e]\n set h $other\n ILX::call $h m\n}\n",
            "when X {\n set h [something_else p e]\n ILX::call $h m\n}\n",
            "when X {\n ILX::call $undefined m\n}\n",
            // The one-word `ILX::init` spelling F5 does not document.
            "when X {\n set h [ILX::init e]\n ILX::call $h m\n}\n",
        ] {
            let got = calls(source);
            assert_eq!(got.len(), 1, "{source}");
            assert_eq!(got[0].0, "m", "{source}");
            assert_eq!(got[0].1, None, "the target must abstain: {source}");
        }
    }

    #[test]
    fn a_computed_method_word_is_not_a_site_at_all() {
        crate::support::install_surfaces();
        for source in [
            "when X {\n set h [ILX::init p e]\n ILX::call $h $method\n}\n",
            "when X {\n set h [ILX::init p e]\n ILX::call $h m$suffix\n}\n",
            "when X {\n set h [ILX::init p e]\n ILX::call $h [get_method]\n}\n",
        ] {
            assert!(calls(source).is_empty(), "{source}");
        }
    }

    #[test]
    fn a_body_that_opens_a_new_frame_does_not_inherit_the_handle() {
        crate::support::install_surfaces();
        // A `proc` body runs in a fresh local frame, so `$h` is *undefined*
        // when `f` runs — resolving it from the enclosing scope would be a
        // false go-to-definition. Which bodies inherit the caller's frame
        // is registry data (`CommandSpec::body_kind`).
        let got = calls(concat!(
            "set h [ILX::init p e]\n",
            "proc f {} { ILX::call $h m }\n",
        ));
        assert_eq!(
            got,
            vec![("m".to_owned(), None, RemoteDispatch::Synchronous)]
        );

        // Its own written constructor still needs an independent evaluated result.
        let own = calls("proc f {} { set h [ILX::init p e]; ILX::call $h m }\n");
        assert_eq!(
            own,
            vec![("m".to_owned(), None, RemoteDispatch::Synchronous)]
        );
    }

    #[test]
    fn control_body_source_cards_do_not_issue_handle_cell_inheritance() {
        crate::support::install_surfaces();
        // Script geometry selects source cards without proving an entered frame or cell.
        for source in [
            "when X {\n set h [ILX::init p e]\n if {1} { ILX::call $h m }\n}\n",
            "when X {\n set h [ILX::init p e]\n foreach i {1 2} { ILX::call $h m }\n}\n",
            "when X {\n set h [ILX::init p e]\n catch { ILX::call $h m }\n}\n",
            "when X {\n set h [ILX::init p e]\n while {0} { ILX::call $h m }\n}\n",
        ] {
            assert_eq!(
                calls(source),
                vec![("m".to_owned(), None, RemoteDispatch::Synchronous)],
                "{source}"
            );
        }
    }

    #[test]
    fn a_sibling_event_handler_does_not_leak_its_handle() {
        crate::support::install_surfaces();
        let got = calls(concat!(
            "when CLIENT_ACCEPTED {\n  set h [ILX::init p e]\n}\n",
            "when HTTP_REQUEST {\n  ILX::call $h m\n}\n",
        ));
        assert_eq!(
            got,
            vec![("m".to_owned(), None, RemoteDispatch::Synchronous)]
        );
    }

    #[test]
    fn a_switch_arm_is_walked() {
        crate::support::install_surfaces();
        let got = calls(concat!(
            "when HTTP_REQUEST {\n",
            "  set h [ILX::init p e]\n",
            "  switch [HTTP::uri] {\n",
            "    \"/api\" { ILX::call $h api_method }\n",
            "  }\n",
            "}\n",
        ));
        assert_eq!(
            got,
            vec![("api_method".to_owned(), None, RemoteDispatch::Synchronous)]
        );
    }

    #[test]
    fn plain_tcl_has_no_ilx_relation() {
        crate::support::install_surfaces();
        // ILX command specs are available only in an iRules registry.
        let registry = CommandRegistry::build_default();
        let got = ilx_method_calls("set h [ILX::init p e]\nILX::call $h m\n", &registry);
        assert!(got.is_empty(), "{got:?}");
    }

    #[test]
    fn original_ilx_source_cards_keep_inline_candidates_and_current_owners() {
        crate::support::install_surfaces();
        let registry = irules_registry();
        let source = "when HTTP_REQUEST { ILX::call [ILX::init p e] -timeout 3000 -- m }";
        let context = tcl_irules::OriginalIrulesSourceContext::capture(source, &registry).unwrap();
        let got = tcl_irules::ilx::ilx_method_calls_from_source_context(source, &context);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].method, "m");
        assert_eq!(
            got[0].source_target,
            Some(IlxExtension {
                plugin: "p".to_owned(),
                extension: "e".to_owned()
            })
        );
        assert_eq!(got[0].target, None);
        assert!(!got[0].obligations.is_empty());
        assert_eq!(&source[got[0].method_span.as_range()], "m");
        assert_eq!(
            tcl_irules::ilx::ilx_method_calls_from_source_context(
                &format!("{source}\n# changed"),
                &context
            ),
            [] as [tcl_irules::IlxMethodCall; 0]
        );
    }

    #[test]
    fn original_ilx_source_cards_do_not_derive_handles_from_cell_or_label_guesses() {
        crate::support::install_surfaces();
        for source in [
            "when HTTP_REQUEST {set h [ILX::init p e]; ILX::call $h m}",
            "when HTTP_REQUEST {set h [ILX::init p e]; unknown; ILX::call $h m}",
            "when HTTP_REQUEST { ILX::call [unknown p e] m }",
        ] {
            let got = ilx_method_calls(source, &irules_registry());
            assert_eq!(got.len(), 1, "{source}");
            assert_eq!(got[0].target, None);
            assert_eq!(got[0].source_target, None);
        }
        assert_eq!(
            ilx_method_calls(
                "when HTTP_REQUEST {ILX::call $h $method}",
                &irules_registry()
            ),
            [] as [tcl_irules::IlxMethodCall; 0]
        );
        assert_eq!(
            ilx_method_calls(
                "when HTTP_REQUEST {ILX::call $h m}",
                &CommandRegistry::build_default()
            ),
            [] as [tcl_irules::IlxMethodCall; 0]
        );
    }

    #[test]
    fn hosted_handle_candidates_retain_the_compiler_owner_and_all_obligations() {
        crate::support::install_surfaces();
        let source = "when HTTP_REQUEST {set h [ILX::init p e]; ILX::call $h m; ILX::notify $h n}";
        let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "f5-irules");
        let calls = tcl_irules::ilx::ilx_method_calls_from_analysis(source, &analysis);
        assert_eq!(calls.len(), 2);
        assert_eq!(
            calls[0].source_target,
            Some(IlxExtension {
                plugin: "p".to_owned(),
                extension: "e".to_owned()
            })
        );
        assert_eq!(calls[0].target, None);
        assert_eq!(calls[0].obligations.len(), 5);
        let association = calls[0].source_association().unwrap();
        assert!(association.matches_source_analysis(source, &analysis));
        let fresh = tcl_compiler::analyser::Analyser::new().analyse(source, "f5-irules");
        assert!(!association.matches_source_analysis(source, &fresh));
        assert_eq!(calls[1].source_target, None);
        assert_eq!(calls[1].source_association(), None);
        assert_eq!(calls[1].target, None);
        assert_eq!(
            tcl_irules::ilx::ilx_method_calls_from_analysis(
                &format!("{source}\n# changed"),
                &analysis
            ),
            [] as [tcl_irules::IlxMethodCall; 0]
        );
        let mut unavailable = analysis.clone();
        unavailable.resolved_input = None;
        assert_eq!(
            tcl_irules::ilx::ilx_method_calls_from_analysis(source, &unavailable),
            [] as [tcl_irules::IlxMethodCall; 0]
        );
    }

    #[test]
    fn hosted_handle_candidates_preserve_same_frame_and_withdraw_at_barriers() {
        crate::support::install_surfaces();
        for source in [
            "when HTTP_REQUEST {set h [ILX::init p e]; if {1} {ILX::call $h m}}",
            "when HTTP_REQUEST {if {1} {set h [ILX::init p e]; ILX::notify $h m}}",
        ] {
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "f5-irules");
            let calls = tcl_irules::ilx::ilx_method_calls_from_analysis(source, &analysis);
            assert_eq!(calls.len(), 1, "{source}");
            assert!(calls[0].source_target.is_some(), "{source}");
            assert_eq!(calls[0].target, None);
        }
        for source in [
            "when HTTP_REQUEST {set h [ILX::init p e]; unknown; ILX::call $h m}",
            "when HTTP_REQUEST {set h [ILX::init p e]; set h other; ILX::call $h m}",
            "when HTTP_REQUEST {set h [ILX::init p e]; trace add variable h read hook; ILX::call $h m}",
            "when RULE_INIT {set h [ILX::init p e]}; when HTTP_REQUEST {ILX::call $h m}",
            "set h [ILX::init p e]; when HTTP_REQUEST {ILX::call $h m}",
        ] {
            let analysis = tcl_compiler::analyser::Analyser::new().analyse(source, "f5-irules");
            let calls = tcl_irules::ilx::ilx_method_calls_from_analysis(source, &analysis);
            assert_eq!(calls.len(), 1, "{source}");
            assert_eq!(calls[0].source_target, None, "{source}");
            assert_eq!(calls[0].source_association(), None);
            assert_eq!(calls[0].target, None);
        }
    }
}
