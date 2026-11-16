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

use super::SourceCommandBindings;
use crate::command_binding::SourceAnalysisOptions;
use tcl_core_types::NameBytes;
use tcl_registry::CommandRegistry;

#[test]
fn original_receiver_table_keeps_opaque_and_zero_names_distinct() {
    let source = r"oo::class create C {self {method p\uD800 {} {return A}; method p\uD801 {} {return B}; method p\u0000tail {} {return ZERO}}}; C p\uD800; C p\uD801; C p\u0000tail";
    for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let mut declarations = Vec::new();
        for (spelling, expected) in [
            (r"p\uD800", b"p\xed\xa0\x80".as_slice()),
            (r"p\uD801", b"p\xed\xa0\x81".as_slice()),
            (r"p\u0000tail", b"p\xc0\x80tail".as_slice()),
        ] {
            let offset = u32::try_from(source.rfind(&format!("C {spelling}")).unwrap()).unwrap();
            let binding = bindings.invocation_at_source("C", offset);
            let (_, entry) = binding
                .class_definition_method_entry()
                .unwrap_or_else(|| panic!("{name}: independently retained selector {spelling}"));
            assert_eq!(entry.original_name_input().bytes(), expected, "{name}");
            declarations.push(entry.declaration().clone());
        }
        assert_ne!(declarations[0], declarations[1]);
        assert_ne!(declarations[1], declarations[2]);
    }
}

#[test]
fn original_overlapping_mixin_keeps_declared_body_without_a_dispatch_chain() {
    // naming.tcloo.original-lexical-member-context
    // docs/design/analysis/name-resolution-proofs/tcloo-original-lexical-member-context.md
    let source = "oo::class create M {method a::b {arg} {}}\n\
                      oo::class create C {mixin M; method a::b {} {next 1 2}}";
    for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.find("next 1 2").unwrap()).unwrap();
        let origin = bindings.source_origin().unwrap();
        let body = bindings
            .declared_receiver_body_entry_at(origin, offset)
            .expect(name);
        assert_eq!(body.source().base(), offset, "{name}");
        let mut commands =
            crate::segmenter::segment_commands_with_offset_and_config("next 1 2", offset, config);
        let command = commands.pop().unwrap();
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &command,
        );
        bindings.stamp_original_tokens(&mut tokens);
        let binding = tokens.source_binding.as_ref().unwrap();
        assert!(
            binding
                .original_catalogue_source_candidate_from_tokens(&tokens, &registry)
                .is_some(),
            "{name}: conditional source helper retains the exact original vector",
        );
        assert!(
            bindings.original_completed_command_world().is_none(),
            "{name}: the overlapping runtime method chain remains unavailable",
        );
    }
}

#[test]
fn original_receiver_formal_admission_preserves_counted_opaque_names() {
    let source = r"oo::class create C {self {method first arg\uD800 {return FIRST}; method second arg\uD801 {return SECOND}}}; C first value; C second value";
    for name in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(name).unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        for (method, expected) in [
            ("first", b"arg\xed\xa0\x80".as_slice()),
            ("second", b"arg\xed\xa0\x81".as_slice()),
        ] {
            let offset = u32::try_from(source.rfind(&format!("C {method}")).unwrap()).unwrap();
            let binding = bindings.invocation_at_source("C", offset);
            let (_, entry) = binding
                .class_definition_method_entry()
                .unwrap_or_else(|| panic!("{name}: selected original {method}"));
            let body = bindings
                .declared_receiver_body_entry_at(&entry.declaration().source, entry.body().base())
                .expect("independent original receiver body declaration");
            let topology = body
                .original_formal_topology()
                .expect("independent original formal-list producer");
            assert_eq!(topology.parameters()[0].name, expected, "{name}");
        }
    }
}

#[test]
fn native_default_constructor_uses_captured_worker_identity_and_rejects_replacement() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let entry: crate::environment_ingress::CapturedNativeEntry =
        crate::environment_ingress::captured_native_entry(profile);
    for (prefix, expected) in [
        ("", true),
        ("rename ::oo::object {}; proc ::oo::object args {}; ", false),
    ] {
        let source =
            format!("{prefix}oo::class create C {{method m {{p}} {{::return $p}}}}; C new");
        let bindings = SourceCommandBindings::analyse_with_options(
            &source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            registry,
            SourceAnalysisOptions {
                native_entry: Some(&entry),
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.rfind("C new").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("C", offset);
        assert_eq!(
            binding.proved_class_definition_factory().is_some(),
            expected,
            "{source}"
        );
    }
}

#[test]
fn visibility_metadata_keeps_original_entries_and_definition_order() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    for (members, name, expected) in [
        (
            "method Upper {} {return ORIGINAL}; export Upper",
            "Upper",
            true,
        ),
        (
            "export Upper; method Upper {} {return ORIGINAL}",
            "Upper",
            false,
        ),
        (
            "method ping {} {return ORIGINAL}; unexport ping",
            "ping",
            false,
        ),
        (
            "unexport ping; method ping {} {return ORIGINAL}",
            "ping",
            true,
        ),
        (
            "method Upper {} {return FIRST}; export Upper; method Upper {} {return ORIGINAL}",
            "Upper",
            false,
        ),
        (
            "export absent; method ping {} {return ORIGINAL}",
            "ping",
            true,
        ),
    ] {
        let source = format!("oo::class create C {{{members}}}; C new");
        let bindings = SourceCommandBindings::analyse_with_options(
            &source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.rfind("C new").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("C", offset);
        let entry = binding
            .class_definition_method_entries()
            .and_then(|(_, entries)| {
                entries.get(&(
                    super::SourceMethodReceiver::Instance,
                    NameBytes::from(name.as_bytes()),
                ))
            })
            .expect("native visibility leaves the original method entry installed");
        assert_eq!(entry.is_exported(), expected, "{source}");
        assert_eq!(entry.body().text.try_text().unwrap(), "return ORIGINAL");
    }
}

#[test]
fn native_classmethod_inheritance_retains_the_original_delegate_owner() {
    for dialect in ["tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
        let registry = CommandRegistry::build_default().project_for_profile(profile);
        for (source, expected) in [
            (
                "oo::class create Base {classmethod find {} {return ORIGINAL}}; oo::class create Child {superclass Base}; Child find",
                true,
            ),
            (
                "oo::class create Base {self method find {} {return ORIGINAL}}; oo::class create Child {superclass Base}; Child find",
                false,
            ),
            (
                "proc ::oo::define::classmethod args {error REPLACED}; oo::class create Base {classmethod find {} {return ORIGINAL}}; oo::class create Child {superclass Base}; Child find",
                false,
            ),
            (
                "oo::class create Base {classmethod find {} {return ORIGINAL}}; oo::class create Child {superclass Base}; oo::define Base classmethod find {} {return REPLACED}; Child find",
                false,
            ),
            (
                "oo::class create Base {classmethod find {} {return ORIGINAL}}; oo::class create Child {superclass Base}; unknownOperation; Child find",
                false,
            ),
            (
                "oo::class create Base {classmethod find {} {return ORIGINAL}}; oo::define ::oo::class constructor args {next {*}$args; oo::define Base classmethod find {} {return REPLACED}}; oo::class create Child {superclass Base}; Child find",
                false,
            ),
        ] {
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                &registry,
                SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            ..Default::default()
                        },
                    ..Default::default()
                },
            );
            let binding = bindings.invocation_at_source(
                "Child",
                u32::try_from(source.rfind("Child find").unwrap()).unwrap(),
            );
            let entry = binding
                .class_definition_method_entries()
                .and_then(|(_, entries)| {
                    entries.get(&(
                        super::SourceMethodReceiver::Class,
                        NameBytes::from(b"find".as_slice()),
                    ))
                });
            assert_eq!(entry.is_some(), expected, "{dialect}: {source}");
            if let Some(entry) = entry {
                assert!(entry.is_native_class_delegate());
                assert_eq!(entry.body().text.try_text().unwrap(), "return ORIGINAL");
                assert_eq!(
                    entry
                        .declaring_class()
                        .unwrap()
                        .command
                        .trim_start_matches("::"),
                    "Base"
                );
                assert_eq!(
                    entry.declaration().offset,
                    u32::try_from(source.find("classmethod").unwrap()).unwrap()
                );
            }
        }
    }
}

#[test]
fn class_method_entry_requires_both_original_definition_receivers() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    for (source, expected) in [
        (
            "oo::class create C {method ping {} {return INSTANCE}; self method ping {original} {return CLASS}}; C ping V",
            true,
        ),
        (
            "proc ::oo::define::self args {error REPLACED}; oo::class create C {self method ping {} {return CLASS}}; C ping",
            false,
        ),
        (
            "proc ::oo::objdefine::method args {error REPLACED}; oo::class create C {self method ping {} {return CLASS}}; C ping",
            false,
        ),
        (
            "oo::class create C {self method ping {} {return CLASS}}; C ping [oo::objdefine C method ping args {return NEW}]",
            false,
        ),
    ] {
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.rfind("C ping").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("C", offset);
        let selected = binding
            .class_definition_method_entries()
            .and_then(|(_, entries)| {
                entries.get(&(
                    super::SourceMethodReceiver::Class,
                    NameBytes::from(b"ping".as_slice()),
                ))
            });
        assert_eq!(selected.is_some(), expected, "{source}: {binding:#?}");
        if let Some(entry) = selected {
            assert_eq!(entry.body().text.try_text().unwrap(), "return CLASS");
            assert_eq!(entry.formals(), &[("original".to_owned(), None)]);
            assert_eq!(
                entry.declaration().offset,
                u32::try_from(source.find("self method").unwrap()).unwrap()
            );
            assert_eq!(entry.receiver(), super::SourceMethodReceiver::Class);
        }
    }
}

#[test]
fn class_block_method_entry_retains_the_original_inner_source() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    for (source, expected) in [
        (
            "oo::class create C {self {method ping {original} {return CLASS}}}; C ping V",
            true,
        ),
        (
            "proc ::oo::define::self args {error REPLACED}; oo::class create C {self {method ping {original} {return CLASS}}}; C ping V",
            false,
        ),
        (
            "proc ::oo::objdefine::method args {error REPLACED}; oo::class create C {self {method ping {original} {return CLASS}}}; C ping V",
            false,
        ),
        (
            "oo::class create C {self {method ping {original} {return CLASS}}}; C ping [oo::objdefine C method ping args {return NEW}]",
            false,
        ),
        (
            "oo::class create C {self {method ping {original} {return FIRST}; method ping {original} {return CLASS}}}; C ping V",
            true,
        ),
        (
            "oo::class create C {self {method ping {original} {return CLASS}; superclass Other}}; C ping V",
            false,
        ),
    ] {
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.rfind("C ping").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("C", offset);
        let selected = binding
            .class_definition_method_entries()
            .and_then(|(_, entries)| {
                entries.get(&(
                    super::SourceMethodReceiver::Class,
                    NameBytes::from(b"ping".as_slice()),
                ))
            });
        assert_eq!(selected.is_some(), expected, "{source}: {binding:#?}");
        if let Some(entry) = selected {
            assert_eq!(entry.body().text.try_text().unwrap(), "return CLASS");
            assert_eq!(entry.formals(), &[("original".to_owned(), None)]);
            assert_eq!(
                entry.declaration().offset,
                u32::try_from(source.rfind("method ping").unwrap()).unwrap()
            );
            assert_eq!(
                entry.name_source().span.start(),
                u32::try_from(source.rfind("method ping").unwrap() + 7).unwrap()
            );
            assert_eq!(entry.receiver(), super::SourceMethodReceiver::Class);
            assert!(matches!(
                entry.frame(),
                crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
            ));
        }
    }
}

#[test]
fn method_source_inventory_is_temporal_and_retains_receiver_kind() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    for (source, expected) in [
        (
            "oo::class create C {method ping {original} {return FIRST}}; C new",
            Some("return FIRST"),
        ),
        (
            "oo::class create C {method ping {} {return FIRST}}; oo::define C method ping {} {return SECOND}; C new",
            None,
        ),
        (
            "oo::class create C {method ping {} {return FIRST}}; C new [oo::define C method ping {} {return SECOND}]",
            None,
        ),
        (
            "oo::class create C {method ping {} {return FIRST}}; rename C {}; oo::class create C {method ping {} {return SECOND}}; C new",
            Some("return SECOND"),
        ),
    ] {
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.rfind("C new").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("C", offset);
        let entries = binding.class_definition_method_entries();
        assert_eq!(
            entries.is_some(),
            expected.is_some(),
            "{source}: {binding:#?}"
        );
        if let Some((target, entries)) = entries {
            let entry = entries
                .get(&(
                    super::SourceMethodReceiver::Instance,
                    NameBytes::from(b"ping".as_slice()),
                ))
                .unwrap();
            assert_eq!(target.command, "::C");
            assert_eq!(entry.body().text.try_text().unwrap(), expected.unwrap());
            assert_eq!(entry.name(), "ping");
            assert!(matches!(
                entry.frame(),
                crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
            ));
            assert!(!entries.contains_key(&(
                super::SourceMethodReceiver::Class,
                NameBytes::from(b"ping".as_slice())
            )));
            let declaration = source.rfind("method ping").unwrap();
            assert_eq!(
                entry.declaration().offset,
                u32::try_from(declaration).unwrap()
            );
        }
    }
}

#[test]
fn constructor_entry_retains_original_formals_without_proving_the_result_class() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    for (source, expected) in [
        (
            "oo::class create C {constructor {original} {error CALLBACK}}; C new value",
            true,
        ),
        (
            "oo::class create C {variable held; constructor {original} {error CALLBACK}}; C new value",
            true,
        ),
        (
            "oo::class create C {constructor {original} {}}; C new",
            false,
        ),
        (
            "oo::class create C {constructor {original} {}}; oo::define C constructor {replacement} {}; C new value",
            false,
        ),
        (
            "oo::class create C {constructor {original} {}}; C new [oo::define C constructor {replacement} {}]",
            false,
        ),
        (
            "oo::class create Base {constructor {base} {}}; oo::class create C {superclass Base; constructor {original} {}}; C new value",
            false,
        ),
    ] {
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.rfind("C new").unwrap()).unwrap();
        let binding = bindings.invocation_at_source("C", offset);
        let entry = binding.constructor_entry(&registry);
        assert_eq!(entry.is_some(), expected, "{source}: {binding:#?}");
        if let Some((target, entry, payload)) = entry {
            assert_eq!(target.command, "::C");
            assert_eq!(entry.formals(), &[("original".to_owned(), None)]);
            assert_eq!(entry.body().text.try_text().unwrap(), "error CALLBACK");
            assert!(matches!(
                entry.frame(),
                crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
            ));
            assert_eq!(payload, 1);
            assert!(binding.proved_construction_result(&registry).is_none());
        }
    }
}

#[test]
fn class_definition_receipt_is_independent_of_constructor_execution() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    for (source, expected) in [
        ("oo::abstract create Base {}; Base new", true),
        (
            "oo::abstract create Base {}; rename Base Moved; Moved new",
            true,
        ),
        (
            "oo::class create C {constructor {} {error NO}}; C new",
            true,
        ),
        (
            "oo::abstract create Base {}; oo::objdefine Base export new; Base new",
            false,
        ),
        (
            "oo::abstract create Base {}; oo::objdefine Base method new {} {return YES}; Base new",
            false,
        ),
    ] {
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            &registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.rfind(';').unwrap() + 2).unwrap();
        let binding = bindings.invocation_at_source("", offset);
        assert_eq!(
            binding.proved_class_definition_factory().is_some(),
            expected,
            "source={source} binding={binding:#?}"
        );
    }
}

#[test]
fn compiled_class_definition_retains_absolute_method_command_identity() {
    let source = "oo::class create C {method m {} {::return 1}}";
    let registry = CommandRegistry::build_default();
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::default(),
        &registry,
        SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                tcl_dialect::TclVersion::V8_6,
            )),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                catch_depth: Some(0),
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let offset = u32::try_from(source.find("::return").unwrap()).unwrap();
    let binding = bindings.invocation_at_source("::return", offset);
    assert!(
        matches!(
            binding.variable_frame,
            crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
        ),
        "{binding:#?}"
    );
    assert!(
        binding
            .proved_handler_target()
            .is_some_and(|target| target.command == "::return"),
        "{binding:#?}"
    );
}

#[test]
fn constructor_declaration_retains_its_formal_and_exact_read_source() {
    let source = "oo::class create C {constructor {s} {::set re {a+}; ::regexp $re $s}}";
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::from_grammar(profile.grammar),
        &registry,
        SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let offset = u32::try_from(source.find("::regexp").unwrap()).unwrap();
    let binding = bindings.invocation_at_source("::regexp", offset);
    assert!(binding.proved_handler_target().is_some(), "{binding:#?}");
    assert!(
        bindings
            .variable_accesses
            .values()
            .flatten()
            .any(|access| { access.original_spelling == "$re" })
    );
}

#[test]
fn method_declaration_retains_local_frame_without_receiver_namespace() {
    let source = "oo::class create C {method m {} {global g; ::puts ok}}";
    let bindings = SourceCommandBindings::analyse_with_options(
        source,
        tcl_lexer::LexerConfig::default(),
        &CommandRegistry::build_default(),
        SourceAnalysisOptions {
            invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                tcl_dialect::TclVersion::V9_1,
            )),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                ..Default::default()
            },
            ..SourceAnalysisOptions::default()
        },
    );
    let global = bindings.invocation_at_source(
        "global",
        u32::try_from(source.find("global").unwrap()).unwrap(),
    );
    assert!(matches!(
        global.variable_frame,
        crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
    ));
    assert!(!global.variable_context.namespace_known);
    assert!(global.unknown);
    assert!(global.targets.iter().any(
        |target| target.registry_backed && target.command.trim_start_matches("::") == "global"
    ));
    let offset = u32::try_from(source.find("global").unwrap()).unwrap();
    let config = tcl_lexer::LexerConfig::default();
    let map = tcl_lexer::SourceMap::new(source);
    let segment =
        crate::segmenter::segment_commands_with_offset_and_config("global g", offset, config)
            .into_iter()
            .next()
            .unwrap();
    let mut tokens = crate::ir::CommandTokens::from_segmented(&map, config, &segment);
    tokens.source_binding = Some(global);
    let registry = CommandRegistry::build_default();
    let footprint =
        crate::registry_invocation::possible_variable_alias_transitions(&registry, None, &tokens)
            .unwrap();
    assert!(footprint.unknown_residual());
    assert!(footprint.aliases().any(|alias| matches!(
        alias.target,
        tcl_registry::VariableAliasTarget::Global { .. }
    ) && alias.local.literal() == Some("g")));
    assert!(
        crate::registry_invocation::normal_transfer_invocation(&registry, None, &tokens).is_none()
    );
}
