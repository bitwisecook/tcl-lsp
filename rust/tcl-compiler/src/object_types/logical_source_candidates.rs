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

//! Scoped readonly receiver suggestions retain their original analysis owner.

use super::{ObjectFactOwner, ObjectHandleFacts, OwnerSpan};
use crate::analyser::{AnalysisResult, ResolvedAnalysisInput};
use crate::compilation_unit::CompilationUnit;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tcl_core_types::BodyFrameId;
use tcl_lexer::{NativeWord, SourceImage};
use tcl_registry::{CommandRegistry, RegistrySemanticKey};

#[derive(Debug, Clone)]
pub(super) struct LogicalSourceCandidateOwner {
    realm: Arc<crate::realm::CommandBindingRealm>,
    image: SourceImage,
    input: ResolvedAnalysisInput,
    registry: RegistrySemanticKey,
    scopes: Vec<OwnerSpan>,
    classes: HashMap<(ObjectFactOwner, String), HashSet<String>>,
    instance_variables: HashMap<ObjectFactOwner, HashSet<String>>,
    frames: HashSet<BodyFrameId>,
    original_classes: HashSet<ObjectFactOwner>,
    definition_classes: HashSet<ObjectFactOwner>,
}

// Presentation equality cannot substitute for the current retained issuer.
impl PartialEq for LogicalSourceCandidateOwner {
    fn eq(&self, other: &Self) -> bool {
        self.image == other.image
            && self.input == other.input
            && self.registry == other.registry
            && self.scopes == other.scopes
            && self.classes == other.classes
            && self.instance_variables == other.instance_variables
    }
}

pub(super) struct LogicalCandidateReport<'a> {
    pub(super) instance_variables: &'a HashMap<ObjectFactOwner, HashSet<String>>,
    pub(super) frames: &'a HashSet<BodyFrameId>,
    pub(super) original_classes: &'a HashSet<ObjectFactOwner>,
    pub(super) definition_classes: &'a HashSet<ObjectFactOwner>,
}

impl LogicalSourceCandidateOwner {
    pub(super) fn report_parts<'a>(
        &'a self,
        facts: &ObjectHandleFacts,
        image: &SourceImage,
        input: &ResolvedAnalysisInput,
        registry: &RegistrySemanticKey,
    ) -> Option<LogicalCandidateReport<'a>> {
        (self.image == *image
            && self.input == *input
            && self.registry == *registry
            && self.scopes == facts.owner_spans
            && self.classes == facts.by_scope)
            .then_some(LogicalCandidateReport {
                instance_variables: &self.instance_variables,
                frames: &self.frames,
                original_classes: &self.original_classes,
                definition_classes: &self.definition_classes,
            })
    }
}

/// Scoped possible receiver classes. Contents versions remain unioned and
/// unknown alternatives remain possible; this is no runtime read or dispatch proof.
pub struct LogicalSourceReceiverClassCandidates<'a> {
    classes: &'a HashSet<String>,
}
impl<'a> LogicalSourceReceiverClassCandidates<'a> {
    /// Possible classes from the actual analytical owner, without name-map joins.
    pub fn classes(&self) -> impl Iterator<Item = &'a String> + 'a {
        self.classes.iter()
    }
    /// Source suggestions do not close unrepresented contents or execution.
    #[must_use]
    pub const fn has_unknown_residual(&self) -> bool {
        true
    }
    /// The scoped producer joins all represented contents versions.
    #[must_use]
    pub const fn contents_versions_are_unioned(&self) -> bool {
        true
    }
}

fn definition_receiver_is_current(
    owner: &ObjectFactOwner,
    method: &crate::ir::MethodDef,
    module: &crate::ir::Module,
    registry: &CommandRegistry,
) -> bool {
    let ObjectFactOwner::DefinitionScript { definition, .. } = owner else {
        return false;
    };
    if method.definition_scope != Some(*definition) {
        return false;
    }
    let Some(source) = module.original_definition_script_source(*definition, registry) else {
        return false;
    };
    method.span.is_some_and(|span| {
        source.content_span().start() <= span.start() && span.end() <= source.content_span().end()
    })
}

pub(super) fn logical_source_unit_input<'a>(
    analysis: &'a AnalysisResult,
    unit: &CompilationUnit,
    registry: &CommandRegistry,
) -> Option<(
    &'a ResolvedAnalysisInput,
    &'a Arc<crate::realm::CommandBindingRealm>,
    &'a SourceImage,
)> {
    let input = analysis.resolved_input.as_ref()?;
    let realm = analysis.retained_command_realm_arc()?;
    let image = realm.original_source_image()?;
    let module = &unit.ir_module;
    if !analysis.allows_retained_logical_declaration_advice()
        || !input.has_logical_source_name_context()
        || !realm.matches_resolved_analysis_input(input)
        || image.try_text().ok() != Some(unit.source.as_str())
        || &module.source != image
        || module.source_metadata_input.as_ref() != Some(input)
        || module.native_lexer_config() != input.lexer_config()
        || module.source_entry.native_entry.is_some()
        || module.source_entry.unknown_entry
        || module
            .retained_source_bindings
            .as_ref()
            .is_none_or(|held| !held.matches_module(module, registry))
        || registry.snapshot().semantic_key()
            != input
                .borrowed_context_registry()
                .commands()
                .snapshot()
                .semantic_key()
    {
        return None;
    }
    Some((input, realm, image))
}

impl ObjectHandleFacts {
    pub(crate) fn retain_logical_source_candidates(
        &mut self,
        analysis: &AnalysisResult,
        unit: &CompilationUnit,
        registry: &CommandRegistry,
    ) {
        self.logical_source_candidates = None;
        let Some((input, realm, image)) = logical_source_unit_input(analysis, unit, registry)
        else {
            return;
        };
        let module = &unit.ir_module;
        let frames = unit
            .all_body_function_units()
            .filter_map(|function| {
                if std::ptr::eq(function, &unit.top_level) {
                    Some(BodyFrameId::Top(module.procedure_ids.module_id()))
                } else {
                    function
                        .body_id
                        .filter(|id| {
                            module.procedure_ids.owns(*id)
                                && (module.methods.contains_key(id)
                                    || module
                                        .original_source_procedure_bodies()
                                        .any(|(body, _)| body == id))
                                && function
                                    .invocation_metadata_context_for_module(registry, module)
                                    .is_some()
                        })
                        .map(BodyFrameId::Body)
                }
            })
            .collect::<HashSet<_>>();
        if self.owner_spans.iter().any(|scope| match scope.frame {
            BodyFrameId::Top(id) => {
                id != module.procedure_ids.module_id()
                    || scope.span
                        != tcl_lexer::Span::new(
                            0,
                            u32::try_from(unit.source.len()).unwrap_or(u32::MAX),
                        )
            }
            BodyFrameId::Body(id) => {
                !module.procedure_ids.owns(id)
                    || module.methods.get(&id).map_or_else(
                        || {
                            let mut bodies = module
                                .original_source_procedure_bodies()
                                .filter(|(body, _)| **body == id);
                            bodies
                                .next()
                                .is_none_or(|(_, body)| body.span != scope.span)
                                || bodies.next().is_some()
                        },
                        |method| method.span != Some(scope.span),
                    )
            }
        }) {
            return;
        }
        let mut instance_variables = HashMap::<ObjectFactOwner, HashSet<String>>::new();
        let mut original_classes = HashSet::new();
        let mut definition_classes = HashSet::new();
        for (id, method) in &module.methods {
            if !frames.contains(&BodyFrameId::Body(*id)) {
                continue;
            }
            if let Some(owner) = self
                .owner_spans
                .iter()
                .find(|scope| scope.frame == BodyFrameId::Body(*id))
                .and_then(|scope| scope.class_owner.as_ref())
            {
                if let ObjectFactOwner::OriginalClass { allocation, .. } = owner
                    && method
                        .original_receiver_context
                        .as_ref()
                        .and_then(|context| context.declaring_allocation())
                        == Some(allocation)
                    && allocation.site.source.source_image() == image
                {
                    original_classes.insert(owner.clone());
                }
                if definition_receiver_is_current(owner, method, module, registry) {
                    definition_classes.insert(owner.clone());
                }
                instance_variables
                    .entry(owner.clone())
                    .or_default()
                    .extend(method.instance_vars.iter().cloned());
            }
        }
        self.logical_source_candidates = Some(Arc::new(LogicalSourceCandidateOwner {
            realm: Arc::clone(realm),
            image: image.clone(),
            input: input.clone(),
            registry: registry.snapshot().semantic_key(),
            scopes: self.owner_spans.clone(),
            classes: self.by_scope.clone(),
            instance_variables,
            frames,
            original_classes,
            definition_classes,
        }));
    }
}

impl AnalysisResult {
    /// Actual original scalar head geometry selects scoped completion advice.
    /// Missing source ownership is terminal and cannot borrow physical read maps.
    #[must_use]
    pub fn logical_source_receiver_class_candidates<'a>(
        &'a self,
        head: &NativeWord,
        registry: &CommandRegistry,
    ) -> Option<LogicalSourceReceiverClassCandidates<'a>> {
        let facts = &self.object_handle_facts;
        let owner = facts.logical_source_candidates.as_ref()?;
        let current = self.retained_command_realm_arc()?;
        if !self.allows_retained_logical_declaration_advice()
            || !Arc::ptr_eq(current, &owner.realm)
            || self.resolved_input.as_ref() != Some(&owner.input)
            || !current.matches_resolved_analysis_input(&owner.input)
            || !self.matches_original_source_image(&owner.image, owner.input.lexer_config())
            || current.original_source_image() != Some(&owner.image)
            || registry.snapshot().semantic_key() != owner.registry
            || facts.owner_spans != owner.scopes
            || facts.by_scope != owner.classes
            || head.image() != &owner.image
            || head.config().nested() != owner.input.lexer_config().nested()
            || head.group().expand
        {
            return None;
        }
        let reference =
            tcl_lexer::word_parts::whole_var_ref(head.bytes(), head.config()).ok()??;
        if reference.index.is_some()
            || reference.name.contains(&0)
            || tcl_syntax::naming::split_element_ref_bytes(reference.name).is_some()
        {
            return None;
        }
        let name = std::str::from_utf8(reference.name).ok()?;
        let parts = head.executable_parts().list(head.executable_parts().root());
        let [part] = parts else {
            return None;
        };
        if !matches!(
            part.part,
            tcl_lexer::ExecutablePart::Variable { index: None, .. }
        ) || part.span != head.word_span()
        {
            return None;
        }
        let scope = facts.owner_at(part.span.start())?;
        if !owner.frames.contains(&scope.frame) {
            return None;
        }
        let key = scope
            .class_owner
            .as_ref()
            .filter(|class| {
                owner
                    .instance_variables
                    .get(*class)
                    .is_some_and(|names| names.contains(name))
            })
            .cloned()
            .unwrap_or(ObjectFactOwner::Frame(scope.frame));
        match &key {
            ObjectFactOwner::Frame(frame) if !owner.frames.contains(frame) => return None,
            ObjectFactOwner::OriginalClass { .. } if !owner.original_classes.contains(&key) => {
                return None;
            }
            ObjectFactOwner::DefinitionScript { .. }
                if !owner.definition_classes.contains(&key) =>
            {
                return None;
            }
            _ => {}
        }
        let classes = owner.classes.get(&(key, name.to_owned()))?;
        (!classes.is_empty()).then_some(LogicalSourceReceiverClassCandidates { classes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::Analyser;

    fn original_head(analysis: &AnalysisResult, offset: u32) -> NativeWord {
        let input = analysis.resolved_input.as_ref().unwrap();
        let image = analysis
            .retained_command_realm()
            .unwrap()
            .original_source_image()
            .unwrap();
        tcl_lexer::native_script_words_in(
            image.clone(),
            tcl_lexer::Span::new(0, u32::try_from(image.bytes().len()).unwrap()),
            input.lexer_config(),
        )
        .unwrap()
        .commands
        .into_iter()
        .find(|command| {
            command
                .words
                .first()
                .is_some_and(|word| word.span().start() == offset)
        })
        .unwrap()
        .words
        .remove(0)
    }

    #[test]
    fn logical_receiver_candidates_keep_original_scope_and_unknown_contents() {
        let source = "oo::class create C {method longMethod {} {}}\nset {two words} [C new]\n${two words} longMet";
        let analysis = Analyser::new().analyse(source, "tcl");
        let head = original_head(&analysis, source.find("${two words}").unwrap() as u32);
        let registry = analysis.resolved_registry().unwrap();
        let candidates = analysis
            .logical_source_receiver_class_candidates(&head, registry)
            .expect("actual source-scoped constructor suggestions");
        assert_eq!(
            candidates.classes().map(String::as_str).collect::<Vec<_>>(),
            ["::C"]
        );
        assert!(candidates.has_unknown_residual());
        assert!(candidates.contents_versions_are_unioned());
        assert!(analysis.object_handle_facts.proven_reads.is_empty());
        assert!(analysis.object_handle_facts.physical_type_reads.is_empty());
        assert!(
            analysis
                .object_handle_facts
                .normal_procedure_results
                .is_empty()
        );
        assert!(
            analysis
                .object_handle_facts
                .classes_in_scope(head.span().start(), "two words")
                .is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(
            missing
                .logical_source_receiver_class_candidates(&head, registry)
                .is_none()
        );
        let mut changed = analysis.clone();
        changed.object_handle_facts.by_scope.clear();
        assert!(
            changed
                .logical_source_receiver_class_candidates(&head, registry)
                .is_none()
        );
        let foreign = Analyser::new().analyse(source, "tcl");
        changed.object_handle_facts = foreign.object_handle_facts;
        assert!(
            changed
                .logical_source_receiver_class_candidates(&head, registry)
                .is_none()
        );
        let mut copied_realm = analysis.clone();
        let mut receipt = (**copied_realm
            .object_handle_facts
            .logical_source_candidates
            .as_ref()
            .unwrap())
        .clone();
        receipt.realm = Arc::new((*receipt.realm).clone());
        copied_realm.object_handle_facts.logical_source_candidates = Some(Arc::new(receipt));
        assert!(
            copied_realm
                .logical_source_receiver_class_candidates(&head, registry)
                .is_none()
        );
        let mut custom_config = head.config();
        custom_config.braced_var = match custom_config.braced_var {
            tcl_dialect::BracedVarStyle::FirstClose => tcl_dialect::BracedVarStyle::Tcl9Nesting,
            tcl_dialect::BracedVarStyle::Tcl9Nesting => tcl_dialect::BracedVarStyle::FirstClose,
        };
        let custom = tcl_lexer::NativeWord::from_group(
            head.image().clone(),
            custom_config,
            head.tokens(),
            head.group(),
        )
        .unwrap();
        assert!(
            analysis
                .logical_source_receiver_class_candidates(&custom, registry)
                .is_none()
        );
        let other = tcl_registry::model::ingress::static_context_for("jim");
        assert!(
            analysis
                .logical_source_receiver_class_candidates(&head, other.commands())
                .is_none()
        );
    }
    #[test]
    fn logical_receiver_candidates_keep_unrelated_original_procedure_scopes() {
        let source = "proc unrelated {} {return VALUE}\noo::class create C {method m {} {}}\nset object [C new]\n$object m";
        let analysis = Analyser::new().analyse(source, "tcl");
        let head = original_head(&analysis, source.rfind("$object").unwrap() as u32);
        let registry = analysis.resolved_registry().unwrap();
        let candidates = analysis
            .logical_source_receiver_class_candidates(&head, registry)
            .expect("unrelated original proc cannot withdraw the top-level source owner");
        assert!(candidates.classes().any(|class| class == "::C"));
        assert!(candidates.has_unknown_residual());
        assert!(analysis.object_handle_facts.proven_reads.is_empty());
        assert!(analysis.object_handle_facts.physical_type_reads.is_empty());
    }

    #[test]
    fn logical_assignment_candidates_keep_literal_data_separate_from_reads() {
        for (value, expected) in [
            ("$object", true),
            (r#""$object""#, true),
            ("{$object}", false),
            ("{[C new]}", false),
            ("prefix$object", false),
        ] {
            let source = format!(
                "oo::class create C {{method longMethod {{}} {{}}}}\nset object [C new]\nset alias {value}\n$alias longMet"
            );
            let analysis = Analyser::new().analyse(&source, "tcl");
            let registry = analysis.resolved_registry().unwrap();
            let head = original_head(&analysis, source.rfind("$alias").unwrap() as u32);
            let classes = analysis.logical_source_receiver_class_candidates(&head, registry);
            assert_eq!(classes.is_some(), expected, "{value}");
            if let Some(classes) = classes {
                assert!(classes.classes().any(|class| class == "::C"));
                assert!(classes.has_unknown_residual());
            }
            assert!(analysis.object_handle_facts.proven_reads.is_empty());
            assert!(analysis.object_handle_facts.physical_type_reads.is_empty());
            assert!(
                analysis
                    .object_handle_facts
                    .normal_procedure_results
                    .is_empty()
            );
        }
    }

    #[test]
    fn logical_receiver_candidates_refuse_array_and_nul_name_geometry() {
        for head_text in ["$value(key)", "${value(key)}", "${value\0tail}"] {
            let source = format!(
                "oo::class create C {{method m {{}} {{}}}}\nset value [C new]\n{head_text} m"
            );
            let analysis = Analyser::new().analyse(&source, "tcl");
            let head = original_head(&analysis, source.rfind(head_text).unwrap() as u32);
            assert!(
                analysis
                    .object_handle_facts
                    .logical_source_candidates
                    .is_some()
            );
            assert!(
                analysis
                    .logical_source_receiver_class_candidates(
                        &head,
                        analysis.resolved_registry().unwrap()
                    )
                    .is_none()
            );
        }
    }
    #[test]
    fn definition_receiver_candidates_require_the_actual_selected_definition_body() {
        use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
        let source = "oo::class create C {variable value; method m {} {$value longMet}}\noo::class create D {variable value; method m {} {$value longMet}}";
        let analysis = Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let registry = input.borrowed_context_registry().commands();
        let unit = CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            input,
        );
        let module = &unit.ir_module;
        let method = module
            .methods
            .values()
            .next()
            .expect("actual source method");
        let definition = method.definition_scope.expect("actual issued definition");
        let owner = ObjectFactOwner::DefinitionScript {
            definition,
            receiver: super::super::ObjectScopeReceiver::Instance,
        };
        assert!(definition_receiver_is_current(
            &owner, method, module, registry
        ));
        let other = module
            .methods
            .values()
            .find(|other| other.definition_scope != Some(definition))
            .unwrap();
        assert!(!definition_receiver_is_current(
            &owner, other, module, registry
        ));
        let mut missing = module.clone();
        missing.original_definition_scripts.remove(&definition);
        assert!(!definition_receiver_is_current(
            &owner, method, &missing, registry
        ));
        let mut changed = method.clone();
        changed.definition_scope = other.definition_scope;
        assert!(!definition_receiver_is_current(
            &owner, &changed, module, registry
        ));
        let mut outside = method.clone();
        outside.span = Some(tcl_lexer::Span::new(0, 1));
        assert!(!definition_receiver_is_current(
            &owner, &outside, module, registry
        ));
    }

    #[test]
    fn report_equality_keeps_supplementary_candidate_scope_maps() {
        let a = Analyser::new().analyse("set counter VALUE", "tcl8.6");
        let b = Analyser::new().analyse("set counter VALUE", "tcl8.6");
        assert!(a.object_handle_facts.logical_source_candidates.is_some());
        assert!(a == b);
        let mut names = b.clone();
        let owner = Arc::make_mut(
            names
                .object_handle_facts
                .logical_source_candidates
                .as_mut()
                .unwrap(),
        );
        let frame = *owner.frames.iter().next().unwrap();
        owner
            .instance_variables
            .entry(ObjectFactOwner::Frame(frame))
            .or_default()
            .insert("$literal".to_owned());
        assert!(names != a);
        assert!(names == names.clone());
        let mut frames = b.clone();
        let owner = Arc::make_mut(
            frames
                .object_handle_facts
                .logical_source_candidates
                .as_mut()
                .unwrap(),
        );
        owner.frames.clear();
        assert!(frames != a);
        assert!(frames == frames.clone());
    }
}
