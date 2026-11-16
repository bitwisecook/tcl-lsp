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

//! Built-in command cards: package requirements, valid events, subcommands and options from the registry.

use super::*;

#[test]
fn builtin_hover_shows_requires_when_package_not_imported() {
    // `http::geturl` needs `package require http`; without the
    // import the hover appends the **Requires** line.
    let src = "http::geturl $url\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 0, 6, &analysis, Some(&registry)).expect("hover");
    assert!(
        h.value.contains("**Requires**: `package require http`"),
        "{}",
        h.value
    );
}

#[test]
fn builtin_hover_omits_requires_when_package_imported() {
    // With `package require http` present, the hint is suppressed.
    let src = "package require http\nhttp::geturl $url\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 1, 6, &analysis, Some(&registry)).expect("hover");
    assert!(!h.value.contains("**Requires**"), "{}", h.value);
}

#[test]
fn builtin_hover_resolves_imported_command() {
    // A bare command imported into the global scope resolves
    // to its qualified spec — hovering `test` after `namespace import
    // ::tcltest::*` surfaces the `tcltest::test` documentation.
    let src = "namespace import ::tcltest::*\ntest t-1 {desc} -body { set x 1 } -result 1\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 1, 2, &analysis, Some(&registry)).expect("hover on bare `test`");
    assert!(
        h.value.contains("tcltest::test"),
        "bare imported `test` must hover as tcltest::test: {}",
        h.value
    );
}

#[test]
fn builtin_hover_bare_test_without_import_is_none() {
    // Control: without an import, a bare `test` is not a known command.
    let src = "test t-1 {desc}\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    assert!(hover(src, 0, 0, &analysis, Some(&registry)).is_none());
}

#[test]
fn builtin_hover_ignores_nested_namespace_import() {
    // An import made inside `namespace eval ns { … }` is not in scope at top
    // level, so hovering a bare top-level `test` must NOT resolve to
    // tcltest::test (mirrors the analyser's scope rule).
    let src = "namespace eval ns { namespace import ::tcltest::* }\ntest t-1 {desc}\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    // `test` is on line 1 (top level) — out of the `ns` import's scope.
    assert!(
        hover(src, 1, 2, &analysis, Some(&registry)).is_none(),
        "a nested-namespace import must not resolve a top-level bare `test`",
    );
}

#[test]
fn builtin_hover_ignores_import_after_cursor() {
    // Source order: hovering a `test` that appears *before* the
    // `namespace import` must not resolve — the import is not yet in effect.
    let src = "test t-1 {desc}\nnamespace import ::tcltest::*\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    assert!(
        hover(src, 0, 2, &analysis, Some(&registry)).is_none(),
        "an import after the cursor must not retroactively resolve `test`",
    );
}

#[test]
fn valid_events_for_asm_profile_requirement() {
    use tcl_registry::events::EventRequires;
    // A command requiring the ASM profile is valid only in ASM events.
    let requires = EventRequires {
        client_side: false,
        server_side: false,
        transport: None,
        profiles: &["ASM"],
        also_in: &[],
        flow: false,
    };
    let events = valid_events(&requires, EventRelease::Unconfigured);
    assert_eq!(
        events,
        vec![
            "ASM_REQUEST_BLOCKING",
            "ASM_REQUEST_DONE",
            "ASM_REQUEST_VIOLATION",
            "ASM_RESPONSE_LOGIN",
            "ASM_RESPONSE_VIOLATION",
        ],
        "ASM-profile events"
    );
}

#[test]
fn valid_events_uses_transitive_profile_expansion() {
    use tcl_registry::events::EventRequires;
    // `HTTP`-profile requirement matches HTTP events directly *and*
    // events whose implied profile transitively requires HTTP (e.g.
    // `ADAPT_REQUEST_HEADERS` via `REQUESTADAPT` → `HTTP`).  This
    // exercises `expand_profile_stack`.
    let requires = EventRequires {
        client_side: false,
        server_side: false,
        transport: Some("tcp"),
        profiles: &["FASTHTTP", "HTTP"],
        also_in: &[],
        flow: false,
    };
    let events = valid_events(&requires, EventRelease::Unconfigured);
    assert!(events.iter().any(|e| e == "HTTP_REQUEST"), "{events:?}");
    assert!(
        events.iter().any(|e| e == "ADAPT_REQUEST_HEADERS"),
        "transitive-profile event missing (expansion broken): {events:?}"
    );
}

#[test]
fn valid_events_apply_the_configured_release_lifecycle() {
    use tcl_registry::events::EventRequires;
    let requires = EventRequires {
        client_side: false,
        server_side: false,
        transport: None,
        profiles: &["XML"],
        also_in: &[],
        flow: false,
    };
    let unconfigured = valid_events(&requires, EventRelease::Unconfigured);
    assert!(unconfigured.iter().any(|e| e == "XML_BEGIN_DOCUMENT"));
    let old = valid_events(&requires, EventRelease::Configured("9.4.0"));
    assert!(old.iter().any(|e| e == "XML_BEGIN_DOCUMENT"), "{old:?}");
    let current = valid_events(&requires, EventRelease::Configured("16.1.0"));
    assert!(
        !current.iter().any(|e| e == "XML_BEGIN_DOCUMENT"),
        "retired at 10.0.0: {current:?}"
    );

    let mut text = String::new();
    append_valid_events(&mut text, &requires, EventRelease::Configured("9.4.0"));
    assert!(text.contains("**Valid events** (BIG-IP 9.4.0)"), "{text}");
    let mut text = String::new();
    append_valid_events(&mut text, &requires, EventRelease::Unconfigured);
    assert!(text.contains("no BIG-IP release configured"), "{text}");
}

#[test]
fn hover_valid_events_follow_the_documents_configured_release() {
    use tcl_compiler::analyser::Analyser;
    for (version, retired_listed) in [("9.4.0", true), ("16.1.0", false)] {
        let analysis = Analyser::new()
            .with_bigip_version(Some(version.to_owned()))
            .analyse("", "f5-irules");
        let context = analysis
            .resolved_input
            .as_ref()
            .expect("an iRules analysis retains its input")
            .availability_context();
        let release = EventRelease::of(context);
        assert_eq!(release, EventRelease::Configured(version));
        let events = valid_events(
            &tcl_registry::events::EventRequires {
                client_side: false,
                server_side: false,
                transport: None,
                profiles: &["XML"],
                also_in: &[],
                flow: false,
            },
            release,
        );
        assert_eq!(
            events.iter().any(|e| e == "XML_BEGIN_DOCUMENT"),
            retired_listed,
            "{version}: {events:?}"
        );
    }
}

#[test]
fn expand_profile_stack_includes_parents() {
    let reg = tcl_registry::profiles::ProfileRegistry::build();
    let stack = reg.expand_profile_stack(&["HTTP"]);
    assert!(stack.contains("HTTP"), "{stack:?}");
    assert!(stack.contains("TCP"), "HTTP should require TCP: {stack:?}");
}

#[test]
fn irules_command_hover_lists_valid_events() {
    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let text =
        builtin_command_hover_text(&registry, "ASM::is_authenticated", &analyse(""), u32::MAX)
            .expect("registered iRules command should have hover");
    assert!(text.contains("**Valid events**"), "{text}");
    assert!(text.contains("ASM_REQUEST_BLOCKING"), "{text}");
    assert!(text.contains("**Requires**: profile ASM"), "{text}");
}

#[test]
fn irules_namespace_backed_profiles_injected_into_hover() {
    // `DIAMETER::retransmission_default` carries `event_requires` with
    // *empty* profiles; `effective_event_requires` substitutes the
    // `DIAMETER` namespace's profiles, so the hover shows a profile
    // **Requires** line (none would appear without the injection).
    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));
    let text = builtin_command_hover_text(
        &registry,
        "DIAMETER::retransmission_default",
        &analyse(""),
        u32::MAX,
    )
    .expect("hover");
    assert!(
        text.contains("**Requires**: profile DIAMETER or DIAMETERSESSION or DIAMETER_ENDPOINT"),
        "{text}"
    );
}

#[test]
fn effective_event_requires_injects_namespace_profiles() {
    // Unit check: empty own-profiles + a known `NS::` prefix → the
    // namespace's profiles; a non-namespaced name is unchanged.
    let base = tcl_registry::events::EventRequires {
        client_side: false,
        server_side: false,
        transport: None,
        profiles: &[],
        also_in: &[],
        flow: false,
    };
    let descriptor = |name| tcl_registry::CommandSpec {
        name,
        surface: Some(tcl_dialect::model::SpecSurface::IRULES),
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let eff = super::super::effective_event_requires(&descriptor("DIAMETER::foo"), &base);
    assert_eq!(
        eff.profiles,
        &["DIAMETER", "DIAMETERSESSION", "DIAMETER_ENDPOINT"]
    );
    // No `::` qualifier → unchanged (still empty).
    assert_eq!(
        super::super::effective_event_requires(&descriptor("puts"), &base).profiles,
        [] as [&str; 0]
    );
    // Already-populated profiles are left as-is.
    let with = tcl_registry::events::EventRequires {
        profiles: &["HTTP"],
        ..base
    };
    assert_eq!(
        super::super::effective_event_requires(&descriptor("HTTP::uri"), &with).profiles,
        &["HTTP"]
    );
}

#[test]
fn builtin_hover_omits_requires_for_core_command() {
    // A core built-in (`lindex`) needs no package — no hint.
    let src = "lindex $l 0\n";
    let analysis = analyse(src);
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 0, 2, &analysis, Some(&registry)).expect("hover");
    assert!(!h.value.contains("**Requires**"), "{}", h.value);
}

// registry-driven hovers

#[test]
fn builtin_command_hover_surfaces_summary_from_registry() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let t = builtin_command_hover_text(&registry, "puts", &analyse(""), u32::MAX).expect("hover");
    assert!(t.contains("built-in command"), "{t}");
    assert!(t.contains("`puts`"), "{t}");
}

#[test]
fn builtin_command_hover_lists_subcommands() {
    let registry = tcl_registry::CommandRegistry::build_default();
    let t = builtin_command_hover_text(&registry, "string", &analyse(""), u32::MAX).expect("hover");
    assert!(t.contains("Subcommands:"), "{t}");
    assert!(t.contains("length"), "{t}");
}

#[test]
fn builtin_command_hover_returns_none_for_unknown() {
    let registry = tcl_registry::CommandRegistry::build_default();
    assert!(
        builtin_command_hover_text(&registry, "totallyMadeUpCommand", &analyse(""), u32::MAX,)
            .is_none()
    );
}

#[test]
fn subcommand_hover_surfaces_for_string_length() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let text =
        retained_registry_text("string length $name\n", 10, "tcl8.6").expect("subcommand hover");
    assert!(text.contains("`string length`"), "{text}");
    assert!(text.contains("subcommand"), "{text}");
}

#[test]
fn subcommand_hover_resolves_unique_prefix_abbreviation() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let text = retained_registry_text("string le $name\n", 8, "tcl8.6").expect("prefix hover");
    assert!(text.contains("`string length`"), "{text}");
}

#[test]
fn subcommand_hover_prefix_is_dialect_aware() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let source = "info class def ::C\n";
    assert!(
        retained_registry_text(source, 13, "tcl8.6")
            .is_some_and(|text| text.contains("`info class definition`"))
    );
    assert!(retained_registry_text(source, 13, "tcl9.0").is_none());
    let source = "string rev abc\n";
    assert!(retained_registry_text(source, 8, "tcl8.6").is_some());
    assert!(retained_registry_text(source, 8, "tcl8.4").is_none());
}

#[test]
fn subcommand_hover_skips_unknown_subcommand() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    assert!(retained_registry_text("string bogusSubcommand\n", 12, "tcl8.6").is_none());
}

#[test]
fn option_hover_resolves_subcommand_scoped_option() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let text = retained_registry_text("chan configure $chan -inputmode raw\n", 26, "tcl9.0")
        .expect("configure-scoped option");
    assert!(text.contains("`-inputmode`"), "{text}");
    assert!(text.contains("chan configure"), "{text}");
}

#[test]
fn option_hover_notes_dialect_unavailability() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let source = "chan configure $chan -inputmode raw\n";
    let old = retained_registry_text(source, 26, "tcl8.6").expect("excluded exact option metadata");
    assert!(old.contains("Not available in the active dialect"), "{old}");
    let new = retained_registry_text(source, 26, "tcl9.0").expect("admitted option");
    assert!(
        !new.contains("Not available in the active dialect"),
        "{new}"
    );
}

#[test]
fn option_hover_falls_back_to_top_level_options_for_simple_command() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let text = retained_registry_text("lsearch -exact {a b} x\n", 11, "tcl8.6")
        .expect("root option metadata");
    assert!(text.contains("`-exact`"), "{text}");
    assert!(text.contains("of `lsearch`"), "{text}");
}

#[test]
fn sub_subcommand_hover_surfaces_for_info_object_class() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let text = retained_registry_text("info object class $obj\n", 14, "tcl8.6")
        .expect("nested subcommand");
    assert!(text.contains("`info object class`"), "{text}");
    assert!(text.contains("subcommand"), "{text}");
    let text =
        retained_registry_text("info class super $cls\n", 13, "tcl8.6").expect("nested prefix");
    assert!(text.contains("`info class superclasses`"), "{text}");
    // The first selector is described by its own retained subcommand row.
    let parent =
        retained_registry_text("info object class\n", 6, "tcl8.6").expect("parent selector");
    assert!(parent.contains("`info object`"), "{parent}");
    assert!(!parent.contains("`info object class`"), "{parent}");
}

#[test]
fn logical_registry_hover_keeps_actual_context_and_source_currency() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    let source = "string cat A B";
    let mut profile = crate::profile_for_dialect("f5-irules").clone();
    profile.name = "explicit-logical-hover-context";
    let profile = profile.intern();
    let driver =
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry();
    for (environment, expected) in [("tcl9.1", true), ("tcl8.4", false)] {
        let actual = tcl_registry::model::ingress::resolve_environment(environment)
            .default_context_registry();
        let context = std::sync::Arc::new(
            actual.with_command_store(driver.commands().snapshot().shared_registry()),
        );
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
        );
        let mut analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, "presentation-only");
        analysis.command_invocations.clear();
        let answer = hover(source, 0, 8, &analysis, None);
        assert_eq!(answer.is_some(), expected, "{environment}: {answer:?}");
        assert!(hover("string cat C D", 0, 8, &analysis, None).is_none());
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::new(
                context.with_command_store(context.commands().snapshot().shared_registry()),
            ),
            input.lexer_config(),
        ));
        assert!(hover(source, 0, 8, &analysis, None).is_none());
    }
}

#[test]
fn logical_registry_hover_preserves_whole_alias_operands_and_declines_data() {
    // naming.core.original-registry-source-hover
    // docs/design/analysis/name-resolution-proofs/original-registry-source-hover.md
    for (source, needle, expected) in [
        (
            "interp alias {} length {} string; length length value",
            "length value",
            "`string length`",
        ),
        (
            "interp alias {} rx {} regexp; rx -nocase pattern text",
            "-nocase",
            "option of `regexp`",
        ),
        (
            "proc p {} {string length value}",
            "length",
            "`string length`",
        ),
        (
            r"string \u006cength value",
            r"\u006cength",
            "`string length`",
        ),
    ] {
        let offset = u32::try_from(source.rfind(needle).unwrap() + 1).unwrap();
        let text = retained_registry_text(source, offset, "tcl8.6").expect(source);
        assert!(text.contains(expected), "{source}: {text}");
    }
    for (source, needle) in [
        (
            "interp alias {} rx {} regexp -nocase; rx pattern -nocase",
            "-nocase",
        ),
        ("puts {string length value}", "length"),
        ("proc string args {}; string length value", "length value"),
        ("string $selector value", "$selector"),
    ] {
        let offset = u32::try_from(source.rfind(needle).unwrap() + 1).unwrap();
        assert!(
            retained_registry_text(source, offset, "tcl8.6").is_none(),
            "{source}"
        );
    }
}

#[test]
fn hover_fires_for_builtin_command_with_registry() {
    let src = "puts hello\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 0, 2, &analysis, Some(&registry)).expect("hover");
    assert!(h.value.contains("built-in command"), "{}", h.value);
}

#[test]
fn hover_fires_for_subcommand_with_registry() {
    let src = "string length $name\n";
    let mut a = tcl_compiler::analyser::Analyser::new();
    let analysis = a.analyse(src, "tcl8.6").clone();
    let registry = tcl_registry::CommandRegistry::build_default();
    let h = hover(src, 0, 10, &analysis, Some(&registry)).expect("hover");
    assert!(h.value.contains("subcommand"), "{}", h.value);
}

#[test]
fn command_profile_metadata_keeps_foreign_surface_and_own_requirements() {
    // naming.analysis.constructed-key-consumer-projections
    // docs/design/analysis/name-resolution-proofs/constructed-key-consumer-projections.md
    let descriptor = |surface| tcl_registry::CommandSpec {
        name: "ACCESS::log",
        surface,
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let mut text = String::new();
    super::super::append_command_event_metadata(
        &mut text,
        &descriptor(Some(tcl_dialect::model::SpecSurface::IRULES)),
        super::super::EventRelease::Unconfigured,
    );
    assert_eq!(text, "\n\n**Requires**: ACCESS profile");
    for surface in [None, Some(tcl_dialect::model::SpecSurface::TCL86)] {
        let mut text = String::new();
        super::super::append_command_event_metadata(
            &mut text,
            &descriptor(surface),
            super::super::EventRelease::Unconfigured,
        );
        assert_eq!(text, "");
    }
    let own = tcl_registry::CommandSpec {
        name: "DIAMETER::fixture",
        surface: Some(tcl_dialect::model::SpecSurface::IRULES),
        event_requires: Some(tcl_registry::events::EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            flow: false,
        }),
        ..tcl_registry::CommandSpec::DEFAULT
    };
    let mut text = String::new();
    super::super::append_command_event_metadata(
        &mut text,
        &own,
        super::super::EventRelease::Unconfigured,
    );
    assert!(text.contains("**Requires**: profile HTTP"), "{text}");
    assert!(!text.contains("profile DIAMETER"), "{text}");
}
