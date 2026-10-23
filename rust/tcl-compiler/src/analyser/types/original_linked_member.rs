// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conditional original `TclOO` link descriptions and their member declarations.

use super::{AnalysisResult, ClassDef, MemberSide, OriginalSourceMethodMetadata, Scope};
use crate::{
    analyser::ResolvedAnalysisInput,
    registry_invocation::source_structure::{OriginalRegistryWords, source_registry_words_at},
    signature_scan::{
        original_name::{OriginalSourceNamePurpose, SourceDeclarationMetadata},
        scope::SignatureSourceNameInput,
    },
};
use tcl_lexer::{NativeWord, SourceImage, Span};
use tcl_syntax::{
    naming::{native_oo_link_alias_geometry, native_oo_link_call_geometry},
    word_rules::WordValueRules,
};

/// Original alias and target fields under an authentic selected link schema.
/// This is conditional source advice, without an entered receiver or alias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceLinkedMember {
    class: SourceDeclarationMetadata<ClassDef>,
    target: OriginalSourceMethodMetadata,
    invocation: OriginalRegistryWords,
    alias: SignatureSourceNameInput,
    target_input: SignatureSourceNameInput,
    target_span: Option<Span>,
    coupled: bool,
    receiver_side: MemberSide,
}

impl OriginalSourceLinkedMember {
    /// Genuine source class publication owning the original member table.
    #[must_use]
    pub const fn class_declaration(&self) -> &SourceDeclarationMetadata<ClassDef> {
        &self.class
    }
    /// Exact current source member; its name is literal method data.
    #[must_use]
    pub const fn target(&self) -> &OriginalSourceMethodMetadata {
        &self.target
    }
    /// Selected original helper invocation and all retained source obligations.
    #[must_use]
    pub const fn original_invocation(&self) -> &OriginalRegistryWords {
        &self.invocation
    }
    /// Alias and target share one original list field only in the one-element form.
    #[must_use]
    pub const fn has_coupled_name(&self) -> bool {
        self.coupled
    }
    /// Readonly source extent of the target; it alone is not replacement permission.
    #[must_use]
    pub const fn target_source_span(&self) -> Option<Span> {
        self.target_span
    }
    /// A current static list field whose literal bytes survive a safe-name
    /// replacement. Decoded or evaluated containers supply no editable span.
    #[must_use]
    pub fn editable_target_span(&self, source: &str) -> Option<Span> {
        let span = self.target_span?;
        let container = self.target_input.original_static_list_container()?;
        let parent = container.parent_word();
        let image = SourceImage::document(source);
        (parent.image() == &image
            && self.invocation.matches_source(&image, parent.config())
            && source.as_bytes().get(span.as_range()) == Some(self.target_input.bytes()))
        .then_some(span)
    }
    /// Original target field, retaining static list ancestry for edit validation.
    #[must_use]
    pub const fn target_input(&self) -> &SignatureSourceNameInput {
        &self.target_input
    }
    /// Original alias field; command publication and method data stay separate.
    #[must_use]
    pub const fn alias_input(&self) -> &SignatureSourceNameInput {
        &self.alias
    }
}

fn scope_owner<'a>(
    scope: &'a Scope,
    analysis: &'a AnalysisResult,
    image: &SourceImage,
) -> Option<(
    &'a SourceDeclarationMetadata<ClassDef>,
    &'a NativeWord,
    &'a ResolvedAnalysisInput,
    MemberSide,
)> {
    if let Some(context) = &scope.original_member_context {
        if !context.matches_source(image, context.resolved_input().lexer_config()) {
            return None;
        }
        return Some((
            context.source_class(analysis)?,
            context.body_word(),
            context.resolved_input(),
            context.side(),
        ));
    }
    let context = scope.original_receiver_body_declaration.as_ref()?;
    let input = analysis.resolved_input.as_ref()?;
    if !context.matches_source(image, input) {
        return None;
    }
    let mut classes = analysis.original_class_declarations().filter(|class| {
        class
            .metadata()
            .original_members
            .declarations()
            .any(|method| method == context.method())
    });
    let class = classes.next()?;
    if classes.next().is_some() {
        return None;
    }
    Some((
        class,
        context.body_word(),
        context.resolved_input(),
        context.side(),
    ))
}

fn original_body_commands(
    body: &NativeWord,
    input: &ResolvedAnalysisInput,
) -> Option<Vec<tcl_lexer::NativeScriptCommandWords>> {
    let root = body.content_span().ok()?;
    let mut pending = vec![root];
    let mut visited = std::collections::HashSet::new();
    let mut commands = Vec::new();
    while let Some(region) = pending.pop() {
        if !visited.insert(region) {
            continue;
        }
        if region.start() < root.start() || region.end() > root.end() {
            return None;
        }
        let plan = tcl_lexer::native_script_words_in_with_purpose(
            body.image().clone(),
            region,
            input.lexer_config(),
            tcl_lexer::SourceSegmentationPurpose::NestedScript,
        )
        .ok()?;
        if plan.fatal_tail.is_some() {
            return None;
        }
        for command in plan.commands {
            for word in &command.words {
                for part in word.executable_parts().all_parts() {
                    if let tcl_lexer::ExecutablePart::Command { body } = part.part {
                        pending.push(body);
                    }
                }
            }
            commands.push(command);
        }
    }
    Some(commands)
}

fn scope_links(
    scope: &Scope,
    analysis: &AnalysisResult,
    source: &str,
) -> Option<Vec<OriginalSourceLinkedMember>> {
    let image = SourceImage::document(source);
    let Some((class, body, input, side)) = scope_owner(scope, analysis, &image) else {
        return Some(Vec::new());
    };
    let Some(publication) = class.metadata().source_name.as_ref() else {
        return Some(Vec::new());
    };
    let commands = original_body_commands(body, input)?;
    let registry = input.context_registry();
    let mut links = Vec::new();
    for command in commands {
        let head = command.words.first()?;
        let Some(invocation) = source_registry_words_at(source, analysis, head.span().start())
        else {
            continue;
        };
        if invocation.with_source_schema(&registry, |schema| {
            schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::TCLOO_BINDS_METHOD_ALIAS)
        }) != Some(true)
        {
            continue;
        }
        for index in 0..invocation.arguments().len() {
            let parent = invocation.original_argument_value_input(index)?;
            let fields = parent.original_list_elements_with_source_spans()?;
            let (alias, target, coupled) = match fields.as_slice() {
                [one] => (one, one, true),
                [alias, target] => (alias, target, false),
                _ => return None,
            };
            if alias.0.policy() != publication.policy()
                || native_oo_link_alias_geometry(alias.0.policy().recipe(), alias.0.bytes())
                    .is_none()
            {
                return None;
            }
            let method = class
                .metadata()
                .original_members
                .method_for_input(side, &target.0)?;
            let method = if side == MemberSide::Instance && method.native_class_delegate() {
                let delegated = class
                    .metadata()
                    .original_members
                    .method_for_input(MemberSide::ClassObject, &target.0)?;
                if !delegated.native_class_delegate()
                    || delegated.declaration() != method.declaration()
                    || delegated.parameters_word() != method.parameters_word()
                    || delegated.body_word() != method.body_word()
                {
                    return None;
                }
                delegated
            } else {
                method
            };
            links.push(OriginalSourceLinkedMember {
                class: class.clone(),
                target: method,
                invocation: invocation.clone(),
                alias: alias.0.clone(),
                target_input: target.0.clone(),
                target_span: target.1,
                coupled,
                receiver_side: side,
            });
        }
    }
    Some(links)
}

/// Original link descriptions for one actual class declaration. Unknown list
/// values poison the class's source alias advice instead of donating absence.
#[must_use]
pub fn original_source_linked_members_for_class(
    analysis: &AnalysisResult,
    source: &str,
    class: &SourceDeclarationMetadata<ClassDef>,
) -> Option<Vec<OriginalSourceLinkedMember>> {
    fn visit(
        scope: &Scope,
        analysis: &AnalysisResult,
        source: &str,
        class: &SourceDeclarationMetadata<ClassDef>,
        output: &mut Vec<OriginalSourceLinkedMember>,
    ) -> Option<()> {
        if let Some((owner, _, _, _)) = scope_owner(scope, analysis, &SourceImage::document(source))
            && owner.name() == class.name()
        {
            output.extend(scope_links(scope, analysis, source)?);
        }
        for child in &scope.children {
            visit(child, analysis, source, class, output)?;
        }
        Some(())
    }
    if !analysis
        .original_class_declarations()
        .any(|current| current == class)
    {
        return None;
    }
    let mut output = Vec::new();
    visit(&analysis.global_scope, analysis, source, class, &mut output)?;
    Some(output)
}

/// Select only an original command head in a genuine member body. Ambiguous
/// source aliases decline; literal operands and reporting maps cannot select.
#[must_use]
pub fn original_source_linked_member_at(
    analysis: &AnalysisResult,
    source: &str,
    cursor: u32,
) -> Option<OriginalSourceLinkedMember> {
    fn scope_at(scope: &Scope, cursor: u32) -> &Scope {
        scope
            .children
            .iter()
            .find(|child| {
                child
                    .body_span
                    .is_some_and(|span| span.start() <= cursor && cursor < span.end())
            })
            .map_or(scope, |child| scope_at(child, cursor))
    }
    let scope = scope_at(&analysis.global_scope, cursor);
    let image = SourceImage::document(source);
    let (class, body, input, side) = scope_owner(scope, analysis, &image)?;
    let publication = class.metadata().source_name.as_ref()?;
    let command = original_body_commands(body, input)?
        .into_iter()
        .find(|command| {
            command
                .words
                .first()
                .is_some_and(|head| head.span().start() <= cursor && cursor < head.span().end())
        })?;
    let words = command.words;
    let key = OriginalSourceNamePurpose::for_analysis_input(input, None, publication.policy())
        .key(
            words.first()?,
            WordValueRules::from_config(&input.lexer_config()),
        )?;
    let called = native_oo_link_call_geometry(publication.policy().recipe(), key.bytes())?;
    let mut candidates = original_source_linked_members_for_class(analysis, source, class)?
        .into_iter()
        .filter(|link| {
            link.class.name() == class.name()
                && link.receiver_side == side
                && native_oo_link_alias_geometry(link.alias.policy().recipe(), link.alias.bytes())
                    == Some(called.clone())
        });
    let selected = candidates.next()?;
    for candidate in candidates {
        if candidate.target != selected.target
            || candidate.coupled != selected.coupled
            || candidate.target_input != selected.target_input
        {
            return None;
        }
    }
    Some(selected)
}

/// Editable original link targets and coupled calls for one actual member.
/// Unknown list ancestry or a publication that changes the call spelling
/// declines the complete edit set rather than returning partial references.
#[must_use]
pub fn original_source_linked_member_reference_spans(
    analysis: &AnalysisResult,
    source: &str,
    class: &ClassDef,
    method: &super::MethodDef,
    side: MemberSide,
) -> Option<Vec<Span>> {
    let Some(publication) = class.source_name.as_ref() else {
        return Some(Vec::new());
    };
    if publication.publication()
        != crate::signature_scan::scope::SourceCommandPublication::TclOoObject
    {
        return Some(Vec::new());
    }
    let mut classes = analysis.original_class_declarations().filter(|record| {
        record.name() == publication && record.metadata().name_span == class.name_span
    });
    let record = classes.next()?;
    if classes.next().is_some() {
        return None;
    }
    let mut members = record
        .metadata()
        .original_members
        .methods(side)?
        .into_iter()
        .filter(|member| {
            member.metadata().name_span == method.name_span
                && member.metadata().body_span == method.body_span
        });
    let member = members.next()?;
    if members.next().is_some() {
        return None;
    }
    let links = original_source_linked_members_for_class(analysis, source, record)?;
    let mut spans = Vec::new();
    for link in links.iter().filter(|link| link.target == member) {
        spans.push(link.editable_target_span(source)?);
        if link.coupled
            && native_oo_link_alias_geometry(link.alias.policy().recipe(), link.alias.bytes())
                != native_oo_link_call_geometry(link.alias.policy().recipe(), link.alias.bytes())
        {
            return None;
        }
    }
    fn visit(
        scope: &Scope,
        analysis: &AnalysisResult,
        source: &str,
        class: &SourceDeclarationMetadata<ClassDef>,
        member: &OriginalSourceMethodMetadata,
        spans: &mut Vec<Span>,
    ) -> Option<()> {
        if let Some((owner, body, input, _side)) =
            scope_owner(scope, analysis, &SourceImage::document(source))
            && owner.name() == class.name()
        {
            for command in original_body_commands(body, input)? {
                let head = command.words.first()?;
                let Some(link) =
                    original_source_linked_member_at(analysis, source, head.span().start())
                else {
                    continue;
                };
                if link.target != *member || !link.coupled {
                    continue;
                }
                let key =
                    OriginalSourceNamePurpose::for_analysis_input(input, None, link.alias.policy())
                        .key(head, WordValueRules::from_config(&input.lexer_config()))?;
                let span = head.content_span().ok()?;
                if key.bytes() != link.alias.bytes()
                    || source.as_bytes().get(span.as_range()) != Some(key.bytes())
                {
                    return None;
                }
                spans.push(span);
            }
        }
        for child in &scope.children {
            visit(child, analysis, source, class, member, spans)?;
        }
        Some(())
    }
    visit(
        &analysis.global_scope,
        analysis,
        source,
        record,
        &member,
        &mut spans,
    )?;
    spans.sort_by_key(|span| (span.start(), span.end()));
    spans.dedup();
    Some(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn linked_method_navigation_retains_source_schema_and_literal_target_data() {
        let source = "oo::class create C {method :target {} {}; constructor {} {link {:shortcut :target}}; method use {} {shortcut; :shortcut; return shortcut}}";
        for dialect in ["tcl9.0", "tcl9.1"] {
            let mut analysis = Analyser::new().analyse(source, dialect);
            analysis.all_classes.clear();
            let cursor = u32::try_from(source.find("{shortcut;").unwrap() + 1).unwrap();
            let selected =
                original_source_linked_member_at(&analysis, source, cursor).expect(dialect);
            assert_eq!(selected.target().original_name_input().bytes(), b":target");
            assert_eq!(selected.alias_input().bytes(), b":shortcut");
            assert!(!selected.has_coupled_name());
            assert!(selected.editable_target_span(source).is_some());
            assert!(original_source_linked_member_at(&analysis, source, cursor + 10).is_none());
            let operand = u32::try_from(source.rfind("shortcut").unwrap()).unwrap();
            assert!(original_source_linked_member_at(&analysis, source, operand).is_none());
            assert!(
                original_source_linked_member_at(
                    &analysis,
                    &source.replace("shortcut", "changed"),
                    cursor
                )
                .is_none()
            );
        }
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let cursor = u32::try_from(source.find("{shortcut;").unwrap() + 1).unwrap();
        assert!(original_source_linked_member_at(&analysis, source, cursor).is_none());
    }

    #[test]
    fn link_list_fields_keep_alias_target_and_edit_topology_separate() {
        let source = "oo::class create C {method foo {} {}; constructor {} {link foo {bar foo}}; method use {} {return [foo]; bar}}";
        let analysis = Analyser::new().analyse(source, "tcl9.0");
        let call = u32::try_from(source.rfind("[foo]").unwrap() + 1).unwrap();
        let selected = original_source_linked_member_at(&analysis, source, call).unwrap();
        assert!(selected.has_coupled_name());
        let class = selected.class_declaration().metadata();
        let spans = original_source_linked_member_reference_spans(
            &analysis,
            source,
            class,
            selected.target().metadata(),
            MemberSide::Instance,
        )
        .unwrap();
        let values: Vec<_> = spans.iter().map(|span| &source[span.as_range()]).collect();
        assert_eq!(values, ["foo", "foo", "foo"]);
        let pair_call = u32::try_from(source.rfind("bar").unwrap()).unwrap();
        assert!(
            !original_source_linked_member_at(&analysis, source, pair_call)
                .unwrap()
                .has_coupled_name()
        );
        let unknown = source.replace("{bar foo}", "$descriptor");
        let analysis = Analyser::new().analyse(&unknown, "tcl9.0");
        let call = u32::try_from(unknown.rfind("[foo]").unwrap() + 1).unwrap();
        assert!(original_source_linked_member_at(&analysis, &unknown, call).is_none());
    }
    #[test]
    fn stock_classmethod_link_keeps_the_original_delegate_and_member_axis() {
        for (dialect, output) in [
            (
                "tcl9.0",
                include_str!(
                    "../../../../tcl-registry/tests/data/source_contract_vectors/tcloo-linked-source/class-delegate/9.0.4/stdout.bin"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../../tcl-registry/tests/data/source_contract_vectors/tcloo-linked-source/class-delegate/9.1.0/stdout.bin"
                ),
            ),
        ] {
            assert!(output.contains("DELEGATE 0 FACTORY"));
            let source = "oo::class create C {constructor {} {link factory}; classmethod factory {} {}; method use {} {factory}}";
            let analysis = Analyser::new().analyse(source, dialect);
            let cursor = u32::try_from(source.rfind("factory").unwrap()).unwrap();
            let linked = original_source_linked_member_at(&analysis, source, cursor).unwrap();
            assert_eq!(linked.target().side(), MemberSide::ClassObject);
            assert!(linked.target().native_class_delegate());
            let references = original_source_linked_member_reference_spans(
                &analysis,
                source,
                linked.class_declaration().metadata(),
                linked.target().metadata(),
                MemberSide::ClassObject,
            )
            .unwrap();
            assert_eq!(references.len(), 2);
        }
    }

    #[test]
    fn original_link_advice_refuses_malformed_foreign_and_conflicting_receipts() {
        for descriptor in ["{}", "{route first second}", "$descriptor"] {
            let source = format!(
                "oo::class create C {{method first {{}} {{}}; method second {{}} {{}}; constructor {{}} {{link {descriptor}}}; method use {{}} {{route}}}}"
            );
            let analysis = Analyser::new().analyse(&source, "tcl9.0");
            let cursor = u32::try_from(source.rfind("route").unwrap()).unwrap();
            assert!(
                original_source_linked_member_at(&analysis, &source, cursor).is_none(),
                "{descriptor}"
            );
        }
        let conflicting = "oo::class create C {method first {} {}; method second {} {}; constructor {} {link {route first} {route second}}; method use {} {route}}";
        let analysis = Analyser::new().analyse(conflicting, "tcl9.0");
        let cursor = u32::try_from(conflicting.rfind("route").unwrap()).unwrap();
        assert!(original_source_linked_member_at(&analysis, conflicting, cursor).is_none());

        let source =
            "oo::class create C {method foo {} {}; constructor {} {link foo}; method use {} {foo}}";
        let mut analysis = Analyser::new().analyse(source, "tcl9.0");
        let cursor = u32::try_from(source.rfind("foo").unwrap()).unwrap();
        let genuine = original_source_linked_member_at(&analysis, source, cursor).unwrap();
        fn change_report_labels(scope: &mut Scope) {
            scope.name = "::report-only".to_owned();
            for child in &mut scope.children {
                change_report_labels(child);
            }
        }
        change_report_labels(&mut analysis.global_scope);
        for class in analysis.all_classes.values_mut() {
            class.qualified_name = "::report-only".to_owned();
            class.linked_members.clear();
        }
        assert_eq!(
            original_source_linked_member_at(&analysis, source, cursor)
                .unwrap()
                .target(),
            genuine.target()
        );
        let foreign_source = format!("{source}\n# independent source image\n");
        let foreign = Analyser::new().analyse(&foreign_source, "tcl9.0");
        let foreign_link =
            original_source_linked_member_at(&foreign, &foreign_source, cursor).unwrap();
        let mut foreign_ancestry = genuine.clone();
        foreign_ancestry.target_input = foreign_link.target_input().clone();
        assert!(foreign_ancestry.editable_target_span(source).is_none());
        let class = genuine.class_declaration();
        assert!(
            original_source_linked_members_for_class(&foreign, &foreign_source, class).is_none()
        );
        analysis.resolved_input = Analyser::new().analyse(source, "tcl9.1").resolved_input;
        assert!(original_source_linked_member_at(&analysis, source, cursor).is_none());
    }

    #[test]
    fn link_does_not_borrow_a_changed_or_unrelated_classmethod_delegate() {
        let source = "oo::class create C {constructor {} {link factory}; classmethod factory {} {}; method factory {} {}; method use {} {factory}}";
        let analysis = Analyser::new().analyse(source, "tcl9.0");
        let cursor = u32::try_from(source.rfind("factory").unwrap()).unwrap();
        let linked = original_source_linked_member_at(&analysis, source, cursor).unwrap();
        assert_eq!(linked.target().side(), MemberSide::Instance);
        assert!(!linked.target().native_class_delegate());
        let changed = "oo::class create C {constructor {} {link factory}; classmethod factory {} {}; method use {} {factory}}; oo::define C {self method factory {} {return OTHER}}";
        let analysis = Analyser::new().analyse(changed, "tcl9.0");
        let class = analysis.original_class_declarations().next().unwrap();
        let instance = class
            .metadata()
            .original_members
            .methods(MemberSide::Instance)
            .unwrap();
        assert!(
            instance
                .iter()
                .any(|method| method.original_name_input().bytes() == b"factory"
                    && method.native_class_delegate())
        );
        let class_object = class
            .metadata()
            .original_members
            .methods(MemberSide::ClassObject)
            .unwrap();
        assert!(
            class_object
                .iter()
                .any(|method| method.original_name_input().bytes() == b"factory"
                    && !method.native_class_delegate())
        );
        let cursor = u32::try_from(changed.find("{factory}").unwrap() + 1).unwrap();
        assert!(original_source_linked_member_at(&analysis, changed, cursor).is_none());
    }
}
