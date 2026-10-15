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

//! Readonly Logical member declarations from the actual completed class walk.

use super::{AnalysisResult, ClassDef, MemberSide, MethodDef};
use crate::analyser::ResolvedAnalysisInput;
use std::sync::Arc;
use tcl_lexer::SourceImage;

/// A completed Logical class declaration, independent of Native name recipes.
/// The snapshot supplies source membership only, never dispatch or editing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalSourceClassDeclaration {
    image: SourceImage,
    input: ResolvedAnalysisInput,
    class: Arc<ClassDef>,
    body_word: Option<tcl_lexer::NativeWord>,
    candidate_coordinate: Option<tcl_core_types::ByteCommandSlot>,
}

/// One exact side and declaration in the retained Logical class snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalSourceMemberDeclaration {
    class: LogicalSourceClassDeclaration,
    side: MemberSide,
    method: MethodDef,
}

/// Original static member-name data and its independently quoted source word.
/// This readonly suggestion supplies no method dispatch or editing permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalSourceMemberInsertion {
    name: String,
    word: String,
}
impl LogicalSourceMemberInsertion {
    /// Exact original static name, independent of a display-map key.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// One quoted word under the retained original source grammar.
    #[must_use]
    pub fn source_word(&self) -> &str {
        &self.word
    }
}

impl LogicalSourceClassDeclaration {
    pub(crate) fn capture(
        image: SourceImage,
        input: ResolvedAnalysisInput,
        class: &ClassDef,
    ) -> Option<Self> {
        let len = u32::try_from(image.bytes().len()).ok()?;
        let bounded = |span: tcl_lexer::Span| span.start() < span.end() && span.end() <= len;
        (input.has_logical_source_name_context()
            && bounded(class.name_span)
            && bounded(class.body_span))
        .then_some(())?;
        Some(Self {
            image,
            input,
            class: Arc::new(class.clone()),
            body_word: None,
            candidate_coordinate: None,
        })
    }

    fn matches(&self, analysis: &AnalysisResult, class: &ClassDef) -> bool {
        if !analysis.allows_retained_logical_declaration_advice()
            || analysis.resolved_input.as_ref() != Some(&self.input)
            || !analysis.matches_original_source_image(&self.image, self.input.lexer_config())
            || !analysis
                .retained_command_realm()
                .is_some_and(|realm| realm.matches_resolved_analysis_input(&self.input))
        {
            return false;
        }
        let mut current = class.clone();
        current
            .qualified_name
            .clone_from(&self.class.qualified_name);
        current == *self.class
    }

    /// Original body selected by the actual definition-script source factory.
    /// It supplies syntax without class allocation, member execution or editing.
    #[must_use]
    pub fn original_body_word(
        &self,
        image: &SourceImage,
        input: &ResolvedAnalysisInput,
    ) -> Option<&tcl_lexer::NativeWord> {
        if image != &self.image || input != &self.input {
            return None;
        }
        let word = self.body_word.as_ref()?;
        (word.image() == image && word.config().nested() == input.lexer_config().nested())
            .then_some(word)
    }

    /// Exact selected original factory Name coordinate for readonly candidates.
    /// The factory's own original name/body correspondence is independent of
    /// the Module-issued body syntax facet and supplies no class identity.
    #[must_use]
    pub fn source_candidate_coordinate(&self) -> Option<&tcl_core_types::ByteCommandSlot> {
        self.candidate_coordinate.as_ref()
    }

    /// Exact retained member metadata on the selected source side.
    #[must_use]
    pub fn member(
        &self,
        side: MemberSide,
        method: &MethodDef,
    ) -> Option<LogicalSourceMemberDeclaration> {
        let members = match side {
            MemberSide::Instance => &self.class.methods,
            MemberSide::ClassObject => &self.class.class_methods,
        };
        let mut matches = members.values().filter(|candidate| *candidate == method);
        let selected = matches.next()?;
        matches
            .next()
            .is_none()
            .then(|| LogicalSourceMemberDeclaration {
                class: self.clone(),
                side,
                method: selected.clone(),
            })
    }
}

impl AnalysisResult {
    pub(crate) fn retain_logical_class_body_sources(
        &mut self,
        module: &crate::ir::Module,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let sources = self
            .logical_class_declarations
            .iter()
            .map(|declaration| {
                let class = self
                    .all_classes
                    .values()
                    .find(|class| declaration.matches(self, class))?;
                let body = module
                    .original_class_definition_body_source(class, registry)
                    .filter(|source| {
                        source.matches_source_input(&declaration.image, &declaration.input)
                    })
                    .map(|source| source.body_word().clone());
                let source_text = declaration.image.try_text().ok()?;
                let offset = class.body_span.start();
                let coordinate = self.command_invocations.iter().find_map(|invocation| {
                    if invocation.range.start() >= offset {
                        return None;
                    }
                    let factory =
                        crate::registry_invocation::source_structure::source_class_declaration_at(
                            source_text,
                            self,
                            invocation.range.start(),
                        )?;
                    std::ptr::eq(factory.logical_source_class(self)?, class).then_some(())?;
                    let original = factory.name_input().logical_input()?;
                    (tcl_syntax::word_rules::original_static_word_unicode_value(
                        original.original_word(),
                    )
                    .as_deref()
                        == Some(original.bytes()))
                    .then_some(())?;
                    Some(factory.source_slot().clone())
                });
                Some((body, coordinate))
            })
            .collect::<Vec<_>>();
        for (declaration, source) in self.logical_class_declarations.iter_mut().zip(sources) {
            if let Some((body, coordinate)) = source {
                declaration.body_word = body;
                declaration.candidate_coordinate = coordinate;
            } else {
                declaration.body_word = None;
                declaration.candidate_coordinate = None;
            }
        }
    }

    /// A current selected member and genuine whole name word supply insertion data.
    /// Computed/generated names without a static original operand remain unavailable.
    #[must_use]
    pub fn logical_source_member_insertion(
        &self,
        class: &ClassDef,
        side: MemberSide,
        method: &MethodDef,
    ) -> Option<LogicalSourceMemberInsertion> {
        let class_owner = self.logical_source_class_declaration(class)?;
        let members = match side {
            MemberSide::Instance => &class.methods,
            MemberSide::ClassObject => &class.class_methods,
        };
        members
            .values()
            .any(|current| std::ptr::eq(current, method))
            .then_some(())?;
        class_owner.member(side, method)?;
        let config = class_owner.input.lexer_config();
        let original = class
            .original_members
            .logical_name_operand(side, method, &class_owner.input, &class_owner.image)?
            .clone();
        let value = tcl_syntax::word_rules::original_static_word_ascii_presentation(&original)?;
        if value.contains(&0) || value.as_slice() != method.name.as_bytes() {
            return None;
        }
        let name = String::from_utf8(value).ok()?;
        let word = tcl_syntax::word_rules::lexical_ascii_source_word(&name, config)?;
        Some(LogicalSourceMemberInsertion { name, word })
    }

    /// A current actual class must still agree with the completed source walk.
    /// Reporting qualified labels are excluded; altered member metadata declines.
    #[must_use]
    pub fn logical_source_class_declaration(
        &self,
        class: &ClassDef,
    ) -> Option<&LogicalSourceClassDeclaration> {
        self.all_classes
            .values()
            .any(|current| std::ptr::eq(current, class))
            .then_some(())?;
        let mut declarations = self
            .logical_class_declarations
            .iter()
            .filter(|declaration| declaration.matches(self, class));
        let selected = declarations.next()?;
        declarations.next().is_none().then_some(selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn logical_class_body_word_comes_from_the_actual_definition_script_factory() {
        let source = "oo::class create C {method m {arg} {return $arg}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let class = &analysis.all_classes["::C"];
        let declaration = analysis.logical_source_class_declaration(class).unwrap();
        let input = analysis.resolved_input.as_ref().unwrap();
        let image = SourceImage::document(source);
        let word = declaration
            .original_body_word(&image, input)
            .expect("actual issued definition body source");
        assert_eq!(
            source.get(word.content_span().unwrap().as_range()),
            Some("method m {arg} {return $arg}")
        );
        assert!(
            declaration
                .original_body_word(&SourceImage::document("changed"), input)
                .is_none()
        );
        let mut changed = analysis.clone();
        changed
            .all_classes
            .get_mut("::C")
            .unwrap()
            .methods
            .get_mut("m")
            .unwrap()
            .params
            .clear();
        assert!(
            changed
                .logical_source_class_declaration(&changed.all_classes["::C"])
                .is_none()
        );
        let empty_source = "oo::class create Empty {}";
        let empty = Analyser::new().analyse(empty_source, "tcl");
        let declaration = empty
            .logical_source_class_declaration(&empty.all_classes["::Empty"])
            .unwrap();
        let word = declaration
            .original_body_word(
                &SourceImage::document(empty_source),
                empty.resolved_input.as_ref().unwrap(),
            )
            .expect("empty original class body");
        assert!(word.content_span().unwrap().is_empty());
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            native
                .logical_source_class_declaration(&native.all_classes["::C"])
                .is_none()
        );
    }

    #[test]
    fn logical_member_insertions_keep_literal_name_data_and_current_side_owner() {
        let source = "oo::class create C {method {$literal} {} {}; method {two words} {} {}; method dynamic$name {} {}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let class = &analysis.all_classes["::C"];
        for (name, expected) in [("$literal", "{$literal}"), ("two words", "{two words}")] {
            let method = &class.methods[name];
            let insertion = analysis
                .logical_source_member_insertion(class, MemberSide::Instance, method)
                .expect("actual static Logical method-name operand");
            assert_eq!(insertion.name(), name);
            assert_eq!(insertion.source_word(), expected);
            assert!(
                analysis
                    .logical_source_member_insertion(class, MemberSide::ClassObject, method)
                    .is_none()
            );
            assert!(
                analysis
                    .logical_source_member_insertion(class, MemberSide::Instance, &method.clone())
                    .is_none()
            );
        }
        let mut changed = analysis.clone();
        changed
            .all_classes
            .get_mut("::C")
            .unwrap()
            .methods
            .get_mut("$literal")
            .unwrap()
            .body_span = tcl_lexer::Span::new(0, 1);
        let class = &changed.all_classes["::C"];
        assert!(
            changed
                .logical_source_member_insertion(
                    class,
                    MemberSide::Instance,
                    &class.methods["$literal"]
                )
                .is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        let class = &missing.all_classes["::C"];
        assert!(
            missing
                .logical_source_member_insertion(
                    class,
                    MemberSide::Instance,
                    &class.methods["$literal"]
                )
                .is_none()
        );
    }

    #[test]
    fn logical_family_insertions_require_selected_original_names() {
        for source in [
            "snit::type C {method {two words} {} {}}",
            "itcl::class C {method {two words} {} {}}",
        ] {
            let analysis = Analyser::new().analyse(source, "tcl");
            let class = &analysis.all_classes["::C"];
            let method = &class.methods["two words"];
            let insertion = analysis
                .logical_source_member_insertion(class, MemberSide::Instance, method)
                .expect("selected original family method-name operand");
            assert_eq!(insertion.name(), "two words");
            assert_eq!(insertion.source_word(), "{two words}");
            assert!(
                analysis
                    .logical_source_member_insertion(class, MemberSide::Instance, &method.clone())
                    .is_none()
            );
        }
    }

    #[test]
    fn logical_member_declarations_keep_actual_side_source_and_current_metadata() {
        let source = "oo::class create Base {method m {arg} {}; self method m {} {}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let class = &analysis.all_classes["::Base"];
        let declaration = analysis
            .logical_source_class_declaration(class)
            .expect("actual Logical class walk");
        let instance = declaration
            .member(MemberSide::Instance, &class.methods["m"])
            .unwrap();
        let class_side = declaration
            .member(MemberSide::ClassObject, &class.class_methods["m"])
            .unwrap();
        assert_ne!(instance, class_side);
        assert!(
            declaration
                .member(MemberSide::ClassObject, &class.methods["m"])
                .is_none()
        );
        assert!(
            analysis
                .logical_source_class_declaration(&class.clone())
                .is_none()
        );
        let mut labels = analysis.clone();
        labels.all_classes.get_mut("::Base").unwrap().qualified_name = "display only".into();
        assert_eq!(
            labels.logical_source_class_declaration(&labels.all_classes["::Base"]),
            Some(declaration)
        );
        let mut changed = analysis.clone();
        changed
            .all_classes
            .get_mut("::Base")
            .unwrap()
            .methods
            .get_mut("m")
            .unwrap()
            .params
            .clear();
        assert!(
            changed
                .logical_source_class_declaration(&changed.all_classes["::Base"])
                .is_none()
        );
        let mut changed = analysis.clone();
        changed.all_classes.get_mut("::Base").unwrap().body_span = tcl_lexer::Span::new(0, 1);
        assert!(
            changed
                .logical_source_class_declaration(&changed.all_classes["::Base"])
                .is_none()
        );
        let mut changed = analysis.clone();
        changed.resolved_input = None;
        assert!(
            changed
                .logical_source_class_declaration(&changed.all_classes["::Base"])
                .is_none()
        );
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            native
                .logical_source_class_declaration(&native.all_classes["::Base"])
                .is_none()
        );
    }
}

#[cfg(test)]
mod source_navigation_facet_tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn logical_class_navigation_keeps_the_selected_factory_independent_of_module_body_syntax() {
        let source = "oo::class create helper {method run {} {}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let class = &analysis.all_classes["::helper"];
        let declaration = analysis.logical_source_class_declaration(class).unwrap();
        let coordinate = declaration
            .source_candidate_coordinate()
            .expect("actual selected factory Name/body and current class correspondence");
        assert_eq!(
            coordinate.namespace,
            tcl_core_types::ByteNamespacePath::root()
        );
        assert_eq!(coordinate.simple.as_bytes(), b"helper");
        let mut without_module_body = declaration.clone();
        without_module_body.body_word = None;
        assert_eq!(
            without_module_body.source_candidate_coordinate(),
            Some(coordinate)
        );
        assert!(
            without_module_body
                .original_body_word(
                    &SourceImage::document(source),
                    analysis.resolved_input.as_ref().unwrap()
                )
                .is_none()
        );
        let detached = LogicalSourceClassDeclaration::capture(
            SourceImage::document(source),
            analysis.resolved_input.as_ref().unwrap().clone(),
            class,
        )
        .unwrap();
        assert!(detached.source_candidate_coordinate().is_none());
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(
            missing
                .logical_source_class_declaration(&missing.all_classes["::helper"])
                .is_none()
        );
        let native = Analyser::new().analyse(source, "tcl8.6");
        assert!(
            native
                .logical_source_class_declaration(&native.all_classes["::helper"])
                .is_none()
        );
    }
}
