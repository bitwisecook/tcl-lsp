// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original class-reference operands retain producer and consumer separately.

use super::{
    AdviceCell, AdviceGraph, AdviceInvocationContext, OriginalSourceClassDeclaration,
    OriginalSourceCommandTransitionAdvice, OriginalSourceTransitionAdviceTape,
    SourceCommandTransitionObligation, source_procedure::OriginalSourceProcedureBody,
};
use crate::signature_scan::scope::SignatureSourceNameInput;
use std::sync::Arc;
use tcl_core_types::ByteNamespacePath;
use tcl_lexer::{LexerConfig, SourceImage};
use tcl_registry::{definer::DefinerFamily, model::ContextRegistry};

/// Readonly original class-reference syntax, without a class object or relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceClassReference {
    consumer: Arc<OriginalSourceCommandTransitionAdvice>,
    input: SignatureSourceNameInput,
    class: Arc<OriginalSourceClassDeclaration>,
    procedure: Option<Arc<OriginalSourceProcedureBody>>,
    producer_namespace: ByteNamespacePath,
    obligations: Vec<SourceCommandTransitionObligation>,
}
impl OriginalSourceClassReference {
    /// Actual command whose authored source layout selects this reference.
    #[must_use]
    pub fn consumer(&self) -> &OriginalSourceCommandTransitionAdvice {
        &self.consumer
    }
    /// Original readonly list child; no synthetic complete naming word is issued.
    #[must_use]
    pub const fn name_input(&self) -> &SignatureSourceNameInput {
        &self.input
    }
    /// Conditional current source class selected independently of reporting maps.
    #[must_use]
    pub fn class_declaration(&self) -> &OriginalSourceClassDeclaration {
        &self.class
    }
    /// Original procedure producer, distinct from a later adopting factory.
    #[must_use]
    pub fn procedure_declaration(&self) -> Option<&OriginalSourceCommandTransitionAdvice> {
        Some(&self.procedure.as_ref()?.declaration)
    }
    /// Whole original procedure naming operand, when it produced this child.
    #[must_use]
    pub fn procedure_name_input(&self) -> Option<&super::SourceAdviceNameInput> {
        Some(&self.procedure.as_ref()?.name)
    }
    /// Canonical original procedure producer, independently of the later factory.
    #[must_use]
    pub fn source_procedure<'a>(
        &self,
        analysis: &'a crate::analyser::AnalysisResult,
    ) -> Option<
        &'a crate::signature_scan::original_name::SourceDeclarationMetadata<
            crate::analyser::ProcDef,
        >,
    > {
        let input = analysis.resolved_input.as_ref()?;
        let original = self.procedure_name_input()?.original_word()?;
        self.matches_source_context(
            original.image(),
            input.lexer_config(),
            &input.context_registry(),
        )
        .then_some(())?;
        self.procedure.as_ref()?.source_procedure(analysis)
    }
    /// Producer naming scope; a later factory cannot replace this source context.
    #[must_use]
    pub const fn producer_namespace(&self) -> &ByteNamespacePath {
        &self.producer_namespace
    }
    /// Source applicability, unknown mutations and original factory remain explicit.
    #[must_use]
    pub fn obligations(&self) -> &[SourceCommandTransitionObligation] {
        &self.obligations
    }
    /// Complete original images/configurations and selected schemas must agree.
    #[must_use]
    pub fn matches_source_context(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        context: &ContextRegistry,
    ) -> bool {
        self.consumer.matches_source(image, config)
            && self.consumer.matches_context(context)
            && self.class.factory().matches_source(image, config)
            && self.class.factory().matches_context(context)
            && (self.input.original_word_key().is_some_and(|key| {
                key.original_word().image() == image && key.original_word().config() == config
            }) || self
                .input
                .original_static_list_container()
                .is_some_and(|container| {
                    container.parent_word().image() == image
                        && container.parent_word().config() == config
                }))
            && self.procedure.as_ref().is_none_or(|procedure| {
                procedure.declaration.matches_source(image, config)
                    && procedure.declaration.matches_context(context)
            })
    }
}
impl OriginalSourceTransitionAdviceTape {
    pub(crate) fn class_reference(
        &self,
        offset: u32,
        input: &SignatureSourceNameInput,
    ) -> Option<&OriginalSourceClassReference> {
        let (_, receipt) = self
            .class_references
            .get(&offset)?
            .iter()
            .find(|(owned, _)| owned == input)?;
        receipt.as_ref()
    }
    pub(super) fn merge_class_reference(
        &mut self,
        offset: u32,
        input: SignatureSourceNameInput,
        receipt: Option<OriginalSourceClassReference>,
    ) {
        let row = self.class_references.entry(offset).or_default();
        if let Some((_, old)) = row.iter_mut().find(|(owned, _)| owned == &input) {
            if old != &receipt {
                *old = None;
            }
        } else {
            row.push((input, receipt));
        }
    }
    pub(super) fn retain_class_reference(&mut self, receipt: OriginalSourceClassReference) {
        self.merge_class_reference(
            receipt.consumer.site().offset,
            receipt.input.clone(),
            Some(receipt),
        );
    }
}
impl AdviceGraph {
    fn source_reference_class(
        &self,
        input: &SignatureSourceNameInput,
    ) -> Option<Arc<OriginalSourceClassDeclaration>> {
        (self.policy.native()? == input.policy()).then_some(())?;
        let key = self.lookup_head_key(input.bytes())?;
        // A command alias is callable syntax, not the class object held by a
        // readonly class-name operand. Only genuine source-class cells join.
        match self.cells.get(&key)? {
            AdviceCell::SourceClass(class) => Some(Arc::clone(class)),
            _ => None,
        }
    }
}
impl AdviceInvocationContext<'_> {
    pub(super) fn retain_source_class_operand_references(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        class: Option<&Arc<OriginalSourceClassDeclaration>>,
        procedure: Option<&Arc<OriginalSourceProcedureBody>>,
    ) {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        if let Some(class) = class {
            self.retain_source_base_class_references(tape, graph, class);
        }
        if let Some(procedure) = procedure
            && let Some(input) = two_word_procedure_class(procedure)
            && let Some(class) = graph.source_reference_class(&input)
            && class
                .grammar(self.context)
                .is_some_and(|grammar| grammar.family == DefinerFamily::JimClass)
        {
            let receipt = Self::source_class_reference(
                graph,
                Arc::clone(&procedure.declaration),
                input,
                class,
                Some(Arc::clone(procedure)),
            );
            tape.retain_class_reference(receipt);
        }
    }
    fn retain_source_base_class_references(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &AdviceGraph,
        class: &Arc<OriginalSourceClassDeclaration>,
    ) {
        let Some(grammar) = class.grammar(self.context) else {
            return;
        };
        let factory = class.factory();
        let Some(at) = grammar.source_base_class_list_argument(factory.arguments.len()) else {
            return;
        };
        let Some(elements) = factory
            .arguments
            .get(at)
            .and_then(|word| word.input.as_ref())
            .and_then(super::SourceAdviceNameInput::native_input)
            .and_then(SignatureSourceNameInput::original_list_elements)
        else {
            return;
        };
        for input in elements {
            if let Some(base) = graph.source_reference_class(&input) {
                let receipt = Self::source_class_reference(
                    graph,
                    Arc::new(factory.clone()),
                    input,
                    base,
                    None,
                );
                tape.retain_class_reference(receipt);
            }
        }
    }
    fn install_source_two_word_factory_initialisers(
        &self,
        graph: &mut AdviceGraph,
        class: Option<&Arc<OriginalSourceClassDeclaration>>,
    ) {
        // naming.class.jim-source-factory-initialiser-replacement
        // docs/design/analysis/name-resolution-proofs/class-jim-source-factory-initialiser-replacement.md
        let Some(class) = class else {
            return;
        };
        let Some(grammar) = class.grammar(self.context) else {
            return;
        };
        let Some(initialisers) =
            grammar.source_two_word_factory_initialisers(class.factory().dialect())
        else {
            return;
        };
        let base_count =
            match grammar.source_base_class_list_argument(class.factory().arguments.len()) {
                None => Some(0),
                Some(at) => class
                    .factory()
                    .arguments
                    .get(at)
                    .and_then(|word| word.input.as_ref())
                    .and_then(super::SourceAdviceNameInput::native_input)
                    .and_then(SignatureSourceNameInput::original_list_elements)
                    .map(|elements| elements.len()),
            };
        let Some(policy) = graph.policy.native() else {
            return;
        };
        for initialiser in initialisers {
            if initialiser.presence.replaces_source_slot(base_count) == Some(false) {
                continue;
            }
            let Some(context) = graph.native_source_context() else {
                return;
            };
            if let Ok(slot) = policy.recipe().jim_two_word_member_publication_slot(
                context,
                class.name_input().bytes(),
                initialiser.name.as_bytes(),
                initialiser.publication,
            ) {
                // Unknown BASES also withdraws the earlier source header;
                // this barrier never claims that the factory actually ran.
                graph.cells.insert(slot, AdviceCell::Shadowed);
            }
        }
    }

    pub(super) fn retain_source_class_adoptions(
        &self,
        tape: &mut OriginalSourceTransitionAdviceTape,
        graph: &mut AdviceGraph,
        class: Option<&Arc<OriginalSourceClassDeclaration>>,
    ) {
        self.install_source_two_word_factory_initialisers(graph, class);
        let Some(class) = class else {
            return;
        };
        if !class
            .grammar(self.context)
            .is_some_and(|grammar| grammar.family == DefinerFamily::JimClass)
        {
            return;
        }
        for cell in graph.cells.values() {
            let AdviceCell::SourceProcedure(procedure) = cell else {
                continue;
            };
            let Some(input) = two_word_procedure_class(procedure) else {
                continue;
            };
            let Some(original_slot) = super::lookup_key(&graph.policy, procedure.name.bytes())
            else {
                continue;
            };
            if original_slot != procedure.slot {
                continue;
            }
            let Some(key) = super::lookup_key(&graph.policy, input.bytes()) else {
                continue;
            };
            if key != *class.source_slot() {
                continue;
            }
            let receipt = Self::source_class_reference(
                graph,
                Arc::new(class.factory().clone()),
                input,
                Arc::clone(class),
                Some(Arc::clone(procedure)),
            );
            tape.retain_class_reference(receipt);
        }
    }
    pub(super) fn source_class_reference(
        graph: &AdviceGraph,
        consumer: Arc<OriginalSourceCommandTransitionAdvice>,
        input: SignatureSourceNameInput,
        class: Arc<OriginalSourceClassDeclaration>,
        procedure: Option<Arc<OriginalSourceProcedureBody>>,
    ) -> OriginalSourceClassReference {
        let producer_namespace = procedure.as_ref().map_or_else(
            || {
                graph
                    .procedure_namespace
                    .clone()
                    .unwrap_or_else(ByteNamespacePath::root)
            },
            |procedure| procedure.declaration_namespace.clone(),
        );
        let mut obligations = consumer.obligations.clone();
        for obligation in class.factory().obligations().iter().chain(
            procedure
                .iter()
                .flat_map(|procedure| procedure.declaration.obligations()),
        ) {
            if !obligations.contains(obligation) {
                obligations.push(obligation.clone());
            }
        }
        OriginalSourceClassReference {
            consumer,
            input,
            class,
            procedure,
            producer_namespace,
            obligations,
        }
    }
}
fn two_word_procedure_class(
    procedure: &OriginalSourceProcedureBody,
) -> Option<SignatureSourceNameInput> {
    let elements = procedure.name.native_input()?.original_list_elements()?;
    (elements.len() == 2).then(|| elements[0].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        analyser::Analyser,
        registry_invocation::source_structure::{
            source_class_declaration_at, source_class_reference_at,
        },
    };

    fn at(source: &str, marker: &str) -> u32 {
        u32::try_from(source.find(marker).unwrap()).unwrap()
    }
    fn child_at(
        source: &str,
        offset: u32,
        ordinal: usize,
        child: usize,
    ) -> SignatureSourceNameInput {
        let profile = tcl_dialect::DialectProfile::find("jim").unwrap();
        let config = LexerConfig::for_file_grammar(profile.grammar);
        let image = SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in(
            image.clone(),
            tcl_lexer::Span::new(offset, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let word = &plan.commands[0].words[ordinal];
        let policy = tcl_registry::InvocationDialect::of_profile(profile)
            .authored_name_policy()
            .unwrap();
        SignatureSourceNameInput::OriginalWord(
            crate::signature_scan::scope::SignatureSourceNameKey::from_original_native_word(
                word,
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
                policy,
            )
            .unwrap(),
        )
        .original_list_elements()
        .unwrap()[child]
            .clone()
    }

    #[test]
    fn original_class_reference_keeps_genuine_base_list_children_and_current_moves() {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        let source = "class Base {}; rename Base Held; unavailable; class Child {Held} {}";
        let analysis = Analyser::new().analyse(source, "jim");
        let offset = at(source, "class Child");
        let child = child_at(source, offset, 2, 0);
        let receipt = source_class_reference_at(source, &analysis, offset, &child).unwrap();
        assert_eq!(receipt.name_input().bytes(), b"Held");
        assert_eq!(receipt.class_declaration().name_input().bytes(), b"Base");
        assert_eq!(
            receipt.class_declaration().source_slot().simple.as_bytes(),
            b"Held"
        );
        assert_eq!(
            receipt
                .class_declaration()
                .source_class(&analysis)
                .unwrap()
                .name_input()
                .bytes(),
            b"Base"
        );
        assert!(receipt.procedure_declaration().is_none());
        assert!(
            receipt
                .obligations()
                .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
        );
        assert_eq!(receipt.consumer().site().offset, offset);
        assert!(
            source_class_reference_at(
                &source.replace("Held}", "Other}"),
                &analysis,
                offset,
                &child
            )
            .is_none()
        );
    }
    #[test]
    fn original_class_reference_keeps_earlier_procedure_producer_separate_from_factory() {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        let source = "proc {C before} {} {}; class C {}; proc {C after} {} {}";
        let analysis = Analyser::new().analyse(source, "jim");
        let factory_at = at(source, "class C");
        let before = child_at(source, 0, 1, 0);
        let adopted = source_class_reference_at(source, &analysis, factory_at, &before).unwrap();
        assert_eq!(adopted.consumer().site().offset, factory_at);
        assert_eq!(adopted.procedure_declaration().unwrap().site().offset, 0);
        assert_eq!(adopted.procedure_name_input().unwrap().bytes(), b"C before");
        assert!(adopted.producer_namespace().is_root());
        let after_at = at(source, "proc {C after}");
        let after = child_at(source, after_at, 1, 0);
        let current = source_class_reference_at(source, &analysis, after_at, &after).unwrap();
        assert_eq!(
            current.procedure_declaration().unwrap().site().offset,
            after_at
        );
        assert_eq!(current.class_declaration(), adopted.class_declaration());
        assert!(source_class_reference_at(source, &analysis, after_at, &before).is_none());
        assert_eq!(
            source_class_declaration_at(source, &analysis, factory_at).unwrap(),
            *adopted.class_declaration()
        );
    }
    #[test]
    fn original_class_reference_refuses_alias_objects_known_shadow_delete_and_moved_proc_adoption()
    {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        for source in [
            "class Base {}; interp alias {} Alias {} Base; class Child {Alias} {}",
            "class Base {}; rename Base {}; class Child {Base} {}",
            "class Base {}; proc Base args {}; class Child {Base} {}",
        ] {
            let analysis = Analyser::new().analyse(source, "jim");
            let offset = at(source, "class Child");
            let child = child_at(source, offset, 2, 0);
            assert!(
                source_class_reference_at(source, &analysis, offset, &child).is_none(),
                "{source}"
            );
        }
        let source = "proc {C before} {} {}; rename {C before} held; class C {}";
        let analysis = Analyser::new().analyse(source, "jim");
        assert!(
            source_class_reference_at(
                source,
                &analysis,
                at(source, "class C"),
                &child_at(source, 0, 1, 0)
            )
            .is_none()
        );
    }
    #[test]
    fn original_class_factory_initialisers_withdraw_earlier_members_and_callback_headers() {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        // Native observation: naming.class.jim-source-factory-initialiser-replacement.
        // docs/design/analysis/name-resolution-proofs/class-jim-source-factory-initialiser-replacement.md
        // These assertions check conditional source carriers, not successful execution.
        for member in [
            "new",
            "finalize",
            "method",
            "vars",
            "classvars",
            "classname",
            "methods",
            "defaultconstructor",
            "constructor",
            "destroy",
            "get",
            "eval",
        ] {
            let source = format!(
                "proc {{C {member}}} {{a b}} {{return ORIGINAL}}; class C {{}}; lsort -command {{C {member}}} {{3 1 2}}"
            );
            let analysis = Analyser::new().analyse(&source, "jim");
            let factory_at = at(&source, "class C");
            let declaration = source_class_declaration_at(&source, &analysis, factory_at).unwrap();
            assert!(
                source_class_reference_at(
                    &source,
                    &analysis,
                    factory_at,
                    &child_at(&source, 0, 1, 0),
                )
                .is_none(),
                "{member}"
            );
            let class = declaration.source_class(&analysis).unwrap().metadata();
            assert!(
                !class
                    .original_members
                    .methods(crate::analyser::MemberSide::Instance)
                    .unwrap()
                    .iter()
                    .any(|method| method.original_name_input().bytes() == member.as_bytes()),
                "{member}"
            );
            let prefix = analysis
                .command_invocations
                .iter()
                .filter_map(|call| call.original_callback_prefix.as_deref())
                .find(|prefix| prefix.name_input().bytes() == format!("C {member}").as_bytes())
                .unwrap();
            let lookup =
                crate::registry_invocation::source_structure::source_callback_procedure_target_at(
                    &source,
                    &analysis,
                    at(&source, "lsort -command"),
                    prefix,
                )
                .unwrap();
            assert_eq!(
                lookup.refusal(),
                Some(super::super::OriginalSourceCallbackProcedureRefusal::KnownSourceBarrier)
            );
            assert!(!lookup.permits_external_signature_lookup());
        }
        let source = "proc {C get} {} {return OLD}; class C {}; proc {C get} {} {return CUSTOM}";
        let analysis = Analyser::new().analyse(source, "jim");
        let later_at = at(source, "proc {C get} {} {return CUSTOM}");
        let reference = source_class_reference_at(
            source,
            &analysis,
            later_at,
            &child_at(source, later_at, 1, 0),
        )
        .unwrap();
        assert_eq!(
            reference
                .source_procedure(&analysis)
                .unwrap()
                .declaration_site()
                .offset,
            later_at
        );
        assert!(
            reference
                .class_declaration()
                .source_class(&analysis)
                .unwrap()
                .metadata()
                .original_members
                .methods(crate::analyser::MemberSide::Instance)
                .unwrap()
                .iter()
                .any(|method| method.original_name_input().bytes() == b"get")
        );
    }

    #[test]
    fn original_class_factory_baseclass_replacement_keeps_exact_source_operand_condition() {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        // naming.class.jim-source-factory-initialiser-replacement
        // docs/design/analysis/name-resolution-proofs/class-jim-source-factory-initialiser-replacement.md
        for (factory, retained) in [
            ("class C {}", true),
            ("class C {} {}", true),
            ("class C {Base} {}", false),
            ("class C $bases {}", false),
        ] {
            let source = format!("proc {{C baseclass}} {{}} {{return ORIGINAL}}; {factory}");
            let analysis = Analyser::new().analyse(&source, "jim");
            let receipt = source_class_reference_at(
                &source,
                &analysis,
                at(&source, "class C"),
                &child_at(&source, 0, 1, 0),
            );
            assert_eq!(receipt.is_some(), retained, "{factory}");
        }
    }

    #[test]
    fn original_class_factory_initialiser_descriptor_keeps_provider_release_and_family() {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        // Release-window assertions are Registry contracts, not native old-Jim observations.
        use tcl_dialect::model::{DialectPoint, Release};
        let source = "class C {}";
        let analysis = Analyser::new().analyse(source, "jim");
        let declaration = source_class_declaration_at(source, &analysis, 0).unwrap();
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let grammar = declaration.grammar(&context).unwrap();
        for (release, constructor_support) in [
            (Release::JIM_0_81, false),
            (Release::JIM_0_82, true),
            (Release::JIM_0_84, true),
        ] {
            let dialect =
                tcl_registry::InvocationDialect::of_point(DialectPoint::canonical(release));
            let descriptors = grammar
                .source_two_word_factory_initialisers(dialect)
                .unwrap();
            assert_eq!(
                descriptors.iter().any(|d| d.name == "constructor"),
                constructor_support
            );
            assert_eq!(
                descriptors.iter().any(|d| d.name == "defaultconstructor"),
                constructor_support
            );
            if let Some(constructor) = descriptors.iter().find(|d| d.name == "constructor") {
                assert_eq!(
                    constructor.publication,
                    tcl_syntax::naming::NativeNamePurpose::AliasPublication
                );
            }
            let base = descriptors.iter().find(|d| d.name == "baseclass").unwrap();
            assert_eq!(
                base.publication,
                tcl_syntax::naming::NativeNamePurpose::CommandPublication
            );
            assert_eq!(base.presence.replaces_source_slot(None), None);
            assert_eq!(base.presence.replaces_source_slot(Some(0)), Some(false));
            assert_eq!(base.presence.replaces_source_slot(Some(1)), Some(true));
        }
        let mut unpinned = declaration.factory().dialect();
        unpinned.core_point = None;
        assert!(
            grammar
                .source_two_word_factory_initialisers(unpinned)
                .is_none()
        );
        for version in [
            tcl_dialect::TclVersion::V8_4,
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            assert!(
                grammar
                    .source_two_word_factory_initialisers(
                        tcl_registry::InvocationDialect::for_version(version)
                    )
                    .is_none()
            );
        }
        assert!(
            tcl_registry::definer::TCLOO_GRAMMAR
                .source_two_word_factory_initialisers(declaration.factory().dialect())
                .is_none()
        );
    }
    #[test]
    fn original_class_factory_initialisers_keep_alias_and_procedure_publication_distinct() {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        // naming.class.jim-source-rooted-factory-initialiser-replacement
        // docs/design/analysis/name-resolution-proofs/class-jim-source-rooted-factory-initialiser-replacement.md
        // This tests conditional source barriers, not successful factory execution.
        let source = "proc {::C constructor} {} {return ORIGINAL}; class ::C {}";
        let analysis = Analyser::new().analyse(source, "jim");
        assert!(
            source_class_reference_at(
                source,
                &analysis,
                at(source, "class ::C"),
                &child_at(source, 0, 1, 0),
            )
            .is_none()
        );
        let source = "proc {::C get} {} {return ORIGINAL}; class ::C {}";
        let analysis = Analyser::new().analyse(source, "jim");
        assert!(
            source_class_reference_at(
                source,
                &analysis,
                at(source, "class ::C"),
                &child_at(source, 0, 1, 0)
            )
            .is_none()
        );
    }
    #[test]
    fn original_class_factory_initialiser_alias_namespace_scope_stays_separate_from_table_key() {
        // naming.source.original-class-reference
        // docs/design/analysis/name-resolution-proofs/source-original-class-reference.md
        // Pure naming-owner scope coverage; no native namespace execution claim.
        let jim = tcl_syntax::naming::NativeNameProtocol::Jim084;
        let namespace = ByteNamespacePath::from_segments(["n"]);
        let context = tcl_syntax::naming::NativeNameContext::with_jim_namespace(&namespace, b"n");
        let alias = jim
            .jim_two_word_member_publication_slot(
                context,
                b"C",
                b"constructor",
                tcl_syntax::naming::NativeNamePurpose::AliasPublication,
            )
            .unwrap();
        let procedure = jim
            .jim_two_word_member_publication_slot(
                context,
                b"C",
                b"get",
                tcl_syntax::naming::NativeNamePurpose::CommandPublication,
            )
            .unwrap();
        assert_eq!(alias.simple.as_bytes(), b"C constructor");
        assert_eq!(procedure.simple.as_bytes(), b"n::C get");
    }
}
