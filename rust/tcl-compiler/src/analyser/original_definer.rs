// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conditional original definer selection for canonical declaration producers.

use super::{Analyser, types::AnalysisResult};
use crate::command_binding::OriginalSourceClassDeclaration;
use tcl_lexer::{SourceMap, Token};
use tcl_registry::definer::{DefinerFamily, DefinitionBodyGrammar};

pub(super) struct OriginalClassDefinerSource {
    declaration: OriginalSourceClassDeclaration,
    grammar: &'static DefinitionBodyGrammar,
    arguments: Vec<String>,
    tokens: Vec<Token>,
}

impl OriginalClassDefinerSource {
    fn capture(source: &str, analysis: &AnalysisResult, offset: u32) -> Option<Self> {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        let declaration =
            crate::registry_invocation::source_structure::source_class_declaration_at(
                source, analysis, offset,
            )?;
        let context = analysis.resolved_input.as_ref()?.context_registry();
        let grammar = declaration.grammar(&context)?;
        if grammar.family == DefinerFamily::TclOo
            && !context
                .context()
                .resolve_spec(context.commands(), declaration.factory().command())?
                .traits
                .contains(tcl_registry::Traits::IS_OO_METACLASS)
        {
            return None;
        }
        let mut arguments = Vec::new();
        let mut tokens = Vec::new();
        for argument in declaration.factory().arguments() {
            let word = &argument.original;
            if word.group().expand {
                return None;
            }
            let (value, token) = original_argument_projection(word, argument.value.as_deref()?)?;
            arguments.push(value);
            tokens.push(token);
        }
        Some(Self {
            declaration,
            grammar,
            arguments,
            tokens,
        })
    }

    pub(super) fn native_argument_input(
        &self,
        ordinal: usize,
    ) -> Option<&crate::signature_scan::scope::SignatureSourceNameInput> {
        self.declaration
            .factory()
            .arguments()
            .get(ordinal)?
            .input
            .as_ref()?
            .native_input()
    }

    pub(super) const fn declaration(&self) -> &OriginalSourceClassDeclaration {
        &self.declaration
    }

    pub(super) const fn grammar(&self) -> &'static DefinitionBodyGrammar {
        self.grammar
    }
    pub(super) fn original_name(
        &self,
    ) -> Option<crate::signature_scan::original_name::SourceOriginalNameOccurrence> {
        crate::signature_scan::original_name::SourceOriginalNameOccurrence::from_original_class_factory(&self.declaration)
    }
}

/// Whole original class-call vector selected by the shared source graph.
/// This retains source declaration grammar and metadata, not Native dispatch.
pub(super) struct OriginalClassCallSource {
    pub(super) call: crate::command_binding::OriginalSourceConstructorCall,
    pub(super) grammar: &'static DefinitionBodyGrammar,
    pub(super) arguments: Vec<String>,
    pub(super) tokens: Vec<Token>,
}

impl OriginalClassCallSource {
    pub(super) fn capture(source: &str, analysis: &AnalysisResult, offset: u32) -> Option<Self> {
        let call = crate::registry_invocation::source_structure::source_constructor_call_at(
            source, analysis, offset,
        )?;
        let context = analysis.resolved_input.as_ref()?.context_registry();
        let grammar = call.class_declaration().grammar(&context)?;
        call.class_declaration().source_class(analysis)?;
        let mut arguments = Vec::new();
        let mut tokens = Vec::new();
        for (ordinal, value) in call.arguments().iter().enumerate() {
            let word = call.argument_word(ordinal)?;
            let (value, token) = original_argument_projection(word, value.literal_bytes()?)?;
            arguments.push(value);
            tokens.push(token);
        }
        Some(Self {
            call,
            grammar,
            arguments,
            tokens,
        })
    }
}

fn original_argument_projection(
    word: &tcl_lexer::NativeWord,
    value: &[u8],
) -> Option<(String, Token)> {
    if word.group().expand {
        return None;
    }
    let value = std::str::from_utf8(value).ok()?;
    // Exact unchanged structural geometry never creates a new command entry.
    let mut projection = crate::segmenter::segment_commands_with_offset_and_config(
        word.try_text().ok()?,
        word.word_span().start(),
        word.config(),
    );
    if projection.len() != 1 {
        return None;
    }
    let projected = projection.pop()?;
    if projected.argv.len() != 1 || projected.all_tokens != word.tokens() {
        return None;
    }
    let token = projected.argv[0];
    (tcl_lexer::word_span(&SourceMap::from_image(word.image()), token) == word.word_span())
        .then(|| (value.to_owned(), token))
}

#[derive(Clone, Copy)]
pub(super) struct ClassDefinerCall<'a> {
    pub(super) cmd_name: &'a str,
    pub(super) args: &'a [String],
    pub(super) arg_tokens: &'a [Token],
    pub(super) scope_path: &'a [usize],
    pub(super) cmd_tok: Option<Token>,
    pub(super) original: Option<&'a OriginalClassDefinerSource>,
}

impl Analyser {
    pub(super) fn dispatch_original_class_definer(
        &mut self,
        token: Token,
        scope_path: &[usize],
    ) -> Option<bool> {
        let original =
            OriginalClassDefinerSource::capture(&self.source, &self.result, token.span.start())?;
        let call = ClassDefinerCall {
            cmd_name: original.declaration.factory().command(),
            args: &original.arguments,
            arg_tokens: &original.tokens,
            scope_path,
            cmd_tok: Some(token),
            original: Some(&original),
        };
        Some(match original.grammar.family {
            DefinerFamily::TclOo => self.handle_oo_class_source(call),
            DefinerFamily::Snit => self.handle_snit_type_source(call),
            DefinerFamily::Itcl => self.handle_itcl_class_source(call),
            DefinerFamily::JimClass => self.handle_jim_class_source(call),
            DefinerFamily::SpecTcl | DefinerFamily::SslicTcl => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_class_definers_preserve_moves_captured_prefixes_and_declaration_sites() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        for source in [
            "rename oo::class Maker; Maker create C {method value {} {return ok}}",
            "interp alias {} Maker {} oo::class create; Maker C {method value {} {return ok}}",
            "interp alias {} Maker {} oo::class create C; Maker {method value {} {return ok}}",
            "interp alias {} Maker {} oo::class create C {method value {} {return ok}}; Maker",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.rfind("Maker").unwrap()).unwrap();
            let receipt =
                crate::registry_invocation::source_structure::source_class_declaration_at(
                    source, &analysis, offset,
                )
                .unwrap();
            let canonical = receipt.source_class(&analysis).unwrap();
            assert_eq!(canonical.metadata().qualified_name, "::C", "{source}");
            assert!(
                canonical.metadata().methods.contains_key("value"),
                "{source}"
            );
            assert_eq!(canonical.declaration_site(), receipt.factory().site());
            assert_eq!(
                canonical.name_input().original_word(),
                receipt.name_input().original_word().unwrap()
            );
            assert!(receipt.factory().obligations().contains(
                &crate::command_binding::SourceCommandTransitionObligation::RegisteredFactoryApplicability));
        }
    }

    #[test]
    fn original_class_definers_refuse_known_shadow_delete_and_non_class_grammars() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        for source in [
            "rename oo::class Maker; proc Maker args {}; Maker create C {}",
            "rename oo::class Maker; rename Maker {}; Maker create C {}",
            "interp alias {} Maker {} puts; Maker C {}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl8.6");
            let offset = u32::try_from(source.rfind("Maker").unwrap()).unwrap();
            assert!(
                OriginalClassDefinerSource::capture(source, &analysis, offset).is_none(),
                "{source}"
            );
            assert!(
                analysis
                    .original_class_declarations()
                    .all(|class| class.metadata().qualified_name != "::C")
            );
        }
    }

    #[test]
    fn original_jim_members_accumulate_in_canonical_source_metadata_without_reporting_maps() {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        // naming.class.jim-source-member-chronology-control
        // docs/design/analysis/name-resolution-proofs/class-jim-source-member-chronology-control.md
        // This checks conditional source metadata and source ordering, not a
        // successful class allocation, invocation or method activation.
        use crate::analyser::types::MemberSide;
        let source =
            "class C {x 1}; C method first {} {}; C method second {} {}; proc {C third} {} {}";
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(source, "jim");
        let offset = u32::try_from(source.find("C method second").unwrap()).unwrap();
        let call = OriginalClassCallSource::capture(source, &analysis, offset).unwrap();
        let class = call
            .call
            .class_declaration()
            .source_class(&analysis)
            .unwrap();
        for name in ["first", "second", "third"] {
            assert!(class.metadata().methods.contains_key(name), "{name}");
        }
        let members = class
            .metadata()
            .original_members
            .methods(MemberSide::Instance)
            .unwrap();
        for name in ["first", "second", "third"] {
            assert!(
                members
                    .iter()
                    .any(|method| method.original_name_input().bytes() == name.as_bytes()),
                "{name}"
            );
        }
        let before = class
            .metadata()
            .original_members
            .methods_before_source_call(MemberSide::Instance, call.call.site())
            .unwrap();
        assert_eq!(before.len(), 1);
        assert_eq!(before[0].original_name_input().bytes(), b"first");
        let third = members
            .iter()
            .find(|method| method.original_name_input().bytes() == b"third")
            .unwrap();
        assert!(third.declaration().static_occurrence().is_none());
        assert!(third.parameters_word().is_some());
        assert!(third.body_word().is_some());

        analyser.result = analysis;
        analyser.result.all_classes.clear();
        let token = call.call.original_words()[0].tokens()[0];
        assert!(analyser.handle_jim_class_member_call(
            "irrelevant-report-label",
            &[],
            &[],
            &[],
            token
        ));
        let canonical = call
            .call
            .class_declaration()
            .source_class(&analyser.result)
            .unwrap();
        let retained = canonical
            .metadata()
            .original_members
            .methods(MemberSide::Instance)
            .unwrap();
        for name in ["first", "second", "third"] {
            assert!(
                retained
                    .iter()
                    .any(|method| method.original_name_input().bytes() == name.as_bytes()),
                "{name}"
            );
        }
        assert!(canonical.metadata().methods.contains_key("third"));
    }

    #[test]
    fn original_jim_two_word_members_keep_earlier_and_later_procedure_producers() {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        // naming.class.jim-source-member-chronology-control
        // docs/design/analysis/name-resolution-proofs/class-jim-source-member-chronology-control.md
        use crate::analyser::types::MemberSide;
        let source = "proc {C before} {} {}; class C {x 1}; proc {C after} {} {}";
        let analysis = Analyser::new().analyse(source, "jim");
        let offset = u32::try_from(source.find("class C").unwrap()).unwrap();
        let class = OriginalClassDefinerSource::capture(source, &analysis, offset).unwrap();
        let canonical = class.declaration().source_class(&analysis).unwrap();
        let members = canonical
            .metadata()
            .original_members
            .methods(MemberSide::Instance)
            .unwrap();
        for name in ["before", "after"] {
            let member = members
                .iter()
                .find(|method| method.original_name_input().bytes() == name.as_bytes())
                .unwrap();
            assert!(member.declaration().static_occurrence().is_none());
            let proc_offset =
                u32::try_from(source.find(&format!("proc {{C {name}}}")).unwrap()).unwrap();
            assert_eq!(member.declaration().site().offset, proc_offset);
            assert_ne!(
                member.declaration().site(),
                class.declaration().factory().site()
            );
        }
    }

    #[test]
    fn original_jim_inherited_variables_use_genuine_base_list_children() {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        // Non-ASCII labels are source characters; this checks retained source
        // naming inputs, without a claim about an inherited runtime invocation.
        let source = "class {Base α} {{value β} 1}; class Derived {{Base α}} {own 2}";
        let analysis = Analyser::new().analyse(source, "jim");
        let offset = u32::try_from(source.find("class Derived").unwrap()).unwrap();
        let factory = OriginalClassDefinerSource::capture(source, &analysis, offset).unwrap();
        let bases = factory
            .native_argument_input(1)
            .unwrap()
            .original_list_elements()
            .unwrap();
        assert_eq!(bases.len(), 1);
        assert!(bases[0].original_word_key().is_none());
        let reference = crate::registry_invocation::source_structure::source_class_reference_at(
            source, &analysis, offset, &bases[0],
        )
        .unwrap();
        assert_eq!(reference.name_input(), &bases[0]);
        assert_eq!(
            reference.class_declaration().name_input().bytes(),
            "Base α".as_bytes()
        );
        let derived = factory.declaration().source_class(&analysis).unwrap();
        assert!(
            derived
                .metadata()
                .variables
                .iter()
                .any(|name| name == "value β")
        );
        assert!(
            derived
                .metadata()
                .variables
                .iter()
                .any(|name| name == "own")
        );
    }

    #[test]
    fn original_class_member_metadata_refuses_known_source_command_barriers() {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        for source in [
            "class C {}; rename C {}; proc {C absent} {} {}",
            "class C {}; proc C args {}; C method absent {} {}",
        ] {
            let analysis = Analyser::new().analyse(source, "jim");
            let canonical = analysis.original_class_declarations().next().unwrap();
            assert!(
                !canonical.metadata().methods.contains_key("absent"),
                "{source}"
            );
            assert!(
                !canonical
                    .metadata()
                    .original_members
                    .declarations()
                    .any(|method| method.original_name_input().bytes() == b"absent"),
                "{source}"
            );
        }
    }

    #[test]
    fn original_oo_configuration_updates_the_canonical_member_ledger() {
        // naming.source.original-class-constructor-call
        // docs/design/analysis/name-resolution-proofs/source-original-class-constructor-call.md
        use crate::analyser::types::MemberSide;
        let source = "oo::class create C {}; oo::define C method first {} {}; oo::define C method second {} {}";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        let declaration =
            crate::registry_invocation::source_structure::source_class_declaration_at(
                source, &analysis, 0,
            )
            .unwrap();
        analysis.all_classes.clear();
        let canonical = declaration.source_class(&analysis).unwrap();
        let methods = canonical
            .metadata()
            .original_members
            .methods(MemberSide::Instance)
            .unwrap();
        for name in ["first", "second"] {
            assert!(
                methods
                    .iter()
                    .any(|method| method.original_name_input().bytes() == name.as_bytes()),
                "{name}"
            );
        }
    }
}
