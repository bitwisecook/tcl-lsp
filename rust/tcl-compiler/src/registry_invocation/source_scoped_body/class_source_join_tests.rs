// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The original factory, definition region and complete member argv are one
//! descriptive source join. Global member spellings supply no fallback.

use super::*;
use crate::analyser::Analyser;
use crate::registry_invocation::source_structure::source_registry_words;

fn assert_complete_original_member_words(
    parent: &OriginalSourceScriptBody,
    image: &SourceImage,
    config: tcl_lexer::LexerConfig,
    original: &[tcl_lexer::NativeWord],
) -> tcl_lexer::NativeScriptWordsPlan {
    let plan =
        tcl_lexer::native_script_words_in(image.clone(), parent.content_span(), config).unwrap();
    assert!(plan.fatal_tail.is_none());
    assert_eq!(plan.commands.len(), 1);
    assert_eq!(original, plan.commands[0].words);
    assert_eq!(original.len(), 4);
    assert!(
        original
            .iter()
            .all(|word| word.image() == image && word.config() == config)
    );
    plan
}

fn assert_foreign_class_member_geometry_refused(
    parent: &OriginalSourceScriptBody,
    region: &OriginalSourceDefinitionMemberRegion,
    image: &SourceImage,
    config: tcl_lexer::LexerConfig,
    source: &str,
) {
    let mut foreign_config = config;
    foreign_config.strict_quoting = !config.strict_quoting;
    assert!(!parent.matches_source(image, foreign_config));
    let foreign_image = SourceImage::document(&source.replace("puts α", "puts β"));
    assert!(!parent.matches_source(&foreign_image, config));
    let foreign_plan =
        tcl_lexer::native_script_words_in(foreign_image, parent.content_span(), config).unwrap();
    assert!(
        region
            .script_bodies(&foreign_plan.commands[0].words)
            .is_none()
    );
}

fn assert_class_factory_schema(
    head: &str,
    selector: Option<&str>,
    roles: &[(u8, ArgRole)],
    complete: bool,
    family: Option<tcl_registry::definer::DefinerFamily>,
    dialect: &str,
) {
    assert_eq!(head, "oo::class");
    assert_eq!(selector, Some("create"));
    assert!(complete, "{dialect}: root schema roles {roles:?}");
    assert!(roles.contains(&(1, ArgRole::Body)), "{dialect}: {roles:?}");
    assert_eq!(family, Some(tcl_registry::definer::DefinerFamily::TclOo));
}

#[test]
fn original_class_factory_body_and_member_joins_retain_the_complete_source_input() {

    // Conditional source grammar and exact geometry only. No class creation,
    // entered definition frame, member table or body execution is established.
    for dialect in ["tcl", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for prefix in ["", "interp alias {} method {} if 1\r\n"] {
            let source = format!("{prefix}oo::class create C {{method m {{}} {{puts α}}}}\r\n");
            let analysis = Analyser::new().analyse(&source, dialect);
            let input = analysis.resolved_input.as_ref().unwrap();
            let context = input.context_registry();
            let config = input.lexer_config();
            let image = SourceImage::document(&source);
            assert_eq!(analysis.body_lexer_config, Some(config));
            assert!(analysis.matches_original_source_image(&image, config));

            let factory_at = u32::try_from(source.find("oo::class create").unwrap()).unwrap();
            let command =
                crate::segmenter::segment_commands_with_offset_and_config(&source, 0, config)
                    .into_iter()
                    .find(|command| command.argv[0].span.start() == factory_at)
                    .unwrap();
            let words = source_registry_words(&source, &analysis, &command).unwrap_or_else(|| {
                panic!("{dialect}: root factory source selection; prefix={prefix:?}")
            });
            assert!(words.matches_source(&image, config));
            assert!(words.matches_registry(context.commands()));
            assert_eq!(words.context(), Some(context.context()));
            assert_eq!(words.arguments().len(), 3);
            assert!(
                words
                    .roles()
                    .is_some_and(|roles| roles.contains(&(2, ArgRole::Body))),
                "{dialect}: root factory body roles {:?}; prefix={prefix:?}",
                words.roles(),
            );
            let schema = words.with_source_schema(&context, |schema| {
                let descriptors = schema.authored_source_descriptors();
                (
                    descriptors.command.name,
                    descriptors.subcommand.map(|sub| sub.name),
                    schema.authored_source_argument_roles(),
                    schema
                        .authored_source_definition_body_grammar()
                        .map(|grammar| grammar.family),
                )
            });
            let (head, selector, (roles, complete), family) = schema.unwrap_or_else(|| {
                panic!("{dialect}: root selected source schema; prefix={prefix:?}")
            });
            assert_class_factory_schema(head, selector, &roles, complete, family, dialect);

            let bodies = words.source_script_bodies(&context);
            assert_eq!(
                bodies.len(),
                1,
                "{dialect}: original class body; prefix={prefix:?}"
            );
            let body = &bodies[0];
            assert!(body.matches_source(&image, config));
            assert!(body.matches_context(&context));
            assert_eq!(
                &source[body.content_span().as_range()],
                "method m {} {puts α}",
            );
            let parent = body
                .definition_parent_for(&context, None)
                .unwrap_or_else(|| {
                    panic!("{dialect}: selected definition parent; prefix={prefix:?}")
                });
            assert_eq!(parent, *body);
            let region = parent
                .definition_member_region(&context, parent.content_span())
                .unwrap_or_else(|| panic!("{dialect}: actual member region; prefix={prefix:?}"));
            let method_at = u32::try_from(source.rfind("method m").unwrap()).unwrap();
            let original = region
                .original_command_words_at(method_at)
                .unwrap_or_else(|| {
                    panic!("{dialect}: complete original method argv; prefix={prefix:?}")
                });
            let _plan = assert_complete_original_member_words(&parent, &image, config, original);
            assert_eq!(region.member_keyword(original).unwrap().keyword, "method");
            let member_bodies = region.script_bodies(original).unwrap_or_else(|| {
                panic!("{dialect}: selected original method body layout; prefix={prefix:?}")
            });
            assert_eq!(member_bodies.len(), 1);
            assert_eq!(member_bodies[0].argument(), 2);
            assert_eq!(member_bodies[0].original_words(), original);
            assert_eq!(member_bodies[0].original_container(), &original[3]);
            assert_eq!(
                &source[member_bodies[0].content_span().as_range()],
                "puts α"
            );
            assert!(member_bodies[0].definition_parent().is_none());
            assert!(region.script_bodies(&original[..3]).is_none());

            assert_foreign_class_member_geometry_refused(&parent, &region, &image, config, &source);
        }
    }
}
